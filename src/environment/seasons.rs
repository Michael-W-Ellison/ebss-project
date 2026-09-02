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

/// Minutes in a day. The body's clock, and the one the calendar is stated in.
pub const MINUTES_PER_DAY: u32 = 1440;

/// Days in a week - and there are two lengths of week.
///
/// "Week durations alternate between a 7-day week and an 8-day week. This
/// results in 30-day months, and a 360-day year." Two of each makes thirty:
/// 7 + 8 + 7 + 8. There is no other way to get four weeks into a month of
/// thirty days, and it is why a season is exactly twelve weeks.
pub const DAYS_IN_A_SHORT_WEEK: u32 = 7;
pub const DAYS_IN_A_LONG_WEEK: u32 = 8;
pub const WEEKS_PER_MONTH: u32 = 4;

/// A short week and a long one, which is the unit the calendar actually
/// repeats on: fifteen days.
pub const A_PAIR_OF_WEEKS: u32 = DAYS_IN_A_SHORT_WEEK + DAYS_IN_A_LONG_WEEK;

/// Days in a month.
pub const DAYS_PER_MONTH: u32 =
    (DAYS_IN_A_SHORT_WEEK + DAYS_IN_A_LONG_WEEK) * (WEEKS_PER_MONTH / 2);

/// Months in a year, and months in a season.
pub const MONTHS_PER_YEAR: u32 = 12;
pub const MONTHS_PER_SEASON: u32 = MONTHS_PER_YEAR / 4;

/// How many days a season lasts. Three months of thirty days.
pub const DAYS_PER_SEASON: u32 = DAYS_PER_MONTH * MONTHS_PER_SEASON;

/// How many days a year lasts.
pub const DAYS_PER_YEAR: u32 = DAYS_PER_MONTH * MONTHS_PER_YEAR;

/// Weeks in a season, which is what the early/late phases are counted in.
pub const WEEKS_PER_SEASON: u32 = WEEKS_PER_MONTH * MONTHS_PER_SEASON;

/// How many minutes a year lasts: 518,400.
pub const MINUTES_PER_YEAR: u32 = MINUTES_PER_DAY * DAYS_PER_YEAR;

/// How long a life runs before old age takes it.
pub const YEARS_BEFORE_OLD_AGE_TAKES_YOU: u32 = 70;

/// The most minutes anybody lives: 36,288,000.
pub const MINUTES_IN_A_WHOLE_LIFE: u32 = MINUTES_PER_YEAR * YEARS_BEFORE_OLD_AGE_TAKES_YOU;

/// How many decision turns a day holds.
///
/// A turn is a decision - the unit at which somebody looks around and picks
/// what to do - and it is not the same thing as a minute. The calendar above
/// is stated in minutes because that is what a body runs on; this is how
/// often anybody in it stops to think.
///
/// Twelve, so a turn is two hours. Every clock that matters is derived from
/// the minute figures rather than from this, so making it finer makes the
/// decision loop denser without making any of the physiology wrong - see
/// `agents::physiology::MINUTES_PER_TURN`. At one, a turn is a minute and a
/// seventy-year life is thirty-six million of them, which is the calendar as
/// specified and is not a thing anybody can run.
pub const TICKS_PER_DAY: u32 = 12;

/// How many turns a year lasts.
pub const TICKS_PER_YEAR: u32 = TICKS_PER_DAY * DAYS_PER_YEAR;

/// Where in a season a day falls: the first fortnight, the long middle, or
/// the last fortnight.
///
/// "Weeks 1-2 (Early Spring), weeks 3-10 (Spring), weeks 11-12 (Late Spring)",
/// and the same shape for every season. Two weeks at each end and eight in
/// the middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartOfSeason {
    Early,
    Deep,
    Late,
}

impl PartOfSeason {
    /// Which part of its season this day of the year falls in.
    pub fn from_day_of_year(day: u32) -> Self {
        let into_the_season = (day % DAYS_PER_YEAR) % DAYS_PER_SEASON;
        let week = week_of_the_season(into_the_season);
        if week < 2 {
            PartOfSeason::Early
        } else if week < WEEKS_PER_SEASON - 2 {
            PartOfSeason::Deep
        } else {
            PartOfSeason::Late
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PartOfSeason::Early => "early",
            PartOfSeason::Deep => "deep",
            PartOfSeason::Late => "late",
        }
    }

    /// The first day of the season that falls in this part of it.
    pub fn first_day_of_season(&self) -> u32 {
        match self {
            PartOfSeason::Early => 0,
            PartOfSeason::Deep => first_day_of_the_week(2),
            PartOfSeason::Late => first_day_of_the_week(WEEKS_PER_SEASON - 2),
        }
    }

    /// The last day of the season that falls in this part of it.
    pub fn last_day_of_season(&self) -> u32 {
        match self {
            PartOfSeason::Early => PartOfSeason::Deep.first_day_of_season() - 1,
            PartOfSeason::Deep => PartOfSeason::Late.first_day_of_season() - 1,
            PartOfSeason::Late => DAYS_PER_SEASON - 1,
        }
    }
}

/// The first day of the season that a given week of it starts on.
///
/// Weeks alternate seven days and eight, so this is not a multiplication. A
/// pair of weeks is fifteen days; the short week opens the pair and the long
/// one closes it.
pub fn first_day_of_the_week(week: u32) -> u32 {
    (week / 2) * A_PAIR_OF_WEEKS + (week % 2) * DAYS_IN_A_SHORT_WEEK
}

/// The first day of the *year* that falls in this part of this season.
///
/// The two of these are what a bearing window is written in: a hedgerow opens
/// in late spring and closes in deep autumn, and those are the days it means.
pub fn first_day_of(season: Season, part: PartOfSeason) -> u32 {
    season.first_day() + part.first_day_of_season()
}

/// The last day of the year that falls in this part of this season.
pub fn last_day_of(season: Season, part: PartOfSeason) -> u32 {
    season.first_day() + part.last_day_of_season()
}

/// Which week of a season a day falls in, counted from nought.
///
/// Weeks alternate seven days and eight, so this is not a division. A pair of
/// weeks is fifteen days; within the pair the first seven are the short week
/// and the next eight the long one.
pub fn week_of_the_season(day_of_season: u32) -> u32 {
    let pairs = day_of_season / A_PAIR_OF_WEEKS;
    let into_the_pair = day_of_season % A_PAIR_OF_WEEKS;
    pairs * 2 + u32::from(into_the_pair >= DAYS_IN_A_SHORT_WEEK)
}

/// How long the week containing this day of the season runs.
pub fn how_long_this_week_is(day_of_season: u32) -> u32 {
    if week_of_the_season(day_of_season) % 2 == 0 {
        DAYS_IN_A_SHORT_WEEK
    } else {
        DAYS_IN_A_LONG_WEEK
    }
}

/// Which month of the year a day falls in, counted from nought.
pub fn month_of_the_year(day_of_year: u32) -> u32 {
    (day_of_year % DAYS_PER_YEAR) / DAYS_PER_MONTH
}

/// Season of the year
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
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

    /// How much meat is on a wild animal at this time of year, against what
    /// the same animal carries at its fattest.
    ///
    /// "Killing an animal in late summer or autumn should result in more meat,
    /// whereas killing an animal in winter and early spring should result in
    /// less meat."
    ///
    /// A deer put on through the summer and the mast season is carrying a
    /// quarter more than the book says; the same deer in March has spent four
    /// months living on bark and is carrying a third less. The curve runs
    /// continuously round the year - each season starts where the last one
    /// finished - so there is no day on which a herd suddenly doubles.
    pub fn how_fat_the_beasts_are(&self) -> f32 {
        let through = self.season_progress();

        // Condition does not run in straight lines. An animal loses most of
        // what it is going to lose in the first hard weeks of the winter and
        // then holds on at very little; and it puts nothing back in the first
        // weeks of the spring, when there is nothing yet to eat, and then
        // fattens as the grass comes. Running both of those as straight lines
        // put a deer in midwinter in the same condition as a deer in midsummer,
        // which is the opposite of what the specification asks for.
        let (opens, closes, along) = match self.current_season() {
            Season::Spring => (Self::LEANEST, 0.85, through * through),
            Season::Summer => (0.85, 1.10, through),
            Season::Fall => (1.10, Self::FATTEST, through),
            Season::Winter => (Self::FATTEST, Self::LEANEST, through.powf(0.7)),
        };

        opens + (closes - opens) * along
    }

    /// What an animal carries in the last of the autumn, having eaten all
    /// summer and all through the mast.
    pub const FATTEST: f32 = 1.25;

    /// And what it carries at the end of a winter spent on bark.
    pub const LEANEST: f32 = 0.65;

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

#[cfg(test)]
mod calendar_tests {
    use super::*;

    /// Every figure in the specification, asserted.
    #[test]
    fn the_calendar_is_the_one_that_was_asked_for() {
        assert_eq!(MINUTES_PER_DAY, 1440, "one tick a minute, 1440 to the day");
        assert_eq!(DAYS_PER_MONTH, 30);
        assert_eq!(MONTHS_PER_YEAR, 12);
        assert_eq!(DAYS_PER_YEAR, 360);
        assert_eq!(MONTHS_PER_SEASON, 3, "about three months to a season");
        assert_eq!(DAYS_PER_SEASON, 90);
        assert_eq!(WEEKS_PER_SEASON, 12);
        assert_eq!(MINUTES_PER_YEAR, 518_400);
        assert_eq!(MINUTES_IN_A_WHOLE_LIFE, 36_288_000);
        assert_eq!(YEARS_BEFORE_OLD_AGE_TAKES_YOU, 70);
    }

    /// "Week durations alternate between a 7-day week and an 8-day week. This
    /// results in 30-day months, and a 360-day year."
    #[test]
    fn weeks_alternate_seven_and_eight_and_make_a_thirty_day_month() {
        let mut days = 0;
        for week in 0..WEEKS_PER_MONTH {
            days += how_long_this_week_is(days);
            assert_eq!(
                how_long_this_week_is(days - 1),
                if week % 2 == 0 { 7 } else { 8 },
                "week {week} should be {} days",
                if week % 2 == 0 { 7 } else { 8 }
            );
        }
        assert_eq!(days, DAYS_PER_MONTH, "four weeks should make a month");
    }

    #[test]
    fn every_day_of_a_season_falls_in_one_of_its_twelve_weeks() {
        for day in 0..DAYS_PER_SEASON {
            let week = week_of_the_season(day);
            assert!(week < WEEKS_PER_SEASON, "day {day} landed in week {week}");
        }
        // And the weeks run in order, one boundary at a time
        let mut last = 0;
        for day in 0..DAYS_PER_SEASON {
            let week = week_of_the_season(day);
            assert!(week == last || week == last + 1, "day {day}: {last} -> {week}");
            last = week;
        }
        assert_eq!(last, WEEKS_PER_SEASON - 1, "the last day is in the last week");
    }

    /// "Weeks 1-2 (Early), weeks 3-10 (main), weeks 11-12 (Late)."
    #[test]
    fn a_season_has_a_fortnight_at_each_end_and_eight_weeks_between() {
        let mut early = 0;
        let mut deep = 0;
        let mut late = 0;
        for day in 0..DAYS_PER_SEASON {
            match PartOfSeason::from_day_of_year(day) {
                PartOfSeason::Early => early += 1,
                PartOfSeason::Deep => deep += 1,
                PartOfSeason::Late => late += 1,
            }
        }
        // Two weeks at each end: a short and a long, so fifteen days
        assert_eq!(early, 15, "weeks one and two");
        assert_eq!(late, 15, "weeks eleven and twelve");
        assert_eq!(deep, DAYS_PER_SEASON - 30, "the eight weeks between");
    }

    #[test]
    fn each_season_gets_a_quarter_of_the_year_and_starts_where_it_should() {
        for (season, index) in [
            (Season::Spring, 0),
            (Season::Summer, 1),
            (Season::Fall, 2),
            (Season::Winter, 3),
        ] {
            assert_eq!(season.first_day(), index * DAYS_PER_SEASON);
            assert_eq!(Season::from_day_of_year(season.first_day()), season);
            assert_eq!(
                Season::from_day_of_year(season.first_day() + DAYS_PER_SEASON - 1),
                season
            );
        }
    }

    #[test]
    fn twelve_months_run_in_order_and_the_year_wraps() {
        for month in 0..MONTHS_PER_YEAR {
            assert_eq!(month_of_the_year(month * DAYS_PER_MONTH), month);
            assert_eq!(month_of_the_year(month * DAYS_PER_MONTH + 29), month);
        }
        assert_eq!(month_of_the_year(DAYS_PER_YEAR), 0, "the year comes round");
    }

    /// The decision turn is separable from the calendar, and every clock the
    /// body runs on is stated in minutes rather than in turns.
    #[test]
    fn the_decision_turn_does_not_change_the_calendar() {
        assert_eq!(TICKS_PER_YEAR, TICKS_PER_DAY * DAYS_PER_YEAR);
        assert_eq!(
            crate::agents::physiology::MINUTES_PER_TURN,
            MINUTES_PER_DAY / TICKS_PER_DAY,
            "a turn is however many minutes a day holds divided by the turns in it"
        );
        assert_eq!(
            crate::agents::physiology::MINUTES_PER_TURN * TICKS_PER_DAY,
            MINUTES_PER_DAY,
            "and the turns in a day cover the whole of it"
        );
    }
}
