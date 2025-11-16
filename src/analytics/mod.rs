// src/analytics/mod.rs
//! Analytics, data logging, and emergence detection.

use crate::world::World;
use crate::agents::Population;

pub mod simulation_controller;
pub mod inspector;

pub use simulation_controller::{SimulationController, SimulationState};
pub use inspector::{
    Inspector, Selection, AgentInspectorData, DriveInspectorData,
    TerrainInspectorData, MemorySummary, InventorySummary, SensorySummary, SkillsSummary,
    EmotionSummary, RelationshipSummary,
};
pub mod metrics;
pub mod emergence;
pub mod export;
pub mod performance;

pub use metrics::{SimulationMetrics, TickSnapshot, PopulationSnapshot, DriveSnapshot, EmotionSnapshot};
pub use emergence::{EmergenceDetector, EmergentPattern, PatternType};
pub use export::{DataExporter, ExportFormat};
pub use performance::{PerformanceMonitor, PerformanceSnapshot};

use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use crate::visualization::AsciiRenderer;
use log::{info, debug};

pub struct Simulation {
    pub world: World,
    pub population: Population,
    pub current_tick: u32,
    pub renderer: Option<AsciiRenderer>,
}

pub struct SimulationConfig;
pub struct Analytics;
pub struct BehaviorAnalysis;

impl Simulation {
    pub fn new(world: World, population: Population) -> Self {
        Self {
            world,
            population,
            current_tick: 0,
            renderer: None,
        }
    }

    /// Enable ASCII visualization
    pub fn with_visualization(mut self) -> Self {
        self.renderer = Some(AsciiRenderer::default());
        self
    }

    /// Run the simulation for a specified number of ticks
    pub fn run_for_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.tick();
        }
        info!("Simulation completed {} ticks", ticks);
    }

    /// Run the simulation with visualization
    pub fn run_visual(&mut self, ticks: u32, update_interval: u32) {
        for _ in 0..ticks {
            self.tick();

            // Render visualization at intervals
            if self.current_tick % update_interval == 0 {
                if let Some(renderer) = &self.renderer {
                    renderer.render(&self.population, self.current_tick);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        // Final render
        if let Some(renderer) = &self.renderer {
            renderer.render(&self.population, self.current_tick);
        }
    }

    /// Execute one simulation tick
    pub fn tick(&mut self) {
        // Process population lifecycle (aging, starvation, deaths, reproduction)
        // This also increments the tick counter and updates all agents
        self.population.tick();

        // Sync simulation tick with population tick
        self.current_tick = self.population.current_tick;

        debug!("=== Tick {} ===", self.current_tick);

        // Process agent behavior and actions
        // Note: agents have already been updated by population.tick() above
        // This loop handles behavior tree execution and action processing

        // Collect agent IDs to avoid borrowing issues
        let agent_ids: Vec<_> = self.population.agents.iter().map(|a| a.id).collect();

        for agent_id in agent_ids {
            // Find the agent
            let agent_index = self.population.agents.iter().position(|a| a.id == agent_id);
            if agent_index.is_none() {
                continue;
            }
            let agent_index = agent_index.unwrap();

            // Get agent data we need
            let (drive_type, drive_value, agent_position) = {
                let agent = &self.population.agents[agent_index];
                if let Some(urgent_drive) = agent.drives.most_urgent() {
                    (Some(urgent_drive.drive_type), urgent_drive.value, agent.state.position)
                } else {
                    (None, 0.0, agent.state.position)
                }
            };

            if drive_type.is_none() {
                continue;
            }
            let drive_type = drive_type.unwrap();

            debug!(
                "Agent {} - Most urgent drive: {:?} (value: {:.2})",
                agent_id, drive_type, drive_value
            );

            // Select and execute behavior tree
            let (tree_name, execution_result) = {
                let agent = &mut self.population.agents[agent_index];
                if let Some(tree) = agent.select_behavior_tree() {
                    let tree_name = tree.name.clone();
                    let execution_result = tree.execute();
                    (Some(tree_name), Some(execution_result))
                } else {
                    (None, None)
                }
            };

            if tree_name.is_some() {
                debug!(
                    "Agent {} - Executed tree: {} -> {:?}",
                    agent_id, tree_name.as_ref().unwrap(), execution_result.as_ref().unwrap()
                );

                // Generate action based on drive type and agent position
                let action = Self::generate_action_for_drive(drive_type, agent_position);

                // Execute action in environment and get feedback
                let action_result = self.execute_action(&action, agent_index);

                debug!(
                    "Agent {} - Action result: {} (satisfaction: {:.2})",
                    agent_id, action_result.message, action_result.drive_satisfaction
                );

                // Apply feedback to agent (drive satisfaction)
                let agent = &mut self.population.agents[agent_index];
                agent.apply_feedback(&action_result, drive_type);

                // Update behavior tree weights based on action success
                if let Some(tree) = agent.select_behavior_tree() {
                    if action_result.success {
                        tree.total_successes += 1;
                    }
                }
            }
        }

        // Log statistics every 10 ticks
        if self.current_tick % 10 == 0 {
            self.log_statistics();
        }
    }

    /// Generate an action based on drive type and position
    fn generate_action_for_drive(drive_type: DriveType, position: (i32, i32, i32)) -> Action {
        // Map drive type to a representative action
        match drive_type {
            DriveType::Hunger => Action::Eat { food_type: "generic".to_string() },
            DriveType::Rest => Action::Sleep { duration: 10 },
            DriveType::Shelter => Action::Build {
                structure_type: "shelter".to_string(),
                position
            },
            DriveType::Construction => Action::Build {
                structure_type: "structure".to_string(),
                position
            },
            DriveType::Industry => Action::Gather { resource_type: "generic".to_string() },
            DriveType::Curiosity => Action::Explore { direction: (1, 0, 0) },
            DriveType::Social => Action::Socialize { target_agent_id: uuid::Uuid::nil() },
            DriveType::Utility => Action::Craft { item_type: "tool".to_string() },
            DriveType::Preparedness => Action::Store { item_type: "resource".to_string(), amount: 1 },
            DriveType::Sustenance => Action::Gather { resource_type: "food".to_string() },
            DriveType::Safety => Action::Move { target: position },
            DriveType::Reproduction => Action::Wait,
            DriveType::Luxury => Action::Gather { resource_type: "luxury".to_string() },
            DriveType::Thirst => Action::Eat { food_type: "water".to_string() },
        }
    }

    /// Execute an action in the environment and return the result
    fn execute_action(&mut self, action: &Action, agent_index: usize) -> ActionResult {
        use rand::Rng;
        use crate::world::{ResourceType, Position};

        let mut rng = rand::thread_rng();

        match action {
            Action::Eat { food_type } => {
                // Find nearby food resources
                let agent = &self.population.agents[agent_index];
                let agent_pos = Position::new(
                    agent.state.position.0,
                    agent.state.position.1
                );

                // Look for food within a 25-tile radius (half the world size)
                let mut nearest_food: Option<(usize, u32)> = None;
                for (i, resource) in self.world.resources.iter().enumerate() {
                    if resource.resource_type == ResourceType::Food && resource.amount > 0 {
                        let distance = agent_pos.distance_to(&resource.position);
                        if distance <= 25 {
                            if let Some((_, nearest_dist)) = nearest_food {
                                if distance < nearest_dist {
                                    nearest_food = Some((i, distance));
                                }
                            } else {
                                nearest_food = Some((i, distance));
                            }
                        }
                    }
                }

                if let Some((food_index, _)) = nearest_food {
                    // Harvest food
                    let harvested = self.world.resources[food_index].harvest(1);

                    if harvested > 0 {
                        // Calculate energy restored (food provides 20-40 energy)
                        let energy_restored = rng.gen_range(20.0..40.0);

                        // Agent eats the food
                        let agent = &mut self.population.agents[agent_index];
                        agent.state.eat(self.current_tick, energy_restored);

                        debug!(
                            "Agent {} ate food, restored {:.1} energy, reset starvation timer",
                            agent.id, energy_restored
                        );

                        ActionResult::success()
                            .with_drive_change(DriveType::Hunger, -0.3)
                            .with_energy_cost(5.0) // Small energy cost to gather/eat
                            .with_message(format!("Ate {} and restored {:.1} energy", food_type, energy_restored))
                    } else {
                        ActionResult::failure("Food source was empty".to_string())
                    }
                } else {
                    // No food nearby, agent fails to eat
                    ActionResult::failure("No food sources nearby".to_string())
                }
            },

            Action::Sleep { duration } => {
                // Restore energy based on sleep duration
                let agent = &mut self.population.agents[agent_index];
                let energy_restored = (*duration as f32) * 2.0; // 2 energy per tick
                agent.state.energy = (agent.state.energy + energy_restored).min(100.0);

                ActionResult::success()
                    .with_drive_change(DriveType::Rest, -0.5)
                    .with_message(format!("Slept for {} ticks, restored {:.1} energy", duration, energy_restored))
            },

            Action::Gather { resource_type } => {
                use crate::world::{ResourceType, Position};
                use crate::agents::InventoryItem;

                // Map resource string to ResourceType
                let resource_type_enum = match resource_type.as_str() {
                    "wood" => Some(ResourceType::Wood),
                    "stone" => Some(ResourceType::Stone),
                    "iron" => Some(ResourceType::Iron),
                    "food" => Some(ResourceType::Food),
                    "generic" => Some(ResourceType::Wood), // Default to wood for generic
                    _ => None,
                };

                if resource_type_enum.is_none() {
                    return ActionResult::failure(format!("Unknown resource type: {}", resource_type));
                }
                let resource_type_enum = resource_type_enum.unwrap();

                // Get agent position
                let agent = &self.population.agents[agent_index];
                let agent_pos = Position::new(
                    agent.state.position.0,
                    agent.state.position.1
                );

                // Look for resources within a 25-tile radius
                let mut nearest_resource: Option<(usize, u32)> = None;
                for (i, resource) in self.world.resources.iter().enumerate() {
                    if resource.resource_type == resource_type_enum && resource.amount > 0 {
                        let distance = agent_pos.distance_to(&resource.position);
                        if distance <= 25 {
                            if let Some((_, nearest_dist)) = nearest_resource {
                                if distance < nearest_dist {
                                    nearest_resource = Some((i, distance));
                                }
                            } else {
                                nearest_resource = Some((i, distance));
                            }
                        }
                    }
                }

                if let Some((resource_index, _)) = nearest_resource {
                    // Determine harvest amount based on resource type and skill
                    let harvest_amount = match resource_type_enum {
                        ResourceType::Wood => rng.gen_range(1..=3),
                        ResourceType::Stone => rng.gen_range(1..=2),
                        ResourceType::Iron => 1,
                        ResourceType::Food => 1,
                        _ => 1,
                    };

                    // Harvest resource
                    let harvested = self.world.resources[resource_index].harvest(harvest_amount);

                    if harvested > 0 {
                        // Add to agent inventory
                        let item_id = match resource_type_enum {
                            ResourceType::Wood => "wood",
                            ResourceType::Stone => "stone",
                            ResourceType::Iron => "iron",
                            ResourceType::Food => "food",
                            _ => "generic",
                        };

                        let item = InventoryItem::new_with_weight(
                            item_id.to_string(),
                            harvested,
                            match resource_type_enum {
                                ResourceType::Wood => 2.0,     // Wood is light but bulky
                                ResourceType::Stone => 5.0,    // Stone is heavy
                                ResourceType::Iron => 8.0,     // Iron is very heavy
                                ResourceType::Food => 0.5,     // Food is light
                                _ => 1.0,
                            }
                        );

                        let agent = &mut self.population.agents[agent_index];
                        if agent.inventory.add_item(item) {
                            debug!(
                                "Agent {} gathered {} {} (total weight: {:.1}/{:.1})",
                                agent.id, harvested, item_id,
                                agent.inventory.current_weight, agent.inventory.max_weight
                            );

                            ActionResult::success()
                                .with_drive_change(DriveType::Industry, -0.15)
                                .with_energy_cost(10.0)
                                .with_message(format!("Gathered {} {}", harvested, resource_type))
                        } else {
                            ActionResult::failure("Inventory full - cannot carry more".to_string())
                        }
                    } else {
                        ActionResult::failure("Resource source was empty".to_string())
                    }
                } else {
                    // No resource nearby
                    ActionResult::failure(format!("No {} sources nearby", resource_type))
                }
            },

            Action::Build { structure_type, position } => {
                use crate::world::{BuildingType, Building, Position, ResourceType};

                // Map structure string to BuildingType
                let building_type = match structure_type.as_str() {
                    "shelter" | "smallhouse" => BuildingType::SmallHouse,
                    "mediumhouse" => BuildingType::MediumHouse,
                    "largehouse" => BuildingType::LargeHouse,
                    "workshop" => BuildingType::Workshop,
                    "storehouse" => BuildingType::Storehouse,
                    "farm" => BuildingType::Farm,
                    "structure" => BuildingType::SmallHouse, // Default
                    _ => BuildingType::SmallHouse, // Default fallback
                };

                // Get resource requirements for this building
                let requirements = building_type.requirements();

                // Check if agent has required resources in inventory
                let agent = &self.population.agents[agent_index];
                let mut has_all_resources = true;
                let mut missing_resources = Vec::new();

                for req in &requirements {
                    let item_id = match req.resource_type {
                        ResourceType::Wood => "wood",
                        ResourceType::Stone => "stone",
                        ResourceType::Iron => "iron",
                        _ => continue,
                    };

                    if let Some(item) = agent.inventory.get_item(item_id) {
                        if item.quantity < req.amount {
                            has_all_resources = false;
                            missing_resources.push(format!("{} {} (have {})", req.amount - item.quantity, item_id, item.quantity));
                        }
                    } else {
                        has_all_resources = false;
                        missing_resources.push(format!("{} {}", req.amount, item_id));
                    }
                }

                if !has_all_resources {
                    return ActionResult::failure(format!(
                        "Missing resources for {:?}: {}",
                        building_type,
                        missing_resources.join(", ")
                    ));
                }

                // Check if position is valid and not occupied
                let build_pos = Position::new(position.0, position.1);
                if self.world.is_position_occupied(&build_pos) {
                    return ActionResult::failure("Position already occupied".to_string());
                }

                // Remove resources from agent inventory
                let agent = &mut self.population.agents[agent_index];
                for req in &requirements {
                    let item_id = match req.resource_type {
                        ResourceType::Wood => "wood",
                        ResourceType::Stone => "stone",
                        ResourceType::Iron => "iron",
                        _ => continue,
                    };

                    agent.inventory.remove_item(item_id, req.amount);
                }

                // Create new building (under construction)
                let building = Building::new_under_construction(building_type, build_pos);

                // Add building to world
                self.world.add_building(building);

                debug!(
                    "Agent {} started construction of {:?} at ({}, {})",
                    agent.id, building_type, position.0, position.1
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Construction, -0.2)
                    .with_energy_cost(20.0)
                    .with_message(format!("Started building {:?}", building_type))
            },

            // For other actions, use simplified success/failure
            _ => {
                let success_probability = 0.7;
                if rng.gen_bool(success_probability) {
                    let satisfaction = match action {
                        Action::Craft { .. } => 0.2,
                        Action::Store { .. } => 0.1,
                        Action::Explore { .. } => 0.15,
                        Action::Socialize { .. } => 0.2,
                        Action::Move { .. } => 0.05,
                        Action::Wait => 0.0,
                        _ => 0.1,
                    };

                    ActionResult::success()
                        .with_message(format!("{:?} succeeded", action))
                } else {
                    ActionResult::failure(format!("{:?} failed", action))
                }
            }
        }
    }

    /// Log simulation statistics
    fn log_statistics(&self) {
        info!("--- Tick {} Statistics ---", self.current_tick);
        info!("Population size: {}", self.population.agents.len());

        // Aggregate drive statistics
        let mut total_hunger = 0.0;
        let mut total_rest = 0.0;
        let mut total_curiosity = 0.0;

        for agent in &self.population.agents {
            if let Some(hunger) = agent.drives.get(DriveType::Hunger) {
                total_hunger += hunger.value;
            }
            if let Some(rest) = agent.drives.get(DriveType::Rest) {
                total_rest += rest.value;
            }
            if let Some(curiosity) = agent.drives.get(DriveType::Curiosity) {
                total_curiosity += curiosity.value;
            }
        }

        let agent_count = self.population.agents.len() as f32;
        if agent_count > 0.0 {
            info!("Average Hunger: {:.2}", total_hunger / agent_count);
            info!("Average Rest: {:.2}", total_rest / agent_count);
            info!("Average Curiosity: {:.2}", total_curiosity / agent_count);
        }
    }
}
