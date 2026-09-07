// src/analytics/tests/trying_something_else_tests.rs
//! Trying something else when what you are doing is not working.
//!
//! Every drive answers with an ordered list and always took the first rung
//! that would answer, however badly that rung was going. `Lessons` could
//! slacken a particular thing until the drive stood aside altogether - which
//! makes a man do *less*, not *differently* - and the coarse `Undertaking`
//! book could not tell two rungs of the same list apart.
//!
//! What decides it now is how long the need has been asking without being
//! met, which `DriveState::denied_ticks` has counted since drives were given
//! pressure and which nothing had ever read except to make the drive shout.

use crate::agents::practices::Undertaking;
use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::{Action, ActionResult};
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation
}

/// How often a man looks past his habit, over a run of turns.
fn how_often_he_looks_past_it(denied: u32, turns: u32) -> u32 {
    crate::core::dice::seed(1);
    let simulation = one_person();
    let mut agent = simulation.population.agents[0].clone();
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.denied_ticks = denied;
    }

    (0..turns)
        .filter(|_| {
            Simulation::how_far_down_the_list_to_look(&agent, DriveType::Hunger) > 0
        })
        .count() as u32
}

/// A need that has only just started asking is answered the way it always is.
///
/// The habit is the right thing to have while it is working, and most of the
/// time it is working. An agent that reconsiders from first principles every
/// turn is not adaptive, it is incoherent.
#[test]
fn a_man_who_missed_lunch_does_what_he_always_does() {
    for denied in [0, TICKS_PER_DAY, 2 * TICKS_PER_DAY - 1] {
        assert_eq!(
            how_often_he_looks_past_it(denied, 400),
            0,
            "denied for {denied} ticks is not long enough to start experimenting"
        );
    }
}

/// A need that has gone unanswered for days sends him to the next thing down.
#[test]
fn a_man_three_days_hungry_tries_the_other_thing() {
    let sometimes = how_often_he_looks_past_it(4 * TICKS_PER_DAY, 400);

    assert!(
        sometimes > 0,
        "a man four days hungry should sometimes walk past the hedgerow that \
         has not fed him and try the next thing"
    );
    assert!(
        sometimes < 400,
        "and he should not abandon what he knows: {sometimes} turns in 400"
    );
}

/// And the longer it goes on the oftener he tries, up to a cap.
#[test]
fn the_longer_it_goes_on_the_oftener_he_tries() {
    let a_while = how_often_he_looks_past_it(3 * TICKS_PER_DAY, 800);
    let a_long_while = how_often_he_looks_past_it(10 * TICKS_PER_DAY, 800);

    assert!(
        a_long_while > a_while,
        "ten days hungry should send him looking oftener than three: \
         {a_long_while} against {a_while}"
    );

    // The cap: what he knows stays what he mostly does.
    let share = a_long_while as f32 / 800.0;
    assert!(
        share <= Simulation::WHAT_SHARE_OF_TURNS_GO_ON_TRYING_SOMETHING_ELSE * 1.6,
        "he should never spend most of his turns experimenting: {share:.2}"
    );
}

/// He looks one rung further, not to the bottom of the list.
///
/// Trying the next thing is a step, not a rout: a starving man who skipped
/// every rung he knew would end up hunting a deer with his hands.
#[test]
fn he_looks_one_rung_further_and_no_more() {
    crate::core::dice::seed(2);
    let simulation = one_person();
    let mut agent = simulation.population.agents[0].clone();
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.denied_ticks = 60 * TICKS_PER_DAY;
    }

    for _ in 0..500 {
        assert!(
            Simulation::how_far_down_the_list_to_look(&agent, DriveType::Hunger) <= 1,
            "one rung at a time"
        );
    }
}

// --------------------------------------------------------------------------
// And the book he judges it out of
// --------------------------------------------------------------------------

/// Setting string is not trapping succeeding.
///
/// A snare goes into the ground whenever an agent decides to put one there,
/// so `SetSnare` never fails; the round is the half that can come back empty
/// and the only half that produces any food. Counting both against
/// `Undertaking::Trapping` is why nobody could ever find out their trapline
/// was not working: measured over twelve worlds, an agent in winter believed
/// trapping worked at 0.93, out of 18.6 attempts at a 76% success rate - of
/// which 11.8 were snares set.
#[test]
fn setting_string_is_not_a_catch() {
    let mut agent = Agent::new(AgentConfig::default());
    let nowhere: Vec<crate::agents::practices::Circumstance> = Vec::new();

    for _ in 0..20 {
        agent.learn_from_this_here(&Action::SetSnare, true, &nowhere);
    }

    assert_eq!(
        agent.lessons.attempts(Undertaking::Trapping),
        0,
        "twenty snares in the ground says nothing about whether trapping feeds \
         anybody"
    );
    assert_eq!(
        agent.lessons.tried_this("setsnare"),
        20,
        "and the fine record keeps it, because whether he can set a snare is a \
         different question and one he can answer"
    );
}

/// A round that comes back empty is trapping failing, and he can tell.
#[test]
fn a_round_that_comes_back_empty_is_what_he_judges_it_by() {
    let mut agent = Agent::new(AgentConfig::default());
    let nowhere: Vec<crate::agents::practices::Circumstance> = Vec::new();

    // Twelve snares set, and six rounds of which one paid - which is roughly
    // what a settlement's winter looked like.
    for _ in 0..12 {
        agent.learn_from_this_here(&Action::SetSnare, true, &nowhere);
    }
    for round in 0..6 {
        agent.learn_from_this_here(&Action::CheckSnares, round == 0, &nowhere);
    }

    assert_eq!(
        agent.lessons.attempts(Undertaking::Trapping),
        6,
        "six rounds is what he has to go on"
    );
    assert_eq!(agent.lessons.successes(Undertaking::Trapping), 1);
    assert!(
        agent.lessons.success_rate(Undertaking::Trapping) < 0.3,
        "and one round in six is what it comes to, not five in six: {}",
        agent.lessons.success_rate(Undertaking::Trapping)
    );
}

/// The whole of it, through the live path: a settlement that cannot find
/// anything does not spend every turn of the winter on the same empty walk.
#[test]
fn a_settlement_that_is_not_being_fed_does_not_do_only_the_one_thing() {
    let _ = ActionResult::success();
    crate::core::dice::seed(7);

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..6 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    for _ in 0..(20 * TICKS_PER_DAY) {
        simulation.tick();
        if simulation.population.agents.is_empty() {
            break;
        }
    }

    let ways: usize = simulation
        .actions_taken
        .iter()
        .filter(|(_, n)| **n > 0)
        .count();

    assert!(
        ways >= 4,
        "twenty days of six people living should show more than a handful of \
         different things done: {:?}",
        simulation.actions_taken
    );
}
