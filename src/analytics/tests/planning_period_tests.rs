// src/analytics/tests/planning_period_tests.rs
//! When somebody may stop and think, and when they may not.
//!
//! "New decisions can be made every 30 ticks... An agent that is going to
//! travel for 105 minutes/ticks to go hunting need not make a new plan 30
//! ticks later."
//!
//! Two clocks: a tick is a minute and there are 1,440 in a day; a planning
//! period is thirty of them and there are forty-eight. What these hold is the
//! gate between them - that an undertaking carries its agent past the periods
//! it spans, and that danger takes the gate away.

use crate::agents::{Agent, AgentConfig, AgentState, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::{
    PLANNING_PERIODS_PER_DAY, TICKS_BETWEEN_PLANS, TICKS_PER_DAY,
};
use crate::prelude::Action;
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(world, population)
}

/// An action is as long as its longest verb, not as long as all of them.
///
/// `done_by` maps many verbs onto one action name, and the verbs under a name
/// are alternatives rather than steps. `HARVEST` and `DRINK` are both
/// `done_by: Some("gather")`; `HUNT` and `THROW` are both `done_by:
/// Some("hunt")`. `verbs::what_this_action_costs` adds them up, which answers
/// "what does this family come to" and not "how long does one of these hold
/// somebody".
///
/// Pricing the gate off the sum charged a gather for a drink it never took.
/// Every gather in the model became two periods and every hunt three, so a
/// gathering agent decided half as often as it should and a hunting one a
/// third - a change to how often everybody in the simulation acts, arrived at
/// by reading a field for something it does not say.
#[test]
fn an_action_is_as_long_as_its_longest_verb_not_the_sum_of_them() {
    use crate::environment::verbs::{what_this_action_costs, EVERY_VERB};

    // The case that was wrong. Gathering is performed by more than one verb,
    // so the sum and the longest are different numbers, and the gate wants
    // the longest.
    let under_gather = EVERY_VERB
        .iter()
        .filter(|verb| verb.done_by == Some("gather") && verb.always)
        .count();
    assert!(
        under_gather > 1,
        "the test is about an action several verbs share, and gather is \
         performed by {under_gather}"
    );

    let gathering = Action::Gather { resource_type: "wood".to_string() };
    let holds_you = AgentState::how_long_this_takes(&gathering);
    let all_of_them = (what_this_action_costs("gather").time * TICKS_BETWEEN_PLANS as f32) as u32;

    assert!(
        holds_you < all_of_them,
        "a gather holds somebody {holds_you} ticks and the whole family comes \
         to {all_of_them}; charging the one for the other is the bug"
    );
    assert_eq!(
        holds_you, TICKS_BETWEEN_PLANS,
        "no verb is priced above a moment yet, so a gather is one period"
    );

    // And the general statement, so pricing a verb later cannot quietly put
    // the sum back. Whatever any action is worth, it is worth its longest
    // verb, floored at a period.
    for name in ["gather", "hunt", "craft", "move", "eat"] {
        let longest = EVERY_VERB
            .iter()
            .filter(|verb| verb.done_by == Some(name) && verb.always)
            .map(|verb| verb.costs.time)
            .fold(1.0_f32, f32::max);
        let sum = what_this_action_costs(name).time;
        assert!(
            longest <= sum.max(1.0),
            "{name}: the longest verb ({longest}) cannot exceed the sum ({sum})"
        );
    }
}

/// The two clocks are the two the specification gives.
#[test]
fn a_tick_is_a_minute_and_a_period_is_half_an_hour() {
    assert_eq!(TICKS_PER_DAY, 1_440, "one tick a minute, and 1,440 in a day");
    assert_eq!(TICKS_BETWEEN_PLANS, 30, "a new decision every thirty ticks");
    assert_eq!(
        PLANNING_PERIODS_PER_DAY, 48,
        "which is forty-eight chances to think in a day"
    );
    assert_eq!(
        PLANNING_PERIODS_PER_DAY * TICKS_BETWEEN_PLANS,
        TICKS_PER_DAY,
        "and the two clocks have to agree about how long a day is"
    );
}

/// A day of stepping puts a day on the clock.
///
/// The step is a planning period and the count is in ticks, so this is the
/// one that catches the two coming apart - which is what happened when the
/// counter still advanced by one.
#[test]
fn a_day_of_thinking_is_a_day_of_ticks() {
    let mut simulation = one_person();
    let started = simulation.current_turn;

    for _ in 0..PLANNING_PERIODS_PER_DAY {
        simulation.take_a_turn();
    }

    assert_eq!(
        simulation.current_turn - started,
        TICKS_PER_DAY,
        "forty-eight periods is a day, and a day is 1,440 ticks"
    );
}

/// What the matrix says a thing costs is what holds somebody to it.
#[test]
fn what_an_undertaking_costs_is_what_it_holds_you_for() {
    // Nothing in the matrix is priced under a period, and nothing may be:
    // an action finishing inside one would let its agent think twice at the
    // same gate.
    for action in [
        Action::Eat { food_type: "food".to_string() },
        Action::Gather { resource_type: "wood".to_string() },
        Action::Craft { item_type: "handaxe".to_string() },
    ] {
        let takes = AgentState::how_long_this_takes(&action);
        assert!(
            takes >= TICKS_BETWEEN_PLANS,
            "{action:?} came to {takes} ticks, which is less than a period"
        );
        assert_eq!(
            takes % TICKS_BETWEEN_PLANS,
            0,
            "{action:?} came to {takes}, which is not a whole number of periods"
        );
    }

    // And sleeping says its own length, which is the case the specification
    // calls out for skipping.
    let a_long_night = AgentState::how_long_this_takes(&Action::Sleep { duration: 8 });
    assert_eq!(
        a_long_night,
        8 * TICKS_BETWEEN_PLANS,
        "eight periods of sleep is eight periods of not thinking"
    );
    assert!(
        a_long_night > AgentState::how_long_this_takes(&Action::Sleep { duration: 1 }),
        "and a longer sleep holds somebody longer than a shorter one"
    );
}

/// The worked example from the specification, run on the gate itself.
///
/// "The agent begins its hunting plan. It skips the next three planning
/// periods as the agent is still hunting. After 105 ticks, the agent completes
/// its hunt and waits another 15 ticks (tick 120) to make a new plan."
#[test]
fn an_undertaking_carries_its_agent_past_the_gates_it_spans() {
    let busy_until = 105;

    let gates: Vec<u32> = (0..=5).map(|n| n * TICKS_BETWEEN_PLANS).collect();
    let passed: Vec<u32> = gates
        .iter()
        .copied()
        .filter(|now| *now > 0 && busy_until > *now)
        .collect();

    assert_eq!(
        passed,
        vec![30, 60, 90],
        "a hundred and five ticks of hunting is three gates walked past"
    );

    let thinks_again = gates.iter().copied().find(|now| busy_until <= *now && *now > 0);
    assert_eq!(
        thinks_again,
        Some(120),
        "and the next thought is at 120, fifteen ticks after the hunt ended"
    );
}

/// Somebody busy is not asked what they want.
#[test]
fn nobody_in_the_middle_of_something_is_asked_again() {
    let mut simulation = one_person();

    // Busy for four periods from now.
    let now = simulation.current_turn;
    simulation.population.agents[0].busy_until = now + 4 * TICKS_BETWEEN_PLANS;
    let stood = simulation.population.agents[0].state.position;

    // Three periods pass. Nothing this one decided can have moved it, because
    // it was never asked.
    for _ in 0..3 {
        simulation.take_a_turn();
    }

    assert!(
        simulation.population.agents[0].busy_until > simulation.current_turn,
        "four periods of work is not done after three"
    );
    assert_eq!(
        simulation.population.agents[0].state.position, stood,
        "and nothing it chose moved it, because it chose nothing"
    );
}

/// But danger takes the gate away, and the plan with it.
///
/// **Asserted on the rule, not on a whole turn, and here is why.** The
/// carve-out reads `Emotions::in_danger` at the moment of deciding, and
/// emotions decay inside `Population::take_a_turn`, which runs before the
/// decision phase. So fear set by a fixture before a turn is gone by the time
/// the gate looks, and a test that steps a whole turn measures the decay
/// rather than the carve-out. In a live settlement the fear is put there by
/// `process_predator_attacks`, which runs in the same turn *before*
/// `everybody_takes_a_turn`, so the production path sees it.
///
/// What is checked here is that the fixture's danger is real danger - a man is
/// frightened *of something*, `should_flee` reading the worst of
/// `fear_sources` rather than the bare `fear` field, which is the mistake this
/// test made first - and that a committed agent is committed until it.
#[test]
fn danger_is_of_something_rather_than_a_mood() {
    let mut simulation = one_person();

    simulation.population.agents[0].emotions.fear = 1.0;
    assert!(
        !simulation.population.agents[0].emotions.in_danger(),
        "a bare mood is not danger: `should_flee` reads what he is afraid *of*"
    );

    simulation.population.agents[0]
        .emotions
        .fear_sources
        .insert(crate::agents::EmotionSource::Creature("wolf".to_string()), 1.0);
    assert!(
        simulation.population.agents[0].emotions.in_danger(),
        "and a wolf he is afraid of is"
    );
}

/// A newly made body is free to think.
#[test]
fn nobody_is_born_busy() {
    let agent = Agent::new(AgentConfig::default());
    assert_eq!(agent.busy_until, 0);
    assert!(
        !(agent.busy_until > 0),
        "zero means free, and the test is `>` rather than `>=` so that a body \
         made at tick zero is not busy at tick zero"
    );
}
