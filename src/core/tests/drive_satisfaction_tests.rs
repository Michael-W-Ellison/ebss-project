// src/core/tests/drive_satisfaction_tests.rs
//! TDD tests for the drive satisfaction system
//!
//! These tests verify that the drive system correctly:
//! - Accumulates drive values over time
//! - Triggers at appropriate thresholds
//! - Satisfies drives properly when needs are met
//! - Handles multiple simultaneous drives

use crate::core::drives::{Drive, DriveType, DriveState};

// Floating-point comparison tolerance
const EPSILON: f32 = 0.0001;

#[test]
fn test_drive_accumulation_over_time() {
    let mut drive = Drive::new(DriveType::Hunger);

    // Start at zero
    assert_eq!(drive.value, 0.0);

    // Accumulate over time (Hunger has 0.01 base rate)
    drive.tick();
    assert!((drive.value - 0.01).abs() < EPSILON, "Expected ~0.01, got {}", drive.value);

    // Continue accumulating
    for _ in 0..99 {
        drive.tick();
    }

    // After 100 ticks, should be at 1.0
    assert!((drive.value - 1.0).abs() < EPSILON, "Expected ~1.0, got {}", drive.value);

    // Should not exceed 1.0
    drive.tick();
    assert!((drive.value - 1.0).abs() < EPSILON, "Expected ~1.0, got {}", drive.value);
}

#[test]
fn test_drive_threshold_activation() {
    let mut drive = Drive::new(DriveType::Hunger);

    // Hunger threshold is 0.7
    assert_eq!(drive.threshold, 0.7);

    // Not active yet
    assert!(!drive.is_active());

    // Accumulate past threshold (use 71 ticks to account for floating-point precision)
    // 71 * 0.01 = 0.71, ensuring we're definitely above 0.7
    for _ in 0..71 {
        drive.tick();
    }

    // Now should be active
    assert!(drive.value > drive.threshold,
            "Expected value ({}) to be > threshold ({})", drive.value, drive.threshold);
    assert!(drive.is_active());
}

#[test]
fn test_drive_satisfaction_resets_value() {
    let mut drive = Drive::new(DriveType::Hunger);

    // Accumulate some hunger
    for _ in 0..50 {
        drive.tick();
    }

    assert!((drive.value - 0.5).abs() < EPSILON, "Expected ~0.5, got {}", drive.value);

    // Satisfy the drive (eating food)
    drive.satisfy();

    // Should reset to zero
    assert_eq!(drive.value, 0.0);
}

#[test]
fn test_partial_drive_satisfaction() {
    let mut drive = Drive::new(DriveType::Hunger);

    // Accumulate to 0.8
    for _ in 0..80 {
        drive.tick();
    }

    assert!((drive.value - 0.8).abs() < EPSILON, "Expected ~0.8, got {}", drive.value);

    // Partially satisfy (e.g., small snack reduces by 0.3)
    drive.partial_satisfy(0.3);

    // Should be reduced but not zero
    assert!((drive.value - 0.5).abs() < EPSILON, "Expected ~0.5, got {}", drive.value);
}

#[test]
fn test_multiple_drives_accumulate_independently() {
    let mut hunger = Drive::new(DriveType::Hunger);
    let mut thirst = Drive::new(DriveType::Thirst);
    let mut rest = Drive::new(DriveType::Rest);

    // Accumulate for 100 ticks
    for _ in 0..100 {
        hunger.tick();  // 0.01/tick
        thirst.tick();  // 0.012/tick
        rest.tick();    // 0.008/tick
    }

    // Each should accumulate at its own rate
    assert!((hunger.value - 1.0).abs() < EPSILON, "Hunger expected ~1.0, got {}", hunger.value);  // Capped at 1.0
    assert!((thirst.value - 1.0).abs() < 0.01, "Thirst expected ~1.0, got {}", thirst.value); // Should be at cap
    assert!((rest.value - 0.8).abs() < EPSILON, "Rest expected ~0.8, got {}", rest.value);    // 100 * 0.008
}

#[test]
fn test_satisfying_one_drive_doesnt_affect_others() {
    let mut hunger = Drive::new(DriveType::Hunger);
    let mut thirst = Drive::new(DriveType::Thirst);

    // Accumulate both
    for _ in 0..50 {
        hunger.tick();
        thirst.tick();
    }

    let hunger_before = hunger.value;
    let thirst_before = thirst.value;

    // Satisfy hunger (eat food)
    hunger.satisfy();

    // Hunger should be zero
    assert_eq!(hunger.value, 0.0);

    // Thirst should be unchanged
    assert_eq!(thirst.value, thirst_before);
}

#[test]
fn test_drive_state_initialization() {
    let drive_state = DriveState::new();

    // Should have all 14 drives
    assert_eq!(drive_state.drives.len(), 14);

    // All should start at zero
    for drive in &drive_state.drives {
        assert_eq!(drive.value, 0.0);
    }
}

#[test]
fn test_drive_state_get_drive() {
    let drive_state = DriveState::new();

    // Should be able to get specific drives
    let hunger = drive_state.get(DriveType::Hunger);
    assert!(hunger.is_some());
    assert_eq!(hunger.unwrap().drive_type, DriveType::Hunger);

    let thirst = drive_state.get(DriveType::Thirst);
    assert!(thirst.is_some());
    assert_eq!(thirst.unwrap().drive_type, DriveType::Thirst);
}

#[test]
fn test_drive_state_update_all_drives() {
    let mut drive_state = DriveState::new();

    // Tick all drives
    drive_state.tick();

    // Each drive should have accumulated
    let hunger = drive_state.get(DriveType::Hunger).unwrap();
    assert!((hunger.value - 0.01).abs() < EPSILON, "Expected ~0.01, got {}", hunger.value);

    let thirst = drive_state.get(DriveType::Thirst).unwrap();
    assert!((thirst.value - 0.012).abs() < EPSILON, "Expected ~0.012, got {}", thirst.value);
}

#[test]
fn test_drive_state_get_most_urgent() {
    let mut drive_state = DriveState::new();

    // Accumulate drives at different rates
    for _ in 0..100 {
        drive_state.tick();
    }

    // Most urgent should be the one with highest weighted value
    let most_urgent = drive_state.get_most_urgent();
    assert!(most_urgent.is_some());

    // Should be one of the fast-accumulating drives
    let urgent_type = most_urgent.unwrap().drive_type;
    assert!(
        urgent_type == DriveType::Hunger ||
        urgent_type == DriveType::Thirst ||
        urgent_type == DriveType::Safety
    );
}

#[test]
fn test_drive_priority_with_weights() {
    let mut drive_state = DriveState::new();

    // Set different weights for drives
    if let Some(hunger) = drive_state.get_mut(DriveType::Hunger) {
        hunger.weight = 2.0;  // High priority
    }

    if let Some(curiosity) = drive_state.get_mut(DriveType::Curiosity) {
        curiosity.weight = 0.5;  // Low priority
    }

    // Accumulate to same value
    if let Some(hunger) = drive_state.get_mut(DriveType::Hunger) {
        hunger.value = 0.5;
    }

    if let Some(curiosity) = drive_state.get_mut(DriveType::Curiosity) {
        curiosity.value = 0.5;
    }

    // Hunger should have higher priority (0.5 * 2.0 = 1.0)
    let hunger_priority = drive_state.get(DriveType::Hunger).unwrap().priority();
    let curiosity_priority = drive_state.get(DriveType::Curiosity).unwrap().priority();

    assert_eq!(hunger_priority, 1.0);
    assert_eq!(curiosity_priority, 0.25);
    assert!(hunger_priority > curiosity_priority);
}

#[test]
fn test_thirst_accumulates_faster_than_hunger() {
    let mut hunger = Drive::new(DriveType::Hunger);
    let mut thirst = Drive::new(DriveType::Thirst);

    // Same number of ticks
    for _ in 0..50 {
        hunger.tick();  // 0.01/tick
        thirst.tick();  // 0.012/tick
    }

    // Thirst should be higher
    assert!((hunger.value - 0.5).abs() < EPSILON, "Expected ~0.5, got {}", hunger.value);
    assert!((thirst.value - 0.6).abs() < EPSILON, "Expected ~0.6, got {}", thirst.value);
    assert!(thirst.value > hunger.value);
}

#[test]
fn test_survival_drives_have_high_thresholds() {
    // Survival drives should activate at high urgency
    let hunger = Drive::new(DriveType::Hunger);
    let thirst = Drive::new(DriveType::Thirst);
    let rest = Drive::new(DriveType::Rest);
    let safety = Drive::new(DriveType::Safety);

    // All survival drives should have thresholds >= 0.6
    assert!(hunger.threshold >= 0.6);
    assert!(thirst.threshold >= 0.6);
    assert!(rest.threshold >= 0.6);
    assert!(safety.threshold >= 0.6);
}

#[test]
fn test_luxury_drives_have_low_thresholds() {
    // Luxury/optional drives should activate more easily
    let luxury = Drive::new(DriveType::Luxury);
    let curiosity = Drive::new(DriveType::Curiosity);

    // Should have low thresholds
    assert!(luxury.threshold <= 0.2);
    assert!(curiosity.threshold <= 0.3);
}

#[test]
fn test_drive_decrease_cannot_go_negative() {
    let mut drive = Drive::new(DriveType::Hunger);

    // Start at low value
    drive.value = 0.1;

    // Try to decrease by more than current value
    drive.decrease(0.5);

    // Should clamp at zero, not go negative
    assert_eq!(drive.value, 0.0);
}

#[test]
fn test_drive_increase_cannot_exceed_one() {
    let mut drive = Drive::new(DriveType::Hunger);

    // Start near max
    drive.value = 0.95;

    // Try to increase beyond 1.0
    drive.increase(0.5);

    // Should clamp at 1.0
    assert_eq!(drive.value, 1.0);
}

#[test]
fn test_all_drive_types_have_valid_defaults() {
    for drive_type in DriveType::all() {
        let drive = Drive::new(drive_type);

        // Threshold should be between 0 and 1
        assert!(drive.threshold >= 0.0 && drive.threshold <= 1.0);

        // Accumulation rate should be positive
        assert!(drive_type.base_accumulation_rate() > 0.0);

        // Should have a satisfaction description
        assert!(!drive_type.satisfaction_description().is_empty());
    }
}
