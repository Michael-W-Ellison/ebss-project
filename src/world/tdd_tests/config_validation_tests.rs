// src/world/tests/config_validation_tests.rs
//! TDD tests for WorldConfig and ResourceConfig validation
//!
//! These tests ensure that invalid configurations are caught early.

use crate::world::{WorldConfig, ResourceConfig};

#[test]
fn test_valid_world_config() {
    let config = WorldConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_world_config_zero_width() {
    let mut config = WorldConfig::default();
    config.size = (0, 50);

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("width"));
}

#[test]
fn test_world_config_zero_height() {
    let mut config = WorldConfig::default();
    config.size = (50, 0);

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("height"));
}

#[test]
fn test_world_config_too_small() {
    let mut config = WorldConfig::default();
    config.size = (5, 5); // Too small for agents to move around

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("minimum"));
}

#[test]
fn test_world_config_too_large() {
    let mut config = WorldConfig::default();
    config.size = (10000, 10000); // Unreasonably large

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("maximum"));
}

#[test]
fn test_world_config_minimum_size_valid() {
    let mut config = WorldConfig::default();
    config.size = (10, 10); // Minimum reasonable size

    assert!(config.validate().is_ok());
}

#[test]
fn test_world_config_large_but_valid() {
    let mut config = WorldConfig::default();
    config.size = (1000, 1000); // Large but valid

    assert!(config.validate().is_ok());
}

#[test]
fn test_resource_config_all_resources_present() {
    let config = WorldConfig::default();

    assert!(config.initial_resources.wood_nodes > 0);
    assert!(config.initial_resources.stone_nodes > 0);
    assert!(config.initial_resources.iron_nodes > 0);
    assert!(config.initial_resources.food_nodes > 0);
}

#[test]
fn test_resource_config_can_have_zero_nodes() {
    // Zero resources should be valid (desert world scenario)
    let mut config = WorldConfig::default();
    config.initial_resources.wood_nodes = 0;
    config.initial_resources.stone_nodes = 0;

    assert!(config.validate().is_ok());
}

#[test]
fn test_resource_config_excessive_resources() {
    let mut config = WorldConfig::default();
    config.size = (50, 50); // 2500 tiles
    config.initial_resources.wood_nodes = 5000; // More resources than tiles

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("resource nodes"));
}

#[test]
fn test_world_config_builder_pattern() {
    let config = WorldConfig::default()
        .with_size(100, 100)
        .with_resources(ResourceConfig {
            wood_nodes: 50,
            stone_nodes: 40,
            iron_nodes: 20,
            food_nodes: 60,
            ..Default::default()
        });

    assert_eq!(config.size, (100, 100));
    assert_eq!(config.initial_resources.wood_nodes, 50);
    assert!(config.validate().is_ok());
}
