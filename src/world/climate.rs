// src/world/climate.rs
//! Climate management system for the world
//!
//! Integrates biomes, weather, seasons, and temperature

use serde::{Deserialize, Serialize};
use crate::environment::{
    Biome, BiomeType, Weather, WeatherGenerator, WeatherType, Season, SeasonalCalendar,
};
use crate::agents::temperature::{Climate, Temperature};
use crate::world::{Position, TerrainType};
use std::collections::HashMap;

/// Maps terrain types to biome types
pub fn terrain_to_biome(terrain: TerrainType) -> BiomeType {
    match terrain {
        TerrainType::Plains => BiomeType::Grassland,
        TerrainType::Forest => BiomeType::TemperateForest,
        TerrainType::Mountain => BiomeType::Alpine,
        TerrainType::Water => BiomeType::Coast,
        TerrainType::Desert => BiomeType::Desert,
        TerrainType::Wetland => BiomeType::Wetland,
        TerrainType::Meadow => BiomeType::Grassland,
        TerrainType::Hills => BiomeType::Grassland,
        TerrainType::Beach => BiomeType::Coast,
        TerrainType::Riverbank => BiomeType::Wetland,
    }
}

/// Lightning strike event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningStrike {
    pub position: Position,
    pub tick: u32,
    pub caused_fire: bool,
}

/// Precipitation accumulation at a position
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrecipitationAccumulation {
    /// Snow depth in cm
    pub snow_depth: f32,
    /// Water accumulation (rain pooling)
    pub water_level: f32,
    /// Ground wetness (0.0-1.0)
    pub ground_wetness: f32,
}

impl PrecipitationAccumulation {
    /// Tick precipitation accumulation based on weather
    pub fn tick(&mut self, weather: &Weather, temperature: f32) {
        let intensity = weather.weather_type.precipitation_intensity();

        match weather.weather_type {
            WeatherType::LightSnow | WeatherType::Snow | WeatherType::Blizzard => {
                // Snow accumulates if cold enough
                if temperature < 0.0 {
                    self.snow_depth += intensity * 0.5;
                } else {
                    // Snow melts
                    self.snow_depth = (self.snow_depth - 0.1).max(0.0);
                    self.water_level += self.snow_depth.min(0.1) * 0.5;
                }
            }
            WeatherType::LightRain | WeatherType::Rain | WeatherType::HeavyRain
            | WeatherType::Thunderstorm | WeatherType::Sleet => {
                // Rain increases water and wetness
                self.water_level += intensity * 0.2;
                self.ground_wetness = (self.ground_wetness + intensity * 0.1).min(1.0);

                // Rain melts snow faster
                if self.snow_depth > 0.0 {
                    self.snow_depth = (self.snow_depth - intensity * 0.3).max(0.0);
                }
            }
            WeatherType::Hail => {
                // Hail adds water but less than rain
                self.water_level += intensity * 0.1;
            }
            _ => {
                // Non-precipitation weather: evaporation and drying
                self.water_level = (self.water_level - 0.02).max(0.0);
                self.ground_wetness = (self.ground_wetness - 0.01).max(0.0);

                // Snow sublimation in dry conditions
                if temperature > 5.0 {
                    self.snow_depth = (self.snow_depth - 0.05).max(0.0);
                }
            }
        }

        // Cap accumulation
        self.snow_depth = self.snow_depth.min(200.0); // 2 meters max
        self.water_level = self.water_level.min(50.0); // Prevent infinite flooding
    }

    /// Check if area is flooded
    pub fn is_flooded(&self) -> bool {
        self.water_level > 10.0
    }

    /// Get movement penalty from accumulation
    pub fn movement_penalty(&self) -> f32 {
        let snow_penalty = (self.snow_depth / 50.0).min(0.3);
        let water_penalty = (self.water_level / 20.0).min(0.2);
        let mud_penalty = self.ground_wetness * 0.1;

        (1.0 - snow_penalty - water_penalty - mud_penalty).max(0.3)
    }
}

/// Climate manager for the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateManager {
    /// Seasonal calendar
    pub calendar: SeasonalCalendar,

    /// Global weather
    pub weather: Weather,

    /// Weather generator
    weather_gen: WeatherGenerator,

    /// Base climate for the world (influences all biomes)
    pub base_climate: Climate,

    /// Biome data per position (cached for performance)
    #[serde(skip, default)]
    biome_cache: HashMap<Position, Biome>,

    /// Precipitation accumulation per region (chunked for performance)
    #[serde(skip, default)]
    precipitation_map: HashMap<(i32, i32), PrecipitationAccumulation>,

    /// Recent lightning strikes
    pub lightning_strikes: Vec<LightningStrike>,

    /// Current tick for lightning tracking
    pub current_tick: u32,

    /// Whether world is in cold climate overall
    cold_climate: bool,

    /// Whether world is in wet climate overall
    wet_climate: bool,

    /// Dominant biome for weather generation
    dominant_biome: Option<BiomeType>,
}

impl ClimateManager {
    pub fn new(cold_climate: bool, wet_climate: bool) -> Self {
        let season = Season::Spring; // Start in spring
        let mut weather_gen = WeatherGenerator::new(
            season,
            wet_climate,
            cold_climate,
        );

        let weather = weather_gen.generate_weather();

        Self {
            calendar: SeasonalCalendar::new(100), // 100 ticks per hour
            weather,
            weather_gen,
            base_climate: Climate::temperate(), // Default temperate
            biome_cache: HashMap::new(),
            precipitation_map: HashMap::new(),
            lightning_strikes: Vec::new(),
            current_tick: 0,
            cold_climate,
            wet_climate,
            dominant_biome: None,
        }
    }

    /// Create with a specific dominant biome
    pub fn with_biome(cold_climate: bool, wet_climate: bool, biome: BiomeType) -> Self {
        let season = Season::Spring;
        let mut weather_gen = WeatherGenerator::with_biome(
            season,
            wet_climate,
            cold_climate,
            biome,
        );

        let weather = weather_gen.generate_weather();

        Self {
            calendar: SeasonalCalendar::new(100),
            weather,
            weather_gen,
            base_climate: Climate::temperate(),
            biome_cache: HashMap::new(),
            precipitation_map: HashMap::new(),
            lightning_strikes: Vec::new(),
            current_tick: 0,
            cold_climate,
            wet_climate,
            dominant_biome: Some(biome),
        }
    }

    /// Set dominant biome for weather generation
    pub fn set_dominant_biome(&mut self, biome: BiomeType) {
        self.dominant_biome = Some(biome);
        self.weather_gen.set_biome(biome);
    }

    /// Rebuild caches after deserialization.
    ///
    /// Clears biome and precipitation caches so they can be regenerated
    /// on demand. This is necessary because caches are not serialized.
    pub fn rebuild_caches(&mut self) {
        self.biome_cache.clear();
        self.precipitation_map.clear();
    }

    /// Tick the climate system
    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Update calendar
        self.calendar.tick();

        // Update weather generator with current season and humidity
        self.weather_gen.season = self.calendar.current_season();
        self.weather_gen.set_humidity(self.base_climate.humidity);
        if let Some(biome) = self.dominant_biome {
            self.weather_gen.set_biome(biome);
        }

        // Update weather
        self.weather.tick();

        // Generate new weather when current one expires
        if self.weather.duration_remaining == 0 {
            self.weather = self.weather_gen.generate_weather();
        }

        // Update base climate temperature based on season
        let base_temp = if self.cold_climate { -5.0 } else { 15.0 };
        let season_mod = self.calendar.current_season().temperature_modifier();
        let time_mod = self.calendar.time_of_day_temperature_modifier();

        self.base_climate.temperature = base_temp * season_mod * time_mod;

        // Update humidity based on weather
        if self.weather.weather_type.precipitation_intensity() > 0.0 {
            self.base_climate.humidity = (self.base_climate.humidity + 0.01).min(1.0);
        } else {
            self.base_climate.humidity = (self.base_climate.humidity - 0.005).max(0.2);
        }

        // Process lightning during thunderstorms
        self.process_lightning();

        // Clean up old lightning strikes (older than 100 ticks)
        self.lightning_strikes.retain(|strike| {
            self.current_tick.saturating_sub(strike.tick) < 100
        });
    }

    /// Process potential lightning strikes during thunderstorms
    fn process_lightning(&mut self) {
        use rand::Rng;

        if !self.weather.weather_type.can_cause_lightning() {
            return;
        }

        let mut rng = rand::thread_rng();
        let chance = self.weather.weather_type.lightning_chance_per_tick();

        if rng.gen::<f32>() < chance {
            // Generate a lightning strike at a random position
            // In a real implementation, this would use world size
            let x = rng.gen_range(-100..100);
            let y = rng.gen_range(-100..100);

            // Fire chance depends on ground wetness (wet = less fire)
            let fire_chance = 0.15; // 15% base chance
            let caused_fire = rng.gen::<f32>() < fire_chance;

            self.lightning_strikes.push(LightningStrike {
                position: Position::new(x, y),
                tick: self.current_tick,
                caused_fire,
            });
        }
    }

    /// Get precipitation accumulation at a position (chunked by 10x10 regions)
    pub fn get_precipitation_at(&mut self, pos: Position) -> &PrecipitationAccumulation {
        let chunk = (pos.x / 10, pos.y / 10);
        self.precipitation_map.entry(chunk).or_default()
    }

    /// Update precipitation accumulation at a position
    pub fn update_precipitation_at(&mut self, pos: Position, temperature: f32) {
        let chunk = (pos.x / 10, pos.y / 10);
        let accumulation = self.precipitation_map.entry(chunk).or_default();
        accumulation.tick(&self.weather, temperature);
    }

    /// Get weather forecast
    pub fn get_forecast(&self, hours_ahead: u32) -> (WeatherType, f32) {
        self.weather_gen.forecast(hours_ahead)
    }

    /// Check if there was a recent lightning strike near a position
    pub fn recent_lightning_near(&self, pos: Position, radius: i32) -> Option<&LightningStrike> {
        self.lightning_strikes.iter().find(|strike| {
            let dx = (strike.position.x - pos.x).abs();
            let dy = (strike.position.y - pos.y).abs();
            dx <= radius && dy <= radius
        })
    }

    /// Get biome for a specific position
    pub fn get_biome(&mut self, pos: Position, terrain: TerrainType) -> &Biome {
        if !self.biome_cache.contains_key(&pos) {
            let biome_type = terrain_to_biome(terrain);
            let mut biome = Biome::new(biome_type);

            // Update biome with current time and season
            biome.time_of_day = self.calendar.time_of_day;
            biome.season = self.calendar.day_of_year as f32 / 365.0;
            biome.update_climate(0.0); // Initial update

            // Apply climate modifiers AFTER update_climate (which overwrites temperature)
            if self.cold_climate {
                // Reduce temperature by 15°C for cold climates
                biome.current_climate.temperature -= 15.0;
            }
            if self.wet_climate {
                // Increase humidity for wet climates
                biome.current_climate.humidity = (biome.current_climate.humidity + 0.3).min(1.0);
            }

            self.biome_cache.insert(pos, biome);
        }

        self.biome_cache.get(&pos).unwrap()
    }

    /// Get effective temperature at a position
    pub fn get_temperature(&mut self, pos: Position, terrain: TerrainType) -> Temperature {
        let biome = self.get_biome(pos, terrain);
        let biome_temp = biome.current_climate.temperature;

        // Apply weather modifier
        self.weather.effective_temperature(biome_temp)
    }

    /// Get climate for a position (combines biome climate with weather)
    pub fn get_climate(&mut self, pos: Position, terrain: TerrainType) -> Climate {
        let mut climate = self.get_biome(pos, terrain).current_climate.clone();

        // Apply weather effects
        climate.temperature = self.weather.effective_temperature(climate.temperature);
        climate.wind_speed = self.weather.effective_wind_speed();
        climate.humidity += self.weather.weather_type.precipitation_intensity();

        climate
    }

    /// Check if it's currently daytime
    pub fn is_daytime(&self) -> bool {
        self.calendar.is_daytime()
    }

    /// Get sun intensity (0.0 to 1.0)
    pub fn sun_intensity(&self) -> f32 {
        self.calendar.sun_intensity()
    }

    /// Get current season
    pub fn current_season(&self) -> Season {
        self.calendar.current_season()
    }

    /// Get formatted date/time string
    pub fn date_time_string(&self) -> String {
        format!(
            "{} | Weather: {:?}",
            self.calendar.date_string(),
            self.weather.weather_type
        )
    }

    /// Get visibility range (affected by weather)
    pub fn visibility_range(&self) -> u32 {
        let base_visibility = if self.is_daytime() { 20 } else { 5 };
        let weather_reduction = self.weather.visibility_reduction();

        ((base_visibility as f32) * (1.0 - weather_reduction)).max(2.0) as u32
    }

    /// Get movement speed modifier (affected by weather)
    pub fn movement_modifier(&self) -> f32 {
        self.weather.movement_modifier()
    }

    /// Check if shelter is available at a position
    ///
    /// Shelter can be provided by:
    /// - Forest: Dense tree cover provides moderate protection from elements
    /// - Mountain: Natural cave formations and rocky overhangs
    /// - Hills: Rocky outcrops can provide limited shelter
    /// - Buildings at the position (checked by caller via World)
    pub fn has_shelter_at(&self, _pos: Position, terrain: TerrainType) -> bool {
        match terrain {
            TerrainType::Forest => true,   // Dense tree cover provides good shelter
            TerrainType::Mountain => true, // Caves and overhangs in mountainous terrain
            TerrainType::Hills => true,    // Rocky outcrops provide some shelter
            _ => false,                    // Other terrains need constructed shelter
        }
    }

    /// Get the shelter quality at a position (0.0 = no shelter, 1.0 = full shelter)
    ///
    /// This affects how well the agent is protected from weather effects.
    pub fn shelter_quality(&self, terrain: TerrainType, has_building: bool) -> f32 {
        if has_building {
            return 1.0; // Buildings provide full shelter
        }

        match terrain {
            TerrainType::Mountain => 0.8, // Caves provide excellent natural shelter
            TerrainType::Forest => 0.6,   // Trees provide moderate shelter
            TerrainType::Hills => 0.4,    // Outcrops provide limited shelter
            _ => 0.0,                     // No natural shelter
        }
    }

    /// Clear biome cache (call when world terrain changes)
    pub fn clear_biome_cache(&mut self) {
        self.biome_cache.clear();
    }
}

impl Default for ClimateManager {
    fn default() -> Self {
        Self::new(false, false) // Temperate, not too wet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_to_biome() {
        assert_eq!(terrain_to_biome(TerrainType::Plains), BiomeType::Grassland);
        assert_eq!(terrain_to_biome(TerrainType::Forest), BiomeType::TemperateForest);
        assert_eq!(terrain_to_biome(TerrainType::Mountain), BiomeType::Alpine);
        assert_eq!(terrain_to_biome(TerrainType::Water), BiomeType::Coast);
    }

    #[test]
    fn test_climate_manager_creation() {
        let manager = ClimateManager::new(false, false);
        assert_eq!(manager.calendar.year, 0);
        assert!(!manager.cold_climate);
        assert!(!manager.wet_climate);
    }

    #[test]
    fn test_climate_manager_tick() {
        let mut manager = ClimateManager::new(false, false);
        let initial_time = manager.calendar.time_of_day;

        // Tick 100 times (one hour)
        for _ in 0..100 {
            manager.tick();
        }

        assert!(manager.calendar.time_of_day > initial_time);
    }

    #[test]
    fn test_get_temperature() {
        let mut manager = ClimateManager::new(false, false);
        let pos = Position::new(10, 10);

        let temp = manager.get_temperature(pos, TerrainType::Plains);
        assert!(temp > -50.0 && temp < 50.0); // Reasonable temperature range
    }

    #[test]
    fn test_get_climate() {
        let mut manager = ClimateManager::new(false, false);
        let pos = Position::new(10, 10);

        let climate = manager.get_climate(pos, TerrainType::Forest);
        assert!(climate.temperature.is_finite());
        assert!(climate.wind_speed >= 0.0);
        assert!(climate.humidity >= 0.0);
    }

    #[test]
    fn test_visibility_range() {
        let manager = ClimateManager::new(false, false);
        let visibility = manager.visibility_range();

        assert!(visibility >= 2); // Minimum visibility
        assert!(visibility <= 20); // Maximum visibility (daytime, clear)
    }

    #[test]
    fn test_daytime_check() {
        let mut manager = ClimateManager::new(false, false);
        manager.calendar.time_of_day = 12.0; // Noon

        assert!(manager.is_daytime());

        manager.calendar.time_of_day = 2.0; // 2 AM
        assert!(!manager.is_daytime());
    }

    #[test]
    fn test_cold_climate() {
        let mut manager = ClimateManager::new(true, false);
        let pos = Position::new(0, 0);

        let temp = manager.get_temperature(pos, TerrainType::Plains);
        assert!(temp < 10.0); // Should be cold
    }

    #[test]
    fn test_biome_caching() {
        let mut manager = ClimateManager::new(false, false);
        let pos = Position::new(5, 5);

        // First access creates cache entry
        let _ = manager.get_biome(pos, TerrainType::Forest);
        assert!(manager.biome_cache.contains_key(&pos));

        // Clear cache
        manager.clear_biome_cache();
        assert!(manager.biome_cache.is_empty());
    }

    #[test]
    fn test_date_time_string() {
        let manager = ClimateManager::new(false, false);
        let date_str = manager.date_time_string();

        assert!(date_str.contains("Year"));
        assert!(date_str.contains("Weather"));
    }

    #[test]
    fn test_movement_modifier() {
        let manager = ClimateManager::new(false, false);
        let modifier = manager.movement_modifier();

        assert!(modifier > 0.0);
        assert!(modifier <= 1.0);
    }
}
