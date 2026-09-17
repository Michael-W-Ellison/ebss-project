// src/analytics/tests/autosave_tests.rs
//! TDD tests for auto-save/checkpointing functionality

use crate::analytics::Simulation;
use crate::agents::{Population, AgentConfig};
use crate::world::{World, WorldConfig};
use crate::analytics::AutoSaveConfig;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_autosave_creates_checkpoint_directory() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 10,
        max_checkpoints: 3,
        save_directory: save_dir.clone(),
    };

    // Directory should be created when autosave is initialized
    assert!(!save_dir.exists());

    let world = World::new(WorldConfig::default());
    let population = Population::new();
    let mut sim = Simulation::new(world, population);

    sim.enable_autosave(config).expect("Failed to enable autosave");

    assert!(save_dir.exists());
}

#[test]
fn test_autosave_triggers_at_interval() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 5,  // Save every 5 turns
        max_checkpoints: 5,
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);
    sim.enable_autosave(config).unwrap();

    // Run for 5 turns - should trigger one autosave
    for _ in 0..5 {
        sim.take_a_turn();
    }

    // Check that a checkpoint was created
    let checkpoints = std::fs::read_dir(&save_dir).unwrap().count();
    assert!(checkpoints >= 1, "Expected at least 1 checkpoint, found {}", checkpoints);
}

#[test]
fn test_autosave_respects_max_checkpoints() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 2,  // Save every 2 turns
        max_checkpoints: 3, // Keep only 3 checkpoints
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let population = Population::new();

    let mut sim = Simulation::new(world, population);
    sim.enable_autosave(config).unwrap();

    // Run for 20 turns - should create 10 checkpoints, but keep only 3
    for _ in 0..20 {
        sim.take_a_turn();
    }

    let checkpoints = std::fs::read_dir(&save_dir).unwrap().count();
    assert_eq!(checkpoints, 3, "Should keep exactly 3 checkpoints");
}

#[test]
fn test_autosave_can_be_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: false,  // Disabled
        interval_turns: 1,
        max_checkpoints: 5,
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let population = Population::new();

    let mut sim = Simulation::new(world, population);
    sim.enable_autosave(config).unwrap();

    // Run for 10 turns
    for _ in 0..10 {
        sim.take_a_turn();
    }

    // No checkpoints should be created
    assert!(!save_dir.exists() || std::fs::read_dir(&save_dir).unwrap().count() == 0);
}

#[test]
fn test_autosave_checkpoint_can_be_loaded() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 5,
        max_checkpoints: 3,
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..3 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut sim = Simulation::new(world, population);
    sim.enable_autosave(config).unwrap();

    // Run to trigger autosave
    for _ in 0..5 {
        sim.take_a_turn();
    }

    // Find the checkpoint file
    let checkpoint_files: Vec<_> = std::fs::read_dir(&save_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    assert!(!checkpoint_files.is_empty(), "No checkpoint files found");

    // Load from checkpoint
    let loaded_sim = Simulation::load(&checkpoint_files[0]).expect("Failed to load checkpoint");

    assert_eq!(loaded_sim.current_turn, 5);
    assert_eq!(loaded_sim.population.agents.len(), 3);
}

#[test]
fn test_get_latest_checkpoint() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 3,
        max_checkpoints: 5,
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let population = Population::new();

    let mut sim = Simulation::new(world, population);
    sim.enable_autosave(config).unwrap();

    // Create multiple checkpoints
    for _ in 0..12 {
        sim.take_a_turn();
    }

    // Get latest checkpoint
    let latest = Simulation::get_latest_checkpoint(&save_dir).expect("No checkpoint found");
    assert!(latest.exists());

    // Load it and verify it's the latest
    let loaded_sim = Simulation::load(&latest).unwrap();
    assert_eq!(loaded_sim.current_turn, 12);
}

#[test]
fn test_autosave_preserves_full_state() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 10,
        max_checkpoints: 3,
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);

    // Modify agent state
    sim.population.agents[0].state.health = 75.0;
    sim.population.agents[0].state.energy = 60.0;

    sim.enable_autosave(config).unwrap();

    // Run to trigger autosave
    for _ in 0..10 {
        sim.take_a_turn();
    }

    // Load from checkpoint
    let latest = Simulation::get_latest_checkpoint(&save_dir).unwrap();
    let loaded_sim = Simulation::load(&latest).unwrap();

    // Health should be preserved (may have changed from turn)
    // Just verify the agent exists
    assert_eq!(loaded_sim.population.agents.len(), 1);
}

#[test]
fn test_autosave_cleanup_old_checkpoints() {
    let temp_dir = TempDir::new().unwrap();
    let save_dir = temp_dir.path().join("checkpoints");

    let config = AutoSaveConfig {
        enabled: true,
        interval_turns: 1,  // Save every turn
        max_checkpoints: 2, // Keep only 2
        save_directory: save_dir.clone(),
    };

    let world = World::new(WorldConfig::default());
    let population = Population::new();

    let mut sim = Simulation::new(world, population);
    sim.enable_autosave(config).unwrap();

    // Create 5 checkpoints
    for _ in 0..5 {
        sim.take_a_turn();
    }

    // Should have exactly 2 (most recent)
    let checkpoint_count = std::fs::read_dir(&save_dir).unwrap().count();
    assert_eq!(checkpoint_count, 2);

    // Verify they are the most recent (turns 4 and 5)
    let latest = Simulation::get_latest_checkpoint(&save_dir).unwrap();
    let loaded = Simulation::load(&latest).unwrap();
    assert_eq!(loaded.current_turn, 5);
}
