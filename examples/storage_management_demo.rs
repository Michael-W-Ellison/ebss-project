// examples/storage_management_demo.rs
//! Demo of storage management system with agent deposit/retrieval.

use ebss::agents::{Population, AgentConfig};
use ebss::world::{World, WorldConfig, ItemType};
use ebss::core::DriveType;
use ebss::agents::storage_integration::{
    add_to_agent_inventory, count_food_in_inventory,
    count_resources_in_inventory,
};

fn main() {
    println!("=== Storage Management Demo ===\n");

    // Create a world and population
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();

    println!("World created with storehouse");
    println!("Initial storehouse food: {}\n", world.storehouse_inventory.count_item(&ItemType::Food));

    // Spawn some agents
    for i in 0..5 {
        population.spawn_agent(AgentConfig::default());

        // Give agents some starting items
        if let Some(agent) = population.agents.last_mut() {
            agent.state.position = (25, 25, 0); // Near storehouse

            // Give varying amounts of food
            add_to_agent_inventory(&mut agent.inventory, ItemType::Food, 10 + i * 5);

            // Give some resources
            add_to_agent_inventory(&mut agent.inventory, ItemType::Wood, 15);
            add_to_agent_inventory(&mut agent.inventory, ItemType::Stone, 10);

            println!("Agent {} spawned with:",  i);
            println!("  Food: {}", count_food_in_inventory(&agent.inventory));
            println!("  Resources: {}", count_resources_in_inventory(&agent.inventory));
        }
    }

    // Add initial stock to storehouse
    world.storehouse_inventory.add_item(ItemType::Food, 50);
    world.storehouse_inventory.add_item(ItemType::Wood, 100);
    world.storehouse_inventory.add_item(ItemType::Stone, 75);

    println!("\nInitial storehouse inventory:");
    println!("  Food: {}", world.storehouse_inventory.count_item(&ItemType::Food));
    println!("  Wood: {}", world.storehouse_inventory.count_item(&ItemType::Wood));
    println!("  Stone: {}", world.storehouse_inventory.count_item(&ItemType::Stone));

    // Print agent storage decisions
    println!("\n=== Agent Storage Decisions ===");
    for (idx, agent) in population.agents.iter().enumerate() {
        let food_count = count_food_in_inventory(&agent.inventory);
        let resource_count = count_resources_in_inventory(&agent.inventory);

        let preparedness = agent.drives.get(DriveType::Preparedness)
            .map(|d| d.value)
            .unwrap_or(0.0);

        let decision = ebss::agents::decide_storage_action(
            food_count,
            resource_count,
            0, // tools
            world.storehouse_inventory.count_item(&ItemType::Food),
            world.storehouse_inventory.count_item(&ItemType::Wood),
            preparedness,
            &agent.storage_preferences,
        );

        println!("\nAgent {}:", idx);
        println!("  Personal food: {}, resources: {}", food_count, resource_count);
        println!("  Preparedness drive: {:.2}", preparedness);

        match decision {
            ebss::agents::StorageDecision::Deposit { item_type, quantity, reason } => {
                println!("  Decision: DEPOSIT {} {:?}", quantity, item_type);
                println!("  Reason: {}", reason);
            }
            ebss::agents::StorageDecision::Retrieve { item_type, quantity, reason } => {
                println!("  Decision: RETRIEVE {} {:?}", quantity, item_type);
                println!("  Reason: {}", reason);
            }
            ebss::agents::StorageDecision::NoAction { reason } => {
                println!("  Decision: NO ACTION");
                println!("  Reason: {}", reason);
            }
        }

        let priority = ebss::agents::calculate_storage_priority(
            food_count,
            agent.inventory.weight_percentage(),
            preparedness,
            &agent.storage_preferences,
        );
        println!("  Storage priority: {:.2}", priority);
    }

    // Check if storage is critical
    println!("\n=== Community Storage Status ===");
    let is_critical = ebss::agents::is_storage_critical(
        world.storehouse_inventory.count_item(&ItemType::Food),
        population.size(),
    );

    println!("Storage critical: {}", if is_critical { "YES" } else { "NO" });

    let should_gather = ebss::agents::should_prioritize_gathering(
        world.storehouse_inventory.count_item(&ItemType::Food),
        world.storehouse_inventory.count_item(&ItemType::Wood),
        population.size(),
    );

    println!("Should prioritize gathering: {}", if should_gather { "YES" } else { "NO" });

    // Simulate agents depositing excess
    println!("\n=== Simulating Deposits ===");
    for (idx, agent) in population.agents.iter_mut().enumerate() {
        let food_count = count_food_in_inventory(&agent.inventory);

        if food_count > agent.storage_preferences.max_personal_food {
            let excess = food_count - agent.storage_preferences.max_personal_food;

            // This would normally be handled by world.execute_action
            // For demo, we'll just show what would happen
            println!("Agent {} would deposit {} food", idx, excess);
        }
    }

    // Show final state
    println!("\n=== Final Summary ===");
    println!("Storehouse food: {}", world.storehouse_inventory.count_item(&ItemType::Food));
    println!("Storehouse wood: {}", world.storehouse_inventory.count_item(&ItemType::Wood));
    println!("Storehouse stone: {}", world.storehouse_inventory.count_item(&ItemType::Stone));

    println!("\nTotal agent food: {}",
        population.agents.iter()
            .map(|a| count_food_in_inventory(&a.inventory))
            .sum::<u32>());

    println!("\nStorage management demo complete!");
}
