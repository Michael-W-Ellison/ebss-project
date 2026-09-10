// src/analytics/tests/config_tests.rs
//! TDD tests for SimulationConfig
//!
//! These tests define the expected behavior for simulation configuration.

use crate::analytics::SimulationConfig;

#[test]
fn test_simulation_config_default() {
    let config = SimulationConfig::default();

    // Should have reasonable defaults
    assert!(config.max_ticks.is_none(), "Default should have no tick limit");
    assert_eq!(config.enable_logging, true);
    assert_eq!(config.enable_metrics, true);
}

/// Fixing a run is `core::dice::seed`, and this config has nothing to do with
/// it. What was here asserted that `SimulationConfig::default()` carried a
/// `random_seed` - which it did, taken from the wall clock, and which nothing
/// in the model ever read.
#[test]
fn the_seed_that_fixes_a_run_is_the_one_in_dice() {
    let run = || {
        crate::core::dice::seed(77);
        (0..8)
            .map(|_| {
                use rand::Rng;
                crate::core::dice::roll().gen_range(0..1_000_000)
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(run(), run(), "and it is the only one that fixes anything");
}

#[test]
fn test_simulation_config_with_max_ticks() {
    let config = SimulationConfig::default().with_max_ticks(1000);

    assert_eq!(config.max_ticks, Some(1000));
}

#[test]
fn test_simulation_config_disable_logging() {
    let config = SimulationConfig::default().with_logging(false);

    assert_eq!(config.enable_logging, false);
}

#[test]
fn test_simulation_config_disable_metrics() {
    let config = SimulationConfig::default().with_metrics(false);

    assert_eq!(config.enable_metrics, false);
}

#[test]
fn test_simulation_config_builder_pattern() {
    let config = SimulationConfig::default()
        .with_max_ticks(5000)
        .with_logging(true)
        .with_metrics(true);

    assert_eq!(config.max_ticks, Some(5000));
    assert_eq!(config.enable_logging, true);
    assert_eq!(config.enable_metrics, true);
}

#[test]
fn test_simulation_config_validate_valid() {
    let config = SimulationConfig::default()
        .with_max_ticks(1000);

    assert!(config.validate().is_ok());
}

#[test]
fn test_simulation_config_validate_zero_max_ticks() {
    let mut config = SimulationConfig::default();
    config.max_ticks = Some(0);

    let result = config.validate();
    assert!(result.is_err(), "max_ticks of 0 should be invalid");
}

#[test]
fn test_simulation_config_metrics_interval() {
    let config = SimulationConfig::default()
        .with_metrics_interval(10);

    assert_eq!(config.metrics_interval, 10);
}

#[test]
fn test_simulation_config_validate_metrics_interval() {
    let mut config = SimulationConfig::default();
    config.metrics_interval = 0;

    let result = config.validate();
    assert!(result.is_err(), "metrics_interval of 0 should be invalid");
}

#[test]
fn test_simulation_config_clone() {
    let config1 = SimulationConfig::default().with_max_ticks(2000);

    let config2 = config1.clone();

    assert_eq!(config1.max_ticks, config2.max_ticks);
}

#[test]
fn test_simulation_config_debug() {
    let config = SimulationConfig::default().with_max_ticks(123);

    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("123"));
}
