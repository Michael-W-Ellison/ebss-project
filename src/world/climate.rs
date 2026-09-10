// src/world/climate.rs
//! Climate management system for the world
//!
//! Integrates biomes, weather, seasons, and temperature

use serde::{Deserialize, Serialize};
use crate::environment::{
    seasons, Biome, BiomeType, Weather, WeatherGenerator, Season, SeasonalCalendar,
};
use crate::agents::temperature::{Climate, Temperature};
use crate::world::{Position, TerrainType};
use std::collections::BTreeMap;

/// What ground of a given sort is, in an ordinary temperate country.
///
/// A convenience for the callers that have no world to hand and want the
/// default. Where the country is known - which is everywhere inside a
/// running world - `ClimateManager::biome_of` is the thing to call, because
/// a wood in a boreal country is taiga and this cannot say so.
pub fn terrain_to_biome(terrain: TerrainType) -> BiomeType {
    BiomeType::THE_ORDINARY_SORT_OF_COUNTRY.on_this_ground(terrain)
}

/// Lightning strike event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningStrike {
    pub position: Position,
    pub tick: u32,
    pub caused_fire: bool,
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
    /// The humidity the weather generator works from, and the wind and
    /// temperature a `Climate` needs to exist at all.
    ///
    /// **Its temperature is not the world's temperature.** How warm it is
    /// somewhere depends on what ground it is, which is what `get_biome`
    /// and `BiomeType::temperature_at` are for; this is the humidity that
    /// the whole country's weather is drawn against, and the rest of the
    /// struct comes along with it.
    pub base_climate: Climate,

    /// The biome under each kind of ground, as it stands today.
    ///
    /// A biome is a question about what kind of ground this is and what the
    /// calendar says, and about nothing else - the position never entered the
    /// calculation. Keying it by position meant one entry for every tile
    /// anything had ever asked about, which on a hundred square kilometres is
    /// a hundred and thirty thousand of them and a lookup per resource per
    /// pass; and, worse, it meant the answer was frozen at the hour and the
    /// day it was first asked, because nothing has ever called
    /// `clear_biome_cache`. A wood in a world a year old still had the
    /// temperature of the first morning in it. Only the weather modifier laid
    /// over the top of it moved at all.
    #[serde(skip)]
    biome_today: BTreeMap<BiomeType, Biome>,

    /// What the calendar said when `biome_today` was worked out.
    ///
    /// Two representations of one fact, so they are checked against each
    /// other on every read rather than trusted: when the hour or the day has
    /// moved on, what is cached is thrown away and worked out again.
    #[serde(skip)]
    biome_as_of: Option<(f32, u32)>,


    /// Recent lightning strikes
    pub lightning_strikes: Vec<LightningStrike>,

    /// Current tick for lightning tracking
    pub current_tick: u32,

    /// What kind of country this whole map is.
    ///
    /// **A hundred square kilometres is ten kilometres by ten, and that is
    /// one climate.** A map does not run from tundra to rainforest, so the
    /// biome family is a property of the world and the terrain picks within
    /// it - see `BiomeType::on_this_ground`. Before this, the biome was read
    /// off the terrain alone, which meant every wood on every map was a
    /// temperate deciduous wood: measured, six of the ten biomes and three
    /// of the four climate zones were unreachable on any map ever generated,
    /// and the banana, the coffee bush, the mahogany, the mangrove, the
    /// monkey and the parrot had nowhere at all to be put.
    ///
    /// Only a country can be a region. Asked to be a marsh or a mountain, it
    /// answers with the ordinary sort - see `BiomeType::as_a_country`.
    region: BiomeType,

    /// Whether world is in cold climate overall
    cold_climate: bool,

    /// Whether world is in wet climate overall
    wet_climate: bool,

    /// Dominant biome for weather generation
    dominant_biome: Option<BiomeType>,
}

impl ClimateManager {
    pub fn new(cold_climate: bool, wet_climate: bool) -> Self {
        Self::in_a_country(
            BiomeType::THE_ORDINARY_SORT_OF_COUNTRY,
            cold_climate,
            wet_climate,
        )
    }

    /// The country this map is, and the weather over it.
    pub fn in_a_country(region: BiomeType, cold_climate: bool, wet_climate: bool) -> Self {
        let mut made = Self::the_old_way(cold_climate, wet_climate);
        made.region = region.as_a_country();
        made
    }

    /// What kind of country this is.
    pub fn region(&self) -> BiomeType {
        self.region
    }

    /// What ground of a given sort is, in this country. The one place
    /// terrain becomes a biome.
    pub fn biome_of(&self, terrain: TerrainType) -> BiomeType {
        self.region.on_this_ground(terrain)
    }

    fn the_old_way(cold_climate: bool, wet_climate: bool) -> Self {
        let season = Season::Spring; // Start in spring
        let mut weather_gen = WeatherGenerator::new(
            season,
            wet_climate,
            cold_climate,
        );

        let weather = weather_gen.generate_weather();

        Self {
            calendar: SeasonalCalendar::new(seasons::TICKS_PER_DAY),
            weather,
            weather_gen,
            base_climate: Climate::temperate(), // Default temperate
            biome_today: BTreeMap::new(),
            biome_as_of: None,
            lightning_strikes: Vec::new(),
            current_tick: 0,
            region: BiomeType::THE_ORDINARY_SORT_OF_COUNTRY,
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
            calendar: SeasonalCalendar::new(seasons::TICKS_PER_DAY),
            weather,
            weather_gen,
            base_climate: Climate::temperate(),
            biome_today: BTreeMap::new(),
            biome_as_of: None,
            lightning_strikes: Vec::new(),
            current_tick: 0,
            // The dominant biome is what the weather is drawn against, and
            // it is also what kind of country this is - one answer, not two.
            region: biome.as_a_country(),
            cold_climate,
            wet_climate,
            dominant_biome: Some(biome),
        }
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

        // **The world's temperature is not written down here any more.**
        //
        // What was here was a third answer to how warm it is: two numbers
        // for the whole world - fifteen degrees, or minus five if the world
        // was called cold - multiplied by a season factor and a time-of-day
        // factor. Multiplied, so in a cold world it worked out summer at
        // minus six and winter at minus three, and nothing at all read it:
        // `base_climate.temperature` is written and never looked at, while
        // every question anybody actually asks goes through `get_biome` and
        // `Biome::update_climate`.
        //
        // A temperature with no place attached to it is not a question this
        // model can answer, so it is not answered. `BiomeType::
        // temperature_at` is the one owner - see `base_climate`.

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

        let mut rng = crate::core::dice::roll();
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





    /// Get biome for a specific position
    ///
    /// The position is what kind of ground it is and nothing else, so what
    /// comes back is shared by every tile of that kind - see `biome_today`.
    pub fn get_biome(&mut self, _pos: Position, terrain: TerrainType) -> &Biome {
        let now = (self.calendar.time_of_day, self.calendar.day_of_year);
        if self.biome_as_of != Some(now) {
            self.biome_today.clear();
            self.biome_as_of = Some(now);
        }

        let biome_type = self.region.on_this_ground(terrain);

        if !self.biome_today.contains_key(&biome_type) {
            let mut biome = Biome::new(biome_type);

            // Update biome with current time and season
            biome.time_of_day = self.calendar.time_of_day;
            biome.season = self.calendar.current_season();
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

            self.biome_today.insert(biome_type, biome);
        }

        self.biome_today.get(&biome_type).unwrap()
    }

    /// Get effective temperature at a position
    pub fn get_temperature(&mut self, pos: Position, terrain: TerrainType) -> Temperature {
        let biome = self.get_biome(pos, terrain);
        let biome_temp = biome.current_climate.temperature;

        // Apply weather modifier
        let weather_temp = self.weather.effective_temperature(biome_temp);

        weather_temp
    }

    /// How warm the water itself is here, as against the air over it.
    ///
    /// **What freezes a river is the water, not the air over it.** Both the
    /// water's flow and the fish run were gated on `get_temperature < 0.0`,
    /// which stops a reach the first frosty night: a river carries far more
    /// heat than the air above it and gives it up far more slowly, and a
    /// running one does not ice over because a night was cold. See
    /// `BiomeType::water_temperature_at`.
    pub fn water_temperature(&mut self, pos: Position, terrain: TerrainType) -> Temperature {
        let biome_type = self.region.on_this_ground(terrain);
        let season = self.calendar.current_season();
        let hour = self.calendar.time_of_day;
        let _ = pos;
        biome_type.water_temperature_at(self.region, season, hour)
    }

    /// And whether that water is ice.
    pub fn is_the_water_frozen(&mut self, pos: Position, terrain: TerrainType) -> bool {
        self.water_temperature(pos, terrain) <= 0.0
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

    /// How much meat is on a wild animal at this time of year - see
    /// [`SeasonalCalendar::how_fat_the_beasts_are`].
    pub fn how_fat_the_beasts_are(&self) -> f32 {
        self.calendar.how_fat_the_beasts_are()
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



    /// Clear biome cache (call when world terrain changes)
    ///
    /// The calendar clears it of its own accord every time the hour moves,
    /// so this is only for a change to the ground itself.
    pub fn clear_biome_cache(&mut self) {
        self.biome_today.clear();
        self.biome_as_of = None;
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
        // A lake and a river are fresh water, and they used to be told they
        // were the sea. What is salt is `Sea` and `Beach`.
        assert_eq!(terrain_to_biome(TerrainType::Water), BiomeType::Freshwater);
        assert_eq!(terrain_to_biome(TerrainType::Riverbank), BiomeType::Freshwater);
        assert_eq!(terrain_to_biome(TerrainType::Sea), BiomeType::Coast);
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
        assert!(manager.biome_today.contains_key(&BiomeType::TemperateForest));

        // Clear cache
        manager.clear_biome_cache();
        assert!(manager.biome_today.is_empty());
    }

    /// Two woods a mile apart are the same wood as far as this is concerned.
    ///
    /// What is cached is one entry for each kind of ground, not one for each
    /// tile anybody has ever stood on: asking about a thousand different
    /// patches of forest leaves one thing in the cache.
    #[test]
    fn the_biome_is_a_question_about_ground_not_about_a_coordinate() {
        let mut manager = ClimateManager::new(false, false);

        for x in 0..1000 {
            let _ = manager.get_biome(Position::new(x, 0), TerrainType::Forest);
        }

        assert_eq!(manager.biome_today.len(), 1);
    }

    /// And it is not still the first morning of the world at midwinter.
    ///
    /// Nothing ever called `clear_biome_cache`, so what was worked out on the
    /// first tick anything asked was the answer for ever after - a wood in a
    /// world a year old still had the temperature of the day it was made in.
    #[test]
    fn the_ground_gets_colder_as_the_year_turns() {
        let mut spring = ClimateManager::new(false, false);
        let here = Position::new(5, 5);
        let in_spring = spring.get_temperature(here, TerrainType::Plains);

        // Ask now, so that anything cached is cached; then run on to winter
        // and ask again.
        let mut winter = spring.clone();
        while winter.calendar.current_season() != Season::Winter {
            winter.tick();
        }
        let in_winter = winter.get_temperature(here, TerrainType::Plains);

        assert!(
            in_winter < in_spring,
            "spring {in_spring:.1}, winter {in_winter:.1} - the year turned and the ground did not"
        );
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
