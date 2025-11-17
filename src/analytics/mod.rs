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
use log::{info, debug, warn};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs::File;
use std::io::{Write, Read};

pub struct Simulation {
    pub world: World,
    pub population: Population,
    pub current_tick: u32,
    pub renderer: Option<AsciiRenderer>,
}

pub struct SimulationConfig;
pub struct Analytics;
pub struct BehaviorAnalysis;

/// Serializable simulation state for save/load functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableSimulationState {
    world: World,
    agents: Vec<crate::agents::Agent>,
    current_tick: u32,
    population_stats: PopulationStatsSnapshot,
}

/// Snapshot of population stats for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PopulationStatsSnapshot {
    total_births: u64,
    total_deaths: u64,
    total_abandonments: u64,
}

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

        // Tick world systems (fauna and flora AI, growth, etc.)
        self.world.climate.tick();
        self.world.animals.tick();
        self.world.plants.tick();

        // Update exposure damage for all agents
        self.update_agent_exposure();

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

                // Generate goals periodically based on drives and emotions
                if self.current_tick % 50 == 0 {
                    let agent = &mut self.population.agents[agent_index];

                    // Collect current drive types and emotion values
                    let drive_types: Vec<crate::core::DriveType> = crate::core::DriveType::all().to_vec();
                    let emotion_values: Vec<(crate::core::EmotionType, f32)> = vec![
                        (crate::core::EmotionType::Happiness, agent.emotions.happiness()),
                        (crate::core::EmotionType::Fear, agent.emotions.fear),
                        (crate::core::EmotionType::Anger, agent.emotions.anger),
                        (crate::core::EmotionType::Sadness, agent.emotions.sadness),
                        (crate::core::EmotionType::Curiosity, 0.5), // Default curiosity level
                    ];

                    // Generate common goals based on current state
                    let new_goals = crate::core::goals::GoalManager::generate_common_goals(
                        &drive_types,
                        &emotion_values,
                        self.current_tick,
                    );

                    // Add generated goals to agent's goals
                    for goal in new_goals {
                        agent.goals.add_goal(goal);
                    }
                }

                // Generate action based on priority: shelter needs > percepts > goals > drives
                let action = {
                    let agent = &self.population.agents[agent_index];

                    // PRIORITY 1: Check if agent needs shelter due to exposure
                    if agent.needs_shelter() && agent.shelter_priority() > 0.7 {
                        // Critical shelter need - override all other actions
                        crate::environment::Action::SeekShelter
                    } else {
                        // PRIORITY 2: Check for high-salience percepts that should override drive-based actions
                        let percept_action = Self::generate_action_from_percepts(
                            &agent.recent_percepts,
                            &agent.drives,
                            agent_position,
                        );

                        if percept_action.is_some() {
                            percept_action.unwrap()
                        } else {
                            // PRIORITY 3: Check if we have active goals and generate goal-aligned action
                            if let Some(active_goal) = agent.goals.highest_priority_goal() {
                                let goal_action = Self::generate_action_for_goal(active_goal, agent_position, drive_type);

                                // Use goal-aligned action if available, otherwise fall back to drive-based
                                goal_action.unwrap_or_else(|| {
                                    Self::generate_action_for_drive(drive_type, agent_position)
                                })
                            } else {
                                // PRIORITY 4: Use drive-based action
                                Self::generate_action_for_drive(drive_type, agent_position)
                            }
                        }
                    }
                };

                // Execute action in environment and get feedback
                let action_result = self.execute_action(&action, agent_index);

                debug!(
                    "Agent {} - Action result: {} (satisfaction: {:.2})",
                    agent_id,
                    action_result.message.as_ref().map(|s| s.as_str()).unwrap_or("No message"),
                    action_result.drive_satisfaction
                );

                // Broadcast action to nearby observers (for observational learning)
                if action_result.success {
                    let agent = &self.population.agents[agent_index];
                    let agent_pos = agent.state.position;

                    // Map action to ActionType for broadcasting
                    if let Some(broadcast_type) = Self::map_action_to_broadcast_type(&action) {
                        self.population.broadcast_action(
                            agent_id,
                            agent_pos,
                            broadcast_type,
                            true, // success
                            format!("{:?}", action),
                            self.current_tick as u64,
                        );
                    }
                }

                // Apply feedback to agent (drive satisfaction)
                let agent = &mut self.population.agents[agent_index];
                agent.apply_feedback(&action_result, drive_type);

                // Update behavior tree weights based on action success
                if let Some(tree) = agent.select_behavior_tree() {
                    if action_result.success {
                        tree.total_successes += 1;
                    }
                }

                // Update goal progress based on action result
                if action_result.success {
                    let action_name = format!("{:?}", action);
                    let agent = &mut self.population.agents[agent_index];

                    // Check if action aligns with any active goals and update progress
                    if agent.goals.action_aligns_with_goals(&action_name) {
                        // Find the highest priority goal that aligns with this action
                        if let Some(goal) = agent.goals.highest_priority_goal() {
                            let progress_delta = match &action {
                                // Resource gathering actions
                                crate::environment::Action::Gather { .. } => 0.2,
                                crate::environment::Action::Hunt { .. } => 0.15,

                                // Building and crafting
                                crate::environment::Action::Build { .. } => 0.3,
                                crate::environment::Action::Craft { .. } => 0.25,

                                // Social actions
                                crate::environment::Action::Mate { .. } => 0.2,
                                crate::environment::Action::Socialize { .. } => 0.15,

                                // Emotional satisfaction
                                crate::environment::Action::Sleep { .. } => 0.1,
                                crate::environment::Action::Eat { .. } => 0.1,

                                _ => 0.05, // Small progress for other actions
                            };

                            let goal_id = goal.id;
                            agent.goals.update_goal_progress(goal_id, progress_delta);
                        }
                    }
                }

                // Cleanup completed goals periodically
                if self.current_tick % 100 == 0 {
                    let agent = &mut self.population.agents[agent_index];
                    agent.goals.cleanup_completed();
                }
            }

            // Check if agent should interact with storehouse (every 20 ticks, or when Preparedness is high)
            // This happens independently of drive-based actions to enable cooperative resource sharing
            if self.current_tick % 20 == 0 || {
                let agent = &self.population.agents[agent_index];
                agent.drives.get(DriveType::Preparedness)
                    .map(|d| d.value > 0.6)
                    .unwrap_or(false)
            } {
                // Calculate storehouse contents
                let (storehouse_food, storehouse_resources) = {
                    use crate::world::ItemType;
                    use crate::agents::storage_integration::{count_in_agent_inventory};

                    let food_types = vec![
                        ItemType::Food, ItemType::Bread, ItemType::Cheese,
                        ItemType::Meat, ItemType::Fish, ItemType::Honey, ItemType::Ale,
                    ];
                    let resource_types = vec![
                        ItemType::Wood, ItemType::Stone, ItemType::Iron,
                        ItemType::Clay, ItemType::Sand, ItemType::Coal,
                    ];

                    let food_total: u32 = food_types.iter()
                        .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                        .map(|item| item.quantity)
                        .sum();

                    let resource_total: u32 = resource_types.iter()
                        .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                        .map(|item| item.quantity)
                        .sum();

                    (food_total, resource_total)
                };

                // Get storage action from agent
                let storage_action = {
                    let agent = &self.population.agents[agent_index];
                    agent.decide_storage_action(storehouse_food, storehouse_resources)
                };

                // Execute storage action if one was decided
                if let Some(action) = storage_action {
                    debug!("Agent {} performing storage action: {:?}", agent_id, action);
                    let action_result = self.execute_action(&action, agent_index);

                    debug!(
                        "Agent {} - Storage action result: {}",
                        agent_id,
                        action_result.message.as_ref().map(|s| s.as_str()).unwrap_or("No message")
                    );
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

    /// Generate an action based on recent percepts (if high-salience percepts exist)
    /// Returns None if no percept warrants immediate action
    fn generate_action_from_percepts(
        recent_percepts: &[(u32, crate::agents::sensory_processing::Percept)],
        agent_drives: &crate::core::DriveState,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::sensory_processing::{Percept, calculate_salience, ThreatType};

        if recent_percepts.is_empty() {
            return None;
        }

        // Find the most salient recent percept
        let most_salient = recent_percepts.iter()
            .max_by(|(_, a), (_, b)| {
                let sal_a = calculate_salience(a, agent_drives);
                let sal_b = calculate_salience(b, agent_drives);
                sal_a.partial_cmp(&sal_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some((_, percept)) = most_salient {
            let salience = calculate_salience(percept, agent_drives);

            // Only override drive-based actions if salience is high (> 0.7)
            if salience > 0.7 {
                match percept {
                    Percept::DangerDetected { threat_type, position, severity } => {
                        // High-priority: flee from danger
                        if let Some(danger_pos) = position {
                            // Move away from danger position
                            let dx = agent_position.0 - danger_pos.0;
                            let dy = agent_position.1 - danger_pos.1;

                            // Normalize and extend to flee further
                            let distance = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
                            let flee_distance = (severity * 15.0) as i32;

                            let flee_x = agent_position.0 + ((dx as f32 / distance) * flee_distance as f32) as i32;
                            let flee_y = agent_position.1 + ((dy as f32 / distance) * flee_distance as f32) as i32;

                            return Some(Action::Move {
                                target: (flee_x, flee_y, agent_position.2),
                            });
                        } else {
                            // Unknown danger location - move to random safe spot
                            use rand::Rng;
                            let mut rng = rand::thread_rng();
                            let safe_x = agent_position.0 + rng.gen_range(-10..=10);
                            let safe_y = agent_position.1 + rng.gen_range(-10..=10);

                            return Some(Action::Move {
                                target: (safe_x, safe_y, agent_position.2),
                            });
                        }
                    }
                    Percept::ResourceDetected { resource_type, position, .. } => {
                        // High-salience resource (usually means high hunger/thirst)
                        // Move towards it
                        return Some(Action::Move {
                            target: *position,
                        });
                    }
                    Percept::AgentDetected { agent_id, position, .. } => {
                        // High-salience agent (usually means high social drive)
                        // Attempt social interaction
                        return Some(Action::Socialize {
                            target_agent_id: *agent_id,
                        });
                    }
                    _ => {
                        // Other percepts don't warrant action override
                        return None;
                    }
                }
            }
        }

        None
    }

    /// Map environment Action to observable ActionType for broadcasting
    fn map_action_to_broadcast_type(action: &Action) -> Option<crate::agents::observational_learning::ActionType> {
        use crate::agents::observational_learning::ActionType;

        match action {
            Action::Gather { .. } => Some(ActionType::Mining),
            Action::Craft { .. } => Some(ActionType::Crafting),
            Action::Build { .. } => Some(ActionType::Building),
            Action::Attack { .. } => Some(ActionType::Combat),
            Action::Hunt { .. } => Some(ActionType::Combat), // Hunting is combat-like
            Action::Tame { .. } => Some(ActionType::Social), // Taming requires social skills
            Action::CollectAnimalProduct { .. } => Some(ActionType::Crafting), // Animal husbandry
            Action::HarvestPlant { .. } => Some(ActionType::Crafting), // Plant farming
            Action::Eat { food_type } if food_type == "cooked" || food_type == "prepared" => {
                Some(ActionType::Cooking)
            }
            Action::Socialize { .. } => Some(ActionType::Social),
            Action::ShareInformation { .. } => Some(ActionType::Social), // Information sharing is social
            Action::Mate { .. } => Some(ActionType::Social), // Mating is a social interaction
            Action::Mount { .. } | Action::Dismount => Some(ActionType::ToolUse), // Mount management is tool use
            Action::Move { .. } | Action::Explore { .. } => Some(ActionType::Navigation),
            Action::Store { .. } | Action::Retrieve { .. } => Some(ActionType::ToolUse), // Resource management
            _ => None, // Sleep, Wait, etc. are not observable learning opportunities
        }
    }

    /// Generate an action based on drive type and position
    fn generate_action_for_drive(drive_type: DriveType, position: (i32, i32, i32)) -> Action {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Map drive type to a representative action
        match drive_type {
            DriveType::Hunger => Action::Eat { food_type: "generic".to_string() },
            DriveType::Thirst => Action::Gather { resource_type: "water".to_string() },
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
            DriveType::Social => {
                // 50% chance to share information, 50% to socialize
                if rng.gen_bool(0.5) {
                    Action::ShareInformation { target_agent_id: uuid::Uuid::nil() }
                } else {
                    Action::Socialize { target_agent_id: uuid::Uuid::nil() }
                }
            },
            DriveType::Utility => Action::Craft { item_type: "woodenaxe".to_string() },
            DriveType::Preparedness => Action::Store { item_type: "resource".to_string(), amount: 1 },
            DriveType::Sustenance => Action::Gather { resource_type: "food".to_string() },
            DriveType::Safety => {
                // Move to a random nearby safe location
                let target_x = position.0 + rng.gen_range(-5..=5);
                let target_y = position.1 + rng.gen_range(-5..=5);
                Action::Move { target: (target_x, target_y, position.2) }
            },
            DriveType::Reproduction => Action::Mate { target_agent_id: uuid::Uuid::nil() },
            DriveType::Luxury => Action::Gather { resource_type: "luxury".to_string() },
            DriveType::Thirst => Action::Eat { food_type: "water".to_string() },
        }
    }

    /// Generate an action based on an active goal
    fn generate_action_for_goal(
        goal: &crate::core::goals::Goal,
        position: (i32, i32, i32),
        fallback_drive: DriveType,
    ) -> Option<Action> {
        use crate::core::goals::{InternalGoal, ExternalGoal};
        use crate::core::EmotionType;

        // Check if it's an internal goal
        if let Some(internal) = &goal.internal {
            match internal {
                InternalGoal::IncreaseEmotion(emotion_type, _target) => {
                    // Map emotions to actions that satisfy them
                    match emotion_type {
                        EmotionType::Happiness => Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() }),
                        EmotionType::Curiosity => Some(Action::Move {
                            target: (position.0 + 10, position.1 + 10, position.2)
                        }),
                        _ => None,
                    }
                },
                InternalGoal::DecreaseEmotion(emotion_type, _target) => {
                    match emotion_type {
                        EmotionType::Fear => Some(Action::SeekShelter),
                        EmotionType::Anger => Some(Action::Sleep { duration: 10 }),
                        EmotionType::Sadness => Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() }),
                        _ => None,
                    }
                },
                InternalGoal::MaintainWellBeing(_threshold) => {
                    Some(Action::Sleep { duration: 10 })
                },
                InternalGoal::ReduceStress => {
                    Some(Action::Sleep { duration: 10 })
                },
                InternalGoal::SeekEntertainment => {
                    Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() })
                },
            }
        // Check if it's an external goal
        } else if let Some(external) = &goal.external {
            match external {
                ExternalGoal::OwnHouse => {
                    Some(Action::Build {
                        structure_type: "house".to_string(),
                        position,
                    })
                },
                ExternalGoal::StockHouseFood(_amount) => {
                    Some(Action::Gather { resource_type: "food".to_string() })
                },
                ExternalGoal::ContributeFoodToStorehouse(amount) => {
                    Some(Action::Store {
                        item_type: "food".to_string(),
                        amount: *amount,
                    })
                },
                ExternalGoal::ObtainProtection => {
                    Some(Action::Craft { item_type: "leatherarmor".to_string() })
                },
                ExternalGoal::CraftItem(item_name) => {
                    Some(Action::Craft { item_type: item_name.clone() })
                },
                ExternalGoal::BuildStructure(structure_name) => {
                    Some(Action::Build {
                        structure_type: structure_name.clone(),
                        position,
                    })
                },
                ExternalGoal::GatherResource(resource_name, _amount) => {
                    Some(Action::Gather { resource_type: resource_name.clone() })
                },
                ExternalGoal::LearnSkill(_skill_name) => {
                    // Learning happens through practice - choose relevant action
                    // For now, map to a generic action
                    None
                },
                ExternalGoal::FormRelationship(_relationship_type) => {
                    Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() })
                },
                ExternalGoal::CompleteJob(_job_name) => {
                    // Jobs are complex, fall back to drive-based action
                    None
                },
                ExternalGoal::ContributeMaterialsToStorehouse(amount) => {
                    Some(Action::Store {
                        item_type: "resource".to_string(),
                        amount: *amount,
                    })
                },
                ExternalGoal::EnsureToolsAvailable(_count) => {
                    Some(Action::Craft { item_type: "woodenaxe".to_string() })
                },
            }
        } else {
            // Goal has neither internal nor external set (shouldn't happen)
            None
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
                let attacker = &self.population.agents[agent_index];
                let attacker_efficiency = attacker.body.tool_efficiency_multiplier();

                // Get mounted combat bonus (warhorses provide significant advantage!)
                let mount_bonus = attacker.transport.mounted_combat_bonus();
                let combat_multiplier = 1.0 + mount_bonus;

                // Apply all modifiers: base * arm_health * mount_bonus
                let actual_damage = base_damage * attacker_efficiency * combat_multiplier;

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

                let attacker_id = self.population.agents[agent_index].id;
                let attacker_mounted = self.population.agents[agent_index].transport.is_mounted();

                debug!(
                    "Agent {} attacked Agent {} ({:?}): {:.1} damage to {:?} ({}, mounted: {}, bonus: +{:.0}%)",
                    attacker_id,
                    self.population.agents[target_index].id,
                    weapon.as_ref().unwrap_or(&"unarmed".to_string()),
                    actual_damage,
                    target_part,
                    if target_alive { "survived" } else { "FATAL" },
                    if attacker_mounted { "yes" } else { "no" },
                    mount_bonus * 100.0
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
                        inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenAxe, 1)],
                        base_time: 80,
                    }, -5, Some("wooden_tools")),  // Very easy, needs wooden tools tech

                    (Recipe {
                        name: "Craft Wooden Pickaxe",
                        inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenPickaxe, 1)],
                        base_time: 80,
                    }, -5, Some("wooden_tools")),

                    (Recipe {
                        name: "Craft Wooden Hammer",
                        inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                        outputs: vec![ProductionOutput::new(ItemType::WoodenHammer, 1)],
                        base_time: 80,
                    }, -5, Some("wooden_tools")),

                    (Recipe {
                        name: "Craft Wooden Spear",
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
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Stone, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::StoneAxe, 1)],
                        base_time: 90,
                    }, 0, Some("stone_tools")),  // Requires basic training + stone tools tech

                    (Recipe {
                        name: "Craft Stone Pickaxe",
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Stone, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::StonePickaxe, 1)],
                        base_time: 90,
                    }, 0, Some("stone_tools")),

                    (Recipe {
                        name: "Craft Stone Hammer",
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
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Iron, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::IronAxe, 1)],
                        base_time: 100,
                    }, 3, Some("iron_working")),  // Requires experience + iron working tech

                    (Recipe {
                        name: "Craft Iron Pickaxe",
                        inputs: vec![
                            ResourceRequirement::new(ResourceType::Iron, 2),
                            ResourceRequirement::new(ResourceType::Wood, 1),
                        ],
                        outputs: vec![ProductionOutput::new(ItemType::IronPickaxe, 1)],
                        base_time: 100,
                    }, 3, Some("iron_working")),

                    (Recipe {
                        name: "Craft Iron Hammer",
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
                let body_speed = agent.body.movement_speed_multiplier();

                // Get transport speed multiplier (mounts provide speed boost!)
                let transport_speed = agent.transport.effective_speed_modifier();

                // Combined movement speed (body health * transport bonus)
                let movement_speed = body_speed * transport_speed;

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
                    "Agent {} moved from ({}, {}) to ({}, {}) (distance to target: {}, speed: {:.2}x, mounted: {})",
                    agent.id, current_pos.0, current_pos.1, next_x, next_y,
                    distance - 1, movement_speed,
                    if agent.transport.is_mounted() { "yes" } else { "no" }
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

            Action::Store { item_type, amount } => {
                use crate::agents::storage_integration::{
                    id_to_item_type, take_from_agent_inventory, add_to_agent_inventory,
                    count_in_agent_inventory
                };

                let agent = &mut self.population.agents[agent_index];

                // Try to convert string item_type to ItemType
                if let Some(item) = id_to_item_type(item_type) {
                    let available = count_in_agent_inventory(&agent.inventory, item);

                    if available == 0 {
                        return ActionResult::failure(format!(
                            "No {} in inventory to store", item_type
                        ));
                    }

                    // Determine how much to deposit based on storage preferences
                    let deposit_amount = (*amount).min(available);

                    // Remove from agent inventory
                    let (success, removed) = take_from_agent_inventory(
                        &mut agent.inventory,
                        item,
                        deposit_amount,
                    );

                    if success && removed > 0 {
                        // Add to world storehouse
                        if let Some(existing) = self.world.storehouse_inventory.items.get_mut(&item) {
                            existing.quantity += removed;
                        } else {
                            self.world.storehouse_inventory.items.insert(
                                item,
                                crate::world::inventory::Item {
                                    item_type: item,
                                    quantity: removed,
                                    durability: 100,
                                    max_durability: 100,
                                },
                            );
                        }

                        debug!(
                            "Agent {} deposited {} {} to storehouse (storehouse now has {})",
                            agent.id,
                            removed,
                            item_type,
                            self.world.storehouse_inventory.items.get(&item)
                                .map(|i| i.quantity)
                                .unwrap_or(0)
                        );

                        ActionResult::success()
                            .with_drive_change(DriveType::Preparedness, -0.15)
                            .with_energy_cost(5.0)
                            .with_message(format!(
                                "Deposited {} {} to storehouse", removed, item_type
                            ))
                    } else {
                        ActionResult::failure(format!(
                            "Failed to remove {} from inventory", item_type
                        ))
                    }
                } else {
                    ActionResult::failure(format!(
                        "Unknown item type: {}", item_type
                    ))
                }
            },

            Action::Retrieve { item_type, amount } => {
                use crate::agents::storage_integration::{
                    id_to_item_type, add_to_agent_inventory, count_in_agent_inventory
                };

                let agent = &mut self.population.agents[agent_index];

                // Try to convert string item_type to ItemType
                if let Some(item) = id_to_item_type(item_type) {
                    // Check storehouse inventory
                    let storehouse_available = self.world.storehouse_inventory.items
                        .get(&item)
                        .map(|i| i.quantity)
                        .unwrap_or(0);

                    if storehouse_available == 0 {
                        return ActionResult::failure(format!(
                            "Storehouse has no {} available", item_type
                        ));
                    }

                    // Determine how much to retrieve
                    let retrieve_amount = (*amount).min(storehouse_available);

                    // Try to add to agent inventory
                    let (success, added) = add_to_agent_inventory(
                        &mut agent.inventory,
                        item,
                        retrieve_amount,
                    );

                    if added > 0 {
                        // Remove from world storehouse
                        if let Some(existing) = self.world.storehouse_inventory.items.get_mut(&item) {
                            existing.quantity -= added;
                            if existing.quantity == 0 {
                                self.world.storehouse_inventory.items.remove(&item);
                            }
                        }

                        debug!(
                            "Agent {} retrieved {} {} from storehouse (storehouse now has {})",
                            agent.id,
                            added,
                            item_type,
                            self.world.storehouse_inventory.items.get(&item)
                                .map(|i| i.quantity)
                                .unwrap_or(0)
                        );

                        let message = if added < retrieve_amount {
                            format!(
                                "Retrieved {} {} from storehouse (inventory full, couldn't take all {})",
                                added, item_type, retrieve_amount
                            )
                        } else {
                            format!("Retrieved {} {} from storehouse", added, item_type)
                        };

                        ActionResult::success()
                            .with_drive_change(DriveType::Preparedness, -0.1)
                            .with_energy_cost(5.0)
                            .with_message(message)
                    } else {
                        ActionResult::failure(format!(
                            "Inventory full, cannot retrieve {}", item_type
                        ))
                    }
                } else {
                    ActionResult::failure(format!(
                        "Unknown item type: {}", item_type
                    ))
                }
            },

            Action::Hunt { animal_id, weapon } => {
                // Get species data first (clone to avoid borrow issues)
                let species = {
                    if let Some(animal) = self.world.animals.get(animal_id) {
                        if !animal.is_alive() {
                            return ActionResult::failure("Animal is already dead".to_string());
                        }
                        if animal.is_domesticated {
                            return ActionResult::failure("Cannot hunt domesticated animals".to_string());
                        }

                        let species_id = animal.species_id.clone();
                        match self.world.animals.get_species(&species_id) {
                            Some(s) => s.clone(),
                            None => return ActionResult::failure("Unknown animal species".to_string()),
                        }
                    } else {
                        return ActionResult::failure("Animal not found".to_string());
                    }
                };

                // Now get mutable reference to animal
                if let Some(animal) = self.world.animals.get_mut(animal_id) {

                    // Calculate success based on agent skill, weapon, and mount
                    let agent = &self.population.agents[agent_index];
                    let hunting_skill = agent.skills.get_skill_if_exists(crate::agents::skills::SkillType::MeleeCombat)
                        .map(|s| s.level)
                        .unwrap_or(-5);
                    let weapon_bonus = if weapon.is_some() { 0.2 } else { 0.0 };

                    // Get mounted combat bonus (hunting from horseback is advantageous!)
                    let mount_bonus = agent.transport.mounted_combat_bonus();

                    let success_prob = (0.5 + (hunting_skill as f32 * 0.05) + weapon_bonus + mount_bonus).min(0.95_f32);

                    if rng.gen_bool(success_prob as f64) {
                        // Successful hunt - damage the animal
                        // Base damage is 70% of max health, modified by mount bonus
                        let base_damage = species.health * 0.7;
                        let combat_multiplier = 1.0 + mount_bonus;
                        let damage = base_damage * combat_multiplier;
                        animal.take_damage(damage);

                        // If killed, get drops
                        let mut items_gained = Vec::new();
                        if !animal.is_alive() {
                            for drop in &species.drops {
                                if rng.gen_bool(drop.drop_chance as f64) {
                                    let quantity = rng.gen_range(drop.min_quantity..=drop.max_quantity);
                                    items_gained.push(crate::environment::ItemStack {
                                        material_id: drop.material_id.clone(),
                                        quantity,
                                    });
                                }
                            }

                            // Add items to agent inventory
                            let agent = &mut self.population.agents[agent_index];
                            for item_stack in &items_gained {
                                use crate::agents::InventoryItem;
                                let item = InventoryItem::new_with_weight(
                                    item_stack.material_id.clone(),
                                    item_stack.quantity,
                                    2.0, // Default weight for animal drops
                                );
                                agent.inventory.add_item(item);
                            }

                            // Increase hunting skill
                            let agent = &mut self.population.agents[agent_index];
                            agent.skills.gain_experience(crate::agents::skills::SkillType::MeleeCombat, 3);

                            let mut result = ActionResult::success()
                                .with_drive_change(DriveType::Hunger, -0.4)
                                .with_energy_cost(20.0)
                                .with_experience(5.0)
                                .with_message(format!("Successfully hunted {} and obtained materials", species.name));

                            // Add all items gained
                            for item in items_gained {
                                result = result.with_item_gained(item);
                            }
                            result
                        } else {
                            ActionResult::success()
                                .with_drive_change(DriveType::Hunger, -0.1)
                                .with_energy_cost(15.0)
                                .with_message(format!("Wounded {} but it escaped", species.name))
                        }
                    } else {
                        ActionResult::failure(format!("{} escaped", species.name))
                            .with_energy_cost(10.0)
                    }
                } else {
                    ActionResult::failure("Animal not found".to_string())
                }
            },

            Action::Tame { animal_id, food_type } => {
                // Get species data first (clone to avoid borrow issues)
                let species = {
                    if let Some(animal) = self.world.animals.get(animal_id) {
                        if !animal.is_alive() {
                            return ActionResult::failure("Animal is dead".to_string());
                        }
                        if animal.is_domesticated {
                            return ActionResult::failure("Animal is already domesticated".to_string());
                        }

                        let species_id = animal.species_id.clone();
                        match self.world.animals.get_species(&species_id) {
                            Some(s) => s.clone(),
                            None => return ActionResult::failure("Unknown animal species".to_string()),
                        }
                    } else {
                        return ActionResult::failure("Animal not found".to_string());
                    }
                };

                if !species.can_domesticate {
                    return ActionResult::failure(format!("{} cannot be domesticated", species.name));
                }

                // Calculate taming progress based on food and agent relationship skills (using Farming)
                let agent = &self.population.agents[agent_index];
                let social_skill = agent.skills.get_skill_if_exists(crate::agents::skills::SkillType::Farming)
                    .map(|s| s.level)
                    .unwrap_or(-5);
                let taming_bonus = if food_type.is_some() { 0.15 } else { 0.05 };
                let taming_progress = 0.1 + (social_skill as f32 * 0.02) + taming_bonus;

                // Now get mutable reference to animal
                if let Some(animal) = self.world.animals.get_mut(animal_id) {
                    animal.tame(taming_progress);

                    if animal.is_domesticated {
                        // Successfully domesticated
                        animal.owner_id = Some(agent.id);

                        // Create Transport for the tamed animal (if suitable)
                        let transport_type = match species.name.as_str() {
                            "Cow" => Some(crate::agents::transport::TransportType::OxCart),
                            "Sheep" => Some(crate::agents::transport::TransportType::PackDonkey),
                            "Goat" => Some(crate::agents::transport::TransportType::PackDonkey),
                            // Rabbit, Chicken, and Wild Boar are too small or unsuitable for transport
                            _ => None,
                        };

                        // Increase social skill
                        let agent = &mut self.population.agents[agent_index];
                        agent.skills.gain_experience(crate::agents::skills::SkillType::Farming, 2);

                        // Add transport to agent's inventory if applicable
                        if let Some(t_type) = transport_type {
                            let transport = crate::agents::transport::Transport::with_animal(t_type, *animal_id);
                            agent.transport.add_transport(transport);
                        }

                        ActionResult::success()
                            .with_drive_change(DriveType::Utility, -0.3)
                            .with_energy_cost(10.0)
                            .with_experience(10.0)
                            .with_message(format!("Successfully tamed {}!", species.name))
                    } else {
                        ActionResult::success()
                            .with_drive_change(DriveType::Utility, -0.1)
                            .with_energy_cost(8.0)
                            .with_message(format!("Made progress taming {} ({:.0}%)", species.name, animal.tame_level * 100.0))
                    }
                } else {
                    ActionResult::failure("Animal not found".to_string())
                }
            },

            Action::CollectAnimalProduct { animal_id } => {
                // Get species data first (clone to avoid borrow issues)
                let species = {
                    if let Some(animal) = self.world.animals.get(animal_id) {
                        if !animal.is_alive() {
                            return ActionResult::failure("Animal is dead".to_string());
                        }
                        if !animal.is_domesticated {
                            return ActionResult::failure("Can only collect from domesticated animals".to_string());
                        }
                        if !animal.is_mature() {
                            return ActionResult::failure("Animal is not yet mature enough to produce".to_string());
                        }

                        let species_id = animal.species_id.clone();
                        match self.world.animals.get_species(&species_id) {
                            Some(s) => s.clone(),
                            None => return ActionResult::failure("Unknown animal species".to_string()),
                        }
                    } else {
                        return ActionResult::failure("Animal not found".to_string());
                    }
                };

                if species.living_products.is_empty() {
                    return ActionResult::failure(format!("{} does not produce any products", species.name));
                }

                // Now get mutable reference to animal
                if let Some(animal) = self.world.animals.get_mut(animal_id) {
                    // Check which products are ready
                    let mut collected_products = Vec::new();
                    for product in &species.living_products {
                        if let Some(timer) = animal.product_timers.get(&product.material_id) {
                            if *timer == 0 {
                                // Product is ready
                                collected_products.push(crate::environment::ItemStack {
                                    material_id: product.material_id.clone(),
                                    quantity: product.quantity,
                                });

                                // Reset timer
                                animal.product_timers.insert(product.material_id.clone(), product.production_time);
                            }
                        }
                    }

                    if !collected_products.is_empty() {
                        // Add to agent inventory
                        let agent = &mut self.population.agents[agent_index];
                        for item_stack in &collected_products {
                            use crate::agents::InventoryItem;
                            let item = InventoryItem::new_with_weight(
                                item_stack.material_id.clone(),
                                item_stack.quantity,
                                1.0, // Animal products are generally light
                            );
                            agent.inventory.add_item(item);
                        }

                        // Practice industry skill
                        let agent = &mut self.population.agents[agent_index];
                        agent.skills.gain_experience(crate::agents::skills::SkillType::Mining, 1);

                        let products_str = collected_products.iter()
                            .map(|p| format!("{} {}", p.quantity, p.material_id))
                            .collect::<Vec<_>>()
                            .join(", ");

                        let mut result = ActionResult::success()
                            .with_drive_change(DriveType::Industry, -0.2)
                            .with_energy_cost(5.0)
                            .with_message(format!("Collected {} from {}", products_str, species.name));

                        // Add all collected products
                        for product in collected_products {
                            result = result.with_item_gained(product);
                        }
                        result
                    } else {
                        ActionResult::failure("No products ready for collection yet".to_string())
                    }
                } else {
                    ActionResult::failure("Animal not found".to_string())
                }
            },

            Action::HarvestPlant { plant_id } => {
                // Get species data first (clone to avoid borrow issues)
                let species = {
                    if let Some(plant) = self.world.plants.get(plant_id) {
                        if !plant.is_harvestable {
                            return ActionResult::failure("Plant is not ready for harvest".to_string());
                        }
                        if plant.has_been_harvested && !plant.is_cultivated {
                            return ActionResult::failure("Plant has already been harvested".to_string());
                        }

                        let species_id = plant.species_id.clone();
                        match self.world.plants.get_species(&species_id) {
                            Some(s) => s.clone(),
                            None => return ActionResult::failure("Unknown plant species".to_string()),
                        }
                    } else {
                        return ActionResult::failure("Plant not found".to_string());
                    }
                };

                // Now get mutable reference to plant
                if let Some(plant) = self.world.plants.get_mut(plant_id) {

                    // Harvest the plant
                    let drops = plant.harvest(&species);

                    if !drops.is_empty() {
                        let mut items_gained = Vec::new();

                        // Generate items from drops
                        for drop in &drops {
                            let quantity = rng.gen_range(drop.min_quantity..=drop.max_quantity);
                            items_gained.push(crate::environment::ItemStack {
                                material_id: drop.material_id.clone(),
                                quantity,
                            });
                        }

                        // Add to agent inventory
                        let agent = &mut self.population.agents[agent_index];
                        for item_stack in &items_gained {
                            use crate::agents::InventoryItem;
                            let item = InventoryItem::new_with_weight(
                                item_stack.material_id.clone(),
                                item_stack.quantity,
                                1.5, // Plant materials weight
                            );
                            agent.inventory.add_item(item);
                        }

                        // Practice farming skill if cultivated, gathering otherwise
                        let agent = &mut self.population.agents[agent_index];
                        if plant.is_cultivated {
                            agent.skills.gain_experience(crate::agents::skills::SkillType::Farming, 2);
                        } else {
                            agent.skills.gain_experience(crate::agents::skills::SkillType::Mining, 2);
                        }

                        let items_str = items_gained.iter()
                            .map(|i| format!("{} {}", i.quantity, i.material_id))
                            .collect::<Vec<_>>()
                            .join(", ");

                        let mut result = ActionResult::success()
                            .with_drive_change(DriveType::Industry, -0.2)
                            .with_energy_cost(8.0)
                            .with_experience(3.0)
                            .with_message(format!("Harvested {} from {}", items_str, species.name));

                        // Add all harvested items
                        for item in items_gained {
                            result = result.with_item_gained(item);
                        }
                        result
                    } else {
                        ActionResult::failure("Plant yielded nothing".to_string())
                    }
                } else {
                    ActionResult::failure("Plant not found".to_string())
                }
            },

            Action::SeekShelter => {
                // Find nearest shelter (completed building or forest)
                let agent_tuple_pos = self.population.agents[agent_index].state.position;
                let agent_pos = crate::world::Position::new(agent_tuple_pos.0, agent_tuple_pos.1);

                // Check if already in shelter
                let in_building = self.world.buildings.iter().any(|b| {
                    b.position == agent_pos && b.is_completed()
                });

                let in_forest = self.world.grid.get_tile(&agent_pos)
                    .map(|t| matches!(t.terrain.terrain_type, crate::world::TerrainType::Forest))
                    .unwrap_or(false);

                if in_building || in_forest {
                    // Already in shelter - recover from exposure
                    let agent = &mut self.population.agents[agent_index];
                    agent.exposure_status.recover(0.05);

                    return ActionResult::success()
                        .with_drive_change(DriveType::Safety, -0.3)
                        .with_energy_cost(0.0)
                        .with_message(format!(
                            "Taking shelter (exposure: {:.2})",
                            agent.exposure_status.exposure_damage
                        ));
                }

                // Find nearest shelter
                let mut nearest_shelter: Option<crate::world::Position> = None;
                let mut min_distance = u32::MAX;

                // Check buildings
                for building in &self.world.buildings {
                    if building.is_completed() {
                        let dist = agent_pos.distance_to(&building.position);
                        if dist < min_distance {
                            min_distance = dist;
                            nearest_shelter = Some(building.position);
                        }
                    }
                }

                // Check for forest tiles (within reasonable search radius)
                for dx in -5..=5 {
                    for dy in -5..=5 {
                        let check_pos = crate::world::Position::new(
                            agent_pos.x + dx,
                            agent_pos.y + dy
                        );

                        if let Some(tile) = self.world.grid.get_tile(&check_pos) {
                            if matches!(tile.terrain.terrain_type, crate::world::TerrainType::Forest) {
                                let dist = agent_pos.distance_to(&check_pos);
                                if dist < min_distance {
                                    min_distance = dist;
                                    nearest_shelter = Some(check_pos);
                                }
                            }
                        }
                    }
                }

                // Move towards nearest shelter
                if let Some(shelter_pos) = nearest_shelter {
                    let agent = &mut self.population.agents[agent_index];
                    let dx = (shelter_pos.x - agent_pos.x).signum();
                    let dy = (shelter_pos.y - agent_pos.y).signum();
                    let new_pos = crate::world::Position::new(
                        agent_pos.x + dx,
                        agent_pos.y + dy
                    );

                    // Check if path is walkable
                    if self.world.grid.get_tile(&new_pos).map(|t| t.terrain.is_walkable()).unwrap_or(false) {
                        agent.state.position = (new_pos.x, new_pos.y, 0);

                        ActionResult::success()
                            .with_drive_change(DriveType::Safety, -0.1)
                            .with_energy_cost(5.0)
                            .with_message(format!(
                                "Moving towards shelter at ({}, {})",
                                shelter_pos.x, shelter_pos.y
                            ))
                    } else {
                        ActionResult::failure("Path to shelter blocked".to_string())
                    }
                } else {
                    ActionResult::failure("No shelter found nearby".to_string())
                }
            },

            Action::Socialize { target_agent_id } => {
                use crate::agents::social_interactions::{
                    SocialInteractionType, ConversationTopic, HelpType,
                    calculate_relationship_change, calculate_social_satisfaction,
                    should_greet, select_conversation_topic, calculate_gift_value, would_accept_gift
                };
                use crate::agents::traits::Trait;

                // Find the target agent
                let target_index = self.population.agents.iter().position(|a| a.id == *target_agent_id);
                if target_index.is_none() {
                    return ActionResult::failure("Target agent not found".to_string());
                }
                let target_index = target_index.unwrap();

                // Don't socialize with self
                if target_index == agent_index {
                    return ActionResult::failure("Cannot socialize with self".to_string());
                }

                // Get relationship data (clone to avoid borrow issues)
                let initiator_traits: Vec<Trait> = self.population.agents[agent_index]
                    .traits.get_traits().iter().copied().collect();
                let recipient_traits: Vec<Trait> = self.population.agents[target_index]
                    .traits.get_traits().iter().copied().collect();

                // Get or create relationship
                let current_tick = self.current_tick;
                let initiator_agent = &mut self.population.agents[agent_index];
                let relationship = initiator_agent.social_network
                    .get_or_create_relationship(*target_agent_id, current_tick);

                let current_relationship = relationship.relationship_level.clone();
                let current_trust = relationship.trust_level.clone();
                let last_interaction_tick = relationship.last_interaction_tick;

                // Determine interaction type based on relationship and context
                let interaction_type = if should_greet(last_interaction_tick, current_tick, &current_relationship) {
                    // Greet if haven't interacted in a while
                    SocialInteractionType::Greet
                } else {
                    // Choose conversation or other interaction based on relationship
                    let choice = rng.gen_range(0..100);

                    match &current_relationship {
                        crate::agents::relationships::RelationshipLevel::Loves(_) => {
                            // Close relationships: more variety
                            if choice < 40 {
                                let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                                SocialInteractionType::Converse { topic }
                            } else if choice < 60 {
                                SocialInteractionType::ShareMeal
                            } else if choice < 75 {
                                SocialInteractionType::Compliment
                            } else if choice < 90 {
                                SocialInteractionType::OfferHelp {
                                    help_type: HelpType::General,
                                }
                            } else {
                                // Try to give a gift if we have something
                                let initiator = &self.population.agents[agent_index];
                                if let Some((item_id, item)) = initiator.inventory.get_all_items().iter().next() {
                                    if item.quantity > 1 {
                                        // Map item_id string to ItemType
                                        let item_type = match item_id.to_lowercase().as_str() {
                                            "wood" => crate::world::ItemType::Wood,
                                            "stone" => crate::world::ItemType::Stone,
                                            "iron" => crate::world::ItemType::Iron,
                                            "food" => crate::world::ItemType::Food,
                                            "bread" => crate::world::ItemType::Bread,
                                            _ => crate::world::ItemType::Wood, // Default
                                        };
                                        SocialInteractionType::GiveGift {
                                            item_type,
                                            quantity: 1,
                                        }
                                    } else {
                                        let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                                        SocialInteractionType::Converse { topic }
                                    }
                                } else {
                                    let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                                    SocialInteractionType::Converse { topic }
                                }
                            }
                        }
                        crate::agents::relationships::RelationshipLevel::Likes(_) => {
                            // Friends: mostly conversation and help
                            if choice < 60 {
                                let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                                SocialInteractionType::Converse { topic }
                            } else if choice < 80 {
                                SocialInteractionType::Compliment
                            } else {
                                SocialInteractionType::OfferHelp {
                                    help_type: HelpType::General,
                                }
                            }
                        }
                        _ => {
                            // Neutral or negative: stick to safe interactions
                            if choice < 80 {
                                let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                                SocialInteractionType::Converse { topic }
                            } else {
                                SocialInteractionType::ThankYou
                            }
                        }
                    }
                };

                // Calculate interaction effects
                let relationship_change = calculate_relationship_change(
                    &interaction_type,
                    &initiator_traits,
                    &recipient_traits,
                    &current_relationship,
                );

                let social_satisfaction = calculate_social_satisfaction(
                    &interaction_type,
                    &initiator_traits,
                    &current_relationship,
                );

                // Handle gift giving specially (may fail if rejected)
                let mut success = true;
                let mut message = String::new();

                match &interaction_type {
                    SocialInteractionType::GiveGift { item_type, quantity } => {
                        // Check if gift would be accepted
                        if would_accept_gift(&current_relationship, &current_trust, &recipient_traits) {
                            // Format item_type as string for inventory operations
                            let item_str = format!("{:?}", item_type).to_lowercase();

                            // Remove from initiator inventory
                            let initiator = &mut self.population.agents[agent_index];
                            if let Some(_removed) = initiator.inventory.remove_item(&item_str, *quantity) {
                                // Add to recipient inventory
                                let recipient = &mut self.population.agents[target_index];
                                let gift_item = crate::agents::InventoryItem::new_with_weight(
                                    item_str.clone(),
                                    *quantity,
                                    2.0, // Default weight
                                );
                                recipient.inventory.add_item(gift_item);

                                let gift_value = calculate_gift_value(item_type, *quantity);
                                message = format!("Gave {} {:?} to agent (value: {:.1})", quantity, item_type, gift_value);
                                success = true;
                            } else {
                                message = "Don't have enough to give as gift".to_string();
                                success = false;
                            }
                        } else {
                            message = "Gift was politely refused".to_string();
                            success = false;
                        }
                    }
                    SocialInteractionType::Greet => {
                        message = format!("Greeted agent (relationship: {:?})", current_relationship);
                    }
                    SocialInteractionType::Converse { topic } => {
                        message = format!("Had conversation about {:?}", topic);
                    }
                    SocialInteractionType::OfferHelp { help_type } => {
                        message = format!("Offered {:?} help", help_type);
                    }
                    SocialInteractionType::ThankYou => {
                        message = "Expressed gratitude".to_string();
                    }
                    SocialInteractionType::Compliment => {
                        message = "Gave a compliment".to_string();
                    }
                    SocialInteractionType::ShareMeal => {
                        message = "Shared a meal together".to_string();
                    }
                }

                // Update initiator's relationship
                let initiator = &mut self.population.agents[agent_index];
                let relationship = initiator.social_network
                    .get_or_create_relationship(*target_agent_id, current_tick);

                if success && relationship_change != 0 {
                    if relationship_change > 0 {
                        relationship.positive_interaction(relationship_change, current_tick);
                    } else {
                        relationship.negative_interaction(relationship_change.abs(), current_tick);
                    }
                }
                relationship.last_interaction_tick = current_tick;
                relationship.total_interactions += 1;

                // Also update target's relationship (reciprocal, but may differ based on their traits)
                let target_relationship_change = calculate_relationship_change(
                    &interaction_type,
                    &recipient_traits,
                    &initiator_traits,
                    &current_relationship,
                );

                // Capture initiator's ID before mutable borrow
                let initiator_id = self.population.agents[agent_index].id;

                let target = &mut self.population.agents[target_index];
                let target_relationship = target.social_network
                    .get_or_create_relationship(initiator_id, current_tick);

                if success && target_relationship_change != 0 {
                    if target_relationship_change > 0 {
                        target_relationship.positive_interaction(target_relationship_change, current_tick);
                    } else {
                        target_relationship.negative_interaction(target_relationship_change.abs(), current_tick);
                    }
                }
                target_relationship.last_interaction_tick = current_tick;
                target_relationship.total_interactions += 1;

                // Calculate target's social satisfaction too
                let target_satisfaction = calculate_social_satisfaction(
                    &interaction_type,
                    &recipient_traits,
                    &current_relationship,
                );

                // Update target's social drive
                let target = &mut self.population.agents[target_index];
                if let Some(social_drive) = target.drives.get_mut(DriveType::Social) {
                    social_drive.decrease(target_satisfaction);
                }

                if success {
                    debug!(
                        "Agent {} socialized with agent {}: {} (relationship change: {:+}, satisfaction: {:.2})",
                        self.population.agents[agent_index].id,
                        target_agent_id,
                        message,
                        relationship_change,
                        social_satisfaction
                    );

                    ActionResult::success()
                        .with_drive_change(DriveType::Social, -social_satisfaction)
                        .with_energy_cost(3.0)
                        .with_message(message)
                } else {
                    ActionResult::failure(message)
                }
            },

            Action::ShareInformation { target_agent_id } => {
                use crate::agents::gossip::{Information, InformationType};
                use crate::agents::traits::Trait;

                // Find the target agent
                let target_index = self.population.agents.iter().position(|a| a.id == *target_agent_id);
                if target_index.is_none() {
                    return ActionResult::failure("Target agent not found".to_string());
                }
                let target_index = target_index.unwrap();

                // Don't share with self
                if target_index == agent_index {
                    return ActionResult::failure("Cannot share information with self".to_string());
                }

                let current_tick = self.current_tick;

                // Capture initiator data before mutable borrows
                let (initiator_id, info_to_share) = {
                    let initiator = &self.population.agents[agent_index];
                    let initiator_traits: Vec<Trait> = initiator.traits.get_traits().iter().copied().collect();
                    let initiator_id = initiator.id;

                    // Select information to share from initiator's knowledge base
                    let info = if !initiator.knowledge.known_information.is_empty() {
                        // Pick a random piece of information from their knowledge
                        let info_list: Vec<_> = initiator.knowledge.known_information.values().collect();
                        let idx = rng.gen_range(0..info_list.len());
                        let original_info = info_list[idx].clone();

                        // Check if initiator would distort information based on traits
                        if let Some(distortion_trait) = initiator.traits.would_distort_info() {
                            // Distort the information
                            original_info.distort(distortion_trait, initiator_id)
                        } else {
                            // Share truthfully
                            original_info
                        }
                    } else {
                        // Generate new information if they don't have any
                        // Share a resource location they might know about
                        let agent_pos = initiator.state.position;
                        Information::new(
                            InformationType::ResourceLocation {
                                resource: "generic".to_string(),
                                location: agent_pos,
                            },
                            initiator_id,
                            true, // Assume they know their current location
                            current_tick as u64,
                        )
                    };

                    (initiator_id, info)
                };

                // Get recipient's traits for belief calculation
                let recipient_traits = self.population.agents[target_index].traits.clone();

                // Share the information with recipient
                let target = &mut self.population.agents[target_index];
                let target_id = target.id;
                target.knowledge.receive_information(
                    info_to_share.clone(),
                    initiator_id,
                    target_id,
                    &recipient_traits,
                    current_tick as u64,
                );

                // Determine message based on information type
                let message = match &info_to_share.info_type {
                    InformationType::ResourceLocation { resource, location } => {
                        format!("Shared knowledge about {} at ({}, {}, {})",
                            resource, location.0, location.1, location.2)
                    }
                    InformationType::Conflict { agent1, agent2 } => {
                        format!("Gossiped about conflict between agents")
                    }
                    InformationType::Death { agent, cause } => {
                        format!("Shared news of death: {}", cause)
                    }
                    InformationType::TechnologyDiscovered { tech } => {
                        format!("Shared discovery of {} technology", tech)
                    }
                    InformationType::Accusation { accused, crime, .. } => {
                        format!("Shared accusation of {}", crime)
                    }
                    _ => "Shared information".to_string(),
                };

                // Distortion affects satisfaction
                let distortion_penalty = if info_to_share.distortion.is_some() { 0.05 } else { 0.0 };
                let social_satisfaction = 0.15 - distortion_penalty;

                debug!(
                    "Agent {} shared information with agent {} (distorted: {}, reliability: {:.2})",
                    initiator_id,
                    target_agent_id,
                    info_to_share.distortion.is_some(),
                    info_to_share.reliability
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Social, -social_satisfaction)
                    .with_energy_cost(2.0)
                    .with_message(message)
            },

            Action::Mate { target_agent_id } => {
                use crate::agents::reproduction::{can_mate, reproduce, MateSelectionCriteria};
                use crate::agents::gossip::{Information, InformationType};

                // Find the target agent
                let target_index = self.population.agents.iter().position(|a| a.id == *target_agent_id);
                if target_index.is_none() {
                    return ActionResult::failure("Target agent not found".to_string());
                }
                let target_index = target_index.unwrap();

                // Don't mate with self
                if target_index == agent_index {
                    return ActionResult::failure("Cannot mate with self".to_string());
                }

                // Check if both agents can mate
                let initiator = &self.population.agents[agent_index];
                let target = &self.population.agents[target_index];
                let criteria = MateSelectionCriteria::default();

                if !can_mate(initiator, target, &criteria) {
                    // Determine specific reason for failure
                    let reason = if !initiator.can_reproduce() {
                        "Initiator cannot reproduce (too young, too old, or pregnant)".to_string()
                    } else if !target.can_reproduce() {
                        "Target cannot reproduce (too young, too old, or pregnant)".to_string()
                    } else if initiator.fertility() < criteria.min_fertility {
                        format!("Initiator fertility too low ({:.2})", initiator.fertility())
                    } else if target.fertility() < criteria.min_fertility {
                        format!("Target fertility too low ({:.2})", target.fertility())
                    } else if target.parent_ids.contains(&initiator.id) || initiator.parent_ids.contains(&target.id) {
                        "Cannot mate with parent/child".to_string()
                    } else {
                        "Agents too far apart for mating".to_string()
                    };

                    return ActionResult::failure(reason);
                }

                // Calculate mating success probability based on relationship
                let initiator_id = initiator.id;
                let target_id = target.id;
                let mut success_probability = 0.5; // Base 50% chance

                // Check relationship - better relationships increase success
                if let Some(relationship) = initiator.social_network.get_relationship(target_id) {
                    match &relationship.relationship_level {
                        crate::agents::relationships::RelationshipLevel::Loves(_) => {
                            success_probability = 0.9; // High success with loved ones
                        }
                        crate::agents::relationships::RelationshipLevel::Likes(_) => {
                            success_probability = 0.7; // Good success with friends
                        }
                        crate::agents::relationships::RelationshipLevel::Neutral(_) => {
                            success_probability = 0.5; // Neutral success
                        }
                        _ => {
                            success_probability = 0.2; // Low success with dislikes/hates
                        }
                    }
                }

                // Attempt mating
                if rng.gen_bool(success_probability as f64) {
                    // Mating successful - create offspring
                    // Clone parent positions before creating offspring to avoid borrow issues
                    let parent1_pos = self.population.agents[agent_index].state.position;
                    let offspring = {
                        let parent1 = &self.population.agents[agent_index];
                        let parent2 = &self.population.agents[target_index];
                        let current_tick = self.current_tick;
                        reproduce(parent1, parent2, current_tick)
                    };
                    let offspring_id = offspring.id;

                    // Add offspring to population
                    self.population.agents.push(offspring);

                    debug!(
                        "Agent {} and agent {} successfully mated! Offspring: {}",
                        initiator_id, target_id, offspring_id
                    );

                    // Generate gossip about the birth
                    let current_tick = self.current_tick;
                    let birth_info = Information::new(
                        InformationType::Childbirth {
                            agent: initiator_id,
                            child: offspring_id,
                        },
                        initiator_id,
                        true, // This is true information
                        current_tick as u64,
                    );

                    // Share birth information with nearby agents
                    for other_agent in &mut self.population.agents {
                        if other_agent.id != initiator_id && other_agent.id != target_id && other_agent.id != offspring_id {
                            // Calculate distance
                            let distance = {
                                let dx = (other_agent.state.position.0 - parent1_pos.0) as f32;
                                let dy = (other_agent.state.position.1 - parent1_pos.1) as f32;
                                (dx * dx + dy * dy).sqrt()
                            };

                            // Share with agents within 20 tiles
                            if distance <= 20.0 {
                                other_agent.knowledge.receive_information(
                                    birth_info.clone(),
                                    initiator_id,
                                    other_agent.id,
                                    &other_agent.traits,
                                    current_tick as u64,
                                );
                            }
                        }
                    }

                    // Update relationships - parents bond with child
                    // Update reproduction drives for both parents
                    let agent = &mut self.population.agents[agent_index];
                    if let Some(repro_drive) = agent.drives.get_mut(DriveType::Reproduction) {
                        repro_drive.decrease(0.8); // Significantly reduce reproduction drive
                    }

                    let target = &mut self.population.agents[target_index];
                    if let Some(repro_drive) = target.drives.get_mut(DriveType::Reproduction) {
                        repro_drive.decrease(0.8); // Significantly reduce reproduction drive
                    }

                    ActionResult::success()
                        .with_drive_change(DriveType::Reproduction, -0.8)
                        .with_energy_cost(15.0) // Mating is energy-intensive
                        .with_message(format!("Successfully mated with agent, offspring: {}", offspring_id))
                } else {
                    // Mating attempt rejected
                    debug!(
                        "Agent {} mating attempt with agent {} was rejected",
                        initiator_id, target_id
                    );

                    ActionResult::failure("Mating attempt rejected by partner".to_string())
                }
            },

            Action::Mount { transport_id } => {
                let agent = &mut self.population.agents[agent_index];

                // Try to mount the transport
                match agent.transport.mount_transport(transport_id) {
                    Ok(()) => {
                        debug!("Agent {} mounted transport {}", agent.id, transport_id);

                        ActionResult::success()
                            .with_drive_change(DriveType::Utility, -0.1)
                            .with_energy_cost(2.0)
                            .with_message("Successfully mounted".to_string())
                    }
                    Err(err) => ActionResult::failure(err),
                }
            },

            Action::Dismount => {
                let agent = &mut self.population.agents[agent_index];

                if !agent.transport.is_mounted() {
                    return ActionResult::failure("Not currently mounted".to_string());
                }

                agent.transport.dismount_current();
                debug!("Agent {} dismounted", agent.id);

                ActionResult::success()
                    .with_energy_cost(1.0)
                    .with_message("Dismounted from transport".to_string())
            },

            Action::Wait => {
                // Wait/rest action - restores energy, calms emotions
                let agent = &mut self.population.agents[agent_index];

                // Restore a small amount of energy (resting)
                let energy_restored = rng.gen_range(3.0..6.0);
                agent.state.energy = (agent.state.energy + energy_restored).min(100.0);

                // Reduce negative emotions slightly (calming effect)
                agent.emotions.anger = (agent.emotions.anger - 0.02).max(0.0);
                agent.emotions.fear = (agent.emotions.fear - 0.02).max(0.0);

                debug!(
                    "Agent {} waited, restored {:.1} energy, reduced stress",
                    agent.id, energy_restored
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Rest, -0.15) // Satisfies rest drive
                    .with_message(format!("Rested and recovered {:.1} energy", energy_restored))
            },

            Action::Explore { direction } => {
                // Exploration action - move and discover new areas
                let agent = &mut self.population.agents[agent_index];
                let agent_id = agent.id;
                let current_pos = agent.state.position;

                // Calculate target position in exploration direction
                let target_x = current_pos.0 + direction.0;
                let target_y = current_pos.1 + direction.1;
                let target_z = current_pos.2 + direction.2;
                let target_pos = (target_x, target_y, target_z);

                // Move agent to new position
                agent.state.position = target_pos;

                // Mark tiles as explored in a radius around new position
                let mut newly_explored_count = 0;
                let exploration_radius = 3; // Can see 3 tiles in each direction

                for dx in -exploration_radius..=exploration_radius {
                    for dy in -exploration_radius..=exploration_radius {
                        let explore_pos = crate::world::Position::new(
                            target_x + dx,
                            target_y + dy,
                        );

                        if agent.exploration_knowledge.explore_tile(explore_pos, self.current_tick) {
                            newly_explored_count += 1;
                        }
                    }
                }

                // Discover nearby resources (within exploration radius)
                let mut discoveries = Vec::new();
                for resource in &self.world.resources {
                    let resource_pos = crate::world::Position::new(
                        resource.position.x,
                        resource.position.y,
                    );
                    let dx = (resource_pos.x - target_x).abs();
                    let dy = (resource_pos.y - target_y).abs();

                    if dx <= exploration_radius && dy <= exploration_radius {
                        if agent.exploration_knowledge.discover_resource(
                            resource_pos,
                            resource.resource_type,
                            self.current_tick,
                        ) {
                            discoveries.push(format!("{:?}", resource.resource_type));
                        }
                    }
                }

                let agent = &mut self.population.agents[agent_index];

                // Construct message about exploration results
                let mut message = format!(
                    "Explored new area, discovered {} tiles",
                    newly_explored_count
                );
                if !discoveries.is_empty() {
                    message.push_str(&format!(", found: {}", discoveries.join(", ")));
                }

                debug!(
                    "Agent {} explored to ({}, {}, {}), discovered {} new tiles",
                    agent_id, target_x, target_y, target_z, newly_explored_count
                );

                // Exploration is rewarding
                let curiosity_satisfaction = if newly_explored_count > 0 { 0.3 } else { 0.1 };

                ActionResult::success()
                    .with_drive_change(DriveType::Curiosity, -curiosity_satisfaction)
                    .with_energy_cost(5.0) // Exploration takes energy
                    .with_message(message)
            },

            // For other actions, use simplified success/failure
            _ => {
                // Base success probability
                let mut success_probability = 0.7;

                // Check if agent has learned this behavior through observation
                // Learned behaviors boost success probability
                if let Some(broadcast_type) = Self::map_action_to_broadcast_type(action) {
                    let agent = &self.population.agents[agent_index];
                    let adopted_behaviors = agent.observational_learning.get_adopted_behaviors();

                    // Check if this action type has been adopted from anyone
                    for (_, action_type, confidence) in adopted_behaviors {
                        if action_type == broadcast_type {
                            // Boost success probability based on confidence in learned behavior
                            // Confidence ranges 0.0 to 1.0, provides up to +0.25 boost
                            let learning_boost = confidence * 0.25;
                            success_probability = (success_probability + learning_boost).min(0.95);
                            debug!(
                                "Agent {} has learned {:?} (confidence: {:.2}), success probability: {:.2}",
                                agent.id, action_type, confidence, success_probability
                            );
                            break;
                        }
                    }
                }

                if rng.gen_bool(success_probability as f64) {
                    let satisfaction = match action {
                        Action::Craft { .. } => 0.2,
                        Action::Store { .. } => 0.1,
                        Action::Socialize { .. } => 0.2,
                        Action::ShareInformation { .. } => 0.15, // Handled separately above
                        Action::Mate { .. } => 0.0, // Handled separately above
                        Action::Mount { .. } => 0.0, // Handled separately above
                        Action::Dismount => 0.0, // Handled separately above
                        Action::Wait => 0.0, // Handled separately above
                        Action::Explore { .. } => 0.0, // Handled separately above
                        Action::Hunt { .. } => 0.3,
                        Action::Tame { .. } => 0.25,
                        Action::CollectAnimalProduct { .. } => 0.15,
                        Action::HarvestPlant { .. } => 0.15,
                        Action::SeekShelter => 0.0, // Handled separately above
                        Action::Move { .. } => 0.05,
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

    /// Update exposure damage for all agents based on weather and environmental conditions
    fn update_agent_exposure(&mut self) {
        let weather = self.world.climate.weather.clone();
        let time_of_day = self.world.climate.calendar.time_of_day;

        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Get environmental temperature at agent's position
            let agent_pos = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
            let terrain_type = self.world.grid.get_tile(&agent_pos)
                .map(|t| t.terrain.terrain_type)
                .unwrap_or(crate::world::TerrainType::Plains);

            let environmental_temp = self.world.climate.get_temperature(agent_pos, terrain_type);

            // Check if agent has shelter
            // Agent has shelter if they're in a completed building
            let has_shelter = self.world.buildings.iter().any(|b| {
                b.position == agent_pos && b.is_completed()
            }) || matches!(terrain_type, crate::world::TerrainType::Forest); // Forest provides partial shelter

            // Check if agent has water access (simplified: check inventory for water containers)
            let has_water_access = agent.inventory.get_item("waterskin")
                .map(|item| item.fill_percentage() > 0.1)
                .unwrap_or(false);

            // Update exposure and apply damage
            let damage = agent.update_exposure(
                &weather,
                environmental_temp,
                has_shelter,
                has_water_access,
                time_of_day,
            );

            // Log critical exposure events
            if damage > 0.05 {
                debug!(
                    "Agent {} taking exposure damage: {:.3} (exposures: {:?})",
                    agent.id, damage, agent.exposure_status.active_exposures
                );
            }

            // If agent is in critical exposure condition, they may die
            if agent.exposure_status.is_critical() && agent.state.health < 20.0 {
                warn!(
                    "Agent {} in critical exposure condition! Health: {:.1}, Exposure: {:.2}",
                    agent.id, agent.state.health, agent.exposure_status.exposure_damage
                );
            }
        }
    }

    /// Save simulation state to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        // Create serializable state
        let state = SerializableSimulationState {
            world: self.world.clone(),
            agents: self.population.agents.clone(),
            current_tick: self.current_tick,
            population_stats: PopulationStatsSnapshot {
                total_births: self.population.stats.total_births,
                total_deaths: self.population.stats.total_deaths,
                total_abandonments: self.population.stats.total_abandonments,
            },
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Write to file
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;

        info!("Simulation saved at tick {}", self.current_tick);
        Ok(())
    }

    /// Load simulation state from a file
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        // Read file
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Deserialize from JSON
        let state: SerializableSimulationState = serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Reconstruct Population
        let mut population = Population::new();
        population.agents = state.agents;
        population.current_tick = state.current_tick;
        population.stats.total_births = state.population_stats.total_births;
        population.stats.total_deaths = state.population_stats.total_deaths;
        population.stats.total_abandonments = state.population_stats.total_abandonments;

        // Reconstruct Simulation
        let sim = Simulation {
            world: state.world,
            population,
            current_tick: state.current_tick,
            renderer: None,
        };

        info!("Simulation loaded from tick {}", sim.current_tick);
        Ok(sim)
    }
}


#[cfg(test)]
mod tests;

