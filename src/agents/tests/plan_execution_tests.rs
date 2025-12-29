// src/agents/tests/plan_execution_tests.rs
//! Tests for the plan execution system

use crate::agents::{Agent, AgentConfig};
use crate::core::DriveType;
use crate::core::planning::{ActionPlan, ActionType as PlanActionType, PlanStep};

#[test]
fn test_agent_has_no_plan_by_default() {
    let agent = Agent::new(AgentConfig::default());
    assert!(!agent.has_active_plan());
    assert!(agent.plan_progress().is_none());
}

#[test]
fn test_create_gather_plan() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    let resource_loc = (50, 50, 0);
    let return_loc = (0, 0, 0);

    agent.create_gather_plan("wood", 5, resource_loc, return_loc, 100);

    assert!(agent.has_active_plan());
    assert!(agent.plan_progress().is_some());
    assert_eq!(agent.plan_progress().unwrap(), 0.0);
}

#[test]
fn test_should_execute_plan_when_not_hungry() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Ensure not hungry
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.2;
    }
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.2;
    }

    // Create a plan
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);

    assert!(agent.should_execute_plan());
}

#[test]
fn test_should_not_execute_plan_when_hungry() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create a plan first
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);
    assert!(agent.has_active_plan());

    // Make agent hungry
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.9; // Above threshold
    }

    // Should not execute plan when survival is threatened
    assert!(!agent.should_execute_plan());
}

#[test]
fn test_should_not_execute_plan_when_thirsty() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create a plan first
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);

    // Make agent thirsty
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.9; // Above threshold
    }

    assert!(!agent.should_execute_plan());
}

#[test]
fn test_get_plan_action() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create a plan
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);

    // Get the first plan action (should be MoveTo)
    let action = agent.get_plan_action();
    assert!(action.is_some());

    // Verify it's a Move action
    match action.unwrap() {
        crate::environment::Action::Move { target } => {
            assert_eq!(target, (50, 50, 0));
        }
        _ => panic!("Expected Move action"),
    }
}

#[test]
fn test_advance_plan_step() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create a plan
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);

    let initial_progress = agent.plan_progress().unwrap();
    assert_eq!(initial_progress, 0.0);

    // Advance the plan
    agent.advance_plan_step(true, 10);

    // Progress should have increased
    let new_progress = agent.plan_progress().unwrap();
    assert!(new_progress > initial_progress);
}

#[test]
fn test_abandon_plan() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create and then abandon a plan
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);
    assert!(agent.has_active_plan());

    agent.abandon_plan();
    assert!(!agent.has_active_plan());
}

#[test]
fn test_plan_step_timeout() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Ensure not hungry
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.2;
    }
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.2;
    }

    // Create a plan
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);
    assert!(agent.should_execute_plan());

    // Simulate many ticks on the same step (timeout)
    for _ in 0..1000 {
        agent.tick_plan_step();
    }

    // Should no longer want to execute plan due to timeout
    assert!(!agent.should_execute_plan());
}

#[test]
fn test_plan_completion() {
    let mut agent = Agent::new(AgentConfig::default());

    // Create a simple single-step plan
    let steps = vec![
        PlanStep {
            action: PlanActionType::GatherResource {
                resource: "food".to_string(),
                amount: 1,
            },
            estimated_ticks: 10,
            required_tool: None,
            required_resources: vec![],
            target_location: Some((50, 50, 0)),
            confidence: 0.9,
        },
    ];

    let plan = ActionPlan::new(
        "Test plan".to_string(),
        steps,
        100,
        "testing".to_string(),
    );

    agent.current_plan = Some(plan);
    assert!(agent.has_active_plan());

    // Complete the single step
    agent.advance_plan_step(true, 10);

    // Plan should be cleared after completion
    assert!(!agent.has_active_plan());
}

#[test]
fn test_current_plan_step_description() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // No plan = no description
    assert!(agent.current_plan_step_description().is_none());

    // Create a plan
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);

    // Should have a description
    let desc = agent.current_plan_step_description();
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Moving"));
}

#[test]
fn test_planner_records_outcomes() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Initially no history
    assert!(agent.planner.action_history.is_empty());

    // Create a plan and advance through it
    agent.create_gather_plan("wood", 5, (50, 50, 0), (0, 0, 0), 100);
    agent.advance_plan_step(true, 15); // Record a successful step

    // Should have recorded the outcome
    assert!(!agent.planner.action_history.is_empty());
}
