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
        simulation.take_a_turn();
    }

    let went = &simulation.population.stats.how_it_went;

    assert!(
        !went.is_empty(),
        "the breeding pass alone should have booked something in 400 turns"
    );
    assert!(
        went.keys().any(|what| what.contains("breed")
            || what.contains("carrying")
            || what.contains("feed a child")),
        "where the breeding pass turns people away should be on the record: {went:?}"
    );
}

// --------------------------------------------------------------------------
// The last straw against the load
// --------------------------------------------------------------------------
//
// Writing the cause down at the time fixed the 70% of deaths that came out as
// "unknown cause" and left a second fault standing behind it: a death credited
// to whatever removed the final point credits the last straw and not the load.
// Measured over eight worlds, a blow took 47.6% of all the health lost in this
// model and was credited with 25.9% of the deaths; thirst took 0.6% and was
// credited with 9.4%. A drip out-ranks a lump, because the drip is nearly
// always what happens to be last.
//
// So what is asserted below is that the reckoning now reads an apportionment
// of the body rather than the name of the last thing to speak.

/// How much of him each thing is holding, by name.
fn holding(agent: &crate::agents::Agent, what: &str) -> f32 {
    agent
        .state
        .what_has_taken_health
        .iter()
        .find(|(named, _)| named == what)
        .map(|(_, taken)| *taken)
        .unwrap_or(0.0)
}

/// The whole of what is missing off a man is accounted for by name.
///
/// This is the property the apportionment rests on: the entries are not a
/// sample of what happened to him, they are the missing part of him, summing
/// to exactly what is gone. Everything else here follows from it.
#[test]
fn what_is_missing_off_a_man_is_all_of_it_accounted_for() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    for (amount, to) in [
        (5.0, "the weather"),
        (12.0, "a blow"),
        (0.4, "hunger"),
        (3.0, "illness"),
        (0.4, "hunger"),
    ] {
        agent.state.lose_health(amount, to);
        assert!(
            (agent.state.what_is_still_standing() - (100.0 - agent.state.health)).abs() < 0.001,
            "the ledger and the man disagree: {} against {}",
            agent.state.what_is_still_standing(),
            100.0 - agent.state.health
        );
    }

    agent.state.heal(7.0);
    assert!(
        (agent.state.what_is_still_standing() - (100.0 - agent.state.health)).abs() < 0.001,
        "mending broke the account"
    );

    // And the same thing twice is one entry, not two.
    assert_eq!(
        agent
            .state
            .what_has_taken_health
            .iter()
            .filter(|(named, _)| named == "hunger")
            .count(),
        1
    );
}

/// A drip that lands last does not out-rank the lump that did the work.
///
/// The case the old reading got wrong every time: a man beaten to within a
/// point of his life, finished by the next turn of hunger. The hunger took a
/// tenth of a point and the reckoning called it starvation.
#[test]
fn the_last_straw_does_not_take_the_credit_for_the_load() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.state.lose_health(99.9, "a blow");
    agent.state.lose_health(0.1, "hunger");

    assert!(!agent.state.is_alive);
    assert_eq!(
        agent.state.what_last_took_health.as_deref(),
        Some("hunger"),
        "hunger is still the last thing that touched him, and that is still true"
    );
    assert_eq!(
        agent.state.what_took_the_most(),
        Some("a blow"),
        "but it is not what killed him"
    );
}

/// And a load made of drips is still the load.
///
/// The converse matters as much: hunger that actually does the killing must
/// still answer for it when a scratch happens to land last.
#[test]
fn a_load_made_of_drips_is_still_the_load() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    for _ in 0..99 {
        agent.state.lose_health(1.0, "hunger");
    }
    agent.state.lose_health(1.0, "a blow");

    assert_eq!(agent.state.what_last_took_health.as_deref(), Some("a blow"));
    assert_eq!(agent.state.what_took_the_most(), Some("hunger"));
}

/// What a thing swung is not what it took.
///
/// A fall is priced at a thousand and a body holds a hundred. Booking the
/// swing would let one overkill outweigh every other thing that ever happened
/// to a man, and would break the account of him besides.
#[test]
fn a_thing_is_booked_for_what_it_took_and_not_for_what_it_swung() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.state.lose_health(70.0, "hunger");
    agent.state.lose_health(1000.0, "a fall");

    assert_eq!(holding(agent, "a fall"), 30.0, "there were only thirty left to take");
    assert_eq!(holding(agent, "hunger"), 70.0);
    assert_eq!(
        agent.state.what_took_the_most(),
        Some("hunger"),
        "the fall finished a man that hunger had already mostly taken"
    );
    assert!((agent.state.what_is_still_standing() - 100.0).abs() < 0.001);
}

/// Health that healed away killed nobody.
///
/// A man beaten half to death at twenty and starved at forty was killed by the
/// starving. Without this the tally is an account of his life rather than of
/// his death, and every old injury he ever walked off would compete with the
/// thing that actually did it.
#[test]
fn what_the_body_made_good_is_not_what_killed_it() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.state.lose_health(60.0, "a blow");
    agent.state.heal(60.0);
    assert_eq!(agent.state.health, 100.0);
    assert!(
        agent.state.what_has_taken_health.is_empty(),
        "he walked it off: {:?}",
        agent.state.what_has_taken_health
    );

    for _ in 0..100 {
        agent.state.lose_health(1.0, "hunger");
    }
    assert_eq!(agent.state.what_took_the_most(), Some("hunger"));
}

/// And mending takes back from everything that is still standing, in
/// proportion to what each is holding.
#[test]
fn mending_forgives_in_proportion() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.state.lose_health(30.0, "a blow");
    agent.state.lose_health(10.0, "illness");
    agent.state.heal(20.0);

    // Half of what was outstanding came back, so each is holding half.
    assert!((holding(agent, "a blow") - 15.0).abs() < 0.001);
    assert!((holding(agent, "illness") - 5.0).abs() < 0.001);
    assert_eq!(
        agent.state.what_took_the_most(),
        Some("a blow"),
        "mending evenly changes nobody's place"
    );
}

/// A body that will not carry a hundred takes health off a man, and until now
/// it was the one drain in the model that did so without saying what it was.
///
/// The drop went straight into the field, named nothing, and the reckoning
/// carried on crediting whatever had spoken last - so every point the cap ever
/// took was booked against something else entirely.
#[test]
fn a_broken_body_says_that_it_is_what_is_holding_him_down() {
    use crate::agents::body::BodyPartType;

    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.body.damage_part(BodyPartType::Torso, 40.0);
    let condition = agent.body.overall_health() * 100.0;
    assert!(condition < 100.0, "the body is hurt");

    agent.turn_with_percepts(1);

    assert!(agent.state.health <= condition + 0.001);
    assert!(
        holding(agent, "a wound") > 0.0,
        "the cap took health and named nothing: {:?}",
        agent.state.what_has_taken_health
    );
    assert!(
        (agent.state.what_is_still_standing() - (100.0 - agent.state.health)).abs() < 0.01,
        "the cap is outside the account"
    );
}

/// Two things that took exactly as much as each other answer the same way
/// every run, because a cause of death is a fact about the world and not about
/// the order a list happened to be built in.
#[test]
fn a_tie_is_broken_the_same_way_every_time() {
    let order = |first: &str, second: &str| {
        let mut population = one_person();
        let agent = &mut population.agents[0];
        agent.state.lose_health(10.0, first);
        agent.state.lose_health(10.0, second);
        agent.state.what_took_the_most().unwrap().to_string()
    };

    assert_eq!(order("a blow", "hunger"), order("hunger", "a blow"));
}

/// Nothing accounts for a man nothing has happened to.
#[test]
fn an_unhurt_man_has_nothing_holding_him() {
    let population = one_person();
    let agent = &population.agents[0];

    assert_eq!(agent.state.what_took_the_most(), None);
    assert_eq!(agent.state.what_is_still_standing(), 0.0);
}

// --------------------------------------------------------------------------
// One spelling per cause
// --------------------------------------------------------------------------

/// A man the slow thing wore down and the quick thing finished is held by one
/// name, not two.
///
/// Hunger arrives as a drip every turn a body is wasting, and when the reserve
/// is gone a single blow takes whatever is left. Those were two strings -
/// "hunger" and "starvation" - and the apportionment adds up *by name*, so one
/// cause was booked under two headings and neither got its due. Thirst had the
/// same pair, "thirst" and "dehydration"; blocks C and D of #210 found
/// dehydration holding 0.9% and 1.2% of the dead where #209 had recorded
/// thirst as nothing at all.
#[test]
fn the_slow_thing_and_the_blow_that_finishes_it_share_a_name() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    // Worn down a turn at a time...
    for _ in 0..60 {
        agent.state.lose_health(0.5, crate::agents::AgentState::HUNGER);
    }
    // ...and then finished off.
    let what_was_left = agent.state.health;
    agent
        .state
        .lose_health(what_was_left, crate::agents::AgentState::HUNGER);

    assert!(!agent.state.is_alive);
    assert_eq!(
        agent.state.what_has_taken_health.len(),
        1,
        "one cause, two headings: {:?}",
        agent.state.what_has_taken_health
    );
    assert_eq!(agent.state.what_took_the_most(), Some("hunger"));
    assert!(
        (agent.state.what_is_still_standing() - 100.0).abs() < 0.001,
        "and it holds the whole man"
    );
}

/// No two of the names are the same name.
#[test]
fn every_cause_is_spelled_once() {
    use crate::agents::AgentState;

    let mut seen = AgentState::EVERYTHING_THAT_TAKES_HEALTH.to_vec();
    seen.sort_unstable();
    let mut once_each = seen.clone();
    once_each.dedup();

    assert_eq!(
        once_each, seen,
        "a cause is listed twice, which is how two names for one thing start"
    );
}

/// And nothing in the model takes health under a name that is not on the list.
///
/// The list is what the tests, and anything else that wants to reason about
/// the whole vocabulary, read instead of keeping their own copy.
#[test]
fn nothing_takes_health_under_a_name_nobody_wrote_down() {
    use crate::agents::AgentState;
    use crate::analytics::Simulation;
    use crate::world::{World, WorldConfig};

    crate::core::dice::seed(7);
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    for _ in 0..2_000 {
        simulation.take_a_turn();
        for agent in &simulation.population.agents {
            for (named, _) in &agent.state.what_has_taken_health {
                assert!(
                    AgentState::EVERYTHING_THAT_TAKES_HEALTH.contains(&named.as_str()),
                    "'{named}' took health and is not one of the names"
                );
            }
        }
    }
}
