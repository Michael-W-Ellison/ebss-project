// src/agents/tests/plan_execution_tests.rs
//! Tests for the plan execution system

use crate::agents::{Agent, AgentConfig};
use crate::core::DriveType;
use crate::core::planning::{ActionPlan, ActionType as PlanActionType, PlanStep};
use crate::core::{Goal, GoalWorldState, ExternalGoal};

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

// === Tests for plan interruption when goals are satisfied by external changes ===

#[test]
fn test_plan_abandoned_when_storehouse_already_stocked() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create a goal to contribute materials to storehouse
    let goal = Goal::new_external(
        ExternalGoal::ContributeMaterialsToStorehouse(50),
        0.8,
        100,
    );
    agent.goals.add_goal(goal);

    // Create a plan to gather wood for the storehouse
    let steps = vec![
        PlanStep {
            action: PlanActionType::MoveTo { location: (50, 50, 0) },
            estimated_ticks: 10,
            required_tool: None,
            required_resources: vec![],
            target_location: Some((50, 50, 0)),
            confidence: 0.9,
        },
        PlanStep {
            action: PlanActionType::GatherResource {
                resource: "wood".to_string(),
                amount: 50,
            },
            estimated_ticks: 20,
            required_tool: None,
            required_resources: vec![],
            target_location: Some((50, 50, 0)),
            confidence: 0.8,
        },
    ];

    let plan = ActionPlan::new(
        "Gather wood for storehouse".to_string(),
        steps,
        100,
        "testing".to_string(),
    );
    agent.current_plan = Some(plan);

    assert!(agent.has_active_plan());

    // Now simulate storehouse being restocked by someone else
    let world_state = GoalWorldState {
        storehouse_food: 0,
        storehouse_materials: 100, // Exceeds the goal target of 50
        storehouse_tools: 0,
        personal_food: 0,
        gathered_resources: 0,
        owns_house: false,
        has_protection: false,
    };

    // Check plan relevance - should return false (goal already satisfied)
    assert!(!agent.is_plan_still_relevant(&world_state));

    // Update plan relevance should abandon the plan
    agent.update_plan_relevance(&world_state);

    // Plan should be abandoned
    assert!(!agent.has_active_plan());
}

#[test]
fn test_goal_marked_complete_when_satisfied_by_world_state() {
    let mut agent = Agent::new(AgentConfig::default());

    // Create a goal to contribute food to storehouse
    let goal = Goal::new_external(
        ExternalGoal::ContributeFoodToStorehouse(30),
        0.9,
        100,
    );
    agent.goals.add_goal(goal);

    // Verify goal is not yet complete
    assert!(!agent.goals.goals[0].completed);
    assert_eq!(agent.goals.goals[0].progress, 0.0);

    // Simulate storehouse having enough food
    let world_state = GoalWorldState {
        storehouse_food: 50, // Exceeds the goal target of 30
        storehouse_materials: 0,
        storehouse_tools: 0,
        personal_food: 0,
        gathered_resources: 0,
        owns_house: false,
        has_protection: false,
    };

    // Update plan relevance (which also marks satisfied goals as complete)
    agent.update_plan_relevance(&world_state);

    // Goal should now be marked as complete
    assert!(agent.goals.goals[0].completed);
    assert_eq!(agent.goals.goals[0].progress, 1.0);
}

#[test]
fn test_plan_continues_when_goal_not_satisfied() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.position = (10, 10, 0);

    // Create a goal to contribute materials to storehouse
    let goal = Goal::new_external(
        ExternalGoal::ContributeMaterialsToStorehouse(100),
        0.8,
        100,
    );
    agent.goals.add_goal(goal);

    // Create a plan for this goal
    let steps = vec![
        PlanStep {
            action: PlanActionType::GatherResource {
                resource: "wood".to_string(),
                amount: 50,
            },
            estimated_ticks: 20,
            required_tool: None,
            required_resources: vec![],
            target_location: Some((50, 50, 0)),
            confidence: 0.8,
        },
    ];

    let plan = ActionPlan::new(
        "Gather wood for storehouse".to_string(),
        steps,
        100,
        "testing".to_string(),
    );
    agent.current_plan = Some(plan);

    // World state still short of goal (only 30 materials, need 100)
    let world_state = GoalWorldState {
        storehouse_food: 0,
        storehouse_materials: 30, // Below the goal target of 100
        storehouse_tools: 0,
        personal_food: 0,
        gathered_resources: 0,
        owns_house: false,
        has_protection: false,
    };

    // Plan should still be relevant
    assert!(agent.is_plan_still_relevant(&world_state));

    // Update plan relevance should NOT abandon the plan
    agent.update_plan_relevance(&world_state);

    // Plan should still be active
    assert!(agent.has_active_plan());
}

#[test]
fn test_personal_food_goal_satisfied() {
    let mut agent = Agent::new(AgentConfig::default());

    // Create a goal to stock house with food
    let goal = Goal::new_external(
        ExternalGoal::StockHouseFood(20),
        0.7,
        100,
    );
    agent.goals.add_goal(goal);

    // Simulate having enough personal food
    let world_state = GoalWorldState {
        storehouse_food: 0,
        storehouse_materials: 0,
        storehouse_tools: 0,
        personal_food: 25, // Exceeds the goal target of 20
        gathered_resources: 0,
        owns_house: false,
        has_protection: false,
    };

    // Goal should be satisfied
    assert!(agent.goals.goals[0].is_satisfied(&world_state));

    // Update plan relevance marks goal as complete
    agent.update_plan_relevance(&world_state);

    assert!(agent.goals.goals[0].completed);
}

#[test]
fn test_tools_goal_satisfied() {
    let mut agent = Agent::new(AgentConfig::default());

    // Create a goal to ensure tools are available
    let goal = Goal::new_external(
        ExternalGoal::EnsureToolsAvailable(5),
        0.6,
        100,
    );
    agent.goals.add_goal(goal);

    // Simulate storehouse having enough tools
    let world_state = GoalWorldState {
        storehouse_food: 0,
        storehouse_materials: 0,
        storehouse_tools: 10, // Exceeds the goal target of 5
        personal_food: 0,
        gathered_resources: 0,
        owns_house: false,
        has_protection: false,
    };

    // Goal should be satisfied
    assert!(agent.goals.goals[0].is_satisfied(&world_state));
}
