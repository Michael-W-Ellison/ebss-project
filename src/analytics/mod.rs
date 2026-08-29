// src/analytics/mod.rs
//! Analytics, data logging, and emergence detection.

use crate::agents::physiology;
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

/// Doing the thing: the action verbs, one module per family.
pub mod doing;

/// A turn of the world, and one person's turn inside it.
pub mod turn;

/// Wanting: given a drive, what would answer it?
pub mod wanting;

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

use crate::agents::practices::Circumstance;
use crate::agents::wondering::Kept;
use crate::core::DriveType;
use crate::world::FoodDatabase;
use crate::environment::Action;
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
    pub actions_taken: std::collections::BTreeMap<String, u64>,
    /// And how many of those came to nothing.
    ///
    /// An action chosen is not an action that worked. Counting only the
    /// choosing hides the case where a settlement spends a sixth of its life
    /// attempting something that almost never succeeds, which is exactly what
    /// it turned out to be doing.
    pub actions_failed: std::collections::BTreeMap<String, u64>,
    /// And what they said when they did.
    ///
    /// A count of failures tells you a settlement is wasting its time; the
    /// reasons tell you what on. Both are one hash lookup on a path that only
    /// runs when something has already gone wrong, and between them they turn
    /// "the drives ask for things that do not happen" into a list of named
    /// defects.
    pub actions_failed_because: std::collections::BTreeMap<String, u64>,
    /// Questions this settlement put to the world and got an answer to, by
    /// question - see `who_came_back_to_look`. Nobody wrote any of these down
    /// either; they are whatever anybody happened to leave lying about.
    pub what_anybody_found_out: std::collections::BTreeMap<String, u64>,
    /// And what one person told another, by discovery. The one way a thing
    /// somebody worked out has ever had of leaving the head that made it.
    pub what_anybody_was_told: std::collections::BTreeMap<String, u64>,
    /// How much of what was killed or gathered would not go in the pack and
    /// stayed where it fell. The other half of the waste - see
    /// `into_the_pack_or_on_the_ground`.
    pub what_would_not_fit_in_the_pack: u64,
    /// Edible items that actually landed in somebody's pack off a forage.
    ///
    /// The other end of the food ledger: `what_would_not_fit_in_the_pack`
    /// counts what was picked and dropped, and this counts what was picked and
    /// kept. Without it there is no way to tell a settlement that is not
    /// gathering enough from one that is gathering plenty and losing it.
    pub food_items_into_packs: u64,
    /// Where the threat tree came out, by the name of the branch.
    ///
    /// The same argument as `actions_failed_because`, one level earlier. An
    /// action tally can only count the answers a decision reached; it cannot
    /// tell a tree that is working from a tree that is never asked. #66
    /// measured `Freeze` at zero in sixty-four worlds and could say nothing
    /// about which of those it was, because every way of declining looks like
    /// `None` from outside. This counts the declining as well as the deciding.
    pub what_a_threat_came_to: std::collections::BTreeMap<String, u64>,
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
pub(crate) fn determine_placement_approach(building_type: crate::world::BuildingType) -> (PlacementCriteria, PlacementStrategy) {
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
            actions_taken: std::collections::BTreeMap::new(),
            actions_failed: std::collections::BTreeMap::new(),
            actions_failed_because: std::collections::BTreeMap::new(),
            what_a_threat_came_to: std::collections::BTreeMap::new(),
            what_anybody_found_out: std::collections::BTreeMap::new(),
            what_anybody_was_told: std::collections::BTreeMap::new(),
            what_would_not_fit_in_the_pack: 0,
            food_items_into_packs: 0,
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
            Action::TakeFrom { .. } => Some(ActionType::Social),
            Action::FleeFrom { .. } => Some(ActionType::Combat),
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

        let mut rng = crate::core::dice::roll();
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

    /// How far a frightened person gets in one turn.
    ///
    /// Further than a walk, which is the whole difference between running and
    /// going somewhere.
    const HOW_FAR_A_FRIGHTENED_PERSON_GETS: i32 = Self::FAR_ENOUGH_AWAY + 4;

    /// And what it takes out of them.
    const WHAT_RUNNING_COSTS: f32 = 14.0;

    /// What it costs to get a thing out of the pack, or put it back.
    ///
    /// Nearly nothing, because it is nearly nothing - the point of the action
    /// is the turn it takes rather than the effort, and a turn is what a
    /// person spends by doing this instead of something else.
    const WHAT_GETTING_A_THING_OUT_COSTS: f32 = 1.5;

    /// What freezing costs, which is nothing but the turn.
    ///
    /// Deliberately cheap in energy and ruinous in every other way: an agent
    /// that freezes has spent a turn not getting away from the thing that is
    /// about to reach it.
    const WHAT_FREEZING_COSTS: f32 = 0.5;

    /// What digging a pit takes out of somebody.
    ///
    /// A real morning's work, and deliberately so: this is the most expensive
    /// single act in the model, because it is the one that buys a settlement a
    /// February.
    const WHAT_DIGGING_A_PIT_COSTS: f32 = 22.0;

    /// How near a fire you have to be to hang something in the smoke of it.
    const WITHIN_REACH_OF_THE_HEARTH: i32 = 2;

    /// How far gone a thing can be and still be worth preserving.
    ///
    /// Preserving does not undo what has already happened to it: all you get
    /// from drying carrion is dry carrion.
    const TOO_FAR_GONE_TO_KEEP: f32 = 0.5;

    /// What laying food out or hanging it in the smoke takes.
    const WHAT_DRYING_COSTS: f32 = 3.0;

    /// How much stone comes out of a hole somebody digs.
    const WHAT_COMES_OUT_OF_A_HOLE: u32 = 3;

    /// How much a person carries away from a store in one go.
    const WHAT_A_PERSON_TAKES_OUT: u32 = 8;

    /// And how much they keep on them when they are standing on it.
    ///
    /// One meal. The store is right there.
    ///
    /// This wants to be *less* than `ENOUGH_NOT_TO_OPEN_THE_STORE`, and the
    /// obvious-looking fix of raising it above so that nobody buries food and
    /// then immediately digs it up again is wrong: at five, a person holding
    /// five or fewer has nothing spare to bury at all, and `Cover` was refused
    /// **3,672 times out of 3,729** with the store left empty. The small
    /// churn is the cheaper of the two failures by a wide margin - it is under
    /// one per cent of the turns in a world, against a store that does not
    /// exist.
    const WHAT_A_PERSON_KEEPS_ON_THEM: u32 = 1;

    /// How far somebody will walk for a thing they can see lying on the ground.
    const WORTH_WALKING_OVER_FOR: u32 = 12;

    /// Something lying about that this agent has a use for.
    ///
    /// A thing on the ground is a thing somebody else made and did not take
    /// with them: a worn axe beside a man who drowned, a spear thrown and not
    /// recovered, whatever fell out of a full pack. Picking it up is the
    /// cheapest way there is to get a tool, and it is why what a people makes
    /// outlives the people who made it.
    /// A material within reach that this agent has never done anything with.
    ///
    /// Only what is close enough to be a detour rather than an expedition,
    /// only what the ground here actually offers, and only where there is a
    /// working nobody here has found out yet that wants it. A person who has
    /// already tried everything clay does walks past the clay.
    fn something_nobody_has_tried_within_reach(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<String> {
        use crate::environment::making;

        // What an untried working wants, that this pack has not got enough of
        let wanted: Vec<&'static str> = making::every_working_to_find_out()
            .filter(|working| !agent.what_i_found_out().contains(working.makes))
            .filter(|working| agent.how_many_i_have(working.to) < working.how_much)
            .map(|working| working.to)
            .collect();

        if wanted.is_empty() {
            return None;
        }

        // And which of those the ground within a short walk is offering.
        //
        // Scanned directly rather than through `nearest_resource_within`,
        // which hands back a position: two nodes can sit on one tile, and
        // looking the position up again could just as easily find the wood
        // as the clay.
        let here = crate::world::Position::new(agent_position.0, agent_position.1);

        let now = self.current_tick;

        self.world
            .resources
            .iter()
            .filter(|resource| resource.amount > 0)
            .filter(|resource| {
                !agent
                    .exploration_knowledge
                    .is_it_picked_out(resource.position, now)
            })
            .filter(|resource| here.distance_to(&resource.position) <= Self::AS_FAR_AS_CURIOSITY_WALKS)
            .filter_map(|resource| {
                Self::gathered_as(resource.resource_type)
                    .filter(|named| wanted.contains(named))
                    .map(|named| (named, here.distance_to(&resource.position)))
            })
            .min_by_key(|(_, apart)| *apart)
            .map(|(named, _)| named.to_string())
    }

    /// What a resource node is called once it is in a pack.
    ///
    /// The same vocabulary `Gather` answers to, kept here so that the
    /// decision and the executor cannot drift apart.
    fn gathered_as(what: crate::world::ResourceType) -> Option<&'static str> {
        use crate::world::ResourceType;

        Some(match what {
            ResourceType::Wood => "wood",
            ResourceType::Stone => "stone",
            ResourceType::Iron => "iron",
            ResourceType::Food => "food",
            ResourceType::Clay => "clay",
            ResourceType::Salt => "salt",
            ResourceType::Sand => "sand",
            ResourceType::Coal => "coal",
            ResourceType::Flax => "flax",
            ResourceType::Cotton => "cotton",
            ResourceType::Grain => "grain",
            ResourceType::Greens => "greens",
            ResourceType::Roots => "roots",
            ResourceType::Herbs => "herbs",
            ResourceType::Fish => "fish",
            ResourceType::Meat => "meat",
            ResourceType::Hides => "hides",
            ResourceType::Wool => "wool",
            ResourceType::Honey => "honey",
            _ => return None,
        })
    }

    /// How far somebody will go out of their way for a handful of something
    /// they have no use for.
    ///
    /// A detour, not an expedition. Curiosity does not outrank supper.
    const AS_FAR_AS_CURIOSITY_WALKS: u32 = 12;

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
    /// Somebody within reach worth taking something off.
    ///
    /// Decided on drive demand, which is what the specification asks for and
    /// what the first cut of this did not do. That one was a temperament roll
    /// - a base chance, nudged by Honest and Greedy and by whether the agent
    /// was starving - and it never looked at what was being taken or what it
    /// was worth. It fired once in eight worlds of ten thousand ticks, and
    /// when it did fire the agent had no idea whether the thing it had just
    /// robbed somebody for was any use to it.
    ///
    /// Now: what would this answer, against what it would cost me later. The
    /// cost runs through the bonds, because in this model everything a person
    /// gets from other people runs through the bonds. And a primary drive
    /// past bearing sets the cost aside, because a man who will be dead by
    /// morning is not weighing his reputation.
    fn somebody_to_take_from(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        let me = self
            .population
            .agents
            .iter()
            .position(|other| other.id == agent.id)?;

        // Who would see it. The same reckoning a liar makes about the people
        // in earshot - it is the same kind of decision.
        let watching: Vec<&crate::agents::Agent> = self
            .population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP
            })
            .collect();

        // And what this agent gets from them, which is what it would be
        // spending
        let bonds = if watching.is_empty() {
            0.0
        } else {
            watching
                .iter()
                .map(|them| {
                    agent
                        .relationships
                        .get_relationship(&them.id)
                        .map(|bond| bond.bond_strength)
                        .unwrap_or(0.0)
                })
                .sum::<f32>()
                / watching.len() as f32
        };

        let cost = agent.what_taking_it_would_cost_me(watching.len(), bonds);

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
            // The best thing anybody standing here has that this agent wants
            .filter_map(|(them, they)| {
                let (what, how_many) = self.what_i_would_hand_over(them, me)?;
                let gain = agent.what_taking_this_would_answer(&what, how_many);
                Some((they.id, gain))
            })
            .filter(|(_, gain)| agent.would_i_take_it(*gain, cost))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(who, _)| who)
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

    /// Whether the sky is doing anything that would dry a thing laid out in
    /// it.
    fn is_the_sky_clear(&self) -> bool {
        matches!(
            self.world.climate.weather.weather_type,
            crate::environment::WeatherType::Clear
                | crate::environment::WeatherType::PartlyCloudy
        )
    }

    /// What the world is doing where this agent is standing.
    ///
    /// Nobody chooses these and nobody is asked about them. They are written
    /// down against every attempt an agent makes, and what the agent works out
    /// afterwards is which of them go with a thing working - see
    /// [`crate::agents::practices::Circumstance`].
    ///
    /// This is the whole of the mechanism by which a lesson can be about a
    /// situation nobody named. Everything that had to be a rule or a discovery
    /// flag before - laying fish out only pays under a clear sky, firing clay
    /// only works at a fire, greens are a spring thing - is in principle
    /// reachable from here without anybody writing it down, because the sky,
    /// the fire and the season are all in this list and the arithmetic that
    /// reads them does not know or care what any of them is for.
    fn what_it_is_like_here(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Vec<Circumstance> {
        use crate::environment::seasons::Season;
        use crate::world::Position;

        let mut here = Vec::with_capacity(4);

        if self.is_the_sky_clear() {
            here.push(Circumstance::ClearSky);
        } else if self
            .world
            .climate
            .weather
            .weather_type
            .precipitation_intensity()
            > 0.0
        {
            here.push(Circumstance::Raining);
        }

        if self
            .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
            .is_some()
        {
            here.push(Circumstance::AFireToHand);
        }

        // Standing under one, not within a walk of one: this is a fact about
        // the afternoon, and a roof across the camp keeps nothing off you.
        if self.world.buildings.iter().any(|building| {
            building.position.x == agent_position.0 && building.position.y == agent_position.1
        }) {
            here.push(Circumstance::UnderARoof);
        }

        if self.population.agents.iter().any(|other| {
            other.state.is_alive
                && other.id != agent.id
                && (other.state.position.0 - agent_position.0).abs() <= Self::WITHIN_SIGHT
                && (other.state.position.1 - agent_position.1).abs() <= Self::WITHIN_SIGHT
        }) {
            here.push(Circumstance::OtherPeopleAbout);
        }

        let by_water = (-Self::WITHIN_A_FEW_PACES..=Self::WITHIN_A_FEW_PACES).any(|dy| {
            (-Self::WITHIN_A_FEW_PACES..=Self::WITHIN_A_FEW_PACES).any(|dx| {
                self.world
                    .grid
                    .get_tile(&Position::new(agent_position.0 + dx, agent_position.1 + dy))
                    .is_some_and(|tile| tile.terrain.is_aquatic())
            })
        });
        if by_water {
            here.push(Circumstance::ByWater);
        }

        here.push(match self.world.climate.current_season() {
            Season::Spring => Circumstance::InSpring,
            Season::Summer => Circumstance::InSummer,
            Season::Fall => Circumstance::InAutumn,
            Season::Winter => Circumstance::InWinter,
        });

        here
    }

    /// How far off somebody else still counts as being about.
    const WITHIN_SIGHT: i32 = 6;

    /// And how far off water still counts as being here.
    const WITHIN_A_FEW_PACES: i32 = 2;

    /// How far somebody will walk to a store, either to fill it or to draw on
    /// it.
    const WORTH_WALKING_TO_THE_STORE: u32 = 14;

    /// Somebody of this agent's own who is worse off than it is, and hungry
    /// enough that the difference matters.
    ///
    /// This is the gift that costs, and it is deliberately kept apart from
    /// `somebody_to_give_to`: that one hands over what is spare, and what is
    /// spare is by definition not a sacrifice. Here an agent hands over food
    /// it is going to want itself, because somebody it loves will not last
    /// the week without it.
    fn somebody_of_mine_who_needs_it_more(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        // Nothing to give
        agent.find_best_food_to_eat()?;

        // And an agent already past bearing itself keeps what it has. This is
        // not selfishness so much as arithmetic: two dead people is not
        // better than one.
        if agent.state.is_starving() && agent.nutrition.is_starving() {
            return None;
        }

        self.population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs())
                    <= Self::CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER
            })
            .filter(|them| {
                agent
                    .relationships
                    .get_relationship(&them.id)
                    .is_some_and(|bond| bond.is_loved_one())
            })
            // Worse off than this agent, and badly enough for it to count
            .filter(|them| them.nutrition.is_starving() || them.state.is_starving())
            .filter(|them| them.find_best_food_to_eat().is_none())
            .min_by(|a, b| {
                a.nutrition
                    .energy_reserves
                    .partial_cmp(&b.nutrition.energy_reserves)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|them| them.id)
    }

    /// What going without for somebody is worth to them, against an ordinary
    /// gift.
    ///
    /// More, and it should be: a thing somebody could spare is not the same
    /// as a thing they could not.
    const WHAT_GOING_WITHOUT_IS_WORTH: f32 = 0.8;

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

        let mut rng = crate::core::dice::roll();

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

        let mut rng = crate::core::dice::roll();
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

    /// The bare name of an action, without whatever it is aimed at.
    ///
    /// `Gather { resource_type: "berries" }` and `Gather { resource_type:
    /// "wood" }` are the same kind of day's work for counting purposes.
    /// What to book an action under in `actions_taken`.
    ///
    /// Almost always its own name. The one exception is the fear branch's
    /// fallback, which runs from another *person* and comes out as an
    /// ordinary `Move` that nothing downstream could tell from a stroll.
    ///
    /// It used to be the other way round - everything chosen in the fear
    /// branch was booked as "Flee", `FleeFrom` and `Freeze` included. Both of
    /// those name themselves, and the failure path below books by
    /// `name_of`, so a *refused* run went under `FleeFrom` while a run that
    /// happened went under "Flee": the verb showed 19,626 failures against no
    /// attempts at all, and `Freeze` showed as never once taken in
    /// sixty-four worlds. Two names for one thing, which is this project's
    /// oldest defect and its sixth appearance. The invariant it broke is that
    /// nothing can fail at a thing it was never recorded doing.
    fn what_to_book(action: &Action, running_away: bool) -> String {
        if running_away && matches!(action, Action::Move { .. }) {
            return "Flee".to_string();
        }

        Self::name_of(action)
    }

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
    /// Put these in the pack, and leave on the ground whatever will not go in.
    ///
    /// `Inventory::add_item` enforces the weight limit and returns `false`,
    /// and butchering ignored what it returned - so a deer that came to more
    /// than a man could carry was **silently deleted**, every time, and
    /// counted nowhere. A hunter walked away from three quarters of an animal
    /// and the world behaved as though the animal had been that size.
    ///
    /// What will not fit stays where it fell. It can be come back for, it
    /// counts against the hunt when it rots, and it is there for something
    /// else to find - which is what makes a second trip a decision rather than
    /// a formality.
    fn into_the_pack_or_on_the_ground(
        &mut self,
        agent_index: usize,
        items: Vec<crate::agents::InventoryItem>,
        where_it_fell: crate::world::Position,
    ) -> u32 {
        let tick_now = self.current_tick;
        let mut left_behind = 0u32;

        for item in items {
            let each = item.weight_per_unit * item.how_much_lighter_it_is();

            let room = self.population.agents[agent_index]
                .inventory
                .weight_capacity_remaining();

            let fits = if each > 0.0 {
                ((room / each).floor() as u32).min(item.quantity)
            } else {
                item.quantity
            };

            if fits > 0 {
                let mut taking = item.clone();
                taking.quantity = fits;

                // The slot limit can still refuse a kind of thing this pack
                // has no room for, in which case the lot stays where it fell
                if !self.population.agents[agent_index].inventory.add_item(taking) {
                    self.world.somebody_left_this(item.clone(), where_it_fell, tick_now);
                    left_behind += item.quantity;
                    continue;
                }
            }

            let over = item.quantity - fits;
            if over > 0 {
                let mut leaving = item.clone();
                leaving.quantity = over;
                self.world.somebody_left_this(leaving, where_it_fell, tick_now);
                left_behind += over;
            }
        }

        if left_behind > 0 {
            self.what_would_not_fit_in_the_pack =
                self.what_would_not_fit_in_the_pack.saturating_add(left_behind as u64);
            debug!(
                "Agent {} left {left_behind} behind at {where_it_fell:?}",
                self.population.agents[agent_index].id
            );
        }

        left_behind
    }

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
    /// The biggest thing a thrown stone brings down.
    ///
    /// Rabbits, birds and the like sit at ten to fifteen health in the fauna
    /// tables; a deer is thirty and upwards. So the line falls between them,
    /// and above it "hunting any larger animal requires at least a spear".
    const AS_BIG_AS_A_STONE_WILL_KILL: f32 = 20.0;

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

                // And it fouls the ground it fell on, which is the whole
                // reason a body is a thing you want to be away from. Until
                // now a corpse was a nutrient deposit and nothing else -
                // agents walked over their own dead with no more consequence
                // than walking over leaf mould.
                tile.soil.somebody_voided_here(
                    soft * Self::HOW_MUCH_OF_A_BODY_IS_FOULING,
                );
            }
        }

        self.what_the_dead_left_behind();
    }

    /// What share of what a body is left on the ground counts as fouling.
    ///
    /// A body is a great deal of soft matter and only some of it is the part
    /// that makes ground foul, so this is well under one. What it has to do
    /// is put a fresh corpse comfortably over `FOUL_ENOUGH_TO_WALK_AWAY_FROM`,
    /// so that people move off ground somebody died on and come back to it
    /// once it has broken down.
    const HOW_MUCH_OF_A_BODY_IS_FOULING: f32 = 0.4;

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

    /// The whole answer to a thing that would kill you, in the order the
    /// specification gives it.
    ///
    /// > if this threat seems like something he can overcome, the man attacks.
    /// > if not, the man flees in fear. if fleeing does not seem like an
    /// > option, then the only alternative is to fight. if fighting does not
    /// > seem like an option, then the only alternative is to flee. if the
    /// > agent cannot select between one of those two options, they freeze.
    ///
    /// The appraisal has already answered the first question: it comes out as
    /// anger where the thing can be overcome and fear where it cannot - see
    /// `Agent::appraise_what_is_there`. What was missing was everything after
    /// it. An agent who could not overcome the thing ran, and if there was
    /// nowhere to run it simply went back to gathering berries with a wolf at
    /// its elbow; an agent who could overcome it fought, and if its arms were
    /// gone it did the same. Neither of the two cornered cases existed, and
    /// nor did the third answer.
    fn how_this_one_answers_a_threat(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        self.what_this_threat_comes_to(agent, agent_position).1
    }

    /// The same tree, and the name of the branch it came out of.
    ///
    /// Every way of declining used to look like `None` from outside, which is
    /// why #66 could measure `Freeze` at zero in sixty-four worlds and say
    /// nothing about whether the tree was working or idle. The name is what
    /// `Simulation::what_a_threat_came_to` counts.
    fn what_this_threat_comes_to(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> (&'static str, Option<Action>) {
        // What it is, where it is, and how hard it hits
        let named = agent
            .emotions
            .what_frightens_me_most()
            .map(|(kind, _)| (kind, false))
            .or_else(|| {
                agent
                    .emotions
                    .what_angers_me_most()
                    .map(|(kind, _)| (kind, true))
            });

        // Frightened or angry enough to act, and not at any creature. A
        // grudge against a neighbour comes out here: the branches below this
        // one deal with people.
        let Some((kind, standing)) = named else {
            return ("nothing named", None);
        };

        // It named something that is not about. The feeling outlasts the
        // thing by however long the decay takes.
        let Some((which, where_it_is, paces)) = self.nearest_of_kind(kind, agent_position) else {
            return ("named, but not about", None);
        };

        let coming = self
            .world
            .animals
            .get_all()
            .iter()
            .find(|animal| animal.id == which)
            .and_then(|animal| self.world.animals.get_species(&animal.species_id))
            .map(|species| species.attack_damage)
            .unwrap_or(0.0);

        // Standing your ground is something you do to what is in front of
        // you. Nobody crosses a field to pick a fight with a wolf, and an
        // agent that is not afraid of a thing four paces off has no business
        // with it at all - it gets on with its day.
        if standing && paces > Self::WITHIN_A_STEP_OR_TWO {
            return ("not worth crossing to", None);
        }

        let could_fight = agent.could_i_fight_at_all(coming);

        // Somebody of this agent's own, in the way of the thing, who cannot
        // deal with it themselves. A person does not run from a wolf that is
        // standing over their child, whatever the odds are - and the odds are
        // exactly what this sets aside. It is the one place in the model
        // where an agent knowingly takes the worse of two options.
        if could_fight && self.somebody_of_mine_is_in_the_way(agent, where_it_is, coming) {
            return (
                "stands over one of its own",
                Some(if paces <= Self::HUNT_REACH {
                    Action::Fight {
                        animal_id: which,
                        weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
                    }
                } else {
                    Action::Move {
                        target: (where_it_is.0, where_it_is.1, agent_position.2),
                    }
                }),
            );
        }

        let could_run = agent.could_i_run_at_all(Self::WHAT_RUNNING_COSTS)
            && self.is_there_anywhere_to_run(
                &agent.exploration_knowledge,
                agent_position,
                where_it_is,
            );

        let fight = || {
            if paces <= Self::HUNT_REACH {
                Action::Fight {
                    animal_id: which,
                    weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
                }
            } else {
                // Close the last pace or two, and no further
                Action::Move {
                    target: (where_it_is.0, where_it_is.1, agent_position.2),
                }
            }
        };

        let run = || Action::FleeFrom {
            away_from: (where_it_is.0, where_it_is.1, agent_position.2),
        };

        match (standing, could_fight, could_run) {
            // What it wanted to do, and it can
            (true, true, _) => ("stands its ground", Some(fight())),
            (false, _, true) => ("runs", Some(run())),

            // Cornered: it wanted to run and there is nowhere to go, so it
            // turns and fights. Or it wanted to fight and cannot lift an arm,
            // so it goes.
            (false, true, false) => ("cornered, so fights", Some(fight())),
            (true, false, true) => ("cannot fight, so runs", Some(run())),

            // Neither. This is the case the decision never had an answer for.
            (_, false, false) => ("freezes", Some(Action::Freeze)),
        }
    }

    /// Whether somebody this agent loves is in the way of the thing, and could
    /// not deal with it themselves.
    ///
    /// The paradigm case, and the reason this exists: a wolf standing over a
    /// child. The child cannot fight it and very likely cannot outrun it, and
    /// the parent is the only thing between them. What comes of that is a
    /// fight the parent may well lose, which is the point - the specification
    /// asks for agents that can lay down their lives for their family, and an
    /// agent that only ever fights what it can beat cannot do that.
    fn somebody_of_mine_is_in_the_way(
        &self,
        agent: &crate::agents::Agent,
        where_it_is: (i32, i32),
        coming: f32,
    ) -> bool {
        self.population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                agent
                    .relationships
                    .get_relationship(&them.id)
                    .is_some_and(|bond| bond.is_loved_one())
            })
            // In the way of it: nearer the thing than a person would choose
            // to be
            .filter(|them| {
                (them.state.position.0 - where_it_is.0)
                    .abs()
                    .max((them.state.position.1 - where_it_is.1).abs())
                    <= Self::STANDING_OVER_THEM
            })
            // And unable to do anything about it. Somebody who can fight it
            // themselves is not being protected, they are being joined
            .any(|them| !them.could_i_fight_at_all(coming))
    }

    /// How near the thing somebody has to be before they count as being in
    /// its way.
    const STANDING_OVER_THEM: i32 = 2;

    /// Whether there is any ground to run to, away from the thing.
    ///
    /// Half the answer to "fleeing does not seem like an option": a man with
    /// his back to a cliff has nowhere to go however much he would like to.
    /// The other half is the body, and belongs to the agent - see
    /// `Agent::could_i_run_at_all`.
    ///
    /// It asks the running itself, rather than asking the same question in
    /// its own words. It used to have its own: three ways out at three
    /// paces, where the running tried three ways out at nineteen. A man
    /// three paces from a shoreline with the thing inland has somewhere to
    /// go at three paces and nothing but water at nineteen, so the decision
    /// said run and the running said there was nowhere to run - and nothing
    /// about the next turn was different, so it said it again. One measured
    /// world produced 76,644 of those refusals, three quarters of every turn
    /// taken in the settlement. Two vocabularies for one question is the
    /// recurring defect; this is the fifth time it has cost something.
    fn is_there_anywhere_to_run(
        &self,
        remembers: &crate::agents::exploration::ExplorationKnowledge,
        from: (i32, i32, i32),
        away_from: (i32, i32),
    ) -> bool {
        self.where_this_one_would_run(remembers, from, away_from)
            .is_some()
    }

    /// Where a frightened person actually goes, if anywhere.
    ///
    /// Eight ways out rather than three, and each of them tried at the full
    /// bolt first and then at every shorter distance down to a single pace.
    /// Both of those are the same point: the ways out that exist are not
    /// always the ways out somebody would choose, and a person hemmed in by
    /// water on three sides does not stand still because the gap is narrow.
    /// Behind is in the list too - running past the thing is a poor answer,
    /// and the scoring says so, but it beats being caught standing.
    fn where_this_one_would_run(
        &self,
        remembers: &crate::agents::exploration::ExplorationKnowledge,
        from: (i32, i32, i32),
        away_from: (i32, i32),
    ) -> Option<(i32, i32, i32)> {
        let dx = from.0 - away_from.0;
        let dy = from.1 - away_from.1;
        let span = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0);

        let straight = (dx as f32 / span, dy as f32 / span);

        // Straight away, then an eighth-turn either side, then a quarter,
        // and so round to behind. Listed nearest-to-away first, so that
        // where the scoring cannot separate two landings the one that was
        // asked about first wins.
        let ways = [0i32, 1, -1, 2, -2, 3, -3, 4].map(|eighths| {
            let (sin, cos) = (eighths as f32 * std::f32::consts::FRAC_PI_4).sin_cos();
            (
                straight.0 * cos - straight.1 * sin,
                straight.0 * sin + straight.1 * cos,
            )
        });

        let bolt = Self::HOW_FAR_A_FRIGHTENED_PERSON_GETS;

        ways.iter()
            .filter_map(|(wx, wy)| {
                // The furthest this way goes. Getting clear is the point of
                // running, so a short bolt is a fallback and not a choice.
                (1..=bolt).rev().find_map(|paces| {
                    let landed = (
                        (from.0 as f32 + wx * paces as f32).round() as i32,
                        (from.1 as f32 + wy * paces as f32).round() as i32,
                        from.2,
                    );

                    let moved = landed.0 != from.0 || landed.1 != from.1;

                    (moved && self.is_passable_tile(landed.0, landed.1)).then_some(landed)
                })
            })
            .min_by(|one, other| {
                self.how_poor_a_way_out(remembers, from, away_from, *one)
                    .partial_cmp(&self.how_poor_a_way_out(remembers, from, away_from, *other))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// What is wrong with running that way.
    ///
    /// Two things, and they pull against each other: what this one remembers
    /// happening where it would land, and how much ground it puts between
    /// itself and the thing. Running headlong into the wood where the pack
    /// lives is how somebody gets away from one animal and into four.
    fn how_poor_a_way_out(
        &self,
        remembers: &crate::agents::exploration::ExplorationKnowledge,
        from: (i32, i32, i32),
        away_from: (i32, i32),
        landed: (i32, i32, i32),
    ) -> f32 {
        let bad = remembers.how_bad_is_it_there(
            crate::world::Position::new(landed.0, landed.1),
            self.current_tick,
        );

        let off = |where_it_is: (i32, i32)| {
            let dx = (where_it_is.0 - away_from.0) as f32;
            let dy = (where_it_is.1 - away_from.1) as f32;
            (dx * dx + dy * dy).sqrt()
        };

        let gained =
            (off((landed.0, landed.1)) - off((from.0, from.1))) / Self::HOW_FAR_A_FRIGHTENED_PERSON_GETS as f32;

        bad - Self::WHAT_GETTING_CLEAR_IS_WORTH * gained.clamp(-1.0, 1.0)
    }

    /// What a full bolt's worth of ground is worth against a place somebody
    /// remembers going badly.
    ///
    /// Less than the worst thing that can be remembered and more than the
    /// least, which is the whole of what it has to be: a wood a man was
    /// mauled in is not worth running into to gain nineteen paces, and a
    /// field he once went hungry in is not worth staying put for.
    const WHAT_GETTING_CLEAR_IS_WORTH: f32 = 0.5;

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

                // What a thing does to somebody who has done nothing to it.
                // Reading this off `attack_damage` said a rabbit was a threat,
                // because a rabbit will bite you if you pick it up - and once
                // several of a thing began adding up, a herd of reindeer came
                // to about a wolf and the settlement stopped hunting.
                let temper = species.behavior.how_much_it_menaces_you();
                if temper <= 0.0 || species.attack_damage <= 0.0 {
                    return None;
                }

                // What it is worth in a fight, on the same scale an agent
                // reckons itself on: a healthy body, and what it can do with it
                let condition = (animal.current_health / species.health.max(1.0)).clamp(0.0, 1.0);
                let menace = (species.attack_damage / 20.0).clamp(0.1, 2.0) * temper;

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

            // Everything within sight of this agent that would eat it. All of
            // it, not the worst of it: the appraisal used to take the single
            // largest thing in view and throw the rest away, so a man
            // surrounded by four wolves faced whichever one happened to be
            // nearest and felt no differently about it than he would about
            // one.
            let closing: Vec<(f32, &String)> = hunters
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
                .collect();

            // What it is called is the name of the worst of them: a man
            // hemmed in by wolves is afraid of wolves, whatever else is in
            // the field
            let worst = closing
                .iter()
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, what)| (*what).clone());

            match worst {
                Some(what) => {
                    let all: Vec<f32> = closing.iter().map(|(strength, _)| *strength).collect();
                    let pack = crate::agents::ThreatAssessment::a_pack_of(&all);

                    agent.appraise_what_is_there(
                        pack,
                        crate::agents::EmotionSource::Creature(what),
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

    /// Whoever is near enough to a question they left open goes and looks.
    ///
    /// This is the half of "what happens if" that no other kind of curiosity
    /// in this model has: the answer arrives days later and somewhere else,
    /// and somebody has to be standing there to get it. What is learned is
    /// learned from the change — the meat has gone off, the strips have dried,
    /// the clay is not clay any more — and it is recorded against the
    /// circumstances the thing was *left* in rather than the ones it is found
    /// in, because the rain that ruined it has usually stopped by then.
    ///
    /// A question that is never answered is also an answer. Four days on, a
    /// thing exactly as it was left teaches that leaving that thing about
    /// comes to nothing, and the agent stops doing it - which is the whole
    /// difference between an experiment and a habit.
    fn who_came_back_to_look(&mut self) {
        let now = self.current_tick;

        for index in 0..self.population.agents.len() {
            if !self.population.agents[index].state.is_alive {
                continue;
            }

            if self.population.agents[index].wonderings.is_empty() {
                continue;
            }

            let standing = self.population.agents[index].state.position;

            // What is answerable this tick, worked out with the world borrowed
            // and the agent not.
            let mut answers: Vec<(String, bool, Vec<Circumstance>, Option<&'static str>)> =
                Vec::new();
            let mut done: Vec<usize> = Vec::new();

            for (which, wondering) in self.population.agents[index].wonderings.iter().enumerate() {
                let near = (standing.0 - wondering.where_it_is.x)
                    .abs()
                    .max((standing.1 - wondering.where_it_is.y).abs());

                let close_enough = near
                    <= crate::agents::wondering::Wondering::CLOSE_ENOUGH_TO_GO_AND_LOOK;

                // Where to go and look depends on what was done. Burying
                // puts a thing in a hole and salting leaves it in the pack;
                // only leaving it out puts it on the grass.
                let as_it_is = match wondering.where_to_look() {
                    Kept::OnTheGround => self
                        .world
                        .what_is_lying_at(&wondering.where_it_is)
                        .into_iter()
                        .find(|left| {
                            left.item.item_id == wondering.what
                                || left.item.item_id != wondering.as_it_was.called
                        })
                        .map(|left| crate::agents::wondering::Watched::of(&left.item)),
                    Kept::InThePit => self
                        .world
                        .pit_at(wondering.where_it_is)
                        .and_then(|pit| {
                            pit.holds
                                .iter()
                                .find(|item| item.item_id == wondering.what)
                        })
                        .map(crate::agents::wondering::Watched::of),
                    // In the pack, which goes where its owner goes - so this
                    // one is always answerable and never wants a walk back.
                    Kept::InMyPack => self.population.agents[index]
                        .inventory
                        .get_item(&wondering.what)
                        .map(crate::agents::wondering::Watched::of),
                };

                let can_see_it = close_enough
                    || wondering.where_to_look() == Kept::InMyPack;
                let waited = wondering.given_up_on(now);

                match (can_see_it, as_it_is) {
                    (true, Some(as_it_is)) => {
                        // What the verb makes of it - and the verb decides,
                        // because a buried thing that has not changed is the
                        // whole point of burying it and a thing left on the
                        // grass that has not changed is nothing at all.
                        if let Some(became) = wondering.what_it_means(&as_it_is, waited) {
                            answers.push((
                                wondering.called(),
                                became.for_the_better,
                                wondering.in_this.clone(),
                                Some(became.says),
                            ));
                            done.push(which);
                        }
                    }
                    // Somebody walked off with it, it was eaten, or it rotted
                    // away to nothing. No answer, and none to be had.
                    (true, None) => done.push(which),
                    (false, _) => {
                        if waited {
                            done.push(which);
                        }
                    }
                }
            }

            if answers.is_empty() && done.is_empty() {
                continue;
            }

            let agent = &mut self.population.agents[index];

            for (called, for_the_better, in_this, says) in answers {
                agent
                    .lessons
                    .record_particular_here(&called, for_the_better, &in_this);

                if let Some(says) = says {
                    debug!("Agent {} came back and found {called}: {says}", agent.id);
                }

                *self.what_anybody_found_out.entry(called).or_insert(0) += 1;
            }

            for which in done.into_iter().rev() {
                agent.wonderings.remove(which);
            }
        }
    }

    /// Whoever was standing near enough to see a thing dry out learns what
    /// dried it.
    ///
    /// The world does the drying; this is what turns it into something a
    /// person knows. It is the same shape as the four ways into farming: a
    /// thing happens, and whoever is near enough to see it happen takes the
    /// lesson. Nobody here is born knowing that cut flesh laid in the sun
    /// keeps and whole flesh laid in the sun does not - it has to be watched
    /// once.
    fn who_saw_that_dry(&mut self) {
        let dried: Vec<(crate::world::Position, String)> =
            std::mem::take(&mut self.world.what_dried_in_the_sun);

        if dried.is_empty() {
            return;
        }

        for (where_it_is, what) in dried {
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive {
                    continue;
                }

                let paces = (agent.state.position.0 - where_it_is.x)
                    .abs()
                    .max((agent.state.position.1 - where_it_is.y).abs());

                if paces > Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    continue;
                }

                if agent.found_out_how_to(Self::THAT_LAYING_IT_OUT_KEEPS_IT) {
                    debug!(
                        "Agent {} watched {what} dry out at {where_it_is:?}",
                        agent.id
                    );
                }

                // And what it was worth: something that would have been
                // carrion is supper
                agent.lessons.record_particular("dry", true);
            }
        }
    }

    /// Living on a midden, or beside somebody's body.
    ///
    /// "Spending time near dead bodies or fresh waste" - and the two are one
    /// question here, because a corpse fouls the ground it falls on the same
    /// way a midden does. Agents already step off foul ground when they
    /// notice it; what was missing is any reason to, beyond distaste.
    ///
    /// Nothing here is certain and nothing is fast. Standing on fouled ground
    /// for one tick is almost always nothing; living on it is what tells.
    fn what_the_ground_underfoot_does(&mut self) {
        use rand::Rng;

        if self.current_tick % Self::HOW_OFTEN_THE_GROUND_IS_ASKED != 0 {
            return;
        }

        let now = self.current_tick;
        let mut rng = crate::core::dice::roll();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive || agent.is_ailing() {
                continue;
            }

            let here = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
            let Some(tile) = self.world.grid.get_tile(&here) else {
                continue;
            };

            if !tile.soil.is_foul() {
                continue;
            }

            // How foul, as a share of as foul as ground gets.
            let how_bad = (tile.soil.fouling / crate::world::Soil::AS_FOUL_AS_IT_GETS)
                .clamp(0.0, 1.0);
            let odds = Self::HOW_OFTEN_FOUL_GROUND_TELLS * how_bad as f64;

            if rng.gen_bool(odds.clamp(0.0, 1.0)) {
                agent.taken_ill_with(
                    crate::agents::Agent::OFF_FOUL_GROUND,
                    0.25 + 0.35 * how_bad,
                    now,
                );
            }
        }
    }

    /// How often the ground under everybody is asked about.
    ///
    /// Once a day rather than every tick: this is a question about living
    /// somewhere, not about walking across it.
    const HOW_OFTEN_THE_GROUND_IS_ASKED: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// And how often a day spent on the worst ground there is makes somebody
    /// ill.
    ///
    /// One day in twenty at the very worst, which over a season on a midden
    /// is most of a settlement and over a week is almost nobody. Fouling
    /// breaks down, so this is a pressure to move rather than a sentence.
    const HOW_OFTEN_FOUL_GROUND_TELLS: f64 = 0.05;

    /// What everybody can see that frightens them, and where it was.
    ///
    /// The map an agent carries had explored tiles, resource positions with
    /// an age and a source, buildings, storage and terrains - a real picture
    /// of the world's *things* - and nothing at all about danger. Somebody
    /// could be mauled at a ford and walk back to the same ford the next
    /// morning, because there was nowhere for "there are wolves in that wood"
    /// to live.
    ///
    /// This is the sight pass for it. What goes in is what an agent would
    /// actually notice: a beast within sight that means it harm, and how the
    /// odds looked at the time. Reading the odds rather than the species is
    /// what stops a man with a spear being as frightened of a wolf as a child
    /// with nothing.
    fn what_everybody_saw_that_frightened_them(&mut self) {
        if self.current_tick % Self::HOW_OFTEN_ANYBODY_LOOKS_ROUND != 0 {
            return;
        }

        let now = self.current_tick;

        // Everything alive that means anybody harm, with what it is worth in
        // a fight and what to call it
        let beasts: Vec<((i32, i32), f32, f32, String)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive())
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                let menace = species.behavior.how_much_it_menaces_you();
                if menace <= 0.0 {
                    return None;
                }
                let worth = Self::what_a_beast_is_worth_in_a_fight(
                    animal.current_health,
                    species.health,
                    species.attack_damage,
                );
                Some((animal.position, worth, menace, species.name.clone()))
            })
            .collect();

        if beasts.is_empty() {
            return;
        }

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            let armed = agent
                .what_i_have_to_work_with(crate::agents::SkillType::MeleeCombat)
                .is_some();
            let i_am_worth = Self::WHAT_A_PERSON_IS_WORTH_TO_A_BEAST
                * if armed { Self::WHAT_A_SPEAR_ADDS } else { 1.0 };

            // Everything in sight that means harm, taken together.
            //
            // Together rather than one at a time, because that is what the
            // specification says a threat is: "a man encountering 4 wolves
            // should see them as a threat". One wolf is not much to a man
            // with a spear and four of them are a different afternoon
            // entirely, and judging each separately would have him walk into
            // the pack four times unafraid.
            let in_sight: Vec<&((i32, i32), f32, f32, String)> = beasts
                .iter()
                .filter(|((x, y), _, _, _)| {
                    (agent.state.position.0 - x)
                        .abs()
                        .max((agent.state.position.1 - y).abs())
                        <= Self::AS_FAR_AS_ANYBODY_SEES_A_BEAST
                })
                .collect();

            if in_sight.is_empty() {
                continue;
            }

            let against_me: f32 = in_sight
                .iter()
                .map(|(_, worth, menace, _)| worth * menace)
                .sum();

            // A thing worth twice what you are is frightening; a thing worth
            // half of you is not worth remembering.
            let odds = against_me / i_am_worth.max(0.01);
            let how_bad = (odds - 1.0).clamp(0.0, 1.0);

            if how_bad <= 0.0 {
                continue;
            }

            // What to call it is whatever the worst single one of them was.
            let called = in_sight
                .iter()
                .max_by(|(_, one, _, _), (_, other, _, _)| {
                    one.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(_, _, _, called)| called.clone())
                .unwrap_or_else(|| "something".to_string());

            for ((x, y), _, _, _) in in_sight {
                agent.exploration_knowledge.saw_danger(
                    crate::world::Position::new(*x, *y),
                    &called,
                    how_bad,
                    now,
                );
            }
        }
    }

    /// And whoever anybody laid eyes on, and where.
    ///
    /// Everything social in the model reads live positions, which is to say
    /// every agent knows where every other agent is standing at all times.
    /// This is what somebody would actually know.
    fn who_everybody_saw(&mut self) {
        if self.current_tick % Self::HOW_OFTEN_ANYBODY_LOOKS_ROUND != 0 {
            return;
        }

        let now = self.current_tick;
        let standing: Vec<(uuid::Uuid, (i32, i32))> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.id, (agent.state.position.0, agent.state.position.1)))
            .collect();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            for (who, (x, y)) in standing.iter() {
                if *who == agent.id {
                    continue;
                }

                let paces = (agent.state.position.0 - x)
                    .abs()
                    .max((agent.state.position.1 - y).abs());

                if paces <= Self::AS_FAR_AS_ANYBODY_SEES_A_PERSON {
                    agent.exploration_knowledge.saw_somebody(
                        *who,
                        crate::world::Position::new(*x, *y),
                        now,
                    );
                }
            }
        }
    }

    /// How often anybody stops and takes in what is round them.
    ///
    /// Every few ticks rather than every one. Nothing in a settlement changes
    /// fast enough to want it more often, and it is a walk over everybody
    /// against everything.
    const HOW_OFTEN_ANYBODY_LOOKS_ROUND: u32 = 5;

    /// How far off a beast is worth noticing.
    const AS_FAR_AS_ANYBODY_SEES_A_BEAST: i32 = 8;

    /// And a person, who is smaller and quieter than a bear.
    const AS_FAR_AS_ANYBODY_SEES_A_PERSON: i32 = 6;

    /// A lump of clay left too near the fire.
    ///
    /// "An agent 'cooks' some clay which causes it to harden into stoneware,
    /// which unlocks that technology." Nobody intends this. Somebody is
    /// sitting at a fire with clay in their pack because they picked it up on
    /// the way past a riverbank, and a lump of it ends up in the embers, and
    /// in the morning it is not clay any more.
    ///
    /// The same shape as `who_saw_that_dry` and for the same reason: a people
    /// at this stage does not reason its way to firing clay, it notices that
    /// firing has happened. What it costs is one lump of clay; what it buys
    /// is the first material this people can make that keeps something else.
    /// What the person being asked could actually explain about this thing.
    ///
    /// They have to know it themselves - a man who has never dried anything
    /// cannot tell you how - and what passes is the name of the discovery
    /// rather than the thing. `None` where there is nothing to be said: most
    /// of what anybody carries is obvious, and nobody explains a stick.
    fn what_asking_about_would_teach(&self, them: usize, what: &str) -> Option<String> {
        use crate::agents::Agent;

        let telling = &self.population.agents[them];

        // A meal that has been somewhere - dried, smoked - where what is worth
        // knowing is where it was rather than how it was made
        if let Some(item) = telling.inventory.get_item(what) {
            if let Some(discovery) = Agent::what_asking_about_this_meal_would_teach(item) {
                if telling.what_i_found_out().contains(discovery) {
                    return Some(discovery.to_string());
                }
            }
        }

        let made = Agent::what_asking_about_this_would_teach(what)?;

        telling.what_i_found_out().contains(&made).then_some(made)
    }

    /// Somebody near enough to ask, and a thing of theirs worth asking about.
    ///
    /// Worth asking about means: they are carrying it, this one has never seen
    /// how it is done, and they can actually explain it. Nobody asks after a
    /// stick.
    ///
    /// This is only ever reached under Curiosity, which is to say only when
    /// nothing worse is pressing - a man does not stop to ask after somebody's
    /// supper while his own children are hungry.
    fn somebody_to_ask_about_something(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, String)> {
        for (index, other) in self.population.agents.iter().enumerate() {
            if !other.state.is_alive || other.id == agent.id {
                continue;
            }

            let apart = (other.state.position.0 - agent_position.0)
                .abs()
                .max((other.state.position.1 - agent_position.1).abs());

            if apart > Self::NEAR_ENOUGH_TO_ASK {
                continue;
            }

            for (item_id, _) in other.inventory.get_all_items().iter() {
                let Some(teaches) = self.what_asking_about_would_teach(index, item_id) else {
                    continue;
                };

                if agent.what_i_found_out().contains(&teaches) {
                    continue;
                }

                return Some((other.id, item_id.clone()));
            }
        }

        None
    }

    /// How near somebody has to be to be asked.
    const NEAR_ENOUGH_TO_ASK: i32 = 2;

    /// Clay left lying at a lit fire is not clay in the morning.
    ///
    /// The ember accident that already existed is somebody *carrying* clay
    /// while they sit at a fire, and it happens to them rather than being
    /// done. This is the deliberate version, and it is what makes "what
    /// happens if I put clay in the fire" a question anybody can actually put
    /// - the answer arrives a few days later at the place it was left, like
    /// every other question of that kind.
    fn what_the_fire_hardened(&mut self) {
        let now = self.current_tick;
        let mut hardened: Vec<crate::world::Position> = Vec::new();

        for which in 0..self.world.dropped.len() {
            let left = &self.world.dropped[which];

            if left.item.item_id != crate::agents::Agent::THE_ONE_MATERIAL_A_FIRE_CHANGES {
                continue;
            }

            if now.saturating_sub(left.since) < Self::HOW_LONG_THE_FIRE_TAKES_TO_HARDEN_IT {
                continue;
            }

            let where_it_is = left.where_it_is;
            let at_a_fire = self
                .nearest_fire_from(
                    (where_it_is.x, where_it_is.y, 0),
                    Self::WITHIN_REACH_OF_THE_HEARTH,
                    true,
                )
                .is_some();

            if !at_a_fire {
                continue;
            }

            let how_many = self.world.dropped[which].item.quantity;
            self.world.dropped[which].item = crate::agents::InventoryItem::new_container(
                "stoneware".to_string(),
                how_many,
                crate::environment::making::WHAT_A_FIRED_POT_HOLDS,
            );

            hardened.push(where_it_is);
        }

        // And whoever is near enough to see it saw it.
        for where_it_was in hardened {
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive {
                    continue;
                }

                let paces = (agent.state.position.0 - where_it_was.x)
                    .abs()
                    .max((agent.state.position.1 - where_it_was.y).abs());

                if paces <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    agent.found_out_how_to(Self::THAT_FIRE_HARDENS_CLAY);
                }
            }
        }
    }

    /// How long a lump has to sit in the embers before it comes out hard.
    ///
    /// A day. Long enough that it is something the fire did rather than
    /// something that happened, short enough that somebody who left it there
    /// on purpose is still about to see it.
    const HOW_LONG_THE_FIRE_TAKES_TO_HARDEN_IT: u32 =
        crate::environment::seasons::TICKS_PER_DAY;

    fn what_the_embers_did(&mut self) {
        use rand::Rng;

        if self.current_tick % Self::HOW_OFTEN_THE_EMBERS_ARE_ASKED != 0 {
            return;
        }

        let mut rng = crate::core::dice::roll();
        let mut hardened: Vec<crate::world::Position> = Vec::new();

        for index in 0..self.population.agents.len() {
            {
                let agent = &self.population.agents[index];
                if !agent.state.is_alive || agent.how_many_i_have("clay") == 0 {
                    continue;
                }
            }

            let stood = self.population.agents[index].state.position;
            if self
                .nearest_fire_from(stood, Self::WITHIN_REACH_OF_THE_HEARTH, true)
                .is_none()
            {
                continue;
            }

            if !rng.gen_bool(Self::HOW_OFTEN_A_LUMP_FINDS_THE_EMBERS) {
                continue;
            }

            let agent = &mut self.population.agents[index];
            agent.inventory.remove_item("clay", 1);
            agent.inventory.add_item(crate::agents::InventoryItem::new_container(
                "stoneware".to_string(),
                1,
                crate::environment::making::WHAT_A_FIRED_POT_HOLDS,
            ));

            hardened.push(crate::world::Position::new(stood.0, stood.1));
        }

        // And whoever was sitting round the same fire saw it happen.
        for where_it_was in hardened {
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive {
                    continue;
                }

                let paces = (agent.state.position.0 - where_it_was.x)
                    .abs()
                    .max((agent.state.position.1 - where_it_was.y).abs());

                if paces > Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    continue;
                }

                if agent.found_out_how_to(Self::THAT_FIRE_HARDENS_CLAY) {
                    debug!("Agent {} saw clay come out of a fire hard", agent.id);
                }
                agent.lessons.record_particular("fire:claypot", true);
            }
        }
    }

    /// How often anybody's fire is asked about.
    const HOW_OFTEN_THE_EMBERS_ARE_ASKED: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// And how often a day at a fire with clay in the pack costs a lump of it.
    ///
    /// Rare. This is meant to happen once or twice in a settlement's life and
    /// then never matter again, because after it has happened once somebody
    /// knows and can do it on purpose.
    const HOW_OFTEN_A_LUMP_FINDS_THE_EMBERS: f64 = 0.02;

    /// What a lump of clay coming out of a fire hard teaches.
    ///
    /// The same name the working that does it deliberately makes, so that
    /// having seen it is the same thing as knowing how - see
    /// `making::FIRE_A_POT`.
    pub const THAT_FIRE_HARDENS_CLAY: &'static str = "stoneware";

    /// What an agent has to have seen before it will deliberately lay food
    /// out to dry.
    pub const THAT_LAYING_IT_OUT_KEEPS_IT: &'static str =
        crate::agents::Agent::THAT_LAYING_IT_OUT_KEEPS_IT;

    /// What the beasts make of us.
    ///
    /// The simplified other half of `feel_about_what_stands_in_the_way`. An
    /// animal has two drives worth the name - eat, and do not be eaten - and
    /// this is the second: run from what you cannot beat, turn on what you
    /// can. `AnimalState::Fleeing` and `AnimalState::Attacking` have been in
    /// the model since the model had animals and nothing had ever set either
    /// of them, so a deer stood placidly in a field while somebody walked up
    /// to it with a spear.
    ///
    /// Temper decides how kindly the odds get read, and a Passive thing never
    /// stands its ground however the arithmetic comes out - a rabbit that
    /// fights a wolf is not a rabbit.
    fn what_the_beasts_make_of_us(&mut self) {
        use crate::environment::fauna::AnimalState;

        // Everybody who might be a threat to something, and what they are
        // worth in a fight
        let people: Vec<((i32, i32), f32, uuid::Uuid)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| {
                let armed = agent
                    .what_i_have_to_work_with(crate::agents::SkillType::MeleeCombat)
                    .is_some();

                let worth = Self::WHAT_A_PERSON_IS_WORTH_TO_A_BEAST
                    * if armed { Self::WHAT_A_SPEAR_ADDS } else { 1.0 };

                (
                    (agent.state.position.0, agent.state.position.1),
                    worth,
                    agent.id,
                )
            })
            .collect();

        // And the beasts, which are a threat to each other
        let beasts: Vec<((i32, i32), f32, uuid::Uuid, bool)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive())
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                let worth = Self::what_a_beast_is_worth_in_a_fight(
                    animal.current_health,
                    species.health,
                    species.attack_damage,
                );
                let hunts = species.behavior.how_much_it_menaces_you() >= 1.0;
                Some((animal.position, worth, animal.id, hunts))
            })
            .collect();

        let mut made_up_their_minds: Vec<(uuid::Uuid, AnimalState)> = Vec::new();

        for animal in self.world.animals.get_all().iter() {
            if !animal.is_alive() || !animal.is_wild() {
                continue;
            }

            let Some(species) = self.world.animals.get_species(&animal.species_id) else {
                continue;
            };

            let mine = Self::what_a_beast_is_worth_in_a_fight(
                animal.current_health,
                species.health,
                species.attack_damage,
            );

            // The worst thing within sight of it: a person, or something
            // bigger than itself that eats meat
            let from_people = people
                .iter()
                .map(|(at, worth, who)| (*at, *worth, *who))
                .filter(|(at, _, _)| Self::within(*at, animal.position, Self::AS_FAR_AS_A_BEAST_LOOKS));

            let from_beasts = beasts
                .iter()
                .filter(|(_, _, who, hunts)| *hunts && *who != animal.id)
                .map(|(at, worth, who, _)| (*at, *worth, *who))
                .filter(|(at, _, _)| Self::within(*at, animal.position, Self::AS_FAR_AS_A_BEAST_LOOKS));

            let worst = from_people
                .chain(from_beasts)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let Some((where_it_is, coming, who)) = worst else {
                continue;
            };

            // Temper reads the odds. A rabbit never fights.
            let nerve = species.behavior.how_readily_it_stands_its_ground();
            let stands = nerve > 0.0 && mine * nerve >= coming * Self::WHAT_IT_TAKES_TO_TURN_AND_FACE;

            made_up_their_minds.push((
                animal.id,
                if stands {
                    AnimalState::Attacking { target_id: who }
                } else {
                    AnimalState::Fleeing {
                        from_position: where_it_is,
                    }
                },
            ));
        }

        for (which, made_up) in made_up_their_minds {
            if let Some(animal) = self
                .world
                .animals
                .get_all_mut()
                .iter_mut()
                .find(|animal| animal.id == which)
            {
                animal.state = made_up;
                animal.state_timer = Self::HOW_LONG_A_BEAST_KEEPS_ITS_NERVE;
            }
        }
    }

    /// And then they do something about it.
    ///
    /// Fleeing puts ground between the animal and whatever it saw. Standing
    /// its ground keeps it where it is - what happens next is whoever came at
    /// it getting bitten, which the hunt already handles.
    fn the_beasts_act_on_it(&mut self) {
        use crate::environment::fauna::AnimalState;

        let width = self.world.grid.width as i32;
        let height = self.world.grid.height as i32;

        let mut bolted: Vec<(uuid::Uuid, (i32, i32))> = Vec::new();

        for animal in self.world.animals.get_all().iter() {
            let AnimalState::Fleeing { from_position } = animal.state else {
                continue;
            };

            if !animal.is_alive() {
                continue;
            }

            let dx = animal.position.0 - from_position.0;
            let dy = animal.position.1 - from_position.1;
            let span = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0);

            let bolt = Self::HOW_FAR_A_FRIGHTENED_BEAST_GETS as f32;
            let landed = (
                (animal.position.0 as f32 + dx as f32 / span * bolt) as i32,
                (animal.position.1 as f32 + dy as f32 / span * bolt) as i32,
            );

            let landed = (
                landed.0.clamp(0, width - 1),
                landed.1.clamp(0, height - 1),
            );

            if self.is_passable_tile(landed.0, landed.1) {
                bolted.push((animal.id, landed));
            }
        }

        for (which, to) in bolted {
            if let Some(animal) = self
                .world
                .animals
                .get_all_mut()
                .iter_mut()
                .find(|animal| animal.id == which)
            {
                animal.position = to;
                animal.use_stamina(Self::WHAT_BOLTING_COSTS_A_BEAST);
            }
        }
    }

    /// What a beast is worth in a fight, on the same scale everything else is
    /// reckoned on: how sound it is, and what it can do with that.
    fn what_a_beast_is_worth_in_a_fight(health: f32, full: f32, damage: f32) -> f32 {
        let condition = (health / full.max(1.0)).clamp(0.0, 1.0);
        condition * (damage / 20.0).clamp(0.1, 2.0)
    }

    fn within(one: (i32, i32), other: (i32, i32), paces: i32) -> bool {
        (one.0 - other.0).abs().max((one.1 - other.1).abs()) <= paces
    }

    /// What a person on their own is worth to something with teeth.
    ///
    /// About a wolf. People are slow, soft and have no claws, and what makes
    /// them dangerous is what is in their hands.
    const WHAT_A_PERSON_IS_WORTH_TO_A_BEAST: f32 = 0.6;

    /// And what something in those hands adds.
    const WHAT_A_SPEAR_ADDS: f32 = 2.2;

    /// How much better than the thing coming at it an animal has to be before
    /// it turns and faces it.
    ///
    /// Above one, because running is the safe answer and a wild thing that
    /// gets this wrong does not get to be wrong twice.
    const WHAT_IT_TAKES_TO_TURN_AND_FACE: f32 = 1.1;

    /// How far a beast looks about itself.
    const AS_FAR_AS_A_BEAST_LOOKS: i32 = 7;

    /// How far a frightened animal gets in one turn. Further than a person:
    /// a deer outruns anything on two legs.
    const HOW_FAR_A_FRIGHTENED_BEAST_GETS: i32 = 6;

    /// What that costs it.
    const WHAT_BOLTING_COSTS_A_BEAST: f32 = 3.0;

    /// How long an animal goes on being frightened before it settles again.
    const HOW_LONG_A_BEAST_KEEPS_ITS_NERVE: u32 = 3;

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

    /// Whether asking to gather this, here, could come to anything at all.
    ///
    /// "No food sources nearby" was **ten thousand refused turns a world** and
    /// "inventory full" another five thousand - between them more than half of
    /// everything a settlement ever got refused. Both come from the same
    /// place: several of the paths that produce a `Gather` cannot see the
    /// world at all. `generate_action_for_drive` is a static table that maps
    /// Sustenance to "gather food" and Industry to "gather generic" with no
    /// notion of whether there is any food or any wood within a day's walk.
    ///
    /// So the question gets asked once, here, on the way past - and a drive
    /// that cannot be answered stands aside and lets the next one have the
    /// turn, which is the same doctrine `how_this_agent_answers` already runs
    /// on.
    fn could_this_gather_come_to_anything(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        named: &str,
    ) -> bool {
        use crate::world::{Position, ResourceType};

        let Some(wanted) = Self::what_a_gather_asks_for(named) else {
            // Not a word this world knows. The executor will refuse it, and
            // there is no sense spending the turn finding that out.
            return false;
        };

        // Water is drunk rather than carried off, and a full waterskin is an
        // answer to thirst on ground with no stream on it. Both of the checks
        // below would be wrong about it.
        if wanted == ResourceType::Water {
            return true;
        }

        // A pack with no room in it. Five thousand refused turns a world were
        // somebody asking for another armful with their arms already full.
        if agent.inventory.weight_capacity_remaining() < Self::AS_MUCH_AS_ONE_TRIP_WEIGHS {
            return false;
        }

        let here = Position::new(agent_position.0, agent_position.1);
        let now = self.current_tick;
        let after_anything_edible = wanted == ResourceType::Food;

        self.world.resources.iter().any(|resource| {
            if resource.amount == 0 {
                return false;
            }
            if here.distance_to(&resource.position) > Self::FORAGE_RADIUS {
                return false;
            }
            // Ground this one has already stripped and has no reason to think
            // has grown back
            if agent
                .exploration_knowledge
                .is_it_picked_out(resource.position, now)
            {
                return false;
            }

            resource.resource_type == wanted
                || (after_anything_edible
                    && Self::edible_item_for(resource.resource_type).is_some())
        })
    }

    /// What one trip out brings back, as near as makes no difference. Below
    /// this much room in the pack there is no point setting off.
    const AS_MUCH_AS_ONE_TRIP_WEIGHS: f32 = 1.0;

    /// Something worth taking while this one is standing here anyway.
    ///
    /// The whole of "make the trip pay". Three things have to be true and each
    /// is doing work. It has to be **underfoot or a pace away**, because the
    /// premise is that the walk has already been paid for - a thing nine paces
    /// off is a trip, not a top-up. It has to be something that **keeps**, so
    /// that a load put by is still a load in a fortnight; there is no sense
    /// carrying home a fortnight of berries. And this one has to hold **less
    /// than a working stock** of it already, or every agent in the world spends
    /// its life at a woodpile.
    ///
    /// Salt is the case that shows why it matters: a salt flat is a long walk
    /// and salt keeps for ever, so taking one lot is throwing away the walk.
    fn what_i_should_take_while_i_am_here(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<String> {
        use crate::world::Position;

        let here = Position::new(agent_position.0, agent_position.1);
        let now = self.current_tick;

        self.world
            .resources
            .iter()
            .filter(|resource| resource.amount > 0)
            .filter(|resource| here.distance_to(&resource.position) <= Self::ALREADY_STANDING_HERE)
            .filter(|resource| {
                !agent
                    .exploration_knowledge
                    .is_it_picked_out(resource.position, now)
            })
            .filter_map(|resource| Self::gathered_as(resource.resource_type))
            .filter(|named| Self::does_it_keep(named))
            .find(|named| {
                agent.how_many_i_have(named) < Self::WHAT_A_WORKING_STOCK_IS
                    && self.could_this_gather_come_to_anything(agent, agent_position, named)
            })
            .map(|named| named.to_string())
    }

    /// Whether a load of this is still a load in a fortnight.
    ///
    /// Deliberately not "is it food": greens and roots go off and stone does
    /// not, and the question here is about keeping rather than about eating.
    fn does_it_keep(named: &str) -> bool {
        matches!(
            named,
            "wood" | "stone" | "salt" | "clay" | "flax" | "cotton" | "iron"
        )
    }

    /// How far off still counts as being here. A thing underfoot or a pace
    /// away costs nothing to pick up; a thing nine paces off is a trip.
    const ALREADY_STANDING_HERE: u32 = 1;

    /// How much of a keeping thing is enough to stop topping up.
    ///
    /// Enough wood for several fires rather than one, and enough salt to see a
    /// winter's meat put by. Above this an agent has better things to do than
    /// stand at a woodpile.
    const WHAT_A_WORKING_STOCK_IS: u32 = 12;

    /// What a request to gather names, in the world's own terms.
    ///
    /// The only vocabulary `Gather` has. It lived inside the executor, where
    /// nothing that had to *decide* whether a gather was worth asking for
    /// could read it - and a vocabulary in one place that a second place has
    /// to guess at is how clay came to spawn in every world for a year with
    /// nobody able to pick any of it up.
    fn what_a_gather_asks_for(named: &str) -> Option<crate::world::ResourceType> {
        use crate::world::ResourceType;

        match named {
            "wood" => Some(ResourceType::Wood),
            "stone" => Some(ResourceType::Stone),
            "iron" => Some(ResourceType::Iron),
            "food" => Some(ResourceType::Food),
            // Wild grain stands in the world and there was no way to ask for
            // it by name: a request for grain fell through to "unknown
            // resource type" and failed. It came back only as an edible
            // substitute for a request for food, which is how a people that
            // had never handled grain came to have none of it to sow.
            "grain" => Some(ResourceType::Grain),
            // What there is to eat before anything has ripened
            "greens" => Some(ResourceType::Greens),
            "roots" => Some(ResourceType::Roots),
            "water" => Some(ResourceType::Water),
            // Clothing materials. Flax and cotton grow in patches an agent can
            // walk to; hides and wool come off animals, so they are here for
            // when an agent has somewhere to get them rather than because the
            // ground offers any.
            "flax" => Some(ResourceType::Flax),
            "cotton" => Some(ResourceType::Cotton),
            // Clay has been spawning on every riverbank and every marsh in
            // every world since the project began and no agent could ever pick
            // any of it up: it was missing from this list.
            "clay" => Some(ResourceType::Clay),
            "salt" => Some(ResourceType::Salt),
            "hides" => Some(ResourceType::Hides),
            "wool" => Some(ResourceType::Wool),
            "generic" => Some(ResourceType::Wood), // Default to wood for generic
            _ => None,
        }
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

    /// What each agent makes of its own provisions against the winter coming.
    ///
    /// "Do I have enough supplies to survive the day? The week? The month? The
    /// winter?" Four horizons, each less frightening to fail than the last,
    /// and the answer comes out as one number that becomes the Preparedness
    /// drive - which already knows how to put food by. See
    /// `agents::provision`.
    ///
    /// What an agent can reach is its own pack and the camp's pits. A pit is
    /// the settlement's, not any one person's, so everybody counts the same
    /// store and everybody is easier for it being full: that is the whole
    /// reason a people digs one.
    fn reckon_what_is_put_by(&mut self) {
        use crate::agents::provision::{WhatIsPutBy, UNITS_IN_ONE_STORED_ITEM};

        let season = self.world.climate.current_season();
        let day_of_year = (self.current_tick
            / crate::environment::seasons::TICKS_PER_DAY)
            % crate::environment::seasons::DAYS_PER_YEAR;

        let in_the_ground: f32 = self
            .world
            .pits
            .iter()
            .map(|pit| pit.how_much_is_in_it() as f32)
            .sum::<f32>()
            * UNITS_IN_ONE_STORED_ITEM;

        let mouths = self
            .population
            .agents
            .iter()
            .filter(|a| a.state.is_alive)
            .count()
            .max(1) as f32;
        let each_ones_share = in_the_ground / mouths;

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            agent.state.winters_seen.another_day(season, day_of_year);

            let in_hand = crate::agents::storage_integration::count_food_in_inventory(
                &agent.inventory,
            ) as f32
                * UNITS_IN_ONE_STORED_ITEM;

            // And what is still coming out of the body's own stores counts:
            // somebody who has just eaten is not short of supper.
            let in_the_body = agent.state.physiology.in_the_stomach()
                + agent.state.physiology.in_the_gut();

            let reckoning = WhatIsPutBy::reckon(
                in_hand + each_ones_share + in_the_body,
                agent.state.physiology.what_i_burn_in_a_day,
                agent.state.winters_seen.how_long_a_winter_lasts(),
                day_of_year,
            );

            if let Some(drive) = agent.drives.get_mut(DriveType::Preparedness) {
                drive.value = reckoning.stress();
            }
            agent.state.what_the_larder_says = Some(reckoning);
        }
    }


    /// Process environmental damage for all agents
    pub fn process_environmental_damage(&mut self) {
        use crate::agents::body::{BodyPartType, InjuryType, CripplingType};
        use crate::world::{Position, TerrainType};
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

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

                agent.state.lose_health(fall_damage * 0.15, "a fall");
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

    /// How much of a child's belly one feed is.
    ///
    /// A third, so a fed child takes three or four in a day and stops when it
    /// is full. Filling the whole stomach every time somebody was standing
    /// nearby put several times what the child burned into it, and its mother
    /// paid for all of it.
    const A_FEED_IS_THIS_MUCH_OF_A_BELLY: f32 = 3.0;

    /// Process nursing for infants
    fn process_nursing(&mut self) {
        use crate::agents::childcare::{MAX_CAREGIVER_DISTANCE, NURSING_ENERGY_GAIN};
        use crate::agents::LifeStage;

        let current_tick = self.current_tick;

        // Collect caregiver positions for distance checks
        let caregiver_positions: std::collections::BTreeMap<uuid::Uuid, (i32, i32, i32)> =
            self.population.agents.iter()
                .filter(|a| a.state.is_alive)
                .map(|a| (a.id, a.state.position))
                .collect();

        // What each nursing costs the woman doing it, applied after the loop
        let mut what_the_milk_cost: Vec<(uuid::Uuid, f32)> = Vec::new();

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

                    // And milk, which is the point of the exercise.
                    //
                    // This did the line above and nothing else: five points of
                    // `energy`, a field that #70 measured as never scarce and
                    // that fires in `is_starving` exactly nought times in
                    // twenty thousand adult-turns. The stomach, the gut and the
                    // reserve - which are what starvation is reckoned on - never
                    // saw a drop. So a nursed infant was fed nothing, and had to
                    // forage for itself from the hour it was born, needing three
                    // and a half meals a day against a grown woman's two and a
                    // half because its stomach is a quarter the size and it
                    // burns more for its size. Every child born in every world
                    // ever measured died as an infant. See ISSUES #78.
                    //
                    // Fed on demand, a mouthful at a time: a child that has
                    // room takes one and a full one does not, so it regulates
                    // itself the way a fed child does rather than being filled
                    // every two hours whether it wants it or not.
                    let a_mouthful =
                        agent.state.physiology.stomach_capacity / Self::A_FEED_IS_THIS_MUCH_OF_A_BELLY;
                    let taken = if agent.state.physiology.room_for_another_mouthful() {
                        agent
                            .state
                            .physiology
                            .eat(a_mouthful, crate::agents::physiology::WHAT_MILK_IS_WORTH)
                    } else {
                        0.0
                    };
                    if taken > 0.0 {
                        agent.state.took_a_meal(current_tick, 0.0);
                        what_the_milk_cost.push((
                            nursing.primary_caregiver,
                            taken * crate::agents::physiology::WHAT_MILK_IS_WORTH,
                        ));
                    }

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
                        agent.state.lose_health(penalty, "illness");
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

        // And what feeding them cost the women who did it.
        //
        // Milk is not free. A nursing mother eats for two, and if there is not
        // enough for two she is the one who goes short - which is why a hungry
        // season shows up in the next generation rather than only in this one.
        for (who, units) in what_the_milk_cost {
            if let Some(mother) = self
                .population
                .agents
                .iter_mut()
                .find(|a| a.id == who && a.state.is_alive)
            {
                mother.state.physiology.reserve =
                    (mother.state.physiology.reserve - units).max(0.0);
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

        let mut rng = crate::core::dice::roll();
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
        let climate_data: std::collections::BTreeMap<_, _> = agent_data.iter()
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

        // Serialize to MessagePack (supports complex BTreeMap keys like Position)
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
            actions_taken: std::collections::BTreeMap::new(),
            actions_failed: std::collections::BTreeMap::new(),
            actions_failed_because: std::collections::BTreeMap::new(),
            what_a_threat_came_to: std::collections::BTreeMap::new(),
            what_anybody_found_out: std::collections::BTreeMap::new(),
            what_anybody_was_told: std::collections::BTreeMap::new(),
            what_would_not_fit_in_the_pack: 0,
            food_items_into_packs: 0,
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
        let nearby_believers: std::collections::BTreeMap<_, _> = agent_data.iter()
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





