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
use ebss::world::{World, WorldConfig, ResourceConfig, AsciiRenderer, Position, Action, ResourceType};
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

/// Process agent actions (simplified autonomous behavior with survival-first priority)
fn process_agent_actions(world: &mut World, population: &mut Population, tick: u32) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Get agent IDs to avoid borrow conflicts
    let agent_ids: Vec<_> = population.agents.iter().map(|a| a.id).collect();

    for agent_id in agent_ids {
        // Find agent and try to eat if hungry
        if let Some(agent) = population.agents.iter_mut().find(|a| a.id == agent_id) {
            // Always try to eat if we have food and are hungry
            agent.try_eat(tick);

            let mut agent_pos = Position::new(agent.state.position.0, agent.state.position.1);

            // SURVIVAL-FIRST AI: Check if agent is in survival-critical state
            let is_critical = agent.state.is_survival_critical();
            let needs_food = agent.needs_food();

            // Simple AI: Check most urgent drive
            let most_urgent = agent.drives.most_urgent();

            let action = if is_critical || needs_food {
                // CRITICAL: Survival needs override everything
                // Try to find and gather food IMMEDIATELY
                if let Some(food_node) = world.resources.iter().find(|r| r.resource_type == ResourceType::Food) {
                    let food_pos = food_node.position;

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
                    // No food available - wander to find some
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
                        if let Some(food_node) = world.resources.iter().find(|r| r.resource_type == ResourceType::Food) {
                            let food_pos = food_node.position;
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
                        if let Some(wood_node) = world.resources.iter().find(|r| r.resource_type == ResourceType::Wood) {
                            let wood_pos = wood_node.position;
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
                let result = world.execute_action(agent_id, &mut agent_pos, &action);

                // Update agent position and inventory
                if let Some(agent) = population.agents.iter_mut().find(|a| a.id == agent_id) {
                    agent.state.position = (agent_pos.x, agent_pos.y, 0);

                    // If harvest was successful, add items to agent inventory
                    if let Some((item_type, quantity)) = result.take_items() {
                        if quantity > 0 {
                            agent.inventory.add_item(item_type, quantity);
                        }
                    }
                }
            }
        }
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

    // Show drives
    if let Some(agent) = population.agents.first() {
        println!("║ Sample Agent Drives:                                                      ║");
        println!("║   Hunger: {:.2}  │  Shelter: {:.2}  │  Safety: {:.2}  │  Construction: {:.2}  ║",
            agent.drives.get(DriveType::Hunger).map(|d| d.value).unwrap_or(0.0),
            agent.drives.get(DriveType::Shelter).map(|d| d.value).unwrap_or(0.0),
            agent.drives.get(DriveType::Safety).map(|d| d.value).unwrap_or(0.0),
            agent.drives.get(DriveType::Construction).map(|d| d.value).unwrap_or(0.0),
        );
        println!("║   Emotions: Happiness: {:.2}  │  Well-being: {:.2}                          ║",
            agent.emotions.get(EmotionType::Happiness).map(|e| e.value).unwrap_or(0.0),
            agent.emotions.well_being(),
        );
    }

    println!("╚═══════════════════════════════════════════════════════════════════════════╝");
}
