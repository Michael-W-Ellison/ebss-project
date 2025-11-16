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

/// Season of the year
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

impl Season {
    /// Get season from day of year (0-365)
    pub fn from_day_of_year(day: u32) -> Self {
        match day % 365 {
            0..=89 => Season::Winter,
            90..=179 => Season::Spring,
            180..=269 => Season::Summer,
            270..=364 => Season::Fall,
            _ => Season::Winter,
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
        let day_in_season = (day_of_year % 365) % 90;
        day_in_season as f32 / 90.0
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

    /// Ticks per hour
    ticks_per_hour: u32,

    /// Current tick counter
    tick_counter: u32,
}

impl SeasonalCalendar {
    /// Create a new calendar
    pub fn new(ticks_per_hour: u32) -> Self {
        Self {
            day_of_year: 0,
            time_of_day: 6.0, // Start at dawn
            year: 0,
            ticks_per_hour,
            tick_counter: 0,
        }
    }

    /// Get current season
    pub fn current_season(&self) -> Season {
        Season::from_day_of_year(self.day_of_year)
    }

    /// Advance time
    pub fn tick(&mut self) {
        self.tick_counter += 1;

        if self.tick_counter >= self.ticks_per_hour {
            self.tick_counter = 0;
            self.time_of_day += 1.0;

            if self.time_of_day >= 24.0 {
                self.time_of_day = 0.0;
                self.day_of_year += 1;

                if self.day_of_year >= 365 {
                    self.day_of_year = 0;
                    self.year += 1;
                }
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
        Self::new(100) // 100 ticks per hour by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_season_from_day() {
        assert_eq!(Season::from_day_of_year(0), Season::Winter);
        assert_eq!(Season::from_day_of_year(90), Season::Spring);
        assert_eq!(Season::from_day_of_year(180), Season::Summer);
        assert_eq!(Season::from_day_of_year(270), Season::Fall);
        assert_eq!(Season::from_day_of_year(364), Season::Fall);
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
        let calendar = SeasonalCalendar::new(100);
        assert_eq!(calendar.day_of_year, 0);
        assert_eq!(calendar.year, 0);
        assert_eq!(calendar.current_season(), Season::Winter);
    }

    #[test]
    fn test_calendar_tick() {
        let mut calendar = SeasonalCalendar::new(100);

        // Advance one hour (100 ticks)
        for _ in 0..100 {
            calendar.tick();
        }

        assert_eq!(calendar.time_of_day, 7.0);
        assert_eq!(calendar.day_of_year, 0);
    }

    #[test]
    fn test_calendar_day_advance() {
        let mut calendar = SeasonalCalendar::new(100);
        calendar.time_of_day = 23.0;

        // Advance one hour to next day
        for _ in 0..100 {
            calendar.tick();
        }

        assert_eq!(calendar.time_of_day, 0.0);
        assert_eq!(calendar.day_of_year, 1);
    }

    #[test]
    fn test_calendar_year_advance() {
        let mut calendar = SeasonalCalendar::new(100);
        calendar.day_of_year = 364;
        calendar.time_of_day = 23.0;

        // Advance one hour to next year
        for _ in 0..100 {
            calendar.tick();
        }

        assert_eq!(calendar.day_of_year, 0);
        assert_eq!(calendar.year, 1);
    }

    #[test]
    fn test_is_daytime() {
        let mut calendar = SeasonalCalendar::new(100);
        calendar.time_of_day = 12.0; // Noon
        assert!(calendar.is_daytime());

        calendar.time_of_day = 2.0; // 2 AM
        assert!(!calendar.is_daytime());
    }

    #[test]
    fn test_sun_intensity() {
        let mut calendar = SeasonalCalendar::new(100);

        // Night time
        calendar.time_of_day = 2.0;
        assert_eq!(calendar.sun_intensity(), 0.0);

        // Noon (peak)
        calendar.time_of_day = 12.0;
        assert!(calendar.sun_intensity() > 0.9);
    }

    #[test]
    fn test_season_progress() {
        let mut calendar = SeasonalCalendar::new(100);
        calendar.day_of_year = 0;
        assert_eq!(calendar.season_progress(), 0.0);

        calendar.day_of_year = 45; // Mid-winter
        assert!(calendar.season_progress() > 0.4);
        assert!(calendar.season_progress() < 0.6);

        calendar.day_of_year = 89; // End of winter
        assert!(calendar.season_progress() > 0.9);
    }

    #[test]
    fn test_temperature_application() {
        let mut calendar = SeasonalCalendar::new(100);
        calendar.day_of_year = 180; // Summer
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
        let mut calendar = SeasonalCalendar::new(100);
        calendar.year = 1;
        calendar.day_of_year = 180;
        calendar.time_of_day = 14.5;

        let date_str = calendar.date_string();
        assert!(date_str.contains("Year 1"));
        assert!(date_str.contains("Day 181"));
        assert!(date_str.contains("Summer"));
    }

    #[test]
    fn test_precipitation_modifiers() {
        assert!(Season::Spring.precipitation_modifier() > Season::Summer.precipitation_modifier());
        assert_eq!(Season::Spring.precipitation_modifier(), 1.3);
    }
}
