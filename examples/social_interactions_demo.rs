// examples/social_interactions_demo.rs
//! Demo of social interactions between agents.

use ebss::agents::{Population, AgentConfig};
use ebss::core::DriveType;

fn main() {
    println!("=== Social Interactions Demo ===\n");

    // Create a population
    let mut population = Population::new();

    // Spawn some agents close to each other
    for i in 0..5 {
        let mut config = AgentConfig::default();
        population.spawn_agent(config);

        // Position agents near each other (within social interaction range)
        if let Some(agent) = population.agents.last_mut() {
            agent.state.position = (i * 2, i * 2, 0); // Spread them out a bit but within range
        }
    }

    println!("Spawned {} agents\n", population.size());

    // Print initial state
    println!("Initial agent states:");
    for (idx, agent) in population.agents.iter().enumerate() {
        let social_drive = agent.drives.get(DriveType::Social)
            .map(|d| d.value)
            .unwrap_or(0.0);
        let relationships = agent.social_network.all_relationships().len();

        println!("  Agent {}: Position: {:?}, Social Drive: {:.2}, Relationships: {}",
            idx, agent.state.position, social_drive, relationships);
    }

    // Run simulation for several ticks
    println!("\n=== Running simulation for 200 ticks ===\n");

    for tick in 0..200 {
        population.tick();

        // Report every 50 ticks
        if tick % 50 == 0 && tick > 0 {
            println!("--- Tick {} ---", tick);
            for (idx, agent) in population.agents.iter().enumerate() {
                let social_drive = agent.drives.get(DriveType::Social)
                    .map(|d| d.value)
                    .unwrap_or(0.0);
                let relationships = agent.social_network.all_relationships();

                println!("  Agent {}: Social Drive: {:.3}", idx, social_drive);

                // Show relationships
                for rel in relationships {
                    println!("    -> Relationship with agent (level: {}, trust: {})",
                        rel.relationship_level.name(),
                        rel.trust_level.name());
                }
            }
            println!();
        }
    }

    // Final summary
    println!("=== Final Summary ===");
    for (idx, agent) in population.agents.iter().enumerate() {
        let social_drive = agent.drives.get(DriveType::Social)
            .map(|d| d.value)
            .unwrap_or(0.0);
        let relationships = agent.social_network.all_relationships();
        let total_interactions: u32 = relationships.iter()
            .map(|r| r.total_interactions)
            .sum();

        println!("Agent {}:", idx);
        println!("  Social Drive: {:.3}", social_drive);
        println!("  Total Relationships: {}", relationships.len());
        println!("  Total Interactions: {}", total_interactions);

        for rel in relationships {
            println!("    Relationship: {} ({}), Trust: {} ({} interactions)",
                rel.relationship_level.name(),
                rel.relationship_level.value(),
                rel.trust_level.name(),
                rel.total_interactions);
        }
        println!();
    }

    println!("\nSocial interactions demo complete!");
}
