// examples/basic_survival.rs
//! Basic survival simulation example.
//!
//! This example creates a simple world with agents that have basic drives
//! and demonstrates the core simulation loop.

use ebss::prelude::*;

fn main() {
    // Initialize logging
    env_logger::init();
    
    println!("=== EBSS Basic Survival Example ===\n");
    
    // Create a world
    println!("Creating world...");
    let world = World::new(GridConfig {
        size: (50, 50, 5),
        chunk_size: 16,
    });
    
    // Create population
    println!("Spawning agents...");
    let mut population = Population::new();
    for i in 0..5 {
        population.spawn_agent(AgentConfig::default());
        println!("  Agent {} spawned", i + 1);
    }
    
    println!("\nInitial population: {} agents\n", population.size());
    
    // Display initial drive states
    println!("Initial drive states:");
    for (i, agent) in population.agents.iter().enumerate() {
        println!("\nAgent {}:", i + 1);
        for drive in &agent.drives.drives {
            println!("  {:?}: {:.2}", drive.drive_type, drive.value);
        }
    }
    
    // Run simulation
    println!("\n--- Running simulation for 100 ticks ---\n");
    let mut sim = Simulation::new(world, population);
    sim.run_for_ticks(100);
    
    println!("\nSimulation complete!");
}
