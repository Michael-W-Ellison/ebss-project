// src/world/tdd_tests/action_execution_tests.rs
//! TDD tests for the action execution system - SIMPLIFIED
//!
//! Note: Many action execution tests require complex world setup and are better
//! tested at the simulation level using test_simulation.rs. These tests focus
//! on basic action result behavior.

use crate::world::{ActionResult, ItemType};

#[test]
fn test_action_result_success_detection() {
    let success = ActionResult::Success {
        message: "Done".to_string(),
    };

    let success_with_items = ActionResult::SuccessWithItems {
        message: "Harvested".to_string(),
        item_type: ItemType::Wood,
        quantity: 10,
    };

    let failure = ActionResult::Failure {
        reason: "No resource".to_string(),
    };

    assert!(success.is_success());
    assert!(success_with_items.is_success());
    assert!(!failure.is_success());
}

#[test]
fn test_action_result_item_extraction() {
    let result = ActionResult::SuccessWithItems {
        message: "Harvested wood".to_string(),
        item_type: ItemType::Wood,
        quantity: 15,
    };

    let (item_type, quantity) = result.take_items().unwrap();
    assert_eq!(item_type, ItemType::Wood);
    assert_eq!(quantity, 15);
}

#[test]
fn test_partial_action_result() {
    let partial = ActionResult::Partial {
        completed: 0.6,
        message: "60% complete".to_string(),
    };

    // Partial results are not considered full success
    assert!(!partial.is_success());
}

#[test]
fn test_social_action_result_extracts_satisfaction() {
    let social = ActionResult::SocialSuccess {
        message: "Had nice chat".to_string(),
        relationship_change: 5,
        trust_change: 2,
        social_satisfaction: 0.3,
    };

    assert!(social.is_success());
    assert_eq!(social.social_satisfaction(), 0.3);

    let (rel_change, trust_change) = social.relationship_change();
    assert_eq!(rel_change, 5);
    assert_eq!(trust_change, 2);
}
