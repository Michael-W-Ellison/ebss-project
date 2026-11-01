// src/analytics/tests/pace_tests.rs
//! How slowly a people comes to new ideas (ISSUES_FOUND #389).
//!
//! - a thread under test starts at the development pace, and a people's pace
//!   is thirty thousand times slower
//! - at the development pace an idea comes the moment the experience it wants
//!   is had
//! - at a slower pace it comes, on average, that many times later
//! - odds of noticing something are divided by the slowness
//! - knowledge passing between people is not slowed: a child is taught at any
//!   pace

use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::discovery;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::world::{World, WorldConfig};

/// Run `body` at `slowness`, putting the thread back after.
fn at_a_pace<T>(slowness: f64, body: impl FnOnce() -> T) -> T {
    discovery::set(slowness);
    let out = body();
    discovery::set(discovery::THE_DEVELOPMENT_PACE);
    out
}

/// The suite tests the machinery at the development pace; a run of the model
/// starts at a people's.
#[test]
fn a_test_starts_at_the_development_pace() {
    assert_eq!(discovery::slowness(), discovery::THE_DEVELOPMENT_PACE);
    assert_eq!(discovery::A_PEOPLES_PACE, 30_000.0);
}

/// At the development pace an idea comes at the count it was written with,
/// and comes once.
#[test]
fn at_the_development_pace_an_idea_comes_at_its_count() {
    let mut agent = Agent::new(AgentConfig::default());
    assert!(agent.it_comes_to_me("do:BurnOff", 12));
    assert!(!agent.it_comes_to_me("do:BurnOff", 12), "and not twice");
}

/// Slower, it takes about the count times the slowness of further
/// experience: at ten times slower and a count of ten, about one in a
/// hundred tries.
#[test]
fn slower_it_comes_that_many_times_later() {
    crate::core::dice::seed(11);
    let came = at_a_pace(10.0, || (0..10_000).filter(|_| discovery::does_it_occur_now(10)).count());
    assert!((50..=160).contains(&came), "about a hundred in ten thousand, not {came}");

    crate::core::dice::seed(11);
    let came = at_a_pace(discovery::A_PEOPLES_PACE, || {
        (0..10_000).filter(|_| discovery::does_it_occur_now(300)).count()
    });
    assert_eq!(came, 0, "at a people's pace, ten thousand gathers is nothing like enough");
}

/// Odds of noticing something are divided by the slowness.
#[test]
fn odds_of_noticing_are_divided_by_the_slowness() {
    assert_eq!(discovery::at_this_pace(0.2), 0.2);
    let slowed = at_a_pace(100.0, || discovery::at_this_pace(0.2));
    assert!((slowed - 0.002).abs() < 1e-12);
}

/// A child is taught what the camp knows at a people's pace as fast as at the
/// development pace: knowledge passing on is not finding it out.
#[test]
fn teaching_is_not_slowed() {
    crate::core::dice::seed(7);
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let parent = {
        let teacher = &mut simulation.population.agents[0];
        teacher.state.now_this_many_years_old(35);
        teacher.hearth = Some((25, 25));
        teacher.found_out_how_to("know:coppice");
        teacher.id
    };
    let child = &mut simulation.population.agents[1];
    child.state.now_this_many_years_old(9);
    child.hearth = Some((25, 25));
    child.parent_ids.push(parent);

    at_a_pace(discovery::A_PEOPLES_PACE, || {
        for day in 1..=365 {
            simulation.current_turn = day * TICKS_PER_DAY;
            simulation.the_young_are_taught();
        }
    });
    assert!(simulation.population.agents[1].what_i_found_out().contains("know:coppice"));
}
