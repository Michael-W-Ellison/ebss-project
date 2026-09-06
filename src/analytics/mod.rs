// src/analytics/mod.rs
//! Analytics, data logging, and emergence detection.

use crate::world::World;
use crate::agents::Population;
use crate::world::spatial_planning::{PlacementCriteria, PlacementStrategy};

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

/// How one agent stands towards another.
pub mod between_us;

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
use crate::world::FoodDatabase;
use crate::environment::Action;
use crate::visualization::AsciiRenderer;
use log::{debug, info};
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
    /// And how much of that went straight back where it came from.
    ///
    /// Two different things were being added into one counter. A carcass too
    /// big to carry is *left on the ground*, where it rots and is gone - that
    /// is the waste #165 is about. An armful of berries that will not go in
    /// the pack is `put_it_back` on the bush, and **nothing is lost at all**:
    /// the patch is exactly as it was, and the same berries are counted again
    /// on the next trip, and the trip after that.
    ///
    /// Pooling the two made a settlement look as though it was throwing away
    /// ten items of food for every one it kept. See ISSUES #118.
    pub what_went_back_on_the_bush: u64,
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
    /// How many extra minutes have been spent on the fast clock.
    ///
    /// A minute of somebody's life that they got to decide about because
    /// something was on them. Counted because the whole mechanism is invisible
    /// otherwise: it fires only for people in danger, and a run where it never
    /// fires looks exactly like a run without it. See
    /// `Simulation::everybody_takes_a_turn`.
    pub minutes_spent_in_danger: u64,
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
            minutes_spent_in_danger: 0,
            what_anybody_found_out: std::collections::BTreeMap::new(),
            what_anybody_was_told: std::collections::BTreeMap::new(),
            what_would_not_fit_in_the_pack: 0,
            what_went_back_on_the_bush: 0,
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
            // Herbalism is what a remedy is read against, and Herbalism is
            // what gathering teaches.
            // Watching somebody dose a sick man is watching a problem being
            // worked at, which is the nearest thing the observation tiers
            // have to doctoring.
            Action::Treat { .. } => Some(ActionType::ProblemSolving),
            Action::Fish => Some(ActionType::Mining), // Taking something off the world
            Action::SetSnare | Action::CheckSnares => Some(ActionType::Mining),
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

            // Water belongs here as much as anything does, and it was the one
            // name the two lists did not share. Merging them without it took
            // `Action::Gather { "water" }` away from everybody: across twelve
            // worlds the person-samples fell from 5,327 to 286 and every
            // person still alive and ill had Thirst pressing hardest on them.
            // A settlement that cannot ask for water is dead in a fortnight.
            ResourceType::Water => "water",

            ResourceType::Clay => "clay",
            ResourceType::Salt => "salt",
            ResourceType::Sand => "sand",
            ResourceType::Coal => "coal",
            ResourceType::Flax => "flax",
            ResourceType::Cotton => "cotton",
            ResourceType::Grain => "grain",
            ResourceType::Greens => "greens",
            ResourceType::Roots => "roots",
            ResourceType::Nuts => "nuts",
            ResourceType::Legumes => "legumes",
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
    /// Put as much of a stack into the pack as will go, and say how much went.
    ///
    /// `Inventory::add_item` is all or nothing: offered twenty items when
    /// there is room for ten it takes **none of them**. That is right for a
    /// tool, which is one thing or no thing, and wrong for an armful of
    /// berries, which is twenty separate berries.
    ///
    /// Butchering had already worked this out - it computed what fits and took
    /// that much - and the two paths that bring food home never learned it.
    /// Measured, 87,667 items of food went back on the bush in eight
    /// world-years while the agents putting them back had, on average, twelve
    /// and a half kilos of room: twenty-five items' worth of space, refusing
    /// an armful of fourteen because it was offered as one lump. See #118.
    pub(in crate::analytics) fn take_what_fits(
        &mut self,
        agent_index: usize,
        item: &crate::agents::InventoryItem,
    ) -> u32 {
        let each = item.weight_per_unit * item.how_much_lighter_it_is();
        let room = self.population.agents[agent_index]
            .inventory
            .weight_capacity_remaining();

        let fits = if each > 0.0 {
            ((room / each).floor() as u32).min(item.quantity)
        } else {
            item.quantity
        };

        if fits == 0 {
            return 0;
        }

        let mut taking = item.clone();
        taking.quantity = fits;

        // The slot limit can still refuse a kind of thing this pack has no
        // room for at all, and that is not a partial answer.
        if self.population.agents[agent_index]
            .inventory
            .add_item(taking)
        {
            fits
        } else {
            0
        }
    }

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
            // What fits goes in; a slot the pack has no room for at all comes
            // back as nought, and then the whole lot stays where it fell.
            let fits = self.take_what_fits(agent_index, &item);

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

    /// Whether this one could bring that down with what it is carrying.
    ///
    /// The decision layer and the executor were asking two different questions
    /// and getting two different answers. `worth_hunting` asked
    /// `equipment.get_weapon()`, which is the equipment slot; the executor
    /// asked `what_i_have_to_work_with(Hunting)`, which is the pack. So an
    /// agent decided to hunt, walked to the animal, threw the turn away and
    /// was refused - **589 hunts in 599 over six worlds**, every one of them
    /// "no spear in hand". Two spellings of one question is how this project
    /// has lost measurements before, so there is now one.
    pub(in crate::analytics) fn could_bring_it_down(
        agent: &crate::agents::Agent,
        species: &crate::environment::AnimalSpecies,
    ) -> bool {
        species.health <= Self::AS_BIG_AS_A_STONE_WILL_KILL
            || agent
                .what_i_have_to_work_with(crate::agents::skills::SkillType::Hunting)
                .is_some()
    }

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
        //
        // Unless it is food, in which case the pack is not the reason to stay
        // where you are. A full pack is full of something, and food is worth
        // more than what it is full of: the stone goes on the grass and the
        // crop goes in - see `set_down_what_is_worth_less_than_food`. Where
        // there is nothing anybody would set down, what will not go in the
        // pack goes in the mouth - see the tail of `gathering`. Either way
        // the trip is worth making, and this gate used to stop the decision
        // before the executor could do either, so a starving man with a full
        // pack did not walk to the bush at all.
        let food_is_the_point = wanted == ResourceType::Food;

        if !food_is_the_point
            && agent.inventory.weight_capacity_remaining() < Self::AS_MUCH_AS_ONE_TRIP_WEIGHS
        {
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

        // "generic" is not a thing in the world. It is what the drive ladder
        // says when Industry wins and it cannot name what it wants, and it
        // comes out as a trip for timber.
        if named == "generic" {
            return Some(ResourceType::Wood);
        }

        // Everything else is the inverse of `gathered_as`, walked rather than
        // written out again.
        //
        // This was a second hand-written list, and `gathered_as` claims in its
        // own docstring to be "the same vocabulary `Gather` answers to, kept
        // here so that the decision and the executor cannot drift apart". They
        // drifted apart three times. Its own comments record two of them:
        // grain, which "fell through to unknown resource type and failed" so
        // that a people who had never handled grain had none to sow; and clay,
        // which "has been spawning on every riverbank and every marsh in every
        // world since the project began and no agent could ever pick any of it
        // up: it was missing from this list."
        //
        // The third was **herbs**, and it cost the whole of the treatment
        // machinery. `Action::Gather { resource_type: "herbs" }` is what an
        // ill agent with an empty pack is sent to do, Rest wins the tick for
        // 194 of 426 ill person-samples, and every one of those turns came
        // back "Unknown resource type: herbs". Measured across twelve worlds
        // and 5,327 person-samples: **not one person ever held a remedy**,
        // with seven thousand bearing herb patches on the maps and the nearest
        // one a median twelve paces away. See ISSUES_FOUND.md #163 and #166.
        //
        // A list cannot fail this way if there is only one of it.
        ResourceType::all()
            .into_iter()
            .find(|what| Self::gathered_as(*what) == Some(named))
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
            minutes_spent_in_danger: 0,
            what_anybody_found_out: std::collections::BTreeMap::new(),
            what_anybody_was_told: std::collections::BTreeMap::new(),
            what_would_not_fit_in_the_pack: 0,
            what_went_back_on_the_bush: 0,
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





