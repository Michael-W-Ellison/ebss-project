// src/analytics/tests/mortality_tests.rs
//! Tests that this model can say what killed somebody.
//!
//! Causes of death used to be worked out *after* the fact, by asking a corpse
//! whether it was hungry — and by then the hunger has been eaten away, the
//! cold has worn off, and the honest answer to every question is no. Measured
//! over eight worlds, **70% of every death came out as "unknown cause"**: a
//! settlement could not say what killed its people, and two capability changes
//! in a row had moved no survival column with nothing able to explain why.
//!
//! So each thing that takes health says what it was as it takes it, and the
//! reckoning reads the record.

use crate::agents::{AgentConfig, Population};

fn one_person() -> Population {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population
}

/// Losing health to a named thing records the name.
#[test]
fn every_drain_says_what_it_was() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    assert_eq!(agent.state.what_last_took_health, None, "nothing has hurt him yet");

    agent.state.lose_health(5.0, "the weather");

    assert_eq!(
        agent.state.what_last_took_health.as_deref(),
        Some("the weather")
    );
    assert!(agent.state.health < 100.0);
}

/// And the last thing to hurt him is what stands, because that is the one that
/// finished it.
#[test]
fn the_last_thing_to_take_health_is_the_one_that_stands() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.state.lose_health(5.0, "the weather");
    agent.state.lose_health(5.0, "a blow");

    assert_eq!(agent.state.what_last_took_health.as_deref(), Some("a blow"));
}

/// Nothing is recorded for a harm that does no harm. A drain of zero is not an
/// event and must not overwrite the thing that is actually killing somebody.
#[test]
fn a_harm_of_nothing_is_not_a_harm() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.state.lose_health(5.0, "illness");
    agent.state.lose_health(0.0, "the weather");

    assert_eq!(
        agent.state.what_last_took_health.as_deref(),
        Some("illness"),
        "a drain of nothing should not take the credit"
    );
}

/// Health taken to nothing is death, wherever the harm came from.
#[test]
fn health_taken_to_nothing_is_death() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    assert!(agent.state.is_alive);
    agent.state.lose_health(1000.0, "a fall");

    assert!(!agent.state.is_alive);
    assert_eq!(agent.state.health, 0.0);
    assert_eq!(agent.state.what_last_took_health.as_deref(), Some("a fall"));
}

/// And the tally the settlement keeps is what the instrument reads.
#[test]
fn a_settlement_keeps_a_reckoning_of_how_it_went() {
    use crate::analytics::Simulation;
    use crate::world::{World, WorldConfig};

    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    for _ in 0..400 {
        simulation.tick();
    }

    let went = &simulation.population.stats.how_it_went;

    assert!(
        !went.is_empty(),
        "the breeding pass alone should have booked something in 400 ticks"
    );
    assert!(
        went.keys().any(|what| what.contains("breed")
            || what.contains("carrying")
            || what.contains("feed a child")),
        "where the breeding pass turns people away should be on the record: {went:?}"
    );
}
