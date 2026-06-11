// src/analytics/tests/save_load_tests.rs
//! TDD tests for simulation save/load functionality
//!
//! These tests define the expected behavior for saving and resuming simulations.

use crate::analytics::Simulation;
use crate::agents::{Population, AgentConfig};
use crate::world::{World, WorldConfig};
use tempfile::TempDir;

#[test]
fn test_simulation_can_be_saved_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create a simple simulation
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);

    // Run for a few ticks to generate state
    sim.tick();
    sim.tick();
    sim.tick();

    // Save the simulation
    sim.save(&save_path).expect("Failed to save simulation");

    // Verify file was created
    assert!(save_path.exists());

    // Verify file has content
    let metadata = std::fs::metadata(&save_path).unwrap();
    assert!(metadata.len() > 0, "Save file is empty");
}

#[test]
fn test_simulation_can_be_loaded_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create and save a simulation
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);
    sim.tick();
    sim.tick();

    let saved_tick = sim.current_tick;
    let saved_pop_count = sim.population.agents.len();

    sim.save(&save_path).unwrap();

    // Load the simulation
    let loaded_sim = Simulation::load(&save_path).expect("Failed to load simulation");

    // Verify state was restored
    assert_eq!(loaded_sim.current_tick, saved_tick);
    assert_eq!(loaded_sim.population.agents.len(), saved_pop_count);
}

#[test]
fn test_loaded_simulation_can_resume() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create, run, and save simulation
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..5 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut sim = Simulation::new(world, population);

    // Run for 10 ticks
    for _ in 0..10 {
        sim.tick();
    }

    assert_eq!(sim.current_tick, 10);

    sim.save(&save_path).unwrap();

    // Load and continue running
    let mut loaded_sim = Simulation::load(&save_path).unwrap();

    // Should start from where we left off
    assert_eq!(loaded_sim.current_tick, 10);

    // Run for 5 more ticks
    for _ in 0..5 {
        loaded_sim.tick();
    }

    // Should now be at tick 15
    assert_eq!(loaded_sim.current_tick, 15);
}

#[test]
fn test_agent_state_preserved_across_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create simulation with specific agent state
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);

    // Modify agent state
    if let Some(agent) = sim.population.agents.first_mut() {
        agent.state.energy = 75.5;
        agent.state.health = 85.0;
        agent.state.age = 1234;
    }

    let agent_id = sim.population.agents[0].id;

    sim.save(&save_path).unwrap();

    // Load and verify agent state
    let loaded_sim = Simulation::load(&save_path).unwrap();

    let loaded_agent = loaded_sim.population.agents.iter()
        .find(|a| a.id == agent_id)
        .expect("Agent not found after load");

    assert_eq!(loaded_agent.state.energy, 75.5);
    assert_eq!(loaded_agent.state.health, 85.0);
    assert_eq!(loaded_agent.state.age, 1234);
}

#[test]
fn test_world_state_preserved_across_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create simulation with world state
    let world = World::new(WorldConfig::default());
    let population = Population::new();

    let mut sim = Simulation::new(world, population);

    // Run world for a bit to generate state
    sim.world.climate.tick();
    sim.world.climate.tick();

    let saved_tick = sim.world.tick;

    sim.save(&save_path).unwrap();

    // Load and verify world state
    let loaded_sim = Simulation::load(&save_path).unwrap();

    assert_eq!(loaded_sim.world.tick, saved_tick);
}

#[test]
fn test_drive_values_preserved_across_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create simulation
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);

    // Accumulate some drive values
    for _ in 0..50 {
        sim.population.agents[0].drives.tick();
    }

    let hunger_value = sim.population.agents[0].drives.get(crate::core::DriveType::Hunger)
        .unwrap().value;

    assert!(hunger_value > 0.0, "Hunger should have accumulated");

    sim.save(&save_path).unwrap();

    // Load and verify drive values
    let loaded_sim = Simulation::load(&save_path).unwrap();

    let loaded_hunger = loaded_sim.population.agents[0].drives.get(crate::core::DriveType::Hunger)
        .unwrap().value;

    assert!((loaded_hunger - hunger_value).abs() < 0.0001,
            "Hunger value should be preserved: expected {}, got {}", hunger_value, loaded_hunger);
}

#[test]
fn test_save_fails_gracefully_with_invalid_path() {
    let world = World::new(WorldConfig::default());
    let population = Population::new();
    let sim = Simulation::new(world, population);

    // Try to save to invalid path
    let result = sim.save("/invalid/path/that/does/not/exist/simulation.dat");

    assert!(result.is_err(), "Save should fail for invalid path");
}

#[test]
fn test_load_fails_gracefully_with_nonexistent_file() {
    let result = Simulation::load("/nonexistent/file.json");

    assert!(result.is_err(), "Load should fail for nonexistent file");
}

#[test]
fn test_load_fails_gracefully_with_corrupted_file() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("corrupted.json");

    // Create corrupted file
    std::fs::write(&save_path, "{ this is not valid json }").unwrap();

    let result = Simulation::load(&save_path);

    assert!(result.is_err(), "Load should fail for corrupted file");
}

#[test]
fn test_multiple_save_load_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create simulation
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);

    // Cycle 1: Run 5 ticks, save, load
    for _ in 0..5 {
        sim.tick();
    }
    sim.save(&save_path).unwrap();
    let mut sim = Simulation::load(&save_path).unwrap();
    assert_eq!(sim.current_tick, 5);

    // Cycle 2: Run 5 more ticks, save, load
    for _ in 0..5 {
        sim.tick();
    }
    sim.save(&save_path).unwrap();
    let mut sim = Simulation::load(&save_path).unwrap();
    assert_eq!(sim.current_tick, 10);

    // Cycle 3: Run 10 more ticks
    for _ in 0..10 {
        sim.tick();
    }

    assert_eq!(sim.current_tick, 20);
}

#[test]
fn test_inventory_preserved_across_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("simulation.dat");

    // Create simulation
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut sim = Simulation::new(world, population);

    // Add items to inventory
    sim.population.agents[0].inventory.add_item(
        crate::agents::InventoryItem::new("food".to_string(), 10)
    );
    sim.population.agents[0].inventory.add_item(
        crate::agents::InventoryItem::new("wood".to_string(), 5)
    );

    sim.save(&save_path).unwrap();

    // Load and verify inventory
    let loaded_sim = Simulation::load(&save_path).unwrap();

    let food = loaded_sim.population.agents[0].inventory.get_item("food");
    assert!(food.is_some());
    assert_eq!(food.unwrap().quantity, 10);

    let wood = loaded_sim.population.agents[0].inventory.get_item("wood");
    assert!(wood.is_some());
    assert_eq!(wood.unwrap().quantity, 5);
}
