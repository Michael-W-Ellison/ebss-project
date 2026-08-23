// src/environment/seasons.rs
//! Seasonal system for annual climate cycles
//!
//! Handles:
//! - Four seasons with transitions
//! - Day length variation
//! - Temperature modifiers
//! - Plant growth cycles
//! - Animal migration/hibernation

use serde::{Deserialize, Serialize};
use crate::agents::temperature::Temperature;

/// How many ticks a day lasts.
///
/// A tick is two hours of world time. That is coarse enough that a life of
/// ten thousand ticks covers years rather than the four days it used to, and
/// fine enough that dawn, noon and midnight are still separate moments an
/// agent can be cold or blind in.
pub const TICKS_PER_DAY: u32 = 12;

/// How many days a season lasts.
///
/// A season is deliberately short. The point of a calendar in a simulation
/// nobody watches for a million ticks is that the people in it have to live
/// through a winter, and a ninety-day season at any tick rate that keeps
/// day and night apart would never arrive.
pub const DAYS_PER_SEASON: u32 = 24;

/// How many days a year lasts.
pub const DAYS_PER_YEAR: u32 = DAYS_PER_SEASON * 4;

/// How many ticks a year lasts.
pub const TICKS_PER_YEAR: u32 = TICKS_PER_DAY * DAYS_PER_YEAR;

/// Season of the year
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

impl Season {
    /// Get season from day of year.
    ///
    /// The year opens in spring: a world starts in the growing season rather
    /// than in the middle of its hardest one.
    pub fn from_day_of_year(day: u32) -> Self {
        match (day % DAYS_PER_YEAR) / DAYS_PER_SEASON {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Fall,
            _ => Season::Winter,
        }
    }

    /// The day of the year this season starts on.
    pub fn first_day(&self) -> u32 {
        match self {
            Season::Spring => 0,
            Season::Summer => DAYS_PER_SEASON,
            Season::Fall => DAYS_PER_SEASON * 2,
            Season::Winter => DAYS_PER_SEASON * 3,
        }
    }

    /// Get temperature modifier for this season
    pub fn temperature_modifier(&self) -> f32 {
        match self {
            Season::Spring => 0.8,
            Season::Summer => 1.2,
            Season::Fall => 0.9,
            Season::Winter => 0.6,
        }
    }

    /// Get day length in hours (assuming temperate latitude)
    pub fn day_length(&self) -> f32 {
        match self {
            Season::Spring => 12.0,
            Season::Summer => 15.0,
            Season::Fall => 12.0,
            Season::Winter => 9.0,
        }
    }

    /// Get plant growth rate modifier
    pub fn plant_growth_modifier(&self) -> f32 {
        match self {
            Season::Spring => 1.5, // Rapid growth
            Season::Summer => 1.2, // Good growth
            Season::Fall => 0.8,   // Slowing down
            Season::Winter => 0.3, // Dormant
        }
    }

    /// Get precipitation modifier
    pub fn precipitation_modifier(&self) -> f32 {
        match self {
            Season::Spring => 1.3, // Rainy
            Season::Summer => 0.8, // Drier
            Season::Fall => 1.1,   // Moderate
            Season::Winter => 1.0, // Normal (snow in cold biomes)
        }
    }

    /// Should this season have snow in cold biomes?
    pub fn has_snow_in_cold_biomes(&self) -> bool {
        matches!(self, Season::Winter | Season::Spring)
    }

    /// Get next season
    pub fn next(&self) -> Self {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Fall,
            Season::Fall => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }

    /// Get previous season
    pub fn previous(&self) -> Self {
        match self {
            Season::Spring => Season::Winter,
            Season::Summer => Season::Spring,
            Season::Fall => Season::Summer,
            Season::Winter => Season::Fall,
        }
    }

    /// Get progress through season (0.0 to 1.0)
    pub fn progress(day_of_year: u32) -> f32 {
        let day_in_season = (day_of_year % DAYS_PER_YEAR) % DAYS_PER_SEASON;
        day_in_season as f32 / DAYS_PER_SEASON as f32
    }

    /// Get season name
    pub fn name(&self) -> &'static str {
        match self {
            Season::Spring => "Spring",
            Season::Summer => "Summer",
            Season::Fall => "Fall",
            Season::Winter => "Winter",
        }
    }
}

/// Seasonal calendar and time tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalCalendar {
    /// Current day of the year (0-364)
    pub day_of_year: u32,

    /// Current time of day (0.0 to 24.0)
    pub time_of_day: f32,

    /// Years elapsed
    pub year: u32,

    /// Ticks per day
    #[serde(default = "default_ticks_per_day")]
    ticks_per_day: u32,
}

/// What a calendar saved before the day had a length in it runs at.
fn default_ticks_per_day() -> u32 {
    TICKS_PER_DAY
}

impl SeasonalCalendar {
    /// Create a new calendar running at the given number of ticks per day.
    pub fn new(ticks_per_day: u32) -> Self {
        Self {
            day_of_year: 0,
            time_of_day: 6.0, // Start at dawn
            year: 0,
            ticks_per_day: ticks_per_day.max(1),
        }
    }

    /// How many hours of world time one tick covers.
    pub fn hours_per_tick(&self) -> f32 {
        24.0 / self.ticks_per_day as f32
    }

    /// How many ticks a day lasts on this calendar.
    pub fn ticks_per_day(&self) -> u32 {
        self.ticks_per_day
    }

    /// How many ticks a year lasts on this calendar.
    pub fn ticks_per_year(&self) -> u32 {
        self.ticks_per_day * DAYS_PER_YEAR
    }

    /// How many whole days have passed since the world began.
    pub fn days_elapsed(&self) -> u32 {
        self.year * DAYS_PER_YEAR + self.day_of_year
    }

    /// Get current season
    pub fn current_season(&self) -> Season {
        Season::from_day_of_year(self.day_of_year)
    }

    /// Advance time by one tick.
    pub fn tick(&mut self) {
        self.time_of_day += self.hours_per_tick();

        while self.time_of_day >= 24.0 {
            self.time_of_day -= 24.0;
            self.day_of_year += 1;

            if self.day_of_year >= DAYS_PER_YEAR {
                self.day_of_year = 0;
                self.year += 1;
            }
        }
    }

    /// Is it daytime?
    pub fn is_daytime(&self) -> bool {
        let season = self.current_season();
        let sunrise = 12.0 - season.day_length() / 2.0;
        let sunset = 12.0 + season.day_length() / 2.0;

        self.time_of_day >= sunrise && self.time_of_day < sunset
    }

    /// Get sun intensity (0.0 to 1.0)
    pub fn sun_intensity(&self) -> f32 {
        if !self.is_daytime() {
            return 0.0;
        }

        let season = self.current_season();
        let sunrise = 12.0 - season.day_length() / 2.0;
        let sunset = 12.0 + season.day_length() / 2.0;
        let noon = 12.0;

        // Peak at noon, fade at sunrise/sunset
        if self.time_of_day < noon {
            (self.time_of_day - sunrise) / (noon - sunrise)
        } else {
            (sunset - self.time_of_day) / (sunset - noon)
        }
    }

    /// Get ambient temperature modifier based on time of day
    pub fn time_of_day_temperature_modifier(&self) -> f32 {
        if self.is_daytime() {
            1.0 + self.sun_intensity() * 0.5
        } else {
            // Coldest at 4am, warmest at 4pm
            let hours_since_midnight = if self.time_of_day < 4.0 {
                self.time_of_day
            } else {
                self.time_of_day - 24.0
            };
            0.7 + (hours_since_midnight / -4.0).abs() * 0.3
        }
    }

    /// Get effective temperature for environment
    pub fn apply_modifiers(&self, base_temperature: Temperature) -> Temperature {
        let season_mod = self.current_season().temperature_modifier();
        let time_mod = self.time_of_day_temperature_modifier();
        base_temperature * season_mod * time_mod
    }

    /// Get season progress (0.0 to 1.0)
    pub fn season_progress(&self) -> f32 {
        Season::progress(self.day_of_year)
    }

    /// Get formatted date string
    pub fn date_string(&self) -> String {
        format!(
            "Year {}, Day {}, {} ({:.0}:00)",
            self.year,
            self.day_of_year + 1,
            self.current_season().name(),
            self.time_of_day.floor()
        )
    }
}

impl Default for SeasonalCalendar {
    fn default() -> Self {
        Self::new(TICKS_PER_DAY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_season_from_day() {
        assert_eq!(Season::from_day_of_year(0), Season::Spring);
        assert_eq!(Season::from_day_of_year(DAYS_PER_SEASON), Season::Summer);
        assert_eq!(Season::from_day_of_year(DAYS_PER_SEASON * 2), Season::Fall);
        assert_eq!(Season::from_day_of_year(DAYS_PER_SEASON * 3), Season::Winter);
        assert_eq!(Season::from_day_of_year(DAYS_PER_YEAR - 1), Season::Winter);
        assert_eq!(Season::from_day_of_year(DAYS_PER_YEAR), Season::Spring);
    }

    #[test]
    fn test_season_cycle() {
        assert_eq!(Season::Spring.next(), Season::Summer);
        assert_eq!(Season::Summer.next(), Season::Fall);
        assert_eq!(Season::Fall.next(), Season::Winter);
        assert_eq!(Season::Winter.next(), Season::Spring);

        assert_eq!(Season::Spring.previous(), Season::Winter);
        assert_eq!(Season::Winter.previous(), Season::Fall);
    }

    #[test]
    fn test_day_length() {
        assert_eq!(Season::Summer.day_length(), 15.0);
        assert_eq!(Season::Winter.day_length(), 9.0);
        assert_eq!(Season::Spring.day_length(), 12.0);
        assert_eq!(Season::Fall.day_length(), 12.0);
    }

    #[test]
    fn test_temperature_modifiers() {
        assert!(Season::Summer.temperature_modifier() > Season::Winter.temperature_modifier());
        assert_eq!(Season::Summer.temperature_modifier(), 1.2);
        assert_eq!(Season::Winter.temperature_modifier(), 0.6);
    }

    #[test]
    fn test_calendar_creation() {
        let calendar = SeasonalCalendar::default();
        assert_eq!(calendar.day_of_year, 0);
        assert_eq!(calendar.year, 0);
        assert_eq!(calendar.current_season(), Season::Spring);
    }

    #[test]
    fn test_calendar_tick() {
        let mut calendar = SeasonalCalendar::new(12);

        // One tick is two hours on a twelve-tick day
        calendar.tick();

        assert_eq!(calendar.time_of_day, 8.0);
        assert_eq!(calendar.day_of_year, 0);
    }

    #[test]
    fn test_calendar_day_advance() {
        let mut calendar = SeasonalCalendar::new(12);

        for _ in 0..12 {
            calendar.tick();
        }

        assert_eq!(calendar.time_of_day, 6.0);
        assert_eq!(calendar.day_of_year, 1);
    }

    #[test]
    fn test_calendar_year_advance() {
        let mut calendar = SeasonalCalendar::new(12);
        calendar.day_of_year = DAYS_PER_YEAR - 1;
        calendar.time_of_day = 22.0;

        calendar.tick();

        assert_eq!(calendar.day_of_year, 0);
        assert_eq!(calendar.year, 1);
    }

    #[test]
    fn test_is_daytime() {
        let mut calendar = SeasonalCalendar::default();
        calendar.time_of_day = 12.0; // Noon
        assert!(calendar.is_daytime());

        calendar.time_of_day = 2.0; // 2 AM
        assert!(!calendar.is_daytime());
    }

    #[test]
    fn test_sun_intensity() {
        let mut calendar = SeasonalCalendar::default();

        // Night time
        calendar.time_of_day = 2.0;
        assert_eq!(calendar.sun_intensity(), 0.0);

        // Noon (peak)
        calendar.time_of_day = 12.0;
        assert!(calendar.sun_intensity() > 0.9);
    }

    #[test]
    fn test_season_progress() {
        let mut calendar = SeasonalCalendar::default();
        calendar.day_of_year = 0;
        assert_eq!(calendar.season_progress(), 0.0);

        calendar.day_of_year = DAYS_PER_SEASON / 2; // Mid-spring
        assert!(calendar.season_progress() > 0.4);
        assert!(calendar.season_progress() < 0.6);

        calendar.day_of_year = DAYS_PER_SEASON - 1; // End of spring
        assert!(calendar.season_progress() > 0.9);
    }

    #[test]
    fn test_temperature_application() {
        let mut calendar = SeasonalCalendar::default();
        calendar.day_of_year = Season::Summer.first_day();
        calendar.time_of_day = 12.0; // Noon

        let base_temp = 20.0;
        let modified = calendar.apply_modifiers(base_temp);

        // Should be warmer in summer at noon
        assert!(modified > base_temp);
    }

    #[test]
    fn test_plant_growth_modifiers() {
        assert!(Season::Spring.plant_growth_modifier() > Season::Summer.plant_growth_modifier());
        assert!(Season::Winter.plant_growth_modifier() < Season::Fall.plant_growth_modifier());
    }

    #[test]
    fn test_snow_in_cold_biomes() {
        assert!(Season::Winter.has_snow_in_cold_biomes());
        assert!(Season::Spring.has_snow_in_cold_biomes());
        assert!(!Season::Summer.has_snow_in_cold_biomes());
        assert!(!Season::Fall.has_snow_in_cold_biomes());
    }

    #[test]
    fn test_date_string() {
        let mut calendar = SeasonalCalendar::default();
        calendar.year = 1;
        calendar.day_of_year = Season::Summer.first_day();
        calendar.time_of_day = 14.5;

        let date_str = calendar.date_string();
        assert!(date_str.contains("Year 1"));
        assert!(date_str.contains(&format!("Day {}", Season::Summer.first_day() + 1)));
        assert!(date_str.contains("Summer"));
    }

    #[test]
    fn test_precipitation_modifiers() {
        assert!(Season::Spring.precipitation_modifier() > Season::Summer.precipitation_modifier());
        assert_eq!(Season::Spring.precipitation_modifier(), 1.3);
    }
}
