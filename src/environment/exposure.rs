// src/environment/exposure.rs
//! Exposure damage system for environmental hazards
//!
//! Handles damage from:
//! - Hypothermia (extreme cold)
//! - Hyperthermia (extreme heat)
//! - Dehydration
//! - Wetness and frostbite
//! - Sun exposure

use serde::{Deserialize, Serialize};
use crate::agents::temperature::{BodyTemperature, Temperature};
use super::weather::Weather;

/// Type of exposure damage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExposureType {
    /// Extreme cold exposure
    Hypothermia,
    /// Extreme heat exposure
    Hyperthermia,
    /// Frostbite from cold + wetness
    Frostbite,
    /// Sunburn from prolonged sun exposure
    Sunburn,
    /// Dehydration in hot/dry conditions
    Dehydration,
    /// Wind burn from high winds
    Windburn,
}

impl ExposureType {

    /// Get damage multiplier per tick
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            ExposureType::Hypothermia => 0.02,
            ExposureType::Hyperthermia => 0.015,
            ExposureType::Frostbite => 0.03,
            ExposureType::Sunburn => 0.005,
            ExposureType::Dehydration => 0.025,
            ExposureType::Windburn => 0.01,
        }
    }
}

/// Exposure status for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureStatus {
    /// Wetness level (0.0 to 1.0)
    pub wetness: f32,

    /// Cumulative sun exposure (accumulates during day)
    pub sun_exposure: f32,

    /// Current exposure conditions
    pub active_exposures: Vec<ExposureType>,

    /// Cumulative exposure damage
    pub exposure_damage: f32,
}

impl ExposureStatus {
    /// Ceiling on accumulated exposure damage - the point at which severity
    /// is already total, so counting higher means nothing
    pub const MAX_EXPOSURE_DAMAGE: f32 = 10.0;

    /// Exposure damage shed per tick while sheltered and out of danger
    const SHELTERED_RECOVERY: f32 = 0.05;

    /// Exposure damage shed per tick in the open once conditions are safe
    const OPEN_AIR_RECOVERY: f32 = 0.02;

    pub fn new() -> Self {
        Self {
            wetness: 0.0,
            sun_exposure: 0.0,
            active_exposures: Vec::new(),
            exposure_damage: 0.0,
        }
    }

    /// Update exposure status based on environment
    pub fn update(
        &mut self,
        body_temp: &BodyTemperature,
        environmental_temp: Temperature,
        weather: &Weather,
        has_shelter: bool,
        has_water_access: bool,
        time_of_day: f32,
    ) -> f32 {
        let mut damage_this_tick = 0.0;
        self.active_exposures.clear();

        // Update wetness
        if !has_shelter {
            self.wetness += weather.wetness_per_tick();
            self.wetness = self.wetness.min(1.0);
        } else {
            // Dry off slowly in shelter
            self.wetness = (self.wetness - 0.01).max(0.0);
        }

        // Check for hypothermia
        if body_temp.is_too_cold() {
            self.active_exposures.push(ExposureType::Hypothermia);
            let severity = body_temp.severity();
            damage_this_tick += severity * ExposureType::Hypothermia.damage_multiplier();

            // Frostbite risk when wet and cold
            if self.wetness > 0.3 && environmental_temp < 0.0 {
                self.active_exposures.push(ExposureType::Frostbite);
                damage_this_tick += self.wetness * ExposureType::Frostbite.damage_multiplier();
            }
        }

        // Check for hyperthermia
        if body_temp.is_too_hot() {
            self.active_exposures.push(ExposureType::Hyperthermia);
            let severity = body_temp.severity();
            damage_this_tick += severity * ExposureType::Hyperthermia.damage_multiplier();

            // Dehydration risk in extreme heat without water
            if environmental_temp > 35.0 && !has_water_access {
                self.active_exposures.push(ExposureType::Dehydration);
                damage_this_tick += ExposureType::Dehydration.damage_multiplier();
            }
        }

        // Sun exposure during daytime (6 AM to 6 PM)
        if !has_shelter && time_of_day >= 6.0 && time_of_day <= 18.0 {
            self.sun_exposure += 0.01;

            // Sunburn after prolonged exposure
            if self.sun_exposure > 0.5 && environmental_temp > 25.0 {
                self.active_exposures.push(ExposureType::Sunburn);
                damage_this_tick += ExposureType::Sunburn.damage_multiplier();
            }
        } else {
            // Sun exposure fades at night
            self.sun_exposure = (self.sun_exposure - 0.005).max(0.0);
        }

        // Wind burn in high wind conditions
        if !has_shelter && weather.effective_wind_speed() > 10.0 {
            self.active_exposures.push(ExposureType::Windburn);
            damage_this_tick += ExposureType::Windburn.damage_multiplier();
        }

        // Add weather-specific exposure damage
        if !has_shelter {
            damage_this_tick += weather.weather_type.exposure_damage_per_tick();
        }

        self.exposure_damage += damage_this_tick;

        // Recover once nothing is harming the agent any more. Shelter speeds
        // it up, but an agent that has simply warmed up in the open is no
        // longer suffering and must be able to shed what it took: recovery
        // used to happen only inside the SeekShelter action, and only under
        // cover, so an agent that could not reach shelter kept accumulating
        // damage until it read as critically exposed for the rest of its life.
        if self.active_exposures.is_empty() {
            let recovery = if has_shelter {
                Self::SHELTERED_RECOVERY
            } else {
                Self::OPEN_AIR_RECOVERY
            };

            self.exposure_damage = (self.exposure_damage - recovery).max(0.0);
        }

        // Cap the accumulated total. Damage is a measure of how bad the
        // agent's condition is, and severity already saturates here; letting
        // it run to arbitrary values leaves an agent that has since warmed up
        // still reading as critically exposed hundreds of ticks later.
        self.exposure_damage = self.exposure_damage.min(Self::MAX_EXPOSURE_DAMAGE);

        damage_this_tick
    }


    /// Is the agent in critical condition from exposure?
    pub fn is_critical(&self) -> bool {
        self.exposure_damage > 5.0
    }

    /// Get recommended action to reduce exposure
    pub fn recommended_action(&self) -> Option<String> {
        if self.active_exposures.is_empty() {
            return None;
        }

        // Prioritize by severity
        if self.active_exposures.contains(&ExposureType::Hypothermia) {
            Some("Seek warm shelter immediately".to_string())
        } else if self.active_exposures.contains(&ExposureType::Frostbite) {
            Some("Get dry and warm urgently".to_string())
        } else if self.active_exposures.contains(&ExposureType::Hyperthermia) {
            Some("Find shade and cool down".to_string())
        } else if self.active_exposures.contains(&ExposureType::Dehydration) {
            Some("Find water immediately".to_string())
        } else if self.active_exposures.contains(&ExposureType::Sunburn) {
            Some("Seek shade".to_string())
        } else if self.active_exposures.contains(&ExposureType::Windburn) {
            Some("Find wind protection".to_string())
        } else {
            None
        }
    }

    /// Recover from exposure (in shelter with warmth/water)
    pub fn recover(&mut self, recovery_rate: f32) {
        self.exposure_damage = (self.exposure_damage - recovery_rate).max(0.0);
        self.wetness = (self.wetness - recovery_rate).max(0.0);
        self.sun_exposure = (self.sun_exposure - recovery_rate * 0.5).max(0.0);
    }
}

impl Default for ExposureStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate protection from clothing/equipment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureProtection {
    /// Cold protection (0.0 to 1.0)
    pub cold_insulation: f32,

    /// Heat protection (0.0 to 1.0)
    pub heat_resistance: f32,

    /// Water resistance (0.0 to 1.0)
    pub water_resistance: f32,

    /// Wind protection (0.0 to 1.0)
    pub wind_protection: f32,

    /// Sun protection (0.0 to 1.0)
    pub sun_protection: f32,
}

impl ExposureProtection {
    pub fn new() -> Self {
        Self {
            cold_insulation: 0.0,
            heat_resistance: 0.0,
            water_resistance: 0.0,
            wind_protection: 0.0,
            sun_protection: 0.0,
        }
    }

    /// Get overall protection rating (0.0 to 1.0)
    pub fn overall_rating(&self) -> f32 {
        (self.cold_insulation + self.heat_resistance + self.water_resistance +
         self.wind_protection + self.sun_protection) / 5.0
    }

    /// Create protection from basic clothing
    pub fn basic_clothing() -> Self {
        Self {
            cold_insulation: 0.3,
            heat_resistance: 0.1,
            water_resistance: 0.1,
            wind_protection: 0.2,
            sun_protection: 0.3,
        }
    }

    /// Create protection from winter gear
    pub fn winter_gear() -> Self {
        Self {
            cold_insulation: 0.8,
            heat_resistance: 0.0,
            water_resistance: 0.6,
            wind_protection: 0.7,
            sun_protection: 0.2,
        }
    }

    /// Create protection from desert clothing
    pub fn desert_clothing() -> Self {
        Self {
            cold_insulation: 0.2,
            heat_resistance: 0.7,
            water_resistance: 0.1,
            wind_protection: 0.4,
            sun_protection: 0.8,
        }
    }

    /// Reduce wetness impact on insulation
    pub fn effective_cold_insulation(&self, wetness: f32) -> f32 {
        // Wetness reduces insulation effectiveness
        self.cold_insulation * (1.0 - wetness * 0.5)
    }
}

impl Default for ExposureProtection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::weather::WeatherType;

    #[test]
    fn test_exposure_status_creation() {
        let status = ExposureStatus::new();
        assert_eq!(status.wetness, 0.0);
        assert_eq!(status.exposure_damage, 0.0);
    }

    #[test]
    fn test_hypothermia_detection() {
        let mut status = ExposureStatus::new();
        let mut body_temp = BodyTemperature::new();
        body_temp.current = 34.0; // Below ideal - tolerance

        let weather = Weather::clear();
        let damage = status.update(&body_temp, -10.0, &weather, false, true, 12.0);

        assert!(status.active_exposures.contains(&ExposureType::Hypothermia));
        assert!(damage > 0.0);
    }

    #[test]
    fn test_hyperthermia_detection() {
        let mut status = ExposureStatus::new();
        let mut body_temp = BodyTemperature::new();
        body_temp.current = 40.0; // Above ideal + tolerance

        let weather = Weather::clear();
        let damage = status.update(&body_temp, 45.0, &weather, false, true, 12.0);

        assert!(status.active_exposures.contains(&ExposureType::Hyperthermia));
        assert!(damage > 0.0);
    }

    #[test]
    fn test_frostbite_requires_wetness() {
        let mut status = ExposureStatus::new();
        status.wetness = 0.8;

        let mut body_temp = BodyTemperature::new();
        body_temp.current = 33.0;

        let weather = Weather::clear();
        status.update(&body_temp, -5.0, &weather, false, true, 12.0);

        assert!(status.active_exposures.contains(&ExposureType::Frostbite));
    }

    #[test]
    fn test_dehydration_in_heat() {
        let mut status = ExposureStatus::new();
        let mut body_temp = BodyTemperature::new();
        body_temp.current = 40.0;

        let weather = Weather::clear();
        status.update(&body_temp, 40.0, &weather, false, false, 12.0); // No water access

        assert!(status.active_exposures.contains(&ExposureType::Dehydration));
    }

    #[test]
    fn test_sunburn_accumulation() {
        let mut status = ExposureStatus::new();
        let body_temp = BodyTemperature::new();
        let weather = Weather::clear();

        // Simulate several hours of sun exposure
        for _ in 0..100 {
            status.update(&body_temp, 30.0, &weather, false, true, 12.0); // Noon
        }

        assert!(status.sun_exposure > 0.5);
        assert!(status.active_exposures.contains(&ExposureType::Sunburn));
    }

    #[test]
    fn test_wetness_accumulation() {
        let mut status = ExposureStatus::new();
        let body_temp = BodyTemperature::new();
        let mut weather = Weather::new(WeatherType::HeavyRain);

        for _ in 0..50 {
            status.update(&body_temp, 20.0, &weather, false, true, 12.0);
        }

        assert!(status.wetness > 0.0);
    }

    #[test]
    fn test_shelter_protects() {
        let mut status_exposed = ExposureStatus::new();
        let mut status_sheltered = ExposureStatus::new();

        let mut body_temp = BodyTemperature::new();
        body_temp.current = 34.0;

        let weather = Weather::new(WeatherType::HeavyRain);

        let damage_exposed = status_exposed.update(&body_temp, -5.0, &weather, false, true, 12.0);
        let damage_sheltered = status_sheltered.update(&body_temp, -5.0, &weather, true, true, 12.0);

        // Shelter should reduce damage
        assert!(damage_sheltered < damage_exposed);
    }

    #[test]
    fn test_recovery() {
        let mut status = ExposureStatus::new();
        status.exposure_damage = 5.0;
        status.wetness = 0.8;

        status.recover(0.5);

        assert!(status.exposure_damage < 5.0);
        assert!(status.wetness < 0.8);
    }

    #[test]
    fn test_critical_condition() {
        let mut status = ExposureStatus::new();
        status.exposure_damage = 3.0;
        assert!(!status.is_critical());

        status.exposure_damage = 6.0;
        assert!(status.is_critical());
    }

    #[test]
    fn test_recommended_actions() {
        let mut status = ExposureStatus::new();
        status.active_exposures.push(ExposureType::Hypothermia);

        let action = status.recommended_action();
        assert!(action.is_some());
        assert!(action.unwrap().contains("warm"));
    }

    #[test]
    fn test_exposure_protection_ratings() {
        let winter = ExposureProtection::winter_gear();
        let desert = ExposureProtection::desert_clothing();

        assert!(winter.cold_insulation > desert.cold_insulation);
        assert!(desert.heat_resistance > winter.heat_resistance);
        assert!(desert.sun_protection > winter.sun_protection);
    }

    #[test]
    fn test_wetness_reduces_insulation() {
        let protection = ExposureProtection::winter_gear();

        let dry_insulation = protection.effective_cold_insulation(0.0);
        let wet_insulation = protection.effective_cold_insulation(0.8);

        assert!(wet_insulation < dry_insulation);
    }

    #[test]
    fn test_sun_exposure_fades_at_night() {
        let mut status = ExposureStatus::new();
        status.sun_exposure = 0.8;

        let body_temp = BodyTemperature::new();
        let weather = Weather::clear();

        // Night time
        for _ in 0..20 {
            status.update(&body_temp, 20.0, &weather, false, true, 2.0);
        }

        assert!(status.sun_exposure < 0.8);
    }

    #[test]
    fn test_windburn_in_high_wind() {
        let mut status = ExposureStatus::new();
        let body_temp = BodyTemperature::new();
        let mut weather = Weather::new(WeatherType::Blizzard);
        weather.base_wind_speed = 15.0;

        status.update(&body_temp, 0.0, &weather, false, true, 12.0);

        assert!(status.active_exposures.contains(&ExposureType::Windburn));
    }
}
