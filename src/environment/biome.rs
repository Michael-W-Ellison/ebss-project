// src/environment/biome.rs
//! Biome system that determines environmental characteristics
//!
//! Biomes combine terrain type with climate data to create distinct ecological zones.
//! Each biome has its own temperature range, precipitation, and environmental hazards.

use serde::{Deserialize, Serialize};
use crate::agents::temperature::{Temperature, Climate};
use crate::environment::seasons::Season;

/// Biome types representing distinct ecological zones
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
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

/// The hour a day is coldest.
///
/// Just before dawn, which is when the ground has been giving up its heat
/// all night and has not yet begun to take any back. Not midnight, which is
/// what a naive clock would pick.
const WHEN_A_DAY_IS_COLDEST: f32 = 5.0;

/// Hours in a day, so the turn of the clock is written once.
const HOURS_IN_A_DAY: f32 = 24.0;

/// How far through the day's warming a given hour stands: nought at the
/// coldest hour, one twelve hours later.
///
/// A cosine rather than a step, because a day does not jump from night to
/// noon. The warmest hour comes out opposite the coldest, at five in the
/// afternoon; a real day peaks nearer three, and that asymmetry is not
/// modelled.
pub fn how_far_through_the_days_warmth(hour: f32) -> f32 {
    let turned = (hour - WHEN_A_DAY_IS_COLDEST) / HOURS_IN_A_DAY * std::f32::consts::TAU;
    (1.0 - turned.cos()) / 2.0
}

/// What the thermometer does in one place, over a year and over a day.
///
/// **Two bands rather than one range, because one pair of numbers cannot
/// answer two questions.** "How cold does it get here" is about January
/// night and "how hot does it get here" is about July afternoon, and a
/// single (min, max) leaves everything in between to be invented by
/// whatever arithmetic reads it. What was invented was a multiplication -
/// see `temperature_at` - and it put a temperate forest at fourteen degrees
/// at winter noon and made the tundra coldest at midday.
///
/// The figures are the specification's own, biome by biome: "Temperate
/// deciduous forest: Winter -5C to 5C, Summer 20C to 30C, four distinct
/// seasons."
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhatTheYearDoesHere {
    /// The coldest and the warmest a winter day gets: before dawn, and in
    /// the afternoon.
    pub winter: (Temperature, Temperature),
    /// And the same two hours of a summer day.
    pub summer: (Temperature, Temperature),
}

impl BiomeType {
    /// What a year and a day come to here - see [`WhatTheYearDoesHere`].
    ///
    /// The one statement about how warm a place is. Everything else about
    /// temperature is derived from it, so a biome cannot be cold for one
    /// purpose and mild for another.
    pub fn what_the_year_does_here(&self) -> WhatTheYearDoesHere {
        let (winter, summer) = match self {
            // Long very cold winters, very short cool summers.
            BiomeType::Tundra => ((-40.0, -10.0), (0.0, 10.0)),
            // Long cold winters, short mild to warm summers.
            BiomeType::Taiga => ((-30.0, -5.0), (10.0, 20.0)),
            // Four distinct seasons.
            BiomeType::TemperateForest => ((-5.0, 5.0), (20.0, 30.0)),
            // Temperate grassland, prairie and steppe: high seasonal
            // contrast, hot summers and cold winters.
            BiomeType::Grassland => ((-20.0, 5.0), (20.0, 35.0)),
            // Cool season against hot season, and the day-night swing here
            // is the widest of any ground.
            BiomeType::Desert => ((5.0, 20.0), (30.0, 45.0)),
            // Tropical rainforest: little seasonal variation, consistently
            // warm. The two bands very nearly meet, which is the point.
            BiomeType::Tropical => ((20.0, 25.0), (25.0, 32.0)),
            // Savanna: warm year round, and it is the moisture that has a
            // season rather than the temperature.
            BiomeType::Savanna => ((15.0, 25.0), (25.0, 35.0)),
            // Alpine and montane: cold winters, short cool summers.
            BiomeType::Alpine => ((-20.0, 0.0), (5.0, 20.0)),
            // Wetland, marsh and riparian: it tracks the region it is in and
            // the water moderates it. Held to the temperate reading here,
            // which is what this map's wetlands are.
            BiomeType::Wetland => ((0.0, 8.0), (15.0, 30.0)),
            // Temperate marine, whose swings are narrower than anything
            // inland because there is a sea against it.
            BiomeType::Coast => ((5.0, 10.0), (15.0, 20.0)),
        };

        WhatTheYearDoesHere { winter, summer }
    }

    /// The coldest and the warmest this place ever is.
    ///
    /// Derived from the bands rather than written down beside them, so the
    /// two cannot come to disagree - the defect this whole change is about,
    /// in miniature.
    pub fn temperature_range(&self) -> (Temperature, Temperature) {
        let year = self.what_the_year_does_here();
        (year.winter.0, year.summer.1)
    }

    /// What it averages over the year, taking the four corners of the two
    /// bands.
    pub fn average_temperature(&self) -> Temperature {
        let year = self.what_the_year_does_here();
        (year.winter.0 + year.winter.1 + year.summer.0 + year.summer.1) / 4.0
    }

    /// How warm it is here, in this season, at this hour.
    ///
    /// **Additive, and that is the whole of the fix.** What was here
    /// multiplied a Celsius temperature by a season factor and then by a
    /// time-of-day factor. Celsius is an interval scale and not a ratio
    /// scale: there is no sense in which twice as many degrees is twice as
    /// warm, and multiplying by 1.5 for noon makes a cold place colder.
    /// Measured before this: the tundra read -11.7 at two in the morning and
    /// **-25.1 at noon**, and the taiga and the alpine the same way round.
    ///
    /// The second thing it did was flatten the year. The season entered as
    /// `range * 0.3 * (factor - 1.0)` with the factor spanning 0.6 to 1.2,
    /// which is minus a eighth to plus a sixteenth of the range: seasonal
    /// swings of four to ten degrees where the specification asks for twenty
    /// to thirty. A temperate forest read **+14.2 at winter noon** and
    /// nothing outside the three arctic biomes ever froze, which is why
    /// "make winter bite" kept coming back.
    ///
    /// Now the year moves the two ends of the day's band between the winter
    /// pair and the summer pair, and the hour moves the reading between
    /// those two ends. Both are degrees.
    pub fn temperature_at(&self, season: Season, hour: f32) -> Temperature {
        let year = self.what_the_year_does_here();
        let into_the_summer = season.how_far_into_the_year_it_is();

        let by_night = year.winter.0 + (year.summer.0 - year.winter.0) * into_the_summer;
        let by_day = year.winter.1 + (year.summer.1 - year.winter.1) * into_the_summer;

        by_night + (by_day - by_night) * how_far_through_the_days_warmth(hour)
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
    /// The season this biome is in.
    ///
    /// This was an `f32` documented as "0.0 to 4.0, representing
    /// spring/summer/fall/winter", read back with `self.season as u32` and
    /// matched against 0..3. Every test that set it wrote 1.0 or 3.0 and got
    /// what it asked for; the one live caller wrote
    /// `day_of_year / DAYS_PER_YEAR`, which is a fraction under one, which
    /// casts to zero, which is spring. So no world has ever had a winter as
    /// far as its biomes were concerned. A number standing in for one of four
    /// named things is how that happens, so it is one of four named things
    /// now.
    pub season: Season,
}

impl Biome {
    pub fn new(biome_type: BiomeType) -> Self {
        Self {
            biome_type,
            current_climate: biome_type.generate_climate(0.5),
            time_of_day: 12.0,
            season: Season::Fall,
        }
    }

    /// Update climate based on time and season
    pub fn update_climate(&mut self, delta_time: f32) {
        // Update time of day (24-hour cycle)
        self.time_of_day = (self.time_of_day + delta_time) % HOURS_IN_A_DAY;

        // One owner for how warm it is - see `BiomeType::temperature_at`.
        // What was here was a second copy of the season and hour curves,
        // written multiplicatively, and it disagreed with everything else.
        self.current_climate.temperature =
            self.biome_type.temperature_at(self.season, self.time_of_day);
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
        assert!(BiomeType::TemperateForest.average_temperature() > 0.0);
        assert!(BiomeType::TemperateForest.average_temperature() < 20.0);

        // A desert is hot but it is not the warmest place on the map over a
        // year: it has cold nights and a cool season, and a rainforest has
        // neither. The old table said 27.5 for a desert against 27.5 for the
        // tropics by writing one range for each and reading the middle of
        // it; with a winter band and a summer band it comes out 25.0 against
        // 25.5, which is the right way round.
        //
        // Ordering rather than a number, because the number is derived from
        // the bands and a threshold written beside it is a second opinion
        // waiting to disagree.
        assert!(
            BiomeType::Desert.average_temperature()
                > BiomeType::TemperateForest.average_temperature()
        );
        assert!(
            BiomeType::Desert.average_temperature()
                < BiomeType::Tropical.average_temperature()
        );
        assert!(
            BiomeType::Tropical.average_temperature() > 25.0,
            "a rainforest is warm all year and all night"
        );
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
        let initial_temp = biome.current_climate.temperature;

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
        summer_biome.season = Season::Summer;
        summer_biome.time_of_day = 12.0;
        summer_biome.update_climate(0.0);
        let summer_temp = summer_biome.current_climate.temperature;

        let mut winter_biome = Biome::new(BiomeType::Grassland);
        winter_biome.season = Season::Winter;
        winter_biome.time_of_day = 12.0;
        winter_biome.update_climate(0.0);
        let winter_temp = winter_biome.current_climate.temperature;

        assert!(summer_temp > winter_temp);
    }
}
