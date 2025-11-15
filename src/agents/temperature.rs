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

    /// Check if body is too hot (hyperthermia risk)
    pub fn is_too_hot(&self) -> bool {
        self.current > self.ideal + self.tolerance
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

    /// Update body temperature based on environment and insulation
    pub fn update(
        &mut self,
        environmental_temp: Temperature,
        cold_insulation: f32,
        heat_resistance: f32,
    ) {
        let temp_diff = environmental_temp - self.current;

        // Calculate heat transfer rate
        let base_transfer_rate = 0.1; // How fast temperature changes

        let effective_transfer = if temp_diff > 0.0 {
            // Environment is warmer - agent heats up
            // Heat resistance reduces heating
            base_transfer_rate * (1.0 - heat_resistance.min(0.9))
        } else {
            // Environment is cooler - agent cools down
            // Cold insulation reduces cooling
            base_transfer_rate * (1.0 - cold_insulation.min(0.9))
        };

        // Apply temperature change
        let change = temp_diff * effective_transfer;
        self.current += change;

        // Body tries to regulate back to ideal (metabolic regulation)
        let regulation = (self.ideal - self.current) * 0.05; // Slow regulation
        self.current += regulation;
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
