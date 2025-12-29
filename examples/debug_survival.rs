// examples/debug_survival.rs
//! Debug simulation to test survival mechanics and drive satisfaction.

use ebss::agents::{Population, AgentConfig, InventoryItem};
use ebss::world::{World, WorldConfig, ResourceConfig, Position, Action, ResourceType, ItemType};
use ebss::core::DriveType;

/// Helper to count items in agent inventory
fn count_inventory_item(agent: &ebss::agents::Agent, item_id: &str) -> u32 {
    agent.inventory.get_item(item_id)
        .map(|item| item.quantity)
        .unwrap_or(0)
}

fn main() {
    println!("=== DEBUG: Survival Mechanics Test ===\n");

    // Create simple world with resources near spawn
    let world_config = WorldConfig {
        size: (20, 20),
        initial_resources: ResourceConfig {
            wood_nodes: 5,
            stone_nodes: 3,
            iron_nodes: 2,
            food_nodes: 10, // Lots of food for testing
        },
    };
    let mut world = World::new(world_config);

    // Create single agent for testing
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    if let Some(agent) = population.agents.first_mut() {
        agent.state.position = (10, 10, 0); // Center of world
        println!("Agent spawned at position ({}, {})\n", agent.state.position.0, agent.state.position.1);
    }

    println!("Food nodes in world:");
    for (i, resource) in world.resources.iter().filter(|r| r.resource_type == ResourceType::Food).enumerate() {
        println!("  Food #{}: position ({}, {}), amount: {}",
            i + 1, resource.position.x, resource.position.y, resource.amount);
    }
    println!();

    // Run simulation for 500 ticks with detailed logging
    for tick in 0..500 {
        if tick % 50 == 0 {
            println!("\n=== TICK {} ===", tick);

            if let Some(agent) = population.agents.first() {
                println!("Agent State:");
                println!("  Position: ({}, {})", agent.state.position.0, agent.state.position.1);
                println!("  Energy: {:.1}%", agent.state.energy);
                println!("  Health: {:.1}%", agent.state.health);
                println!("  Ticks without food: {}", agent.state.ticks_without_food);
                println!("  Is starving: {}", agent.state.is_starving());
                println!("  Is survival critical: {}", agent.state.is_survival_critical());

                println!("\nAgent Inventory:");
                println!("  Food: {}", count_inventory_item(agent, "food"));
                println!("  Wood: {}", count_inventory_item(agent, "wood"));

                println!("\nTop 5 Drives:");
                let active_drives = agent.drives.active_drives();
                for (i, drive) in active_drives.iter().take(5).enumerate() {
                    println!("  {}. {:?}: value={:.2}, weight={:.2}, urgency={:.2}, active={}",
                        i + 1,
                        drive.drive_type,
                        drive.value,
                        drive.weight,
                        drive.urgency(),
                        drive.is_active());
                }

                if let Some(most_urgent) = agent.drives.most_urgent() {
                    println!("\nMost Urgent Drive: {:?} (urgency: {:.2})",
                        most_urgent.drive_type, most_urgent.urgency());
                }
            }
        }

        // Process single agent action
        if let Some(agent) = population.agents.first_mut() {
            let agent_id = agent.id;

            // Try to eat if we have food and are hungry
            let hunger_value = agent.drives.get(DriveType::Hunger).map(|d| d.value).unwrap_or(0.0);
            let needs_food = hunger_value > 0.6 || agent.state.energy < 30.0;
            let has_food = count_inventory_item(agent, "food") > 0;
            let mut ate = false;

            if has_food && needs_food {
                if agent.inventory.remove_item("food", 1).is_some() {
                    agent.state.eat(tick, 25.0);
                    if let Some(hunger_drive) = agent.drives.get_mut(DriveType::Hunger) {
                        hunger_drive.partial_satisfy(0.3);
                    }
                    ate = true;
                }
            }
            if ate && tick % 50 < 10 {
                println!("  → Agent ate food! Energy restored.");
            }

            let mut agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
            let is_critical = agent.state.is_survival_critical();

            // Simple AI: prioritize food if needed
            let action = if needs_food || is_critical {
                // Find closest food
                if let Some(food_node) = world.resources.iter()
                    .filter(|r| r.resource_type == ResourceType::Food && r.amount > 0)
                    .min_by_key(|r| agent_pos.distance_to(&r.position))
                {
                    let food_pos = food_node.position;
                    let distance = agent_pos.distance_to(&food_pos);

                    if distance > 1 {
                        if tick % 50 < 10 {
                            println!("  → Moving towards food at ({}, {}) - distance: {}",
                                food_pos.x, food_pos.y, distance);
                        }
                        Some(Action::MoveTo { destination: food_pos })
                    } else {
                        if tick % 50 < 10 {
                            println!("  → Harvesting food at ({}, {})", food_pos.x, food_pos.y);
                        }
                        Some(Action::HarvestResource {
                            resource_position: food_pos,
                            resource_type: ResourceType::Food,
                            amount: 5,
                        })
                    }
                } else {
                    if tick % 50 < 10 {
                        println!("  → No food found!");
                    }
                    None
                }
            } else {
                // Once fed, check other drives
                if let Some(most_urgent) = agent.drives.most_urgent() {
                    match most_urgent.drive_type {
                        DriveType::Shelter | DriveType::Construction => {
                            // Gather wood
                            if let Some(wood_node) = world.resources.iter()
                                .filter(|r| r.resource_type == ResourceType::Wood && r.amount > 0)
                                .min_by_key(|r| agent_pos.distance_to(&r.position))
                            {
                                let wood_pos = wood_node.position;
                                if agent_pos.distance_to(&wood_pos) > 1 {
                                    if tick % 50 < 10 {
                                        println!("  → Moving towards wood (for {:?})", most_urgent.drive_type);
                                    }
                                    Some(Action::MoveTo { destination: wood_pos })
                                } else {
                                    if tick % 50 < 10 {
                                        println!("  → Harvesting wood (for {:?})", most_urgent.drive_type);
                                    }
                                    Some(Action::HarvestResource {
                                        resource_position: wood_pos,
                                        resource_type: ResourceType::Wood,
                                        amount: 3,
                                    })
                                }
                            } else {
                                None
                            }
                        }
                        _ => {
                            if tick % 50 < 10 {
                                println!("  → Resting (drive: {:?})", most_urgent.drive_type);
                            }
                            Some(Action::Rest { duration: 1 })
                        }
                    }
                } else {
                    None
                }
            };

            // Execute action
            if let Some(action) = action {
                // No other agents in single-agent test, so empty occupied positions
                let occupied_positions: Vec<Position> = vec![];
                let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied_positions);

                // Update position and inventory
                if let Some(agent) = population.agents.first_mut() {
                    agent.state.position = (agent_pos.x, agent_pos.y, 0);

                    // Add harvested items to inventory
                    if let Some((item_type, quantity)) = result.take_items() {
                        if quantity > 0 {
                            let item_id = match item_type {
                                ItemType::Food => "food",
                                ItemType::Wood => "wood",
                                ItemType::Stone => "stone",
                                ItemType::Iron => "iron",
                                _ => "misc",
                            };
                            let item = InventoryItem::new(item_id.to_string(), quantity);
                            agent.inventory.add_item(item);
                            if tick % 50 < 10 {
                                println!("  ✓ Added {} {:?} to inventory", quantity, item_type);
                            }
                        }
                    }

                    if !result.is_success() && tick % 50 < 10 {
                        println!("  ✗ Action failed");
                    }
                }
            }
        }

        // Update population (ticks drives and ages agents)
        population.tick();

        // Update world
        world.tick();

        // Check if agent died
        if population.agents.is_empty() {
            println!("\n⚠️  Agent died at tick {}!", tick);
            break;
        }
    }

    println!("\n=== Simulation Complete ===");
}
