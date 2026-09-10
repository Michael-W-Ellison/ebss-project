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

    /// What one of these does to a body in a day of it.
    ///
    /// Per *day*, not per tick, which is a correction rather than a rescaling:
    /// these were written as per-tick literals in November 2025, nine months
    /// before this model had a calendar at all, so "per tick" named no length
    /// of time and could not. When the turn went from two hours to half an
    /// hour they quietly became four times what they had been - and it shows.
    /// Measured over eight worlds a side, the share of deaths booked to the
    /// weather went from **3.2% at the two-hour turn to 28% at the half-hour
    /// one**, on the same map and the same people.
    ///
    /// The figures below are the old literals times twelve, which is the turn
    /// they were last balanced at. Nothing about the numbers is new; what is
    /// new is that they now name a day, so the next change to the turn length
    /// leaves the weather alone. See ISSUES #171 for the eight clocks this
    /// file was missed out of.
    pub fn damage_in_a_day(&self) -> f32 {
        self.damage_multiplier() * ExposureStatus::THE_TURN_THESE_WERE_WRITTEN_FOR
    }

    /// The literals as they were written, per turn of an unnamed length.
    ///
    /// Kept private to the day figure above so there is one way to ask.
    fn damage_multiplier(&self) -> f32 {
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

    /// Exposure damage shed in a day under cover, out of danger
    const SHELTERED_RECOVERY: f32 = 0.05 * Self::THE_TURN_THESE_WERE_WRITTEN_FOR;

    /// Exposure damage shed in a day in the open once conditions are safe
    const OPEN_AIR_RECOVERY: f32 = 0.02 * Self::THE_TURN_THESE_WERE_WRITTEN_FOR;

    /// How much of a day the literals in this file were written against.
    ///
    /// Twelve turns to the day - the two-hour turn - which is the clock this
    /// model's balance was last measured at before the half-hour turn. The
    /// literals themselves predate any calendar, so this is not what they were
    /// *designed* for; it is the last length of time at which the weather was
    /// observed to behave, and it is stated here so that a rate in this file
    /// means something rather than meaning "per call".
    pub(crate) const THE_TURN_THESE_WERE_WRITTEN_FOR: f32 = 12.0;

    /// Wetness taken on in a day of standing out in it, per unit of rain.
    const SOAKED_IN_A_DAY: f32 = 1.0 * Self::THE_TURN_THESE_WERE_WRITTEN_FOR;

    /// Wetness dried off in a day under cover.
    const DRIED_IN_A_DAY: f32 = 0.01 * Self::THE_TURN_THESE_WERE_WRITTEN_FOR;

    /// Sun taken on in a day of it, and shed in a night.
    const BURNT_IN_A_DAY: f32 = 0.01 * Self::THE_TURN_THESE_WERE_WRITTEN_FOR;
    const FADES_IN_A_DAY: f32 = 0.005 * Self::THE_TURN_THESE_WERE_WRITTEN_FOR;

    /// What a day's worth of one of these rates comes to in one turn.
    ///
    /// The one place the calendar enters this file. Everything above is per
    /// day, so shortening the turn makes each step smaller rather than making
    /// the weather worse.
    fn in_one_turn(in_a_day: f32) -> f32 {
        in_a_day / crate::environment::seasons::TICKS_PER_DAY as f32
    }

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
            self.wetness += Self::in_one_turn(
                weather.how_wet_it_gets_you_in_a_day() * Self::SOAKED_IN_A_DAY,
            );
            self.wetness = self.wetness.min(1.0);
        } else {
            // Dry off slowly in shelter
            self.wetness =
                (self.wetness - Self::in_one_turn(Self::DRIED_IN_A_DAY)).max(0.0);
        }

        // Check for hypothermia
        if body_temp.is_too_cold() {
            self.active_exposures.push(ExposureType::Hypothermia);
            let severity = body_temp.severity();
            damage_this_tick +=
                severity * Self::in_one_turn(ExposureType::Hypothermia.damage_in_a_day());

            // Frostbite risk when wet and cold
            if self.wetness > 0.3 && environmental_temp < 0.0 {
                self.active_exposures.push(ExposureType::Frostbite);
                damage_this_tick += self.wetness
                    * Self::in_one_turn(ExposureType::Frostbite.damage_in_a_day());
            }
        }

        // Check for hyperthermia
        if body_temp.is_too_hot() {
            self.active_exposures.push(ExposureType::Hyperthermia);
            let severity = body_temp.severity();
            damage_this_tick +=
                severity * Self::in_one_turn(ExposureType::Hyperthermia.damage_in_a_day());

            // Dehydration risk in extreme heat without water
            if environmental_temp > 35.0 && !has_water_access {
                self.active_exposures.push(ExposureType::Dehydration);
                damage_this_tick +=
                    Self::in_one_turn(ExposureType::Dehydration.damage_in_a_day());
            }
        }

        // Sun exposure during daytime (6 AM to 6 PM)
        if !has_shelter && time_of_day >= 6.0 && time_of_day <= 18.0 {
            self.sun_exposure += Self::in_one_turn(Self::BURNT_IN_A_DAY);

            // Sunburn after prolonged exposure
            if self.sun_exposure > 0.5 && environmental_temp > 25.0 {
                self.active_exposures.push(ExposureType::Sunburn);
                damage_this_tick +=
                    Self::in_one_turn(ExposureType::Sunburn.damage_in_a_day());
            }
        } else {
            // Sun exposure fades at night
            self.sun_exposure =
                (self.sun_exposure - Self::in_one_turn(Self::FADES_IN_A_DAY)).max(0.0);
        }

        // Wind burn in high wind conditions
        if !has_shelter && weather.effective_wind_speed() > 10.0 {
            self.active_exposures.push(ExposureType::Windburn);
            damage_this_tick +=
                Self::in_one_turn(ExposureType::Windburn.damage_in_a_day());
        }

        // Add weather-specific exposure damage
        if !has_shelter {
            damage_this_tick +=
                Self::in_one_turn(weather.weather_type.exposure_damage_in_a_day());
        }

        self.exposure_damage += damage_this_tick;

        // Recover once nothing is harming the agent any more. Shelter speeds
        // it up, but an agent that has simply warmed up in the open is no
        // longer suffering and must be able to shed what it took: recovery
        // used to happen only inside the SeekShelter action, and only under
        // cover, so an agent that could not reach shelter kept accumulating
        // damage until it read as critically exposed for the rest of its life.
        if self.active_exposures.is_empty() {
            let recovery = Self::in_one_turn(if has_shelter {
                Self::SHELTERED_RECOVERY
            } else {
                Self::OPEN_AIR_RECOVERY
            });

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

    /// A day of it costs a day's worth, whatever a turn is.
    ///
    /// The guard on the whole of this file. Every rate here was a bare
    /// literal applied once a turn, so shortening the turn made the weather
    /// four times as deadly without anybody changing a number - see
    /// `ExposureStatus::THE_TURN_THESE_WERE_WRITTEN_FOR`. Ticking a body
    /// through a whole simulated day and adding up what it took has to come
    /// to the per-day figure, and that stays true if `TICKS_PER_DAY` changes
    /// again.
    #[test]
    fn a_day_of_a_blizzard_costs_a_day_of_a_blizzard() {
        use crate::environment::seasons::TICKS_PER_DAY;

        let mut status = ExposureStatus::new();
        let body_temp = BodyTemperature::new();
        let mut weather = Weather::clear();
        weather.weather_type = WeatherType::Blizzard;

        // A body at its ideal temperature, so the only thing being counted is
        // the weather itself rather than the cold on top of it. Midnight, so
        // no sun. In the open, or the weather does not reach him.
        let mut took = 0.0;
        for _ in 0..TICKS_PER_DAY {
            took += status.update(&body_temp, 5.0, &weather, false, true, 0.0);
        }

        let a_day_of_it = WeatherType::Blizzard.exposure_damage_in_a_day();
        assert!(
            (took - a_day_of_it).abs() < a_day_of_it * 0.01,
            "a day in a blizzard came to {took:.4} against the {a_day_of_it:.4} \
             it is written down as"
        );
    }

    /// And the wind that comes with it is on the same clock.
    #[test]
    fn a_day_of_wind_costs_a_day_of_wind() {
        use crate::environment::seasons::TICKS_PER_DAY;

        let mut status = ExposureStatus::new();
        let body_temp = BodyTemperature::new();
        let mut weather = Weather::clear();
        weather.base_wind_speed = 20.0;

        let mut took = 0.0;
        for _ in 0..TICKS_PER_DAY {
            took += status.update(&body_temp, 5.0, &weather, false, true, 0.0);
        }

        let a_day_of_it = ExposureType::Windburn.damage_in_a_day();
        assert!(
            (took - a_day_of_it).abs() < a_day_of_it * 0.01,
            "a day in the wind came to {took:.4} against the {a_day_of_it:.4} \
             it is written down as"
        );
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
        use crate::environment::seasons::TICKS_PER_DAY;

        let mut status = ExposureStatus::new();
        let body_temp = BodyTemperature::new();
        let weather = Weather::clear();

        // Five days of standing out at noon. This used to say "100 ticks",
        // which was eight days at the two-hour turn and two at the half-hour
        // one - the run length changed meaning when the turn did, which is
        // the whole of ISSUES #171. Said in days it stays five days.
        for _ in 0..(5 * TICKS_PER_DAY) {
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
