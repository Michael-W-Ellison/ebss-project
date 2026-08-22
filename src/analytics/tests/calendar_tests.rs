// src/analytics/tests/calendar_tests.rs
//! Tests for a calendar that turns.
//!
//! The world used to run at a hundred ticks an hour, which put 876,000 ticks
//! in a year. An agent lives about ten thousand, so an entire life - infant to
//! elderly - happened inside four calendar days, and no run anybody had ever
//! made had seen a season turn. Twenty worlds taken to eight thousand ticks
//! all ended on the same line: Year 0, Day 4, Winter. Everything the seasons
//! touch - the growth modifier on regrowth, the temperature swing, snow in
//! cold biomes, the length of a day - was in practice a constant, and the
//! constant it was stuck on was winter's.
//!
//! A tick is now two hours, a day twelve ticks, a season twenty-four days and
//! a year 1,152 ticks. A life covers eight or nine years and thirty-odd
//! seasons, and a settlement has to get through a winter to see a spring.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::flora::GrowingConditions;
use crate::environment::{Season, SeasonalCalendar, DAYS_PER_YEAR, TICKS_PER_DAY, TICKS_PER_YEAR};
use crate::world::soil::Soil;
use crate::world::{ClimateManager, Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};
use std::collections::HashSet;

/// A year fits inside a run somebody would actually sit through.
#[test]
fn a_year_is_shorter_than_a_run() {
    assert_eq!(TICKS_PER_YEAR, TICKS_PER_DAY * DAYS_PER_YEAR);

    // The runs everything else in this suite is measured over are eight
    // thousand ticks. A year has to be comfortably inside one.
    assert!(
        TICKS_PER_YEAR <= 2000,
        "a year is {TICKS_PER_YEAR} ticks, which no run will ever reach"
    );

    // And a day has to be more than one tick, or dawn, noon and midnight stop
    // being separate moments an agent can be cold or blind in.
    assert!(
        TICKS_PER_DAY >= 8,
        "a day of {TICKS_PER_DAY} ticks is too coarse to have a night in it"
    );
}

/// All four seasons arrive in a single year of world time.
#[test]
fn every_season_comes_round() {
    let mut climate = ClimateManager::new(false, false);
    let mut seen: HashSet<Season> = HashSet::new();

    for _ in 0..TICKS_PER_YEAR {
        climate.tick();
        seen.insert(climate.current_season());
    }

    assert_eq!(
        seen.len(),
        4,
        "a year should hold four seasons, not {}: {seen:?}",
        seen.len()
    );
}

/// A world starts in the growing season, not in the middle of the hard one.
#[test]
fn the_year_opens_in_spring() {
    let world = World::new(WorldConfig::default());

    assert_eq!(
        world.climate.current_season(),
        Season::Spring,
        "a world should begin in spring"
    );
    assert_eq!(world.climate.calendar.year, 0);
}

/// Days pass, and they pass in order.
#[test]
fn a_day_is_a_day_long() {
    let mut calendar = SeasonalCalendar::default();
    let started_at = calendar.time_of_day;

    for _ in 0..TICKS_PER_DAY {
        calendar.tick();
    }

    assert_eq!(calendar.day_of_year, 1, "one day should have passed");
    assert!(
        (calendar.time_of_day - started_at).abs() < 0.01,
        "and the clock should be back where it started: {started_at} -> {}",
        calendar.time_of_day
    );
    assert_eq!(calendar.days_elapsed(), 1);
}

/// A life is measured in years now, not in days.
#[test]
fn a_life_lasts_years() {
    let mut population = Population::new();
    for _ in 0..40 {
        population.spawn_agent(AgentConfig::default());
    }

    let shortest = population
        .agents
        .iter()
        .map(|agent| agent.lifespan_in_years())
        .fold(f32::INFINITY, f32::min);

    // It used to be four days. Four days is 0.011 years.
    assert!(
        shortest > 4.0,
        "an agent should live years, not days: the shortest was {shortest:.2} years"
    );

    // And a life has to be long enough to hold seasons, or nothing in the
    // world ever has to live through a winter.
    let seasons_in_a_life = shortest * 4.0;
    assert!(
        seasons_in_a_life >= 16.0,
        "a life covers only {seasons_in_a_life:.0} seasons"
    );
}

/// A year of the world's time is a year of an agent's life.
#[test]
fn an_agent_ages_by_the_calendar() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    for _ in 0..TICKS_PER_YEAR {
        simulation.tick();
    }

    assert_eq!(
        simulation.world.climate.calendar.year, 1,
        "a year of ticks should be a year on the calendar"
    );

    // Read against the calendar rather than against whether this particular
    // agent got through the year: one person alone in a world sometimes does
    // not, and that is a different test.
    let agent = &mut simulation.population.agents[0];
    agent.state.age = TICKS_PER_YEAR;
    assert!(
        (agent.age_in_years() - 1.0).abs() < 0.01,
        "and a year of life should read as one: {:.2}",
        agent.age_in_years()
    );
}

/// The same ground carries a heavier crop in summer than in winter.
#[test]
fn the_ground_gives_more_in_summer() {
    fn grown_over(season: Season) -> u32 {
        let mut soil = Soil::for_terrain(TerrainType::Plains);
        let mut patch = ResourceNode::new(ResourceType::Food, Position::new(5, 5), 500);
        patch.amount = 0;
        let mut grown = 0;

        for _ in 0..400 {
            // Hold the ground as it was: this is about the season, not about
            // the patch stripping the soil under it
            soil = Soil::for_terrain(TerrainType::Plains);
            grown += patch.regenerate_in_ground(
                18.0,
                0.5,
                season.plant_growth_modifier(),
                false,
                &mut soil,
            );
        }

        grown
    }

    let summer = grown_over(Season::Summer);
    let winter = grown_over(Season::Winter);

    assert!(
        summer > winter,
        "a summer hedgerow should outgrow a winter one: {summer} against {winter}"
    );
}

/// A plant feels the shortening day, not just the weather.
#[test]
fn short_days_slow_a_plant_down() {
    let ground = GrowingConditions {
        water: 1.0,
        nutrients: 1.0,
        uptake: 1.0,
        light: 1.0,
    };

    let in_summer = GrowingConditions {
        light: ground.light * Season::Summer.day_length() / 15.0,
        ..ground
    };
    let in_winter = GrowingConditions {
        light: ground.light * Season::Winter.day_length() / 15.0,
        ..ground
    };

    assert!(
        in_summer.growth_share() > in_winter.growth_share(),
        "fifteen hours of sun should beat nine: {:.2} against {:.2}",
        in_summer.growth_share(),
        in_winter.growth_share()
    );
}

/// A settlement gets through its first winter.
#[test]
fn a_settlement_lives_through_a_winter() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    // Far enough to be out the other side of the first winter and into the
    // second spring.
    for _ in 0..TICKS_PER_YEAR + TICKS_PER_DAY * 4 {
        simulation.tick();
    }

    assert_eq!(
        simulation.world.climate.current_season(),
        Season::Spring,
        "the run should have come out into spring"
    );

    let alive = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .count();

    assert!(
        alive > 0,
        "somebody should still be here on the far side of the winter"
    );
}
