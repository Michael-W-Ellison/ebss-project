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
use crate::world::{FoodDatabase, NutritionalContent, EatResult};
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
    /// Nutritional data for food items, used when agents forage and eat
    food_database: FoodDatabase,
    /// A tally of every action any agent has chosen, by name.
    ///
    /// "Seventy-nine per cent of everything a settlement does is foraging" is
    /// the kind of claim this project keeps needing and kept answering by
    /// patching a counter in by hand and throwing it away afterwards. Kept
    /// here so the answer is reproducible and costs one hash lookup a tick.
    pub actions_taken: std::collections::HashMap<String, u64>,
    /// And how many of those came to nothing.
    ///
    /// An action chosen is not an action that worked. Counting only the
    /// choosing hides the case where a settlement spends a sixth of its life
    /// attempting something that almost never succeeds, which is exactly what
    /// it turned out to be doing.
    pub actions_failed: std::collections::HashMap<String, u64>,
    /// And what they said when they did.
    ///
    /// A count of failures tells you a settlement is wasting its time; the
    /// reasons tell you what on. Both are one hash lookup on a path that only
    /// runs when something has already gone wrong, and between them they turn
    /// "the drives ask for things that do not happen" into a list of named
    /// defects.
    pub actions_failed_because: std::collections::HashMap<String, u64>,
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
            food_database: FoodDatabase::default(),
            actions_taken: std::collections::HashMap::new(),
            actions_failed: std::collections::HashMap::new(),
            actions_failed_because: std::collections::HashMap::new(),
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
        // Food does not sit on a fire forever: it is taken off, or it burns
        // away. Either way the smell of cooking is a passing thing, so old
        // contents are cleared before scents are worked out.
        self.clear_finished_cooking();

        // Let agents smell nearby food and water before they perceive and act,
        // so world resources reach the percept/memory pipeline this tick.
        self.emit_scents();

        // Process population lifecycle (aging, starvation, deaths, reproduction)
        // This also increments the tick counter and updates all agents
        self.population.tick();

        // Sync simulation tick with population tick
        self.current_tick = self.population.current_tick;

        // Let agents look around them. Sight needs both the population and the
        // world, which only exist together here, so this is the one place it
        // can happen - and until it did, agents found food by smell alone.
        self.population.process_exploration_with_world(&mut self.world);

        // Looking around fills a head faster than talking does, so what
        // nobody has a use for goes out of it again after the looking rather
        // than before
        {
            let now = self.current_tick;
            for agent in self.population.agents.iter_mut() {
                if agent.state.is_alive {
                    agent.forget_what_does_not_matter(now);
                }
            }
        }

        // World systems - climate, fauna, flora - are ticked by World::tick
        // further down this function. Ticking them here as well ran the whole
        // living world at double speed: animals aged, starved, bred and grazed
        // twice for every tick an agent lived through.

        // A man sitting at a fire with a bright stone in his hand may notice
        // what the fire does to it
        self.somebody_notices_something();

        // And the ground they fouled last season comes up in berries
        self.what_was_dropped_comes_up();

        // Grain carried through a wet season starts growing in the pack, and
        // what is dropped out of a pack takes root where it falls
        self.what_got_wet_sprouts();
        self.what_was_dropped_takes_root();

        // Let hungry predators try their luck with the people
        self.process_predator_attacks();

        // Update exposure damage for all agents
        self.update_agent_exposure();

        // Tell each agent what the world around it is doing, so that next
        // tick its drives rise on the conditions the design document gives
        // them rather than on a clock
        self.read_the_situation();

        // Put back on the ground what came off it
        self.return_what_the_living_and_the_dead_leave();

        // And feel about whatever is standing in the way
        self.feel_about_what_stands_in_the_way();
        self.square_up_to_the_people_i_resent();

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

            if let (Some(name), Some(result)) = (tree_name.as_ref(), execution_result.as_ref()) {
                debug!("Agent {} - Executed tree: {} -> {:?}", agent_id, name, result);
            }

            // Action selection runs whether or not a behavior tree matched the
            // agent's most urgent drive. The tree is only consulted for
            // learning; gating the whole pipeline on it meant an agent whose
            // most urgent drive had no tree (or whose tree had been pruned)
            // stopped acting entirely and stood still until it died.
            {
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

                // Generate action based on priority:
                // starvation > emotions > shelter > percepts > plan > goals > drives
                //
                // Running away comes out as an ordinary Move, so without a
                // note of why it was chosen it is invisible in the tally
                let mut running_away = false;
                let (action, is_plan_action) = {
                    let agent = &self.population.agents[agent_index];

                    // PRIORITY -1: An agent already starving eats before it does
                    // anything else, including running from a threat.
                    if let Some(survival_action) =
                        self.survival_action(agent, agent_position, true)
                    {
                        debug!(
                            "Agent {} is starving at {:?}, survival action: {:?}",
                            agent_id, agent_position, survival_action
                        );
                        (survival_action, false)
                    }
                    // PRIORITY 0: Check emotional overrides (fear/anger from
                    // what is in front of the agent, or from being attacked)
                    else if agent.emotions.should_flee() {
                        // Frightened of something that is actually there: put
                        // ground between the two of you. This is the branch
                        // the appraisal feeds; the attacker branches below it
                        // are for agents, who are not creatures.
                        if let Some(away) = self
                            .run_from_what_frightens_me(agent, agent_position)
                            .or_else(|| {
                                self.run_from_whoever_frightens_me(agent, agent_position)
                            })
                        {
                            debug!(
                                "Agent {} RUNNING from {:?} (fear={:.2})",
                                agent_id,
                                agent.emotions.what_frightens_me_most().map(|(k, _)| k),
                                agent.emotions.fear
                            );
                            running_away = true;
                            (away, false)
                        }
                        // High fear - flee from attacker or danger
                        else if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
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
                            self.generate_non_emotional_action(agent, agent_position)
                        }
                    } else if agent.emotions.should_attack() {
                        // Angry at something within arm's reach: turn on it.
                        // An angry agent stands its ground - it does not walk
                        // across the map after a wolf it can see, which is
                        // what keeps this from eating a settlement's whole day.
                        if let Some(strike) = self
                            .round_on_whoever_angers_me(agent, agent_position)
                            .or_else(|| self.round_on_what_angers_me(agent, agent_position))
                        {
                            debug!(
                                "Agent {} STANDING GROUND against {:?} (anger={:.2})",
                                agent_id,
                                agent.emotions.what_angers_me_most().map(|(k, _)| k),
                                agent.emotions.anger
                            );
                            (strike, false)
                        }
                        // High anger, low fear - retaliate against attacker
                        else if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
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
                            self.generate_non_emotional_action(agent, agent_position)
                        }
                    } else {
                        self.generate_non_emotional_action(agent, agent_position)
                    }
                };

                // Resolve nil UUIDs in social actions to actual nearby agents
                let action = Self::resolve_action_target(
                    action,
                    agent_id,
                    agent_position,
                    &agent_positions,
                );

                // Drop travel plans toward places the agent cannot reach
                let action = self.retarget_unreachable_move(agent_index, action);

                // What the settlement spends its days doing
                let did = if running_away {
                    "Flee".to_string()
                } else {
                    Self::name_of(&action)
                };
                *self.actions_taken.entry(did).or_insert(0) += 1;

                // Execute action in environment and get feedback
                let action_result = self.execute_action(&action, agent_index);

                if !action_result.success {
                    *self
                        .actions_failed
                        .entry(Self::name_of(&action))
                        .or_insert(0) += 1;
                    if let Some(why) = action_result.message.as_ref() {
                        *self
                            .actions_failed_because
                            .entry(format!("{}: {}", Self::name_of(&action), why))
                            .or_insert(0) += 1;
                    }
                }

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

                // And note how it went, so the agent does more of what pays
                // and less of what does not
                agent.learn_from(&action, action_result.success);

                // Then join the doing to the need it answered and the ground
                // it was answered on, which is what lets a thirsty man walk
                // back to the bank he drank from yesterday
                let where_it_was = agent.state.position;
                let now = self.current_tick;
                agent.link_what_worked(&action, &action_result, drive_type, where_it_was, now);

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

        // Lies are found out by walking to the place - see the sight pass in
        // `Population::process_exploration_with_world`.
        //
        // There used to be a second path here: a sweep every hundred ticks
        // over remembered claims, checking each with `verify_resource_claim`.
        // That reads the agent's own map as though it were ground truth, and
        // an agent's map holds what it has been told as well as what it has
        // seen, so the check confirmed hearsay against itself. Measured with
        // lying switched off entirely, it still accused every agent of being
        // a proven liar to twenty-seven others - every one of those
        // accusations false, and none of them from the sight pass, which
        // fired not once. It is retired rather than repaired: standing on the
        // spot is the honest test and the sweep cannot be made into one.
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
            Action::Cook { .. } => Some(ActionType::Cooking),
            Action::MakeClothing { .. } => Some(ActionType::Crafting),
            Action::TillSoil => Some(ActionType::Farming),
            Action::TendField => Some(ActionType::Farming),
            Action::Examine { .. } => Some(ActionType::Crafting),
            Action::PickUp { .. } | Action::PutDown { .. } => Some(ActionType::Mining),
            Action::Trade { .. } | Action::GiveTo { .. } => Some(ActionType::Social),
            Action::Work { .. } => Some(ActionType::Crafting),
            Action::Taste => Some(ActionType::Farming),
            Action::TrySwapping { .. } => Some(ActionType::Crafting),
            Action::TakeCutting => Some(ActionType::Farming),
            Action::PlantCutting => Some(ActionType::Farming),
            Action::SpreadMuck => Some(ActionType::Farming),
            Action::Fish => Some(ActionType::Mining), // Taking something off the world
            Action::LightFire => Some(ActionType::Cooking), // Getting a fire going is half of cooking
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
            DriveType::Shelter => Action::Gather { resource_type: "hides".to_string() },
            // Building with nothing to build from is the commonest wasted
            // turn in the model - the drive path above checks the pack first.
            // A trip out for timber is what this falls back to.
            DriveType::Construction => Action::Gather { resource_type: "wood".to_string() },
            DriveType::Industry => Action::Gather { resource_type: "generic".to_string() },
            // Answered by going to the children, which needs to know where
            // they are - see `protective_action`. On its own it comes to
            // waiting where they last were.
            DriveType::Protection => Action::Wait,
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
            DriveType::Utility => Action::Craft { item_type: "spear".to_string() },
            // Putting something by needs something to put by, which this
            // ladder cannot see - the drive path above names a real thing out
            // of the agent's own pack. A trip out is the honest fallback.
            DriveType::Preparedness => Action::Gather { resource_type: "wood".to_string() },
            DriveType::Sustenance => Action::Gather { resource_type: "food".to_string() },
            DriveType::Safety => {
                // Move to a random nearby safe location
                let target_x = position.0 + rng.gen_range(-5..=5);
                let target_y = position.1 + rng.gen_range(-5..=5);
                Action::Move { target: (target_x, target_y, position.2) }
            },
            // Proposing to whoever is nearest is how Mate came to be a fifth
            // of everything a settlement did and to fail 99.9% of the time.
            // The drive path above finds somebody who could actually have a
            // child and whom this agent trusts.
            DriveType::Reproduction => Action::Wait,
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
                    Some(Action::Craft { item_type: "spear".to_string() })
                },
            }
        } else {
            // Goal has neither internal nor external set (shouldn't happen)
            None
        }
    }

    /// Action an agent needs to take to stay alive, if any.
    ///
    /// Nothing else in the decision pipeline satisfies hunger or exhaustion:
    /// drives sit below goals and percepts, so a long-running goal (stocking a
    /// house with food, say) or a steady stream of resource percepts keeps
    /// winning the tie until the agent starves holding a full inventory.
    ///
    /// With `critical_only` this reports only what an agent already dying of
    /// hunger must do, which is the one thing urgent enough to outrank fleeing
    /// a threat. Fear can stay pinned for hundreds of ticks with no attacker
    /// left to run from, and an agent that flees until it starves has not
    /// survived either.
    fn survival_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        critical_only: bool,
    ) -> Option<Action> {
        let thirsty = agent
            .drives
            .get(DriveType::Thirst)
            .map(|thirst| thirst.is_active())
            .unwrap_or(false);
        let dehydrated = agent.state.is_dehydrated();

        let hungry = agent
            .drives
            .get(DriveType::Hunger)
            .map(|hunger| hunger.is_active())
            .unwrap_or(false);
        let starving = agent.state.is_starving() || agent.nutrition.is_starving();

        if critical_only && !(starving || dehydrated) {
            return None;
        }

        // Water before food: thirst kills in about three days here where
        // hunger takes seven, so a parched agent drinks first.
        if thirsty || dehydrated {
            if let Some(action) = self.water_action(agent, agent_position, dehydrated) {
                return Some(action);
            }
        }

        if hungry || starving {
            if let Some(action) = self.food_action(agent, agent_position, starving) {
                return Some(action);
            }
        }

        // Collapse-level fatigue takes precedence over everything but hunger
        if agent.fatigue.desperately_needs_sleep() && !agent.fatigue.is_sleeping {
            return Some(Action::Sleep { duration: 10 });
        }

        None
    }

    /// How a thirsty agent gets a drink, if it can.
    ///
    /// `desperate` marks an agent far enough gone that finding water is worth
    /// abandoning whatever else it was doing for.
    fn water_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        desperate: bool,
    ) -> Option<Action> {
        use crate::agents::senses::ScentType;
        use crate::core::memory::SpatialMemoryType;
        use crate::world::ResourceType;

        // A drink from the waterskin, or from a spring within reach. Both go
        // through the same action: it prefers open water and falls back to
        // whatever the agent is carrying.
        // Enough for a swallow, not a dribble. `Gather` drinks from a
        // waterskin when there is no source about, but only in whole units -
        // so an agent with half a mouthful left kept choosing to drink and
        // being told there was no water anywhere, which was the largest single
        // failure in the simulation.
        let carrying_water = agent.inventory.available_water() >= 1.0;
        let water_in_reach = self
            .nearest_resource_within(agent_position, Self::FORAGE_RADIUS, |resource| {
                resource.resource_type == ResourceType::Water
            })
            .is_some();

        if carrying_water || water_in_reach {
            return Some(Action::Gather { resource_type: "water".to_string() });
        }

        // Otherwise head for water the agent can smell or remembers
        if let Some(target) = self.known_source_position(
            agent,
            agent_position,
            ScentType::Water,
            SpatialMemoryType::Water,
        ) {
            let distance =
                (target.0 - agent_position.0).abs() + (target.1 - agent_position.1).abs();

            if distance > 1 {
                return Some(Action::Move { target });
            }

            return Some(Action::Gather { resource_type: "water".to_string() });
        }

        // Nowhere known to drink by sight or smell, but somewhere that has
        // answered this before.
        //
        // This was aimed at `Gather: No water sources nearby`, the largest
        // single failure in the simulation, on the theory that nothing joined
        // the drink an agent had yesterday to the bank it drank from. Measured
        // over eight worlds a side it did not move that failure at all - the
        // rate is 3.7% of all actions without this and 4.7% with it, which is
        // noise - so whatever is producing those refusals is not an agent
        // being unable to remember where water is. See ISSUES_FOUND #2. It is
        // kept because it is the answer of last resort before striking out
        // blind, and because it costs nothing.
        if let Some(there) =
            agent.somewhere_that_answered(DriveType::Thirst, agent_position, self.current_tick)
        {
            return Some(Action::Move { target: there });
        }

        // Nowhere known to drink: go looking, if it has come to that
        if desperate {
            return Some(Self::search_leg(agent, agent_position, self.current_tick));
        }

        None
    }

    /// How a hungry agent gets a meal, if it can.
    ///
    /// `desperate` marks an agent starving badly enough that finding food is
    /// worth abandoning whatever else it was doing for.
    fn food_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        desperate: bool,
    ) -> Option<Action> {
        use crate::agents::senses::ScentType;
        use crate::core::memory::SpatialMemoryType;

        let carrying_food = agent.has_edible_food();

        // A fire right here turns a third of what is in raw meat into nearly
        // all of it, so one tick spent cooking buys back several meals' worth.
        // Not when starving: then the difference between a poor meal now and a
        // good one next tick is the difference between eating and dying.
        if !desperate
            && Self::has_food_worth_cooking(agent)
            && self
                .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                .is_some()
        {
            return Some(Action::Cook {
                food_type: "generic".to_string(),
            });
        }

        // Eat what we carry as soon as we are hungry; an agent that walks
        // around starving with a full pack is the bug this guards against.
        if carrying_food {
            return Some(Action::Eat { food_type: "generic".to_string() });
        }

        // Anything edible within foraging reach can simply be eaten
        if self
            .nearest_edible_within(agent_position, Self::FORAGE_RADIUS)
            .is_some()
        {
            return Some(Action::Eat { food_type: "generic".to_string() });
        }

        // Otherwise head for the closest source the agent knows of
        if let Some(target) = self.known_source_position(
            agent,
            agent_position,
            ScentType::Food,
            SpatialMemoryType::Food,
        ) {
            let distance =
                (target.0 - agent_position.0).abs() + (target.1 - agent_position.1).abs();

            // Walk to food we know about before trying to pick anything up
            if distance > 1 {
                return Some(Action::Move { target });
            }

            return Some(Action::Gather { resource_type: "food".to_string() });
        }

        // Hungry for long enough, with the country round about picked bare:
        // go somewhere else. This is above the local search below because
        // walking twelve tiles and back is what an agent does when it has
        // mislaid its dinner, not when the ground has stopped producing one.
        // A need that keeps going unanswered is a reason to live somewhere
        // else, not a reason to walk further today
        if let Some(action) = self.go_and_live_where_it_is(agent, agent_position) {
            return Some(action);
        }

        if let Some(action) = self.migration_action(agent, agent_position) {
            return Some(action);
        }

        // Ground that has fed this agent before, when nothing nearer will.
        if let Some(there) =
            agent.somewhere_that_answered(DriveType::Hunger, agent_position, self.current_tick)
        {
            return Some(Action::Move { target: there });
        }

        // Starving with nowhere known to go: search rather than stand still
        // and wait to die. Agents that are merely hungry let the tick go to
        // whatever comes next - sheltering from the cold, a plan, a goal -
        // because gathering thin air on the spot accomplishes nothing and
        // blocks everything they could usefully be doing.
        if desperate {
            // An animal is food. Hunting does not pay against berries and
            // fish, which is why an agent does not do it for the meat as a
            // rule - but an agent with nothing else left is a different case.
            if let Some((animal_id, animal_position)) =
                self.nearest_prey(agent, agent_position)
            {
                let reach = (animal_position.0 - agent_position.0)
                    .abs()
                    .max((animal_position.1 - agent_position.1).abs());

                if reach <= Self::HUNT_REACH {
                    return Some(Action::Hunt {
                        animal_id,
                        weapon: agent
                            .equipment
                            .get_weapon()
                            .map(|weapon| weapon.name.clone()),
                    });
                }

                return Some(Action::Move {
                    target: (animal_position.0, animal_position.1, agent_position.2),
                });
            }

            return Some(Self::search_leg(agent, agent_position, self.current_tick));
        }

        None
    }

    /// A place the agent knows to look: what it can smell right now, falling
    /// back to the nearest place it remembers.
    ///
    /// Scent wins because it is current, where a memory may be of a patch
    /// already eaten bare. Scent also carries by straight-line distance while
    /// walking is counted in steps, so what an agent smells can still be a
    /// journey away - which is why this reports somewhere to go rather than
    /// somewhere to reach for.
    fn known_source_position(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        scent_type: crate::agents::senses::ScentType,
        memory_type: crate::core::memory::SpatialMemoryType,
    ) -> Option<(i32, i32, i32)> {
        let walking_distance = |candidate: &(i32, i32, i32)| {
            (candidate.0 - agent_position.0).abs() + (candidate.1 - agent_position.1).abs()
        };

        let smelled = agent
            .senses
            .smell
            .get_scents_by_type(scent_type)
            .into_iter()
            .map(|scent| scent.source_position)
            .min_by_key(walking_distance);

        smelled.or_else(|| {
            agent
                .memory
                .recall_locations(memory_type)
                .into_iter()
                .map(|memory| memory.position)
                .min_by_key(walking_distance)
        })
    }

    /// One leg of a search for something the agent cannot find nearby.
    ///
    /// The heading holds for a stretch of ticks: re-rolling it every tick
    /// produces a random walk that barely leaves the spot it started from, so
    /// an agent would jitter in place while what it needed sat just outside
    /// its range. It varies per agent and per leg, so agents setting out from
    /// the same place fan out instead of marching together.
    /// How long hunger has to go unanswered before an agent gives up on the
    /// country it is standing in.
    ///
    /// Ten days of the world's calendar of being hungry and not being fed.
    /// Short enough that a settlement whose ground has stopped producing does
    /// something about it; long enough that a bad afternoon, a hard winter or
    /// one picked-over hedgerow does not empty a village.
    const HUNGRY_ENOUGH_TO_LEAVE: u32 = 120;

    /// How far off counts as somewhere else rather than the next field over.
    const FAR_ENOUGH_TO_BE_WORTH_THE_WALK: i32 = 20;

    /// Leaving: what an agent does when the ground where it lives has stopped
    /// feeding it.
    ///
    /// Nobody decides this on the settlement's behalf. It falls out of the
    /// drive: hunger that keeps being denied presses harder every tick it
    /// waits, and past a certain point the agent stops working the fields it
    /// has and walks. Where it walks to is the best thing it can remember
    /// that is far enough away to be different country; failing any memory,
    /// it strikes out on a bearing of its own and keeps going, which is what
    /// turns a starving settlement into a scattering one.

    /// How long a need has to keep going unanswered before an agent stops
    /// walking back and forth to it and goes to live beside it.
    const ASKED_FOR_IT_ONCE_TOO_OFTEN: u32 = 96;

    /// How near counts as camped on a thing.
    ///
    /// Wide enough that a people spread out along a river rather than piling
    /// onto the one tile of it. At four tiles they concentrated hard enough to
    /// work the ground out under themselves: the nutrient-loop regression,
    /// which asks that farmed ground not lose half its fertility in ten
    /// thousand ticks, started failing about one run in three.
    const CAMPED_ON_IT: i32 = 4;

    /// The need this agent keeps having and keeps not getting.
    ///
    /// `denied_ticks` counts how long a drive has gone unanswered, and until
    /// now only hunger was ever read for the purpose of moving house. Thirst
    /// was the largest single failure in the whole simulation - a hundred and
    /// thirty-one thousand refusals of `Gather: No water sources nearby` in
    /// one pair of worlds - because an agent that could not find water walked
    /// to it, drank, wandered off about its business, and was thirsty again
    /// half a day later in the same dry place.
    fn what_i_keep_going_short_of(agent: &crate::agents::Agent) -> Option<DriveType> {
        // Water only, and deliberately.
        //
        // Water is a fixed point on the map: it is in one place, it does not
        // run out, and camping beside it answers the need for good. Food is
        // not - it is spread about and it is *consumed*, so a people who move
        // house towards it concentrate their foraging on whatever ground they
        // land on and work it out from under themselves. Measured, letting
        // hunger move a settlement took the nutrient-loop regression from
        // passing three times in three to twice in five: farmed ground losing
        // more than half its fertility inside ten thousand ticks.
        //
        // Ranging for food and settling by water is the division the land
        // itself makes.
        [DriveType::Thirst]
            .into_iter()
            .filter(|need| {
                agent
                    .drives
                    .get(*need)
                    .map(|drive| drive.denied_ticks() >= Self::ASKED_FOR_IT_ONCE_TOO_OFTEN)
                    .unwrap_or(false)
            })
            .max_by_key(|need| {
                agent
                    .drives
                    .get(*need)
                    .map(|drive| drive.denied_ticks())
                    .unwrap_or(0)
            })
    }

    /// Go and live where the thing you keep needing is.
    ///
    /// "The agents must anticipate their future drive demands. If they
    /// consistently need water, they should camp or colonize near water."
    ///
    /// Answering a need where you stand is what every other path here does.
    /// This is the one that reads the *pattern* of a need instead of the need
    /// itself: a man who has been short of water for eight days does not want
    /// a drink, he wants to be somewhere else.
    ///
    /// It fires only once a need has been going unanswered for days, and stops
    /// the moment the agent is camped on the answer, so it moves a settlement
    /// rather than keeping it walking.
    fn go_and_live_where_it_is(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::ResourceType;

        // Whether to move house is a question worth asking once a day, not
        // eight times: it walks the whole resource list at sixty tiles, and a
        // people do not reconsider where they live every two hours.
        if self.current_tick % crate::environment::seasons::TICKS_PER_DAY != 0 {
            return None;
        }

        let short_of = Self::what_i_keep_going_short_of(agent)?;

        let wanted = match short_of {
            DriveType::Thirst => ResourceType::Water,
            _ => ResourceType::Food,
        };

        // The nearest place that answers it, however far off - this is a
        // decision to move house, so the ordinary foraging radius does not
        // apply
        let there = self.nearest_resource_within(agent_position, Self::HOW_FAR_A_PEOPLE_WILL_MOVE, |resource| {
            resource.resource_type == wanted
                || (wanted == ResourceType::Food
                    && Self::edible_item_for(resource.resource_type).is_some())
        })?;

        let paces = (there.x - agent_position.0)
            .abs()
            .max((there.y - agent_position.1).abs());

        // Already living on it: nothing to do, and importantly nothing that
        // keeps the agent walking in circles round the thing it wanted
        if paces <= Self::CAMPED_ON_IT {
            return None;
        }

        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// How far a people will pick up and move for water they can count on.
    const HOW_FAR_A_PEOPLE_WILL_MOVE: u32 = 60;

    /// What one person wants standing within reach of the camp before the
    /// ground counts as feeding them.
    ///
    /// Wild food regrows about four times slower than a settlement eats it, so
    /// a camp of any size strips its own ground and the number here is what
    /// "stripped" means. A nomad moves while there is still something to eat,
    /// because a nomad that waits until there is nothing has to walk on an
    /// empty stomach.
    ///
    /// The first cut of this was 25 a head, which is about what a person
    /// eats in a season and reads as the right number until you notice that
    /// no ground anywhere in the world carries that much for a grown
    /// settlement. It fired every tick of every life. Over eight worlds
    /// foraging fell forty per cent, the food standing on the map went up
    /// four and a half times because nobody was eating it, the camp did not
    /// end up any further from where it started, and it cost about twelve
    /// people.
    const WHAT_A_CAMP_WANTS_STANDING: u32 = 4;

    /// And how much better somewhere else has to be before it is worth
    /// picking the camp up.
    ///
    /// This is the half that stops the walking. An absolute standard for good
    /// ground is a standard nowhere meets, so a camp held to one walks for
    /// ever; a camp that moves because somewhere is three times better stops
    /// the moment it gets there, because it is now standing on the best ground
    /// it knows of.
    const WORTH_PICKING_THE_CAMP_UP_FOR: u32 = 3;

    /// Moving camp, for a people that has no other way of making food happen.
    ///
    /// "Until there is a method of producing food through farming, the agents
    /// should likely stick to a nomadic way of life."
    ///
    /// This is the Sustenance answer for anybody who cannot farm: you cannot
    /// make this ground carry more, so you go where the ground already does.
    /// An agent that has worked farming out does not do this - a field is a
    /// reason to stay, and the whole of what settling down is.
    ///
    /// It is not the same thing as `migration_action`, which fires on an agent
    /// that has already been going hungry for a hundred and twenty ticks. This
    /// fires while there is still food here, on the strength of there not
    /// being much of it, which is the difference between moving camp and
    /// fleeing.
    fn moving_on(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Practice;

        // A farmer stays. So does anybody standing beside a field with
        // something in it, farmer or not: whatever is growing there is a
        // better answer than a fortnight's walk.
        if agent.practices.is_established(Practice::Farming) {
            return None;
        }

        if self.crop_standing_on_fields_within(agent_position, Self::FIELD_WALK_RADIUS) > 0 {
            return None;
        }

        // Enough hands here to strip the place, and enough standing to feed
        // them. Both are counted within the distance somebody actually walks
        // to forage.
        let mouths = self.how_many_camped_within(agent_position, Self::FORAGE_RADIUS);
        let standing = self.edible_standing_within(agent_position, Self::FORAGE_RADIUS);

        if standing >= mouths * Self::WHAT_A_CAMP_WANTS_STANDING {
            return None;
        }

        // Somewhere better, far enough off to be a move rather than a stroll.
        // The best ground within the distance a people will shift for, not the
        // nearest: this is a decision about where to spend a season.
        let here = crate::world::Position::new(agent_position.0, agent_position.1);

        let (there, carrying) = self
            .world
            .resources
            .iter()
            .filter(|resource| resource.amount > 0)
            .filter(|resource| Self::edible_item_for(resource.resource_type).is_some())
            .map(|resource| (resource.position, here.distance_to(&resource.position), resource.amount))
            .filter(|(_, distance, _)| {
                *distance >= Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK as u32
                    && *distance <= Self::HOW_FAR_A_PEOPLE_WILL_MOVE
            })
            .max_by_key(|(_, _, amount)| *amount)
            .map(|(where_it_is, _, amount)| (where_it_is, amount))?;

        // And it has to be worth the walk. Without this the camp sets out for
        // whatever is furthest, arrives, finds the same thin ground, and sets
        // out again: it walks for ever and forages a great deal less than a
        // people that stayed put.
        if carrying < standing.max(1) * Self::WORTH_PICKING_THE_CAMP_UP_FOR {
            return None;
        }

        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// How many people are living within reach of this spot
    fn how_many_camped_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let reach = radius as i32;

        self.population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                (agent.state.position.0 - position.0).abs() <= reach
                    && (agent.state.position.1 - position.1).abs() <= reach
            })
            .count() as u32
    }

    /// How much there is to eat standing within reach of this spot
    fn edible_standing_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let here = crate::world::Position::new(position.0, position.1);

        self.world
            .resources
            .iter()
            .filter(|resource| Self::edible_item_for(resource.resource_type).is_some())
            .filter(|resource| here.distance_to(&resource.position) <= radius)
            .map(|resource| resource.amount)
            .sum()
    }

    /// And how much of that is standing on ground somebody has broken
    fn crop_standing_on_fields_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let here = crate::world::Position::new(position.0, position.1);

        self.world
            .resources
            .iter()
            .filter(|resource| here.distance_to(&resource.position) <= radius)
            .filter(|resource| {
                self.world
                    .grid
                    .get_tile(&resource.position)
                    .map(|tile| tile.terrain.is_cultivated())
                    .unwrap_or(false)
            })
            .map(|resource| resource.amount)
            .sum()
    }

    fn migration_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::core::memory::SpatialMemoryType;

        let starved_for = agent
            .drives
            .get(DriveType::Hunger)
            .map(|hunger| hunger.denied_ticks())
            .unwrap_or(0);

        if starved_for < Self::HUNGRY_ENOUGH_TO_LEAVE {
            return None;
        }

        let far_off = |candidate: &(i32, i32, i32)| {
            (candidate.0 - agent_position.0)
                .abs()
                .max((candidate.1 - agent_position.1).abs())
        };

        // Somewhere it remembers food that is not on this doorstep. Anything
        // near enough to walk to in the ordinary way has already been tried by
        // the code above, and found wanting.
        let remembered = agent
            .memory
            .spatial_memories
            .iter()
            .filter(|memory| matches!(memory.memory_type, SpatialMemoryType::Food))
            .map(|memory| (memory.position.0, memory.position.1, agent_position.2))
            .filter(|candidate| far_off(candidate) >= Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK)
            .max_by_key(far_off);

        if let Some(target) = remembered {
            return Some(Action::Move { target });
        }

        // Nothing remembered worth the walk: pick a bearing and hold it. The
        // bearing comes from the agent rather than the tick, so somebody who
        // sets out keeps going the same way instead of milling about, and two
        // people leaving the same place do not necessarily leave together.
        let bearings = [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ];
        let (dx, dy) = bearings[(agent.id.as_u128() % bearings.len() as u128) as usize];

        let target = (
            (agent_position.0 + dx * Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK)
                .clamp(0, self.world.grid.width as i32 - 1),
            (agent_position.1 + dy * Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK)
                .clamp(0, self.world.grid.height as i32 - 1),
            agent_position.2,
        );

        // Already hard against that edge: there is nowhere further this way
        if target.0 == agent_position.0 && target.1 == agent_position.1 {
            return None;
        }

        Some(Action::Move { target })
    }

    fn search_leg(
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        current_tick: u32,
    ) -> Action {
        const SEARCH_LEG_TICKS: u32 = 300;
        const SEARCH_LEG_DISTANCE: i32 = 12;

        let directions = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        let leg = (current_tick / SEARCH_LEG_TICKS) as u64;
        let seed = (agent.id.as_u128() as u64) ^ leg.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let (dx, dy) = directions[(seed % directions.len() as u64) as usize];

        Action::Move {
            target: (
                agent_position.0 + dx * SEARCH_LEG_DISTANCE,
                agent_position.1 + dy * SEARCH_LEG_DISTANCE,
                agent_position.2,
            ),
        }
    }

    /// What an agent does when nothing has frightened it, in order:
    ///
    /// 1. stay alive - eat, drink, sleep
    /// 2. put on or make clothing, if it can be done where it stands
    /// 3. get out of the weather
    /// 4. cook what it is carrying
    /// 5. go after an animal, for the meat or the skin
    /// 6. go and get the material to clothe itself
    /// 7. act on something it can see or smell
    /// 8. carry on with a plan
    /// 9. work towards a goal
    /// 10. whatever its most pressing drive suggests
    /// What an agent does, when nothing has frightened or angered it.
    ///
    /// This used to be thirteen fixed priorities with the drives consulted at
    /// the thirteenth, which meant the drives decided almost nothing: seventy-
    /// nine per cent of everything a settlement did was foraging chosen off
    /// the ladder before a drive was ever asked, and `Action::Build` and
    /// `Action::Socialize` were chosen zero times in seven hundred and
    /// seventy-seven agent-lives. Giving every agent a personality changed
    /// nothing measurable for the same reason: personality reaches the drives,
    /// and the drives reached nothing.
    ///
    /// It is the other way round now. Two things pre-empt, because they are
    /// emergencies rather than wants; after that the needs are ranked by how
    /// hard each is pressing - see `Agent::how_hard_it_presses` - and the
    /// first that can actually be answered here and now takes the turn.
    fn generate_non_emotional_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> (Action, bool) {
        // A child of one's own in trouble. Not a want: a parent goes, and the
        // Protection *drive* underneath is a tertiary disposition that waits
        // its turn like anything else.
        if let Some(action) = self.protective_action(agent, agent_position) {
            return (action, false);
        }

        // And freezing, where there is a roof within reach. Exposure is
        // already doing damage by the time this fires, so it is not a matter
        // of how much the agent wants to be warm.
        if agent.needs_shelter() && self.nearest_shelter_from(agent_position).is_some() {
            return (Action::SeekShelter, false);
        }

        // Everything else the agent wants, in the order it wants it
        let mut ranked: Vec<(DriveType, f32)> = DriveType::all()
            .into_iter()
            .map(|drive_type| (drive_type, agent.how_hard_it_presses(drive_type)))
            .filter(|(_, pressing)| *pressing > 0.0)
            .collect();

        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (drive_type, _) in ranked {
            if let Some(action) = self.how_this_agent_answers(drive_type, agent, agent_position) {
                return (action, false);
            }
        }

        // Nothing is pressing and nothing is wanted, which is when an agent
        // gets to follow its own nose: what it has just noticed, then what it
        // had planned, then what it was working towards.
        let recent_percepts: Vec<(u32, crate::agents::sensory_processing::Percept)> =
            agent.recent_percepts.iter().cloned().collect();

        if let Some(percept_action) = Self::generate_action_from_percepts(
            &recent_percepts,
            &agent.drives,
            agent_position,
        ) {
            return (percept_action, false);
        }

        if agent.should_execute_plan() {
            if let Some(plan_action) = agent.get_plan_action() {
                return (plan_action, true);
            }
        }

        if let Some(goal) = agent.goals.highest_priority_goal() {
            let fallback_drive = agent
                .what_presses_hardest()
                .unwrap_or(DriveType::Curiosity);

            if let Some(goal_action) =
                Self::generate_action_for_goal(&goal, agent_position, fallback_drive)
            {
                return (goal_action, false);
            }
        }

        (Action::Wait, false)
    }

    /// How this agent would go about answering that need, if it can at all.
    ///
    /// `None` means this particular need has no answer available here and now -
    /// a wish to build with nowhere to build, a wish for company with nobody
    /// about - and the turn passes to whatever is pressing next. That is the
    /// part the old ladder could not do: it had one fixed order for everybody
    /// and no way for a need to stand aside.
    fn how_this_agent_answers(
        &self,
        drive_type: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        // "When an action fails to satisfy a drive, its odds of repeating
        // should decrease. Inversely, when an action satisfies a drive, its
        // odds of repeating should increase."
        //
        // Every attempt has been recorded against the particular thing tried
        // since `Lessons` was written, and nothing but hunting ever read it
        // back. So a settlement that could not put a roof up went on trying
        // to for fifteen thousand ticks, and one whose thirsty men were
        // nowhere near water asked for it a hundred and thirty thousand times.
        //
        // A drive that offers something this agent has learned does not work
        // stands aside and lets the next drive have the turn. It is a
        // slackening rather than a ban - see `Lessons::NEVER_QUITE_GIVES_UP` -
        // so a man who has failed at something forty times still tries it now
        // and again, which is how he finds out the world has changed.
        let answer = self.what_this_drive_offers(drive_type, agent, agent_position)?;

        if agent
            .lessons
            .will_try_this_again(&crate::agents::Agent::what_was_tried(&answer))
        {
            Some(answer)
        } else {
            None
        }
    }

    fn what_this_drive_offers(
        &self,
        drive_type: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        match drive_type {
            // Water first of the two, always, because it runs out first - but
            // that is now decided by the clocks in `how_hard_it_presses`
            // rather than written down here
            DriveType::Thirst => {
                self.water_action(agent, agent_position, agent.state.is_dehydrated())
            }

            // Eat what is carried, go and get what is not, and failing both
            // stand in a river or go after an animal
            DriveType::Hunger => {
                let starving = agent.state.is_starving() || agent.nutrition.is_starving();
                self.food_action(agent, agent_position, starving)
                    .or_else(|| self.fishing_action(agent, agent_position))
                    .or_else(|| self.hunting_action(agent, agent_position))
            }

            DriveType::Rest => {
                if agent.fatigue.is_sleeping {
                    None
                } else if let Some(clean) = self.somewhere_that_does_not_stink(agent_position) {
                    // "Waste should smell unpleasant and repulse the agents."
                    // Nobody lies down in it. This is the repulsion: a man who
                    // wants to sleep and is standing on a midden moves off it
                    // first, which over a settlement's life is what puts the
                    // midden at the edge of the camp rather than in it.
                    Some(Action::Move { target: clean })
                } else {
                    Some(Action::Sleep { duration: 10 })
                }
            }

            // Being out of harm's way, and only while there is harm about
            DriveType::Safety => {
                let threatened = agent.surroundings.predator_near
                    || agent.surroundings.recently_hurt;

                if threatened && self.nearest_shelter_from(agent_position).is_some() {
                    Some(Action::SeekShelter)
                } else {
                    None
                }
            }

            // Everything that puts food on next year's table
            DriveType::Sustenance => self
                .cooking_action(agent, agent_position)
                .or_else(|| self.muck_action(agent, agent_position))
                .or_else(|| self.farming_action(agent, agent_position))
                .or_else(|| self.transplanting_action(agent, agent_position))
                .or_else(|| self.moving_on(agent, agent_position))
                .or_else(|| self.fishing_action(agent, agent_position)),

            // A coat is shelter you carry, and it comes before walking to a
            // roof: an agent that goes indoors every time it feels the wind
            // never gets around to dressing itself. Walking to a roof is worth
            // a turn only when the weather is actually doing something.
            DriveType::Shelter => self
                .clothing_action(agent, agent_position, true)
                .or_else(|| {
                    let worth_going_in = agent.needs_shelter()
                        || agent.body_temperature.is_too_cold()
                        || agent.surroundings.foul_weather;

                    if worth_going_in && !agent.surroundings.under_shelter {
                        self.nearest_shelter_from(agent_position)
                            .map(|_| Action::SeekShelter)
                    } else {
                        None
                    }
                })
                .or_else(|| self.clothing_action(agent, agent_position, false)),

            // A roof, when there is anything to make one from
            DriveType::Construction => self.raising_a_roof(agent, agent_position),

            // Looking after somebody of your own
            DriveType::Protection => self.protective_action(agent, agent_position),

            // Children, and only when this agent could actually have one and
            // expects to be able to feed it. Without this an agent proposes to
            // the empty air a third of every life - it was the single
            // commonest thing anybody did.
            DriveType::Reproduction => {
                if !agent.should_attempt_reproduction() {
                    return None;
                }
                self.somebody_to_have_a_child_with(agent, agent_position)
                    .map(|them| Action::Mate {
                        target_agent_id: them,
                    })
            }

            // Making a thing needs something to make it out of, and a step
            // that the makings in the pack will actually carry. Asking for a
            // wooden axe was asking for a technology these people have not
            // got: every one of those turns came back
            // `missing technology 'wooden_tools'`.
            // Reducing first, then assembling: a core has to be broken before
            // there is a flake to haft. What each of these wants in the hand
            // is the matrix's business and is enforced before it runs.
            DriveType::Utility => agent
                .what_i_would_work_on()
                .map(|(verb, to)| Action::Work { verb, to })
                .or_else(|| agent.what_i_would_make().map(|item_type| Action::Craft { item_type }))
                .or_else(|| {
                    // Somebody standing here with the thing, who wants what is
                    // going spare. A trade is quicker than a walk.
                    self.somebody_to_trade_with(agent, agent_position)
                        .map(|with| Action::Trade { with })
                })
                // And a thing already lying on the ground is quicker than
                // either: stooping is where scavenging belongs, beside going
                // out to fetch a thing rather than ahead of making one out of
                // what is already in the pack.
                .or_else(|| self.something_worth_stooping_for(agent, agent_position))
                .or_else(|| {
                    agent
                        .what_i_must_find()
                        .map(|resource_type| Action::Gather { resource_type })
                }),

            // Putting something by needs something to put by
            DriveType::Preparedness => {
                if let Some((what, how_many)) = agent.what_i_can_spare() {
                    Some(Action::Store {
                        item_type: what,
                        amount: how_many,
                    })
                } else {
                    None
                }
            }
            // Nothing in the world is fine enough to want yet - see
            // ISSUES_FOUND.md #5. Until something is, this need has no answer
            // and stands aside rather than spending the turn walking after a
            // resource that does not exist.
            DriveType::Luxury => None,

            // A curious man picks up the bright stone he walked past.
            //
            // Nothing else in the model ever puts iron in a pack: no drive
            // asks for it, because nobody yet knows what it is for. It gets
            // picked up because it glitters, which is the only way anybody
            // ever came to be holding one at a fire.
            DriveType::Curiosity => {
                // First, doing again the thing he has just worked out how to
                // do. There is no use in mind for what comes out of it; that
                // is the point. Nobody can want a metal knife until somebody
                // has held a metal blade, and nobody holds one until somebody
                // does the trick a second time for its own sake.
                // Something growing underfoot that nobody has ever tried.
                // This is where a people's larder comes from and where some
                // of its people go: the only way to find out whether a plant
                // is food is for somebody to eat one.
                if let Some(action) = self.tasting_action(agent, agent_position) {
                    return Some(action);
                }

                // Turning over something in the pack that might be for
                // something. Cheaper than any other experiment - it costs the
                // turn and nothing else.
                if let Some(what) = agent.what_i_would_look_at() {
                    return Some(Action::Examine { what });
                }

                // Doing something to a thing to see what it turns into. The
                // cheapest kind of experiment there is: the materials are in
                // the pack and the tool is in the hand either way.
                if let Some((verb, to)) = agent.what_working_i_would_try_out() {
                    return Some(Action::Work { verb, to });
                }

                // And putting the wrong thing where a part goes, which is
                // how a people gets past the things it already knows how to
                // make. Rare, because it costs the materials whether or not
                // anything comes of it.
                let feeling_experimental = {
                    use rand::Rng;
                    rand::thread_rng().gen_bool(Self::HOW_OFTEN_ANYBODY_TRIES_A_SWAP)
                };

                if feeling_experimental {
                    if let Some((instead_of_making, instead_of, put_in)) =
                        agent.what_i_would_swap()
                    {
                        return Some(Action::TrySwapping {
                            instead_of_making,
                            instead_of,
                            put_in,
                        });
                    }
                }

                if let Some(what) = agent.what_i_would_try_out() {
                    // The conditions are checked here and not only in the
                    // executor, because an agent that keeps asking for a thing
                    // it cannot do here learns not to ask for it at all - see
                    // `Lessons::will_try_this_again` - and would give up on
                    // metalworking after a dozen turns spent away from a fire.
                    let wants_a_fire = crate::environment::making::how_to_make(&what)
                        .is_some_and(|step| step.over_a_fire);

                    let can_do_it_here = !wants_a_fire
                        || self
                            .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                            .is_some();

                    if can_do_it_here {
                        return Some(Action::Craft { item_type: what });
                    }
                }

                let agent_has_none = agent.how_many_i_have("iron") < 2;
                if agent_has_none && agent.have_i_seen("iron") {
                    Some(Action::Gather {
                        resource_type: "iron".to_string(),
                    })
                } else {
                    Some(Self::generate_action_for_drive(drive_type, agent_position))
                }
            }

            // Company needs somebody to keep it
            DriveType::Social => {
                if !agent.surroundings.company {
                    return None;
                }

                // Handing somebody something they need is the plainest sociable
                // act there is, and it costs the giver, which is what makes it
                // one. It comes before talking because a gift says more.
                if let Some(to) = self.somebody_to_give_to(agent, agent_position) {
                    return Some(Action::GiveTo { to });
                }

                Some(Self::generate_action_for_drive(drive_type, agent_position))
            }

            // The rest keep the simple mapping they had
            other => Some(Self::generate_action_for_drive(other, agent_position)),
        }
    }


    /// Execute an action in the environment and return the result
    /// Which trade a thing taken off the land belongs to.
    ///
    /// The same split the experience grants already used, in one place, so
    /// that what a hand is worth at a job and what the job teaches it are
    /// never allowed to drift apart.
    fn trade_for_gathering(
        resource_type: crate::world::ResourceType,
    ) -> crate::agents::skills::SkillType {
        use crate::agents::skills::SkillType;
        use crate::world::ResourceType;

        match resource_type {
            ResourceType::Wood => SkillType::Woodcutting,
            ResourceType::Stone | ResourceType::Iron | ResourceType::Coal
            | ResourceType::Clay | ResourceType::Sand => SkillType::Mining,
            ResourceType::Grain => SkillType::Farming,
            ResourceType::Food | ResourceType::Herbs => SkillType::Herbalism,
            ResourceType::Flax | ResourceType::Cotton => SkillType::Farming,
            ResourceType::Fish => SkillType::Fishing,
            _ => SkillType::Herbalism,
        }
    }

    /// How far an agent will walk to reach a resource while foraging, in
    /// walking (Manhattan) distance
    const FORAGE_RADIUS: u32 = 25;

    /// How close an agent has to be to a fire to light it, feed it or cook on
    /// it: near enough to reach into the flames
    const FIRE_REACH: i32 = 1;

    /// How much soft litter one unit of spoiled food amounts to
    const MUCK_PER_UNIT: f32 = 0.12;

    /// And what a spoiled fish is worth, tipped on the same field.
    ///
    /// Several times a turnip, and the difference is not in the size of it.
    /// Everything else in the pack was grown on the settlement's own ground
    /// and is at best going back where it came from. The fish was grown at
    /// sea. It is the only muck a farming people ever get that leaves the
    /// country better off than it found it.
    const MUCK_PER_FISH: f32 = 0.9;

    /// How much a field holds when it is full.
    ///
    /// Wild food regrows about four times slower than a grown settlement eats
    /// it. A handful of fields is what closes that gap: the same patch of
    /// ground yields many times what the hedgerow beside it does.
    const FIELD_YIELD: u32 = 80;

    /// Wood a campfire is built from, matching `HeatSourceType::Campfire`
    const FIRE_BUILD_WOOD: u32 = 5;

    /// Wood put on to burn, worth about fifty ticks at a campfire's rate
    const FIRE_FUEL_WOOD: u32 = 5;

    /// How long food goes on smelling of cooking after it is taken off the
    /// fire, in ticks
    const COOKING_SMELL_TICKS: u32 = 60;

    /// How much food fits over a campfire at once
    const COOK_BATCH: u32 = 5;

    /// How often a cook leaves food on the fire too long, by how practised
    /// they are.
    ///
    /// Deliberately gentler than the generic `SkillCategory::failure_chance`,
    /// which is calibrated for botching an axe: burning a meal is a smaller
    /// and commoner mistake, and a fifty-fifty campfire would make cooking not
    /// worth attempting.
    fn burn_chance(cooking_level: i32) -> f32 {
        match cooking_level {
            level if level <= -6 => 0.20, // has never done it before
            -5..=-1 => 0.10,
            0..=5 => 0.04,
            _ => 0.0, // years of it
        }
    }

    /// What to call food that has come off a fire.
    ///
    /// `id_to_item_type` reads through these prefixes, so cooked fish is still
    /// fish to everything that asks what it is.
    fn prepared_item_id(item_id: &str, cooked_well: bool) -> String {
        let base = crate::agents::storage_integration::base_item_id(item_id);

        if cooked_well {
            format!("cooked_{}", base)
        } else {
            format!("burnt_{}", base)
        }
    }

    /// How often a man in exactly the right position works out what he is
    /// looking at.
    ///
    /// Set so that finding out is a thing that happens to a settlement over
    /// seasons rather than to an individual over an afternoon: a curious agent
    /// with the makings in his hands and a fire in front of him needs of the
    /// order of a hundred turns of standing there.
    const HOW_OFTEN_ANYBODY_WORKS_IT_OUT: f64 = 0.01;

    /// Nobody works anything out while they are frightened or starving.
    const CURIOUS_ENOUGH_TO_NOTICE: f32 = 0.25;

    /// Somebody, somewhere, finds out how to do something new.
    ///
    /// This is the specification's "rock + fire = ?": the outcome of putting
    /// two things together is not apparent until the conditions are right, and
    /// then it is apparent all at once. Nothing here is a plan - an agent
    /// cannot want a metal knife before anybody has seen metal - it is the
    /// accident of standing in the right place holding the right things while
    /// curious enough to be paying attention.
    fn somebody_notices_something(&mut self) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut found: Vec<(usize, &'static str)> = Vec::new();

        for (index, agent) in self.population.agents.iter().enumerate() {
            if !agent.state.is_alive {
                continue;
            }

            let curiosity = agent
                .drives
                .get(crate::core::DriveType::Curiosity)
                .map(|drive| drive.value)
                .unwrap_or(0.0);
            if curiosity < Self::CURIOUS_ENOUGH_TO_NOTICE {
                continue;
            }

            let holding = |what: &str| agent.how_many_i_have(what);

            for step in crate::environment::making::everything_to_find_out() {
                if agent.knows_how_to(step) || !step.makings_to_hand(&holding) {
                    continue;
                }
                if let Some(wanted) = step.wants_in_hand {
                    if agent.how_many_i_have(wanted) == 0 {
                        continue;
                    }
                }
                if step.over_a_fire
                    && self
                        .nearest_fire_from(agent.state.position, Self::FIRE_REACH, true)
                        .is_none()
                {
                    continue;
                }

                // A practised hand notices sooner, because it knows what it
                // is looking at.
                let odds = Self::HOW_OFTEN_ANYBODY_WORKS_IT_OUT
                    * curiosity as f64
                    * agent.skills.hand_for(step.hands) as f64;

                if rng.gen_bool(odds.clamp(0.0, 1.0)) {
                    found.push((index, step.makes));
                    break;
                }
            }
        }

        for (index, what) in found {
            if self.population.agents[index].found_out_how_to(what) {
                debug!(
                    "Agent {} worked out how to make {what}",
                    self.population.agents[index].id
                );
            }
        }
    }

    /// How far a man will walk to get off fouled ground.
    ///
    /// Not far. The point is to step off the midden, not to leave the country.
    const OFF_THE_MIDDEN: i32 = 3;

    /// Clean ground within a step or two, for somebody standing on a midden.
    ///
    /// `None` when the ground underfoot is fine, which is the ordinary case
    /// and costs one lookup.
    fn somewhere_that_does_not_stink(
        &self,
        from: (i32, i32, i32),
    ) -> Option<(i32, i32, i32)> {
        use crate::world::Position;

        let here = Position::new(from.0, from.1);
        let underfoot = self.world.grid.get_tile(&here)?;
        if !underfoot.soil.is_foul() {
            return None;
        }

        let mut best: Option<((i32, i32, i32), f32)> = None;

        for dy in -Self::OFF_THE_MIDDEN..=Self::OFF_THE_MIDDEN {
            for dx in -Self::OFF_THE_MIDDEN..=Self::OFF_THE_MIDDEN {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let there = Position::new(from.0 + dx, from.1 + dy);
                let Some(tile) = self.world.grid.get_tile(&there) else {
                    continue;
                };
                if tile.soil.is_foul() || !tile.terrain.is_walkable() {
                    continue;
                }

                // The nearest clean tile, so that this is a step aside rather
                // than a march.
                let how_far = (dx.abs() + dy.abs()) as f32;
                if best.is_none_or(|(_, best_so_far)| how_far < best_so_far) {
                    best = Some(((there.x, there.y, from.2), how_far));
                }
            }
        }

        best.map(|(where_it_is, _)| where_it_is)
    }

    /// What a midden turns into, once it has stopped being a midden.
    ///
    /// "If the agents are expelling their waste and piling it away from their
    /// tents, then over time the waste should break down and seeds from the
    /// plants they have eaten should sprout."
    ///
    /// Everything it needs is already on the tile: the seeds that came through
    /// whole, the nutrient the rot released, and enough time for the smell to
    /// go. When all three line up something comes up, and it is food, and
    /// nobody planted it.
    fn what_was_dropped_comes_up(&mut self) {
        use crate::world::{Position, ResourceNode, ResourceType};

        let mut came_up: Vec<Position> = Vec::new();

        for (y, row) in self.world.grid.tiles.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                if tile.soil.ready_to_sprout() {
                    came_up.push(Position::new(x as i32, y as i32));
                }
            }
        }

        for where_it_is in came_up {
            // Not on top of something already growing there.
            if self
                .world
                .resources
                .iter()
                .any(|resource| resource.position == where_it_is)
            {
                continue;
            }

            let seed = match self.world.grid.get_tile_mut(&where_it_is) {
                Some(tile) => tile.soil.it_came_up(),
                None => continue,
            };

            // What comes up is a volunteer, not a field: a few plants off one
            // midden, and no bigger for a bigger midden.
            let how_much = ((seed * Self::WHAT_A_MIDDEN_COMES_UP_IN).round() as u32).clamp(1, 8);

            let mut volunteer =
                ResourceNode::new(ResourceType::Food, where_it_is, how_much);
            volunteer.amount = how_much;
            self.world.resources.push(volunteer);

            debug!("Something came up on the midden at {where_it_is:?}");

            // And whoever is close enough to see it takes the lesson: what
            // they threw away last season is standing here as food. This is
            // the only thing in the world that teaches farming outright -
            // everything else is somebody breaking ground on a hunch.
            for agent in self
                .population
                .agents
                .iter_mut()
                .filter(|agent| agent.body.is_alive())
            {
                let apart = (agent.state.position.0 - where_it_is.x).abs()
                    + (agent.state.position.1 - where_it_is.y).abs();

                if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    agent
                        .practices
                        .saw_it_work(crate::agents::practices::Practice::Farming);
                }
            }
        }
    }

    /// What a fair trade is worth to a bond, and what a gift is.
    ///
    /// A gift is worth more, which is the whole difference between the two:
    /// a trade leaves both parties square and a gift leaves one of them owing.
    const WHAT_A_FAIR_TRADE_IS_WORTH: f32 = 0.15;
    const WHAT_A_GIFT_IS_WORTH: f32 = 0.4;

    /// How near two people have to be standing to hand anything over.
    const CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER: i32 = 3;

    /// What these two would swap, if anything: what the first has spare and
    /// the second wants, and the other way round.
    ///
    /// "The agents should also use a barter system if they have an abundance
    /// of something another agent wants and that agent has an abundance of
    /// something they want." Both halves, and it returns `None` unless both
    /// hold.
    fn what_the_two_of_them_would_swap(
        &self,
        me: usize,
        them: usize,
    ) -> Option<((String, u32), (String, u32))> {
        let mine = self.what_i_would_hand_over(me, them)?;
        let theirs = self.what_i_would_hand_over(them, me)?;

        if mine.0 == theirs.0 {
            return None;
        }

        Some((mine, theirs))
    }

    /// What the first of these would hand the second, if anything.
    ///
    /// One-sided on purpose: it is what a gift is, and it is half of what a
    /// trade is. Abundance is measured against the other pack rather than
    /// against a number — what makes a thing worth handing over is that they
    /// have much less of it than you do, which is a comparison and not a
    /// threshold. The first cut of this asked for six of a thing on one side
    /// and fewer than six on the other, and over eight worlds of ten thousand
    /// ticks a settlement traded once.
    fn what_i_would_hand_over(&self, me: usize, them: usize) -> Option<(String, u32)> {
        let mine = self.population.agents[me].what_i_can_spare()?;

        let they_have = self.population.agents[them].how_many_i_have(&mine.0);
        let i_have = self.population.agents[me].how_many_i_have(&mine.0);

        // They want it if they have markedly less of it than I do. A man with
        // forty sticks and a man with thirty-eight are not trading partners.
        if they_have * Self::WHAT_MAKES_IT_WORTH_HAVING >= i_have {
            return None;
        }

        Some(mine)
    }

    /// How many times more of a thing you have to have before it is worth
    /// somebody else's while to take it off you.
    const WHAT_MAKES_IT_WORTH_HAVING: u32 = 2;

    /// Somebody within reach worth trading with.
    ///
    /// Trust matters: you do not put a thing in the hands of somebody you
    /// think would take it. What decides that is the same judgement that
    /// decides whether to take their word - see `Agent::would_take_their_word`.
    fn somebody_to_trade_with(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        let me = self
            .population
            .agents
            .iter()
            .position(|other| other.id == agent.id)?;

        self.population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, them)| them.id != agent.id && them.state.is_alive)
            .filter(|(_, them)| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            .filter(|(_, them)| agent.would_take_their_word(them.id, &them.traits))
            .find(|(them, _)| self.what_the_two_of_them_would_swap(me, *them).is_some())
            .map(|(_, them)| them.id)
    }

    /// How often turning a thing over in your hands tells you what it is for.
    ///
    /// Low, and scaled by the hand doing the turning. It has to be low: this
    /// costs a turn and no materials, so if it were generous it would collapse
    /// the whole chain into an afternoon spent looking at things.
    const WHAT_LOOKING_CLOSELY_IS_WORTH: f32 = 0.06;

    /// How far somebody will walk for a thing they can see lying on the ground.
    const WORTH_WALKING_OVER_FOR: u32 = 12;

    /// Something lying about that this agent has a use for.
    ///
    /// A thing on the ground is a thing somebody else made and did not take
    /// with them: a worn axe beside a man who drowned, a spear thrown and not
    /// recovered, whatever fell out of a full pack. Picking it up is the
    /// cheapest way there is to get a tool, and it is why what a people makes
    /// outlives the people who made it.
    fn something_worth_stooping_for(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        let here = Position::new(agent_position.0, agent_position.1);
        let short_of = agent.what_i_am_short_of();

        // Worth having: something the chain wants that this pack is short of,
        // or any tool at all - a spare axe is never wasted
        let worth_it = |what: &str| {
            short_of.contains(&what)
                || crate::environment::making::EVERY_TOOL
                    .iter()
                    .any(|tool| tool.called == what)
        };

        // Underfoot first
        if let Some(left) = self
            .world
            .what_is_lying_at(&here)
            .into_iter()
            .find(|left| worth_it(&left.item.item_id))
        {
            return Some(Action::PickUp {
                what: left.item.item_id.clone(),
            });
        }

        // Then anything close enough to be worth the walk
        let there = self
            .world
            .dropped
            .iter()
            .filter(|left| worth_it(&left.item.item_id))
            .map(|left| (left.where_it_is, here.distance_to(&left.where_it_is)))
            .filter(|(_, apart)| *apart <= Self::WORTH_WALKING_OVER_FOR)
            .min_by_key(|(_, apart)| *apart)
            .map(|(where_it_is, _)| where_it_is)?;

        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// How generous somebody has to feel about a person before handing them
    /// anything for nothing.
    ///
    /// Higher than the bar for trading with them: a trade is square and a gift
    /// is not, so it goes to people you actually think well of.
    const WELL_ENOUGH_OF_THEM_TO_GIVE: f32 = 0.4;

    /// Somebody within reach worth giving something to.
    fn somebody_to_give_to(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        let me = self
            .population
            .agents
            .iter()
            .position(|other| other.id == agent.id)?;

        let spare = agent.what_i_can_spare()?;

        self.population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, them)| them.id != agent.id && them.state.is_alive)
            .filter(|(_, them)| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            .filter(|(_, them)| {
                agent.how_far_i_trust(them.id, &them.traits) >= Self::WELL_ENOUGH_OF_THEM_TO_GIVE
            })
            .filter(|(_, them)| them.what_i_am_short_of().contains(&spare.0.as_str()))
            .find(|(them, _)| self.what_i_would_hand_over(me, *them).is_some())
            .map(|(_, them)| them.id)
    }

    /// How near you have to be standing to notice that the midden is growing.
    const CLOSE_ENOUGH_TO_SEE_IT_COME_UP: i32 = 6;

    /// What a mouthful of a strange plant that turns out to be food is worth.
    const WHAT_ONE_MOUTHFUL_IS_WORTH: f32 = 60.0;

    /// And what one that turns out not to be costs, in health.
    ///
    /// A person carries a hundred. The low end is a bad afternoon; the high
    /// end kills somebody who was not in good condition to start with, which
    /// is what makes tasting a thing a people does carefully and rarely.
    const WHAT_A_BAD_PLANT_DOES: (f32, f32) = (12.0, 55.0);

    /// Everything an agent might put in the ground, by what a pack calls it.
    ///
    /// Seed is not a separate thing an agent carries: a handful of the grain
    /// in the pack is next year's field, which is exactly the choice a hungry
    /// people has to make.
    fn what_can_be_sown() -> [(&'static str, crate::world::ResourceType, bool); 6] {
        use crate::world::ResourceType;

        // The flag is whether it is worth breaking ground for when the thing
        // driving you is hunger. Sprouted grain comes first because it is the
        // one thing in the list that is visibly already doing what a field is
        // for - a man holding it does not have to be told.
        [
            ("sproutedgrain", ResourceType::Grain, true),
            ("grain", ResourceType::Grain, true),
            ("food", ResourceType::Food, true),
            ("flax", ResourceType::Flax, false),
            ("cotton", ResourceType::Cotton, false),
            ("herbs", ResourceType::Herbs, false),
        ]
    }

    /// What this agent puts in the ground, given what it is carrying and what
    /// it has come to think of each.
    ///
    /// Of the sowable things in the pack it picks the one its own record rates
    /// best - which for an agent that has never farmed is whichever comes
    /// first, and for one that has walked back to a field of berries three
    /// autumns running is emphatically not berries. An agent carrying nothing
    /// sowable puts in what it has been eating, and learns from that too.
    fn what_this_one_would_sow(agent: &crate::agents::Agent) -> crate::world::ResourceType {
        use crate::world::ResourceType;

        let mut best: Option<(ResourceType, f32)> = None;

        for (called, crop, feeds_anybody) in Self::what_can_be_sown() {
            // A field is broken to answer hunger, so what goes in it is
            // something a person can eat. The first cut of this let an agent
            // sow whatever was in the pack, and over eight worlds the people
            // put in flax and cotton and starved beside their own linen.
            if !feeds_anybody {
                continue;
            }

            if agent.how_many_i_have(called) == 0 {
                continue;
            }

            let believed = agent
                .lessons
                .how_likely_to_try_this(&format!("sow:{called}"));

            if best.map(|(_, so_far)| believed > so_far).unwrap_or(true) {
                best = Some((crop, believed));
            }
        }

        best.map(|(crop, _)| crop).unwrap_or(ResourceType::Food)
    }

    /// How much comes up off one tile's worth of seed.
    const WHAT_A_MIDDEN_COMES_UP_IN: f32 = 8.0;

    /// How wet it has to be under a pack before grain in it starts moving.
    ///
    /// Set against `Soil::humidity`, which reads the country and the sky
    /// together: a wetland or a riverbank is wet enough standing still, a
    /// forest floor is on the line, and open plains only get there when it is
    /// actually raining. Dry ground under a clear sky never does.
    const WET_ENOUGH_TO_START_IT: f32 = 0.7;

    /// And how readily it goes, per tick, at that wetness.
    ///
    /// Slow: a handful of grain that gets rained on does not come up the same
    /// afternoon. Over a wet season most of what a person is carrying will
    /// have started, which is the point - the seed spoils as food and becomes
    /// something else.
    const HOW_READILY_GRAIN_TAKES: f32 = 0.01;

    /// Grain carried in the wet stops being grain.
    ///
    /// "Something like grain getting wet should result in the grains
    /// sprouting." Nobody does this on purpose. It is a thing that happens to
    /// a pack in the rain, and it is the plainest lesson in the world about
    /// what seed does, because it happens in the owner's hands.
    fn what_got_wet_sprouts(&mut self) {
        use crate::agents::InventoryItem;
        use rand::Rng;

        let raining = self
            .world
            .climate
            .weather
            .weather_type
            .precipitation_intensity();

        let mut rng = rand::thread_rng();

        for index in 0..self.population.agents.len() {
            if !self.population.agents[index].state.is_alive {
                continue;
            }

            let where_it_stands = self.population.agents[index].state.position;

            // Rain on the pack, or the wet of the ground it is set down on.
            // A camp beside a river is a wet camp whatever the sky is doing.
            let wet = self
                .world
                .grid
                .get_tile(&crate::world::Position::new(where_it_stands.0, where_it_stands.1))
                .map(|tile| {
                    crate::world::Soil::humidity(tile.terrain.terrain_type, raining)
                })
                .unwrap_or(0.0);

            if wet < Self::WET_ENOUGH_TO_START_IT {
                continue;
            }

            let agent = &mut self.population.agents[index];

            if agent.how_many_i_have("grain") == 0 {
                continue;
            }

            if !rng.gen_bool((wet * Self::HOW_READILY_GRAIN_TAKES).clamp(0.0, 1.0) as f64) {
                continue;
            }

            agent.inventory.remove_item("grain", 1);
            agent.inventory.add_item(InventoryItem::new_with_weight(
                "sproutedgrain".to_string(),
                1,
                0.5,
            ));

            debug!("Agent {} found the grain in its pack coming up", agent.id);
        }
    }

    /// How readily a sprouted grain works its way out of a pack, per tick.
    const WHAT_FALLS_OUT_OF_A_PACK: f64 = 0.02;

    /// And what a plant grown from one carries when it is full grown.
    const WHAT_ONE_SEED_COMES_TO: u32 = 30;

    /// A sprouted grain dropped where it can grow, grows.
    ///
    /// "If sprouted grains are thrown out or dropped, they could grow into
    /// adult plants." Nobody plants this. It falls out of a pack onto ground
    /// somebody happened to be standing on, and the next time anybody walks
    /// past there is a plant. Whoever is near enough to see it takes the
    /// lesson, the same as with the midden - this is the second of the two
    /// accidents that teach a people what seed is for.
    fn what_was_dropped_takes_root(&mut self) {
        use crate::world::{Position, ResourceNode, ResourceType};
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut took_root: Vec<Position> = Vec::new();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            if agent.how_many_i_have("sproutedgrain") == 0 {
                continue;
            }

            if !rng.gen_bool(Self::WHAT_FALLS_OUT_OF_A_PACK) {
                continue;
            }

            agent.inventory.remove_item("sproutedgrain", 1);
            took_root.push(Position::new(
                agent.state.position.0,
                agent.state.position.1,
            ));
        }

        for where_it_fell in took_root {
            // Not on rock, not in a river, and not on top of something already
            // growing there
            let can_grow = self
                .world
                .grid
                .get_tile(&where_it_fell)
                .map(|tile| {
                    tile.terrain.can_be_tilled() || tile.terrain.is_cultivated()
                })
                .unwrap_or(false);

            if !can_grow {
                continue;
            }

            if self
                .world
                .resources
                .iter()
                .any(|resource| resource.position == where_it_fell)
            {
                continue;
            }

            let mut plant = ResourceNode::new(
                ResourceType::Grain,
                where_it_fell,
                Self::WHAT_ONE_SEED_COMES_TO,
            );
            plant.amount = 1;
            self.world.resources.push(plant);

            debug!("A dropped grain took root at {where_it_fell:?}");

            for agent in self
                .population
                .agents
                .iter_mut()
                .filter(|agent| agent.state.is_alive)
            {
                let apart = (agent.state.position.0 - where_it_fell.x).abs()
                    + (agent.state.position.1 - where_it_fell.y).abs();

                if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    agent
                        .practices
                        .saw_it_work(crate::agents::practices::Practice::Farming);
                }
            }
        }
    }

    /// The nearest fire within reach, and where it is.
    ///
    /// With `lit_only` this reports only a fire that is actually burning; with
    /// it false, a cold hearth counts too, which is what relighting looks for.
    fn nearest_fire_from(
        &self,
        position: (i32, i32, i32),
        reach: i32,
        lit_only: bool,
    ) -> Option<(uuid::Uuid, (i32, i32, i32))> {
        self.world
            .heat_sources
            .all()
            .into_iter()
            .filter(|fire| !lit_only || fire.is_lit)
            .filter(|fire| {
                (fire.position.0 - position.0).abs() <= reach
                    && (fire.position.1 - position.1).abs() <= reach
            })
            .min_by_key(|fire| {
                (fire.position.0 - position.0).abs() + (fire.position.1 - position.1).abs()
            })
            .map(|fire| (fire.id, fire.position))
    }

    /// What the agent would put on a fire, if anything.
    ///
    /// A named food is taken at its word - cook a berry if you insist, and
    /// lose it. Asked for anything, an agent picks something a fire actually
    /// improves, because nobody sets out to burn their dinner.
    fn cookable_item(agent: &crate::agents::Agent, food_type: &str) -> Option<String> {
        use crate::world::nutrition::CookingOutcome;

        let named = !food_type.is_empty() && food_type != "generic";
        if named {
            return agent
                .inventory
                .get_item(food_type)
                .filter(|item| item.quantity > 0)
                .map(|item| item.item_id.clone());
        }

        agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter(|item| {
                item.food_data
                    .as_ref()
                    .map(|food| food.preparation == crate::world::nutrition::PreparationState::Raw)
                    .unwrap_or(true)
            })
            .filter(|item| {
                crate::agents::storage_integration::id_to_item_type(&item.item_id)
                    .map(|item_type| item_type.cooking_outcome() == CookingOutcome::Improves)
                    .unwrap_or(false)
            })
            .map(|item| item.item_id.clone())
            .min()
    }

    /// Whether the agent is carrying something a fire would improve
    fn has_food_worth_cooking(agent: &crate::agents::Agent) -> bool {
        Self::cookable_item(agent, "generic").is_some()
    }

    /// How far an agent will walk to reach a fire that is already burning
    const FIRE_WALK_RADIUS: i32 = 20;

    /// How much warmer a garment has to be before it is worth changing into.
    ///
    /// Without a margin an agent swaps between two near-identical coats every
    /// tick forever: whatever it is wearing wears down a little each tick, so
    /// the one folded in its pack is always fractionally better.
    const WARMTH_WORTH_CHANGING_FOR: f32 = 0.05;

    /// How much better a new garment has to be before it is worth the material
    /// and the work of making one.
    ///
    /// Whatever is on an agent's back wears a little thinner every tick, so
    /// against a bare comparison there is always a fresh coat worth making:
    /// agents replaced their clothes every few hundred ticks and ended up
    /// carrying dozens of cast-offs. A quarter better means a real
    /// improvement - a better material, or a hand that has learned something -
    /// rather than ordinary wear.
    const WORTH_MAKING_ANEW: f32 = 1.25;

    /// Whether a garment of this warmth is worth making, given what is already
    /// on that slot
    fn worth_making(warmth: f32, worn: f32) -> bool {
        warmth > worn * Self::WORTH_MAKING_ANEW + Self::WARMTH_WORTH_CHANGING_FOR
    }

    /// How far below its ideal an agent has to be to want another layer.
    ///
    /// Well short of `is_too_cold`, which is two degrees down and already
    /// dangerous: nobody waits until they are hypothermic to think about a
    /// coat, and an agent that did would spend the whole time it was cold
    /// walking to shelter instead of making one.
    const CHILLY_MARGIN: f32 = 0.5;

    /// How far an agent will travel for the material to clothe itself.
    ///
    /// Further than it will go for food, because flax and cotton grow in a
    /// handful of patches on a map where there is something to eat almost
    /// everywhere - but not so far that the trip costs more than the coat is
    /// worth.
    const CLOTHING_MATERIAL_RADIUS: u32 = 40;

    /// Insulation past which an agent counts itself dressed and gets on with
    /// its life.
    ///
    /// Without a stopping point this is a bottomless job. An unclothed agent
    /// sits about a degree under its ideal most of the time, so it is nearly
    /// always a little cold, and there is nearly always another patch of flax
    /// somewhere worth walking to: agents chased marginal warmth across the
    /// map instead of eating, and populations fell by a quarter.
    const ENOUGH_INSULATION: f32 = 0.35;

    /// Whether the agent can spare the material for this garment.
    ///
    /// Wood is the one material that is wanted for something else: a fire
    /// takes ten and cooking is worth more than a pair of bark boots is, so
    /// wood only goes into clothing once there is a fire's worth left over.
    /// Without this agents made boots out of the firewood, stopped cooking,
    /// and went back to eating raw - four points of the fed population, for an
    /// insulation of about one part in a hundred.
    fn can_spare_material(
        agent: &crate::agents::Agent,
        recipe: &crate::agents::equipment::GarmentRecipe,
    ) -> bool {
        let reserve = if recipe.material_item == "wood" {
            Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
        } else {
            0
        };

        agent
            .inventory
            .has_item(recipe.material_item, recipe.material_amount + reserve)
    }

    /// Whether the agent is cold enough, and bare enough, to want another layer
    fn wants_more_clothing(agent: &crate::agents::Agent) -> bool {
        agent.body_temperature.current < agent.body_temperature.ideal - Self::CHILLY_MARGIN
            && agent.body.total_cold_insulation() < Self::ENOUGH_INSULATION
    }

    /// What an agent of this much practice turns a given material into.
    ///
    /// The generic skill quality curve puts every untrained agent at Pathetic,
    /// and skills start ten levels below untrained, so a first cloak was worth
    /// half of nothing and no agent ever cooked or sewed often enough to climb
    /// out. A first attempt here is crude but wearable, and practice tells.
    fn expected_garment_quality(agent: &crate::agents::Agent) -> crate::agents::skills::Quality {
        use crate::agents::skills::Quality;

        let practice = agent
            .skills
            .get_skill_if_exists(crate::agents::SkillType::Leatherworking)
            .map(|skill| skill.level)
            .unwrap_or(-10);

        match practice {
            level if level < 0 => Quality::Crude,
            0..=3 => Quality::Basic,
            4..=6 => Quality::Moderate,
            7..=8 => Quality::Advanced,
            _ => Quality::Expert,
        }
    }

    /// How warm a garment of this recipe and quality is
    fn garment_warmth(
        recipe: &crate::agents::equipment::GarmentRecipe,
        quality: crate::agents::skills::Quality,
    ) -> f32 {
        recipe.warmth() * quality.modifier()
    }

    /// How warm the agent is already, in that slot
    fn warmth_worn(agent: &crate::agents::Agent, slot: crate::agents::equipment::EquipmentSlot) -> f32 {
        agent
            .body
            .equipment
            .get(&slot)
            .map(|worn| worn.cold_insulation())
            .unwrap_or(0.0)
    }

    /// A garment in the pack worth changing into
    fn garment_to_put_on(agent: &crate::agents::Agent) -> Option<String> {
        use crate::agents::equipment::garment_recipe;

        agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter_map(|item| {
                let recipe = garment_recipe(&item.item_id)?;
                let quality = item.quality.unwrap_or(crate::agents::skills::Quality::Crude);
                let wear = match (item.current_durability, item.max_durability) {
                    (Some(current), Some(max)) if max > 0.0 => (current / max).clamp(0.0, 1.0),
                    _ => 1.0,
                };
                let warmth = Self::garment_warmth(recipe, quality) * wear;

                if warmth > Self::warmth_worn(agent, recipe.slot) + Self::WARMTH_WORTH_CHANGING_FOR
                {
                    Some((recipe.id.to_string(), warmth))
                } else {
                    None
                }
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }

    /// The warmest garment the agent could make right now that would be an
    /// improvement on what it is wearing
    fn garment_to_make(agent: &crate::agents::Agent) -> Option<String> {
        let quality = Self::expected_garment_quality(agent);

        crate::agents::equipment::GARMENT_RECIPES
            .iter()
            .filter(|recipe| Self::can_spare_material(agent, recipe))
            .filter(|recipe| {
                Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
            })
            .max_by(|a, b| {
                Self::garment_warmth(a, quality)
                    .partial_cmp(&Self::garment_warmth(b, quality))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|recipe| recipe.id.to_string())
    }

    /// The material for the warmest garment an agent could go and get, and the
    /// patch it grows in
    fn material_to_gather(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(String, crate::world::Position)> {
        use crate::world::ResourceType;

        let quality = Self::expected_garment_quality(agent);

        crate::agents::equipment::GARMENT_RECIPES
            .iter()
            .filter(|recipe| {
                Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
            })
            .filter_map(|recipe| {
                let resource = match recipe.material_item {
                    "flax" => ResourceType::Flax,
                    "cotton" => ResourceType::Cotton,
                    "hides" => ResourceType::Hides,
                    "wool" => ResourceType::Wool,
                    "wood" => ResourceType::Wood,
                    _ => return None,
                };

                let patch = self.nearest_resource_within(
                    agent_position,
                    Self::CLOTHING_MATERIAL_RADIUS,
                    |node| node.resource_type == resource,
                )?;

                // Warmth is worth having, but not at any distance. A cloak's
                // worth of flax forty tiles off is a worse bargain than bark
                // from the trees an agent is standing in, and agents that
                // always went for the warmest thing walked instead of ate.
                let from = crate::world::Position::new(agent_position.0, agent_position.1);
                let travel = from.distance_to(&patch) as f32;
                let worth = Self::garment_warmth(recipe, quality) / (1.0 + travel / 10.0);

                Some((recipe.material_item.to_string(), patch, worth))
            })
            .max_by(|(_, _, a), (_, _, b)| {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(material, patch, _)| (material, patch))
    }

    /// How close a hunter has to be to strike: a spear's throw, not a
    /// line of sight across the valley
    const HUNT_REACH: i32 = 2;



    /// How far apart two people can be and still come to anything.
    const CLOSE_ENOUGH_TO_COURT: i32 = 3;

    /// Somebody worth having a child with.
    ///
    /// `resolve_action_target` filled a nil Mate target with whoever happened
    /// to be nearest, which is neither a courtship nor a plan. Measured, Mate
    /// was 19.7% of everything a settlement did and failed 99.9% of the time:
    /// the target could not reproduce, or was too far off, or one of the two
    /// was barely fertile. One birth per thousand-odd attempts.
    ///
    /// Three things decide it, and trust is the first of them. Somebody an
    /// agent has not built up any confidence in is not somebody it will have a
    /// child with, however close they are standing - and trust here is the
    /// whole of what one agent thinks of another: the bond, whether they have
    /// been straight with it before, and what sort of people they both are.
    ///
    /// Then the plain facts of the matter, so the attempt can actually come to
    /// something: near enough, and a pair who could have a child at all.
    fn somebody_to_have_a_child_with(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        use crate::agents::reproduction::{can_mate, MateSelectionCriteria};

        let criteria = MateSelectionCriteria::default();

        self.population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                let paces = (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs());
                paces <= Self::CLOSE_ENOUGH_TO_COURT
            })
            .filter(|them| agent.would_take_their_word(them.id, &them.traits))
            .filter(|them| can_mate(agent, them, &criteria))
            .max_by(|a, b| {
                let trust = |them: &&crate::agents::Agent| {
                    agent.how_far_i_trust(them.id, &them.traits)
                };
                trust(a)
                    .partial_cmp(&trust(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|them| them.id)
    }

    /// Putting a roof up, or going to fetch what it needs.
    ///
    /// Building was chosen without ever looking at what the agent was
    /// carrying, so the Construction drive spent an eighth of a settlement's
    /// life restating that it was short of materials. Measured, `Build` failed
    /// 100.0% of the time and the commonest single reason was being
    /// twenty-six wood and all thirty stone short of a house.
    ///
    /// An agent that has what a tent takes puts one up. An agent that does not
    /// goes and gets the thing it is shortest of, which is the same answer a
    /// person would give and turns a wasted turn into a useful one.
    fn raising_a_roof(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::BuildingType;

        // A tent is what a stone-age people can raise. Anything grander needs
        // stone they have no way to quarry.
        let wanted = BuildingType::SkinTent.requirements();

        let short_of = wanted
            .iter()
            .filter_map(|needed| {
                let name = format!("{:?}", needed.resource_type).to_lowercase();
                let have = agent
                    .inventory
                    .get_item(&name)
                    .map(|item| item.quantity)
                    .unwrap_or(0);
                if have >= needed.amount {
                    None
                } else {
                    Some((name, needed.amount - have))
                }
            })
            .max_by_key(|(_, missing)| *missing);

        match short_of {
            None => Some(Action::Build {
                structure_type: "tent".to_string(),
                position: agent_position,
            }),

            // Hides do not grow on bushes. Sending an agent to forage for them
            // is a wild goose chase, and a measured one: eighteen thousand
            // refusals of `No hides sources nearby` in a single world before
            // this told the difference between what the ground gives and what
            // has to be taken off an animal.
            Some((what, _)) if what.contains("hide") || what.contains("leather") => {
                self.hunting_action(agent, agent_position)
            }

            Some((what, _)) => Some(Action::Gather { resource_type: what }),
        }
    }

    /// The bare name of an action, without whatever it is aimed at.
    ///
    /// `Gather { resource_type: "berries" }` and `Gather { resource_type:
    /// "wood" }` are the same kind of day's work for counting purposes.
    fn name_of(action: &Action) -> String {
        let full = format!("{:?}", action);
        match full.find(|c: char| c == ' ' || c == '{' || c == '(') {
            Some(at) => full[..at].to_string(),
            None => full,
        }
    }

    /// Turn what a carcass drops into things an agent can actually use.
    ///
    /// A kill drops names from the fauna model - mutton, deer_meat, thick_hide
    /// - that nothing downstream knows about, so meat that was never renamed
    /// could not be cooked or eaten. This is where a carcass becomes food with
    /// nutrition in it and skins that can become a coat.
    ///
    /// `with_a_knife` is what the tool in the butcher's hand multiplies the
    /// carcass by - see `Agent::how_much_my_tools_help`. Taking a deer apart
    /// with a sharp flake and taking it apart with your hands are not the
    /// same job, and until now they were.
    fn butcher(
        &self,
        dropped: &[crate::environment::ItemStack],
        with_a_knife: f32,
    ) -> Vec<crate::agents::InventoryItem> {
        use crate::agents::InventoryItem;

        let current_tick = self.current_tick;

        // And how much there was on it to begin with, which is a question
        // about the time of year - see `Climate::how_fat_the_beasts_are`.
        let condition = self.world.climate.how_fat_the_beasts_are();

        dropped
            .iter()
            .map(|stack| {
                let off_the_carcass = ((stack.quantity as f32 * with_a_knife * condition).round()
                    as u32)
                    .max(1);
                let item_id =
                    crate::agents::storage_integration::butchered_item_id(&stack.material_id)
                        .to_string();
                let food_data = crate::agents::storage_integration::id_to_item_type(&item_id)
                    .filter(|item_type| item_type.is_consumable())
                    .and_then(|item_type| {
                        self.food_database.create_food_data(&item_type, current_tick)
                    });

                // Two kilos is what an animal drop weighs unless something
                // else says otherwise
                let mut item = InventoryItem::new_with_weight(item_id, off_the_carcass, 2.0);
                item.food_data = food_data;
                item
            })
            .collect()
    }

    /// How often a throw tells, for somebody with no skill and no spear.
    ///
    /// Everything else - the hand, the shaft, being up on a horse - is added
    /// to this. It was six in ten, which made hunting a matter of finding an
    /// animal rather than of killing one.
    const A_THROW_THAT_TELLS: f32 = 0.3;

    /// What a throw that tells takes out of an animal.
    const WHAT_ONE_THROW_TAKES_OUT_OF_IT: f32 = 0.35;

    /// How often a throw that misses puts the shaft somewhere you have to go
    /// and get it.
    ///
    /// Half. A spear is not spent by being thrown, it is mislaid by it, and
    /// the difference between those two is a walk.
    const HOW_OFTEN_A_MISS_LOSES_THE_SHAFT: f64 = 0.5;

    /// What a throw costs, whether or not it lands.
    ///
    /// Hunting is walking, waiting, and throwing, and most of it comes to
    /// nothing. Three or four throws to bring a deer down at twenty-two apiece
    /// is a morning's work for a morning's meat.
    const WHAT_A_THROW_COSTS: f32 = 22.0;

    /// How far an agent will go after prey it has spotted.
    ///
    /// Short on purpose. Crossing the map for a sheep costs more than the
    /// skin is worth: at thirty tiles hunting took a seventh of the
    /// population and returned no more warmth than not hunting at all.
    const HUNT_SEARCH_RADIUS: f32 = 12.0;

    /// Whether this agent should be taking on this animal at all.
    ///
    /// Anything that fights back is a job for someone with a weapon in hand.
    /// An unarmed agent that walks up to a bear is not hunting, it is dying.
    fn worth_hunting(
        &self,
        agent: &crate::agents::Agent,
        animal: &crate::environment::Animal,
    ) -> bool {
        use crate::environment::AnimalBehavior;

        if !animal.is_alive() || animal.is_domesticated {
            return false;
        }

        let species = match self.world.animals.get_species(&animal.species_id) {
            Some(species) => species,
            None => return false,
        };

        let dangerous = matches!(
            species.behavior,
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial
        );

        !dangerous || agent.equipment.get_weapon().is_some()
    }

    /// The nearest animal this agent could reasonably take, and where it is
    fn nearest_prey(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, (i32, i32))> {
        self.world
            .get_animals_in_radius(
                (agent_position.0, agent_position.1),
                Self::HUNT_SEARCH_RADIUS,
            )
            .into_iter()
            .filter(|animal| self.worth_hunting(agent, animal))
            .min_by_key(|animal| {
                (animal.position.0 - agent_position.0).abs()
                    + (animal.position.1 - agent_position.1).abs()
            })
            .map(|animal| (animal.id, animal.position))
    }

    /// Whether the agent has a reason to go after an animal.
    ///
    /// Two of them: nothing to eat, or nothing warm to wear and no skins to
    /// make it from. Fur and hides are the warm half of the garment table and
    /// the only way to them is off an animal.
    fn wants_to_hunt(agent: &crate::agents::Agent) -> bool {
        // An agent hunts for skins, and the meat is a bonus.
        //
        // Hunting for the meat as such does not pay: berries and fish are
        // there for the taking and an animal has to be found, walked to and
        // hit. Agents that went after every animal because their pack was
        // empty starved for it, and two settlements in forty died out.
        //
        // It also keeps hunting until there are enough skins for the garment,
        // not until there is one skin: a fur coat takes five hides, and an
        // agent that stopped at the first came home with a single pelt over
        // and over and never wore anything warmer than woven flax.
        if !Self::wants_more_clothing(agent) {
            return false;
        }

        let quality = Self::expected_garment_quality(agent);

        let wants = crate::agents::equipment::GARMENT_RECIPES.iter().any(|recipe| {
            matches!(recipe.material_item, "hides" | "leather" | "wool")
                && Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
        });

        if !wants {
            return false;
        }

        // Stop once there is enough of anything to make one. An agent with a
        // pack full of hides has no business going after a sheep for the wool
        // it has never had.
        let can_already_make = crate::agents::equipment::GARMENT_RECIPES.iter().any(|recipe| {
            matches!(recipe.material_item, "hides" | "leather" | "wool")
                && Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
                && Self::can_spare_material(agent, recipe)
        });

        !can_already_make
    }

    /// Going after an animal: strike if it is within reach, close on it if not.
    ///
    /// Nothing in the simulation had ever selected `Action::Hunt` - the one
    /// place it appeared passed a nil animal id that the executor could not
    /// resolve - so no agent had ever hunted, and meat, hides and wool never
    /// reached an inventory at all.
    fn hunting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        if !Self::wants_to_hunt(agent) {
            return None;
        }

        // Somebody who has gone after animals a dozen times and come back
        // empty every time stops going after animals. Nothing tells them to:
        // it is what their own record says, and a hunter with a good record
        // keeps at it on the same evidence.
        if !agent.lessons.worth_trying(Undertaking::Hunting) {
            return None;
        }

        let (animal_id, animal_position) = self.nearest_prey(agent, agent_position)?;

        let reach = (animal_position.0 - agent_position.0)
            .abs()
            .max((animal_position.1 - agent_position.1).abs());

        if reach <= Self::HUNT_REACH {
            return Some(Action::Hunt {
                animal_id,
                weapon: agent.equipment.get_weapon().map(|weapon| weapon.name.clone()),
            });
        }

        Some(Action::Move {
            target: (animal_position.0, animal_position.1, agent_position.2),
        })
    }

    /// How far a parent lets a child of its own get before going after it
    const CHILD_LEASH: i32 = 8;

    /// How far an agent can work a reach from where it stands
    const CAST: i32 = 1;

    /// How far an agent will walk to get to water
    const WORTH_WALKING_TO_WATER: i32 = 14;

    /// A reach carrying this many fish is as good as fishing gets
    const A_GOOD_REACH: f32 = 60.0;

    /// What comes out of the water on a cast that works
    const FISH_PER_CAST: u32 = 2;

    /// How often a thrust tells in an empty reach, for somebody with nothing
    /// in his hands.
    ///
    /// Everything worth having is added to this: the thickness of the run, the
    /// hand, a rod, a spear. On its own it is a man standing in a river
    /// hoping.
    const A_THRUST_THAT_TELLS: f32 = 0.15;

    /// What standing in the water costs, whether or not anything takes.
    const WHAT_A_THRUST_COSTS: f32 = 8.0;

    /// What share of a fish is guts, heads and bone rather than meat.
    ///
    /// It goes to waste in the pack the moment the fish is caught, which is
    /// what puts a fishing agent in the way of doing a field good without ever
    /// meaning to.
    const OFFAL_SHARE: f32 = 0.35;

    /// The reach an agent standing here can work, if there is one.
    fn reach_within_cast(
        &self,
        agent_position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        self.world
            .resources
            .iter()
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .map(|resource| resource.position)
            .find(|position| {
                (position.x - agent_position.0).abs() <= Self::CAST
                    && (position.y - agent_position.1).abs() <= Self::CAST
            })
    }

    /// Standing in a river after fish, and walking to a river worth standing in.
    ///
    /// A fishery is not another way of getting a meal. It is the only food a
    /// settlement can take that the land does not pay for, because a fish is
    /// grown at sea and comes up the river under its own power - so what is
    /// left of it, put on a field, makes the country richer rather than
    /// slower to run down. Everything else a settlement does with the ground
    /// is at best a return of what it already took.
    ///
    /// Nobody is told this. An agent fishes because it is hungry and there is
    /// water; the guts go into its pack as waste like anything else; and if it
    /// has learned that tipping the pack on a field does the ground good, the
    /// two habits meet on their own. What the agent keeps of that meeting is
    /// its own record of whether fishing pays - a person who stood in an empty
    /// winter river a dozen times stops going.
    fn fishing_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        // Somebody who has stood in the water a dozen times and come out with
        // nothing stops going to the water.
        if !agent.lessons.worth_trying(Undertaking::Fishing) {
            return None;
        }

        // Fishing is what an agent does when it wants food or wants a store of
        // it. Both, in a settlement beside a river, most of the year.
        let hunger = agent
            .drives
            .get(DriveType::Hunger)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);
        let sustenance = agent
            .drives
            .get(DriveType::Sustenance)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);

        if hunger.max(sustenance) < Self::WORTH_GETTING_WET {
            return None;
        }

        if self.reach_within_cast(agent_position).is_some() {
            return Some(Action::Fish);
        }

        // Otherwise walk to the best water within reason: the thickest reach,
        // discounted by how far it is. A river in the run is worth crossing a
        // settlement for and an empty pool next door is not.
        let (best, _) = self
            .world
            .resources
            .iter()
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .filter_map(|resource| {
                let reach = (resource.position.x - agent_position.0)
                    .abs()
                    .max((resource.position.y - agent_position.1).abs());

                if reach > Self::WORTH_WALKING_TO_WATER {
                    return None;
                }

                let worth = resource.amount as f32 / (1.0 + reach as f32);
                Some((resource.position, worth))
            })
            .fold(
                None,
                |best: Option<(crate::world::Position, f32)>, (position, worth)| {
                    match best {
                        Some((_, best_worth)) if best_worth >= worth => best,
                        _ => Some((position, worth)),
                    }
                },
            )?;

        Some(Action::Move {
            target: (best.x, best.y, agent_position.2),
        })
    }

    /// How much an agent has to want food before it will go and stand in a river
    const WORTH_GETTING_WET: f32 = 0.35;

    /// Leave on the ground what bodies have to leave.
    ///
    /// Everything a settlement grew used to leave the world for good: eaten
    /// and gone, spoiled and deleted, buried nowhere. The soil was a stock
    /// being mined with no return at all, and the only thing that ever put
    /// anything back was an agent who had learned to tip a spoiled basket onto
    /// a field. Traced over thirty thousand ticks, farmed ground went from
    /// 0.53 fertility to 0.03 and stayed there.
    ///
    /// What a body takes in mostly comes out again, and what a body is comes
    /// back when it stops. Neither is a free lunch - rot keeps three fifths of
    /// what it works on and loses the rest, so the loop turns and loses on
    /// every turn. And it lands where the agent is standing rather than where
    /// the crop grew, which is exactly why carting muck onto a field is worth
    /// an agent's time.
    fn return_what_the_living_and_the_dead_leave(&mut self) {
        use crate::world::Position;

        // What the living have to pass
        let leavings: Vec<((i32, i32, i32), f32)> = self
            .population
            .agents
            .iter_mut()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.state.position, agent.state.void_waste()))
            .filter(|(_, waste)| *waste > 0.0)
            .collect();

        for (position, waste) in leavings {
            let here = Position::new(position.0, position.1);
            if let Some(tile) = self.world.grid.get_tile_mut(&here) {
                // Not just litter: a midden also has a smell and seeds in it.
                tile.soil.somebody_voided_here(waste);
            }
        }

        // And what the dead leave where they fell
        let bodies = std::mem::take(&mut self.population.bodies_where_they_fell);

        for (position, soft, bone) in bodies {
            let here = Position::new(position.0, position.1);
            if let Some(tile) = self.world.grid.get_tile_mut(&here) {
                tile.soil.add_leaf_litter(soft);
                tile.soil.add_woody_litter(bone);
            }
        }

        self.what_the_dead_left_behind();
    }

    /// What a person was carrying stays where they fell.
    ///
    /// Everything a people makes used to go into the ground with whoever
    /// happened to be holding it: an axe was a thing that existed for exactly
    /// as long as its owner did. A pack falls where its owner does, and the
    /// next person along can pick it up - which is most of how a stone-age
    /// people ever accumulates anything at all.
    fn what_the_dead_left_behind(&mut self) {
        use crate::world::Position;

        let left = std::mem::take(&mut self.population.what_the_dead_left);
        let now = self.current_tick;

        for (item, position) in left {
            self.world
                .somebody_left_this(item, Position::new(position.0, position.1), now);
        }
    }

    /// How far off something has to be before it stops being this agent's
    /// problem
    const CLOSE_ENOUGH_TO_WORRY_ABOUT: i32 = 10;

    /// How far a frightened agent puts between itself and the thing
    ///
    /// Far enough to be out of the range at which it would appraise the thing
    /// again, or it runs one pace, looks round, and runs one pace again.
    const FAR_ENOUGH_AWAY: i32 = Self::CLOSE_ENOUGH_TO_WORRY_ABOUT + 5;

    /// How far an angry agent will go to reach the thing it is angry at
    const WITHIN_A_STEP_OR_TWO: i32 = 5;

    /// The nearest living animal of a named kind, and how far off it is.
    ///
    /// An agent's fear and anger are held against a species name rather than
    /// against a particular animal - it is afraid of wolves, not of wolf #4 -
    /// so acting on the feeling means finding which wolf it can see.
    fn nearest_of_kind(
        &self,
        kind: &str,
        from: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, (i32, i32), i32)> {
        self.world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                if species.name != kind {
                    return None;
                }

                let paces = (animal.position.0 - from.0)
                    .abs()
                    .max((animal.position.1 - from.1).abs());
                if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
                    return None;
                }

                Some((animal.id, animal.position, paces))
            })
            .min_by_key(|(_, _, paces)| *paces)
    }

    /// Go, and put the thing behind you.
    ///
    /// The flight branch of action selection was keyed on `last_attacker`,
    /// which is only ever another agent, so an agent frightened of a wolf fell
    /// straight through it and carried on foraging with the wolf at its elbow.
    /// Fear of a creature now moves the agent directly away from it.
    fn run_from_what_frightens_me(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (kind, _) = agent.emotions.what_frightens_me_most()?;
        let (_, where_it_is, _) = self.nearest_of_kind(kind, agent_position)?;

        Some(Self::put_ground_between(agent_position, (where_it_is.0, where_it_is.1)))
    }

    /// Head off in the opposite direction, far enough not to arrive back where
    /// you started worrying.
    fn put_ground_between(from: (i32, i32, i32), away_from: (i32, i32)) -> Action {
        let dx = from.0 - away_from.0;
        let dy = from.1 - away_from.1;

        // Standing on top of it, which should not happen, is still a reason to
        // be somewhere else
        let span = ((dx * dx + dy * dy) as f32).sqrt();
        let (dx, dy, span) = if span < 1.0 { (1, 0, 1.0) } else { (dx, dy, span) };

        let far = Self::FAR_ENOUGH_AWAY as f32;
        Action::Move {
            target: (
                from.0 + (dx as f32 / span * far) as i32,
                from.1 + (dy as f32 / span * far) as i32,
                from.2,
            ),
        }
    }


    /// How much an agent has to resent one particular person before it will do
    /// anything about it.
    ///
    /// Read per person rather than off the total, because `should_attack` sums
    /// every source: three mild grudges of 0.2 read as a man ready to fight,
    /// and there is nobody he is actually ready to fight.
    const ENOUGH_TO_ROUND_ON_SOMEBODY: f32 = 0.5;

    /// Square up to the people you resent, or shrink from them.
    ///
    /// A grudge is the reason; whether it comes out as standing up or backing
    /// down is the same appraisal a wolf gets. Measured before this existed,
    /// anger at people ran at 0.806 for every agent that read as ready to
    /// fight and anger at creatures at 0.025 - so nearly all the anger in the
    /// model was a grudge against somebody, held against them for life,
    /// decaying at one per cent a tick and with no way to be acted on at all.
    ///
    /// The grudge itself is not touched. Only which feeling it comes out as.
    fn square_up_to_the_people_i_resent(&mut self) {
        let standing: Vec<(uuid::Uuid, (i32, i32, i32), f32, bool)> = self
            .population
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.id,
                    agent.state.position,
                    agent.own_strength(),
                    agent.state.is_alive,
                )
            })
            .collect();

        for index in 0..self.population.agents.len() {
            let (mine, from) = {
                let agent = &self.population.agents[index];
                if !agent.state.is_alive {
                    continue;
                }
                (agent.own_strength(), agent.state.position)
            };

            let resented: Vec<(uuid::Uuid, f32)> = {
                let agent = &self.population.agents[index];
                agent
                    .emotions
                    .anger_at_people()
                    .into_iter()
                    .filter(|(_, held)| *held >= Self::ENOUGH_TO_ROUND_ON_SOMEBODY)
                    .collect()
            };

            for (who, held) in resented {
                let Some((_, where_they_are, theirs, alive)) =
                    standing.iter().copied().find(|(id, ..)| *id == who)
                else {
                    continue;
                };
                if !alive {
                    continue;
                }

                let paces = (where_they_are.0 - from.0)
                    .abs()
                    .max((where_they_are.1 - from.1).abs());

                let agent = &mut self.population.agents[index];

                // Out of sight is not out of mind - the grudge stands - but
                // there is nothing to shrink from, and leaving the fear
                // standing would keep the agent running from an empty field
                // and keep it below the bar for ever squaring up to anybody.
                if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
                    agent
                        .emotions
                        .set_fear(crate::agents::EmotionSource::Agent(who), 0.0);
                    continue;
                }

                if theirs > mine {
                    // You cannot take them. What you feel about them is the
                    // same; what you will do about it is get out of the way.
                    let nearness = 1.0
                        - (paces as f32 / (Self::CLOSE_ENOUGH_TO_WORRY_ABOUT as f32 + 1.0));
                    agent.emotions.set_fear(
                        crate::agents::EmotionSource::Agent(who),
                        held * nearness,
                    );
                } else {
                    // You can, so it stays anger and stays where it was
                    agent
                        .emotions
                        .set_fear(crate::agents::EmotionSource::Agent(who), 0.0);
                }
            }
        }
    }

    /// Turn on the person you resent, if they are within arm's reach.
    ///
    /// Gated on the grudge against that one person rather than on total anger,
    /// and on the agent reckoning it can take them - which the appraisal above
    /// has already decided by turning the hopeless cases into fear.
    fn round_on_whoever_angers_me(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (who, held) = agent.emotions.who_angers_me_most()?;
        if held < Self::ENOUGH_TO_ROUND_ON_SOMEBODY {
            return None;
        }

        let them = self
            .population
            .agents
            .iter()
            .find(|other| other.id == who && other.state.is_alive)?;

        // Nobody raises a hand to a child, and nobody to their own parent
        if them.state.life_stage == crate::agents::LifeStage::Infant
            || them.state.life_stage == crate::agents::LifeStage::Child
            || them.parent_ids.contains(&agent.id)
            || agent.parent_ids.contains(&who)
        {
            return None;
        }

        let paces = (them.state.position.0 - agent_position.0)
            .abs()
            .max((them.state.position.1 - agent_position.1).abs());
        if paces > Self::HUNT_REACH {
            return None;
        }

        Some(Action::Attack {
            target_agent_id: who,
            weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
        })
    }

    /// Get away from the person you are afraid of.
    fn run_from_whoever_frightens_me(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (who, _) = agent.emotions.who_frightens_me_most()?;
        let them = self
            .population
            .agents
            .iter()
            .find(|other| other.id == who && other.state.is_alive)?;

        let where_they_are = them.state.position;
        let paces = (where_they_are.0 - agent_position.0)
            .abs()
            .max((where_they_are.1 - agent_position.1).abs());
        if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
            return None;
        }

        Some(Self::put_ground_between(
            agent_position,
            (where_they_are.0, where_they_are.1),
        ))
    }

    /// Turn on the thing, if it is close enough to hit.
    ///
    /// An angry agent stands its ground; it does not cross the map looking for
    /// a fight. Anything out of arm's reach is left alone and the agent gets on
    /// with its day, which is also what keeps a settlement from spending a
    /// quarter of its life walking towards wolves.
    fn round_on_what_angers_me(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (kind, _) = agent.emotions.what_angers_me_most()?;
        let (which, where_it_is, paces) = self.nearest_of_kind(kind, agent_position)?;

        if paces <= Self::HUNT_REACH {
            return Some(Action::Fight {
                animal_id: which,
                weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
            });
        }

        // Close the last pace or two, but no further. The appraisal already
        // scales a creature's strength by how near it is, so anything that
        // angers an agent past the threshold is close by anyway - this is for
        // the wolf that is nearly in reach, not the one across the field.
        if paces <= Self::WITHIN_A_STEP_OR_TWO {
            return Some(Action::Move {
                target: (where_it_is.0, where_it_is.1, agent_position.2),
            });
        }

        None
    }

    /// What share of what an agent has left counts as having got off lightly
    const A_SCRATCH: f32 = 0.25;

    /// Feel about whatever is standing between an agent and what it needs.
    ///
    /// The specification, in two questions. Does a thing threaten my ability
    /// to satisfy my drives - and if so, can I fight it? Did a thing prevent
    /// it - and if so, can I fight *that*? Where the answer is yes it comes
    /// out as anger and the agent stands its ground; where it is no it comes
    /// out as fear and the agent goes.
    ///
    /// `ThreatAssessment` has always turned coping potential into one or the
    /// other, and `respond_to_threat` has always called it. What was missing
    /// was anything to call `respond_to_threat` except the resolution of a
    /// blow that had already landed: a wolf ten paces off and closing
    /// produced no feeling at all until it bit somebody. Measured over three
    /// worlds, mean fear ran at 0.01 to 0.06 and mean anger at exactly zero,
    /// and not one agent in a hundred and seventy ever reached the 0.6 that
    /// `should_flee` wants - so the branch of `generate_action` that lets an
    /// agent run or fight never once fired.
    fn feel_about_what_stands_in_the_way(&mut self) {
        // What is out there, and how much of a match each one is
        let hunters: Vec<((i32, i32), f32, String)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                if species.attack_damage <= 0.0 {
                    return None;
                }

                // What it is worth in a fight, on the same scale an agent
                // reckons itself on: a healthy body, and what it can do with it
                let condition = (animal.current_health / species.health.max(1.0)).clamp(0.0, 1.0);
                let menace = (species.attack_damage / 20.0).clamp(0.1, 2.0);

                Some((
                    (animal.position.0, animal.position.1),
                    condition * menace,
                    species.name.clone(),
                ))
            })
            .collect();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            let (x, y, _) = agent.state.position;

            // The worst thing within sight of this agent, if anything
            let worst = hunters
                .iter()
                .filter_map(|((hx, hy), strength, what)| {
                    let paces = (hx - x).abs().max((hy - y).abs());
                    if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
                        return None;
                    }

                    // A wolf across the field is not the wolf at your elbow.
                    // Without this an agent felt the full weight of anything
                    // within ten paces, and spent a third of its life angry at
                    // something it could barely see.
                    let nearness = 1.0
                        - (paces as f32 / (Self::CLOSE_ENOUGH_TO_WORRY_ABOUT as f32 + 1.0));

                    Some((strength * nearness, what))
                })
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            match worst {
                Some((strength, what)) => {
                    agent.appraise_what_is_there(
                        strength,
                        crate::agents::EmotionSource::Creature(what.clone()),
                    );
                }
                None => {
                    // Nothing is stalking this one, so whatever it was
                    // frightened of has gone
                    agent.emotions.nothing_is_stalking_me();
                }
            }
        }
    }

    /// How close something that would eat you counts as close
    const A_THREAT_NEARBY: i32 = 8;

    /// How far an agent looks when judging whether the ground round about is
    /// still bearing
    const GROUND_ROUND_ABOUT: u32 = 10;

    /// What a tile of ground within reach ought to be carrying before an agent
    /// stops worrying about next year's food
    const A_TILE_WORTH_HAVING: f32 = 25.0;

    /// Tell every agent what the world around it is doing.
    ///
    /// The drives are specified by the conditions that raise them - "hostile
    /// entity proximity", "nightfall", "others building", "crop depletion" -
    /// and half of those are things only the world knows. This gathers them
    /// once a tick per agent. The agent folds in what it knows about itself
    /// when its own drives are ticked, one tick later, which is near enough:
    /// nothing here changes faster than an agent can walk.
    fn read_the_situation(&mut self) {
        use crate::world::{Position, TerrainType};

        let night = !self.world.climate.is_daytime();
        let foul_weather = self.world.climate.weather.weather_type.precipitation_intensity() > 0.0
            || self.world.climate.weather.effective_wind_speed() > 8.0;

        // Where the predators are, and where anybody is building
        let hunters: Vec<(i32, i32)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter(|animal| {
                self.world
                    .animals
                    .get_species(&animal.species_id)
                    .map(|species| species.attack_damage > 0.0)
                    .unwrap_or(false)
            })
            .map(|animal| (animal.position.0, animal.position.1))
            .collect();

        let building_sites: Vec<(i32, i32)> = self
            .world
            .buildings
            .iter()
            .filter(|building| !building.is_completed())
            .map(|building| (building.position.x, building.position.y))
            .collect();

        let current_tick = self.current_tick;

        // Small children, by whose parent they are
        let young: Vec<(Vec<uuid::Uuid>, (i32, i32, i32))> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                matches!(
                    agent.state.life_stage,
                    crate::agents::LifeStage::Infant | crate::agents::LifeStage::Child
                )
            })
            .map(|agent| (agent.parent_ids.clone(), agent.state.position))
            .collect();

        let grown: Vec<(i32, i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| agent.state.position)
            .collect();

        // What the ground within reach is carrying, per agent position
        let crop_at = |position: (i32, i32, i32)| -> f32 {
            let here = Position::new(position.0, position.1);
            let mut standing = 0u32;
            let mut patches = 0u32;

            for resource in &self.world.resources {
                if !resource.resource_type.is_edible() {
                    continue;
                }
                if here.distance_to(&resource.position) > Self::GROUND_ROUND_ABOUT {
                    continue;
                }
                standing += resource.amount;
                patches += 1;
            }

            if patches == 0 {
                return 0.0;
            }

            (standing as f32 / (patches as f32 * Self::A_TILE_WORTH_HAVING)).clamp(0.0, 1.0)
        };

        let mut readings = Vec::with_capacity(self.population.agents.len());

        for agent in &self.population.agents {
            if !agent.state.is_alive {
                readings.push(None);
                continue;
            }

            let position = agent.state.position;
            let near = |spot: &(i32, i32), reach: i32| {
                (spot.0 - position.0).abs().max((spot.1 - position.1).abs()) <= reach
            };

            let mine: Vec<&(Vec<uuid::Uuid>, (i32, i32, i32))> = young
                .iter()
                .filter(|(parents, _)| parents.contains(&agent.id))
                .collect();

            let child_astray = mine.iter().any(|(_, child)| {
                let strayed = (child.0 - position.0).abs().max((child.1 - position.1).abs())
                    > Self::CHILD_LEASH;
                let stalked = hunters.iter().any(|hunter| {
                    (hunter.0 - child.0).abs().max((hunter.1 - child.1).abs())
                        <= Self::DANGER_TO_A_CHILD
                });
                strayed || stalked
            });

            let here = Position::new(position.0, position.1);
            let ground = self
                .world
                .grid
                .get_tile(&here)
                .map(|tile| tile.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            readings.push(Some(crate::core::Surroundings {
                predator_near: hunters.iter().any(|spot| near(spot, Self::A_THREAT_NEARBY)),
                night,
                foul_weather,
                under_shelter: self
                    .world
                    .buildings
                    .iter()
                    .any(|building| building.position == here && building.is_completed()),
                recently_hurt: agent.emotions.recent_attacker(current_tick).is_some(),
                crop_near: crop_at(position),
                somewhere_to_build: crate::world::Terrain::new(ground).can_be_tilled(),
                neighbours_building: building_sites.iter().any(|spot| near(spot, 12)),
                children_to_mind: mine.len() as u32,
                child_astray,
                company: grown
                    .iter()
                    .filter(|other| **other != position)
                    .any(|other| {
                        (other.0 - position.0).abs().max((other.1 - position.1).abs()) <= 6
                    }),
            }));
        }

        for (agent, reading) in self.population.agents.iter_mut().zip(readings) {
            if let Some(reading) = reading {
                agent.surroundings = reading;
            }
        }
    }

    /// How close a predator has to be to a child before its parent runs
    const DANGER_TO_A_CHILD: i32 = 10;

    /// Going to a child of one's own.
    ///
    /// A parent keeps its children near it, and goes to one that has strayed
    /// or that something is stalking. This is the whole of the Protection
    /// drive: it is answered by being where the children are, not by
    /// acquiring anything.
    ///
    /// It matters more than it looks. The young are kept warm by whoever is
    /// beside them, so a parent that wanders off leaves its child to the
    /// weather - and children freezing is what emptied settlements before.
    fn protective_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::LifeStage;

        // Only the small ones. An adolescent can look after itself.
        let mine: Vec<(i32, i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|child| child.state.is_alive)
            .filter(|child| child.parent_ids.contains(&agent.id))
            .filter(|child| {
                matches!(child.state.life_stage, LifeStage::Infant | LifeStage::Child)
            })
            .map(|child| child.state.position)
            .collect();

        if mine.is_empty() {
            return None;
        }

        // Anything with teeth near one of them brings a parent at a run
        let hunted = mine.iter().find(|child| {
            self.world
                .get_animals_in_radius((child.0, child.1), Self::DANGER_TO_A_CHILD as f32)
                .into_iter()
                .any(|animal| {
                    animal.is_alive()
                        && !animal.is_domesticated
                        && self
                            .world
                            .animals
                            .get_species(&animal.species_id)
                            .map(|species| !species.prey_species.is_empty())
                            .unwrap_or(false)
                })
        });

        if let Some(child) = hunted {
            return Some(Action::Move {
                target: (child.0, child.1, agent_position.2),
            });
        }

        // Otherwise, the one that has wandered furthest off
        let strayed = mine
            .iter()
            .map(|child| {
                let distance = (child.0 - agent_position.0)
                    .abs()
                    .max((child.1 - agent_position.1).abs());
                (child, distance)
            })
            .max_by_key(|(_, distance)| *distance)
            .filter(|(_, distance)| *distance > Self::CHILD_LEASH);

        strayed.map(|(child, _)| Action::Move {
            target: (child.0, child.1, agent_position.2),
        })
    }

    /// How far an agent will walk to break new ground
    const FIELD_WALK_RADIUS: u32 = 12;

    /// How many fields a settlement wants within reach of where it is standing
    const FIELDS_WANTED: usize = 6;

    /// Fields already broken within reach
    fn fields_within(&self, position: (i32, i32, i32), radius: u32) -> usize {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);

        let reach = radius as i32;
        let mut fields = 0;

        for dx in -reach..=reach {
            for dy in -reach..=reach {
                let candidate = Position::new(from.x + dx, from.y + dy);

                if from.distance_to(&candidate) > radius {
                    continue;
                }

                if self
                    .world
                    .grid
                    .get_tile(&candidate)
                    .map(|tile| tile.terrain.is_cultivated())
                    .unwrap_or(false)
                {
                    fields += 1;
                }
            }
        }

        fields
    }

    /// Somewhere nearby worth breaking: open grass with nothing growing on it
    fn ground_to_break(&self, position: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);
        let radius = Self::FIELD_WALK_RADIUS as i32;

        // What is already growing, gathered once: asking the resource list per
        // candidate tile turns this into tens of thousands of comparisons per
        // agent per tick
        let occupied: std::collections::HashSet<(i32, i32)> = self
            .world
            .resources
            .iter()
            .map(|resource| (resource.position.x, resource.position.y))
            .collect();

        let mut best: Option<(Position, u32)> = None;

        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let candidate = Position::new(from.x + dx, from.y + dy);

                if occupied.contains(&(candidate.x, candidate.y)) {
                    continue;
                }

                if !self.world.grid.is_valid_position(&candidate) {
                    continue;
                }

                let tillable = self
                    .world
                    .grid
                    .get_tile(&candidate)
                    .map(|tile| tile.terrain.can_be_tilled())
                    .unwrap_or(false);

                if !tillable {
                    continue;
                }

                let distance = from.distance_to(&candidate);
                if best.map(|(_, d)| distance < d).unwrap_or(true) {
                    best = Some((candidate, distance));
                }
            }
        }

        best.map(|(position, _)| position)
    }

    /// Tipping the spoiled contents of a pack onto the ground.
    ///
    /// Nothing tells an agent to do this. It carries refuse it cannot eat, it
    /// is standing on ground it has broken, and now and again - out of
    /// curiosity, and more often once it has half a notion the thing works - it
    /// tips the basket out and sees what happens. What it sees is the ground
    /// getting richer, which is worth something; what it works out over several
    /// seasons is whether that was worth the carrying.
    ///
    /// The practice spreads by being seen, and it is dropped by agents who try
    /// it half a dozen times on ground where it does nothing.
    fn muck_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Practice;
        use crate::world::Position;
        use rand::Rng;

        // Nothing to tip out
        let carrying_refuse = agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .any(|item| {
                item.food_data
                    .as_ref()
                    .map(|food| food.is_rotting() || food.is_ruined())
                    .unwrap_or(false)
            });

        if !carrying_refuse {
            return None;
        }

        // On a field, which is where it might do some good
        let here = Position::new(agent_position.0, agent_position.1);
        let on_a_field = self
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.is_cultivated())
            .unwrap_or(false);

        if !on_a_field {
            return None;
        }

        let curiosity = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        let roll = rand::thread_rng().gen::<f32>();

        if agent
            .practices
            .would_try(Practice::SpreadingMuck, curiosity, roll)
        {
            return Some(Action::SpreadMuck);
        }

        None
    }

    /// Breaking ground, and walking to somewhere worth breaking.
    ///
    /// Wild food regrows about four times slower than a grown settlement eats
    /// it, which is why settlements that got past a dozen people starved back
    /// down again. A field yields many times what the same ground does wild,
    /// and this is how one comes to exist: an agent with the immediate needs
    /// answered and the Sustenance drive up on it goes and breaks ground.
    fn farming_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        // Only somebody with nothing more pressing on. The drive itself only
        // climbs in an agent that is fed, watered, rested and warm.
        if !agent.immediate_needs_met() {
            return None;
        }

        let wants_to_provide = agent
            .drives
            .get(DriveType::Sustenance)
            .map(|drive| drive.is_active())
            .unwrap_or(false);

        if !wants_to_provide {
            return None;
        }

        // A standing field that has gone over to weeds is worth more than a
        // new one. "Farmers should not just drop seeds and get crops" - a
        // field neglected for a season carries almost nothing, and going round
        // it pulling weeds and picking pests off is most of what growing a
        // crop consists of.
        if let Some(field) = self.field_wanting_work(agent_position) {
            if field.x == agent_position.0 && field.y == agent_position.1 {
                return Some(Action::TendField);
            }

            return Some(Action::Move {
                target: (field.x, field.y, agent_position.2),
            });
        }

        // Enough fields around here already
        if self.fields_within(agent_position, Self::FIELD_WALK_RADIUS) >= Self::FIELDS_WANTED {
            return None;
        }

        // And breaking new ground is a thing that has to be worked out first.
        // Until an agent has seen food come up out of ground somebody put seed
        // in, spending a day digging grass over is a strange way to answer
        // hunger, and it does it only out of curiosity.
        let curiosity = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        let roll = {
            use rand::Rng;
            rand::thread_rng().gen::<f32>()
        };

        if !agent.practices.would_try(
            crate::agents::practices::Practice::Farming,
            curiosity,
            roll,
        ) {
            return None;
        }

        let ground = self.ground_to_break(agent_position)?;

        if ground.x == agent_position.0 && ground.y == agent_position.1 {
            return Some(Action::TillSoil);
        }

        Some(Action::Move {
            target: (ground.x, ground.y, agent_position.2),
        })
    }

    /// How near the camp a plant has to stand before nobody would bother
    /// moving it, and how near a cutting has to be put in for the move to have
    /// been worth making.
    const A_SHORT_WALK: u32 = 6;

    /// How many people standing together make a camp, when there is no roof up
    /// yet to mark one.
    const ENOUGH_PEOPLE_TO_BE_A_CAMP: u32 = 3;

    /// How much of a plant comes away as a cutting, and how much the slip
    /// grows into once it is in.
    ///
    /// The first cut of this took three units off the parent and grew into a
    /// plant carrying forty, and left the parent as big as it was. Over eight
    /// worlds the people planted two hundred slips apiece and the food
    /// standing on the map went up six times: transplanting was not moving
    /// food about, it was manufacturing it out of nothing.
    ///
    /// A slip is a piece of the plant. It comes off the parent's carrying
    /// capacity and not only off this year's crop, and what it grows into is
    /// somewhat more than what it cost - because a plant put in open ground
    /// with nobody's roots against it does better than one more stem on a
    /// crowded patch. Somewhat, not thirteen times.
    const WHAT_A_CUTTING_TAKES: u32 = 8;
    const WHAT_A_CUTTING_STARTS_WITH: u32 = 2;
    const WHAT_A_MOVED_PLANT_COMES_TO: u32 = 20;

    /// And how small a patch has to be before nobody digs any more out of it.
    const TOO_THIN_TO_DIG: u32 = 12;

    /// Where the camp is, from where this agent is standing.
    ///
    /// There is no settlement object in this model - see ISSUES_FOUND #11 -
    /// so a camp is the nearest roof, and failing that the middle of whatever
    /// knot of people the agent is standing in. Both are rough and both are
    /// good enough to answer "is this plant near where I live".
    fn where_the_camp_is(&self, position: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;

        // The people first, and the roof only when there are not enough of
        // them about to make a camp. `nearest_shelter_from` searches out from
        // wherever the agent is standing, so for a man twenty tiles out on the
        // moor it answers "the nearest cave to the moor", which is not his
        // home and is exactly the wrong answer to what this asks.
        let reach = Self::FORAGE_RADIUS as i32;

        let neighbours: Vec<(i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.state.position.0, agent.state.position.1))
            .filter(|(x, y)| {
                (x - position.0).abs() <= reach && (y - position.1).abs() <= reach
            })
            .collect();

        if (neighbours.len() as u32) >= Self::ENOUGH_PEOPLE_TO_BE_A_CAMP {
            return Some(Position::new(
                neighbours.iter().map(|(x, _)| x).sum::<i32>() / neighbours.len() as i32,
                neighbours.iter().map(|(_, y)| y).sum::<i32>() / neighbours.len() as i32,
            ));
        }

        self.nearest_shelter_from(position)
    }

    /// Moving a plant that is known to be good to ground beside the camp.
    ///
    /// This is the third way into farming and the one that needs no seed and
    /// no theory at all. A person who walks half a morning to the same berry
    /// bush every day, and who has already dug up plants for one reason or
    /// another, eventually digs up that one and puts it in beside the tents.
    /// It is not an idea about agriculture. It is an idea about the walk.
    ///
    /// Two halves: lift a piece of something growing a long way off, and put
    /// it in the ground where you live. What it teaches is taught by the
    /// plant standing there afterwards, which is `record_outcome` on the
    /// harvest like any other crop.
    fn transplanting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        let here = Position::new(agent_position.0, agent_position.1);
        let camp = self.where_the_camp_is(agent_position)?;

        // Carrying one already: get it in the ground somewhere near home
        if let Some(cutting) = Self::a_cutting_in_the_pack(agent) {
            let _ = cutting;

            if camp.distance_to(&here) > Self::A_SHORT_WALK {
                return Some(Action::Move {
                    target: (camp.x, camp.y, agent_position.2),
                });
            }

            let can_carry_it = self
                .world
                .grid
                .get_tile(&here)
                .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
                .unwrap_or(false);

            let taken = self
                .world
                .resources
                .iter()
                .any(|resource| resource.position == here);

            if can_carry_it && !taken {
                return Some(Action::PlantCutting);
            }

            // Standing on the wrong tile at home: step to one that will do
            let spot = self.ground_to_break((camp.x, camp.y, agent_position.2))?;
            if spot != here {
                return Some(Action::Move {
                    target: (spot.x, spot.y, agent_position.2),
                });
            }

            return None;
        }

        // Nothing carried: lift a piece of whatever is standing here, if it is
        // worth lifting - which means it is a long way from home and there is
        // something growing here that is known to be good
        if camp.distance_to(&here) <= Self::A_SHORT_WALK {
            return None;
        }

        // The first thing growing here that is worth lifting - not the first
        // thing growing here. A tile can carry more than one, and a strange
        // plant nobody has tried standing on the same ground as a berry bush
        // was enough to hide the bush.
        self.world
            .resources
            .iter()
            .filter(|resource| {
                resource.position == here
                    && resource.amount > Self::WHAT_A_CUTTING_TAKES
                    && resource.max_amount > Self::TOO_THIN_TO_DIG + Self::WHAT_A_CUTTING_TAKES
            })
            .find(|resource| {
                Self::what_can_be_sown()
                    .into_iter()
                    .any(|(_, crop, _)| crop == resource.resource_type)
            })
            .map(|_| Action::TakeCutting)
    }

    /// How often a curious man with a pack full of parts tries putting the
    /// wrong one in the right place.
    ///
    /// Low, and it is meant to be. Each try costs the makings of a spear, and
    /// the great majority of them come to nothing at all.
    const HOW_OFTEN_ANYBODY_TRIES_A_SWAP: f64 = 0.04;

    /// How willing somebody has to be before they put a strange plant in
    /// their mouth.
    ///
    /// Set against the Curiosity drive rather than a trait, because this is a
    /// thing done on an idle afternoon by somebody with nothing pressing on
    /// them - never by a man with a wolf behind him, and only rarely by
    /// anybody.
    const CURIOUS_ENOUGH_TO_EAT_IT: f32 = 0.55;

    /// And how often even a curious man actually does it, per chance.
    ///
    /// Low. A person who walks past a strange plant every day for years
    /// eventually tries one; a person who tries every plant he passes does not
    /// get to be a person for long.
    ///
    /// This is the chance of setting out towards one, not of eating it: a man
    /// who has walked to the plant eats it. Rolling again on arrival, which is
    /// what the first cut did, compounded a small chance against itself once
    /// per tick of the walk and meant nobody in eight worlds ever arrived.
    const HOW_OFTEN_ANYBODY_RISKS_IT: f64 = 0.06;

    /// Trying an unknown plant.
    fn tasting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::{Position, ResourceType};
        use rand::Rng;

        // Not while anything is actually wrong. A hungry man eating a strange
        // plant is a different story and a worse one; this is the idle
        // curiosity that finds things out cheaply.
        if !agent.immediate_needs_met() {
            return None;
        }

        let curious = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        if curious < Self::CURIOUS_ENOUGH_TO_EAT_IT {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);

        // The nearest one of a sort nobody here has an opinion about. The
        // first cut of this asked the agent to be standing exactly on the
        // plant, and over eight worlds of ten thousand ticks not one person
        // ever tried anything: sixteen tiles in ten thousand is not a thing
        // that happens by accident.
        let strange = self
            .world
            .resources
            .iter()
            .filter(|resource| {
                resource.resource_type == ResourceType::StrangePlant && resource.amount > 0
            })
            .filter(|resource| !agent.have_i_tried_that_plant(resource.kind))
            .map(|resource| (resource.position, here.distance_to(&resource.position)))
            .filter(|(_, apart)| *apart <= Self::FORAGE_RADIUS)
            .min_by_key(|(_, apart)| *apart)
            .map(|(where_it_is, _)| where_it_is)?;

        // Standing on it already: the walk was the deciding, and re-deciding
        // on arrival is what kept anybody from ever getting there. The roll is
        // made once, to set out.
        if strange == here {
            return Some(Action::Taste);
        }

        if !rand::thread_rng().gen_bool(Self::HOW_OFTEN_ANYBODY_RISKS_IT) {
            return None;
        }

        Some(Action::Move {
            target: (strange.x, strange.y, agent_position.2),
        })
    }

    /// What a cutting of a named crop is called in a pack
    fn a_cutting_of(called: &str) -> String {
        format!("{called}cutting")
    }

    /// The cutting this agent is carrying, if any
    fn a_cutting_in_the_pack(
        agent: &crate::agents::Agent,
    ) -> Option<(&'static str, crate::world::ResourceType)> {
        Self::what_can_be_sown()
            .into_iter()
            .find(|(called, _, _)| agent.how_many_i_have(&Self::a_cutting_of(called)) > 0)
            .map(|(called, crop, _)| (called, crop))
    }

    /// The nearest field within reach that has gone over to weeds and pests
    fn field_wanting_work(
        &self,
        position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);
        let reach = Self::FIELD_WALK_RADIUS as i32;

        let mut best: Option<(Position, u32)> = None;

        for dx in -reach..=reach {
            for dy in -reach..=reach {
                let candidate = Position::new(from.x + dx, from.y + dy);

                let Some(tile) = self.world.grid.get_tile(&candidate) else {
                    continue;
                };

                if !tile.terrain.is_cultivated() || !tile.soil.wants_working() {
                    continue;
                }

                let distance = from.distance_to(&candidate);
                if distance > Self::FIELD_WALK_RADIUS {
                    continue;
                }

                if best.map(|(_, apart)| distance < apart).unwrap_or(true) {
                    best = Some((candidate, distance));
                }
            }
        }

        best.map(|(where_it_is, _)| where_it_is)
    }

    /// Getting dressed, in whatever order the situation needs: put on what is
    /// already made, make what there is material for, or go and gather it.
    ///
    /// Only a cold agent bothers. Insulation was always zero before this,
    /// because nothing ever drove an agent to make or wear anything, so cold
    /// was a thing agents endured for their whole lives rather than solved.
    ///
    /// With `immediate_only` this reports only what can be done on the spot,
    /// which is what outranks walking to shelter: pulling on a coat you are
    /// already carrying beats crossing a field to get out of the wind, and
    /// going off to cut flax does not.
    fn clothing_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        immediate_only: bool,
    ) -> Option<Action> {
        if !Self::wants_more_clothing(agent) {
            return None;
        }

        if let Some(garment) = Self::garment_to_put_on(agent) {
            return Some(Action::WearClothing { garment });
        }

        if let Some(garment) = Self::garment_to_make(agent) {
            return Some(Action::MakeClothing { garment });
        }

        if immediate_only {
            return None;
        }

        // Gathering reaches only as far as foraging does, so a patch further
        // off than that is somewhere to walk to first
        let (material, patch) = self.material_to_gather(agent, agent_position)?;

        let from = crate::world::Position::new(agent_position.0, agent_position.1);
        if from.distance_to(&patch) > Self::FORAGE_RADIUS {
            return Some(Action::Move {
                target: (patch.x, patch.y, agent_position.2),
            });
        }

        Some(Action::Gather {
            resource_type: material,
        })
    }

    /// Getting raw food onto a fire, in whatever order the situation needs:
    /// cook here, walk to the fire, light one, or go and cut the wood for it.
    ///
    /// Cooking is worth the trouble - raw meat gives up about a third of what
    /// is in it, cooked meat nearly all of it - but only for food a fire
    /// improves, so an agent carrying nothing but berries never lights one.
    fn cooking_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        if !Self::has_food_worth_cooking(agent) {
            return None;
        }

        // Standing at a fire that is burning: put the food on it
        if self
            .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
            .is_some()
        {
            return Some(Action::Cook {
                food_type: "generic".to_string(),
            });
        }

        // A fire burning within walking distance is worth the walk
        if let Some((_, position)) =
            self.nearest_fire_from(agent_position, Self::FIRE_WALK_RADIUS, true)
        {
            return Some(Action::Move { target: position });
        }

        // A cold hearth in reach costs only the fuel to bring back to life
        let relightable = self
            .nearest_fire_from(agent_position, Self::FIRE_REACH, false)
            .is_some();
        let wood_needed = if relightable {
            Self::FIRE_FUEL_WOOD
        } else {
            Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
        };

        if agent.inventory.has_item("wood", wood_needed) {
            return Some(Action::LightFire);
        }

        // No wood, but trees within reach: fetch some
        if self
            .nearest_resource_within(agent_position, Self::FORAGE_RADIUS, |resource| {
                resource.resource_type == crate::world::ResourceType::Wood
            })
            .is_some()
        {
            return Some(Action::Gather {
                resource_type: "wood".to_string(),
            });
        }

        None
    }

    /// Whether standing on this tile counts as being under cover
    fn is_shelter_tile(&self, position: &crate::world::Position) -> bool {
        use crate::world::TerrainType;

        let in_building = self
            .world
            .get_building_at(position)
            .map(|building| building.is_completed())
            .unwrap_or(false);

        let in_woodland = self
            .world
            .grid
            .get_tile(position)
            .map(|tile| matches!(tile.terrain.terrain_type, TerrainType::Forest))
            .unwrap_or(false);

        in_building || in_woodland
    }

    /// Closest cover the agent can actually walk to, by walking distance.
    ///
    /// Reachability rather than raw proximity: a hut across a lake is no use,
    /// and heading for one leaves the agent stepping back and forth in the
    /// weather instead of sheltering. `None` means there is nowhere to go, and
    /// the agent is better off getting on with something it can accomplish.
    fn nearest_shelter_from(&self, position: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;
        use std::collections::{HashSet, VecDeque};

        const MAX_VISITED: usize = 4096;

        let start = (position.0, position.1);

        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();

        queue.push_back(start);
        seen.insert(start);

        let mut visited = 0usize;

        while let Some(current) = queue.pop_front() {
            visited += 1;
            if visited > MAX_VISITED {
                break;
            }

            let candidate = Position::new(current.0, current.1);

            if self.is_shelter_tile(&candidate) {
                return Some(candidate);
            }

            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (current.0 + dx, current.1 + dy);

                if !seen.insert(next) {
                    continue;
                }

                if self.is_passable_tile(next.0, next.1) {
                    queue.push_back(next);
                }
            }
        }

        None
    }

    /// Position of the closest resource within `radius` walking steps that the
    /// agent has some use for
    fn nearest_resource_within(
        &self,
        position: (i32, i32, i32),
        radius: u32,
        wanted: impl Fn(&crate::world::ResourceNode) -> bool,
    ) -> Option<crate::world::Position> {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);

        self.world
            .resources
            .iter()
            .filter(|resource| resource.amount > 0 && wanted(resource))
            .map(|resource| (resource.position, from.distance_to(&resource.position)))
            .filter(|(_, distance)| *distance <= radius)
            .min_by_key(|(_, distance)| *distance)
            .map(|(position, _)| position)
    }

    /// Position of the closest edible resource within `radius` walking steps
    fn nearest_edible_within(
        &self,
        position: (i32, i32, i32),
        radius: u32,
    ) -> Option<crate::world::Position> {
        self.nearest_resource_within(position, radius, |resource| {
            Self::edible_item_for(resource.resource_type).is_some()
        })
    }

    /// Resource types an agent can eat straight from the land, paired with the
    /// inventory item they correspond to.
    ///
    /// Foraging accepts everything that smells of food, so an agent does not
    /// starve standing in a grain field because only berries counted as edible.
    fn edible_resources() -> [(crate::world::ResourceType, crate::world::ItemType); 4] {
        use crate::world::{ItemType, ResourceType};

        [
            (ResourceType::Food, ItemType::Food),
            (ResourceType::Grain, ItemType::Grain),
            (ResourceType::Fish, ItemType::Fish),
            (ResourceType::Meat, ItemType::Meat),
        ]
    }

    /// The inventory item a resource yields when eaten, if it is edible at all
    ///
    /// `ResourceType::is_edible` is the authority on whether something counts
    /// as food; this only says what it turns into in a pack.
    fn edible_item_for(resource: crate::world::ResourceType) -> Option<crate::world::ItemType> {
        if !resource.is_edible() {
            return None;
        }

        Self::edible_resources()
            .into_iter()
            .find(|(resource_type, _)| *resource_type == resource)
            .map(|(_, item_type)| item_type)
    }

    /// Emit the smells of the world to agents in range.
    ///
    /// Resource percepts are only ever derived from smell, so without this the
    /// agents never perceive resources at all. What carries how far is
    /// deliberate: a human nose is poor, and finds food mainly when the food is
    /// cooking or rotting. Agents find whole, raw food by looking instead.
    ///
    /// Three things give themselves away:
    /// - what lies on the ground, faintly, and mostly if it is flesh
    /// - food that has turned, wherever it is being carried
    /// - a lit fire with something in it, which carries furthest of all
    /// Take food off the fire once it has had its time there.
    ///
    /// The heat sources were built for smelting, where contents sit until a
    /// recipe consumes them. Food has no such recipe, so without this a fire
    /// that once had a meal on it would smell of cooking for the rest of the
    /// run.
    fn clear_finished_cooking(&mut self) {
        let cooking_time = Self::COOKING_SMELL_TICKS;

        for heat_source in self.world.heat_sources.all_mut() {
            heat_source.contents.retain(|content| {
                let is_food = crate::agents::storage_integration::id_to_item_type(
                    &content.material_id,
                )
                .map(|item_type| {
                    item_type.cooking_outcome() != crate::world::nutrition::CookingOutcome::NotFood
                })
                .unwrap_or(false);

                !is_food || content.heating_time < cooking_time
            });
        }
    }

    fn emit_scents(&mut self) {
        use crate::agents::senses::{Scent, ScentType};

        let sources = self.collect_scent_sources();

        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let agent_pos = agent.state.position;

            // Scents are re-derived from the world every tick, so the previous
            // set is dropped first. Appending instead would pile up thousands
            // of duplicates, and stale ones would keep rebuilding memories of
            // patches that no longer exist.
            agent.senses.smell.detected_scents.retain(|scent| {
                !matches!(
                    scent.scent_type,
                    ScentType::Food | ScentType::Water | ScentType::Decay
                )
            });

            for (source_position, scent_type, strength) in &sources {
                if agent.senses.smell.can_smell(agent_pos, *source_position, *strength) {
                    agent.senses.smell.detect_scent(Scent {
                        source_position: *source_position,
                        scent_type: scent_type.clone(),
                        strength: *strength,
                        age: 0,
                    });
                }
            }
        }
    }

    /// Everything in the world currently giving off a smell
    fn collect_scent_sources(
        &self,
    ) -> Vec<((i32, i32, i32), crate::agents::senses::ScentType, f32)> {
        use crate::agents::senses::ScentType;
        use crate::world::ResourceType;

        let mut sources = Vec::new();

        // What lies on the ground. Berries on the bush are close to odourless,
        // so an agent finds those by looking rather than by sniffing.
        for resource in &self.world.resources {
            if resource.amount == 0 {
                continue;
            }

            let strength = resource.resource_type.raw_scent_strength();
            if strength <= 0.0 {
                continue;
            }

            let scent_type = if resource.resource_type == ResourceType::Water {
                ScentType::Water
            } else {
                ScentType::Food
            };

            sources.push((
                (resource.position.x, resource.position.y, 0),
                scent_type,
                strength,
            ));
        }

        // Food that has turned announces itself, wherever it is being carried.
        // This is decay rather than food: it says something is rotten here, and
        // does not send an agent over to eat it.
        for agent in &self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let rot = agent
                .inventory
                .get_all_items()
                .iter()
                .filter_map(|(_, item)| item.food_data.as_ref())
                .filter(|food| food.is_rotting() || food.is_ruined())
                .map(|food| food.scent_strength())
                .fold(0.0_f32, f32::max);

            if rot > 0.0 {
                sources.push((agent.state.position, ScentType::Decay, rot));
            }
        }

        // A midden. "Waste should smell unpleasant and repulse the agents":
        // this is the smell of it. It reaches further than a berry does and
        // nowhere near as far as a cooking fire, which is about right for
        // something you notice when you are nearly standing in it.
        for (y, row) in self.world.grid.tiles.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                if !tile.soil.is_foul() {
                    continue;
                }

                let here = (x as i32, y as i32, 0);
                let strength = (tile.soil.fouling
                    / crate::world::soil::Soil::AS_FOUL_AS_IT_GETS)
                    .clamp(0.0, 1.0);
                sources.push((here, ScentType::Decay, strength));
            }
        }

        // A lit fire with food in it: the strongest smell there is, and the one
        // a nose is really for.
        //
        // Nothing lights a fire or puts food in one yet, so this source is
        // dormant in a live run - see ISSUES_FOUND.md.
        for heat_source in self.world.heat_sources.all() {
            if !heat_source.is_lit || heat_source.contents.is_empty() {
                continue;
            }

            sources.push((heat_source.position, ScentType::Food, 1.0));
        }

        sources
    }

    /// Whether an agent can stand on this tile
    fn is_passable_tile(&self, x: i32, y: i32) -> bool {
        use crate::world::{Position, TerrainType};

        if x < 0
            || x >= self.world.grid.width as i32
            || y < 0
            || y >= self.world.grid.height as i32
        {
            return false;
        }

        let pos = Position::new(x, y);

        if let Some(tile) = self.world.grid.get_tile(&pos) {
            if tile.terrain.terrain_type == TerrainType::Water {
                return false;
            }
        }

        // A finished building is somewhere to go, not an obstacle: agents take
        // shelter by standing in one, so refusing to walk onto its tile makes
        // shelter unreachable by any route the pathfinder will take. A site
        // still under construction is scaffolding, and does block.
        if let Some(building) = self.world.get_building_at(&pos) {
            return building.is_completed();
        }

        // Resources sit on the ground rather than walling it off - a berry
        // patch or a stand of trees is somewhere to walk to, not around, and
        // treating them as solid cuts agents off from woodland shelter.
        true
    }

    /// First step of a route from `from` to `target`, routing around obstacles.
    ///
    /// A breadth-first search over passable tiles, bounded so a walled-off
    /// destination cannot cost an unbounded scan. Stepping greedily toward the
    /// target instead traps agents against terrain: a lake between an agent
    /// and a berry patch leaves it stepping east, west, east forever, which is
    /// fatal when it is the trip to food that stalls.
    fn next_step_toward(
        &self,
        from: (i32, i32, i32),
        target: (i32, i32, i32),
    ) -> Option<(i32, i32, i32)> {
        use std::collections::{HashMap, VecDeque};

        const MAX_VISITED: usize = 4096;

        let start = (from.0, from.1);
        let goal = (target.0, target.1);

        if start == goal {
            return None;
        }

        let mut queue = VecDeque::new();
        let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

        queue.push_back(start);
        came_from.insert(start, start);

        let mut visited = 0usize;

        while let Some(current) = queue.pop_front() {
            visited += 1;
            if visited > MAX_VISITED {
                break;
            }

            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (current.0 + dx, current.1 + dy);

                if came_from.contains_key(&next) {
                    continue;
                }

                // The goal tile itself may hold a building or resource the
                // agent is heading for, so only intermediate tiles must be
                // walkable.
                if next != goal && !self.is_passable_tile(next.0, next.1) {
                    continue;
                }

                came_from.insert(next, current);

                if next == goal {
                    let mut step = next;
                    while came_from[&step] != start {
                        step = came_from[&step];
                    }
                    return Some((step.0, step.1, from.2));
                }

                queue.push_back(next);
            }
        }

        None
    }

    /// Replace a move toward somewhere the agent cannot actually reach.
    ///
    /// A remembered patch can sit behind a lake or inside a pocket of terrain.
    /// Walking greedily at an unreachable target leaves the agent shuffling
    /// between two tiles forever, so the memory is dropped as unusable and the
    /// next-best survival option taken instead.
    fn retarget_unreachable_move(&mut self, agent_index: usize, action: Action) -> Action {
        use crate::core::memory::SpatialMemoryType;

        let mut action = action;

        // Bounded: each pass drops one memory, so this cannot spin
        for _ in 0..4 {
            let target = match &action {
                Action::Move { target } => *target,
                _ => return action,
            };

            let position = self.population.agents[agent_index].state.position;

            if target == position || self.next_step_toward(position, target).is_some() {
                return action;
            }

            let forgotten = {
                let agent = &mut self.population.agents[agent_index];
                agent.memory.forget_location(SpatialMemoryType::Food, target)
            };

            if !forgotten {
                return action;
            }

            debug!(
                "Agent {} cannot reach remembered food at {:?}, forgetting it",
                self.population.agents[agent_index].id, target
            );

            let agent = &self.population.agents[agent_index];
            match self.survival_action(agent, position, false) {
                Some(next_action) => action = next_action,
                None => return action,
            }
        }

        action
    }

    /// Drop food memories near the agent after a fruitless search there.
    ///
    /// Resource nodes are removed once exhausted, so an agent that walks to a
    /// remembered berry patch and finds nothing would otherwise keep walking
    /// back to the same empty spot until it starved.
    fn forget_nearby_food_memories(&mut self, agent_index: usize) {
        use crate::core::memory::SpatialMemoryType;

        let agent = &mut self.population.agents[agent_index];
        let pos = agent.state.position;

        let stale: Vec<(i32, i32, i32)> = agent
            .memory
            .recall_locations(SpatialMemoryType::Food)
            .into_iter()
            .map(|memory| memory.position)
            .filter(|remembered| {
                (remembered.0 - pos.0).abs() + (remembered.1 - pos.1).abs() <= 3
            })
            .collect();

        for position in stale {
            agent.memory.forget_location(SpatialMemoryType::Food, position);
        }
    }

    /// Whether the agent is currently standing in a completed building
    fn agent_has_shelter(&self, agent_index: usize) -> bool {
        use crate::world::Position;

        let agent = &self.population.agents[agent_index];
        let pos = Position::new(agent.state.position.0, agent.state.position.1);

        self.world
            .get_building_at(&pos)
            .map(|building| building.is_completed())
            .unwrap_or(false)
    }

    /// Whether the hands doing this have what the verb wants in them.
    ///
    /// Returns what is missing, or `None` when the action can go ahead. The
    /// requirement comes from the matrix rather than from here: this function
    /// knows how to ask an agent what it is holding and nothing else about
    /// which verbs want what.
    fn what_these_hands_are_short_of(
        &self,
        action: &Action,
        agent_index: usize,
    ) -> Option<String> {
        use crate::environment::verbs;

        let agent = &self.population.agents[agent_index];

        // The action's bare name, which is how the matrix refers to it:
        // "gather:wood" is a gather
        let tried = crate::agents::Agent::what_was_tried(action);
        let named = tried.split(':').next().unwrap_or(&tried);

        let wanted = verbs::what_this_action_cannot_do_without(named);
        if wanted.is_empty() {
            return None;
        }

        let holding = |what: &str| agent.how_many_i_have(what);
        let helped_by = |trade| agent.what_i_have_to_work_with(trade).is_some();
        let a_hand_to_spare = agent.a_hand_to_spare();
        let carrying_liquid = agent.how_much_water_i_carry();

        wanted
            .into_iter()
            .find(|wants| {
                !wants.satisfied_by_hands(
                    &holding,
                    &helped_by,
                    a_hand_to_spare,
                    carrying_liquid,
                )
            })
            .map(|wants| match wants {
                verbs::Wants::ThisInHand(what) => format!("No {what} in hand for that"),
                verbs::Wants::AToolFor(trade) => {
                    format!("Nothing in hand that is any use for {}", trade.name())
                }
                verbs::Wants::AFreeHand => "Both hands full".to_string(),
                verbs::Wants::AVessel => "Nothing to hold water in".to_string(),
                verbs::Wants::BareHands => "Nothing wanting".to_string(),
            })
    }

    fn execute_action(&mut self, action: &Action, agent_index: usize) -> ActionResult {
        use rand::Rng;
        use crate::world::nutrition::CookingOutcome;
        use crate::world::Position;

        let mut rng = rand::thread_rng();

        // Doing the work is what keeps a hand in it - see
        // `Skills::let_unused_skills_rust`
        let tick_now = self.current_tick;

        // What the verb matrix says this cannot be done without. One check
        // here, from one table, rather than thirty arms each deciding for
        // themselves whether a man needs a knife to skin something - see
        // `environment::verbs`.
        if let Some(missing) = self.what_these_hands_are_short_of(action, agent_index) {
            return ActionResult::failure(missing);
        }

        match action {
            Action::Eat { food_type } => {
                // PRIORITY 1: eat food the agent is already carrying.
                //
                // Agents gather food into their inventory long before they are
                // hungry; without this they would starve while fully stocked.
                let agent = &mut self.population.agents[agent_index];
                let carried_food = agent.find_best_food_to_eat().or_else(|| {
                    agent
                        .inventory
                        .get_item("food")
                        .filter(|item| item.quantity > 0 && item.food_data.is_none())
                        .map(|item| item.item_id.clone())
                });

                if let Some(item_id) = carried_food {
                    match agent.eat_food_item(&item_id, self.current_tick) {
                        EatResult::Success(nutrition) => {
                            debug!(
                                "Agent {} ate carried {} ({:.1} energy, {:.1} protein), reset starvation timer",
                                agent.id, item_id, nutrition.energy, nutrition.protein
                            );

                            return ActionResult::success()
                                .with_drive_change(DriveType::Hunger, -0.3)
                                .with_energy_cost(1.0) // Eating from inventory is cheap
                                .with_message(format!(
                                    "Ate carried {} ({:.1} energy restored)",
                                    item_id, nutrition.energy
                                ));
                        }
                        EatResult::MadeSick(damage) => {
                            return ActionResult::failure(format!(
                                "Ate spoiled {} and got sick ({:.1} damage)",
                                item_id, damage
                            ));
                        }
                        // Spoiled/NoFood fall through to foraging below
                        EatResult::Spoiled | EatResult::NoFood => {}
                    }
                }

                // PRIORITY 2: forage from a nearby food resource node
                let agent = &self.population.agents[agent_index];
                let agent_pos = Position::new(
                    agent.state.position.0,
                    agent.state.position.1
                );

                // Look for anything edible within a 25-tile radius
                let mut nearest_food: Option<(usize, u32)> = None;
                for (i, resource) in self.world.resources.iter().enumerate() {
                    if Self::edible_item_for(resource.resource_type).is_some() && resource.amount > 0 {
                        let distance = agent_pos.distance_to(&resource.position);
                        if distance <= Self::FORAGE_RADIUS {
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
                        let agent = &mut self.population.agents[agent_index];

                        // Foraged food carries real nutrition, so eating it
                        // refills the nutritional reserves that metabolism
                        // draws down rather than only the felt-energy value.
                        let foraged_item = Self::edible_item_for(
                            self.world.resources[food_index].resource_type,
                        )
                        .unwrap_or(crate::world::ItemType::Food);

                        let nutrition = self
                            .food_database
                            .get(&foraged_item)
                            .map(|template| template.base_nutrition)
                            .unwrap_or_else(|| NutritionalContent::new(20.0, 5.0, 35.0, 0.8));

                        agent.nutrition.consume(&nutrition);
                        agent.state.eat(self.current_tick, nutrition.energy);

                        // Foraged fruit and berries carry water too
                        if nutrition.water_content > 0.3 {
                            if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                                thirst.decrease(nutrition.water_content * 0.1);
                            }
                        }

                        debug!(
                            "Agent {} foraged and ate food, restored {:.1} energy, reset starvation timer",
                            agent.id, nutrition.energy
                        );

                        ActionResult::success()
                            .with_drive_change(DriveType::Hunger, -0.3)
                            .with_energy_cost(5.0) // Small energy cost to gather/eat
                            .with_message(format!("Ate {} and restored {:.1} energy", food_type, nutrition.energy))
                    } else {
                        ActionResult::failure("Food source was empty".to_string())
                    }
                } else {
                    // No food nearby, agent fails to eat
                    self.forget_nearby_food_memories(agent_index);
                    ActionResult::failure("No food sources nearby".to_string())
                }
            },

            Action::Sleep { duration } => {
                let current_tick = self.current_tick;
                let has_shelter = self.agent_has_shelter(agent_index);
                let agent = &mut self.population.agents[agent_index];

                // Sleep quality depends on the agent's circumstances
                let quality_factors = crate::agents::fatigue::SleepQualityFactors {
                    has_shelter,
                    has_bed: has_shelter,
                    safety: 1.0 - agent.emotions.fear.min(1.0),
                    health: (agent.state.health / 100.0).clamp(0.0, 1.0),
                    hunger: agent
                        .drives
                        .get(DriveType::Hunger)
                        .map(|d| d.value)
                        .unwrap_or(0.0),
                    comfort: 0.5,
                };

                // Actually recover fatigue rather than only topping up energy;
                // without this the agent's fatigue never falls, so an exhausted
                // agent re-selects Sleep every tick and never does anything else.
                let energy_before = agent.state.energy;
                let mut fatigue_recovered = 0.0;
                for _ in 0..(*duration).max(1) {
                    fatigue_recovered += agent.sleep_tick(current_tick, &quality_factors);
                }
                agent.wake_up(current_tick);

                let energy_restored = agent.state.energy - energy_before;

                ActionResult::success()
                    .with_drive_change(DriveType::Rest, -0.5)
                    .with_message(format!(
                        "Slept for {} ticks, recovered {:.2} fatigue and {:.1} energy",
                        duration, fatigue_recovered, energy_restored
                    ))
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
                    // Wild grain stands in the world and there was no way to
                    // ask for it by name: a request for grain fell through to
                    // "unknown resource type" and failed. It came back only as
                    // an edible substitute for a request for food, which is
                    // how a people that had never handled grain came to have
                    // none of it to sow.
                    "grain" => Some(ResourceType::Grain),
                    "water" => Some(ResourceType::Water),
                    // Clothing materials. Flax and cotton grow in patches an
                    // agent can walk to; hides and wool come off animals, so
                    // they are here for when an agent has somewhere to get
                    // them rather than because the ground offers any.
                    "flax" => Some(ResourceType::Flax),
                    "cotton" => Some(ResourceType::Cotton),
                    "hides" => Some(ResourceType::Hides),
                    "wool" => Some(ResourceType::Wool),
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

                // Look for resources within a 25-tile radius. A request for
                // food accepts anything edible, so foraging is not limited to
                // generic berries when grain or fish is what is growing here.
                let gathering_food = resource_type_enum == ResourceType::Food;

                let mut nearest_resource: Option<(usize, u32)> = None;
                // What this particular person will accept as food. Everybody
                // takes berries; only somebody who has seen a strange plant
                // eaten and survived will pick one.
                let knows_it_is_food = |resource: &crate::world::ResourceNode| {
                    resource.resource_type == ResourceType::StrangePlant
                        && self.population.agents[agent_index].is_that_plant_food(resource.kind)
                };

                for (i, resource) in self.world.resources.iter().enumerate() {
                    let matches_request = resource.resource_type == resource_type_enum
                        || (gathering_food
                            && (Self::edible_item_for(resource.resource_type).is_some()
                                || knows_it_is_food(resource)));

                    if matches_request && resource.amount > 0 {
                        let distance = agent_pos.distance_to(&resource.position);
                        if distance <= Self::FORAGE_RADIUS {
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
                    // The harvested node may be an edible substitute for a
                    // generic food request, so classify by what was found.
                    // A strange plant somebody has established is food is
                    // food from here on: it goes in the pack and feeds people
                    // like anything else that grows.
                    let resource_type_enum = match self.world.resources[resource_index].resource_type
                    {
                        ResourceType::StrangePlant => ResourceType::Food,
                        found => found,
                    };

                    // What an ordinary pair of hands brings back in a trip
                    let ordinary = match resource_type_enum {
                        ResourceType::Wood => rng.gen_range(1..=3),
                        // An armful at a time, like wood: a garment's worth of
                        // flax one stem per trip is a week's work
                        ResourceType::Flax | ResourceType::Cotton => rng.gen_range(1..=3),
                        ResourceType::Stone => rng.gen_range(1..=2),
                        ResourceType::Iron => 1,
                        ResourceType::Food => 1,
                        _ => 1,
                    };

                    // And what these particular hands make of it. The comment
                    // here used to say "based on resource type and skill" and
                    // the skill was not consulted: a lifetime of farming
                    // brought back exactly what a first day did. A practised
                    // hand knows which plants are worth stripping and how to
                    // take a crop without ruining what is left, and brings
                    // back up to twice what a beginner does.
                    let trade = Self::trade_for_gathering(resource_type_enum);
                    let hand = self.population.agents[agent_index].skills.hand_for(trade);

                    // And what he has in his hands while he does it. A stone
                    // axe was, until now, a thing an agent counted and nothing
                    // else: a man carrying one felled timber at exactly the
                    // rate of a man with his bare hands.
                    let tool = self.population.agents[agent_index].how_much_my_tools_help(trade);

                    let worth = ordinary as f32 * hand * tool;

                    // Carry the fraction as a chance rather than rounding it
                    // away, so that a small difference in skill still tells
                    // over a season of trips
                    let whole = worth.floor();
                    let harvest_amount =
                        (whole as u32) + u32::from(rng.gen::<f32>() < worth - whole);
                    let harvest_amount = harvest_amount.max(1);

                    // Harvest resource
                    let where_it_grew = self.world.resources[resource_index].position;
                    let harvested = self.world.resources[resource_index].harvest(harvest_amount);

                    // What a crop off broken ground teaches. Nobody is born
                    // believing that seed put in the ground on purpose comes
                    // back as food; carrying an armful home off a field is the
                    // evidence that settles it.
                    if harvested > 0
                        && self
                            .world
                            .grid
                            .get_tile(&where_it_grew)
                            .map(|tile| tile.terrain.is_cultivated())
                            .unwrap_or(false)
                    {
                        let agent = &mut self.population.agents[agent_index];
                        agent
                            .practices
                            .record_outcome(crate::agents::practices::Practice::Farming, true);

                        // And which plant it was that repaid the work. This is
                        // the whole of how a people finds out that grain is
                        // worth sowing and a berry bush is not: it sowed both
                        // and kept count of what it carried home.
                        if let Some((called, _, _)) = Self::what_can_be_sown()
                            .into_iter()
                            .find(|(_, crop, _)| *crop == resource_type_enum)
                        {
                            agent
                                .lessons
                                .record_particular(&format!("sow:{called}"), true);
                        }
                    }

                    // Stone and wood go quickly, and a trip out for timber is
                    // one more trip an axe will not make again.
                    if harvested > 0 && tool > 1.0 {
                        if let Some(broke) =
                            self.population.agents[agent_index].wear_what_i_worked_with(trade)
                        {
                            debug!(
                                "Agent {} wore out a {broke}",
                                self.population.agents[agent_index].id
                            );
                        }
                    }

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
                            ResourceType::Grain => "grain",
                            ResourceType::Fish => "fish",
                            ResourceType::Meat => "meat",
                            ResourceType::Flax => "flax",
                            ResourceType::Cotton => "cotton",
                            ResourceType::Hides => "hides",
                            ResourceType::Wool => "wool",
                            ResourceType::Herbs => "herbs",
                            _ => "generic",
                        };

                        let mut item = InventoryItem::new_with_weight(
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

                        // Gathered food carries nutrition and spoils over time
                        if let Some(item_type) = Self::edible_item_for(resource_type_enum) {
                            item.food_data = self
                                .food_database
                                .create_food_data(&item_type, self.current_tick);
                        }

                        let agent = &mut self.population.agents[agent_index];
                        if agent.inventory.add_item(item) {
                            // Grant skill XP based on resource type
                            let skill_type = Self::trade_for_gathering(resource_type_enum);
                            // A trip out is the commonest thing anybody does
                            // and the whole of some people's trade, so it is
                            // what the climb is sized against
                            agent.skills.practise(skill_type, 8, tick_now);

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
                    // No source in range. Water can still be drunk from a
                    // waterskin, which is the whole point of carrying one -
                    // an agent crossing dry ground should not go thirsty with
                    // a full flask on its belt.
                    if resource_type_enum == ResourceType::Water {
                        let current_tick = self.current_tick;
                        let agent = &mut self.population.agents[agent_index];

                        if agent.inventory.available_water() > 0.0 {
                            let drunk = agent.drink_water(1.0);

                            if drunk {
                                agent.state.drink(current_tick);

                                debug!("Agent {} drank from its own container", agent.id);

                                return ActionResult::success()
                                    .with_drive_change(DriveType::Thirst, -0.2)
                                    .with_energy_cost(1.0)
                                    .with_message("Drank from a carried container".to_string());
                            }
                        }
                    }

                    if resource_type_enum == ResourceType::Food {
                        self.forget_nearby_food_memories(agent_index);
                    }

                    ActionResult::failure(format!("No {} sources nearby", resource_type))
                }
            },

            Action::Build { structure_type, position } => {
                use crate::world::{BuildingType, Building, Position, ResourceType};

                // Map structure string to BuildingType
                let building_type = match structure_type.as_str() {
                    // What "put up a shelter" means to people who have hides
                    // and poles and no quarry. Every house in the list needs
                    // stone, the cheapest of them thirty, and a settlement
                    // that has never had a single block of it spent an eighth
                    // of its life saying so.
                    "tent" | "skintent" => BuildingType::SkinTent,
                    "shelter" => BuildingType::SkinTent,
                    "smallhouse" => BuildingType::SmallHouse,
                    "mediumhouse" => BuildingType::MediumHouse,
                    "largehouse" => BuildingType::LargeHouse,
                    "workshop" => BuildingType::Workshop,
                    "storehouse" => BuildingType::Storehouse,
                    "farm" => BuildingType::Farm,
                    "structure" => BuildingType::SkinTent,
                    _ => BuildingType::SkinTent,
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
                agent.skills.practise(crate::agents::skills::SkillType::Construction, construction_xp, tick_now);

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

                // And what it does to what the two of them are to each other.
                //
                // The executor dealt damage, wrote anger and broke a bone, and
                // never touched the relationship, so a man who had just been
                // hit went on counting the man who hit him a close friend and
                // the settlement graph had no hostile edge anywhere in it.
                {
                    use crate::agents::Relationship;

                    let current_tick = self.current_tick;

                    let struck = self.population.agents[target_index]
                        .relationships
                        .get_or_create_relationship(attacker_id, current_tick);
                    struck.weaken(Relationship::WHAT_A_BLOW_COSTS);
                    struck.settle_what_we_are();

                    // You do not warm to somebody you have just hit either
                    let striking = self.population.agents[agent_index]
                        .relationships
                        .get_or_create_relationship(target_id, current_tick);
                    striking.weaken(Relationship::WHAT_THROWING_ONE_COSTS);
                    striking.settle_what_we_are();
                }

                // Check if target died from the attack
                let target_alive = self.population.agents[target_index].body.is_alive()
                    && self.population.agents[target_index].state.health > 0.0;

                // What each of them takes away from it.
                //
                // "If an agent has fought back and won, then fighting becomes
                // a more attractive option. If an agent has fought back and
                // lost, then running away becomes a more attractive option."
                // The record is kept in the same place every other lesson is,
                // and it moves what the agent reckons itself worth the next
                // time something comes at it - see `Agent::own_strength`.
                {
                    use crate::agents::practices::Undertaking;

                    let attacker_standing = self.population.agents[agent_index]
                        .state
                        .health;
                    let target_standing = self.population.agents[target_index]
                        .state
                        .health;

                    // The attacker has won if the other one is down, and is
                    // losing if it is the worse off of the two
                    let attacker_won = !target_alive || attacker_standing > target_standing;

                    self.population.agents[agent_index]
                        .lessons
                        .record(Undertaking::Fighting, attacker_won);

                    // And the one being set upon learns from standing there
                    // just as much. Being alive at the end of it is the whole
                    // of winning, from that side.
                    if target_alive {
                        self.population.agents[target_index]
                            .lessons
                            .record(Undertaking::Fighting, !attacker_won);
                    } else {
                        self.population.agents[target_index]
                            .lessons
                            .record(Undertaking::Fighting, false);
                    }
                }

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
                attacker.skills.practise(crate::agents::skills::SkillType::MeleeCombat, combat_xp, tick_now);

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
                // The stone-age chain comes first. These steps take named
                // things and turn out named things, so what one step produces
                // the next can pick up; the table below it cannot express that,
                // because its inputs are only ever things dug out of the ground.
                if crate::environment::making::is_made_not_found(item_type) {
                    let step = {
                        let agent = &self.population.agents[agent_index];
                        let holding = |what: &str| agent.how_many_i_have(what);

                        // A step nobody has found out is not a step this
                        // agent can take, whatever is in his pack.
                        if !agent.knows_how_to_make(item_type) {
                            return ActionResult::failure(format!(
                                "Nobody here knows how to make a {}",
                                item_type
                            ));
                        }

                        match crate::environment::making::every_way_to_make(item_type)
                            .filter(|step| agent.knows_how_to(step))
                            .filter(|step| {
                                step.wants_in_hand.is_none_or(|wanted| {
                                    agent.how_many_i_have(wanted) > 0
                                })
                            })
                            .find(|step| step.makings_to_hand(&holding))
                        {
                            Some(step) => *step,
                            None => {
                                // Say what is missing rather than that it cannot
                                // be done: the shortfall is the next job.
                                let short = crate::environment::making::every_way_to_make(item_type)
                                    .filter(|step| agent.knows_how_to(step))
                                    .filter_map(|step| step.short_of(&holding))
                                    .min_by_key(|(_, missing)| *missing);

                                return match short {
                                    Some((what, how_many)) => ActionResult::failure(format!(
                                        "Cannot make {}: short {} {}",
                                        item_type, how_many, what
                                    )),
                                    None => {
                                        // Everything is in the pack, so what
                                        // is missing is the thing to do it
                                        // with.
                                        let wanted = crate::environment::making::every_way_to_make(
                                            item_type,
                                        )
                                        .filter(|step| agent.knows_how_to(step))
                                        .find_map(|step| step.wants_in_hand);

                                        match wanted {
                                            Some(tool) => ActionResult::failure(format!(
                                                "Cannot make {}: nothing to do it with, wants a {}",
                                                item_type, tool
                                            )),
                                            None => ActionResult::failure(format!(
                                                "Cannot make {}",
                                                item_type
                                            )),
                                        }
                                    }
                                };
                            }
                        }
                    };

                    if step.over_a_fire {
                        let where_he_is = self.population.agents[agent_index].state.position;
                        if self
                            .nearest_fire_from(where_he_is, Self::FIRE_REACH, true)
                            .is_none()
                        {
                            return ActionResult::failure(format!(
                                "Cannot make {}: no fire burning here",
                                item_type
                            ));
                        }
                    }

                    // What the work is done with is worn by the doing of it,
                    // and is not part of what comes out.
                    if let Some(wanted) = step.wants_in_hand {
                        if let Some(tool) = crate::environment::making::EVERY_TOOL
                            .iter()
                            .find(|tool| tool.called == wanted)
                        {
                            self.population.agents[agent_index]
                                .wear_what_i_worked_with(tool.helps);
                        }
                    }

                    let agent = &mut self.population.agents[agent_index];
                    for (what, how_many) in step.needs {
                        agent.inventory.remove_item(what, *how_many);
                    }

                    // A thing that took more doing is the heavier thing to
                    // carry, and a thing made by a better hand is a better
                    // thing: it lasts longer and it works better.
                    // A worn-through one of the same thing is thrown away
                    // rather than stacked with the new one: stacking would
                    // hand the fresh tool the broken one's durability.
                    if agent
                        .inventory
                        .get_item(step.makes)
                        .is_some_and(|carried| carried.durability_percentage() <= 0.0)
                    {
                        let had = agent.inventory.count_item(step.makes);
                        agent.inventory.remove_item(step.makes, had);
                    }

                    let made = agent.a_tool_fresh_from_these_hands(
                        step.makes,
                        step.how_many,
                        step.effort / 4.0,
                    );
                    if !agent.inventory.add_item(made) {
                        debug!(
                            "Agent {} made {} but had nowhere to put it",
                            agent.id, step.makes
                        );
                    }

                    {
                        // A spear teaches more than a length of cordage does,
                        // in the proportion the two cost to do.
                        let learned = (step.effort / 4.0).round().max(1.0) as u32;
                        let skill = agent.skills.get_skill_mut(step.hands);
                        skill.gain_experience(learned);
                        skill.last_used = tick_now;
                    }

                    return ActionResult::success()
                        .with_drive_change(DriveType::Utility, -0.2)
                        .with_energy_cost(step.effort)
                        .with_message(format!("Made {} {}", step.how_many, step.makes));
                }

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

                {
                    let skill = agent.skills.get_skill_mut(SkillType::Crafting);
                    skill.gain_experience(experience_gained);
                    skill.last_used = tick_now;
                }

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
                //
                // Candidates are ordered best-first: the direct step, then the
                // other axis, then a sidestep. Without the fallbacks an agent
                // whose direct route runs into a lake retries the same blocked
                // step forever, which strands it (and, if it was on its way to
                // food, starves it).
                let mut candidates: Vec<(i32, i32, i32)> = Vec::new();
                let push = |candidate: (i32, i32, i32), candidates: &mut Vec<(i32, i32, i32)>| {
                    if candidate != current_pos && !candidates.contains(&candidate) {
                        candidates.push(candidate);
                    }
                };

                let x_step = (current_pos.0 + step_x, current_pos.1, current_pos.2);
                let y_step = (current_pos.0, current_pos.1 + step_y, current_pos.2);
                let z_step = (current_pos.0, current_pos.1, current_pos.2 + step_z);

                if dx.abs() >= dy.abs() && dx.abs() >= dz.abs() {
                    push(x_step, &mut candidates);
                    push(y_step, &mut candidates);
                    push(z_step, &mut candidates);
                } else if dy.abs() >= dz.abs() {
                    push(y_step, &mut candidates);
                    push(x_step, &mut candidates);
                    push(z_step, &mut candidates);
                } else {
                    push(z_step, &mut candidates);
                    push(x_step, &mut candidates);
                    push(y_step, &mut candidates);
                }

                // Sidesteps perpendicular to the blocked direction, so agents
                // can work their way around an obstacle rather than stall
                for (side_x, side_y) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    push(
                        (current_pos.0 + side_x, current_pos.1 + side_y, current_pos.2),
                        &mut candidates,
                    );
                }

                // Take the direct step when it is clear; otherwise search for a
                // route around whatever is in the way before falling back to a
                // sidestep, so agents do not oscillate against an obstacle.
                let direct_step = candidates
                    .first()
                    .copied()
                    .filter(|candidate| self.is_passable_tile(candidate.0, candidate.1));

                let step = direct_step
                    .or_else(|| self.next_step_toward(current_pos, *target))
                    .or_else(|| {
                        candidates
                            .iter()
                            .copied()
                            .find(|candidate| self.is_passable_tile(candidate.0, candidate.1))
                    });

                let (next_x, next_y, next_z) = match step {
                    Some(step) => step,
                    None => {
                        return ActionResult::failure(
                            "No passable route toward destination".to_string(),
                        )
                    }
                };

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

                        // You have to be near enough to throw something at it.
                        // Without this an agent could kill a deer on the far
                        // side of the map without leaving where it stood.
                        let agent_position = self.population.agents[agent_index].state.position;
                        let reach = (animal.position.0 - agent_position.0)
                            .abs()
                            .max((animal.position.1 - agent_position.1).abs());

                        if reach > Self::HUNT_REACH {
                            return ActionResult::failure(format!(
                                "Too far to hunt: {} tiles away",
                                reach
                            ));
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

                    // Calculate success based on agent skill, weapon, and mount.
                    //
                    // This used to read MeleeCombat and have no floor, which
                    // made hunting self-defeating: an untrained agent has that
                    // skill at -10 and 0.5 + (-10 x 0.05) is zero, so the first
                    // kill an agent ever made created the skill and left it
                    // unable to hunt for the rest of its life. It reads the
                    // Hunting skill now, which existed and had no callers, and
                    // never falls below a fifth.
                    let agent = &self.population.agents[agent_index];
                    let hunting_skill = agent
                        .skills
                        .get_skill_if_exists(crate::agents::skills::SkillType::Hunting)
                        .map(|s| s.level)
                        .unwrap_or(-10);
                    // A spear in the hand, which is the whole of stone-age
                    // hunting. `weapon` is the older flag and still counts;
                    // what is in the pack counts for more, and counts for
                    // less as it wears.
                    let spear = agent.how_much_my_tools_help(crate::agents::skills::SkillType::Hunting);
                    let carried_flag: f32 = if weapon.is_some() { 0.2 } else { 0.0 };
                    let weapon_bonus = carried_flag.max((spear - 1.0) * 0.25);

                    // Get mounted combat bonus (hunting from horseback is advantageous!)
                    let mount_bonus = agent.transport.mounted_combat_bonus();

                    // Hunting is slow work. It used to land six throws in ten
                    // for anybody at all, which made a deer a thing you walked
                    // up to rather than a thing you stalked; a stone-age hunt
                    // is mostly missing. What makes the difference is the
                    // spear and the hand that throws it, not the walking up.
                    let success_prob = (Self::A_THROW_THAT_TELLS
                        + (hunting_skill as f32 * 0.03)
                        + weapon_bonus
                        + mount_bonus)
                        .clamp(0.1_f32, 0.9_f32);

                    if rng.gen_bool(success_prob as f64) {
                        // Successful hunt - damage the animal.
                        //
                        // A third of what it can take, not two thirds: one
                        // clean throw did not kill a bull, and a hunt that
                        // ends on the first hit is not a hunt.
                        let base_damage = species.health * Self::WHAT_ONE_THROW_TAKES_OUT_OF_IT;
                        let combat_multiplier = 1.0 + mount_bonus;
                        let damage = base_damage * combat_multiplier;
                        animal.take_damage(damage);
                        // A throw at a deer is one more throw the shaft will
                        // not take. Twenty-five or so, and it is firewood.
                        let wore_out = spear > 1.0;

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

                            let knife = self.population.agents[agent_index]
                                .how_much_my_tools_help(
                                    crate::agents::skills::SkillType::Leatherworking,
                                );
                            let butchered = self.butcher(&items_gained, knife);
                            let agent = &mut self.population.agents[agent_index];
                            for item in butchered {
                                agent.inventory.add_item(item);
                            }

                            // Both tools are one job further through their
                            // lives: the spear that was thrown and the flake
                            // that took the carcass apart.
                            if wore_out {
                                agent.wear_what_i_worked_with(
                                    crate::agents::skills::SkillType::Hunting,
                                );
                            }
                            if knife > 1.0 {
                                agent.wear_what_i_worked_with(
                                    crate::agents::skills::SkillType::Leatherworking,
                                );
                            }

                            // Increase hunting skill
                            let agent = &mut self.population.agents[agent_index];
                            agent
                                .skills
                                .practise(crate::agents::skills::SkillType::Hunting, 30, tick_now);

                            let mut result = ActionResult::success()
                                .with_drive_change(DriveType::Hunger, -0.4)
                                .with_energy_cost(Self::WHAT_A_THROW_COSTS)
                                .with_experience(5.0)
                                .with_message(format!("Successfully hunted {} and obtained materials", species.name));

                            // Add all items gained
                            for item in items_gained {
                                result = result.with_item_gained(item);
                            }
                            result
                        } else {
                            let agent = &mut self.population.agents[agent_index];
                            if wore_out {
                                agent.wear_what_i_worked_with(
                                    crate::agents::skills::SkillType::Hunting,
                                );
                            }
                            agent
                                .skills
                                .practise(crate::agents::skills::SkillType::Hunting, 10, tick_now);

                            // A wounded animal is not a meal. It used to
                            // answer a tenth of a hunger for nothing at all,
                            // which is a hunt that pays whether or not it
                            // works.
                            ActionResult::success()
                                .with_energy_cost(Self::WHAT_A_THROW_COSTS)
                                .with_message(format!("Wounded {} but it escaped", species.name))
                        }
                    } else {
                        // You learn something from the ones that get away
                        self.population.agents[agent_index]
                            .skills
                            .practise(crate::agents::skills::SkillType::Hunting, 10, tick_now);

                        // And a throw that misses is a spear on the ground
                        // somewhere out past where the animal was. Half the
                        // time it is close enough to walk over and pick up
                        // and half the time it is in the bracken; either way
                        // it is not in the hand any more, which is what makes
                        // a missed throw cost something besides the walk.
                        //
                        // This is the state-change half of `throw` in the verb
                        // matrix: what leaves the hand goes somewhere.
                        if spear > 1.0 && rng.gen_bool(Self::HOW_OFTEN_A_MISS_LOSES_THE_SHAFT) {
                            let stood = self.population.agents[agent_index].state.position;
                            let fell = crate::world::Position::new(
                                stood.0 + rng.gen_range(-3..=3),
                                stood.1 + rng.gen_range(-3..=3),
                            );

                            let thrown = self.population.agents[agent_index]
                                .inventory
                                .get_item("spear")
                                .cloned();

                            if let Some(mut thrown) = thrown {
                                thrown.quantity = 1;
                                self.population.agents[agent_index]
                                    .inventory
                                    .remove_item("spear", 1);
                                self.world.somebody_left_this(thrown, fell, tick_now);

                                debug!(
                                    "Agent {} threw and missed; the spear is at {fell:?}",
                                    self.population.agents[agent_index].id
                                );
                            }
                        }

                        // A rabbit runs. A boar turns round.
                        let fights_back = matches!(
                            species.behavior,
                            crate::environment::AnimalBehavior::Aggressive
                                | crate::environment::AnimalBehavior::Defensive
                                | crate::environment::AnimalBehavior::Territorial
                        );

                        if fights_back && species.attack_damage > 0.0 {
                            let agent = &mut self.population.agents[agent_index];
                            agent.take_damage(species.attack_damage);

                            return ActionResult::failure(format!(
                                "{} turned on the hunter ({:.0} damage)",
                                species.name, species.attack_damage
                            ))
                            .with_energy_cost(Self::WHAT_A_THROW_COSTS);
                        }

                        ActionResult::failure(format!("{} escaped", species.name))
                            .with_energy_cost(Self::WHAT_A_THROW_COSTS)
                    }
                } else {
                    ActionResult::failure("Animal not found".to_string())
                }
            },
            Action::Fight { animal_id, weapon } => {
                // Standing your ground. The agent is not after this thing's
                // skin - it is here because the thing is close enough to be a
                // problem and the agent reckons it can be driven off.
                let (species, animal_position) = {
                    let Some(animal) = self.world.animals.get(animal_id) else {
                        return ActionResult::failure("Nothing there to fight".to_string());
                    };
                    if !animal.is_alive() {
                        return ActionResult::failure("It is already dead".to_string());
                    }

                    let species_id = animal.species_id.clone();
                    let position = animal.position;
                    match self.world.animals.get_species(&species_id) {
                        Some(found) => (found.clone(), position),
                        None => return ActionResult::failure("Unknown creature".to_string()),
                    }
                };

                // You cannot fight what you cannot reach, and an agent that
                // stands its ground does not go looking - if the thing has
                // moved off, that is the fight over.
                let standing = self.population.agents[agent_index].state.position;
                let reach = (animal_position.0 - standing.0)
                    .abs()
                    .max((animal_position.1 - standing.1).abs());
                if reach > Self::HUNT_REACH {
                    return ActionResult::failure(format!(
                        "{} is {} tiles off",
                        species.name, reach
                    ));
                }

                // Whether the blow lands is what the agent is worth against
                // what the creature is worth, on the same scale the appraisal
                // used to decide to be here at all.
                let mine = self.population.agents[agent_index].own_strength();
                let condition = {
                    let animal = self.world.animals.get(animal_id);
                    animal
                        .map(|a| (a.current_health / species.health.max(1.0)).clamp(0.0, 1.0))
                        .unwrap_or(1.0)
                };
                let theirs = condition * (species.attack_damage / 20.0).clamp(0.1, 2.0);
                let odds = (mine / (mine + theirs).max(0.01)).clamp(0.1, 0.9);

                let landed = rng.gen_bool(odds as f64);

                // A blow struck in a fight teaches the arm that struck it,
                // whichever way the fight goes
                self.population.agents[agent_index].skills.practise(
                    crate::agents::skills::SkillType::MeleeCombat,
                    if landed { 25 } else { 10 },
                    tick_now,
                );

                if landed {
                    let hurt = 25.0 + mine * 25.0;
                    let killed = {
                        let Some(animal) = self.world.animals.get_mut(animal_id) else {
                            return ActionResult::failure("Nothing there to fight".to_string());
                        };
                        animal.take_damage(hurt);
                        !animal.is_alive()
                    };

                    // Winning without a mark on you is what teaches an agent
                    // that fighting is worth doing
                    self.population.agents[agent_index]
                        .lessons
                        .record(crate::agents::practices::Undertaking::Fighting, true);

                    if !killed {
                        return ActionResult::success()
                            .with_energy_cost(12.0)
                            .with_experience(3.0)
                            .with_message(format!("Beat {} back", species.name));
                    }

                    // What is killed in a fight is still worth butchering -
                    // a wolf driven off is a wolf, a wolf killed is a hide
                    let mut items_gained = Vec::new();
                    for drop in &species.drops {
                        if rng.gen_bool(drop.drop_chance as f64) {
                            let quantity =
                                rng.gen_range(drop.min_quantity..=drop.max_quantity);
                            items_gained.push(crate::environment::ItemStack {
                                material_id: drop.material_id.clone(),
                                quantity,
                            });
                        }
                    }

                    let knife = self.population.agents[agent_index]
                        .how_much_my_tools_help(
                            crate::agents::skills::SkillType::Leatherworking,
                        );
                    let butchered = self.butcher(&items_gained, knife);
                    {
                        let agent = &mut self.population.agents[agent_index];
                        for item in butchered {
                            agent.inventory.add_item(item);
                        }
                    }

                    let mut result = ActionResult::success()
                        .with_drive_change(DriveType::Safety, -0.3)
                        .with_energy_cost(20.0)
                        .with_experience(6.0)
                        .with_message(format!("Killed {}", species.name));
                    for item in items_gained {
                        result = result.with_item_gained(item);
                    }
                    result
                } else {
                    // It got the better of the exchange
                    let damage = species.attack_damage.max(1.0);
                    let agent = &mut self.population.agents[agent_index];
                    let came_off_well = damage < agent.state.health * Self::A_SCRATCH;
                    agent.take_damage(damage);
                    agent
                        .lessons
                        .record(crate::agents::practices::Undertaking::Fighting, came_off_well);

                    ActionResult::failure(format!(
                        "{} got the better of it ({:.0} damage)",
                        species.name, damage
                    ))
                    .with_energy_cost(12.0)
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
                        agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, tick_now);

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
                        agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, tick_now);

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
                            agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, tick_now);
                        } else {
                            agent.skills.practise(crate::agents::skills::SkillType::Mining, 2, tick_now);
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

                let nearest_shelter = self.nearest_shelter_from(agent_tuple_pos);

                // Move towards nearest shelter, routing around obstacles.
                // Stepping straight at it stalls against the first lake or
                // building in the way, which strands the agent in the weather
                // it was trying to escape.
                if let Some(shelter_pos) = nearest_shelter {
                    let target = (shelter_pos.x, shelter_pos.y, agent_tuple_pos.2);

                    match self.next_step_toward(agent_tuple_pos, target) {
                        Some(step) => {
                            let agent = &mut self.population.agents[agent_index];
                            agent.state.position = step;

                            ActionResult::success()
                                .with_drive_change(DriveType::Safety, -0.1)
                                .with_energy_cost(5.0)
                                .with_message(format!(
                                    "Moving towards shelter at ({}, {})",
                                    shelter_pos.x, shelter_pos.y
                                ))
                        }
                        None => ActionResult::failure("Path to shelter blocked".to_string()),
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
                    initiator.skills.practise(crate::agents::skills::SkillType::Social, 1, tick_now);

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

            Action::LightFire => {
                // A hearth is worth more than the wood in it, so an unlit fire
                // already standing here is relit rather than rebuilt.
                let agent_pos = self.population.agents[agent_index].state.position;

                let existing = self
                    .nearest_fire_from(agent_pos, Self::FIRE_REACH, false)
                    .map(|(id, _)| id);

                // Shavings catch where a log will not, so a hearth laid with
                // tinder under it takes half the timber to get going. This is
                // what scraping a stick is for - see `making::SCRAPE_A_STICK`.
                let has_tinder = self.population.agents[agent_index].how_many_i_have("tinder") > 0;

                let wood_needed = if existing.is_some() {
                    Self::FIRE_FUEL_WOOD
                } else if has_tinder {
                    (Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD).div_ceil(2)
                } else {
                    Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
                };

                {
                    let agent = &self.population.agents[agent_index];
                    if !agent.inventory.has_item("wood", wood_needed) {
                        return ActionResult::failure(format!(
                            "Not enough wood for a fire: needs {}",
                            wood_needed
                        ));
                    }
                }

                let builder = self.population.agents[agent_index].id;
                let fire_id = match existing {
                    Some(id) => id,
                    None => match self.world.build_heat_source(
                        crate::environment::HeatSourceType::Campfire,
                        agent_pos,
                        Some(builder),
                    ) {
                        Ok(id) => id,
                        Err(reason) => {
                            return ActionResult::failure(format!(
                                "Could not build a fire here: {}",
                                reason
                            ))
                        }
                    },
                };

                self.population.agents[agent_index]
                    .inventory
                    .remove_item("wood", wood_needed);

                let _ = self.world.add_fuel_to_heat_source(
                    &fire_id,
                    "wood".to_string(),
                    Self::FIRE_FUEL_WOOD as f32,
                );

                if let Err(reason) = self.world.light_heat_source(&fire_id) {
                    return ActionResult::failure(format!("Could not light the fire: {}", reason));
                }

                let agent = &mut self.population.agents[agent_index];
                agent
                    .skills
                    .gain_experience(crate::agents::SkillType::Cooking, 5);

                debug!("Agent {} lit a fire at {:?}", agent.id, agent_pos);

                ActionResult::success()
                    .with_energy_cost(4.0)
                    .with_message("Lit a fire".to_string())
            },

            Action::Cook { food_type } => {
                // Cooking needs a fire, and the agent has to be standing at it
                let agent_pos = self.population.agents[agent_index].state.position;
                let fire = self.nearest_fire_from(agent_pos, Self::FIRE_REACH, true);

                let (fire_id, _) = match fire {
                    Some(fire) => fire,
                    None => return ActionResult::failure("No lit fire within reach".to_string()),
                };

                // Which of the things it is carrying goes on the fire
                let chosen = {
                    let agent = &self.population.agents[agent_index];
                    match Self::cookable_item(agent, food_type) {
                        Some(item_id) => item_id,
                        None => {
                            return ActionResult::failure(
                                "Nothing worth putting on the fire".to_string(),
                            )
                        }
                    }
                };

                let item_type = crate::agents::storage_integration::id_to_item_type(&chosen);
                let outcome = item_type
                    .map(|item_type| item_type.cooking_outcome())
                    .unwrap_or(CookingOutcome::NotFood);

                if outcome == CookingOutcome::NotFood {
                    return ActionResult::failure(format!("{} is not food", chosen));
                }

                let current_tick = self.current_tick;
                let fresh_food_data = item_type
                    .and_then(|item_type| self.food_database.create_food_data(&item_type, current_tick));

                let agent = &mut self.population.agents[agent_index];

                // Watching a fire is a skill. Someone who has never done it
                // burns one meal in five; someone who has done it for years
                // burns none, so the same food is ruined or not depending on
                // who is standing over it.
                let practice = agent
                    .skills
                    .get_skill_if_exists(crate::agents::SkillType::Cooking)
                    .map(|skill| skill.level)
                    .unwrap_or(-10);
                let attentive = rng.gen::<f32>() >= Self::burn_chance(practice);

                let outcome = if attentive {
                    outcome
                } else {
                    CookingOutcome::Ruins
                };

                // Only what fits over the flames goes on. Cooking a whole pack
                // at once would mean one lapse of attention costing everything
                // an agent had gathered, and would leave it with no raw food
                // to fall back on.
                let carried = agent
                    .inventory
                    .get_item(&chosen)
                    .map(|item| item.quantity)
                    .unwrap_or(0);
                let quantity = carried.min(Self::COOK_BATCH);

                if quantity == 0 {
                    return ActionResult::failure(format!("No {} to cook", chosen));
                }

                let mut batch = match agent.inventory.remove_item(&chosen, quantity) {
                    Some(batch) => batch,
                    None => return ActionResult::failure(format!("No {} to cook", chosen)),
                };

                // Food gathered before the nutrition system knew about it can
                // reach the fire without any food data at all
                if batch.food_data.is_none() {
                    batch.food_data = fresh_food_data;
                }

                match batch.food_data.as_mut() {
                    Some(food) => {
                        food.cook(outcome);
                    }
                    None => {
                        agent.inventory.add_item(batch);
                        return ActionResult::failure(format!("{} is not food", chosen));
                    }
                }

                // What comes off the fire is a different thing from what went
                // on, and an agent carries the two side by side: a stack holds
                // one preparation state, so cooked fish cannot share an entry
                // with the raw fish still in the pack.
                batch.item_id = Self::prepared_item_id(&chosen, outcome == CookingOutcome::Improves);

                if !agent.inventory.add_item(batch) {
                    return ActionResult::failure(format!(
                        "Nowhere to put {} {} coming off the fire",
                        quantity, chosen
                    ));
                }

                agent
                    .skills
                    .gain_experience(crate::agents::SkillType::Cooking, 25);

                // What is on the fire is what the neighbours smell
                let _ = self
                    .world
                    .add_to_heat_source(&fire_id, chosen.clone(), quantity);

                if outcome == CookingOutcome::Improves {
                    debug!("Agent {} cooked {} {}", self.population.agents[agent_index].id, quantity, chosen);

                    ActionResult::success()
                        .with_energy_cost(2.0)
                        .with_message(format!("Cooked {} {}", quantity, chosen))
                } else if attentive {
                    debug!(
                        "Agent {} ruined {} {}: a fire is no good to it",
                        self.population.agents[agent_index].id, quantity, chosen
                    );

                    ActionResult::failure(format!(
                        "Ruined {} {}: a fire is no good to it",
                        quantity, chosen
                    ))
                } else {
                    debug!(
                        "Agent {} burnt {} {}",
                        self.population.agents[agent_index].id, quantity, chosen
                    );

                    ActionResult::failure(format!("Burnt {} {}", quantity, chosen))
                }
            },

            Action::MakeClothing { garment } => {
                use crate::agents::equipment::garment_recipe;
                use crate::agents::skills::SkillType;

                let recipe = match garment_recipe(garment) {
                    Some(recipe) => recipe,
                    None => {
                        return ActionResult::failure(format!("No such garment: {}", garment))
                    }
                };

                let agent = &mut self.population.agents[agent_index];

                if !agent
                    .inventory
                    .has_item(recipe.material_item, recipe.material_amount)
                {
                    return ActionResult::failure(format!(
                        "Not enough {} for a {}: needs {}",
                        recipe.material_item, recipe.name, recipe.material_amount
                    ));
                }

                // Making a garment is a skill like any other: the same flax in
                // different hands comes out as something that keeps the cold
                // off or as something that falls apart in a week. Quality
                // carries into both warmth and durability.
                //
                // And a beginner does not merely make a worse coat - a good
                // many of their attempts come to nothing at all, with the
                // material spoiled in the trying. `Skill::perform_check` had
                // been built to say exactly this and had no callers anywhere,
                // so every attempt succeeded and a first-day tailor turned out
                // garments as fast as a master. Half of a raw beginner's
                // attempts fail; a master's never do. That is what makes a
                // dedicated tailor quicker as well as better, without anything
                // in the model needing a notion of how long a job takes.
                let attempt = agent
                    .skills
                    .get_skill_mut(SkillType::Leatherworking)
                    .perform_check(None);

                // Cuts and needle-stabs, which are a beginner's other tax
                if let Some(hurt) = attempt.injury {
                    let harm = match hurt {
                        crate::agents::skills::InjuryType::Small => 2.0,
                        crate::agents::skills::InjuryType::Large => 8.0,
                    };
                    agent.state.health = (agent.state.health - harm).max(1.0);
                }

                if !attempt.success {
                    // The material is spoiled in the trying, and something is
                    // learned from having spoiled it
                    agent
                        .inventory
                        .remove_item(recipe.material_item, recipe.material_amount);
                    agent.skills.practise(SkillType::Leatherworking, 8, tick_now);

                    return ActionResult::failure(format!(
                        "Spoiled the {} in the making",
                        recipe.name
                    ));
                }

                let quality = Self::expected_garment_quality(agent);

                let made = match crate::agents::equipment::ClothingTemplate::from_id(
                    recipe.id, quality,
                ) {
                    Some(made) => made,
                    None => {
                        return ActionResult::failure(format!("Cannot make a {}", recipe.name))
                    }
                };

                agent
                    .inventory
                    .remove_item(recipe.material_item, recipe.material_amount);

                agent.skills.practise(SkillType::Leatherworking, 25, tick_now);

                // Making a coat and putting it on is one act.
                //
                // Leaving it in the pack to be worn later does not work: an
                // inventory stack carries one quality for the whole stack, so
                // a better second coat merged into the first and was recorded
                // as no better than it. Agents made coat after coat, each an
                // improvement, and wore none of them - over eight thousand
                // ticks one settlement made two hundred and eighty garments
                // and put on a hundred and sixty.
                let worn_now = Self::warmth_worn(agent, recipe.slot);
                let put_on = made.cold_insulation() > worn_now;

                if put_on {
                    // The coat this replaces is worn out or simply worse, and
                    // an agent that kept every one of them ended up carrying
                    // twenty cast-offs at two kilos each - a third of what it
                    // could carry, in old clothes, instead of food.
                    agent.body.unequip(recipe.slot);
                    agent.body.equip(made);
                } else {
                    let mut folded = crate::agents::InventoryItem::new_with_weight(
                        recipe.id.to_string(),
                        1,
                        2.0,
                    );
                    folded.quality = Some(quality);
                    folded.current_durability = Some(made.durability);
                    folded.max_durability = Some(made.max_durability);

                    if !agent.inventory.add_item(folded) {
                        return ActionResult::failure(format!(
                            "Nowhere to put the {} just made",
                            recipe.name
                        ));
                    }
                }

                debug!(
                    "Agent {} made a {:?} {} (insulation now {:.2})",
                    agent.id,
                    quality,
                    recipe.name,
                    agent.body.total_cold_insulation()
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Shelter, -0.15)
                    .with_energy_cost(8.0)
                    .with_message(format!("Made and put on a {:?} {}", quality, recipe.name))
            },

            Action::WearClothing { garment } => {
                use crate::agents::equipment::{garment_recipe, ClothingTemplate};

                let recipe = match garment_recipe(garment) {
                    Some(recipe) => recipe,
                    None => {
                        return ActionResult::failure(format!("No such garment: {}", garment))
                    }
                };

                let agent = &mut self.population.agents[agent_index];

                let carried = match agent.inventory.remove_item(recipe.id, 1) {
                    Some(carried) => carried,
                    None => {
                        return ActionResult::failure(format!("No {} to put on", recipe.name))
                    }
                };

                let quality = carried
                    .quality
                    .unwrap_or(crate::agents::skills::Quality::Basic);

                let mut clothing = match ClothingTemplate::from_id(recipe.id, quality) {
                    Some(clothing) => clothing,
                    None => {
                        agent.inventory.add_item(carried);
                        return ActionResult::failure(format!("Cannot wear {}", recipe.name));
                    }
                };

                // A garment picked back up is as worn as it was when it came off
                if let Some(durability) = carried.current_durability {
                    clothing.durability = durability.min(clothing.max_durability);
                }

                // Whatever was in that slot is worse than what is going on
                // over it, and is left behind rather than carried around
                agent.body.unequip(recipe.slot);
                agent.body.equip(clothing);

                debug!(
                    "Agent {} put on a {} (insulation now {:.2})",
                    agent.id,
                    recipe.name,
                    agent.body.total_cold_insulation()
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Shelter, -0.2)
                    .with_energy_cost(1.0)
                    .with_message(format!("Put on a {}", recipe.name))
            },

            Action::TillSoil => {
                use crate::world::{Position, ResourceNode, ResourceType, TerrainType};

                let agent_position = self.population.agents[agent_index].state.position;
                let tile_position = Position::new(agent_position.0, agent_position.1);

                let ground = match self.world.grid.get_tile(&tile_position) {
                    Some(tile) => tile.terrain.terrain_type,
                    None => return ActionResult::failure("Nowhere to dig".to_string()),
                };

                if !crate::world::Terrain::new(ground).can_be_tilled() {
                    return ActionResult::failure(format!(
                        "Cannot break {:?} into a field",
                        ground
                    ));
                }

                // Somewhere to put the crop, and something to sow it with.
                // A field is only worth breaking where there is room for one.
                if self
                    .world
                    .resources
                    .iter()
                    .any(|resource| resource.position == tile_position)
                {
                    return ActionResult::failure("Something already grows here".to_string());
                }

                if let Some(tile) = self.world.grid.get_tile_mut(&tile_position) {
                    tile.terrain = crate::world::Terrain::new(TerrainType::Farmland);
                }

                // What goes in the ground is what the agent has to put in it,
                // and of what it has, whatever it has come to believe is worth
                // sowing. Nobody hands out grain seed: an agent that has only
                // ever stripped berry bushes sows berries, works the field all
                // season, and finds out what a berry bush thinks of a plough.
                let sown = Self::what_this_one_would_sow(&self.population.agents[agent_index]);

                // The seed itself goes in the ground. Sowing was free before
                // this, which made a field a thing you got for a day's digging
                // rather than for a day's digging and a meal you did not eat.
                for (called, crop, _) in Self::what_can_be_sown() {
                    if crop != sown {
                        continue;
                    }
                    let agent = &mut self.population.agents[agent_index];
                    if agent.how_many_i_have(called) > 0 {
                        agent.inventory.remove_item(called, 1);
                        break;
                    }
                }

                // A newly sown field starts empty and fills as it grows
                let mut field = ResourceNode::new(
                    sown,
                    tile_position,
                    Self::FIELD_YIELD,
                );
                field.amount = 0;
                self.world.resources.push(field);

                let agent = &mut self.population.agents[agent_index];
                agent
                    .skills
                    .practise(crate::agents::SkillType::Farming, 25, tick_now);

                debug!(
                    "Agent {} broke ground at {:?} and sowed {:?}",
                    agent.id, tile_position, sown
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Sustenance, -0.4)
                    .with_energy_cost(12.0)
                    .with_message("Broke ground and sowed a field".to_string())
            },

            Action::TrySwapping {
                instead_of_making,
                instead_of,
                put_in,
            } => {
                use crate::environment::making;

                let Some(step) = making::how_to_make(instead_of_making) else {
                    return ActionResult::failure("No such job".to_string());
                };

                // The parts have to be to hand: everything the step wants
                // except the one left out, and one of whatever is going in
                // instead.
                {
                    let agent = &self.population.agents[agent_index];

                    let short = step.needs.iter().any(|(what, how_many)| {
                        *what != instead_of.as_str()
                            && agent.how_many_i_have(what) < *how_many
                    }) || agent.how_many_i_have(put_in) == 0;

                    if short {
                        return ActionResult::failure(
                            "Not the makings for that, either way".to_string(),
                        );
                    }
                }

                let outcome = making::what_comes_of_swapping(
                    instead_of_making,
                    instead_of,
                    put_in,
                );

                // The materials go whether it works or not. That is the whole
                // cost of trying things: a man who puts a lump of iron where
                // the flake goes has spent a stick and a length of cord and
                // has a lump of iron tied to a stick.
                {
                    let agent = &mut self.population.agents[agent_index];
                    for (what, how_many) in step.needs {
                        if *what == instead_of.as_str() {
                            continue;
                        }
                        agent.inventory.remove_item(what, *how_many);
                    }
                    agent.inventory.remove_item(put_in, 1);
                }

                let worked = outcome.is_some();

                if let Some(swap) = outcome {
                    let made = self.population.agents[agent_index]
                        .a_tool_fresh_from_these_hands(swap.makes, swap.how_many, 2.0);

                    let agent = &mut self.population.agents[agent_index];
                    agent.inventory.add_item(made);

                    // And he knows how to do it now, which is what makes it a
                    // discovery rather than an accident
                    agent.found_out_how_to(swap.makes);
                    agent.skills.practise(step.hands, 20, tick_now);

                    debug!(
                        "Agent {} put {put_in} where the {instead_of} goes and got a {}",
                        agent.id, swap.makes
                    );
                }

                let called = making::what_that_swap_is_called(
                    instead_of_making,
                    instead_of,
                    put_in,
                );
                self.population.agents[agent_index]
                    .lessons
                    .record_particular(&called, worked);

                if worked {
                    ActionResult::success()
                        .with_drive_change(DriveType::Curiosity, -0.5)
                        .with_drive_change(DriveType::Utility, -0.3)
                        .with_energy_cost(step.effort)
                        .with_message(format!("Put {put_in} in and got something new"))
                } else {
                    ActionResult::failure(format!(
                        "{put_in} where the {instead_of} goes comes to nothing"
                    ))
                    .with_drive_change(DriveType::Curiosity, -0.3)
                    .with_energy_cost(step.effort)
                }
            },

            Action::Examine { what } => {
                use crate::environment::making;

                if self.population.agents[agent_index].how_many_i_have(what) == 0 {
                    return ActionResult::failure(format!("No {what} in hand to look at"));
                }

                // What this thing goes into that nobody here has worked out.
                // Looking closely at a lump of something you are already
                // carrying is the cheapest experiment there is - it costs a
                // turn and no materials - and it is the third road into the
                // chain, beside doing a thing twice to see it again and
                // putting the wrong thing where a part goes.
                if making::is_a_familiar_thing(what) {
                    return ActionResult::failure(format!(
                        "A {what} is a {what}; there is nothing to see in one"
                    ))
                    .with_energy_cost(1.0);
                }

                let could_be_for: Vec<&'static str> = making::everything_to_find_out()
                    .filter(|step| step.needs.iter().any(|(needs, _)| needs == what))
                    .map(|step| step.makes)
                    .chain(
                        making::every_working_to_find_out()
                            .filter(|working| working.to == *what)
                            .map(|working| working.makes),
                    )
                    .filter(|makes| {
                        !self.population.agents[agent_index]
                            .what_i_found_out()
                            .contains(*makes)
                    })
                    .collect();

                let agent = &mut self.population.agents[agent_index];
                agent.skills.practise(crate::agents::SkillType::Crafting, 4, tick_now);

                let Some(worth_a_look) = could_be_for.first().copied() else {
                    return ActionResult::failure(format!("Nothing new about a {what}"))
                        .with_drive_change(DriveType::Curiosity, -0.1)
                        .with_energy_cost(1.0);
                };

                // Turning it over in your hands is not the same as knowing.
                // Most of the time it tells you nothing, which is why this
                // does not collapse the chain into an afternoon's inspection.
                let hand = agent.skills.hand_for(crate::agents::SkillType::Crafting);
                let odds = (Self::WHAT_LOOKING_CLOSELY_IS_WORTH * hand).clamp(0.0, 0.5);

                if !rng.gen_bool(odds as f64) {
                    return ActionResult::failure(format!("Turned the {what} over, none the wiser"))
                        .with_drive_change(DriveType::Curiosity, -0.2)
                        .with_energy_cost(1.0);
                }

                agent.found_out_how_to(worth_a_look);

                debug!(
                    "Agent {} looked at a {what} and saw what it is for ({worth_a_look})",
                    agent.id
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Curiosity, -0.5)
                    .with_energy_cost(1.0)
                    .with_message(format!("Looked at a {what}: it is for a {worth_a_look}"))
            },

            Action::PickUp { what } => {
                use crate::world::Position;

                let here = {
                    let at = self.population.agents[agent_index].state.position;
                    Position::new(at.0, at.1)
                };

                let Some(item) = self.world.take_off_the_ground(&here, what) else {
                    return ActionResult::failure(format!("No {what} lying here"));
                };

                let how_many = item.quantity;
                let agent = &mut self.population.agents[agent_index];

                // A full pack cannot take it, and it stays where it was
                if agent.inventory.weight_capacity_remaining()
                    < item.weight_per_unit * how_many as f32
                {
                    self.world.somebody_left_this(item, here, tick_now);
                    return ActionResult::failure("No room for it".to_string());
                }

                agent.inventory.add_item(item);

                debug!("Agent {} picked up {how_many} {what} at {here:?}", agent.id);

                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.2)
                    .with_energy_cost(1.0)
                    .with_message(format!("Picked up {how_many} {what}"))
            },

            Action::PutDown { what } => {
                use crate::world::Position;

                let here = {
                    let at = self.population.agents[agent_index].state.position;
                    Position::new(at.0, at.1)
                };

                let Some(item) = self.population.agents[agent_index]
                    .inventory
                    .get_item(what)
                    .cloned()
                else {
                    return ActionResult::failure(format!("No {what} to put down"));
                };

                if item.quantity == 0 {
                    return ActionResult::failure(format!("No {what} to put down"));
                }

                let how_many = item.quantity;
                self.population.agents[agent_index]
                    .inventory
                    .remove_item(what, how_many);
                self.world.somebody_left_this(item, here, tick_now);

                debug!(
                    "Agent {} put down {how_many} {what} at {here:?}",
                    self.population.agents[agent_index].id
                );

                ActionResult::success()
                    .with_energy_cost(1.0)
                    .with_message(format!("Put down {how_many} {what}"))
            },

            Action::Trade { with } => {
                let Some(them) = self
                    .population
                    .agents
                    .iter()
                    .position(|other| other.id == *with && other.state.is_alive)
                else {
                    return ActionResult::failure("Nobody there to trade with".to_string());
                };

                let Some((mine, theirs)) = self.what_the_two_of_them_would_swap(agent_index, them)
                else {
                    return ActionResult::failure(
                        "Nothing between us that either of us wants".to_string(),
                    );
                };

                // Half of what each has spare, so a trade leaves both better
                // off and neither stripped
                let how_much = |spare: u32| (spare / 2).max(1);
                let i_hand_over = how_much(mine.1);
                let they_hand_over = how_much(theirs.1);

                {
                    let agent = &mut self.population.agents[agent_index];
                    agent.inventory.remove_item(&mine.0, i_hand_over);
                    agent.inventory.add_item(
                        crate::agents::InventoryItem::new_with_weight(
                            theirs.0.clone(),
                            they_hand_over,
                            1.0,
                        ),
                    );
                    agent.skills.practise(crate::agents::SkillType::Social, 8, tick_now);
                }

                let me = self.population.agents[agent_index].id;
                let them_id = self.population.agents[them].id;

                {
                    let other = &mut self.population.agents[them];
                    other.inventory.remove_item(&theirs.0, they_hand_over);
                    other.inventory.add_item(
                        crate::agents::InventoryItem::new_with_weight(
                            mine.0.clone(),
                            i_hand_over,
                            1.0,
                        ),
                    );
                    other.skills.practise(crate::agents::SkillType::Social, 8, tick_now);

                    // A good trade is a good turn on both sides, and both
                    // remember who it was with
                    other.they_did_me_a_good_turn(me, Self::WHAT_A_FAIR_TRADE_IS_WORTH);
                }

                self.population.agents[agent_index]
                    .they_did_me_a_good_turn(them_id, Self::WHAT_A_FAIR_TRADE_IS_WORTH);

                debug!(
                    "Agent {me} gave {them_id} {i_hand_over} {} for {they_hand_over} {}",
                    mine.0, theirs.0
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.35)
                    .with_drive_change(DriveType::Social, -0.15)
                    .with_energy_cost(2.0)
                    .with_message(format!(
                        "Traded {i_hand_over} {} for {they_hand_over} {}",
                        mine.0, theirs.0
                    ))
            },

            Action::GiveTo { to } => {
                let Some(them) = self
                    .population
                    .agents
                    .iter()
                    .position(|other| other.id == *to && other.state.is_alive)
                else {
                    return ActionResult::failure("Nobody there to give to".to_string());
                };

                // Something they are short of that I have too much of. A gift
                // is one-sided: what the other has is nothing to do with it.
                let Some(mine) = self.what_i_would_hand_over(agent_index, them) else {
                    return ActionResult::failure(
                        "Nothing of mine they have any use for".to_string(),
                    );
                };

                let handed_over = (mine.1 / 2).max(1);
                let me = self.population.agents[agent_index].id;

                {
                    let agent = &mut self.population.agents[agent_index];
                    agent.inventory.remove_item(&mine.0, handed_over);
                    agent.skills.practise(crate::agents::SkillType::Social, 10, tick_now);
                }

                {
                    let other = &mut self.population.agents[them];
                    other.inventory.add_item(
                        crate::agents::InventoryItem::new_with_weight(
                            mine.0.clone(),
                            handed_over,
                            1.0,
                        ),
                    );
                    other.they_did_me_a_good_turn(me, Self::WHAT_A_GIFT_IS_WORTH);
                }

                debug!("Agent {me} gave away {handed_over} {}", mine.0);

                ActionResult::success()
                    .with_drive_change(DriveType::Social, -0.4)
                    .with_energy_cost(1.0)
                    .with_message(format!("Gave away {handed_over} {}", mine.0))
            },

            Action::Work { verb, to } => {
                use crate::environment::making;

                let Some(working) = making::how_to_work(verb, to) else {
                    return ActionResult::failure(format!("Nothing comes of {verb} a {to}"));
                };

                if self.population.agents[agent_index].how_many_i_have(to) < working.how_much {
                    return ActionResult::failure(format!("Not enough {to} to {verb}"));
                }

                // Water has to be carried to where the work is, which is the
                // whole reason a vessel matters
                if working.wants_water > 0.0
                    && self.population.agents[agent_index].how_much_water_i_carry()
                        < working.wants_water
                {
                    return ActionResult::failure(format!("Not enough water to {verb} {to}"));
                }

                if working.over_a_fire {
                    let stood = self.population.agents[agent_index].state.position;
                    if self
                        .nearest_fire_from(stood, Self::FIRE_REACH, true)
                        .is_none()
                    {
                        return ActionResult::failure(format!("No fire here to {verb} {to} over"));
                    }
                }

                // What comes off it, and how much of it these hands get. A
                // practised hand wastes less of the core.
                let hand = self.population.agents[agent_index]
                    .skills
                    .hand_for(working.hands);
                let tool = self.population.agents[agent_index]
                    .how_much_my_tools_help(working.hands);

                let worth = working.how_many as f32 * hand.min(2.0) * tool.min(2.0);
                let whole = worth.floor();
                let came_off = (whole as u32)
                    + u32::from(rng.gen::<f32>() < worth - whole);
                let came_off = came_off.max(1);

                // What comes out. A bowl is a thing you can put water in and a
                // handful of flour is a meal, and neither is a lump of stuff
                // with a name on it - see `Working::holds` and `Working::feeds`.
                let mut made = match working.holds {
                    Some(capacity) => crate::agents::InventoryItem::new_container(
                        working.makes.to_string(),
                        came_off,
                        capacity,
                    ),
                    None => crate::agents::InventoryItem::new_with_weight(
                        working.makes.to_string(),
                        came_off,
                        1.0,
                    ),
                };

                if let Some(as_food) = working.feeds {
                    made.food_data = self.food_database.create_food_data(&as_food, tick_now);
                }

                {
                    let agent = &mut self.population.agents[agent_index];
                    agent.inventory.remove_item(to, working.how_much);

                    if working.wants_water > 0.0 {
                        agent.draw_from_what_i_carry(working.wants_water);
                    }

                    agent.inventory.add_item(made);
                    agent.skills.practise(working.hands, 12, tick_now);

                    // Having done it once he can do it on purpose. For the
                    // obvious ones this is a formality; for the rest it is the
                    // whole of the discovery - somebody with a scraper in his
                    // hand and a fire that will not light finds out what
                    // shavings are for by making some.
                    agent.found_out_how_to(working.makes);
                }

                // And the edge that did it is the worse for it
                if let Some(broke) = self.population.agents[agent_index]
                    .wear_what_i_worked_with(working.hands)
                {
                    debug!(
                        "Agent {} wore out a {broke}",
                        self.population.agents[agent_index].id
                    );
                }

                debug!(
                    "Agent {} {verb} {} {to} into {came_off} {}",
                    self.population.agents[agent_index].id,
                    working.how_much,
                    working.makes
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.25)
                    .with_energy_cost(working.effort)
                    .with_message(format!("{verb} {to} into {came_off} {}", working.makes))
            },

            Action::Taste => {
                use crate::world::Position;

                let agent_position = self.population.agents[agent_index].state.position;
                let here = Position::new(agent_position.0, agent_position.1);

                let Some(index) = self.world.resources.iter().position(|resource| {
                    resource.position == here
                        && resource.resource_type == crate::world::ResourceType::StrangePlant
                        && resource.amount > 0
                }) else {
                    return ActionResult::failure("Nothing here to try".to_string());
                };

                let kind = self.world.resources[index].kind;
                let feeds_you = self.world.does_this_one_feed_you(kind);

                self.world.resources[index].harvest(1);

                let agent = &mut self.population.agents[agent_index];
                agent.now_i_know_that_plant(kind, feeds_you);

                let result = if feeds_you {
                    // It is food. Not much of a meal - one mouthful of a
                    // strange plant is a mouthful - but the man is no worse for
                    // it and the people have one more thing to eat.
                    agent
                        .nutrition
                        .consume(&crate::world::nutrition::NutritionalContent {
                            energy: Self::WHAT_ONE_MOUTHFUL_IS_WORTH,
                            protein: 1.0,
                            micronutrients: 2.0,
                            water_content: 5.0,
                        });

                    debug!("Agent {} found that plant {kind} is food", agent.id);

                    ActionResult::success()
                        .with_drive_change(DriveType::Curiosity, -0.4)
                        .with_drive_change(DriveType::Hunger, -0.05)
                        .with_energy_cost(1.0)
                        .with_message(format!("Tried plant {kind}: it is food"))
                } else {
                    // It is not. What that costs runs from a bad afternoon to
                    // everything, which is what makes the trying a real choice
                    // rather than a free lookup.
                    let harm = rng.gen_range(
                        Self::WHAT_A_BAD_PLANT_DOES.0..=Self::WHAT_A_BAD_PLANT_DOES.1,
                    );
                    agent.take_damage(harm);

                    debug!(
                        "Agent {} was poisoned by plant {kind} ({harm:.0} damage, {:.0} health left)",
                        agent.id, agent.state.health
                    );

                    ActionResult::failure(format!("Tried plant {kind}: it is poison"))
                        .with_drive_change(DriveType::Curiosity, -0.4)
                        .with_energy_cost(6.0)
                };

                // And whoever was standing about learns it too, without paying
                // for it. This is the whole value of other people: one man is
                // ill and forty know not to eat that.
                for onlooker in self
                    .population
                    .agents
                    .iter_mut()
                    .filter(|agent| agent.state.is_alive)
                {
                    let apart = (onlooker.state.position.0 - here.x).abs()
                        + (onlooker.state.position.1 - here.y).abs();

                    if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                        onlooker.now_i_know_that_plant(kind, feeds_you);
                    }
                }

                result
            },

            Action::TakeCutting => {
                use crate::agents::InventoryItem;
                use crate::world::Position;

                let agent_position = self.population.agents[agent_index].state.position;
                let here = Position::new(agent_position.0, agent_position.1);

                let Some(index) = self.world.resources.iter().position(|resource| {
                    resource.position == here
                        && resource.amount > Self::WHAT_A_CUTTING_TAKES
                        && Self::what_can_be_sown()
                            .into_iter()
                            .any(|(_, sowable, _)| sowable == resource.resource_type)
                }) else {
                    return ActionResult::failure("Nothing here worth lifting".to_string());
                };

                let crop = self.world.resources[index].resource_type;

                let Some((called, _, _)) = Self::what_can_be_sown()
                    .into_iter()
                    .find(|(_, sowable, _)| *sowable == crop)
                else {
                    return ActionResult::failure(format!("{crop:?} does not move"));
                };

                if self.world.resources[index].max_amount
                    <= Self::TOO_THIN_TO_DIG + Self::WHAT_A_CUTTING_TAKES
                {
                    return ActionResult::failure("Too thin to dig out of".to_string());
                }

                // Taking a cutting costs the plant it came off, permanently: a
                // slip is a piece of the plant and not a piece of this year's
                // crop. A patch dug over for slips carries less from now on.
                self.world.resources[index].harvest(Self::WHAT_A_CUTTING_TAKES);
                self.world.resources[index].max_amount = self.world.resources[index]
                    .max_amount
                    .saturating_sub(Self::WHAT_A_CUTTING_TAKES);

                let agent = &mut self.population.agents[agent_index];
                agent.inventory.add_item(InventoryItem::new_with_weight(
                    Self::a_cutting_of(called),
                    1,
                    1.5,
                ));
                agent
                    .skills
                    .practise(crate::agents::SkillType::Farming, 8, tick_now);

                debug!("Agent {} lifted a slip of {called} at {here:?}", agent.id);

                ActionResult::success()
                    .with_energy_cost(5.0)
                    .with_message(format!("Lifted a slip of {called}"))
            },

            Action::PlantCutting => {
                use crate::world::{Position, ResourceNode};

                let agent_position = self.population.agents[agent_index].state.position;
                let here = Position::new(agent_position.0, agent_position.1);

                let Some((called, crop)) =
                    Self::a_cutting_in_the_pack(&self.population.agents[agent_index])
                else {
                    return ActionResult::failure("Nothing to plant".to_string());
                };

                let will_take = self
                    .world
                    .grid
                    .get_tile(&here)
                    .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
                    .unwrap_or(false);

                if !will_take {
                    return ActionResult::failure("Nothing will take here".to_string());
                }

                if self
                    .world
                    .resources
                    .iter()
                    .any(|resource| resource.position == here)
                {
                    return ActionResult::failure("Something already grows here".to_string());
                }

                let mut moved = ResourceNode::new(crop, here, Self::WHAT_A_MOVED_PLANT_COMES_TO);
                moved.amount = Self::WHAT_A_CUTTING_STARTS_WITH;
                self.world.resources.push(moved);

                let agent = &mut self.population.agents[agent_index];
                agent
                    .inventory
                    .remove_item(&Self::a_cutting_of(called), 1);
                agent
                    .skills
                    .practise(crate::agents::SkillType::Farming, 15, tick_now);

                debug!("Agent {} put a slip of {called} in at {here:?}", agent.id);

                ActionResult::success()
                    .with_drive_change(DriveType::Sustenance, -0.3)
                    .with_energy_cost(8.0)
                    .with_message(format!("Put a slip of {called} in beside the camp"))
            },

            Action::TendField => {
                use crate::world::Position;

                let agent_position = self.population.agents[agent_index].state.position;
                let tile_position = Position::new(agent_position.0, agent_position.1);

                let Some(tile) = self.world.grid.get_tile(&tile_position) else {
                    return ActionResult::failure("Nowhere to work".to_string());
                };

                if !tile.terrain.is_cultivated() {
                    return ActionResult::failure("No field here to work".to_string());
                }

                let before = tile.soil.weeds + tile.soil.pests;

                if before <= 0.0 {
                    return ActionResult::failure("Nothing wants doing here".to_string());
                }

                // A walk out to a field that is still bare after all this work
                // is what teaches an agent that it sowed the wrong thing.
                let standing = self
                    .world
                    .resources
                    .iter()
                    .find(|resource| resource.position == tile_position)
                    .map(|resource| (resource.resource_type, resource.amount));

                if let Some((crop, amount)) = standing {
                    if let Some((called, _, _)) = Self::what_can_be_sown()
                        .into_iter()
                        .find(|(_, sowable, _)| *sowable == crop)
                    {
                        self.population.agents[agent_index]
                            .lessons
                            .record_particular(&format!("sow:{called}"), amount > 0);
                    }

                    // And whether the whole business is worth anybody's day.
                    // A man standing in his own field can see whether there is
                    // anything in it; he does not have to wait until he is
                    // carrying it home. This is where farming is mostly either
                    // confirmed or given up on.
                    self.population.agents[agent_index]
                        .practices
                        .record_outcome(crate::agents::practices::Practice::Farming, amount > 0);
                }

                // What a practised hand gets through in a turn. Somebody who
                // has done it for years knows a weed from a seedling and takes
                // the whole field; somebody who has not clears half of it and
                // treads on the rest.
                let hand = self.population.agents[agent_index]
                    .skills
                    .hand_for(crate::agents::SkillType::Farming);

                let cleared = {
                    let Some(tile) = self.world.grid.get_tile_mut(&tile_position) else {
                        return ActionResult::failure("Nowhere to work".to_string());
                    };

                    for _ in 0..(hand.round() as u32).clamp(1, 3) {
                        tile.soil.somebody_worked_this_field();
                    }

                    before - (tile.soil.weeds + tile.soil.pests)
                };

                let agent = &mut self.population.agents[agent_index];
                agent
                    .skills
                    .practise(crate::agents::SkillType::Farming, 12, tick_now);

                debug!(
                    "Agent {} worked the field at {:?} (weeds and pests down {:.2})",
                    agent.id, tile_position, cleared
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Sustenance, -0.2)
                    .with_energy_cost(7.0)
                    .with_message(format!("Worked the field, clearing {cleared:.2}"))
            },

            Action::SpreadMuck => {
                use crate::agents::practices::Practice;
                use crate::world::Position;

                let agent_position = self.population.agents[agent_index].state.position;
                let tile_position = Position::new(agent_position.0, agent_position.1);

                // What is in the pack that is fit for nothing else
                let refuse: Vec<(String, u32)> = {
                    let agent = &self.population.agents[agent_index];
                    agent
                        .inventory
                        .get_all_items()
                        .values()
                        .filter(|item| item.quantity > 0)
                        .filter(|item| {
                            item.food_data
                                .as_ref()
                                .map(|food| food.is_rotting() || food.is_ruined())
                                .unwrap_or(false)
                        })
                        .map(|item| (item.item_id.clone(), item.quantity))
                        .collect()
                };

                if refuse.is_empty() {
                    return ActionResult::failure("Nothing spoiled to tip out".to_string());
                }

                let before = self
                    .world
                    .grid
                    .get_tile(&tile_position)
                    .map(|tile| tile.soil.fertility() + tile.soil.litter())
                    .unwrap_or(0.0);

                let mut tipped = 0;
                let mut worth = 0.0;
                {
                    let agent = &mut self.population.agents[agent_index];
                    for (item_id, quantity) in &refuse {
                        agent.inventory.remove_item(item_id, *quantity);
                        tipped += quantity;

                        // A rotten fish is not a rotten turnip. The turnip is
                        // giving back what this ground grew it with; the fish
                        // is bringing in what the sea grew it with, and is
                        // worth many times as much to a field on that account
                        // alone.
                        worth += *quantity as f32
                            * if crate::world::Soil::came_out_of_the_water(item_id) {
                                Self::MUCK_PER_FISH
                            } else {
                                Self::MUCK_PER_UNIT
                            };
                    }
                }

                // Spoiled food is soft matter and goes quickly, given wet ground
                if let Some(tile) = self.world.grid.get_tile_mut(&tile_position) {
                    tile.soil.add_leaf_litter(worth);
                }

                let after = self
                    .world
                    .grid
                    .get_tile(&tile_position)
                    .map(|tile| tile.soil.fertility() + tile.soil.litter())
                    .unwrap_or(0.0);

                // What the agent can actually see: the ground here is richer
                // than it was. Whether that was worth doing is a judgement it
                // makes for itself, and gets wrong sometimes - tipping muck on
                // bare rock or in a desert does nothing much.
                let worked = after > before + 0.05;

                let agent = &mut self.population.agents[agent_index];
                agent.practices.record_outcome(Practice::SpreadingMuck, worked);
                agent
                    .skills
                    .practise(crate::agents::SkillType::Farming, 10, tick_now);

                debug!(
                    "Agent {} tipped {} spoiled units onto {:?} (ground {:.2} -> {:.2})",
                    agent.id, tipped, tile_position, before, after
                );

                ActionResult::success()
                    .with_energy_cost(3.0)
                    .with_message(format!("Spread {} of muck on the ground", tipped))
            },

            Action::Fish => {
                // Whether it worked is recorded by `Agent::learn_from`, off
                // this arm's own success, along with every other undertaking.
                let agent_position = self.population.agents[agent_index].state.position;

                let Some(reach) = self.reach_within_cast(agent_position) else {
                    return ActionResult::failure("No water in reach".to_string());
                };

                // What the agent brings to it. A rod is worth having and a
                // practised hand is worth more, but a river in the run will
                // feed somebody who has neither.
                let skill = self.population.agents[agent_index]
                    .skills
                    .get_skill_if_exists(crate::agents::SkillType::Fishing)
                    .map(|skill| skill.level)
                    .unwrap_or(-10) as f32;
                let rod = self.population.agents[agent_index]
                    .inventory
                    .get_all_items()
                    .values()
                    .any(|item| {
                        item.quantity > 0 && item.item_id.to_lowercase().contains("rod")
                    });

                let standing = self
                    .world
                    .resources
                    .iter()
                    .find(|resource| resource.position == reach)
                    .map(|resource| resource.amount)
                    .unwrap_or(0);

                if standing == 0 {
                    return ActionResult::failure("The reach is empty".to_string());
                }

                // How thick the water is decides most of it. A run is a run:
                // anybody standing in it comes out with something, which is
                // exactly why a fishery is worth building a life beside and a
                // deer is not. It is still slow: standing in cold water
                // waiting for something to come within reach of a thrust is
                // most of a morning for a couple of fish.
                let thickness = (standing as f32 / Self::A_GOOD_REACH).clamp(0.0, 1.0);

                // A spear is what a people with no line fishes with, and it
                // is slow work: standing in the shallows waiting for
                // something to come within reach of a thrust.
                let spear = self.population.agents[agent_index]
                    .how_much_my_tools_help(crate::agents::SkillType::Fishing);

                let hand = (skill / 10.0).clamp(0.0, 0.5)
                    + if rod { 0.2 } else { 0.0 }
                    + (spear - 1.0) * 0.3;
                let odds = (Self::A_THRUST_THAT_TELLS + 0.4 * thickness + hand).clamp(0.0, 0.9);

                if spear > 1.0 {
                    self.population.agents[agent_index]
                        .wear_what_i_worked_with(crate::agents::SkillType::Fishing);
                }

                if rng.gen::<f32>() > odds {
                    return ActionResult::failure("Nothing took".to_string())
                        .with_energy_cost(Self::WHAT_A_THRUST_COSTS);
                }

                let caught = Self::FISH_PER_CAST
                    + u32::from(rod)
                    + u32::from(spear > 1.3);

                let taken = {
                    let resource = self
                        .world
                        .resources
                        .iter_mut()
                        .find(|resource| resource.position == reach);
                    match resource {
                        Some(resource) => {
                            let taken = caught.min(resource.amount);
                            resource.amount -= taken;
                            taken
                        }
                        None => 0,
                    }
                };

                if taken == 0 {
                    return ActionResult::failure("The reach is empty".to_string());
                }

                // A fish is not all meat. The guts and heads go straight to
                // waste, and that waste is the richest thing a farming people
                // beside a river ever get their hands on - it came out of the
                // sea rather than out of their own fields.
                let food_data = self
                    .food_database
                    .create_food_data(&crate::world::inventory::ItemType::Fish, self.current_tick);

                let agent = &mut self.population.agents[agent_index];
                let mut catch =
                    crate::agents::InventoryItem::new_with_weight("fish".to_string(), taken, 0.8);
                catch.food_data = food_data;
                agent.inventory.add_item(catch);
                agent.state.waste_carried +=
                    taken as f32 * crate::world::Soil::NUTRIENT_PER_FISH * Self::OFFAL_SHARE;
                agent
                    .skills
                    .practise(crate::agents::SkillType::Fishing, 12, tick_now);

                debug!("Agent {} took {} fish from {:?}", agent.id, taken, reach);

                ActionResult::success()
                    .with_drive_change(DriveType::Hunger, -0.15)
                    .with_drive_change(DriveType::Sustenance, -0.2)
                    .with_energy_cost(Self::WHAT_A_THRUST_COSTS)
                    .with_message(format!("Took {} fish out of the water", taken))
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
                let current_pos = self.population.agents[agent_index].state.position;

                // Calculate target position in exploration direction
                let target_x = current_pos.0 + direction.0;
                let target_y = current_pos.1 + direction.1;
                let target_z = current_pos.2 + direction.2;
                let target_pos = (target_x, target_y, target_z);

                // What is really out here, before anybody's opinion of it
                let exploration_radius = 3; // Can see 3 tiles in each direction
                let really_here: std::collections::HashSet<crate::world::Position> = self
                    .world
                    .resources
                    .iter()
                    .filter(|resource| {
                        (resource.position.x - target_x).abs() <= exploration_radius
                            && (resource.position.y - target_y).abs() <= exploration_radius
                    })
                    .map(|resource| {
                        crate::world::Position::new(resource.position.x, resource.position.y)
                    })
                    .collect();

                let agent = &mut self.population.agents[agent_index];
                let agent_id = agent.id;

                // Move agent to new position
                agent.state.position = target_pos;

                // Mark tiles as explored in a radius around new position
                let mut newly_explored_count = 0;

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

                // Seeing for yourself.
                //
                // An agent's knowledge of where things are is fed both by
                // looking and by being told, and the two went into the same
                // map with nothing to tell them apart. So a man walked to the
                // place he had been told about, found bare ground, and read
                // his own hearsay back off the map as confirmation - which
                // made every lie verify as true and left the whole
                // lie-detection apparatus unable to detect anything.
                //
                // This is the moment a lie is found out, and the only moment
                // it can be: the agent is standing on the spot and there is
                // nothing there. Sweeping a buffer of remembered claims every
                // hundred ticks caught almost none of them, because a claim
                // had to survive the buffer *and* the agent had to happen to
                // walk to it inside the same window.
                let centre = crate::world::Position::new(target_x, target_y);
                let found_out = agent.exploration_knowledge.hearsay_in_view(
                    centre,
                    exploration_radius,
                    &really_here,
                );

                // What is not there when you are standing on it is not there
                agent
                    .exploration_knowledge
                    .known_resources
                    .retain(|where_it_is, _| {
                        let in_view = (where_it_is.x - centre.x).abs() <= exploration_radius
                            && (where_it_is.y - centre.y).abs() <= exploration_radius;
                        !in_view || really_here.contains(where_it_is)
                    });
                agent
                    .exploration_knowledge
                    .who_told_me
                    .retain(|where_it_is, _| {
                        let in_view = (where_it_is.x - centre.x).abs() <= exploration_radius
                            && (where_it_is.y - centre.y).abs() <= exploration_radius;
                        !in_view || really_here.contains(where_it_is)
                    });

                for (_, said, what_they_said) in found_out {
                    if said.who == agent_id {
                        continue;
                    }
                    let subject = format!("{:?}", what_they_said).to_lowercase();
                    if said.was_he_answerable_for_it(self.current_tick) {
                        agent.found_out_i_was_lied_to(said.who, &subject, self.current_tick);
                    } else {
                        agent.found_out_they_were_out_of_date(said.who);
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
                agent.skills.practise(crate::agents::skills::SkillType::Navigation, nav_xp, tick_now);

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
    /// How close a predator has to be to an agent to try it
    const PREDATOR_STRIKE_RANGE: i32 = 3;

    /// A hungry predator turns on the people.
    ///
    /// Nothing in the model let an animal touch an agent: predation was
    /// animal-on-animal only, so a wolf could starve beside a settlement. A
    /// predator that is merely hungry keeps to the herds; one that is close
    /// to starving takes what it can reach, and that includes an agent.
    ///
    /// This is where thinning the herds comes back on the settlement that did
    /// it. Agents hunt for skins, the herds go down, the predators go hungry,
    /// and hungry predators come looking.
    fn process_predator_attacks(&mut self) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let current_tick = self.current_tick;

        // Who is where, and who is desperate enough to try
        let agent_positions: Vec<(usize, (i32, i32))> = self
            .population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.state.is_alive)
            .map(|(index, agent)| (index, (agent.state.position.0, agent.state.position.1)))
            .collect();

        if agent_positions.is_empty() {
            return;
        }

        let mut strikes: Vec<(uuid::Uuid, usize, f32, f32)> = Vec::new();

        for animal in self.world.animals.get_all() {
            if !animal.is_alive() || animal.is_domesticated || !animal.is_hungry() {
                continue;
            }

            let species = match self.world.animals.get_species(&animal.species_id) {
                Some(species) => species,
                None => continue,
            };

            if species.prey_species.is_empty() || species.attack_damage <= 0.0 {
                continue;
            }

            // Nearest agent within striking distance
            let target = agent_positions
                .iter()
                .filter(|(_, position)| {
                    (position.0 - animal.position.0).abs() <= Self::PREDATOR_STRIKE_RANGE
                        && (position.1 - animal.position.1).abs() <= Self::PREDATOR_STRIKE_RANGE
                })
                .min_by_key(|(_, position)| {
                    (position.0 - animal.position.0).abs()
                        + (position.1 - animal.position.1).abs()
                });

            let (agent_index, _) = match target {
                Some(target) => target,
                None => continue,
            };

            // A full belly makes a cautious animal. Hunger is what changes
            // its mind, and only really at the end of it.
            let pressure =
                ((animal.hunger / animal.max_hunger.max(1.0)) - 0.5).clamp(0.0, 0.5) / 0.5;
            let odds = 0.01 + pressure * pressure * 0.14;

            if rng.gen::<f32>() < odds {
                strikes.push((
                    animal.id,
                    *agent_index,
                    species.attack_damage,
                    species.food_value * 0.25,
                ));
            }
        }

        for (animal_id, agent_index, damage, fed) in strikes {
            // Standing there while something bites you is a fight, and how it
            // goes is what the agent takes away from it. This is where the
            // record mostly comes from: agents seldom set upon one another,
            // but the country is full of things that will try them.
            {
                use crate::agents::practices::Undertaking;

                // Winning is driving the thing off with a scratch; losing is
                // being mauled and living. Reckoning it as "did the blow kill
                // me" made every survivor a winner, which is half a lesson:
                // nobody ever learned that running was the better idea,
                // because everyone who learned it was dead.
                let agent = &mut self.population.agents[agent_index];
                let came_off_well = damage < agent.state.health * Self::A_SCRATCH;
                agent.lessons.record(Undertaking::Fighting, came_off_well);
            }

            {
                // What is in the hand when the thing comes at you. A man who
                // gets a spear between himself and a wolf takes a good deal
                // less of it than a man who gets an arm up, and this is the
                // whole of what the matrix means by a verb that wants a tool:
                // `defend with` cannot be done bare-handed, so a man with
                // nothing in his hands simply does not do it.
                let landed = self.population.agents[agent_index].what_a_blow_costs_me(damage);
                let turned = damage - landed;

                let agent = &mut self.population.agents[agent_index];

                // And putting a shaft in the way of something is hard on the
                // shaft
                if turned > 0.0 {
                    if let Some(broke) =
                        agent.wear_what_i_worked_with(crate::agents::SkillType::MeleeCombat)
                    {
                        debug!("Agent {} broke a {broke} keeping it off", agent.id);
                    }
                }

                agent.take_damage(landed);
                agent.emotions.record_attack(animal_id, current_tick);

                debug!(
                    "Agent {} was attacked by a hungry animal ({landed:.0} of {damage:.0} damage got through)",
                    agent.id
                );
            }

            // Even a glancing blow is something in the stomach
            if let Some(animal) = self.world.animals.get_mut(&animal_id) {
                animal.feed(fed);
            }
        }
    }

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

        // Where the grown-ups are, so a baby can be counted as being held by
        // one of them
        let carers: Vec<(i32, i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                matches!(
                    agent.state.life_stage,
                    crate::agents::LifeStage::Adolescent
                        | crate::agents::LifeStage::Adult
                        | crate::agents::LifeStage::Elderly
                )
            })
            .map(|agent| agent.state.position)
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

            // Check if agent has shelter
            // Agent has shelter if they're in a completed building
            let mut has_shelter = self.world.buildings.iter().any(|b| {
                b.position == agent_pos && b.is_completed()
            }) || matches!(terrain_type, crate::world::TerrainType::Forest); // Forest provides partial shelter

            // The young are kept warm by whoever is looking after them.
            //
            // A child has no clothing of its own - it cannot gather flax, has
            // no skill to sew and nobody makes anything for it - so left to
            // the weather it runs two or three degrees colder than the adults
            // around it and dies of that. Nearly half of everyone ever born
            // died before growing up, which no birth rate can carry: it is
            // what emptied every settlement inside thirty thousand ticks.
            let too_young_to_manage = matches!(
                agent.state.life_stage,
                crate::agents::LifeStage::Infant | crate::agents::LifeStage::Child
            );

            if !has_shelter && too_young_to_manage {
                let position = agent.state.position;
                has_shelter = carers.iter().any(|carer| {
                    let dx = (carer.0 - position.0) as f32;
                    let dy = (carer.1 - position.1) as f32;
                    (dx * dx + dy * dy).sqrt()
                        <= crate::agents::childcare::MAX_CAREGIVER_DISTANCE
                });
            }

            // Update agent's body temperature based on climate, taking cover
            // into account so that reaching shelter actually warms the agent
            agent.update_temperature_with_shelter(&climate, has_shelter);

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
            food_database: FoodDatabase::default(),
            // A tally of this run, not of the saved one
            actions_taken: std::collections::HashMap::new(),
            actions_failed: std::collections::HashMap::new(),
            actions_failed_because: std::collections::HashMap::new(),
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

