// examples/ascii_simulation.rs
//! Complete ASCII-based standalone simulation.
//!
//! This example demonstrates the full EBSS system with:
//! - Procedurally generated world with terrain and resources
//! - Population of agents with drives, emotions, and traits
//! - Real-time ASCII visualization
//! - Agents autonomously gather resources and build
//! - Drive progression system (Basic → Luxury tiers)
//!
//! Run with: cargo run --example ascii_simulation

use ebss::agents::{Population, PopulationConfig, AgentConfig};
use ebss::world::{World, WorldConfig, ResourceConfig, AsciiRenderer, Position, Action, ResourceType, ItemType};
use ebss::world::render::ViewPort;
use ebss::analytics::{SimulationMetrics, EmergenceDetector, PerformanceMonitor};
use ebss::core::{DriveType, EmotionType};
use std::thread;
use std::time::Duration;

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║       EBSS - Emergent Behavior Society Simulator              ║");
    println!("║       ASCII Standalone Simulation                             ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Initialize world
    println!("Generating world...");
    let world_config = WorldConfig {
        size: (50, 30),
        initial_resources: ResourceConfig {
            wood_nodes: 30,
            stone_nodes: 20,
            iron_nodes: 10,
            food_nodes: 40,
        },
    };
    let mut world = World::new(world_config);

    // Initialize population
    println!("Creating initial population...");
    let mut population = Population::new();
    population.config = PopulationConfig {
        abandonment_happiness_threshold: -0.3,
        abandonment_unhappy_duration: 1000,
        abandonment_probability: 0.01,
    };

    // Spawn initial agents near longhouse
    let longhouse_pos = Position::new(25, 15); // Center of world
    for i in 0..5 {
        let config = AgentConfig::default();
        population.spawn_agent(config);

        // Position agents near the longhouse
        let offset_x = (i % 3) as i32 - 1;
        let offset_y = (i / 3) as i32 - 1;

        if let Some(agent) = population.agents.last_mut() {
            agent.state.position = (longhouse_pos.x + offset_x, longhouse_pos.y + offset_y, 0);
        }
    }

    // Initialize systems
    let renderer = AsciiRenderer::new();
    let mut metrics = SimulationMetrics::new(50, 200);
    let mut emergence = EmergenceDetector::new();
    let mut performance = PerformanceMonitor::new(1000);

    println!("World generated with {} resources", world.resources.len());
    println!("Starting simulation with {} agents", population.agents.len());
    println!("\nPress Ctrl+C to exit\n");

    // Display legend
    print!("{}", renderer.render_legend());

    thread::sleep(Duration::from_secs(2));

    // Main simulation loop
    for tick in 0..10000 {
        performance.start_tick();

        // Process agents (they move and take actions autonomously)
        process_agent_actions(&mut world, &mut population, tick);

        // Update population (aging, reproduction, death, abandonment)
        population.tick();

        // Update world (building construction, resource respawn)
        world.tick();

        // Record metrics
        metrics.record_if_time(tick, &population);

        // Detect emergence
        if tick % 100 == 0 && tick > 0 {
            emergence.detect_patterns(&metrics, tick);
        }

        performance.end_tick(tick, population.agents.len());

        // Render frame every 10 ticks
        if tick % 10 == 0 {
            render_frame(&renderer, &world, &population, tick, &metrics, &emergence, &performance);

            // Slow down for visibility
            thread::sleep(Duration::from_millis(100));
        }

        // Stop if population dies out
        if population.agents.is_empty() {
            println!("\n⚠️  Population has died out!");
            break;
        }
    }

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                   Simulation Complete                         ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // Final summary
    let summary = metrics.summary();
    println!("\nFinal Statistics:");
    println!("  Total Ticks: {}", summary.total_ticks);
    println!("  Initial Population: {}", summary.initial_population);
    println!("  Final Population: {}", population.agents.len());
    println!("  Population Change: {:+}", summary.population_change);
    println!("  Peak Population: {}", summary.peak_population);
    println!("  Average Happiness: {:.2}", summary.average_happiness);

    let world_stats = world.stats();
    println!("\nFinal World State:");
    println!("  Resources Remaining: {}", world_stats.total_resources);
    println!("  Wood in Storehouse: {}", world_stats.wood_stored);
    println!("  Stone in Storehouse: {}", world_stats.stone_stored);
    println!("  Iron in Storehouse: {}", world_stats.iron_stored);
    println!("  Food in Storehouse: {}", world_stats.food_stored);
    println!("  Buildings: {}", world_stats.total_buildings);

    // Show emergent patterns
    if !emergence.detected_patterns.is_empty() {
        println!("\nEmergent Patterns Detected: {}", emergence.detected_patterns.len());
        for (i, pattern) in emergence.most_severe_patterns(3).iter().enumerate() {
            println!(
                "  {}. [Tick {}] {}",
                i + 1,
                pattern.detected_at_tick,
                pattern.description
            );
        }
    }
}

/// Discover nearby resources (vision range of 10 tiles)
fn discover_nearby_resources(world: &World, population: &mut Population, agent_id: uuid::Uuid, agent_pos: &Position) {
    const VISION_RANGE: u32 = 10;

    for resource in &world.resources {
        if agent_pos.distance_to(&resource.position) <= VISION_RANGE && resource.amount > 0 {
            // Agent can see this resource, add to shared knowledge
            population.shared_knowledge.discover_resource(
                resource.position,
                resource.resource_type,
                resource.amount,
                agent_id,
            );
        }
    }
}

/// Process agent actions (simplified autonomous behavior with survival-first priority)
fn process_agent_actions(world: &mut World, population: &mut Population, tick: u32) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Get agent IDs and positions for collision detection
    let agent_data: Vec<_> = population.agents.iter()
        .map(|a| (a.id, Position::new(a.state.position.0, a.state.position.1)))
        .collect();

    for (agent_id, agent_starting_pos) in &agent_data {
        // Discover nearby resources for this agent
        discover_nearby_resources(world, population, *agent_id, agent_starting_pos);

        // Find agent and try to eat if hungry
        if let Some(agent) = population.agents.iter_mut().find(|a| a.id == agent_id) {
            // Always try to eat if we have food and are hungry
            agent.try_eat(tick);

            let mut agent_pos = Position::new(agent.state.position.0, agent.state.position.1);

            // SURVIVAL-FIRST AI: Check if agent is in survival-critical state
            let is_critical = agent.state.is_survival_critical();
            let needs_food = agent.needs_food();

            // Debug logging
            if tick % 50 == 0 && tick < 150 {
                let hunger_val = agent.drives.get(DriveType::Hunger).map(|d| d.value).unwrap_or(0.0);
                eprintln!("[DEBUG Tick {}] Agent: critical={}, needs_food={}, hunger={:.2}, energy={:.1}, food_inv={}",
                    tick, is_critical, needs_food, hunger_val, agent.state.energy,
                    agent.inventory.count_item(&ItemType::Food));
            }

            // Simple AI: Check most urgent drive
            let most_urgent = agent.drives.most_urgent();

            // Use SHARED KNOWLEDGE to find resources (agents communicate!)
            let known_food = population.shared_knowledge
                .find_closest_known_to_agent(*agent_id, &agent_pos, ResourceType::Food)
                .map(|r| r.position);

            let known_wood = population.shared_knowledge
                .find_closest_known_to_agent(*agent_id, &agent_pos, ResourceType::Wood)
                .map(|r| r.position);

            let action = if is_critical || needs_food {
                // CRITICAL: Survival needs override everything
                // Try to find and gather food IMMEDIATELY using shared knowledge
                if let Some(food_pos) = known_food {
                    if agent_pos.distance_to(&food_pos) > 1 {
                        Some(Action::MoveTo { destination: food_pos })
                    } else {
                        Some(Action::HarvestResource {
                            resource_position: food_pos,
                            resource_type: ResourceType::Food,
                            amount: 5,
                        })
                    }
                } else {
                    // No known food - wander to discover more
                    if rng.gen_bool(0.5) {
                        let dx = rng.gen_range(-3..=3);
                        let dy = rng.gen_range(-3..=3);
                        let destination = Position::new(agent_pos.x + dx, agent_pos.y + dy);
                        if world.grid.is_valid_position(&destination) {
                            Some(Action::MoveTo { destination })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            } else {
                // Normal behavior based on drives
                match most_urgent.map(|d| &d.drive_type) {
                    Some(DriveType::Hunger) => {
                        if let Some(food_pos) = known_food {
                            if agent_pos.distance_to(&food_pos) > 1 {
                                Some(Action::MoveTo { destination: food_pos })
                            } else {
                                Some(Action::HarvestResource {
                                    resource_position: food_pos,
                                    resource_type: ResourceType::Food,
                                    amount: 5,
                                })
                            }
                        } else {
                            None
                        }
                    }

                    Some(DriveType::Construction) => {
                        if let Some(wood_pos) = known_wood {
                            if agent_pos.distance_to(&wood_pos) > 1 {
                                Some(Action::MoveTo { destination: wood_pos })
                            } else {
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
                        if rng.gen_bool(0.3) {
                            let dx = rng.gen_range(-2..=2);
                            let dy = rng.gen_range(-2..=2);
                            let destination = Position::new(agent_pos.x + dx, agent_pos.y + dy);
                            if world.grid.is_valid_position(&destination) {
                                Some(Action::MoveTo { destination })
                            } else {
                                None
                            }
                        } else {
                            Some(Action::Rest { duration: 1 })
                        }
                    }
                }
            };

            // Execute action
            if let Some(action) = action {
                // Get occupied positions (all agents except current one)
                let occupied_positions: Vec<Position> = agent_data.iter()
                    .filter(|(id, _)| id != agent_id)
                    .map(|(_, pos)| *pos)
                    .collect();

                let result = world.execute_action(*agent_id, &mut agent_pos, &action, &occupied_positions);

                // Debug: Log failed food harvesting
                if matches!(action, Action::HarvestResource { resource_type: ResourceType::Food, .. }) {
                    if !result.is_success() && tick % 100 < 5 {
                        if let Action::HarvestResource { resource_position, .. } = action {
                            eprintln!("[DEBUG Tick {}] Food harvest FAILED at ({}, {}) - Agent at ({}, {})",
                                tick, resource_position.x, resource_position.y,
                                agent_pos.x, agent_pos.y);
                        }
                    } else if result.is_success() && tick % 100 < 5 {
                        eprintln!("[DEBUG Tick {}] Food harvest SUCCESS", tick);
                    }
                }

                // Update agent position and inventory
                if let Some(agent) = population.agents.iter_mut().find(|a| a.id == agent_id) {
                    agent.state.position = (agent_pos.x, agent_pos.y, 0);

                    // If harvest was successful, add items to agent inventory
                    if let Some((item_type, quantity)) = result.take_items() {
                        if quantity > 0 {
                            agent.inventory.add_item(item_type, quantity);
                            if matches!(item_type, ItemType::Food) && tick % 100 < 5 {
                                eprintln!("[DEBUG Tick {}] Added {} food to inventory", tick, quantity);
                            }
                        }
                    }
                }
            }
        }
    }

    // Clean up depleted resources from shared knowledge
    cleanup_depleted_resources(world, population);
}

/// Remove depleted resources from shared knowledge
fn cleanup_depleted_resources(world: &World, population: &mut Population) {
    // Get positions of all depleted resources
    let depleted_positions: Vec<Position> = population.shared_knowledge
        .all_resources()
        .iter()
        .filter(|known_resource| {
            // Check if this resource still exists in the world with amount > 0
            !world.resources.iter().any(|r| {
                r.position == known_resource.position && r.amount > 0
            })
        })
        .map(|r| r.position)
        .collect();

    // Remove depleted resources
    for pos in depleted_positions {
        population.shared_knowledge.remove_resource(&pos);
    }
}

/// Render a complete frame
fn render_frame(
    renderer: &AsciiRenderer,
    world: &World,
    population: &Population,
    tick: u32,
    metrics: &SimulationMetrics,
    emergence: &EmergenceDetector,
    performance: &PerformanceMonitor,
) {
    // Create viewport centered on world
    let viewport = Some(ViewPort::new(0, 0, 50, 30));

    // Render frame
    let frame = renderer.render_frame(world, population, viewport);
    print!("{}", frame);

    // Show additional info
    println!("╔═══════════════════════════ SIMULATION INFO ════════════════════════════╗");
    println!("║ Tick: {:6}  │  Population: {:3}  │  TPS: {:6.1}  │  Emergent Patterns: {:2} ║",
        tick,
        population.agents.len(),
        performance.snapshots.last().map(|s| s.ticks_per_second).unwrap_or(0.0),
        emergence.detected_patterns.len()
    );

    // Show drives and inventory
    if let Some(agent) = population.agents.first() {
        println!("║ Sample Agent Drives:                                                      ║");
        println!("║   Hunger: {:.2}  │  Shelter: {:.2}  │  Safety: {:.2}  │  Construction: {:.2}  ║",
            agent.drives.get(DriveType::Hunger).map(|d| d.value).unwrap_or(0.0),
            agent.drives.get(DriveType::Shelter).map(|d| d.value).unwrap_or(0.0),
            agent.drives.get(DriveType::Safety).map(|d| d.value).unwrap_or(0.0),
            agent.drives.get(DriveType::Construction).map(|d| d.value).unwrap_or(0.0),
        );
        println!("║   Emotions: Happiness: {:.2}  │  Well-being: {:.2}  │  Energy: {:.1}%          ║",
            agent.emotions.get(EmotionType::Happiness).map(|e| e.value).unwrap_or(0.0),
            agent.emotions.well_being(),
            agent.state.energy,
        );
        println!("║   Inventory: Food: {}  │  Wood: {}  │  Stone: {}  │  Iron: {}            ║",
            agent.inventory.count_item(&ItemType::Food),
            agent.inventory.count_item(&ItemType::Wood),
            agent.inventory.count_item(&ItemType::Stone),
            agent.inventory.count_item(&ItemType::Iron),
        );
    }

    println!("╚═══════════════════════════════════════════════════════════════════════════╝");
}
