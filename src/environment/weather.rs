// src/environment/weather.rs
//! Weather system with precipitation, storms, and dynamic conditions

use serde::{Deserialize, Serialize};
use crate::agents::temperature::Temperature;
use crate::environment::BiomeType;
use super::seasons::{Season, TICKS_PER_DAY};

/// How long a stretch of weather lasts, given in hours and answered in ticks.
///
/// Durations used to be written straight in ticks, back when a tick was
/// thirty-six seconds and five hundred to two thousand of them was five to
/// twenty hours - about how long a front sits over one place. A tick is two
/// hours now, so those same numbers had become forty to a hundred and sixty
/// days: a single blizzard outlasting the winter that started it and still
/// blowing the following summer, which is what the runs showed. Snow turned up
/// in all four seasons in equal measure.
fn hours_in_ticks(hours: u32) -> u32 {
    (hours * TICKS_PER_DAY / 24).max(1)
}

/// A spell of weather somewhere between the two lengths, in hours.
fn spell_of_weather(rng: &mut impl rand::Rng, from_hours: u32, to_hours: u32) -> u32 {
    let from = hours_in_ticks(from_hours);
    let to = hours_in_ticks(to_hours).max(from + 1);
    rng.gen_range(from..to)
}

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
    Sleet,
    Hail,
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
            WeatherType::Sleet => PrecipitationType::Sleet,
            WeatherType::Hail => PrecipitationType::Hail,
            _ => PrecipitationType::None,
        }
    }

    /// Get precipitation intensity (0.0 to 1.0)
    pub fn precipitation_intensity(&self) -> f32 {
        match self {
            WeatherType::LightRain | WeatherType::LightSnow => 0.3,
            WeatherType::Rain | WeatherType::Snow | WeatherType::Sleet => 0.6,
            WeatherType::HeavyRain | WeatherType::Blizzard | WeatherType::Thunderstorm | WeatherType::Hail => 1.0,
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
            WeatherType::Rain | WeatherType::Snow | WeatherType::Sleet => 0.4,
            WeatherType::Fog => 0.6,
            WeatherType::HeavyRain | WeatherType::Thunderstorm | WeatherType::Hail => 0.6,
            WeatherType::Blizzard | WeatherType::Sandstorm => 0.8,
        }
    }

    /// Get movement speed modifier (1.0 = normal, 0.5 = half speed)
    pub fn movement_modifier(&self) -> f32 {
        match self {
            WeatherType::Clear | WeatherType::PartlyCloudy | WeatherType::Cloudy => 1.0,
            WeatherType::Overcast | WeatherType::LightRain | WeatherType::LightSnow => 0.9,
            WeatherType::Rain | WeatherType::Snow | WeatherType::Fog | WeatherType::Sleet => 0.8,
            WeatherType::HeavyRain | WeatherType::Thunderstorm => 0.7,
            WeatherType::Hail => 0.6,
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
            WeatherType::Sleet => -6.0,
            WeatherType::Hail => -4.0,
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
            WeatherType::Cloudy | WeatherType::Fog => 1.2,
            WeatherType::Overcast | WeatherType::LightRain | WeatherType::LightSnow => 1.3,
            WeatherType::Rain | WeatherType::Snow | WeatherType::Sleet => 1.5,
            WeatherType::HeavyRain | WeatherType::Hail => 1.8,
            WeatherType::Thunderstorm => 2.5,
            WeatherType::Blizzard | WeatherType::Sandstorm => 3.0,
        }
    }

    /// Is this dangerous weather?
    pub fn is_dangerous(&self) -> bool {
        matches!(self,
            WeatherType::Thunderstorm | WeatherType::Blizzard | WeatherType::Sandstorm | WeatherType::Hail
        )
    }

    /// Get exposure damage per tick (0.0 to 1.0)
    pub fn exposure_damage_per_tick(&self) -> f32 {
        match self {
            WeatherType::Thunderstorm => 0.02,
            WeatherType::Blizzard => 0.05,
            WeatherType::Sandstorm => 0.03,
            WeatherType::Hail => 0.04,
            WeatherType::HeavyRain => 0.01,
            WeatherType::Sleet => 0.015,
            _ => 0.0,
        }
    }

    /// Can this weather cause lightning?
    pub fn can_cause_lightning(&self) -> bool {
        matches!(self, WeatherType::Thunderstorm)
    }

    /// Get lightning strike chance per tick (0.0 to 1.0)
    pub fn lightning_chance_per_tick(&self) -> f32 {
        match self {
            WeatherType::Thunderstorm => 0.001, // ~0.1% chance per tick
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
            duration_remaining: hours_in_ticks(10),
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
    /// Current humidity level (0.0 to 1.0)
    pub humidity: f32,
    /// Dominant biome type (affects weather patterns)
    pub biome: Option<BiomeType>,
    /// Previous weather (for transitions)
    previous_weather: Option<WeatherType>,
}

impl WeatherGenerator {
    pub fn new(season: Season, wet_climate: bool, cold_climate: bool) -> Self {
        Self {
            season,
            wet_climate,
            cold_climate,
            humidity: 0.5,
            biome: None,
            previous_weather: None,
        }
    }

    /// Create generator with biome
    pub fn with_biome(season: Season, wet_climate: bool, cold_climate: bool, biome: BiomeType) -> Self {
        Self {
            season,
            wet_climate,
            cold_climate,
            humidity: 0.5,
            biome: Some(biome),
            previous_weather: None,
        }
    }

    /// Set humidity level
    pub fn set_humidity(&mut self, humidity: f32) {
        self.humidity = humidity.clamp(0.0, 1.0);
    }

    /// Set biome type
    pub fn set_biome(&mut self, biome: BiomeType) {
        self.biome = Some(biome);
    }

    /// Generate new weather based on current conditions
    pub fn generate_weather(&mut self) -> Weather {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        // Check for biome-specific weather first
        if let Some(biome_weather) = self.generate_biome_specific_weather(&mut rng) {
            let duration = spell_of_weather(&mut rng, 3, 15);
            let mut weather = Weather::new(biome_weather);
            weather.duration_remaining = duration;
            self.previous_weather = Some(biome_weather);
            return weather;
        }

        // Check for fog conditions (high humidity, calm weather)
        if self.humidity > 0.7 && rng.gen::<f32>() < 0.15 {
            let duration = spell_of_weather(&mut rng, 2, 8);
            let mut weather = Weather::new(WeatherType::Fog);
            weather.duration_remaining = duration;
            self.previous_weather = Some(WeatherType::Fog);
            return weather;
        }

        // Determine if precipitation occurs
        let base_precip_chance = if self.wet_climate { 0.3 } else { 0.1 };
        let humidity_bonus = (self.humidity - 0.5).max(0.0) * 0.3;
        let precip_chance = base_precip_chance + humidity_bonus;
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
            self.generate_precipitation_weather(&mut rng)
        };

        // Five to twenty hours: about as long as a front sits over one place
        let duration = spell_of_weather(&mut rng, 5, 20);

        let mut weather = Weather::new(weather_type);
        weather.duration_remaining = duration;
        self.previous_weather = Some(weather_type);
        weather
    }

    /// Generate biome-specific weather
    fn generate_biome_specific_weather(&self, rng: &mut impl rand::Rng) -> Option<WeatherType> {
        let biome = self.biome?;

        match biome {
            // Desert biomes can have sandstorms
            BiomeType::Desert => {
                if rng.gen::<f32>() < 0.12 {
                    return Some(WeatherType::Sandstorm);
                }
            }
            // Tropical biomes have more thunderstorms
            BiomeType::Tropical => {
                if rng.gen::<f32>() < 0.08 {
                    return Some(WeatherType::Thunderstorm);
                }
            }
            // Tundra/Arctic biomes have more blizzards
            BiomeType::Tundra => {
                if rng.gen::<f32>() < 0.10 {
                    return Some(WeatherType::Blizzard);
                }
            }
            // Wetlands have more fog
            BiomeType::Wetland => {
                if self.humidity > 0.5 && rng.gen::<f32>() < 0.15 {
                    return Some(WeatherType::Fog);
                }
            }
            // Coastal areas can have fog
            BiomeType::Coast => {
                if rng.gen::<f32>() < 0.10 {
                    return Some(WeatherType::Fog);
                }
            }
            _ => {}
        }

        None
    }

    /// Generate precipitation-based weather
    fn generate_precipitation_weather(&self, rng: &mut impl rand::Rng) -> WeatherType {
        let is_cold_season = matches!(self.season, Season::Winter) ||
                            (matches!(self.season, Season::Spring | Season::Fall) && self.cold_climate);

        // Transition seasons can have sleet
        let is_transition = matches!(self.season, Season::Spring | Season::Fall);

        if is_cold_season {
            // Snow weather
            let intensity = rng.gen::<f32>();
            if intensity < 0.4 {
                WeatherType::LightSnow
            } else if intensity < 0.8 {
                WeatherType::Snow
            } else {
                WeatherType::Blizzard
            }
        } else if is_transition && self.cold_climate {
            // Transition season in cold climate - can have sleet or hail
            let roll = rng.gen::<f32>();
            if roll < 0.15 {
                WeatherType::Sleet
            } else if roll < 0.20 {
                WeatherType::Hail
            } else {
                self.generate_rain_weather(rng)
            }
        } else {
            // Rain weather (with possible hail in summer storms)
            let roll = rng.gen::<f32>();
            if roll < 0.05 && matches!(self.season, Season::Summer) {
                WeatherType::Hail // Summer hailstorms
            } else {
                self.generate_rain_weather(rng)
            }
        }
    }

    /// Generate rain-type weather
    fn generate_rain_weather(&self, rng: &mut impl rand::Rng) -> WeatherType {
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


    /// Predict upcoming weather (returns likely next weather and confidence)
    pub fn forecast(&self, hours_ahead: u32) -> (WeatherType, f32) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        // Base confidence decreases with time
        let confidence = (1.0 - (hours_ahead as f32 / 48.0)).max(0.2);

        // Current weather tends to persist
        if let Some(current) = self.previous_weather {
            if rng.gen::<f32>() < 0.6 * confidence {
                return (current, confidence);
            }
        }

        // Otherwise predict based on conditions
        let predicted = if self.humidity > 0.7 {
            if self.cold_climate {
                WeatherType::Snow
            } else {
                WeatherType::Rain
            }
        } else if self.humidity < 0.3 {
            WeatherType::Clear
        } else {
            WeatherType::PartlyCloudy
        };

        (predicted, confidence)
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

        let clear_temp = weather.effective_temperature(weather.base_temperature);

        weather.weather_type = WeatherType::Blizzard;
        let blizzard_temp = weather.effective_temperature(weather.base_temperature);

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
        let mut generator = WeatherGenerator::new(Season::Winter, false, true); // Winter, dry, cold climate
        let weather = generator.generate_weather(); // Generate weather

        // In cold winter, precipitation should be snow if it occurs
        if weather.weather_type.precipitation_type() != PrecipitationType::None {
            assert_eq!(weather.weather_type.precipitation_type(), PrecipitationType::Snow);
        }
    }

    #[test]
    fn test_sleet_and_hail_types() {
        assert_eq!(WeatherType::Sleet.precipitation_type(), PrecipitationType::Sleet);
        assert_eq!(WeatherType::Hail.precipitation_type(), PrecipitationType::Hail);
        assert!(WeatherType::Hail.is_dangerous());
        assert!(WeatherType::Sleet.exposure_damage_per_tick() > 0.0);
        assert!(WeatherType::Hail.exposure_damage_per_tick() > WeatherType::Sleet.exposure_damage_per_tick());
    }

    #[test]
    fn test_fog_properties() {
        assert!(!WeatherType::Fog.is_dangerous());
        assert!(WeatherType::Fog.visibility_reduction() > WeatherType::Rain.visibility_reduction());
        assert_eq!(WeatherType::Fog.precipitation_type(), PrecipitationType::None);
    }

    #[test]
    fn test_sandstorm_properties() {
        assert!(WeatherType::Sandstorm.is_dangerous());
        assert!(WeatherType::Sandstorm.visibility_reduction() > 0.7);
        assert!(WeatherType::Sandstorm.temperature_modifier() > 0.0); // Hot wind
        assert!(WeatherType::Sandstorm.exposure_damage_per_tick() > 0.0);
    }

    #[test]
    fn test_biome_specific_weather() {
        // Desert biome should generate sandstorms
        let mut desert_gen = WeatherGenerator::with_biome(
            Season::Summer, false, false, BiomeType::Desert
        );

        // Generate many times and check we get at least one sandstorm
        let mut got_sandstorm = false;
        for _ in 0..100 {
            let weather = desert_gen.generate_weather();
            if weather.weather_type == WeatherType::Sandstorm {
                got_sandstorm = true;
                break;
            }
        }
        assert!(got_sandstorm, "Desert biome should occasionally generate sandstorms");
    }

    #[test]
    fn test_fog_generation_high_humidity() {
        let mut gen = WeatherGenerator::new(Season::Spring, true, false);
        gen.set_humidity(0.9);

        // With high humidity, fog should sometimes occur
        let mut got_fog = false;
        for _ in 0..100 {
            let weather = gen.generate_weather();
            if weather.weather_type == WeatherType::Fog {
                got_fog = true;
                break;
            }
        }
        assert!(got_fog, "High humidity should occasionally generate fog");
    }

    #[test]
    fn test_weather_forecast() {
        let mut gen = WeatherGenerator::new(Season::Summer, true, false);
        gen.set_humidity(0.8);
        let _ = gen.generate_weather(); // Generate to set previous_weather

        let (forecast, confidence) = gen.forecast(6);
        assert!(confidence > 0.0 && confidence <= 1.0);
        // Weather should be a valid type
        assert!(matches!(forecast, WeatherType::Clear | WeatherType::PartlyCloudy |
                        WeatherType::Rain | WeatherType::Snow | _));
    }

    #[test]
    fn test_lightning_chance() {
        assert!(WeatherType::Thunderstorm.can_cause_lightning());
        assert!(WeatherType::Thunderstorm.lightning_chance_per_tick() > 0.0);
        assert!(!WeatherType::Rain.can_cause_lightning());
        assert_eq!(WeatherType::Rain.lightning_chance_per_tick(), 0.0);
    }

    #[test]
    fn test_wetness_accumulation() {
        let rain = Weather::new(WeatherType::HeavyRain);
        let clear = Weather::clear();

        assert!(rain.wetness_per_tick() > 0.0);
        assert_eq!(clear.wetness_per_tick(), 0.0);
    }
}
