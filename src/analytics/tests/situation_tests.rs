// src/analytics/tests/situation_tests.rs
//! Tests for a lesson being about a situation rather than a hand-written
//! string.
//!
//! `Lessons` has always recorded what worked, keyed on the thing attempted:
//! `dry`, `gather:greens`, `fire:claypot`. Every one of those keys was written
//! out by hand by somebody who had already thought of it, and the record
//! against it was a single flat number. So an agent could learn *that*
//! gathering greens does not work, and could never learn that it does not work
//! *in the autumn* - and a settlement that learned the first went hungry in
//! the spring for it.
//!
//! Everything in the model that depends on when a thing works had therefore to
//! be a rule somebody wrote: the bearing year is a table, the sun-drying is a
//! discovery flag, the fire is a precondition checked in the executor. The
//! agents were never in a position to find any of it out.
//!
//! What is here instead is the circumstances: ten coarse facts about the
//! afternoon, attached to every attempt automatically, and the arithmetic to
//! notice that one of them goes with a thing working. Nobody names the
//! situation. Nothing in the arithmetic knows what a season is, or what a fire
//! is for.

use crate::agents::practices::{Circumstance, Lessons};
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::WeatherType;
use crate::world::{Position, TerrainType, World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation
}

/// Somebody who has done a thing this many times under this sky.
fn did_it(lessons: &mut Lessons, what: &str, times: u32, worked: bool, here: &[Circumstance]) {
    for _ in 0..times {
        lessons.record_particular_here(what, worked, here);
    }
}

// --------------------------------------------------------------------------
// Noticing
// --------------------------------------------------------------------------

/// Nobody has worked anything out to begin with.
#[test]
fn a_new_agent_has_worked_nothing_out() {
    let lessons = Lessons::new();

    assert_eq!(lessons.how_much_i_have_worked_out(), 0);
    assert_eq!(lessons.what_this_changes("dry", Circumstance::ClearSky), None);
}

/// One good afternoon is not a pattern. Two coincidences must not turn into a
/// rule, or an agent spends its life acting on superstitions.
#[test]
fn one_good_afternoon_is_not_a_pattern() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 3, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "dry", 3, false, &[Circumstance::Raining]);

    assert_eq!(lessons.what_this_changes("dry", Circumstance::ClearSky), None);
    assert_eq!(lessons.how_much_i_have_worked_out(), 0);
}

/// A season of it is. Something that works in the sun and fails in the rain
/// comes out as exactly that, and nobody had to write down that the sun is
/// what dries things.
#[test]
fn a_thing_that_works_in_the_sun_is_worked_out_as_such() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 20, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "dry", 20, false, &[Circumstance::Raining]);

    let sun = lessons
        .what_this_changes("dry", Circumstance::ClearSky)
        .expect("twenty afternoons apiece is a record");
    let rain = lessons
        .what_this_changes("dry", Circumstance::Raining)
        .expect("and so is twenty wet ones");

    assert!(sun > 0.0, "it works in the sun: {sun}");
    assert!(rain < 0.0, "and not in the rain: {rain}");
}

/// And it is the strongest thing this agent knows, said in its own words.
#[test]
fn what_somebody_has_worked_out_can_be_said() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 20, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "dry", 20, false, &[Circumstance::Raining]);

    let worked_out = lessons.what_i_have_worked_out();

    assert!(
        worked_out
            .iter()
            .any(|(what, when, _)| *what == "dry" && *when == Circumstance::ClearSky),
        "nobody wrote this down anywhere: {worked_out:?}"
    );
    assert_eq!(Circumstance::ClearSky.describe(), "in the sun");
}

/// A man who has only ever done a thing in the sun has learned nothing
/// whatever about the sun. There is nothing to compare it with, and the
/// arithmetic says so rather than inventing a lesson.
#[test]
fn only_ever_having_done_it_in_the_sun_teaches_nothing_about_the_sun() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 40, true, &[Circumstance::ClearSky]);

    let sun = lessons
        .what_this_changes("dry", Circumstance::ClearSky)
        .expect("forty attempts is a record of something");

    assert!(
        sun.abs() < 0.001,
        "the sun is all he has ever seen: {sun}"
    );
    assert_eq!(
        lessons.how_much_i_have_worked_out(),
        0,
        "it takes one wet afternoon to teach him anything at all"
    );
}

/// The circumstance an agent has never met teaches it nothing either.
#[test]
fn a_circumstance_nobody_has_met_is_not_a_lesson() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 40, true, &[Circumstance::ClearSky]);

    assert_eq!(
        lessons.what_this_changes("dry", Circumstance::InWinter),
        None,
        "he has never tried it in the winter"
    );
}

// --------------------------------------------------------------------------
// Acting on it
// --------------------------------------------------------------------------

/// Which is the point of any of it: the same man, the same job, and a
/// different answer depending on the afternoon.
#[test]
fn the_same_job_gets_a_different_answer_on_a_different_afternoon() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 20, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "dry", 20, false, &[Circumstance::Raining]);

    let in_the_sun = lessons.how_likely_to_try_this_here("dry", &[Circumstance::ClearSky]);
    let in_the_rain = lessons.how_likely_to_try_this_here("dry", &[Circumstance::Raining]);

    assert!(
        in_the_sun > in_the_rain,
        "{in_the_sun} in the sun against {in_the_rain} in the rain"
    );
}

/// Where an agent has worked nothing out, this is exactly what every caller
/// had before there were circumstances at all. Nothing changes for anybody
/// until something has been found out.
#[test]
fn where_nothing_is_worked_out_it_is_the_flat_belief() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "hunt", 20, false, &[Circumstance::InWinter]);

    assert_eq!(
        lessons.how_likely_to_try_this_here("hunt", &[]),
        lessons.how_likely_to_try_this("hunt"),
        "no circumstances, no difference"
    );
    assert_eq!(
        lessons.how_likely_to_try_this_here("gather:wood", &[Circumstance::ClearSky]),
        lessons.how_likely_to_try_this("gather:wood"),
        "and nothing found out about a job is no difference either"
    );
}

/// Nobody swears off anything for life, however bad the afternoon, and nobody
/// is ever quite certain. The circumstances move the belief; they do not
/// escape the bounds it has always had.
#[test]
fn the_floor_and_the_ceiling_still_hold() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 30, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "dry", 30, false, &[Circumstance::Raining]);

    let worst = lessons.how_likely_to_try_this_here(
        "dry",
        &[Circumstance::Raining, Circumstance::InWinter],
    );
    let best = lessons.how_likely_to_try_this_here("dry", &[Circumstance::ClearSky]);

    assert!(worst >= Lessons::NEVER_QUITE_GIVES_UP, "{worst}");
    assert!(best <= Lessons::NEVER_QUITE_CERTAIN, "{best}");
}

// --------------------------------------------------------------------------
// What a head will hold
// --------------------------------------------------------------------------

/// `what` is open-ended - every resource and every made thing in the world has
/// its own key - so this cannot be allowed to grow without limit.
#[test]
fn a_head_is_not_a_filing_cabinet() {
    let mut lessons = Lessons::new();

    for i in 0..200 {
        did_it(
            &mut lessons,
            &format!("gather:thing{i}"),
            2,
            true,
            &[Circumstance::ClearSky],
        );
    }

    assert!(
        lessons.tried_this_here("gather:thing199", Circumstance::ClearSky) > 0,
        "the last thing done is remembered"
    );
    assert!(
        lessons.what_i_have_worked_out().len() <= 200,
        "and the whole lot is not"
    );
}

/// And what falls out of it is whatever this agent hardly ever does, rather
/// than whatever it happened to do least recently.
#[test]
fn the_thing_done_least_is_the_thing_forgotten() {
    let mut lessons = Lessons::new();

    // A trade this agent lives by
    did_it(&mut lessons, "gather:wood", 40, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "gather:wood", 40, false, &[Circumstance::Raining]);

    // And two hundred things it did once each, long afterwards
    for i in 0..200 {
        did_it(
            &mut lessons,
            &format!("craft:oddity{i}"),
            1,
            true,
            &[Circumstance::ClearSky],
        );
    }

    assert!(
        lessons
            .what_this_changes("gather:wood", Circumstance::ClearSky)
            .is_some(),
        "eighty days of woodcutting is not driven out by two hundred idle afternoons"
    );
}

/// And what is worked out survives being written down and read back, which a
/// map keyed on anything but a string does not always.
#[test]
fn what_is_worked_out_survives_a_round_trip() {
    let mut lessons = Lessons::new();

    did_it(&mut lessons, "dry", 20, true, &[Circumstance::ClearSky]);
    did_it(&mut lessons, "dry", 20, false, &[Circumstance::Raining]);

    let written = serde_json::to_string(&lessons).expect("an agent is saved as JSON");
    let read_back: Lessons = serde_json::from_str(&written).expect("and read back");

    assert_eq!(
        read_back.what_this_changes("dry", Circumstance::ClearSky),
        lessons.what_this_changes("dry", Circumstance::ClearSky)
    );
    assert_eq!(
        read_back.tried_this_here("dry", Circumstance::Raining),
        20
    );
}

// --------------------------------------------------------------------------
// Where the circumstances come from
// --------------------------------------------------------------------------

/// There is always a season, and the sky is one thing or the other and never
/// both.
#[test]
fn there_is_always_a_season_and_one_sky() {
    let mut simulation = one_person();
    simulation.world.climate.weather.weather_type = WeatherType::HeavyRain;
    simulation.world.climate.weather.duration_remaining = u32::MAX;

    let here = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));

    let seasons = here
        .iter()
        .filter(|circumstance| {
            matches!(
                circumstance,
                Circumstance::InSpring
                    | Circumstance::InSummer
                    | Circumstance::InAutumn
                    | Circumstance::InWinter
            )
        })
        .count();
    assert_eq!(seasons, 1, "{here:?}");

    assert!(here.contains(&Circumstance::Raining), "{here:?}");
    assert!(!here.contains(&Circumstance::ClearSky), "{here:?}");
}

/// A clear sky is a circumstance, and it is not raining.
#[test]
fn a_clear_sky_is_a_circumstance() {
    let mut simulation = one_person();
    simulation.world.climate.weather.weather_type = WeatherType::Clear;
    simulation.world.climate.weather.duration_remaining = u32::MAX;

    let here = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));

    assert!(here.contains(&Circumstance::ClearSky), "{here:?}");
    assert!(!here.contains(&Circumstance::Raining), "{here:?}");
}

/// A lit fire close enough to work at.
#[test]
fn a_fire_to_hand_is_a_circumstance() {
    let mut simulation = one_person();
    let where_it_is = (25, 25, 0);

    let before = simulation
        .what_it_is_like_here(&simulation.population.agents[0], where_it_is);
    assert!(!before.contains(&Circumstance::AFireToHand), "{before:?}");

    let fire = simulation
        .world
        .build_heat_source(
            crate::environment::HeatSourceType::Campfire,
            where_it_is,
            None,
        )
        .expect("a fire can be built here");
    let _ = simulation
        .world
        .add_fuel_to_heat_source(&fire, "wood".to_string(), 100.0);
    let _ = simulation.world.light_heat_source(&fire);

    let after = simulation
        .what_it_is_like_here(&simulation.population.agents[0], where_it_is);
    assert!(after.contains(&Circumstance::AFireToHand), "{after:?}");
}

/// Standing under a roof, rather than within a walk of one: this is a fact
/// about the afternoon, and a roof across the camp keeps nothing off you.
#[test]
fn standing_under_a_roof_is_a_circumstance() {
    let mut simulation = one_person();

    simulation
        .world
        .add_building_at(crate::world::BuildingType::SkinTent, (25, 25, 0));

    let under = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));
    let beside = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (35, 35, 0));

    assert!(under.contains(&Circumstance::UnderARoof), "{under:?}");
    assert!(!beside.contains(&Circumstance::UnderARoof), "{beside:?}");
}

/// Water a few paces off.
#[test]
fn water_a_few_paces_off_is_a_circumstance() {
    let mut simulation = one_person();

    for dy in -3..=3 {
        for dx in -3..=3 {
            if let Some(tile) = simulation
                .world
                .grid
                .get_tile_mut(&Position::new(25 + dx, 25 + dy))
            {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }

    let dry = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));
    assert!(!dry.contains(&Circumstance::ByWater), "{dry:?}");

    if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(26, 25)) {
        tile.terrain.terrain_type = TerrainType::Water;
    }

    let wet = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));
    assert!(wet.contains(&Circumstance::ByWater), "{wet:?}");
}

/// And somebody else about, which is not the same afternoon as being alone.
#[test]
fn other_people_about_is_a_circumstance() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[1].state.position = (26, 25, 0);

    let together = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));
    assert!(
        together.contains(&Circumstance::OtherPeopleAbout),
        "{together:?}"
    );

    simulation.population.agents[1].state.position = (60, 60, 0);
    let alone = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));
    assert!(
        !alone.contains(&Circumstance::OtherPeopleAbout),
        "{alone:?}"
    );
}

/// Nobody counts themselves as company.
#[test]
fn nobody_counts_themselves_as_company() {
    let simulation = one_person();

    let here = simulation
        .what_it_is_like_here(&simulation.population.agents[0], (25, 25, 0));

    assert!(!here.contains(&Circumstance::OtherPeopleAbout), "{here:?}");
}

// --------------------------------------------------------------------------
// In the running world
// --------------------------------------------------------------------------

/// Every attempt an agent makes goes down with the afternoon it was made in,
/// without the agent or the code that chose the action having any opinion
/// about which parts of the afternoon matter.
#[test]
fn an_attempt_goes_down_with_the_afternoon_it_was_made_in() {
    let mut simulation = one_person();

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 4) {
        simulation.tick();
        if !simulation.population.agents[0].state.is_alive {
            break;
        }
    }

    let lessons = &simulation.population.agents[0].lessons;

    let recorded: u32 = Circumstance::EVERY_CIRCUMSTANCE
        .iter()
        .map(|circumstance| {
            ["move", "gather:water", "gather:food", "eat:any", "wait"]
                .iter()
                .map(|what| lessons.tried_this_here(what, *circumstance))
                .sum::<u32>()
        })
        .sum();

    assert!(
        recorded > 0,
        "four days of doing things and not one afternoon written down"
    );
}

/// And a settlement left to itself works things out that nobody wrote down.
///
/// This is the whole claim. There is no rule anywhere that says gathering is a
/// summer job or that a fire is what firing wants: what is here is ten coarse
/// facts about the afternoon and the arithmetic to notice that one of them
/// goes with a thing working.
#[test]
fn a_settlement_works_things_out_that_nobody_wrote_down() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    // Long enough for a year to turn, which is what most of these lessons are
    // about.
    for _ in 0..(crate::environment::seasons::TICKS_PER_YEAR + 400) {
        simulation.tick();
        if !simulation.population.agents.iter().any(|a| a.state.is_alive) {
            break;
        }
    }

    let worked_out: usize = simulation
        .population
        .agents
        .iter()
        .map(|agent| agent.lessons.how_much_i_have_worked_out())
        .sum();

    assert!(
        worked_out > 0,
        "a year and a season of twelve people living, and not one of them \
         noticed that anything ever went better on one sort of afternoon \
         than another"
    );
}
