// src/analytics/mod.rs
//! Analytics, data logging, and emergence detection.

use crate::world::World;
use crate::agents::Population;
use crate::world::spatial_planning::{SpatialPlanner, PlacementStrategy, PlacementCriteria};

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

// New observation interface modules
pub mod events;
pub mod replay;
pub mod storage;
pub mod web_api;

pub use metrics::{SimulationMetrics, TickSnapshot, PopulationSnapshot, DriveSnapshot, EmotionSnapshot};
pub use emergence::{
    EmergenceDetector, EmergentPattern, PatternType,
    DetectionThresholds, TrainingSample, CalibrationResult,
    TrendDirection, PatternPrediction,
};
pub use export::{DataExporter, ExportFormat};
pub use performance::{PerformanceMonitor, PerformanceSnapshot};

// Export new modules
pub use events::{EventBus, EventData, EventType, EventFilter, EventEmitter, SubscriptionId, EventValue};
pub use replay::{SessionRecorder, SessionPlayer, StateSnapshot, AgentSnapshot, WorldSnapshot, RecordingConfig};
pub use storage::{StorageManager, StorageConfig, TimeSeriesStore, DocumentStore, DataPoint};
pub use web_api::{ApiServer, ApiConfig, SimulationDataProvider, SimulationStatus, PopulationSummary, AgentSummary, AgentDetail};

use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use crate::visualization::AsciiRenderer;
use crate::agents::religious_effects::{
    calculate_religious_effects, total_happiness_modifier, RELIGIOUS_EFFECT_RADIUS,
};
use log::{info, debug, warn};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::fs::{File, self};
use std::io::{Write, Read};

/// Auto-save configuration for checkpointing
#[derive(Debug, Clone)]
pub struct AutoSaveConfig {
    pub enabled: bool,
    pub interval_ticks: u32,
    pub max_checkpoints: usize,
    pub save_directory: PathBuf,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ticks: 100,
            max_checkpoints: 5,
            save_directory: PathBuf::from("./checkpoints"),
        }
    }
}

pub struct Simulation {
    pub world: World,
    pub population: Population,
    pub current_tick: u32,
    pub renderer: Option<AsciiRenderer>,
    autosave_config: Option<AutoSaveConfig>,
    last_autosave_tick: u32,
}

/// Configuration for simulation behavior and limits
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Random seed for deterministic simulations (None = random)
    pub random_seed: Option<i64>,
    /// Maximum number of ticks before simulation auto-stops (None = unlimited)
    pub max_ticks: Option<u32>,
    /// Enable logging output
    pub enable_logging: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// How often to record metrics (every N ticks)
    pub metrics_interval: u32,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Generate a random seed from system time
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64);

        Self {
            random_seed: seed,
            max_ticks: None,
            enable_logging: true,
            enable_metrics: true,
            metrics_interval: 1,
        }
    }
}

impl SimulationConfig {
    /// Set a specific random seed for deterministic simulations
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    /// Set maximum number of ticks
    pub fn with_max_ticks(mut self, max_ticks: u32) -> Self {
        self.max_ticks = Some(max_ticks);
        self
    }

    /// Enable or disable logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Enable or disable metrics collection
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    /// Set metrics collection interval
    pub fn with_metrics_interval(mut self, interval: u32) -> Self {
        self.metrics_interval = interval;
        self
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if let Some(max_ticks) = self.max_ticks {
            if max_ticks == 0 {
                return Err("max_ticks must be greater than 0".to_string());
            }
        }

        if self.metrics_interval == 0 {
            return Err("metrics_interval must be greater than 0".to_string());
        }

        Ok(())
    }
}

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

/// Determines the appropriate placement strategy and criteria for a building type
fn determine_placement_approach(building_type: crate::world::BuildingType) -> (PlacementCriteria, PlacementStrategy) {
    use crate::world::BuildingType;

    match building_type {
        // Residential buildings should cluster near existing settlement
        BuildingType::SmallHouse | BuildingType::MediumHouse | BuildingType::LargeHouse => {
            (PlacementCriteria::NearSettlement, PlacementStrategy::BalancedProximity)
        },

        // Storage buildings should be central to settlement
        BuildingType::Storehouse => {
            (PlacementCriteria::CentralToSettlement, PlacementStrategy::BalancedProximity)
        },

        // Production buildings with specific resource needs
        BuildingType::Farm => {
            (PlacementCriteria::NearSettlement, PlacementStrategy::BalancedProximity)
        },
        BuildingType::Mill => {
            // Needs Farm as prerequisite
            (PlacementCriteria::NearRelatedBuilding, PlacementStrategy::NearResources)
        },
        BuildingType::Bakery => {
            // Needs Mill as prerequisite
            (PlacementCriteria::NearRelatedBuilding, PlacementStrategy::NearResources)
        },
        BuildingType::Workshop => {
            // Needs wood primarily
            (PlacementCriteria::NearResource("wood".to_string()), PlacementStrategy::NearResources)
        },
        BuildingType::Forge => {
            // Needs iron primarily - prioritize iron resources
            (PlacementCriteria::NearResource("iron".to_string()), PlacementStrategy::NearResources)
        },
        BuildingType::Smithy => {
            // Advanced metalworking - needs Forge and iron
            (PlacementCriteria::NearResource("iron".to_string()), PlacementStrategy::NearResources)
        },

        // Default for any other building types
        _ => {
            (PlacementCriteria::NearSettlement, PlacementStrategy::NearestAvailable)
        }
    }
}

impl Simulation {
    pub fn new(world: World, population: Population) -> Self {
        Self {
            world,
            population,
            current_tick: 0,
            renderer: None,
            autosave_config: None,
            last_autosave_tick: 0,
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

        // Collect agent IDs and positions to avoid borrowing issues
        // This also allows us to resolve nil UUIDs in social actions
        let agent_ids: Vec<_> = self.population.agents.iter().map(|a| a.id).collect();
        let agent_positions: Vec<(uuid::Uuid, (i32, i32, i32))> = self.population.agents
            .iter()
            .map(|a| (a.id, a.state.position))
            .collect();

        for agent_id in agent_ids {
            // Find the agent
            let agent_index = self.population.agents.iter().position(|a| a.id == agent_id);
            if agent_index.is_none() {
                continue;
            }
            let agent_index = agent_index.unwrap();

            // Get agent data we need
            // Use happiness-aware drive selection so agents prefer enjoyable work
            // when survival needs are met
            let (drive_type, drive_value, agent_position) = {
                let agent = &self.population.agents[agent_index];
                // Use happiness-aware selection for non-survival situations
                if let Some(selected_drive) = agent.select_drive_with_happiness() {
                    let value = agent.drives.get(selected_drive)
                        .map(|d| d.value)
                        .unwrap_or(0.0);
                    (Some(selected_drive), value, agent.state.position)
                } else if let Some(urgent_drive) = agent.drives.most_urgent() {
                    // Fallback to most urgent if no drive selected
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
                        (crate::core::EmotionType::Happiness, agent.emotions.happiness),
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

                // Check if current plan is still relevant given updated world state
                // This allows agents to abandon plans when goals are already satisfied
                // (e.g., another agent restocked the storehouse)
                {
                    use crate::world::ItemType;
                    use crate::core::GoalWorldState;

                    // Calculate storehouse contents
                    let food_types = vec![
                        ItemType::Food, ItemType::Bread, ItemType::Cheese,
                        ItemType::Meat, ItemType::Fish, ItemType::Honey, ItemType::Ale,
                    ];
                    let resource_types = vec![
                        ItemType::Wood, ItemType::Stone, ItemType::Iron,
                        ItemType::Clay, ItemType::Sand, ItemType::Coal,
                    ];
                    let tool_types = vec![
                        ItemType::WoodenAxe, ItemType::StoneAxe, ItemType::IronAxe,
                        ItemType::WoodenPickaxe, ItemType::StonePickaxe, ItemType::IronPickaxe,
                        ItemType::WoodenHammer, ItemType::StoneHammer, ItemType::IronHammer,
                    ];

                    let storehouse_food: u32 = food_types.iter()
                        .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                        .map(|item| item.quantity)
                        .sum();

                    let storehouse_materials: u32 = resource_types.iter()
                        .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                        .map(|item| item.quantity)
                        .sum();

                    let storehouse_tools: u32 = tool_types.iter()
                        .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                        .map(|item| item.quantity)
                        .sum();

                    // Get agent's personal inventory state
                    let agent = &self.population.agents[agent_index];
                    let personal_food = agent.inventory.get_item("food")
                        .map(|i| i.quantity)
                        .unwrap_or(0);
                    let gathered_resources = agent.inventory.get_item("wood")
                        .map(|i| i.quantity)
                        .unwrap_or(0)
                        + agent.inventory.get_item("stone")
                            .map(|i| i.quantity)
                            .unwrap_or(0);

                    // Check if agent has protection equipment (check for any armor items)
                    let has_protection = agent.inventory.get_all_items().iter()
                        .any(|(item_id, _)| {
                            item_id.contains("armor") ||
                            item_id.contains("Armor") ||
                            item_id.contains("shield")
                        });

                    // Check if agent owns a house by checking actual building ownership
                    let owns_house = self.world.buildings.iter().any(|b| {
                        b.owner == Some(agent_id) &&
                        b.is_completed() &&
                        b.building_type.is_residential()
                    });

                    let world_state = GoalWorldState {
                        storehouse_food,
                        storehouse_materials,
                        storehouse_tools,
                        personal_food,
                        gathered_resources,
                        owns_house,
                        has_protection,
                        ..Default::default()
                    };

                    // Update plan relevance - this will abandon the plan if goal is satisfied
                    let agent = &mut self.population.agents[agent_index];
                    agent.update_plan_relevance(&world_state);
                }

                // Generate action based on priority: emotions > shelter > percepts > plan > goals > drives
                let (action, is_plan_action) = {
                    let agent = &self.population.agents[agent_index];

                    // PRIORITY 0: Check emotional overrides (fear/anger from being attacked)
                    if agent.emotions.should_flee() {
                        // High fear - flee from attacker or danger
                        if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
                            // Find attacker position and flee away from them
                            if let Some(attacker) = self.population.agents.iter().find(|a| a.id == attacker_id) {
                                let attacker_pos = attacker.state.position;
                                let dx = agent_position.0 - attacker_pos.0;
                                let dy = agent_position.1 - attacker_pos.1;
                                let distance = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
                                let flee_distance = 15;
                                let flee_x = agent_position.0 + ((dx as f32 / distance) * flee_distance as f32) as i32;
                                let flee_y = agent_position.1 + ((dy as f32 / distance) * flee_distance as f32) as i32;

                                debug!(
                                    "Agent {} FLEEING from attacker {} (fear={:.2})",
                                    agent_id, attacker_id, agent.emotions.fear
                                );

                                (crate::environment::Action::Move {
                                    target: (flee_x, flee_y, agent_position.2),
                                }, false)
                            } else {
                                // Attacker not found, flee in random direction
                                use rand::Rng;
                                let mut rng = rand::thread_rng();
                                let flee_x = agent_position.0 + rng.gen_range(-15..=15);
                                let flee_y = agent_position.1 + rng.gen_range(-15..=15);
                                (crate::environment::Action::Move {
                                    target: (flee_x, flee_y, agent_position.2),
                                }, false)
                            }
                        } else {
                            // No specific attacker, continue with other priorities
                            // (fear might be from other sources like predators)
                            Self::generate_non_emotional_action(agent, agent_position, &self.population, self.current_tick)
                        }
                    } else if agent.emotions.should_attack() {
                        // High anger, low fear - retaliate against attacker
                        if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
                            debug!(
                                "Agent {} RETALIATING against {} (anger={:.2}, fear={:.2})",
                                agent_id, attacker_id, agent.emotions.anger, agent.emotions.fear
                            );

                            (crate::environment::Action::Attack {
                                target_agent_id: attacker_id,
                                weapon: agent.equipment.get_weapon().map(|w| w.name.clone()),
                            }, false)
                        } else {
                            // Angry but no target, continue with other priorities
                            Self::generate_non_emotional_action(agent, agent_position, &self.population, self.current_tick)
                        }
                    } else {
                        Self::generate_non_emotional_action(agent, agent_position, &self.population, self.current_tick)
                    }
                };

                // Resolve nil UUIDs in social actions to actual nearby agents
                let action = Self::resolve_action_target(
                    action,
                    agent_id,
                    agent_position,
                    &agent_positions,
                );

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

                // Apply trait-based happiness rewards for successful actions
                if action_result.success {
                    agent.apply_trait_action_rewards(&action);
                }

                // Update plan execution state if this was a plan action
                if is_plan_action {
                    if action_result.success {
                        // Successful action - advance to next plan step
                        agent.advance_plan_step(true, agent.plan_step_ticks + 1);
                        debug!(
                            "Agent {} completed plan step, progress: {:?}",
                            agent_id,
                            agent.plan_progress()
                        );
                    } else {
                        // Failed action - increment step ticks and potentially abandon plan
                        agent.tick_plan_step();
                        if !agent.should_execute_plan() {
                            // Plan has timed out or is no longer viable
                            debug!("Agent {} abandoning plan due to failure/timeout", agent_id);
                            agent.abandon_plan();
                        }
                    }
                } else {
                    // Not a plan action - tick the plan step counter anyway
                    // This allows plans to timeout if agent keeps getting interrupted
                    agent.tick_plan_step();
                }

                // Try to create a plan for goals if agent doesn't have one
                // Only do this periodically to avoid constant replanning
                if !agent.has_active_plan() && self.current_tick % 50 == 0 {
                    // Use a default resource/return location (should be enhanced with real world data)
                    let resource_loc = (50, 50, 0);
                    let return_loc = (0, 0, 0);
                    if agent.create_plan_for_goal(resource_loc, return_loc, self.current_tick) {
                        debug!(
                            "Agent {} created new plan: {:?}",
                            agent_id,
                            agent.current_plan_step_description()
                        );
                    }
                }

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

        // Process building production collection (every 50 ticks)
        // Agents near production buildings automatically collect resources
        if self.current_tick % 50 == 0 {
            self.process_building_production_collection();
        }

        // Process building maintenance (every 100 ticks)
        // Generate maintenance tasks for buildings in poor condition
        if self.current_tick % 100 == 0 {
            self.process_building_maintenance();
        }

        // Process information verification and lie detection (every 100 ticks)
        // Agents verify information they've received against their knowledge
        if self.current_tick % 100 == 0 {
            self.process_information_verification();
        }
        // Process pregnancies and births
        self.process_pregnancies_and_births();

        // Process nursing for infants
        self.process_nursing();

        // Tick world (building construction progress, etc.)
        self.world.tick();

        // Apply religious building effects to agent happiness
        self.apply_religious_effects();

        // Log statistics every 10 ticks
        if self.current_tick % 10 == 0 {
            self.log_statistics();
        }

        // Check if autosave should trigger
        if let Err(e) = self.check_autosave() {
            warn!("Auto-save failed: {}", e);
        }
    }

    /// Generate an action based on recent percepts (if high-salience percepts exist)
    /// Returns None if no percept warrants immediate action
    fn generate_action_from_percepts(
        recent_percepts: &[(u32, crate::agents::sensory_processing::Percept)],
        agent_drives: &crate::core::DriveState,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::sensory_processing::{Percept, calculate_salience};

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
                    Percept::DangerDetected { threat_type: _, position, severity } => {
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
                    Percept::ResourceDetected {  position, .. } => {
                        // High-salience resource (usually means high hunger/thirst)
                        // Move towards it
                        return Some(Action::Move {
                            target: *position,
                        });
                    }
                    Percept::AgentDetected { agent_id,  .. } => {
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

    /// Find the nearest agent to use as a social interaction target
    /// Returns None if no suitable target is found
    fn find_nearest_social_target(
        agent_id: uuid::Uuid,
        position: (i32, i32, i32),
        agents: &[(uuid::Uuid, (i32, i32, i32))],
    ) -> Option<uuid::Uuid> {
        agents
            .iter()
            .filter(|(id, _)| *id != agent_id) // Exclude self
            .min_by_key(|(_, pos)| {
                let dx = (pos.0 - position.0).abs();
                let dy = (pos.1 - position.1).abs();
                dx + dy // Manhattan distance
            })
            .map(|(id, _)| *id)
    }

    /// Resolve a nil UUID in an action to an actual nearby agent
    fn resolve_action_target(
        action: Action,
        agent_id: uuid::Uuid,
        position: (i32, i32, i32),
        nearby_agents: &[(uuid::Uuid, (i32, i32, i32))],
    ) -> Action {
        match action {
            Action::Socialize { target_agent_id } if target_agent_id.is_nil() => {
                if let Some(target) = Self::find_nearest_social_target(agent_id, position, nearby_agents) {
                    Action::Socialize { target_agent_id: target }
                } else {
                    // No nearby agents, fall back to waiting
                    Action::Wait
                }
            }
            Action::ShareInformation { target_agent_id } if target_agent_id.is_nil() => {
                if let Some(target) = Self::find_nearest_social_target(agent_id, position, nearby_agents) {
                    Action::ShareInformation { target_agent_id: target }
                } else {
                    Action::Wait
                }
            }
            Action::Mate { target_agent_id } if target_agent_id.is_nil() => {
                if let Some(target) = Self::find_nearest_social_target(agent_id, position, nearby_agents) {
                    Action::Mate { target_agent_id: target }
                } else {
                    Action::Wait
                }
            }
            other => other, // Return unchanged
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
        }
    }

    /// Generate an action based on an active goal
    fn generate_action_for_goal(
        goal: &crate::core::goals::Goal,
        position: (i32, i32, i32),
        _fallback_drive: DriveType,
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
                ExternalGoal::LearnSkill(skill_name) => {
                    // Learning happens through practice - map skill to relevant action
                    let skill_lower = skill_name.to_lowercase();
                    if skill_lower.contains("mining") {
                        Some(Action::Gather { resource_type: "stone".to_string() })
                    } else if skill_lower.contains("woodcutting") || skill_lower.contains("carpentry") {
                        Some(Action::Gather { resource_type: "wood".to_string() })
                    } else if skill_lower.contains("crafting") || skill_lower.contains("metalworking") {
                        Some(Action::Craft { item_type: "tool".to_string() })
                    } else if skill_lower.contains("construction") || skill_lower.contains("masonry") {
                        Some(Action::Build { structure_type: "structure".to_string(), position })
                    } else if skill_lower.contains("farming") || skill_lower.contains("herbalism") {
                        Some(Action::Gather { resource_type: "food".to_string() })
                    } else if skill_lower.contains("cooking") || skill_lower.contains("smelting") {
                        Some(Action::Craft { item_type: "processed".to_string() })
                    } else if skill_lower.contains("hunting") || skill_lower.contains("combat") || skill_lower.contains("archery") {
                        Some(Action::Hunt { animal_id: uuid::Uuid::nil(), weapon: None })
                    } else if skill_lower.contains("fishing") {
                        Some(Action::Gather { resource_type: "fish".to_string() })
                    } else if skill_lower.contains("social") {
                        Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() })
                    } else if skill_lower.contains("navigation") {
                        Some(Action::Explore { direction: (1, 0, 0) })
                    } else {
                        // Default: generic crafting to practice skills
                        Some(Action::Craft { item_type: "practice".to_string() })
                    }
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

    /// Generate an action based on non-emotional priorities:
    /// PRIORITY 1: Check if agent needs shelter due to exposure
    /// PRIORITY 2: Check for high-salience percepts
    /// PRIORITY 3: Execute current plan step
    /// PRIORITY 4: Check active goals
    /// PRIORITY 5: Use drive-based action
    fn generate_non_emotional_action(
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        _population: &Population,
        _current_tick: u32,
    ) -> (Action, bool) {
        // PRIORITY 1: Check if agent needs shelter due to exposure
        if agent.exposure_status.exposure_damage > 0.5 {
            return (Action::SeekShelter, false);
        }

        // PRIORITY 2: Check for high-salience percepts (danger, resources, social opportunities)
        let recent_percepts: Vec<(u32, crate::agents::sensory_processing::Percept)> = agent.recent_percepts
            .iter()
            .cloned()
            .collect();

        if let Some(percept_action) = Self::generate_action_from_percepts(
            &recent_percepts,
            &agent.drives,
            agent_position,
        ) {
            return (percept_action, false);
        }

        // PRIORITY 3: Execute current plan step (if agent has an active plan)
        if agent.should_execute_plan() {
            if let Some(plan_action) = agent.get_plan_action() {
                return (plan_action, true);
            }
        }

        // PRIORITY 4: Check active goals and generate goal-directed action
        if let Some(goal) = agent.goals.highest_priority_goal() {
            // Get most urgent drive for fallback
            let fallback_drive = agent.drives.most_urgent()
                .map(|d| d.drive_type)
                .unwrap_or(DriveType::Curiosity);

            if let Some(goal_action) = Self::generate_action_for_goal(&goal, agent_position, fallback_drive) {
                return (goal_action, false);
            }
        }

        // PRIORITY 5: Use drive-based action as fallback
        let drive_type = agent.select_drive_with_happiness()
            .or_else(|| agent.drives.most_urgent().map(|d| d.drive_type))
            .unwrap_or(DriveType::Curiosity);

        (Self::generate_action_for_drive(drive_type, agent_position), false)
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
                    "water" => Some(ResourceType::Water),
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
                        // Water is consumed immediately (drinking), not stored
                        if resource_type_enum == ResourceType::Water {
                            let agent = &mut self.population.agents[agent_index];

                            // Satisfy thirst drive
                            if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                                thirst.partial_satisfy(0.5);
                            }

                            // Reset dehydration counter
                            agent.state.last_drank_tick = self.current_tick;
                            agent.state.ticks_without_water = 0;

                            // Fill containers if agent has any
                            let filled = agent.inventory.fill_containers(harvested as f32);

                            debug!(
                                "Agent {} drank water and filled {:.1} units into containers",
                                agent.id, filled
                            );

                            return ActionResult::success()
                                .with_drive_change(DriveType::Thirst, -0.5)
                                .with_energy_cost(5.0)
                                .with_message(format!("Drank water, filled {:.1} into containers", filled));
                        }

                        // Add to agent inventory (non-water resources)
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
                            // Grant skill XP based on resource type
                            let skill_type = match resource_type_enum {
                                ResourceType::Wood => crate::agents::skills::SkillType::Woodcutting,
                                ResourceType::Stone | ResourceType::Iron => crate::agents::skills::SkillType::Mining,
                                ResourceType::Food => crate::agents::skills::SkillType::Herbalism,
                                _ => crate::agents::skills::SkillType::Mining,
                            };
                            agent.skills.gain_experience(skill_type, 2);

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

                // Use spatial planning to find optimal build location
                let (_criteria, strategy) = determine_placement_approach(building_type);
                let planner = SpatialPlanner::new(&self.world);

                let optimal_pos = planner.find_optimal_location_for_agent(
                    building_type,
                    *position,  // agent's current position
                    strategy
                );

                // Use optimal position if found, otherwise fall back to agent's position
                let build_tuple_pos = optimal_pos.unwrap_or_else(|| {
                    debug!("No optimal position found for {:?}, using agent position", building_type);
                    *position
                });

                let build_pos = Position::new(build_tuple_pos.0, build_tuple_pos.1);
                if self.world.is_position_occupied(&build_pos) {
                    return ActionResult::failure("No suitable building location found (all positions occupied)".to_string());
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

                // Emit building started event for timeline
                #[cfg(feature = "gui")]
                {
                    use crate::gui::events::{SimulationEvent, SimulationEventType};
                    let agent = &self.population.agents[agent_index];
                    let event = SimulationEvent::new(
                        self.current_tick,
                        SimulationEventType::BuildingStarted {
                            building_type,
                            position: build_pos,
                            builder_id: agent.id,
                        },
                        Some((build_pos.x, build_pos.y)),
                    );
                    self.population.pending_events.push(event);
                }

                // Grant Construction XP (more XP for larger buildings)
                let construction_xp = match building_type {
                    BuildingType::SmallHouse => 5,
                    BuildingType::MediumHouse => 10,
                    BuildingType::LargeHouse => 15,
                    BuildingType::Workshop => 12,
                    BuildingType::Storehouse => 8,
                    BuildingType::Farm => 10,
                    _ => 5,
                };
                let agent = &mut self.population.agents[agent_index];
                agent.skills.gain_experience(crate::agents::skills::SkillType::Construction, construction_xp);

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

                // Check if target is in range
                let distance = ((target_pos.0 - attacker_pos.0).abs() + (target_pos.1 - attacker_pos.1).abs()) as u32;

                // Get weapon range from equipment (melee = 1, ranged = further)
                let attacker = &self.population.agents[agent_index];
                let weapon_range = attacker.equipment.weapon_range();

                if (distance as f32) > weapon_range {
                    return ActionResult::failure(format!("Target too far away (distance: {}, weapon range: {})", distance, weapon_range));
                }

                // Calculate weapon-based damage
                let attacker = &self.population.agents[agent_index];
                let weapon_damage = attacker.equipment.weapon_damage();
                let weapon_speed = attacker.equipment.weapon_attack_speed();

                // Get combat skill bonus (MeleeCombat for melee, Archery for ranged)
                let skill_level = attacker.skills.get_skill_if_exists(crate::agents::SkillType::MeleeCombat)
                    .map(|s| s.level)
                    .unwrap_or(0);
                let skill_modifier = 1.0 + (skill_level as f32 / 20.0); // -10 to 10 -> 0.5 to 1.5

                // Calculate base damage with variance
                // Weapon speed affects damage: faster weapons deal slightly less damage per hit
                let damage_variance = rng.gen_range(0.8..1.2); // +/- 20% randomness
                let speed_factor = (2.0 - weapon_speed).max(0.5); // Fast weapons (1.5) -> 0.5, slow (0.6) -> 1.4
                let base_damage = weapon_damage * damage_variance * skill_modifier * speed_factor;

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

                // Get IDs before borrowing
                let attacker_id = self.population.agents[agent_index].id;
                let target_id = self.population.agents[target_index].id;

                // EMOTIONAL RESPONSE: Target responds emotionally to being attacked
                {
                    // Calculate attacker's apparent strength
                    let attacker = &self.population.agents[agent_index];
                    let attacker_health = attacker.state.health / 100.0;
                    let attacker_armor = attacker.equipment.total_armor() / 100.0;
                    let attacker_has_weapon = attacker.equipment.get_weapon().is_some();
                    let attacker_strength = attacker_health * (1.0 + attacker_armor * 0.5)
                        + if attacker_has_weapon { 0.3 } else { 0.0 };

                    // Target responds to threat
                    let target = &mut self.population.agents[target_index];
                    let emotion_source = crate::agents::EmotionSource::Agent(attacker_id);

                    // Record who attacked for potential retaliation
                    target.emotions.record_attack(attacker_id, self.current_tick);

                    // Scale emotional response by damage severity
                    let damage_severity = (actual_damage / 50.0).min(1.0);

                    // Use threat assessment to determine fear vs anger
                    target.respond_to_threat(attacker_strength + damage_severity * 0.5, emotion_source);

                    debug!(
                        "Agent {} emotional response to attack: fear={:.2}, anger={:.2}, should_flee={}, should_attack={}",
                        target_id, target.emotions.fear, target.emotions.anger,
                        target.emotions.should_flee(), target.emotions.should_attack()
                    );
                }

                // Check if target died from the attack
                let target_alive = self.population.agents[target_index].body.is_alive()
                    && self.population.agents[target_index].state.health > 0.0;

                let attacker_mounted = self.population.agents[agent_index].transport.is_mounted();

                // Emit conflict event for timeline
                #[cfg(feature = "gui")]
                {
                    use crate::gui::events::{SimulationEvent, SimulationEventType};
                    let event = SimulationEvent::new(
                        self.current_tick,
                        SimulationEventType::Conflict {
                            attacker_id,
                            target_id,
                            damage: actual_damage,
                            fatal: !target_alive,
                        },
                        Some((attacker_pos.0, attacker_pos.1)),
                    );
                    self.population.pending_events.push(event);
                }

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

                // Grant combat XP (more for kills, check weapon type for skill)
                let attacker = &mut self.population.agents[agent_index];
                let combat_xp = if !target_alive { 5 } else { 2 };
                // TODO: Check weapon type for Archery vs MeleeCombat
                attacker.skills.gain_experience(crate::agents::skills::SkillType::MeleeCombat, combat_xp);

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

                // Target position
                let target_2d = Position::new(target.0, target.1);

                // Check if already at target (including Z-axis)
                if current_2d == target_2d && current_pos.2 == target.2 {
                    return ActionResult::success()
                        .with_message("Already at destination".to_string());
                }

                // Calculate movement distance (3D Manhattan distance)
                let dx = target.0 - current_pos.0;
                let dy = target.1 - current_pos.1;
                let dz = target.2 - current_pos.2;

                // Normalize to -1, 0, or 1 for each axis
                let step_x = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
                let step_y = if dy > 0 { 1 } else if dy < 0 { -1 } else { 0 };
                let step_z = if dz > 0 { 1 } else if dz < 0 { -1 } else { 0 };

                // Determine next step - prioritize horizontal movement, then vertical
                // This models climbing/descending as slower than horizontal movement
                let (next_x, next_y, next_z) = if dx.abs() >= dy.abs() && dx.abs() >= dz.abs() {
                    // Move along X axis
                    (current_pos.0 + step_x, current_pos.1, current_pos.2)
                } else if dy.abs() >= dz.abs() {
                    // Move along Y axis
                    (current_pos.0, current_pos.1 + step_y, current_pos.2)
                } else {
                    // Move along Z axis (climbing/descending)
                    (current_pos.0, current_pos.1, current_pos.2 + step_z)
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

                // Update agent position (including Z-axis)
                let agent = &mut self.population.agents[agent_index];
                agent.state.position = (next_x, next_y, next_z);

                // Calculate remaining 3D distance
                let remaining_distance = ((target.0 - next_x).abs() + (target.1 - next_y).abs() + (target.2 - next_z).abs()) as u32;

                debug!(
                    "Agent {} moved from ({}, {}, {}) to ({}, {}, {}) (distance to target: {}, speed: {:.2}x, mounted: {})",
                    agent.id, current_pos.0, current_pos.1, current_pos.2, next_x, next_y, next_z,
                    remaining_distance, movement_speed,
                    if agent.transport.is_mounted() { "yes" } else { "no" }
                );

                // Determine drive satisfaction based on purpose (Safety or Curiosity)
                let drive_type = if remaining_distance <= 5 {
                    Some(DriveType::Safety) // Moving to nearby location (fleeing or seeking safety)
                } else {
                    Some(DriveType::Curiosity) // Exploring distant location
                };

                let mut result = ActionResult::success()
                    .with_energy_cost(actual_energy_cost)
                    .with_message(format!("Moved to ({}, {}, {}), {} steps to goal", next_x, next_y, next_z, remaining_distance));

                if let Some(drive) = drive_type {
                    result = result.with_drive_change(drive, -0.05);
                }

                result
            },

            Action::Store { item_type, amount } => {
                use crate::agents::storage_integration::{
                    id_to_item_type, take_from_agent_inventory,
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

                        let agent_id = agent.id;
                        let agent_pos = (agent.state.position.0, agent.state.position.1);

                        debug!(
                            "Agent {} deposited {} {} to storehouse (storehouse now has {})",
                            agent.id,
                            removed,
                            item_type,
                            self.world.storehouse_inventory.items.get(&item)
                                .map(|i| i.quantity)
                                .unwrap_or(0)
                        );

                        // Emit storehouse deposit event for timeline (only for significant deposits)
                        #[cfg(feature = "gui")]
                        if removed >= 3 {
                            use crate::gui::events::{SimulationEvent, SimulationEventType};
                            let event = SimulationEvent::new(
                                self.current_tick,
                                SimulationEventType::StorehouseDeposit {
                                    agent_id,
                                    resource: item_type.clone(),
                                    amount: removed,
                                },
                                Some(agent_pos),
                            );
                            self.population.pending_events.push(event);
                        }

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
                    id_to_item_type, add_to_agent_inventory
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
                    let (_success, added) = add_to_agent_inventory(
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

                        // Practice animal husbandry (Farming skill)
                        let agent = &mut self.population.agents[agent_index];
                        agent.skills.gain_experience(crate::agents::skills::SkillType::Farming, 2);

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
                    SocialInteractionType, HelpType,
                    calculate_relationship_change, calculate_social_satisfaction,
                    should_greet, select_conversation_topic, calculate_gift_value, would_accept_gift
                };
                use crate::core::traits::Trait;

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
                let relationship = initiator_agent.relationships
                    .get_or_create_relationship(*target_agent_id, current_tick);

                let current_relationship = relationship.relationship_level();
                let current_trust = relationship.trust_level();
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
                let message;

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
                let relationship = initiator.relationships
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
                let target_relationship = target.relationships
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
                    // Grant Social skill XP
                    let initiator = &mut self.population.agents[agent_index];
                    initiator.skills.gain_experience(crate::agents::skills::SkillType::Social, 1);

                    // Record that this agent satisfied our social drive
                    let tick = self.current_tick;
                    let initiator = &mut self.population.agents[agent_index];
                    initiator.record_drive_satisfaction(DriveType::Social, *target_agent_id, social_satisfaction, tick);

                    // Helper happiness for initiator (providing social satisfaction to target)
                    let initiator = &mut self.population.agents[agent_index];
                    initiator.process_helper_happiness(*target_agent_id, target_satisfaction);

                    // Also record for the target (reciprocal satisfaction)
                    let target = &mut self.population.agents[target_index];
                    target.record_drive_satisfaction(DriveType::Social, initiator_id, target_satisfaction, tick);

                    // Helper happiness for target (providing social satisfaction to initiator)
                    let target = &mut self.population.agents[target_index];
                    target.process_helper_happiness(initiator_id, social_satisfaction);

                    debug!(
                        "Agent {} socialized with agent {}: {} (relationship change: {:+}, satisfaction: {:.2})",
                        initiator_id,
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
                use crate::core::traits::Trait;

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
                    let _initiator_traits: Vec<Trait> = initiator.traits.get_traits().iter().copied().collect();
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
                    InformationType::Conflict { agent1: _, agent2: _ } => {
                        format!("Gossiped about conflict between agents")
                    }
                    InformationType::Death { agent: _, cause } => {
                        format!("Shared news of death: {}", cause)
                    }
                    InformationType::TechnologyDiscovered { tech } => {
                        format!("Shared discovery of {} technology", tech)
                    }
                    InformationType::Accusation {  crime, .. } => {
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
                use crate::agents::reproduction::{can_mate, MateSelectionCriteria};
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
                if let Some(relationship) = initiator.relationships.get_relationship(&target_id) {
                    match &relationship.relationship_level() {
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
                    // Mating successful - determine male/female and attempt impregnation
                    use crate::agents::reproduction::attempt_impregnation;
                    use crate::agents::Gender;

                    let initiator = &self.population.agents[agent_index];
                    let target = &self.population.agents[target_index];

                    // Get male and female from the pair
                    let (male_index, female_index) = match (initiator.gender, target.gender) {
                        (Gender::Male, Gender::Female) => (agent_index, target_index),
                        (Gender::Female, Gender::Male) => (target_index, agent_index),
                        _ => {
                            return ActionResult::failure("Same-gender mating not possible".to_string());
                        }
                    };

                    // Attempt impregnation
                    let male = &self.population.agents[male_index];
                    let female = &self.population.agents[female_index];
                    let current_tick = self.current_tick;

                    if let Some(pregnancy) = attempt_impregnation(male, female, current_tick) {
                        // Pregnancy started!
                        let female = &mut self.population.agents[female_index];
                        female.pregnancy = Some(pregnancy);

                        debug!(
                            "Agent {} (male) and agent {} (female) mated - pregnancy started!",
                            self.population.agents[male_index].id,
                            self.population.agents[female_index].id
                        );

                        // Generate gossip about the pregnancy
                        let female_id = self.population.agents[female_index].id;
                        let female_pos = self.population.agents[female_index].state.position;
                        let pregnancy_info = Information::new(
                            InformationType::Pregnancy {
                                agent: female_id,
                            },
                            female_id,
                            true,
                            current_tick as u64,
                        );

                        // Share pregnancy information with nearby agents
                        for other_agent in &mut self.population.agents {
                            if other_agent.id != initiator_id && other_agent.id != target_id {
                                let distance = {
                                    let dx = (other_agent.state.position.0 - female_pos.0) as f32;
                                    let dy = (other_agent.state.position.1 - female_pos.1) as f32;
                                    (dx * dx + dy * dy).sqrt()
                                };

                                if distance <= 15.0 {
                                    other_agent.knowledge.receive_information(
                                        pregnancy_info.clone(),
                                        female_id,
                                        other_agent.id,
                                        &other_agent.traits,
                                        current_tick as u64,
                                    );
                                }
                            }
                        }

                        // Update reproduction drives for both parents
                        let male = &mut self.population.agents[male_index];
                        if let Some(repro_drive) = male.drives.get_mut(DriveType::Reproduction) {
                            repro_drive.decrease(0.5); // Male drive reduces moderately
                        }

                        let female = &mut self.population.agents[female_index];
                        if let Some(repro_drive) = female.drives.get_mut(DriveType::Reproduction) {
                            repro_drive.decrease(0.9); // Female drive significantly reduces (pregnant)
                        }

                        ActionResult::success()
                            .with_drive_change(DriveType::Reproduction, -0.7)
                            .with_energy_cost(15.0)
                            .with_message("Mating successful - pregnancy started!".to_string())
                    } else {
                        // Conception failed (fertility roll failed)
                        debug!(
                            "Agent {} and agent {} mated but conception failed",
                            initiator_id, target_id
                        );

                        // Still reduce drives somewhat
                        let agent = &mut self.population.agents[agent_index];
                        if let Some(repro_drive) = agent.drives.get_mut(DriveType::Reproduction) {
                            repro_drive.decrease(0.3);
                        }

                        let target = &mut self.population.agents[target_index];
                        if let Some(repro_drive) = target.drives.get_mut(DriveType::Reproduction) {
                            repro_drive.decrease(0.3);
                        }

                        ActionResult::success()
                            .with_drive_change(DriveType::Reproduction, -0.3)
                            .with_energy_cost(10.0)
                            .with_message("Mating occurred but no conception".to_string())
                    }
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

                let _agent = &mut self.population.agents[agent_index];

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

                // Grant Navigation XP for exploration (more for new discoveries)
                let agent = &mut self.population.agents[agent_index];
                let nav_xp = if newly_explored_count > 0 { 2 } else { 1 };
                agent.skills.gain_experience(crate::agents::skills::SkillType::Navigation, nav_xp);

                // Exploration is rewarding
                let curiosity_satisfaction = if newly_explored_count > 0 { 0.3 } else { 0.1 };

                ActionResult::success()
                    .with_drive_change(DriveType::Curiosity, -curiosity_satisfaction)
                    .with_energy_cost(5.0) // Exploration takes energy
                    .with_message(message)
            },
        }
    }

    /// Process environmental damage for all agents
    pub fn process_environmental_damage(&mut self) {
        use crate::agents::body::{BodyPartType, InjuryType, CripplingType};
        use crate::world::{Position, TerrainType};
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for agent in &mut self.population.agents {
            let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);

            // Get terrain at agent position
            let terrain_type = self.world.grid.get_tile(&agent_pos)
                .map(|tile| tile.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            // Get actual temperature from climate system (returns f32 in Celsius)
            let temp_celsius = self.world.climate.get_temperature(agent_pos, terrain_type);

            // 1. EXPOSURE DAMAGE - Cold/Heat based on actual environment temperature
            let cold_insulation = agent.body.total_cold_insulation();
            let heat_resistance = agent.body.total_heat_resistance();

            // Cold exposure (temperature below 5°C with inadequate insulation)
            if temp_celsius < 5.0 {
                let cold_severity = ((5.0_f32 - temp_celsius) / 30.0).min(1.0); // Max severity at -25°C
                let effective_cold = cold_severity * (1.0 - cold_insulation.min(1.0));

                if effective_cold > 0.1 && rng.gen_bool((effective_cold * 0.02) as f64) {
                    let cold_damage = rng.gen_range(1.0..5.0) * effective_cold;
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
                        debug!("Agent {} suffered cold exposure at {:.1}°C: {:.1} damage to {:?}",
                            agent.id, temp_celsius, cold_damage, part);
                    }
                }
            }

            // Heat exposure (temperature above 35°C with inadequate heat resistance)
            if temp_celsius > 35.0 {
                let heat_severity = ((temp_celsius - 35.0) / 20.0).min(1.0); // Max severity at 55°C
                let effective_heat = heat_severity * (1.0 - heat_resistance.min(1.0));

                if effective_heat > 0.1 && rng.gen_bool((effective_heat * 0.01) as f64) {
                    let heat_damage = rng.gen_range(2.0..8.0) * effective_heat;
                    // Heat affects torso and head
                    let affected_parts = [BodyPartType::Head, BodyPartType::Torso];
                    let part = affected_parts[rng.gen_range(0..affected_parts.len())];

                    if let Some(body_part) = agent.body.get_part_mut(part) {
                        body_part.apply_injury(InjuryType::Minor, heat_damage, self.current_tick as u64);
                        debug!("Agent {} suffered heat exposure at {:.1}°C: {:.1} damage to {:?}",
                            agent.id, temp_celsius, heat_damage, part);
                    }
                }
            }

            // 2. FALLING DAMAGE - Based on terrain type and elevation
            // Higher fall risk on mountains, hills, and near water (slippery)
            let fall_risk = match terrain_type {
                TerrainType::Mountain => 0.001,    // 0.1% - steep terrain
                TerrainType::Hills => 0.0003,      // 0.03% - uneven ground
                TerrainType::Riverbank => 0.0002,  // 0.02% - slippery banks
                TerrainType::Wetland => 0.0002,    // 0.02% - unstable footing
                TerrainType::Beach => 0.0001,      // 0.01% - uneven sand
                TerrainType::Forest => 0.00005,    // 0.005% - roots and obstacles
                _ => 0.00002,                      // 0.002% - flat terrain
            };

            if rng.gen_bool(fall_risk) {
                // Fall severity based on terrain
                let max_fall_height = match terrain_type {
                    TerrainType::Mountain => 5,
                    TerrainType::Hills => 3,
                    _ => 2,
                };
                let fall_height = rng.gen_range(1..=max_fall_height);
                let fall_damage = (fall_height as f32) * rng.gen_range(3.0..8.0);

                // Falls primarily affect legs, with chance of head/torso on severe falls
                let injured_part = if fall_height >= 4 && rng.gen_bool(0.3) {
                    if rng.gen_bool(0.5) { BodyPartType::Head } else { BodyPartType::Torso }
                } else {
                    if rng.gen_bool(0.5) { BodyPartType::LeftLeg } else { BodyPartType::RightLeg }
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
                    debug!("Agent {} fell on {:?} terrain: {:.1} damage to {:?} ({:?})",
                        agent.id, terrain_type, fall_damage, injured_part, injury_severity);
                }

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
                        let _infection_damage = rng.gen_range(0.5..2.0);

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

    /// Process building production collection
    /// Agents within range of production buildings automatically collect pending resources
    fn process_building_production_collection(&mut self) {
        use crate::world::Position;

        const COLLECTION_RANGE: f32 = 5.0; // Agents must be within 5 units to collect

        // Get all pending production
        let pending_production = self.world.get_pending_production_info();

        if pending_production.is_empty() {
            return;
        }

        // For each agent, check if they're near a production building
        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);

            // Check each building with pending production
            for (building_pos, (building_type, _resource_count)) in &pending_production {
                let dx = (agent_pos.x - building_pos.x) as f32;
                let dy = (agent_pos.y - building_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= COLLECTION_RANGE {
                    // Agent is close enough to collect - collect from this building
                    let resources = self.world.collect_building_production_at(*building_pos);

                    for resource in resources {
                        // Add resource to agent's inventory
                        let item_name = format!("{:?}", resource.resource_type).to_lowercase();
                        agent.inventory.add_item(
                            crate::agents::InventoryItem::new(item_name.clone(), resource.amount)
                        );

                        debug!(
                            "Agent {} collected {} {} from {:?} at ({}, {})",
                            agent.id, resource.amount, item_name, building_type,
                            building_pos.x, building_pos.y
                        );
                    }

                    // Only collect from one building per tick per agent
                    break;
                }
            }
        }
    }

    /// Process building maintenance needs
    /// Generates maintenance goals for agents near buildings that need repair
    fn process_building_maintenance(&mut self) {
        use crate::world::Position;
        use crate::core::goals::{Goal, ExternalGoal};

        const MAINTENANCE_RANGE: f32 = 20.0; // Agents within 20 units get maintenance goals

        // Get buildings needing maintenance
        let maintenance_needed = self.world.get_buildings_needing_maintenance();
        let critical_buildings = self.world.get_critical_buildings();

        if maintenance_needed.is_empty() {
            return;
        }

        // For critical buildings, assign maintenance to nearby agents
        for (building_pos, building_type, condition) in &critical_buildings {
            // Find the closest agent to this building
            let mut closest_agent_idx: Option<usize> = None;
            let mut closest_distance = f32::MAX;

            for (idx, agent) in self.population.agents.iter().enumerate() {
                if !agent.state.is_alive {
                    continue;
                }

                let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
                let dx = (agent_pos.x - building_pos.x) as f32;
                let dy = (agent_pos.y - building_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < closest_distance && distance <= MAINTENANCE_RANGE {
                    closest_distance = distance;
                    closest_agent_idx = Some(idx);
                }
            }

            // Assign maintenance goal to closest agent
            if let Some(idx) = closest_agent_idx {
                let agent = &mut self.population.agents[idx];
                let maintenance_job = format!("maintain_{:?}", building_type);

                // Check if agent already has a maintenance goal for this building
                let has_maintenance_goal = agent.goals.goals.iter().any(|g| {
                    if let Some(ExternalGoal::CompleteJob(ref job)) = g.external {
                        job.contains("maintain")
                    } else {
                        false
                    }
                });

                if !has_maintenance_goal {
                    let priority = if *condition < 0.25 { 0.9 } else { 0.6 };
                    let goal = Goal::new_external(
                        ExternalGoal::CompleteJob(maintenance_job),
                        priority,
                        self.current_tick,
                    );
                    agent.goals.add_goal(goal);

                    debug!(
                        "Agent {} assigned maintenance goal for {:?} at ({}, {}) - condition: {:.0}%",
                        agent.id, building_type, building_pos.x, building_pos.y, condition * 100.0
                    );
                }
            }
        }

        // For non-critical but degraded buildings, inform nearby agents (lower priority)
        for (building_pos, building_type, condition) in &maintenance_needed {
            if critical_buildings.iter().any(|(p, _, _)| p == building_pos) {
                continue; // Already handled as critical
            }

            // Add to exploration knowledge of nearby agents so they're aware
            for agent in &mut self.population.agents {
                if !agent.state.is_alive {
                    continue;
                }

                let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
                let dx = (agent_pos.x - building_pos.x) as f32;
                let dy = (agent_pos.y - building_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= MAINTENANCE_RANGE {
                    // Agent is aware of this building's condition
                    // Could be used to generate lower-priority maintenance tasks
                    // For now, just log it
                    debug!(
                        "Agent {} aware of degraded {:?} at ({}, {}) - condition: {:.0}%",
                        agent.id, building_type, building_pos.x, building_pos.y, condition * 100.0
                    );
                }
            }
        }
    }

    /// Process information verification and lie detection
    /// Agents periodically verify information they've received against their knowledge
    fn process_information_verification(&mut self) {
        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Call the agent's lie detection processing
            agent.process_information_verification(self.current_tick);
        }
    }

    /// Process pregnancies and handle births
    fn process_pregnancies_and_births(&mut self) {
        use crate::agents::reproduction::give_birth;
        use crate::agents::gossip::{Information, InformationType};

        let current_tick = self.current_tick;

        // Collect births to process (to avoid borrowing issues)
        let mut births_to_process: Vec<(usize, uuid::Uuid)> = Vec::new();

        // First pass: update pregnancies and collect due births
        for (idx, agent) in self.population.agents.iter_mut().enumerate() {
            if let Some(ref mut pregnancy) = agent.pregnancy {
                // Update prenatal nutrition based on mother's current state
                let hunger_drive = agent.drives.get(DriveType::Hunger)
                    .map(|d| d.value)
                    .unwrap_or(0.0);
                pregnancy.update_nutrition(hunger_drive, agent.state.health);

                // Check if due
                if pregnancy.is_due(current_tick) {
                    births_to_process.push((idx, pregnancy.father_id));
                }
            }
        }

        // Second pass: process births
        for (mother_idx, father_id) in births_to_process {
            // Find the father
            let father_idx = self.population.agents.iter()
                .position(|a| a.id == father_id);

            // Get pregnancy data before clearing it
            let pregnancy = self.population.agents[mother_idx].pregnancy.take();

            if let Some(preg) = pregnancy {
                // Create offspring
                let offspring = if let Some(f_idx) = father_idx {
                    let mother = &self.population.agents[mother_idx];
                    let father = &self.population.agents[f_idx];
                    give_birth(mother, father, &preg, current_tick)
                } else {
                    // Father not found (dead?), use mother twice (not ideal but handles edge case)
                    let mother = &self.population.agents[mother_idx];
                    give_birth(mother, mother, &preg, current_tick)
                };

                let offspring_id = offspring.id;
                let mother_id = self.population.agents[mother_idx].id;
                let mother_pos = self.population.agents[mother_idx].state.position;

                // Add offspring to population
                self.population.agents.push(offspring);
                self.population.stats.total_births += 1;

                debug!(
                    "Agent {} gave birth to {}! Prenatal nutrition: {:.2}",
                    mother_id, offspring_id, preg.nutrition_quality
                );

                // Generate gossip about the birth
                let birth_info = Information::new(
                    InformationType::Childbirth {
                        agent: mother_id,
                        child: offspring_id,
                    },
                    mother_id,
                    true,
                    current_tick as u64,
                );

                // Share birth information with nearby agents
                for other_agent in &mut self.population.agents {
                    if other_agent.id != mother_id && other_agent.id != offspring_id {
                        let distance = {
                            let dx = (other_agent.state.position.0 - mother_pos.0) as f32;
                            let dy = (other_agent.state.position.1 - mother_pos.1) as f32;
                            (dx * dx + dy * dy).sqrt()
                        };

                        if distance <= 20.0 {
                            other_agent.knowledge.receive_information(
                                birth_info.clone(),
                                mother_id,
                                other_agent.id,
                                &other_agent.traits,
                                current_tick as u64,
                            );
                        }
                    }
                }

                // Add parent-child relationships
                let offspring_idx = self.population.agents.len() - 1;

                // Mother bonds with child
                use crate::agents::emotions::{Relationship, RelationshipType};
                self.population.agents[mother_idx].relationships.add_relationship(
                    Relationship::new(offspring_id, RelationshipType::Child)
                );

                // Father bonds with child (if alive)
                if let Some(f_idx) = father_idx {
                    self.population.agents[f_idx].relationships.add_relationship(
                        Relationship::new(offspring_id, RelationshipType::Child)
                    );
                }
            }
        }
    }

    /// Process nursing for infants
    fn process_nursing(&mut self) {
        use crate::agents::childcare::{MAX_CAREGIVER_DISTANCE, NURSING_ENERGY_GAIN};
        use crate::agents::LifeStage;

        let current_tick = self.current_tick;

        // Collect caregiver positions for distance checks
        let caregiver_positions: std::collections::HashMap<uuid::Uuid, (i32, i32, i32)> =
            self.population.agents.iter()
                .filter(|a| a.state.is_alive)
                .map(|a| (a.id, a.state.position))
                .collect();

        for agent in &mut self.population.agents {
            // Only process living infants with nursing state
            if !agent.state.is_alive || agent.state.life_stage != LifeStage::Infant {
                continue;
            }

            if let Some(ref mut nursing) = agent.nursing {
                // Check if still in nursing period
                if !nursing.needs_nursing(current_tick) {
                    // Nursing period ended
                    agent.nursing = None;
                    continue;
                }

                // Check if caregiver is nearby
                let agent_pos = agent.state.position;
                let caregiver_nearby = nursing.is_caregiver(nursing.primary_caregiver)
                    && caregiver_positions.get(&nursing.primary_caregiver)
                        .map(|&pos| {
                            let dx = (pos.0 - agent_pos.0) as f32;
                            let dy = (pos.1 - agent_pos.1) as f32;
                            (dx * dx + dy * dy).sqrt() <= MAX_CAREGIVER_DISTANCE
                        })
                        .unwrap_or(false);

                // Also check secondary caregivers
                let secondary_nearby = nursing.secondary_caregivers.iter()
                    .any(|&cg_id| {
                        caregiver_positions.get(&cg_id)
                            .map(|&pos| {
                                let dx = (pos.0 - agent_pos.0) as f32;
                                let dy = (pos.1 - agent_pos.1) as f32;
                                (dx * dx + dy * dy).sqrt() <= MAX_CAREGIVER_DISTANCE
                            })
                            .unwrap_or(false)
                    });

                if caregiver_nearby || secondary_nearby {
                    // Being nursed
                    nursing.nurse();

                    // Gain energy from nursing
                    agent.state.energy = (agent.state.energy + NURSING_ENERGY_GAIN).min(100.0);

                    // Update developmental nutrition (well nursed)
                    let hunger_satisfaction = 1.0 - agent.drives.get(DriveType::Hunger)
                        .map(|d| d.value)
                        .unwrap_or(0.0);
                    agent.developmental_nutrition.update_infant_nutrition(hunger_satisfaction, true);
                } else {
                    // Not being nursed
                    nursing.tick_without_nursing();

                    // Apply health penalty if suffering
                    let penalty = nursing.health_penalty();
                    if penalty > 0.0 {
                        agent.state.health = (agent.state.health - penalty).max(0.0);
                        debug!(
                            "Infant {} suffering from lack of nursing: -{:.1} health",
                            agent.id, penalty
                        );
                    }

                    // Update developmental nutrition (not nursed)
                    let hunger_satisfaction = 1.0 - agent.drives.get(DriveType::Hunger)
                        .map(|d| d.value)
                        .unwrap_or(0.0);
                    agent.developmental_nutrition.update_infant_nutrition(hunger_satisfaction, false);
                }
            }

            // Update child nutrition for children
            if agent.state.life_stage == LifeStage::Child {
                let hunger_satisfaction = 1.0 - agent.drives.get(DriveType::Hunger)
                    .map(|d| d.value)
                    .unwrap_or(0.0);
                agent.developmental_nutrition.update_child_nutrition(hunger_satisfaction, agent.state.health);
            }

            // Finalize developmental stats when transitioning to adult
            if agent.state.life_stage == LifeStage::Adult && !agent.developmental_nutrition.finalized {
                let became_infertile = agent.developmental_nutrition.finalize();

                if became_infertile {
                    // Severe malnutrition caused permanent infertility
                    agent.traits.add_trait(crate::core::traits::Trait::Infertile);
                    debug!(
                        "Agent {} reached adulthood but severe malnutrition caused INFERTILITY",
                        agent.id
                    );
                }

                debug!(
                    "Agent {} reached adulthood with development: {:?}",
                    agent.id, agent.developmental_nutrition.stat_modifiers
                );
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

    /// Update exposure damage for all agents based on weather and environmental conditions
    fn update_agent_exposure(&mut self) {
        let weather = self.world.climate.weather.clone();
        let time_of_day = self.world.climate.calendar.time_of_day;

        // Collect position data first to avoid borrow issues with climate.get_climate
        let agent_data: Vec<_> = self.population.agents.iter()
            .filter(|a| a.state.is_alive)
            .map(|a| {
                let pos = crate::world::Position::new(a.state.position.0, a.state.position.1);
                let terrain = self.world.grid.get_tile(&pos)
                    .map(|t| t.terrain.terrain_type)
                    .unwrap_or(crate::world::TerrainType::Plains);
                (a.id, pos, terrain)
            })
            .collect();

        // Get climate data for each agent position
        let climate_data: std::collections::HashMap<_, _> = agent_data.iter()
            .map(|(id, pos, terrain)| {
                let climate = self.world.climate.get_climate(*pos, *terrain);
                (*id, climate)
            })
            .collect();

        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Get the climate for this agent
            let climate = match climate_data.get(&agent.id) {
                Some(c) => c.clone(),
                None => continue,
            };

            // Get environmental temperature at agent's position
            let agent_pos = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
            let terrain_type = self.world.grid.get_tile(&agent_pos)
                .map(|t| t.terrain.terrain_type)
                .unwrap_or(crate::world::TerrainType::Plains);

            let environmental_temp = climate.temperature;

            // Update agent's body temperature based on climate
            agent.update_temperature(&climate);

            // Check if agent has shelter
            // Agent has shelter if they're in a completed building
            let has_shelter = self.world.buildings.iter().any(|b| {
                b.position == agent_pos && b.is_completed()
            }) || matches!(terrain_type, crate::world::TerrainType::Forest); // Forest provides partial shelter

            // Check if agent has water access:
            // 1. Water containers in inventory (waterskin, water_flask, water_bucket)
            // 2. Near water terrain (river, lake)
            // 3. Near well building
            let has_water_container = agent.inventory.get_item("waterskin")
                .or_else(|| agent.inventory.get_item("water_flask"))
                .or_else(|| agent.inventory.get_item("water_bucket"))
                .map(|item| item.fill_percentage() > 0.1)
                .unwrap_or(false);

            let near_water_terrain = matches!(
                terrain_type,
                crate::world::TerrainType::Water |
                crate::world::TerrainType::Riverbank |
                crate::world::TerrainType::Wetland
            );

            let has_water_access = has_water_container || near_water_terrain;

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

            // Log body temperature issues
            if agent.body_temperature.is_hypothermic() {
                debug!(
                    "Agent {} is hypothermic! Body temp: {:.1}°C",
                    agent.id, agent.body_temperature.current
                );
            } else if agent.body_temperature.is_hyperthermic() {
                debug!(
                    "Agent {} is hyperthermic! Body temp: {:.1}°C",
                    agent.id, agent.body_temperature.current
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

        // Serialize to MessagePack (supports complex HashMap keys like Position)
        let bytes = rmp_serde::to_vec(&state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Write to file
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;

        info!("Simulation saved at tick {}", self.current_tick);
        Ok(())
    }

    /// Load simulation state from a file
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        // Read file
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        // Deserialize from MessagePack
        let state: SerializableSimulationState = rmp_serde::from_slice(&bytes)
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
            autosave_config: None,
            last_autosave_tick: 0,
        };

        info!("Simulation loaded from tick {}", sim.current_tick);
        Ok(sim)
    }

    /// Enable auto-save with given configuration
    pub fn enable_autosave(&mut self, config: AutoSaveConfig) -> std::io::Result<()> {
        // Create save directory if it doesn't exist
        if !config.save_directory.exists() {
            fs::create_dir_all(&config.save_directory)?;
        }

        self.autosave_config = Some(config);
        self.last_autosave_tick = self.current_tick;

        info!("Auto-save enabled: interval={}, max_checkpoints={}, directory={:?}",
              self.autosave_config.as_ref().unwrap().interval_ticks,
              self.autosave_config.as_ref().unwrap().max_checkpoints,
              self.autosave_config.as_ref().unwrap().save_directory);

        Ok(())
    }

    /// Disable auto-save
    pub fn disable_autosave(&mut self) {
        self.autosave_config = None;
        info!("Auto-save disabled");
    }

    /// Execute a building action for an agent, using spatial planning to determine optimal location
    ///
    /// This is a test helper method that allows direct building action execution.
    /// The building will be placed at an optimal location determined by the spatial planner.
    ///
    /// # Arguments
    /// * `agent_index` - Index of the agent in the population
    /// * `building_type` - Type of building to construct
    ///
    /// # Returns
    /// * `Ok(Position)` - The position where the building was placed
    /// * `Err(String)` - Error message if building failed
    pub fn execute_building_action(
        &mut self,
        agent_index: usize,
        building_type: crate::world::BuildingType,
    ) -> Result<(i32, i32, i32), String> {
        use crate::world::{Building, Position, ResourceType};

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
            return Err(format!(
                "Missing resources for {:?}: {}",
                building_type,
                missing_resources.join(", ")
            ));
        }

        // Get agent's position
        let agent_pos = {
            let agent = &self.population.agents[agent_index];
            (agent.state.position.0, agent.state.position.1, agent.state.position.2)
        };

        // Use spatial planning to find optimal build location
        let (criteria, strategy) = determine_placement_approach(building_type);
        let planner = SpatialPlanner::new(&self.world);

        debug!("Spatial planning for {:?}: criteria={:?}, strategy={:?}",
               building_type, criteria, strategy);
        debug!("World has {} resource node types", self.world.resource_nodes.len());

        let optimal_pos = planner.find_optimal_location_for_agent(
            building_type,
            agent_pos,
            strategy
        );

        debug!("Optimal position found: {:?}", optimal_pos);

        // Use optimal position if found, otherwise fall back to agent's position
        let build_tuple_pos = optimal_pos.ok_or_else(|| {
            "No suitable building location found".to_string()
        })?;

        let build_pos = Position::new(build_tuple_pos.0, build_tuple_pos.1);
        if self.world.is_position_occupied(&build_pos) {
            return Err("No suitable building location found (all positions occupied)".to_string());
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
            "Agent {} started construction of {:?} at ({}, {}, {})",
            agent_index, building_type, build_tuple_pos.0, build_tuple_pos.1, build_tuple_pos.2
        );

        Ok(build_tuple_pos)
    }

    /// Check if auto-save should trigger and perform it
    fn check_autosave(&mut self) -> std::io::Result<()> {
        if let Some(config) = &self.autosave_config {
            if !config.enabled {
                return Ok(());
            }

            // Check if it's time to auto-save
            let ticks_since_last_save = self.current_tick - self.last_autosave_tick;
            if ticks_since_last_save >= config.interval_ticks {
                self.perform_autosave()?;
                self.last_autosave_tick = self.current_tick;
            }
        }

        Ok(())
    }

    /// Perform an auto-save checkpoint
    fn perform_autosave(&self) -> std::io::Result<()> {
        if let Some(config) = &self.autosave_config {
            // Generate checkpoint filename with timestamp
            let filename = format!("checkpoint_tick_{:08}.json", self.current_tick);
            let checkpoint_path = config.save_directory.join(&filename);

            // Save the simulation
            self.save(&checkpoint_path)?;

            info!("Auto-save checkpoint created: {}", filename);

            // Clean up old checkpoints
            self.cleanup_old_checkpoints()?;
        }

        Ok(())
    }

    /// Remove old checkpoints, keeping only the most recent max_checkpoints
    fn cleanup_old_checkpoints(&self) -> std::io::Result<()> {
        if let Some(config) = &self.autosave_config {
            // Get all checkpoint files
            let mut checkpoint_files: Vec<_> = fs::read_dir(&config.save_directory)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("checkpoint_") && n.ends_with(".json"))
                        .unwrap_or(false)
                })
                .collect();

            // Sort by modification time (newest first)
            checkpoint_files.sort_by_cached_key(|path| {
                fs::metadata(path)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            checkpoint_files.reverse();

            // Remove old checkpoints beyond max_checkpoints
            if checkpoint_files.len() > config.max_checkpoints {
                for old_checkpoint in checkpoint_files.iter().skip(config.max_checkpoints) {
                    fs::remove_file(old_checkpoint)?;
                    debug!("Removed old checkpoint: {:?}", old_checkpoint.file_name());
                }
            }
        }

        Ok(())
    }

    /// Get the latest checkpoint file from a directory
    pub fn get_latest_checkpoint<P: AsRef<Path>>(checkpoint_dir: P) -> std::io::Result<PathBuf> {
        let mut checkpoint_files: Vec<_> = fs::read_dir(checkpoint_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("checkpoint_") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();

        if checkpoint_files.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No checkpoint files found"
            ));
        }

        // Sort by modification time (newest first)
        checkpoint_files.sort_by_cached_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        checkpoint_files.reverse();

        Ok(checkpoint_files[0].clone())
    }

    /// Apply religious building effects to agent happiness
    /// Believers gain happiness near Shrines/Temples, Atheists feel uncomfortable
    fn apply_religious_effects(&mut self) {
        use crate::world::{BuildingType, Position};
        use crate::agents::Trait;

        // Collect religious buildings (position, type, is_completed)
        let religious_buildings: Vec<(Position, BuildingType, bool)> = self.world.buildings
            .iter()
            .filter(|b| b.building_type.is_religious())
            .map(|b| (b.position, b.building_type, b.is_completed()))
            .collect();

        // If no religious buildings, skip processing
        if religious_buildings.is_empty() {
            return;
        }

        // First, count believers near each agent for zealot community bonuses
        // Pre-calculate positions and traits
        let agent_data: Vec<_> = self.population.agents.iter()
            .filter(|a| a.state.is_alive)
            .map(|a| {
                let pos = Position::new(a.state.position.0, a.state.position.1);
                let is_believer = a.traits.has(Trait::Believer) || a.traits.has(Trait::Zealot);
                (a.id, pos, is_believer)
            })
            .collect();

        // Calculate nearby believers for each agent
        let nearby_believers: std::collections::HashMap<_, _> = agent_data.iter()
            .map(|(id, pos, _)| {
                let count = agent_data.iter()
                    .filter(|(other_id, other_pos, is_believer)| {
                        *is_believer
                            && other_id != id
                            && pos.distance_to(other_pos) <= RELIGIOUS_EFFECT_RADIUS
                    })
                    .count() as u32;
                (*id, count)
            })
            .collect();

        // Apply religious effects to each agent
        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
            let believers_nearby = *nearby_believers.get(&agent.id).unwrap_or(&0);

            // Calculate religious effects for this agent
            let effects = calculate_religious_effects(
                agent_pos,
                &agent.traits,
                &religious_buildings,
                believers_nearby,
            );

            // Apply effects
            let total_modifier = total_happiness_modifier(&effects);
            if total_modifier.abs() > 0.001 {
                // Generate a combined source description
                let source = if total_modifier > 0.0 {
                    format!("Religious fulfillment ({})", effects.len())
                } else {
                    format!("Religious discomfort ({})", effects.len())
                };

                agent.apply_religious_happiness(total_modifier, &source);

                debug!(
                    "Agent {} received religious effect: {:.3} happiness from {} sources",
                    agent.id, total_modifier, effects.len()
                );
            }
        }
    }
}


#[cfg(test)]
mod tests;

