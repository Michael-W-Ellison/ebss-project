// src/world/climate.rs
//! Climate management system for the world
//!
//! Integrates biomes, weather, seasons, and temperature

use serde::{Deserialize, Serialize};
use crate::environment::{
    Biome, BiomeType, Weather, WeatherGenerator, Season, SeasonalCalendar,
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
    #[serde(skip)]
    biome_cache: HashMap<Position, Biome>,

    /// Whether world is in cold climate overall
    cold_climate: bool,

    /// Whether world is in wet climate overall
    wet_climate: bool,
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
            cold_climate,
            wet_climate,
        }
    }

    /// Tick the climate system
    pub fn tick(&mut self) {
        // Update calendar
        self.calendar.tick();

        // Update weather generator with current season
        self.weather_gen.season = self.calendar.current_season();

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

            self.biome_cache.insert(pos, biome);
        }

        self.biome_cache.get(&pos).unwrap()
    }

    /// Get effective temperature at a position
    pub fn get_temperature(&mut self, pos: Position, terrain: TerrainType) -> Temperature {
        let biome = self.get_biome(pos, terrain);
        let biome_temp = biome.current_climate.temperature;

        // Apply weather modifier
        let weather_temp = self.weather.effective_temperature(biome_temp);

        weather_temp
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
    pub fn has_shelter_at(&self, pos: Position, terrain: TerrainType) -> bool {
        // For now, only buildings provide shelter
        // In future, this could check for caves, dense forest, etc.
        matches!(terrain, TerrainType::Forest) // Forest provides partial shelter
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
