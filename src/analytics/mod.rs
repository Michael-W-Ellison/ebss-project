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

        // Process environmental damage (exposure, falling, disease)
        self.process_environmental_damage();

        // Tick world (building construction progress, etc.)
        self.world.tick();

        // Log statistics every 10 ticks
        if self.current_tick % 10 == 0 {
            self.log_statistics();
        }
    }

    /// Generate an action based on drive type and position
    fn generate_action_for_drive(drive_type: DriveType, position: (i32, i32, i32)) -> Action {
        use rand::Rng;
        let mut rng = rand::thread_rng();

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
            DriveType::Curiosity => {
                // Explore by moving to a random distant location
                let target_x = position.0 + rng.gen_range(-20..=20);
                let target_y = position.1 + rng.gen_range(-20..=20);
                Action::Move { target: (target_x, target_y, position.2) }
            },
            DriveType::Social => Action::Socialize { target_agent_id: uuid::Uuid::nil() },
            DriveType::Utility => Action::Craft { item_type: "woodenaxe".to_string() },
            DriveType::Preparedness => Action::Store { item_type: "resource".to_string(), amount: 1 },
            DriveType::Sustenance => Action::Gather { resource_type: "food".to_string() },
            DriveType::Safety => {
                // Move to a random nearby safe location
                let target_x = position.0 + rng.gen_range(-5..=5);
                let target_y = position.1 + rng.gen_range(-5..=5);
                Action::Move { target: (target_x, target_y, position.2) }
            },
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

            Action::Attack { target_agent_id, weapon } => {
                use crate::agents::body::{BodyPartType, InjuryType};

                // Find target agent
                let target_index = self.population.agents.iter()
                    .position(|a| &a.id == target_agent_id);

                if target_index.is_none() {
                    return ActionResult::failure("Target agent not found".to_string());
                }
                let target_index = target_index.unwrap();

                // Can't attack yourself
                if target_index == agent_index {
                    return ActionResult::failure("Cannot attack yourself".to_string());
                }

                // Get attacker and target positions
                let attacker_pos = self.population.agents[agent_index].state.position;
                let target_pos = self.population.agents[target_index].state.position;

                // Check if target is in range (melee range = 1 tile, unarmed or melee weapon)
                let distance = ((target_pos.0 - attacker_pos.0).abs() + (target_pos.1 - attacker_pos.1).abs()) as u32;
                let max_range = 1; // Melee range for now

                if distance > max_range {
                    return ActionResult::failure(format!("Target too far away (distance: {})", distance));
                }

                // Calculate base damage
                let base_damage = if weapon.is_some() {
                    // Weapon damage (will be expanded later with actual weapon stats)
                    rng.gen_range(10.0..25.0)
                } else {
                    // Unarmed combat
                    rng.gen_range(5.0..15.0)
                };

                // Get attacker's tool efficiency (arm health affects combat)
                let attacker_efficiency = self.population.agents[agent_index].body.tool_efficiency_multiplier();
                let actual_damage = base_damage * attacker_efficiency;

                // Select random body part to hit (weighted toward torso/limbs)
                let body_parts = [
                    (BodyPartType::Head, 10),       // 10% chance (critical)
                    (BodyPartType::Torso, 30),      // 30% chance (common target)
                    (BodyPartType::LeftArm, 15),    // 15% chance
                    (BodyPartType::RightArm, 15),   // 15% chance
                    (BodyPartType::LeftLeg, 12),    // 12% chance
                    (BodyPartType::RightLeg, 12),   // 12% chance
                    (BodyPartType::Back, 6),        // 6% chance (hard to hit)
                ];

                let total_weight: u32 = body_parts.iter().map(|(_, w)| w).sum();
                let roll = rng.gen_range(0..total_weight);

                let mut cumulative = 0;
                let mut target_part = BodyPartType::Torso; // Default
                for (part, weight) in &body_parts {
                    cumulative += weight;
                    if roll < cumulative {
                        target_part = *part;
                        break;
                    }
                }

                // Determine injury type based on damage and weapon
                let injury_type = if actual_damage >= 30.0 {
                    // High damage can cause crippling injuries
                    if rng.gen_bool(0.3) {
                        InjuryType::Crippling(crate::agents::body::CripplingType::Partial)
                    } else {
                        InjuryType::Major
                    }
                } else if actual_damage >= 15.0 {
                    InjuryType::Major
                } else {
                    InjuryType::Minor
                };

                // Apply damage to target
                let target = &mut self.population.agents[target_index];
                if let Some(part) = target.body.get_part_mut(target_part) {
                    part.apply_injury(injury_type, actual_damage, self.current_tick as u64);
                }

                // Also reduce target's overall health
                let target = &mut self.population.agents[target_index];
                target.state.health = (target.state.health - actual_damage * 0.2).max(0.0);

                // Check if target died from the attack
                let target_alive = self.population.agents[target_index].body.is_alive()
                    && self.population.agents[target_index].state.health > 0.0;

                debug!(
                    "Agent {} attacked Agent {} ({:?}): {:.1} damage to {:?} ({})",
                    self.population.agents[agent_index].id,
                    self.population.agents[target_index].id,
                    weapon.as_ref().unwrap_or(&"unarmed".to_string()),
                    actual_damage,
                    target_part,
                    if target_alive { "survived" } else { "FATAL" }
                );

                if !target_alive {
                    ActionResult::success()
                        .with_drive_change(DriveType::Safety, -0.3)
                        .with_energy_cost(25.0)
                        .with_message(format!(
                            "Attacked and killed target ({:.1} damage to {:?})",
                            actual_damage, target_part
                        ))
                } else {
                    ActionResult::success()
                        .with_drive_change(DriveType::Safety, -0.2)
                        .with_energy_cost(15.0)
                        .with_message(format!(
                            "Attacked target ({:.1} damage to {:?}, {:?} injury)",
                            actual_damage, target_part, injury_type
                        ))
                }
            },

            Action::Craft { item_type } => {
                use crate::world::production::{Quality as ProductionQuality, Recipe, ResourceRequirement, ProductionOutput};
                use crate::world::{ItemType, ResourceType};
                use crate::agents::skills::SkillType;

                // Define skill and technology-based crafting recipes
                // Format: (recipe, required_skill_level, required_technology)
                // Skill levels: -10 to 10, where 0 is untrained adult
                // Technology: Optional technology ID that must be known
                let skill_gated_recipes: Vec<(Recipe, i32, Option<&str>)> = vec![
                    // BEGINNER (skill -10 to 0): Basic wooden tools - requires wooden_tools technology
                    (Recipe {
                        name: "Craft Wooden Axe",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenAxe, 1)],
                        base_time: 80,
                    }, -5, Some("wooden_tools")),  // Very easy, needs wooden tools tech

                    (Recipe {
                        name: "Craft Wooden Pickaxe",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenPickaxe, 1)],
                        base_time: 80,
                    }, -5, Some("wooden_tools")),

                    (Recipe {
                        name: "Craft Wooden Hammer",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenHammer, 1)],
                        base_time: 80,
                    }, -5, Some("wooden_tools")),

                    (Recipe {
                        name: "Craft Wooden Spear",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Wood, 2),
                            ResourceRequirement::new(ResourceType::Stone, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenSpear, 1)],
                        base_time: 85,
                    }, 1, Some("wooden_tools")),

                    // NOVICE (skill 0-3): Stone tools - requires stone_tools technology
                    (Recipe {
                        name: "Craft Stone Axe",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Stone, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::StoneAxe, 1)],
                        base_time: 90,
                    }, 0, Some("stone_tools")),  // Requires basic training + stone tools tech

                    (Recipe {
                        name: "Craft Stone Pickaxe",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Stone, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::StonePickaxe, 1)],
                        base_time: 90,
                    }, 0, Some("stone_tools")),

                    (Recipe {
                        name: "Craft Stone Hammer",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Stone, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::StoneHammer, 1)],
                        base_time: 90,
                    }, 0, Some("stone_tools")),

                    // APPRENTICE (skill 3-5): Iron tools - requires iron_working technology
                    (Recipe {
                        name: "Craft Iron Axe",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Iron, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::IronAxe, 1)],
                        base_time: 100,
                    }, 3, Some("iron_working")),  // Requires experience + iron working tech

                    (Recipe {
                        name: "Craft Iron Pickaxe",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Iron, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::IronPickaxe, 1)],
                        base_time: 100,
                    }, 3, Some("iron_working")),

                    (Recipe {
                        name: "Craft Iron Hammer",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Iron, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::IronHammer, 1)],
                        base_time: 100,
                    }, 3, Some("iron_working")),

                    // JOURNEYMAN (skill 5-8): Advanced weapons - requires iron_working technology
                    (Recipe {
                        name: "Craft Iron Sword",
                        job: crate::agents::profession::JobType::Unemployed,
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Iron, 3),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::IronSword, 1)],
                        base_time: 120,
                    }, 5, Some("iron_working")),  // Requires significant experience + iron working tech
                ];

                // Get agent's crafting skill level and known technologies
                let (agent_skill_level, agent_known_techs) = {
                    let agent = &mut self.population.agents[agent_index];
                    let skill = agent.skills.get_skill(SkillType::Crafting).level;
                    let techs: std::collections::HashSet<String> = agent.technology_knowledge
                        .known_technologies.keys().cloned().collect();
                    (skill, techs)
                };

                // Filter recipes by skill level AND technology - only show recipes agent can craft
                let available_recipes: Vec<(&Recipe, i32, Option<&str>)> = skill_gated_recipes
                    .iter()
                    .filter(|(_, required_skill, required_tech)| {
                        // Check skill requirement
                        if agent_skill_level < *required_skill {
                            return false;
                        }

                        // Check technology requirement
                        if let Some(tech_id) = required_tech {
                            if !agent_known_techs.contains(*tech_id) {
                                return false;
                            }
                        }

                        true
                    })
                    .map(|(recipe, skill, tech)| (recipe, *skill, *tech))
                    .collect();

                // Try to find a recipe that matches the item type
                let recipe_match = available_recipes.iter().find(|(r, _, _)| {
                    r.outputs.iter().any(|output| {
                        format!("{:?}", output.item_type).to_lowercase() == item_type.to_lowercase()
                    })
                });

                // If no recipe found in available recipes, check if it exists but agent doesn't meet requirements
                if recipe_match.is_none() {
                    // Find the recipe in the full list to give a helpful error message
                    let full_recipe = skill_gated_recipes.iter().find(|(r, _, _)| {
                        r.outputs.iter().any(|output| {
                            format!("{:?}", output.item_type).to_lowercase() == item_type.to_lowercase()
                        })
                    });

                    if let Some((_, required_skill, required_tech)) = full_recipe {
                        // Determine what's missing
                        let mut reasons = Vec::new();

                        if agent_skill_level < *required_skill {
                            reasons.push(format!("insufficient skill (need {}, have {})",
                                required_skill, agent_skill_level));
                        }

                        if let Some(tech_id) = required_tech {
                            if !agent_known_techs.contains(*tech_id) {
                                reasons.push(format!("missing technology '{}'", tech_id));
                            }
                        }

                        return ActionResult::failure(format!(
                            "Cannot craft {}: {}",
                            item_type,
                            reasons.join(", ")
                        ));
                    } else {
                        return ActionResult::failure(format!(
                            "Unknown recipe: {}",
                            item_type
                        ));
                    }
                }
                let (recipe, _, _) = recipe_match.unwrap();

                // Check if agent has all required materials in inventory
                let agent = &self.population.agents[agent_index];
                let mut has_all_materials = true;
                let mut missing_materials = Vec::new();

                for req in &recipe.inputs {
                    let item_id = match req.resource_type {
                        ResourceType::Wood => "wood",
                        ResourceType::Stone => "stone",
                        ResourceType::Iron => "iron",
                        ResourceType::Food => "food",
                        _ => continue,
                    };

                    if let Some(item) = agent.inventory.get_item(item_id) {
                        if item.quantity < req.amount {
                            has_all_materials = false;
                            missing_materials.push(format!("{} {} (have {})",
                                req.amount - item.quantity, item_id, item.quantity));
                        }
                    } else {
                        has_all_materials = false;
                        missing_materials.push(format!("{} {}", req.amount, item_id));
                    }
                }

                if !has_all_materials {
                    return ActionResult::failure(format!(
                        "Missing materials for {}: {}",
                        recipe.name,
                        missing_materials.join(", ")
                    ));
                }

                // Get agent's crafting skill level (-10 to 10)
                let agent = &mut self.population.agents[agent_index];
                let skill_level = agent.skills.get_skill_mut(SkillType::Crafting).level;

                // Convert skill level (-10 to 10) to skill value (0 to 100) for quality calculation
                // -10 -> 0, 0 -> 50, 10 -> 100
                let skill_value = ((skill_level + 10) * 5) as u8;

                // Determine quality based on skill
                let quality = ProductionQuality::from_skill(skill_value);

                // Calculate actual outputs with quality multiplier
                let outputs = recipe.calculate_output(quality);

                // Consume materials from inventory
                for req in &recipe.inputs {
                    let item_id = match req.resource_type {
                        ResourceType::Wood => "wood",
                        ResourceType::Stone => "stone",
                        ResourceType::Iron => "iron",
                        ResourceType::Food => "food",
                        _ => continue,
                    };
                    agent.inventory.remove_item(item_id, req.amount);
                }

                // Add crafted items to inventory
                for (output_item, quantity) in outputs {
                    let item_id = format!("{:?}", output_item).to_lowercase();

                    // Create inventory item with appropriate weight
                    let item = crate::agents::InventoryItem::new_with_weight(
                        item_id.clone(),
                        quantity,
                        5.0, // Default weight for crafted tools
                    );

                    if !agent.inventory.add_item(item) {
                        debug!(
                            "Agent {} crafted {} but inventory full, item dropped",
                            agent.id, item_id
                        );
                    }
                }

                // Grant crafting experience
                let experience_gained = match quality {
                    ProductionQuality::Poor => 1,
                    ProductionQuality::Common => 2,
                    ProductionQuality::Good => 3,
                    ProductionQuality::Excellent => 4,
                    ProductionQuality::Masterwork => 5,
                };

                agent.skills.get_skill_mut(SkillType::Crafting).gain_experience(experience_gained);

                debug!(
                    "Agent {} crafted {} (quality: {:?}, skill: {}, exp: +{})",
                    agent.id, recipe.name, quality, skill_level, experience_gained
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.2)
                    .with_energy_cost(15.0)
                    .with_message(format!("Crafted {} ({:?} quality)", recipe.name, quality))
            },

            Action::Move { target } => {
                use crate::world::grid::Position;

                // Get agent current position
                let agent = &self.population.agents[agent_index];
                let current_pos = agent.state.position;
                let current_2d = Position::new(current_pos.0, current_pos.1);

                // Target position (2D, ignore z for now)
                let target_2d = Position::new(target.0, target.1);

                // Check if already at target
                if current_2d == target_2d {
                    return ActionResult::success()
                        .with_message("Already at destination".to_string());
                }

                // Calculate movement distance (Manhattan distance)
                let distance = current_2d.distance_to(&target_2d);

                // Determine next step towards target (simple pathfinding - move one step)
                let dx = target.0 - current_pos.0;
                let dy = target.1 - current_pos.1;

                // Normalize to -1, 0, or 1 for each axis
                let step_x = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
                let step_y = if dy > 0 { 1 } else if dy < 0 { -1 } else { 0 };

                // Prioritize longer axis for movement
                let (next_x, next_y) = if dx.abs() >= dy.abs() {
                    (current_pos.0 + step_x, current_pos.1)
                } else {
                    (current_pos.0, current_pos.1 + step_y)
                };

                let next_pos = Position::new(next_x, next_y);

                // Check if next position is within world bounds
                let world_width = self.world.grid.width as i32;
                let world_height = self.world.grid.height as i32;

                if next_x < 0 || next_x >= world_width || next_y < 0 || next_y >= world_height {
                    return ActionResult::failure("Cannot move outside world bounds".to_string());
                }

                // Check if position is passable (not water, not occupied by building)
                if let Some(tile) = self.world.grid.get_tile(&next_pos) {
                    use crate::world::TerrainType;
                    if tile.terrain.terrain_type == TerrainType::Water {
                        return ActionResult::failure("Cannot move into water".to_string());
                    }
                }

                // Check if position is occupied by a building
                if self.world.is_position_occupied(&next_pos) {
                    return ActionResult::failure("Position blocked by building".to_string());
                }

                // Get movement speed multiplier from leg health
                let agent = &self.population.agents[agent_index];
                let movement_speed = agent.body.movement_speed_multiplier();

                // Base energy cost (modified by speed and distance)
                let base_energy_cost = 2.0;
                let actual_energy_cost = if movement_speed > 0.1 {
                    base_energy_cost / movement_speed
                } else {
                    // Legs too damaged, can't move
                    return ActionResult::failure("Too injured to move (legs crippled)".to_string());
                };

                // Update agent position
                let agent = &mut self.population.agents[agent_index];
                agent.state.position = (next_x, next_y, target.2);

                debug!(
                    "Agent {} moved from ({}, {}) to ({}, {}) (distance to target: {}, speed: {:.2}x)",
                    agent.id, current_pos.0, current_pos.1, next_x, next_y,
                    distance - 1, movement_speed
                );

                // Determine drive satisfaction based on purpose (Safety or Curiosity)
                let drive_type = if distance <= 5 {
                    Some(DriveType::Safety) // Moving to nearby location (fleeing or seeking safety)
                } else {
                    Some(DriveType::Curiosity) // Exploring distant location
                };

                let mut result = ActionResult::success()
                    .with_energy_cost(actual_energy_cost)
                    .with_message(format!("Moved to ({}, {}), {} steps to goal", next_x, next_y, distance - 1));

                if let Some(drive) = drive_type {
                    result = result.with_drive_change(drive, -0.05);
                }

                result
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

    /// Process environmental damage for all agents
    pub fn process_environmental_damage(&mut self) {
        use crate::agents::body::{BodyPartType, InjuryType, CripplingType};
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for agent in &mut self.population.agents {
            // 1. EXPOSURE DAMAGE - Cold/Heat based on environment
            // Check if agent has adequate protection from equipment
            let cold_insulation = agent.body.total_cold_insulation();
            let heat_resistance = agent.body.total_heat_resistance();

            // Simplified environmental model - can be enhanced with actual world temperature
            // Assume baseline comfortable temperature, extreme cold/heat causes damage
            // In a full implementation, this would check world.get_temperature_at(position)

            // Cold exposure (lack of insulation)
            if cold_insulation < 1.0 {
                // Missing adequate cold protection
                let exposure_severity = 1.0 - cold_insulation;
                if rng.gen_bool(exposure_severity as f64 * 0.01) { // 1% chance per severity point
                    let cold_damage = rng.gen_range(1.0..5.0);
                    // Cold affects extremities most
                    let affected_parts = [
                        BodyPartType::LeftArm,
                        BodyPartType::RightArm,
                        BodyPartType::LeftLeg,
                        BodyPartType::RightLeg,
                    ];
                    let part = affected_parts[rng.gen_range(0..affected_parts.len())];

                    if let Some(body_part) = agent.body.get_part_mut(part) {
                        body_part.apply_injury(InjuryType::Minor, cold_damage, self.current_tick as u64);
                        debug!("Agent {} suffered cold exposure: {:.1} damage to {:?}",
                            agent.id, cold_damage, part);
                    }
                }
            }

            // Heat exposure (lack of heat resistance) - less common, more severe
            if heat_resistance < 0.5 {
                let exposure_severity = 0.5 - heat_resistance;
                if rng.gen_bool(exposure_severity as f64 * 0.005) { // 0.5% chance per severity
                    let heat_damage = rng.gen_range(2.0..8.0);
                    // Heat affects torso and head
                    let affected_parts = [BodyPartType::Head, BodyPartType::Torso];
                    let part = affected_parts[rng.gen_range(0..affected_parts.len())];

                    if let Some(body_part) = agent.body.get_part_mut(part) {
                        body_part.apply_injury(InjuryType::Minor, heat_damage, self.current_tick as u64);
                        debug!("Agent {} suffered heat exposure: {:.1} damage to {:?}",
                            agent.id, heat_damage, part);
                    }
                }
            }

            // 2. FALLING DAMAGE - Based on height/terrain
            // In a full implementation, this would check for actual falls
            // For now, simulate random accidents
            if rng.gen_bool(0.0001) { // 0.01% chance per tick (~14 falls per million ticks)
                let fall_height = rng.gen_range(1..=5); // Units of height
                let fall_damage = (fall_height as f32) * rng.gen_range(3.0..8.0);

                // Falls primarily affect legs, with chance of head/torso on severe falls
                let injured_part = if fall_height >= 4 && rng.gen_bool(0.3) {
                    // High fall with head/torso injury
                    if rng.gen_bool(0.5) {
                        BodyPartType::Head
                    } else {
                        BodyPartType::Torso
                    }
                } else {
                    // Normal fall - legs
                    if rng.gen_bool(0.5) {
                        BodyPartType::LeftLeg
                    } else {
                        BodyPartType::RightLeg
                    }
                };

                let injury_severity = if fall_damage >= 25.0 {
                    InjuryType::Crippling(CripplingType::Partial)
                } else if fall_damage >= 12.0 {
                    InjuryType::Major
                } else {
                    InjuryType::Minor
                };

                if let Some(body_part) = agent.body.get_part_mut(injured_part) {
                    body_part.apply_injury(injury_severity, fall_damage, self.current_tick as u64);
                    debug!("Agent {} suffered fall damage: {:.1} damage to {:?} ({:?})",
                        agent.id, fall_damage, injured_part, injury_severity);
                }

                // Also reduce overall health
                agent.state.health = (agent.state.health - fall_damage * 0.15).max(0.0);
            }

            // 3. DISEASE/INFECTION - Random chance
            // Agents with existing injuries have higher infection risk
            let injury_count: usize = agent.body.parts.values()
                .map(|part| part.injuries.len())
                .sum();

            if injury_count > 0 {
                let infection_chance = (injury_count as f64) * 0.0001; // 0.01% per injury per tick
                if rng.gen_bool(infection_chance) {
                    // Random body part gets infected
                    let parts: Vec<BodyPartType> = agent.body.parts.keys().cloned().collect();
                    if !parts.is_empty() {
                        let part = parts[rng.gen_range(0..parts.len())];
                        let infection_damage = rng.gen_range(0.5..2.0);

                        if let Some(body_part) = agent.body.get_part_mut(part) {
                            body_part.add_condition(crate::agents::body::Condition {
                                condition_type: crate::agents::body::ConditionType::Infected,
                                severity: rng.gen_range(0.3..0.8),
                                duration: rng.gen_range(100..500), // Lasts 100-500 ticks
                            });
                            debug!("Agent {} developed infection on {:?}", agent.id, part);
                        }
                    }
                }
            }

            // 4. NATURAL HEALING - Process body tick (handles conditions, bleeding, etc.)
            agent.body.tick();
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
