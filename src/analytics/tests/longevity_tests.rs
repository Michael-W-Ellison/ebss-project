// src/analytics/tests/longevity_tests.rs
//! Tests for a settlement that has to last.
//!
//! Every population tested was gone by thirty thousand ticks. Two things were
//! killing them, and neither showed up in the eight-thousand-tick runs
//! everything else is measured over:
//!
//! - Nearly half of everyone ever born died before growing up. Children have
//!   no clothing of their own and cannot get any, so they ran two or three
//!   degrees colder than the adults beside them and died of it.
//! - Water was consumed and never came back. Every drink took a unit out of
//!   the world for good, and a lake drunk dry was deleted, so a world lost
//!   more than half its water in fifteen thousand ticks and the people
//!   drinking from it died of thirst and then of hunger.

use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::Simulation;
use crate::world::{ResourceType, World, WorldConfig};

/// Drinking does not empty the rivers.
#[test]
fn water_is_not_used_up() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    let water_at = |simulation: &Simulation| -> u32 {
        simulation
            .world
            .resources
            .iter()
            .filter(|resource| resource.resource_type == ResourceType::Water)
            .map(|resource| resource.amount)
            .sum()
    };

    let before = water_at(&simulation);
    assert!(before > 0, "a world should have water in it");

    for _ in 0..6000 {
        simulation.tick();
    }

    let after = water_at(&simulation);

    assert!(
        after >= before,
        "a river should not be drunk dry: {before} -> {after}"
    );
    assert!(
        simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.resource_type == ResourceType::Water),
        "the water should still be on the map"
    );
}

/// A child beside an adult is kept warm by them.
#[test]
fn the_young_are_kept_warm_by_the_adults_around_them() {
    fn coldest_child(with_adults: bool) -> f32 {
        let world = World::new(WorldConfig::default());
        let mut population = Population::new();
        for _ in 0..6 {
            population.spawn_agent(AgentConfig::default());
        }

        let mut simulation = Simulation::new(world, population);

        // Half of them are small children; the rest are grown, or are not
        // there at all
        for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
            agent.state.position = (25, 25, 0);
            if index < 3 {
                agent.state.age = 800;
            } else {
                agent.state.age = 4000;
            }
            agent.update_life_stage();
        }

        if !with_adults {
            simulation.population.agents.truncate(3);
        }

        for _ in 0..600 {
            // Keep them together: this is about who is standing beside whom
            for agent in &mut simulation.population.agents {
                agent.state.position = (25, 25, 0);
            }
            simulation.tick();
        }

        simulation
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                matches!(agent.state.life_stage, LifeStage::Child | LifeStage::Infant)
            })
            .map(|agent| agent.body_temperature.current)
            .fold(f32::INFINITY, f32::min)
    }

    let looked_after = coldest_child(true);
    let left_alone = coldest_child(false);

    assert!(
        looked_after >= left_alone,
        "a child beside adults should be no colder than one on its own: \
         {looked_after:.1} against {left_alone:.1}"
    );
}

/// A settlement is still there after thirty thousand ticks.
///
/// Ignored by default: it runs thirty thousand ticks, which takes most of a
/// minute on its own against a suite that otherwise finishes in seconds, and
/// the answer is a probability rather than a fact. Run it with
/// `cargo test --release -- --ignored a_settlement_lasts`.
///
/// Measured over twelve independent worlds on the commit that added this,
/// nine were still inhabited at thirty thousand ticks and thirteen of sixteen
/// at twenty thousand. Before the two fixes above, three of three were empty -
/// at 15,994, 18,253 and 26,907 ticks. A single world is therefore worth
/// running by hand and not asserting on: a quarter of them die of their own
/// accord, on a fifty-by-fifty map that carries perhaps forty people.
#[test]
#[ignore]
fn a_settlement_lasts_thirty_thousand_ticks() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..30000 {
        simulation.tick();
    }

    let alive = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .count();

    assert!(
        alive > 0,
        "the settlement should still be there after thirty thousand ticks"
    );
}

/// A settlement is still growing children into adults well past the point it
/// used to stop.
///
/// The collapse showed up first as births simply stopping: in one traced run
/// the count froze at thirty-three around twelve thousand ticks and never
/// moved again while the last adults aged out. Nearly half of everyone born
/// had died before growing up, so there was never a second generation to take
/// over.
#[test]
fn a_settlement_still_raises_children_late_on() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..9000 {
        simulation.tick();
    }

    let grown_here = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .filter(|agent| agent.state.age < 6500)
        .count();

    assert!(
        grown_here > 0,
        "nine thousand ticks in, the settlement should hold people born into it"
    );
}
