// src/environment/biome.rs
//! Biome system that determines environmental characteristics
//!
//! Biomes combine terrain type with climate data to create distinct ecological zones.
//! Each biome has its own temperature range, precipitation, and environmental hazards.

use serde::{Deserialize, Serialize};
use crate::world::terrain::TerrainType;
use crate::agents::temperature::{Temperature, Climate};

/// Biome types representing distinct ecological zones
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BiomeType {
    /// Cold, snowy regions
    Tundra,
    /// Pine forests in cold climates
    Taiga,
    /// Temperate forests with four seasons
    TemperateForest,
    /// Grasslands with moderate rainfall
    Grassland,
    /// Hot, dry regions with minimal vegetation
    Desert,
    /// Hot, wet regions with dense vegetation
    Tropical,
    /// Dry grasslands with scattered trees
    Savanna,
    /// High altitude mountains
    Alpine,
    /// Swampy wetlands
    Wetland,
    /// Coastal regions
    Coast,
}

impl BiomeType {
    /// Get base temperature range for this biome (min, max in Celsius)
    pub fn temperature_range(&self) -> (Temperature, Temperature) {
        match self {
            BiomeType::Tundra => (-30.0, 5.0),
            BiomeType::Taiga => (-20.0, 15.0),
            BiomeType::TemperateForest => (0.0, 25.0),
            BiomeType::Grassland => (5.0, 30.0),
            BiomeType::Desert => (10.0, 45.0),
            BiomeType::Tropical => (20.0, 35.0),
            BiomeType::Savanna => (15.0, 35.0),
            BiomeType::Alpine => (-20.0, 10.0),
            BiomeType::Wetland => (10.0, 30.0),
            BiomeType::Coast => (5.0, 25.0),
        }
    }

    /// Get average temperature for this biome
    pub fn average_temperature(&self) -> Temperature {
        let (min, max) = self.temperature_range();
        (min + max) / 2.0
    }

    /// Get average humidity (0.0 to 1.0)
    pub fn average_humidity(&self) -> f32 {
        match self {
            BiomeType::Tundra => 0.3,
            BiomeType::Taiga => 0.5,
            BiomeType::TemperateForest => 0.6,
            BiomeType::Grassland => 0.4,
            BiomeType::Desert => 0.1,
            BiomeType::Tropical => 0.9,
            BiomeType::Savanna => 0.3,
            BiomeType::Alpine => 0.4,
            BiomeType::Wetland => 0.9,
            BiomeType::Coast => 0.7,
        }
    }

    /// Get typical wind speed (m/s)
    pub fn typical_wind_speed(&self) -> f32 {
        match self {
            BiomeType::Tundra => 6.0,
            BiomeType::Taiga => 3.0,
            BiomeType::TemperateForest => 2.0,
            BiomeType::Grassland => 4.0,
            BiomeType::Desert => 5.0,
            BiomeType::Tropical => 1.0,
            BiomeType::Savanna => 3.0,
            BiomeType::Alpine => 8.0,
            BiomeType::Wetland => 2.0,
            BiomeType::Coast => 5.0,
        }
    }

    /// Get typical terrain for this biome
    pub fn typical_terrain(&self) -> TerrainType {
        match self {
            BiomeType::Tundra => TerrainType::Plains,
            BiomeType::Taiga => TerrainType::Forest,
            BiomeType::TemperateForest => TerrainType::Forest,
            BiomeType::Grassland => TerrainType::Plains,
            BiomeType::Desert => TerrainType::Plains,
            BiomeType::Tropical => TerrainType::Forest,
            BiomeType::Savanna => TerrainType::Plains,
            BiomeType::Alpine => TerrainType::Mountain,
            BiomeType::Wetland => TerrainType::Water,
            BiomeType::Coast => TerrainType::Plains,
        }
    }

    /// Generate a climate appropriate for this biome
    pub fn generate_climate(&self, variation: f32) -> Climate {
        let (min_temp, max_temp) = self.temperature_range();
        let avg_temp = self.average_temperature();

        // Apply variation to temperature
        let temp = avg_temp + (max_temp - min_temp) * 0.5 * (variation - 0.5);

        Climate {
            temperature: temp,
            humidity: self.average_humidity(),
            wind_speed: self.typical_wind_speed(),
        }
    }

    /// Get exposure risk level (0.0 to 1.0) based on biome characteristics
    pub fn exposure_risk(&self) -> f32 {
        match self {
            BiomeType::Tundra => 0.9,      // Extreme cold
            BiomeType::Taiga => 0.6,       // Moderate cold
            BiomeType::TemperateForest => 0.3,
            BiomeType::Grassland => 0.4,
            BiomeType::Desert => 0.8,      // Extreme heat
            BiomeType::Tropical => 0.5,    // Heat and humidity
            BiomeType::Savanna => 0.5,
            BiomeType::Alpine => 0.9,      // Extreme cold and altitude
            BiomeType::Wetland => 0.6,     // Disease and exposure
            BiomeType::Coast => 0.4,
        }
    }

    /// Get natural shelter availability (0.0 to 1.0)
    pub fn shelter_availability(&self) -> f32 {
        match self {
            BiomeType::Tundra => 0.2,
            BiomeType::Taiga => 0.7,
            BiomeType::TemperateForest => 0.8,
            BiomeType::Grassland => 0.3,
            BiomeType::Desert => 0.2,
            BiomeType::Tropical => 0.7,
            BiomeType::Savanna => 0.4,
            BiomeType::Alpine => 0.3,
            BiomeType::Wetland => 0.4,
            BiomeType::Coast => 0.5,
        }
    }

    /// Get resource abundance (food, water) rating (0.0 to 1.0)
    pub fn resource_abundance(&self) -> f32 {
        match self {
            BiomeType::Tundra => 0.2,
            BiomeType::Taiga => 0.5,
            BiomeType::TemperateForest => 0.8,
            BiomeType::Grassland => 0.6,
            BiomeType::Desert => 0.1,
            BiomeType::Tropical => 0.9,
            BiomeType::Savanna => 0.6,
            BiomeType::Alpine => 0.3,
            BiomeType::Wetland => 0.7,
            BiomeType::Coast => 0.7,
        }
    }
}

/// A biome instance with current environmental state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Biome {
    pub biome_type: BiomeType,
    pub current_climate: Climate,
    /// Time of day (0.0 to 24.0)
    pub time_of_day: f32,
    /// Season (0.0 to 4.0, representing spring/summer/fall/winter)
    pub season: f32,
}

impl Biome {
    pub fn new(biome_type: BiomeType) -> Self {
        Self {
            biome_type,
            current_climate: biome_type.generate_climate(0.5),
            time_of_day: 12.0,
            season: 2.0, // Start in summer
        }
    }

    /// Update climate based on time and season
    pub fn update_climate(&mut self, delta_time: f32) {
        // Update time of day (24-hour cycle)
        self.time_of_day = (self.time_of_day + delta_time) % 24.0;

        // Temperature variation by time of day
        let time_factor = if self.time_of_day >= 6.0 && self.time_of_day <= 18.0 {
            // Daytime (6 AM to 6 PM) - warmer
            let progress = (self.time_of_day - 6.0) / 12.0;
            // Peak at noon
            1.0 + 0.5 * (1.0 - (progress - 0.5).abs() * 2.0)
        } else {
            // Nighttime - cooler
            0.7
        };

        // Temperature variation by season
        let season_factor = match self.season as u32 {
            0 => 0.8,  // Spring - mild
            1 => 1.2,  // Summer - hot
            2 => 0.9,  // Fall - cooling
            3 => 0.6,  // Winter - cold
            _ => 1.0,
        };

        let base_temp = self.biome_type.average_temperature();
        let (min_temp, max_temp) = self.biome_type.temperature_range();
        let temp_range = max_temp - min_temp;

        // Calculate current temperature
        let mut current_temp = base_temp + (temp_range * 0.3 * (season_factor - 1.0));
        current_temp *= time_factor;

        self.current_climate.temperature = current_temp;
    }

    /// Get current effective temperature (with wind chill/heat index)
    pub fn effective_temperature(&self) -> Temperature {
        self.current_climate.effective_temperature()
    }

    /// Check if it's currently nighttime
    pub fn is_night(&self) -> bool {
        self.time_of_day < 6.0 || self.time_of_day > 20.0
    }

    /// Get exposure danger level at current conditions (0.0 to 1.0)
    pub fn current_exposure_danger(&self) -> f32 {
        let base_risk = self.biome_type.exposure_risk();

        // Night increases risk
        let night_multiplier = if self.is_night() { 1.3 } else { 1.0 };

        // Extreme temperatures increase risk
        let temp = self.effective_temperature();
        let temp_risk = if temp < 0.0 {
            (-temp / 30.0).min(1.0) // Cold risk
        } else if temp > 35.0 {
            ((temp - 35.0) / 15.0).min(1.0) // Heat risk
        } else {
            0.0
        };

        ((base_risk + temp_risk) * night_multiplier).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biome_temperature_ranges() {
        assert!(BiomeType::Tundra.average_temperature() < 0.0);
        assert!(BiomeType::Desert.average_temperature() > 25.0);
        assert!(BiomeType::TemperateForest.average_temperature() > 0.0);
        assert!(BiomeType::TemperateForest.average_temperature() < 20.0);
    }

    #[test]
    fn test_biome_humidity() {
        assert!(BiomeType::Desert.average_humidity() < 0.2);
        assert!(BiomeType::Tropical.average_humidity() > 0.8);
        assert!(BiomeType::Wetland.average_humidity() > 0.8);
    }

    #[test]
    fn test_biome_exposure_risk() {
        assert!(BiomeType::Tundra.exposure_risk() > 0.7);
        assert!(BiomeType::Desert.exposure_risk() > 0.7);
        assert!(BiomeType::TemperateForest.exposure_risk() < 0.5);
    }

    #[test]
    fn test_biome_shelter() {
        assert!(BiomeType::TemperateForest.shelter_availability() > 0.7);
        assert!(BiomeType::Desert.shelter_availability() < 0.3);
    }

    #[test]
    fn test_biome_resources() {
        assert!(BiomeType::Tropical.resource_abundance() > 0.8);
        assert!(BiomeType::Desert.resource_abundance() < 0.2);
    }

    #[test]
    fn test_biome_climate_generation() {
        let desert = BiomeType::Desert;
        let climate = desert.generate_climate(0.5);

        assert!(climate.temperature > 20.0);
        assert!(climate.humidity < 0.3);
    }

    #[test]
    fn test_biome_time_of_day() {
        let mut biome = Biome::new(BiomeType::TemperateForest);
        biome.time_of_day = 2.0; // 2 AM

        assert!(biome.is_night());

        biome.time_of_day = 14.0; // 2 PM
        assert!(!biome.is_night());
    }

    #[test]
    fn test_climate_update() {
        let mut biome = Biome::new(BiomeType::Grassland);
        let _initial_temp = biome.current_climate.temperature;

        // Move to nighttime
        biome.time_of_day = 2.0;
        biome.update_climate(0.0);

        let night_temp = biome.current_climate.temperature;

        // Move to daytime
        biome.time_of_day = 14.0;
        biome.update_climate(0.0);

        let day_temp = biome.current_climate.temperature;

        // Day should be warmer than night
        assert!(day_temp > night_temp);
    }

    #[test]
    fn test_exposure_danger() {
        let mut biome = Biome::new(BiomeType::Tundra);
        biome.current_climate.temperature = -25.0;

        let danger = biome.current_exposure_danger();
        assert!(danger > 0.5); // Very dangerous in extreme cold
    }

    #[test]
    fn test_seasonal_variation() {
        let mut summer_biome = Biome::new(BiomeType::Grassland);
        summer_biome.season = 1.0; // Summer
        summer_biome.time_of_day = 12.0;
        summer_biome.update_climate(0.0);
        let summer_temp = summer_biome.current_climate.temperature;

        let mut winter_biome = Biome::new(BiomeType::Grassland);
        winter_biome.season = 3.0; // Winter
        winter_biome.time_of_day = 12.0;
        winter_biome.update_climate(0.0);
        let winter_temp = winter_biome.current_climate.temperature;

        assert!(summer_temp > winter_temp);
    }
}
