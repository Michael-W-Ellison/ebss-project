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
    // --- the ten a country can be -----------------------------------------
    //
    // A hundred square kilometres is ten kilometres by ten, and that is one
    // climate. A map does not run from tundra to rainforest; it is one of
    // these, and the four below are what its own ground does to it. Which
    // one a country is is `ClimateManager::region`.
    /// Cold, snowy regions: tundra and the polar and subpolar country.
    Tundra,
    /// Boreal forest. Pine, long cold winters, short warm summers.
    Taiga,
    /// Temperate deciduous forest, with four distinct seasons.
    TemperateForest,
    /// Temperate conifer forest: cool to cold winters, mild to warm summers,
    /// and a narrower year than the deciduous wood beside it.
    TemperateConifer,
    /// Temperate grassland, prairie and steppe. The hardest year on any map:
    /// cold winters and hot summers both.
    Grassland,
    /// Mediterranean shrubland and chaparral: mild wet winters, hot dry
    /// summers.
    Mediterranean,
    /// Dry grasslands with scattered trees. Warm the year round, and it is
    /// the moisture that has a season rather than the temperature.
    Savanna,
    /// Tropical rainforest. Consistently warm, and next to no year at all.
    Tropical,
    /// Tropical seasonal and dry forest: warm year round, with a stronger
    /// wet and dry season than the rainforest has.
    TropicalDryForest,
    /// Hot, dry regions with minimal vegetation, and the widest day on any
    /// map.
    Desert,

    // --- and the four that are what the ground does to a country ----------
    //
    // These read their year off the country they are in - see
    // `what_the_year_does_here_in`. A marsh in a boreal country is not a
    // marsh in a temperate one, and the specification says so in as many
    // words: "Wetlands in tundra, tropics, or deserts should inherit those
    // broader biome patterns."
    /// High ground, above where the country's own trees stop.
    Alpine,
    /// Swampy wetlands, marshes and riparian ground.
    Wetland,
    /// Lakes and rivers: the air over them, moderated by the water under.
    Freshwater,
    /// The coast and the sea, whose year is the narrowest there is.
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
        self.what_the_year_does_here_in(*self)
    }

    /// What a year and a day come to on ground of this sort, **in a country
    /// of this kind**.
    ///
    /// The one owner. A wood, a steppe or a desert is a country in its own
    /// right and reads its own band; a mountain, a marsh, a river and the
    /// sea are not places on the map so much as things the ground does to
    /// wherever it is, so they read the country's band and bend it. The
    /// specification asks for exactly that - "Wetlands in tundra, tropics, or
    /// deserts should inherit those broader biome patterns", "Freshwater ...
    /// air temperature depends on surrounding biome" - and it is the only way
    /// fourteen categories come out of ten regions without fourteen tables to
    /// keep in step.
    pub fn what_the_year_does_here_in(&self, region: BiomeType) -> WhatTheYearDoesHere {
        // A marsh is not a country, so a marsh asked what country it is in
        // gets the default one rather than itself - which is also what stops
        // this recursing for ever.
        let region = region.as_a_country();

        // The four that answer to the country rather than to themselves.
        match self {
            BiomeType::Alpine => return region.up_a_mountain(),
            BiomeType::Wetland => return region.steadied_by(Self::WHAT_A_MARSH_STEADIES),
            BiomeType::Freshwater => return region.steadied_by(Self::WHAT_A_LAKE_STEADIES),
            BiomeType::Coast => return region.out_at_sea(),
            _ => {}
        }

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
            // Temperate conifer: cool to cold winters, mild to warm
            // summers, and a narrower year than the deciduous wood.
            BiomeType::TemperateConifer => ((-10.0, 5.0), (10.0, 25.0)),
            // Mediterranean shrubland: mild wetter winters, hot dry summers.
            BiomeType::Mediterranean => ((5.0, 15.0), (25.0, 35.0)),
            // Tropical seasonal and dry forest.
            BiomeType::TropicalDryForest => ((20.0, 28.0), (28.0, 35.0)),

            // Handled above, off the country they are in. Written out rather
            // than left to a wildcard so that a new region cannot be added
            // without this arm being looked at.
            BiomeType::Alpine
            | BiomeType::Wetland
            | BiomeType::Freshwater
            | BiomeType::Coast => unreachable!("the ground kinds read their country"),
        };

        WhatTheYearDoesHere { winter, summer }
    }

    /// What kind of country a map is when nobody has said.
    ///
    /// Temperate deciduous, which is what every world this project has ever
    /// measured has been, and what the terrain tables were written for.
    pub const THE_ORDINARY_SORT_OF_COUNTRY: BiomeType = BiomeType::TemperateForest;

    /// Whether this is a kind of country, or a kind of ground that takes its
    /// year from whatever country it is in.
    pub fn is_a_country(&self) -> bool {
        !matches!(
            self,
            BiomeType::Alpine | BiomeType::Wetland | BiomeType::Freshwater | BiomeType::Coast
        )
    }

    /// This, if it is a country; the ordinary sort if it is not.
    ///
    /// A mountain is not a climate, it is a height; a marsh is not a
    /// climate, it is wet ground. Asked which country they are, they answer
    /// for the country they are ordinarily in.
    pub fn as_a_country(self) -> BiomeType {
        if self.is_a_country() {
            self
        } else {
            Self::THE_ORDINARY_SORT_OF_COUNTRY
        }
    }

    /// How much of a country's swing standing water takes out of it.
    const WHAT_A_MARSH_STEADIES: f32 = 0.25;

    /// And a lake or a river, which holds more heat than a marsh does.
    const WHAT_A_LAKE_STEADIES: f32 = 0.35;

    /// And the sea, which is the steadiest thing there is.
    const WHAT_THE_SEA_STEADIES: f32 = 0.65;

    /// What a country's year looks like with water standing in it: the same
    /// year, pulled in towards its own average.
    ///
    /// Water takes a long time to warm and a long time to cool, so ground
    /// with water in it has a shorter year and a shorter day than the
    /// country around it. One rule, one number per kind of water, rather
    /// than a table of bands that could drift away from the country's.
    fn steadied_by(&self, how_much: f32) -> WhatTheYearDoesHere {
        let country = self.as_a_country();
        let year = country.what_the_year_does_here_in(country);
        let settled =
            (year.winter.0 + year.winter.1 + year.summer.0 + year.summer.1) / 4.0;
        let pull = |t: f32| t + (settled - t) * how_much;

        WhatTheYearDoesHere {
            winter: (pull(year.winter.0), pull(year.winter.1)),
            summer: (pull(year.summer.0), pull(year.summer.1)),
        }
    }

    /// The coldest the open sea gets before it is ice, and the warmest it
    /// gets at all.
    ///
    /// Salt water freezes near minus two, and no sea on this earth runs much
    /// above thirty. They are the reason a polar coast reads warmer than the
    /// tundra behind it: "Polar marine -2C to 5C" while the land is at minus
    /// forty. The clamp is what makes the specification's three marine
    /// readings fall out of the three kinds of country rather than being
    /// written down three times.
    const THE_COLDEST_THE_SEA_GETS: f32 = -2.0;
    const THE_WARMEST_THE_SEA_GETS: f32 = 30.0;

    fn out_at_sea(&self) -> WhatTheYearDoesHere {
        let year = self.steadied_by(Self::WHAT_THE_SEA_STEADIES);
        let hold = |t: f32| t.clamp(Self::THE_COLDEST_THE_SEA_GETS, Self::THE_WARMEST_THE_SEA_GETS);

        WhatTheYearDoesHere {
            winter: (hold(year.winter.0), hold(year.winter.1)),
            summer: (hold(year.summer.0), hold(year.summer.1)),
        }
    }

    /// What height takes off a country's thermometer.
    ///
    /// The lapse rate, near enough: six and a half degrees a kilometre, and
    /// mountain ground on this map stands a couple of kilometres above the
    /// valley it looks down on. It is the same subtraction in every season,
    /// which is what makes an alpine year the country's year moved bodily
    /// down rather than a different year.
    const WHAT_HEIGHT_TAKES_OFF: f32 = 13.0;

    fn up_a_mountain(&self) -> WhatTheYearDoesHere {
        let country = self.as_a_country();
        let year = country.what_the_year_does_here_in(country);
        let up = |t: f32| t - Self::WHAT_HEIGHT_TAKES_OFF;

        WhatTheYearDoesHere {
            winter: (up(year.winter.0), up(year.winter.1)),
            summer: (up(year.summer.0), up(year.summer.1)),
        }
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

    /// What ground of this sort is, in a country of this kind.
    ///
    /// **The one place terrain becomes a biome.** There used to be two
    /// functions keyed on terrain alone - `terrain_to_biome` and
    /// `terrain_to_climate_zone` - which is one question answered twice and
    /// is the defect this project keeps finding. Worse, keying on terrain
    /// alone meant a wood was a temperate wood wherever it stood: measured,
    /// **six of ten biomes and three of four climate zones were reachable on
    /// any map at all**, and the banana, the coffee bush, the mahogany, the
    /// mangrove, the monkey and the parrot could never be placed anywhere.
    ///
    /// A country is a kind and the ground picks within it. Woodland and open
    /// ground are the country itself; the rest is what the ground does to
    /// it.
    pub fn on_this_ground(&self, terrain: crate::world::TerrainType) -> BiomeType {
        use crate::world::TerrainType as T;

        let country = self.as_a_country();
        match terrain {
            // Above the tree line, wherever the tree line is.
            T::Mountain => BiomeType::Alpine,
            // Fresh water: a lake, a river, and the bank you stand on to
            // fish it.
            T::Water | T::Riverbank => BiomeType::Freshwater,
            // Salt water and the strand beside it.
            T::Sea | T::Beach => BiomeType::Coast,
            // Wet ground that is not open water.
            T::Wetland | T::SaltMarsh => BiomeType::Wetland,
            // Ground too dry for the country it is in - a rain shadow, or
            // where a shallow sea dried up.
            T::Desert | T::SaltFlat => BiomeType::Desert,
            // And the country itself, standing timber or open.
            T::Forest => country.where_its_trees_are(),
            T::Plains | T::Meadow | T::Hills | T::Farmland => country.where_its_open_ground_is(),
        }
    }

    /// What woodland is, in a country of this kind.
    ///
    /// A wood is a wood everywhere, but a wood at the edge of the tundra is
    /// taiga and a wood in the tropics is rainforest, and they are not the
    /// same year at all. The country names its own timber.
    pub fn where_its_trees_are(&self) -> BiomeType {
        match self.as_a_country() {
            // The treeline's edge: what trees there are are boreal.
            BiomeType::Tundra | BiomeType::Taiga => BiomeType::Taiga,
            // A wood in a steppe is a temperate wood.
            BiomeType::TemperateForest | BiomeType::Grassland => BiomeType::TemperateForest,
            BiomeType::TemperateConifer => BiomeType::TemperateConifer,
            // Dry woodland, whether it is chaparral or an oasis.
            BiomeType::Mediterranean | BiomeType::Desert => BiomeType::Mediterranean,
            BiomeType::Tropical => BiomeType::Tropical,
            BiomeType::Savanna | BiomeType::TropicalDryForest => BiomeType::TropicalDryForest,
            other => other,
        }
    }

    /// And what open ground is.
    ///
    /// **The mistake worth writing down: the first cut mapped open ground to
    /// the country itself, so a plain in a deciduous country came out a
    /// deciduous forest.** A country's kind names its climate, not what is
    /// standing on any particular field. Open ground in a temperate country
    /// is grassland, in a polar country it is tundra, and in the tropics it
    /// is savanna.
    pub fn where_its_open_ground_is(&self) -> BiomeType {
        match self.as_a_country() {
            BiomeType::Tundra => BiomeType::Tundra,
            // A boreal clearing is still boreal; there is no band for it of
            // its own and inventing one would be a number to keep in step.
            BiomeType::Taiga => BiomeType::Taiga,
            BiomeType::TemperateForest
            | BiomeType::TemperateConifer
            | BiomeType::Grassland => BiomeType::Grassland,
            BiomeType::Mediterranean => BiomeType::Mediterranean,
            BiomeType::Savanna | BiomeType::Tropical | BiomeType::TropicalDryForest => {
                BiomeType::Savanna
            }
            BiomeType::Desert => BiomeType::Desert,
            other => other,
        }
    }

    /// Which of the four coarse zones this is, for the plants and the beasts
    /// that are written down against zones rather than biomes.
    ///
    /// **Derived, not a second table.** `terrain_to_climate_zone` used to be
    /// its own match on terrain, and the two answers were only accidentally
    /// consistent: a mountain was `Alpine` to the thermometer and `Arctic` to
    /// the fauna, a sea was `Coast` and `Temperate`, a marsh was `Wetland`
    /// and `Temperate`. They agreed on every terrain by luck rather than by
    /// construction, and `a_zone_is_what_its_biome_says` proves that this
    /// derivation reproduces the old table exactly.
    pub fn climate_zone(&self) -> crate::environment::flora::ClimateZone {
        use crate::environment::flora::ClimateZone as Z;

        match self {
            BiomeType::Tundra | BiomeType::Taiga | BiomeType::Alpine => Z::Arctic,
            BiomeType::Desert => Z::Desert,
            BiomeType::Tropical | BiomeType::TropicalDryForest | BiomeType::Savanna => Z::Tropical,
            BiomeType::TemperateForest
            | BiomeType::TemperateConifer
            | BiomeType::Grassland
            // A Mediterranean country is hot and dry in summer, but what the
            // model's Desert zone stands for is ground that grows almost
            // nothing, and chaparral is not that.
            | BiomeType::Mediterranean
            | BiomeType::Wetland
            | BiomeType::Freshwater
            | BiomeType::Coast => Z::Temperate,
        }
    }

    /// How warm the water itself is here, as against the air over it.
    ///
    /// The specification asks for this and the model had no such thing: a
    /// river's ice and a fish run were both decided by **air** temperature,
    /// so a reach stopped running the first frosty night. Water carries far
    /// more heat than air and gives it up far more slowly, so it lags the
    /// day almost entirely and the year only partly, and it cannot go below
    /// freezing - it becomes ice instead, which is the state the callers
    /// actually want to know about.
    ///
    /// "Water temperature 0C to 25C depending on depth, flow, and latitude,
    /// buffered relative to adjacent land": the buffering is the lag, and the
    /// nought and the twenty-five are the clamp.
    pub fn water_temperature_at(&self, region: BiomeType, season: Season, hour: f32) -> Temperature {
        /// The coldest fresh water gets before it is ice.
        const THE_COLDEST_FRESH_WATER_GETS: f32 = 0.0;
        /// And the warmest a lake or a river gets anywhere.
        const THE_WARMEST_FRESH_WATER_GETS: f32 = 25.0;
        /// How much of the day's swing reaches the water at all. Almost
        /// none: a lake is the same temperature at dawn as at dusk.
        const WHAT_OF_THE_DAY_REACHES_THE_WATER: f32 = 0.1;

        let year = self.what_the_year_does_here_in(region);
        let into_the_summer = season.how_far_into_the_year_it_is();
        let by_night = year.winter.0 + (year.summer.0 - year.winter.0) * into_the_summer;
        let by_day = year.winter.1 + (year.summer.1 - year.winter.1) * into_the_summer;

        let over_the_day = how_far_through_the_days_warmth(hour) - 0.5;
        let settled = (by_night + by_day) / 2.0;
        let in_the_water =
            settled + (by_day - by_night) * over_the_day * WHAT_OF_THE_DAY_REACHES_THE_WATER;

        if matches!(self, BiomeType::Coast) {
            // Salt water freezes lower and holds warmer.
            in_the_water.clamp(Self::THE_COLDEST_THE_SEA_GETS, Self::THE_WARMEST_THE_SEA_GETS)
        } else {
            in_the_water.clamp(THE_COLDEST_FRESH_WATER_GETS, THE_WARMEST_FRESH_WATER_GETS)
        }
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
            BiomeType::TemperateConifer => 0.6,
            BiomeType::Mediterranean => 0.4,
            BiomeType::TropicalDryForest => 0.6,
            BiomeType::Freshwater => 0.9,
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
            BiomeType::TemperateConifer => 2.0,
            BiomeType::Mediterranean => 3.0,
            BiomeType::TropicalDryForest => 2.0,
            BiomeType::Freshwater => 3.0,
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
            BiomeType::TemperateConifer => 0.4,   // colder than the deciduous wood
            BiomeType::Mediterranean => 0.4,      // hot dry summers
            BiomeType::TropicalDryForest => 0.5,
            BiomeType::Freshwater => 0.5,         // cold water and nowhere to get out of it
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
            BiomeType::TemperateConifer => 0.8,   // standing timber
            BiomeType::Mediterranean => 0.5,      // scrub, and not much of it
            BiomeType::TropicalDryForest => 0.6,
            BiomeType::Freshwater => 0.3,
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
            BiomeType::TemperateConifer => 0.6,
            BiomeType::Mediterranean => 0.5,
            BiomeType::TropicalDryForest => 0.7,
            BiomeType::Freshwater => 0.8,         // fish, reeds, and a drink
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
