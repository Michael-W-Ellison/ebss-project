// src/agents/temperature.rs
//! Temperature and climate system for agents.

use serde::{Deserialize, Serialize};

/// Temperature in Celsius
pub type Temperature = f32;

/// Agent's body temperature and tolerance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyTemperature {
    /// Current body temperature (37°C is normal for humans)
    pub current: Temperature,
    /// Ideal body temperature
    pub ideal: Temperature,
    /// Temperature tolerance range (±degrees before effects)
    pub tolerance: f32,
}

impl Default for BodyTemperature {
    fn default() -> Self {
        Self {
            current: 37.0,
            ideal: 37.0,
            tolerance: 2.0, // ±2°C before effects start
        }
    }
}

impl BodyTemperature {
    /// Create new body temperature system
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if body is too cold (hypothermia risk)
    pub fn is_too_cold(&self) -> bool {
        self.current < self.ideal - self.tolerance
    }

    /// Alias for is_too_cold - check for hypothermia risk
    pub fn is_hypothermic(&self) -> bool {
        self.is_too_cold()
    }

    /// Check if body is too hot (hyperthermia risk)
    pub fn is_too_hot(&self) -> bool {
        self.current > self.ideal + self.tolerance
    }

    /// Alias for is_too_hot - check for hyperthermia risk
    pub fn is_hyperthermic(&self) -> bool {
        self.is_too_hot()
    }

    /// Get temperature deviation from ideal (-ve = cold, +ve = hot)
    pub fn deviation(&self) -> f32 {
        self.current - self.ideal
    }

    /// Get severity of temperature problem (0.0 to 1.0)
    pub fn severity(&self) -> f32 {
        let deviation = self.deviation().abs();
        if deviation <= self.tolerance {
            return 0.0;
        }

        // Beyond tolerance, scale up to severity
        let excess = deviation - self.tolerance;
        // At 5°C beyond tolerance = 50% severity
        // At 10°C beyond tolerance = 100% severity
        (excess / 10.0).min(1.0)
    }

    /// How fast the body exchanges heat with its surroundings, per tick,
    /// per degree of difference
    const BASE_TRANSFER_RATE: f32 = 0.02;

    /// Degrees per tick the body can generate by shivering and burning fuel
    const WARMING_CAPACITY: f32 = 0.6;

    /// Degrees per tick the body can shed by sweating. Lower than the warming
    /// capacity: shedding heat is the harder direction for a body.
    const COOLING_CAPACITY: f32 = 0.1;

    /// Update body temperature based on environment and insulation
    ///
    /// Core temperature is held near the ideal by metabolic regulation, not
    /// left to drift toward the air: a person standing in 10°C weather is
    /// uncomfortable, not 10°C inside. Regulation counteracts what the
    /// environment is doing up to a fixed capacity, and only once the
    /// environment outpaces that capacity does the core actually move -
    /// which is what insulation buys, by slowing the exchange enough for
    /// regulation to keep up.
    ///
    /// Modelling regulation as a weak pull toward the ideal instead leaves
    /// every agent's temperature settling near ambient, so in ordinary
    /// weather they are permanently hypothermic.
    pub fn update(
        &mut self,
        environmental_temp: Temperature,
        cold_insulation: f32,
        heat_resistance: f32,
    ) {
        let temp_diff = environmental_temp - self.current;

        let effective_transfer = if temp_diff > 0.0 {
            // Environment is warmer - agent heats up
            // Heat resistance reduces heating
            Self::BASE_TRANSFER_RATE * (1.0 - heat_resistance.clamp(0.0, 0.9))
        } else {
            // Environment is cooler - agent cools down
            // Cold insulation reduces cooling
            Self::BASE_TRANSFER_RATE * (1.0 - cold_insulation.clamp(0.0, 0.9))
        };

        let environmental_change = temp_diff * effective_transfer;

        // Regulation opposes the environment, but only as far as the body can
        // manage and never further than the environment is pushing
        let regulation = if environmental_change < 0.0 {
            (-environmental_change).min(Self::WARMING_CAPACITY)
        } else {
            -environmental_change.min(Self::COOLING_CAPACITY)
        };

        self.current += environmental_change + regulation;

        // Recovering the last of the deviation once conditions allow
        let recovery = (self.ideal - self.current) * 0.05;
        self.current += recovery;
    }
}

/// Environmental climate conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Climate {
    /// Current temperature
    pub temperature: Temperature,
    /// Humidity (0.0 to 1.0)
    pub humidity: f32,
    /// Wind speed (affects effective temperature)
    pub wind_speed: f32,
}

impl Default for Climate {
    fn default() -> Self {
        Self {
            temperature: 20.0, // 20°C = comfortable room temperature
            humidity: 0.5,
            wind_speed: 0.0,
        }
    }
}

impl Climate {
    /// Create new climate
    pub fn new(temperature: Temperature) -> Self {
        Self {
            temperature,
            ..Default::default()
        }
    }

    /// Get effective temperature (accounting for wind chill / heat index)
    /// Temperature the body experiences while under cover.
    ///
    /// Shelter blocks the wind and holds some warmth, so the inside of a hut
    /// during a cold snap is milder than the field outside it. It does not
    /// make the weather pleasant - it takes the edge off, which is the
    /// difference between sheltering being worth the walk and being pointless.
    pub fn sheltered_effective_temperature(&self) -> Temperature {
        /// What a sheltered space tends toward: cool, but survivable
        const INDOOR_COMFORT: Temperature = 18.0;
        /// How far shelter closes the gap to that
        const SHELTER_MODERATION: f32 = 0.6;

        // Out of the wind, so no wind chill
        let out_of_the_wind = Climate {
            wind_speed: 0.0,
            ..self.clone()
        };

        let outside = out_of_the_wind.effective_temperature();

        outside + (INDOOR_COMFORT - outside) * SHELTER_MODERATION
    }

    pub fn effective_temperature(&self) -> Temperature {
        let mut temp = self.temperature;

        // Wind chill when cold
        if temp < 10.0 && self.wind_speed > 0.0 {
            temp -= self.wind_speed * 0.5;
        }

        // Heat index when hot and humid
        if temp > 25.0 && self.humidity > 0.5 {
            temp += (temp - 25.0) * self.humidity * 0.3;
        }

        temp
    }

    /// Preset climates
    pub fn arctic() -> Self {
        Self {
            temperature: -20.0,
            humidity: 0.3,
            wind_speed: 5.0,
        }
    }

    pub fn temperate() -> Self {
        Self {
            temperature: 15.0,
            humidity: 0.5,
            wind_speed: 2.0,
        }
    }

    pub fn desert() -> Self {
        Self {
            temperature: 35.0,
            humidity: 0.2,
            wind_speed: 3.0,
        }
    }

    pub fn tropical() -> Self {
        Self {
            temperature: 30.0,
            humidity: 0.8,
            wind_speed: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_temperature_normal() {
        let body_temp = BodyTemperature::new();
        assert_eq!(body_temp.current, 37.0);
        assert!(!body_temp.is_too_cold());
        assert!(!body_temp.is_too_hot());
        assert_eq!(body_temp.severity(), 0.0);
    }

    #[test]
    fn test_body_temperature_cold() {
        let mut body_temp = BodyTemperature::new();
        body_temp.current = 34.0; // 3°C below ideal

        assert!(body_temp.is_too_cold());
        assert!(!body_temp.is_too_hot());
        assert!(body_temp.severity() > 0.0);
    }

    #[test]
    fn test_body_temperature_hot() {
        let mut body_temp = BodyTemperature::new();
        body_temp.current = 40.0; // 3°C above ideal

        assert!(!body_temp.is_too_cold());
        assert!(body_temp.is_too_hot());
        assert!(body_temp.severity() > 0.0);
    }

    #[test]
    fn test_temperature_update_cold_environment() {
        let mut body_temp = BodyTemperature::new();
        let initial = body_temp.current;

        // Cold environment, no insulation
        body_temp.update(0.0, 0.0, 0.0);

        // Should be cooling down
        assert!(body_temp.current < initial);
    }

    #[test]
    fn test_temperature_update_with_insulation() {
        let mut body_temp1 = BodyTemperature::new();
        let mut body_temp2 = BodyTemperature::new();

        // Same cold environment, different insulation
        for _ in 0..10 {
            body_temp1.update(0.0, 0.0, 0.0);  // No insulation
            body_temp2.update(0.0, 0.8, 0.0);  // Good insulation
        }

        // Agent with insulation should be warmer
        assert!(body_temp2.current > body_temp1.current);
    }

    #[test]
    fn test_temperature_update_hot_environment() {
        let mut body_temp = BodyTemperature::new();
        let initial = body_temp.current;

        // Hot environment, no heat resistance
        body_temp.update(45.0, 0.0, 0.0);

        // Should be heating up
        assert!(body_temp.current > initial);
    }

    #[test]
    fn test_heat_resistance() {
        let mut body_temp1 = BodyTemperature::new();
        let mut body_temp2 = BodyTemperature::new();

        // Hot environment, different heat resistance
        for _ in 0..10 {
            body_temp1.update(45.0, 0.0, 0.0);  // No resistance
            body_temp2.update(45.0, 0.0, 0.8);  // Good resistance
        }

        // Agent with heat resistance should be cooler
        assert!(body_temp2.current < body_temp1.current);
    }

    #[test]
    fn test_climate_presets() {
        let arctic = Climate::arctic();
        let desert = Climate::desert();
        let tropical = Climate::tropical();

        assert!(arctic.temperature < 0.0);
        assert!(desert.temperature > 30.0);
        assert!(tropical.humidity > 0.5);
    }

    #[test]
    fn test_wind_chill() {
        let mut climate = Climate::new(0.0);
        climate.wind_speed = 0.0;
        let temp_no_wind = climate.effective_temperature();

        climate.wind_speed = 10.0;
        let temp_with_wind = climate.effective_temperature();

        // Wind makes it feel colder
        assert!(temp_with_wind < temp_no_wind);
    }

    #[test]
    fn test_heat_index() {
        let mut climate = Climate::new(30.0);
        climate.humidity = 0.2;
        let temp_low_humidity = climate.effective_temperature();

        climate.humidity = 0.9;
        let temp_high_humidity = climate.effective_temperature();

        // High humidity makes heat feel worse
        assert!(temp_high_humidity > temp_low_humidity);
    }
}
