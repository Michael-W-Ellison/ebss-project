// src/analytics/tests/tally_tests.rs
//! Tests that the tallies count one thing under one name.
//!
//! `actions_taken` booked everything chosen in the fear branch as "Flee".
//! `actions_failed` booked by the action's own name. So a run that happened
//! went under "Flee" and a run that was refused went under "FleeFrom", and
//! ISSUES #66 could report 19,626 failures of a verb that showed no attempts
//! at all — and `Freeze` as never once taken in sixty-four worlds, when the
//! decision had reached it and the count went somewhere else.
//!
//! Both claims were wrong, and neither is the sort of thing a reader can
//! check. The invariant that catches it is small: **nothing can fail at a
//! thing it was never recorded doing.**

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn a_settlement(founders: usize, ticks: u32) -> Simulation {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..founders {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);
    for _ in 0..ticks {
        simulation.tick();
    }

    simulation
}

// --------------------------------------------------------------------------
// One thing, one name
// --------------------------------------------------------------------------

/// The rule, stated where it can be read. A verb that names itself is booked
/// under its own name whatever branch chose it.
#[test]
fn a_verb_that_names_itself_is_booked_under_its_own_name() {
    let running = Action::FleeFrom { away_from: (3, 3, 0) };

    assert_eq!(
        Simulation::what_to_book(&running, true),
        Simulation::name_of(&running),
        "a run chosen out of fear is still a run"
    );
    assert_eq!(
        Simulation::what_to_book(&Action::Freeze, true),
        Simulation::name_of(&Action::Freeze),
        "and freezing is still freezing"
    );
}

/// The one exception, and why it exists: running from another person comes
/// out as an ordinary `Move`, which nothing downstream could tell from a
/// stroll.
#[test]
fn walking_away_from_somebody_is_still_worth_telling_from_a_stroll() {
    let walking = Action::Move { target: (9, 9, 0) };

    assert_eq!(Simulation::what_to_book(&walking, true), "Flee");
    assert_eq!(Simulation::what_to_book(&walking, false), "Move");
}

/// And the invariant that would have caught the defect: a settlement cannot
/// fail at something it was never recorded doing.
#[test]
fn nothing_can_fail_at_a_thing_it_was_never_recorded_doing() {
    let simulation = a_settlement(12, 600);

    for (what, failed) in &simulation.actions_failed {
        let taken = simulation.actions_taken.get(what).copied().unwrap_or(0);

        assert!(
            taken >= *failed,
            "{what} was refused {failed} times and recorded as attempted {taken}; \
             the two tallies are not naming the same thing"
        );
    }
}

// --------------------------------------------------------------------------
// And the threat tree's own tally
// --------------------------------------------------------------------------

/// Every turn is counted, so every share has a denominator.
#[test]
fn every_turn_a_decision_was_made_on_is_counted() {
    let simulation = a_settlement(12, 400);

    let turns = simulation
        .what_a_threat_came_to
        .get("turns decided")
        .copied()
        .unwrap_or(0);

    assert!(turns > 0, "a settlement of twelve over 400 ticks decides something");

    for (what, n) in &simulation.what_a_threat_came_to {
        assert!(
            *n <= turns,
            "{what} was booked {n} times in {turns} turns, which cannot be"
        );
    }
}

/// And the tree's branches add up to the number of times it was asked. This
/// is the conservation check: a branch that is never counted is exactly how
/// #66 came to publish a wrong number about `Freeze`.
#[test]
fn the_branches_of_the_tree_account_for_every_time_it_was_asked() {
    let simulation = a_settlement(12, 600);
    let count = |what: &str| {
        simulation
            .what_a_threat_came_to
            .get(what)
            .copied()
            .unwrap_or(0)
    };

    // Asked once for every turn something was felt strongly enough to act
    // on, less the turns where a higher priority went first
    let asked = count("felt: afraid enough to act") + count("felt: angry enough to act")
        - count("something else came first");

    let came_out = [
        "nothing named",
        "named, but not about",
        "not worth crossing to",
        "a grudge answered before the tree was asked",
        "stands over one of its own",
        "stands its ground",
        "runs",
        "cornered, so fights",
        "cannot fight, so runs",
        "freezes",
    ]
    .iter()
    .map(|what| count(what))
    .sum::<u64>();

    assert_eq!(
        asked, came_out,
        "the tree was asked {asked} times and its branches account for {came_out}"
    );
}

/// And the instrument is wired to the decision rather than only to its own
/// denominator. Put a wolf at somebody's elbow rather than hoping a random
/// world produces one: the first cut of this test ran a settlement for 600
/// ticks and asserted that *something* had been booked, which is a coin toss
/// on whether anybody met an animal.
#[test]
fn the_instrument_is_actually_wired_to_the_decision() {
    use crate::agents::LifeStage;

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = 100.0;

    // Kept at his elbow on purpose. A lone wolf reads the odds against a
    // healthy adult, decides against it and leaves at six paces a tick — see
    // `what_the_beasts_make_of_us` — which is the fauna model working and not
    // what this test is about.
    for _ in 0..24 {
        if let Some(wolf) = simulation.world.animals.get_all_mut().first_mut() {
            wolf.position = (31, 30);
        }
        simulation.tick();
    }

    let on_the_mind = simulation
        .what_a_threat_came_to
        .iter()
        .filter(|(what, _)| what.starts_with("a creature is on the mind"))
        .map(|(_, n)| *n)
        .sum::<u64>();

    assert!(
        on_the_mind > 0,
        "a wolf one pace off should be on somebody's mind, and the counters \
         should say so: {:?}",
        simulation.what_a_threat_came_to
    );
}
