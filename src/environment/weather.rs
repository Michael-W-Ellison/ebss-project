// src/environment/weather.rs
//! Weather system with precipitation, storms, and dynamic conditions

use serde::{Deserialize, Serialize};
use crate::agents::temperature::Temperature;
use super::seasons::Season;

/// Types of precipitation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecipitationType {
    None,
    Rain,
    Snow,
    Sleet,
    Hail,
}

/// Weather conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherType {
    Clear,
    PartlyCloudy,
    Cloudy,
    Overcast,
    LightRain,
    Rain,
    HeavyRain,
    Thunderstorm,
    LightSnow,
    Snow,
    Blizzard,
    Fog,
    Sandstorm,
}

impl WeatherType {
    /// Get precipitation type for this weather
    pub fn precipitation_type(&self) -> PrecipitationType {
        match self {
            WeatherType::LightRain | WeatherType::Rain | WeatherType::HeavyRain | WeatherType::Thunderstorm => {
                PrecipitationType::Rain
            }
            WeatherType::LightSnow | WeatherType::Snow | WeatherType::Blizzard => {
                PrecipitationType::Snow
            }
            _ => PrecipitationType::None,
        }
    }

    /// Get precipitation intensity (0.0 to 1.0)
    pub fn precipitation_intensity(&self) -> f32 {
        match self {
            WeatherType::LightRain | WeatherType::LightSnow => 0.3,
            WeatherType::Rain | WeatherType::Snow => 0.6,
            WeatherType::HeavyRain | WeatherType::Blizzard | WeatherType::Thunderstorm => 1.0,
            _ => 0.0,
        }
    }

    /// Get visibility reduction (0.0 = clear, 1.0 = no visibility)
    pub fn visibility_reduction(&self) -> f32 {
        match self {
            WeatherType::Clear => 0.0,
            WeatherType::PartlyCloudy => 0.0,
            WeatherType::Cloudy | WeatherType::Overcast => 0.1,
            WeatherType::LightRain | WeatherType::LightSnow => 0.2,
            WeatherType::Rain | WeatherType::Snow | WeatherType::Fog => 0.4,
            WeatherType::HeavyRain | WeatherType::Thunderstorm => 0.6,
            WeatherType::Blizzard | WeatherType::Sandstorm => 0.8,
        }
    }

    /// Get movement speed modifier (1.0 = normal, 0.5 = half speed)
    pub fn movement_modifier(&self) -> f32 {
        match self {
            WeatherType::Clear | WeatherType::PartlyCloudy | WeatherType::Cloudy => 1.0,
            WeatherType::Overcast | WeatherType::LightRain | WeatherType::LightSnow => 0.9,
            WeatherType::Rain | WeatherType::Snow | WeatherType::Fog => 0.8,
            WeatherType::HeavyRain | WeatherType::Thunderstorm => 0.7,
            WeatherType::Blizzard | WeatherType::Sandstorm => 0.5,
        }
    }

    /// Get temperature modifier (adjustment in degrees C)
    pub fn temperature_modifier(&self) -> Temperature {
        match self {
            WeatherType::Clear => 0.0,
            WeatherType::PartlyCloudy => -1.0,
            WeatherType::Cloudy | WeatherType::Overcast => -2.0,
            WeatherType::LightRain | WeatherType::Rain => -3.0,
            WeatherType::HeavyRain | WeatherType::Thunderstorm => -5.0,
            WeatherType::LightSnow | WeatherType::Snow => -8.0,
            WeatherType::Blizzard => -12.0,
            WeatherType::Fog => -2.0,
            WeatherType::Sandstorm => 5.0, // Hot wind
        }
    }

    /// Get wind speed modifier (multiplier)
    pub fn wind_modifier(&self) -> f32 {
        match self {
            WeatherType::Clear | WeatherType::PartlyCloudy => 1.0,
            WeatherType::Cloudy => 1.2,
            WeatherType::Overcast | WeatherType::LightRain | WeatherType::LightSnow => 1.3,
            WeatherType::Rain | WeatherType::Snow | WeatherType::Fog => 1.5,
            WeatherType::HeavyRain => 1.8,
            WeatherType::Thunderstorm => 2.5,
            WeatherType::Blizzard | WeatherType::Sandstorm => 3.0,
        }
    }

    /// Is this dangerous weather?
    pub fn is_dangerous(&self) -> bool {
        matches!(self,
            WeatherType::Thunderstorm | WeatherType::Blizzard | WeatherType::Sandstorm
        )
    }

    /// Get exposure damage per tick (0.0 to 1.0)
    pub fn exposure_damage_per_tick(&self) -> f32 {
        match self {
            WeatherType::Thunderstorm => 0.02,
            WeatherType::Blizzard => 0.05,
            WeatherType::Sandstorm => 0.03,
            WeatherType::HeavyRain => 0.01,
            _ => 0.0,
        }
    }
}

/// Current weather state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weather {
    pub weather_type: WeatherType,
    pub duration_remaining: u32, // Ticks until weather changes
    pub base_temperature: Temperature,
    pub base_wind_speed: f32,
}

impl Weather {
    pub fn new(weather_type: WeatherType) -> Self {
        Self {
            weather_type,
            duration_remaining: 1000, // Default: weather lasts 1000 ticks
            base_temperature: 20.0,
            base_wind_speed: 2.0,
        }
    }

    /// Create clear weather
    pub fn clear() -> Self {
        Self::new(WeatherType::Clear)
    }

    /// Get effective temperature with weather modifier
    pub fn effective_temperature(&self, base_temp: Temperature) -> Temperature {
        base_temp + self.weather_type.temperature_modifier()
    }

    /// Get effective wind speed with weather modifier
    pub fn effective_wind_speed(&self) -> f32 {
        self.base_wind_speed * self.weather_type.wind_modifier()
    }

    /// Tick the weather system
    pub fn tick(&mut self) {
        if self.duration_remaining > 0 {
            self.duration_remaining -= 1;
        }
    }

    /// Check if weather should change
    pub fn should_change(&self) -> bool {
        self.duration_remaining == 0
    }

    /// Get wetness accumulation per tick (for agents/items)
    pub fn wetness_per_tick(&self) -> f32 {
        self.weather_type.precipitation_intensity() * 0.01
    }

    /// Get visibility reduction (0.0 to 1.0)
    pub fn visibility_reduction(&self) -> f32 {
        self.weather_type.visibility_reduction()
    }

    /// Get movement speed modifier (0.0 to 1.0)
    pub fn movement_modifier(&self) -> f32 {
        self.weather_type.movement_modifier()
    }
}

impl Default for Weather {
    fn default() -> Self {
        Self::clear()
    }
}

/// Weather generator that creates realistic weather patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherGenerator {
    /// Current season
    pub season: Season,
    /// Is this a wet climate? (affects rain probability)
    wet_climate: bool,
    /// Is this a cold climate? (affects snow probability)
    cold_climate: bool,
}

impl WeatherGenerator {
    pub fn new(season: Season, wet_climate: bool, cold_climate: bool) -> Self {
        Self {
            season,
            wet_climate,
            cold_climate,
        }
    }

    /// Generate new weather based on current conditions
    pub fn generate_weather(&self) -> Weather {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Determine if precipitation occurs
        let precip_chance = if self.wet_climate { 0.3 } else { 0.1 };
        let has_precipitation = rng.gen::<f32>() < precip_chance;

        let weather_type = if !has_precipitation {
            // Clear weather types
            let cloud_roll = rng.gen::<f32>();
            if cloud_roll < 0.4 {
                WeatherType::Clear
            } else if cloud_roll < 0.7 {
                WeatherType::PartlyCloudy
            } else if cloud_roll < 0.9 {
                WeatherType::Cloudy
            } else {
                WeatherType::Overcast
            }
        } else {
            // Precipitation - check season and climate for type
            let is_cold_season = matches!(self.season, Season::Winter) ||
                                (matches!(self.season, Season::Spring | Season::Fall) && self.cold_climate);

            if is_cold_season {
                // Snow
                let intensity = rng.gen::<f32>();
                if intensity < 0.4 {
                    WeatherType::LightSnow
                } else if intensity < 0.8 {
                    WeatherType::Snow
                } else {
                    WeatherType::Blizzard
                }
            } else {
                // Rain
                let intensity = rng.gen::<f32>();
                if intensity < 0.3 {
                    WeatherType::LightRain
                } else if intensity < 0.7 {
                    WeatherType::Rain
                } else if intensity < 0.9 {
                    WeatherType::HeavyRain
                } else {
                    WeatherType::Thunderstorm
                }
            }
        };

        // Determine duration (500-2000 ticks)
        let duration = rng.gen_range(500..2000);

        let mut weather = Weather::new(weather_type);
        weather.duration_remaining = duration;
        weather
    }

    /// Set current season
    pub fn set_season(&mut self, season: Season) {
        self.season = season;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precipitation_types() {
        assert_eq!(WeatherType::Rain.precipitation_type(), PrecipitationType::Rain);
        assert_eq!(WeatherType::Snow.precipitation_type(), PrecipitationType::Snow);
        assert_eq!(WeatherType::Clear.precipitation_type(), PrecipitationType::None);
    }

    #[test]
    fn test_precipitation_intensity() {
        assert!(WeatherType::HeavyRain.precipitation_intensity() > WeatherType::LightRain.precipitation_intensity());
        assert!(WeatherType::Blizzard.precipitation_intensity() > WeatherType::LightSnow.precipitation_intensity());
    }

    #[test]
    fn test_visibility_reduction() {
        assert!(WeatherType::Blizzard.visibility_reduction() > WeatherType::Rain.visibility_reduction());
        assert!(WeatherType::Clear.visibility_reduction() == 0.0);
    }

    #[test]
    fn test_dangerous_weather() {
        assert!(WeatherType::Thunderstorm.is_dangerous());
        assert!(WeatherType::Blizzard.is_dangerous());
        assert!(!WeatherType::Rain.is_dangerous());
    }

    #[test]
    fn test_exposure_damage() {
        assert!(WeatherType::Blizzard.exposure_damage_per_tick() > 0.0);
        assert!(WeatherType::Clear.exposure_damage_per_tick() == 0.0);
    }

    #[test]
    fn test_movement_modifier() {
        assert!(WeatherType::Blizzard.movement_modifier() < WeatherType::Rain.movement_modifier());
        assert!(WeatherType::Clear.movement_modifier() == 1.0);
    }

    #[test]
    fn test_weather_tick() {
        let mut weather = Weather::new(WeatherType::Rain);
        weather.duration_remaining = 10;

        weather.tick();
        assert_eq!(weather.duration_remaining, 9);

        assert!(!weather.should_change());

        for _ in 0..9 {
            weather.tick();
        }

        assert!(weather.should_change());
    }

    #[test]
    fn test_effective_temperature() {
        let mut weather = Weather::clear();
        weather.base_temperature = 20.0;

        let clear_temp = weather.effective_temperature();

        weather.weather_type = WeatherType::Blizzard;
        let blizzard_temp = weather.effective_temperature();

        assert!(blizzard_temp < clear_temp);
    }

    #[test]
    fn test_effective_wind_speed() {
        let mut weather = Weather::clear();
        weather.base_wind_speed = 2.0;

        let clear_wind = weather.effective_wind_speed();

        weather.weather_type = WeatherType::Thunderstorm;
        let storm_wind = weather.effective_wind_speed();

        assert!(storm_wind > clear_wind);
    }

    #[test]
    fn test_weather_generator() {
        let generator = WeatherGenerator::new(false, true); // Dry, cold climate
        let weather = generator.generate_weather(-5.0); // Cold temperature

        // In cold temperature, precipitation should be snow if it occurs
        if weather.weather_type.precipitation_type() != PrecipitationType::None {
            assert_eq!(weather.weather_type.precipitation_type(), PrecipitationType::Snow);
        }
    }

    #[test]
    fn test_wetness_accumulation() {
        let rain = Weather::new(WeatherType::HeavyRain);
        let clear = Weather::clear();

        assert!(rain.wetness_per_tick() > 0.0);
        assert_eq!(clear.wetness_per_tick(), 0.0);
    }
}
