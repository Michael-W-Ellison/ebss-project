// examples/exploration_demo.rs
//! Demo of exploration and map discovery system.

use ebss::agents::{Population, AgentConfig};
use ebss::world::{World, WorldConfig};
use ebss::core::DriveType;

fn main() {
    println!("=== Exploration & Map Discovery Demo ===\n");

    // Create a world and population
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();

    println!("World size: {}x{}", world.grid.width, world.grid.height);
    println!("Total tiles: {}\n", world.total_tiles());

    // Spawn some agents
    for i in 0..3 {
        population.spawn_agent(AgentConfig::default());

        // Position agents at different starting points
        if let Some(agent) = population.agents.last_mut() {
            agent.state.position = (
                10 + i * 5,
                10 + i * 5,
                0
            );

            println!("Agent {} spawned at position ({}, {})",
                i, agent.state.position.0, agent.state.position.1);
        }
    }

    // Print initial exploration state
    println!("\n=== Initial Exploration State ===");
    for (idx, agent) in population.agents.iter().enumerate() {
        let curiosity = agent.drives.get(DriveType::Curiosity)
            .map(|d| d.value)
            .unwrap_or(0.0);

        println!("Agent {}: Curiosity: {:.2}, Explored tiles: {}",
            idx,
            curiosity,
            agent.exploration_knowledge.total_tiles_explored);
    }

    // Run simulation for several ticks
    println!("\n=== Running exploration simulation for 100 ticks ===\n");

    for tick in 0..100 {
        // Update population tick counter
        population.current_tick = tick;

        // Process exploration with world
        population.process_exploration_with_world(&mut world);

        // Move agents slightly (random walk for exploration)
        for agent in &mut population.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Random walk to explore
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let dx = rng.gen_range(-1..=1);
            let dy = rng.gen_range(-1..=1);

            let new_x = (agent.state.position.0 + dx).max(0).min(world.grid.width as i32 - 1);
            let new_y = (agent.state.position.1 + dy).max(0).min(world.grid.height as i32 - 1);

            agent.state.position = (new_x, new_y, 0);
        }

        // Update agent drives
        for agent in &mut population.agents {
            agent.drives.tick();
        }

        // Report every 25 ticks
        if tick % 25 == 0 && tick > 0 {
            println!("--- Tick {} ---", tick);
            for (idx, agent) in population.agents.iter().enumerate() {
                let curiosity = agent.drives.get(DriveType::Curiosity)
                    .map(|d| d.value)
                    .unwrap_or(0.0);

                let explored = agent.exploration_knowledge.total_tiles_explored;
                let resources_found = agent.exploration_knowledge.known_resources.len();
                let terrains = agent.exploration_knowledge.encountered_terrains.len();
                let exploration_pct = agent.exploration_knowledge.exploration_percentage(world.total_tiles());

                println!("  Agent {}: Curiosity: {:.3}, Explored: {} tiles ({:.1}%), Resources: {}, Terrains: {}",
                    idx, curiosity, explored, exploration_pct, resources_found, terrains);
            }
            println!();
        }
    }

    // Final summary
    println!("=== Final Exploration Summary ===");
    for (idx, agent) in population.agents.iter().enumerate() {
        let curiosity = agent.drives.get(DriveType::Curiosity)
            .map(|d| d.value)
            .unwrap_or(0.0);

        let knowledge = &agent.exploration_knowledge;
        let exploration_pct = knowledge.exploration_percentage(world.total_tiles());

        println!("\nAgent {}:", idx);
        println!("  Final position: ({}, {})", agent.state.position.0, agent.state.position.1);
        println!("  Curiosity drive: {:.3}", curiosity);
        println!("  Tiles explored: {} / {} ({:.1}%)",
            knowledge.total_tiles_explored,
            world.total_tiles(),
            exploration_pct);
        println!("  Resources discovered: {}", knowledge.known_resources.len());
        println!("  Buildings discovered: {}", knowledge.known_buildings.len());
        println!("  Terrain types encountered: {}", knowledge.encountered_terrains.len());
        println!("  Total discoveries: {}", knowledge.discoveries.len());

        // Show recent discoveries
        if !knowledge.discoveries.is_empty() {
            println!("  Recent discoveries:");
            for discovery in knowledge.recent_discoveries(5) {
                match &discovery.discovery_type {
                    ebss::agents::DiscoveryType::Terrain(terrain_type) => {
                        println!("    - Terrain: {:?} at ({}, {}) on tick {}",
                            terrain_type, discovery.position.x, discovery.position.y, discovery.tick);
                    }
                    ebss::agents::DiscoveryType::Resource { resource_type, position } => {
                        println!("    - Resource: {:?} at ({}, {}) on tick {}",
                            resource_type, position.x, position.y, discovery.tick);
                    }
                    ebss::agents::DiscoveryType::Building { building_type, position } => {
                        println!("    - Building: {:?} at ({}, {}) on tick {}",
                            building_type, position.x, position.y, discovery.tick);
                    }
                    ebss::agents::DiscoveryType::AreaExplored { tiles_count } => {
                        println!("    - Explored {} tiles at ({}, {}) on tick {}",
                            tiles_count, discovery.position.x, discovery.position.y, discovery.tick);
                    }
                    ebss::agents::DiscoveryType::Storage { storage_type, position, capacity } => {
                        println!("    - Storage: {} at ({}, {}) capacity {:.0}% on tick {}",
                            storage_type, position.x, position.y, capacity * 100.0, discovery.tick);
                    }
                }
            }
        }

        // Show terrain types encountered
        if !knowledge.encountered_terrains.is_empty() {
            print!("  Terrain types: ");
            for terrain in &knowledge.encountered_terrains {
                print!("{:?} ", terrain);
            }
            println!();
        }
    }

    // Calculate total explored tiles globally
    let mut globally_explored = 0;
    for y in 0..world.grid.height {
        for x in 0..world.grid.width {
            if world.grid.tiles[y][x].explored {
                globally_explored += 1;
            }
        }
    }

    println!("\n=== World Exploration Statistics ===");
    println!("Total tiles globally explored: {} / {} ({:.1}%)",
        globally_explored,
        world.total_tiles(),
        (globally_explored as f32 / world.total_tiles() as f32) * 100.0);

    println!("\nExploration demo complete!");
}
