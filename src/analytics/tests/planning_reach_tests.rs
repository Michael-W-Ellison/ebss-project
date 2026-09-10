// src/analytics/tests/planning_reach_tests.rs
//! The plan branch: why it was unreachable, and what it carries now.
//!
//! See ISSUES_FOUND.md #187.

use crate::agents::patterns::{Element, Patterns};
use crate::agents::{Agent, AgentConfig};
use crate::core::planning::PlanActionType;
use crate::core::DriveType;
use crate::environment::Action;

/// Teach an agent a run, hard enough to be followed.
fn teach(agent: &mut Agent, need: DriveType, first: &str, next: &str) {
    let run = Element::Then(first.to_string(), next.to_string());
    for round in 0..14u32 {
        agent.patterns.it_worked(need, &[run.clone()], 0.5, round);
    }
}

// --------------------------------------------------------------------------
// Joining the pairs back up
// --------------------------------------------------------------------------

/// Two overlapping pairs are one chain.
#[test]
fn overlapping_pairs_make_a_chain() {
    let mut patterns = Patterns::default();
    for round in 0..14u32 {
        patterns.it_worked(
            DriveType::Hunger,
            &[
                Element::Then("move".to_string(), "pickup".to_string()),
                Element::Then("pickup".to_string(), "eat".to_string()),
            ],
            0.5,
            round,
        );
    }

    assert_eq!(
        patterns.the_chain_that_answers(DriveType::Hunger, "move"),
        vec!["pickup".to_string(), "eat".to_string()],
        "go to the store, take something out, eat it - held as two pairs and \
         joined back into the three-step run nobody wrote down"
    );
}

/// A chain that comes back to where it started is not a chain.
#[test]
fn a_chain_does_not_eat_its_own_tail() {
    let mut patterns = Patterns::default();
    for round in 0..14u32 {
        patterns.it_worked(
            DriveType::Hunger,
            &[
                Element::Then("gather".to_string(), "eat".to_string()),
                Element::Then("eat".to_string(), "gather".to_string()),
            ],
            0.5,
            round,
        );
    }

    let chain = patterns.the_chain_that_answers(DriveType::Hunger, "gather");
    assert!(
        chain.len() <= Patterns::AS_LONG_A_CHAIN_AS_ANYBODY_HOLDS,
        "a plan that goes round for ever is not a plan: {chain:?}"
    );
    assert!(
        !chain.iter().any(|step| step == "gather"),
        "and it does not come back to where it started: {chain:?}"
    );
}

// --------------------------------------------------------------------------
// What the branch will and will not carry
// --------------------------------------------------------------------------

/// A run of two steps or more is laid down as a plan.
#[test]
fn a_learned_run_becomes_a_plan() {
    let mut agent = Agent::new(AgentConfig::default());
    teach(&mut agent, DriveType::Hunger, "move", "pickup");
    teach(&mut agent, DriveType::Hunger, "pickup", "eat");

    agent.that_is_what_i_just_did(&Action::Move { target: (1, 1, 0) });

    assert!(agent.plan_the_run_that_answers(DriveType::Hunger, 0));
    assert_eq!(agent.what_the_plan_wants_next(), Some("pickup"));
    assert!(agent.is_the_plan_for(DriveType::Hunger));
    assert!(
        agent.should_execute_plan(),
        "a run worked out for a need is what the branch is for"
    );
}

/// One step is not a plan: the reactive reader already answers that.
#[test]
fn one_step_is_not_a_plan() {
    let mut agent = Agent::new(AgentConfig::default());
    teach(&mut agent, DriveType::Hunger, "gather", "eat");
    agent.that_is_what_i_just_did(&Action::Gather {
        resource_type: "berries".to_string(),
    });

    assert_eq!(
        agent.what_usually_comes_next(DriveType::Hunger),
        Some("eat"),
        "the one step is there and the reactive reader has it"
    );
    assert!(
        !agent.plan_the_run_that_answers(DriveType::Hunger, 0),
        "and routing it through the plan machinery as well would be two \
         answers to one question"
    );
}

/// The branch refuses a goal plan, and that is a measurement rather than an
/// oversight.
///
/// Fixing the step counter took agents that would run a plan from 1.3% to
/// 88.8%, and what it made reachable was the goal planner, whose steps are
/// built against hard-coded coordinates - (50, 50, 0) on a fifty-square map.
/// Over 32 seeded worlds that cost 108,235 person-days to 102,708.
#[test]
fn the_branch_will_not_carry_a_plan_built_against_nowhere() {
    use crate::core::planning::{ActionPlan, PlanStep};

    let mut agent = Agent::new(AgentConfig::default());
    agent.current_plan = Some(ActionPlan::new(
        "stock the storehouse".to_string(),
        vec![PlanStep {
            action: PlanActionType::MoveTo { location: (50, 50, 0) },
            estimated_ticks: 10,
            required_tool: None,
            required_resources: Vec::new(),
            target_location: None,
            confidence: 1.0,
        }],
        0,
        "the old planner".to_string(),
    ));

    assert!(agent.has_active_plan());
    assert!(
        !agent.should_execute_plan(),
        "the branch is reachable now, and what it carries is a run the agent \
         worked out - not a step somebody wrote against a corner of the map"
    );
}

// --------------------------------------------------------------------------
// The counter that locked the branch shut
// --------------------------------------------------------------------------

/// A plan that has not been worked is waiting, not stuck.
///
/// The step counter used to tick every turn whether or not the plan ran, and
/// it is the only thing `should_execute_plan` measures staleness by - so a
/// plan the ladder never reached aged out of its step's allowance and could
/// then never be reached at all.
#[test]
fn a_plan_nobody_has_worked_does_not_go_stale_on_its_own() {
    let mut agent = Agent::new(AgentConfig::default());
    teach(&mut agent, DriveType::Hunger, "move", "pickup");
    teach(&mut agent, DriveType::Hunger, "pickup", "eat");
    agent.that_is_what_i_just_did(&Action::Move { target: (1, 1, 0) });
    assert!(agent.plan_the_run_that_answers(DriveType::Hunger, 0));

    assert!(agent.should_execute_plan());
    assert_eq!(
        agent.plan_step_ticks, 0,
        "a plan just laid down has had no turns spent on it"
    );
    assert!(
        agent.should_execute_plan(),
        "and asking about it does not age it"
    );
}

/// But turns spent failing at it do age it, and it is dropped.
#[test]
fn a_plan_that_keeps_failing_is_dropped() {
    let mut agent = Agent::new(AgentConfig::default());
    teach(&mut agent, DriveType::Hunger, "move", "pickup");
    teach(&mut agent, DriveType::Hunger, "pickup", "eat");
    agent.that_is_what_i_just_did(&Action::Move { target: (1, 1, 0) });
    assert!(agent.plan_the_run_that_answers(DriveType::Hunger, 0));

    for _ in 0..20 {
        agent.tick_plan_step();
    }

    assert!(
        !agent.should_execute_plan(),
        "a step that has taken twenty turns of a one-turn allowance is stuck"
    );
}
