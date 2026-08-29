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

/// What happens whether or not anybody decides anything.
pub mod happening;

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
}


#[cfg(test)]
mod tests;





