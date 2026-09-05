// src/world/mod.rs
//! Complete world simulation system with terrain, resources, buildings, and spatial management.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use rand::Rng;

/// World size presets for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    /// Tiny world: 64x64x64 - for quick testing
    Tiny,
    /// Small world: 128x128x96 - for small simulations
    Small,
    /// Medium world: 256x256x128 - default balanced size
    Medium,
    /// Large world: 512x512x160 - for complex simulations
    Large,
    /// Huge world: 1024x1024x192 - for massive simulations
    Huge,
    /// Custom size
    Custom(i32, i32, i32),
}

impl WorldSize {
    /// Get the dimensions (width, depth, height) for this size
    pub fn dimensions(&self) -> (i32, i32, i32) {
        match self {
            WorldSize::Tiny => (64, 64, 64),
            WorldSize::Small => (128, 128, 96),
            WorldSize::Medium => (256, 256, 128),
            WorldSize::Large => (512, 512, 160),
            WorldSize::Huge => (1024, 1024, 192),
            WorldSize::Custom(w, d, h) => (*w, *d, *h),
        }
    }

    /// Validate that dimensions are reasonable
    pub fn is_valid(&self) -> bool {
        let (w, d, h) = self.dimensions();
        w > 0 && d > 0 && h > 0 && w <= 4096 && d <= 4096 && h <= 512
    }

    /// Get estimated memory usage in MB (rough estimate)
    pub fn estimated_memory_mb(&self) -> f32 {
        let (w, d, h) = self.dimensions();
        let blocks = w as f32 * d as f32 * h as f32;
        // Rough estimate: ~100 bytes per block on average (with sparse storage)
        blocks * 100.0 / (1024.0 * 1024.0)
    }
}

// Module declarations
pub mod terrain;
pub mod resources;
pub mod buildings;
pub mod inventory;
pub mod actions;
pub mod grid;
pub mod render;
pub mod production;
pub mod economy;
pub mod technology;
pub mod climate;
pub mod combat;
pub mod crafting;
pub mod spatial_planning;
pub mod zoning;
pub mod path_planning;
pub mod territory;
pub mod resource_spawning;
pub mod nutrition;
pub mod soil;

// Re-exports
pub use terrain::{Terrain, TerrainType, Tile, TileVisibility};
pub use soil::Soil;
pub use resources::{Bearing, Resource, ResourceType, ResourceNode};
pub use buildings::{Building, BuildingType, BuildingState};
pub use inventory::{Inventory, Item, ItemType};
pub use actions::{Action, ActionResult};
pub use grid::{Grid, Position};
pub use render::AsciiRenderer;
pub use production::{Recipe, Quality, ResourceRequirement, ProductionOutput};
pub use economy::{TradeOffer, Marketplace, MarketData, CompletedTrade, MarketStatistics};
pub use technology::{Technology, TechnologyTree, KnownTechnologies, TechEra, DiscoveryEvent};
pub use climate::{ClimateManager, terrain_to_biome};
pub use resource_spawning::{
    NaturalisticResourceConfig, NaturalisticSpawner, TerrainResourceMapper,
    AnimalResourceConfig, AnimalResourceMapper, TerrainGenerator,
};
pub use nutrition::{
    NutrientType, NutritionalContent, PreparationState, FoodData,
    FoodTemplate, FoodDatabase, NutritionalState, EatResult,
};

use crate::environment::{
    AnimalManager, AnimalSize, AnimalSpawnConfig, HeatSourceRegistry, PlantManager, TrophicRole,
};

/// How one stage of a country coming up went - see
/// [`World::let_the_country_come_up`].
#[derive(Debug, Clone)]
pub struct HowATierCameUp {
    /// What was admitted at this stage.
    pub tiers: Vec<TrophicRole>,
    /// And what band of grazers went with it.
    pub grazers: (AnimalSize, AnimalSize),
    /// How much of it was actually put down.
    pub put_down: usize,
    /// And what the whole country came to by the end of the stage, which is
    /// the number that says whether it held.
    pub standing_after: usize,
}

/// Status of a heat source for smelting
#[derive(Debug, Clone)]
pub struct HeatSourceStatus {
    pub is_lit: bool,
    pub current_temperature: f32,
    pub fuel_remaining: f32,
    pub contents: Vec<(String, u32, u32, f32)>, // (material_id, quantity, heating_time, current_temp)
}

/// Complete world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub grid: Grid,
    pub resources: Vec<ResourceNode>,
    pub buildings: Vec<Building>,
    pub storehouse_inventory: Inventory,
    pub marketplace: Marketplace,
    #[serde(skip)]
    pub tech_tree: TechnologyTree, // Global technology tree (not serialized, recreated)
    pub climate: ClimateManager,
    pub heat_sources: HeatSourceRegistry,
    pub animals: AnimalManager,
    pub plants: PlantManager,
    #[serde(skip)]
    pub combat_manager: combat::CombatManager, // Combat system (not serialized)
    #[serde(skip)]
    pub crafting_manager: crafting::CraftingManager, // Crafting system (not serialized)
    pub tick: u32,
    pub config: WorldConfig, // Store configuration for spatial planning
    pub resource_nodes: std::collections::BTreeMap<String, Vec<(i32, i32, i32)>>, // Resource locations by type (as tuples)
    pub zone_manager: zoning::ZoneManager, // Spatial zoning for settlement planning
    pub road_network: path_planning::RoadNetwork, // Road and path network
    pub territory_manager: territory::TerritoryManager, // Territory claiming and ownership

    /// Everything lying about on the ground where somebody left it.
    ///
    /// Before this a thing was either in somebody's pack or it did not exist.
    /// Nothing could be put down and picked up again, and when a person died
    /// everything they had carried went out of the world with them - so a
    /// people that spent a season making axes had nothing to show for it the
    /// morning after the man who made them drowned.
    #[serde(default)]
    pub dropped: Vec<Dropped>,

    /// Snares set in the ground, and what has gone into them.
    ///
    /// The only way anybody reaches the lower tiers of the food web now that
    /// those are a population rather than records - see
    /// [`crate::environment::SmallLife`]. You cannot stalk a number.
    #[serde(default)]
    pub snares: Vec<crate::environment::small_life::Snare>,

    /// Pits dug in the ground, and what is keeping in them.
    ///
    /// A settlement had nowhere to put anything. The storehouse is a single
    /// global bag of counts with no position, nothing in it ever spoils, and
    /// what an agent could put by explicitly excluded food - so nothing that
    /// anybody eats was ever stored anywhere by anybody. Measured at ten
    /// thousand ticks, not one of sixty-five living agents was carrying so
    /// much as a meal. A hole in the cold ground with the earth back over it
    /// is what a people this far along actually has, and it is the difference
    /// between a settlement that eats what it finds today and one that eats
    /// in February.
    #[serde(default)]
    pub pits: Vec<Pit>,

    /// Where a seam was worked out.
    ///
    /// A mined-out mineral node is genuinely gone and is taken off the map,
    /// which left no way to tell worked ground from ground that never held
    /// anything. That mattered in one place and mattered a lot: a man who
    /// honestly reported a clay seam he passed yesterday was recorded as a
    /// **liar** the moment somebody else mined it out and walked over the
    /// spot, because the spot was then indistinguishable from the invented
    /// one a liar names. A settlement of twenty-five people who would not
    /// dream of lying produced dozens of proven liars. See ISSUES_FOUND #48.
    ///
    /// Ground that has been worked looks worked. This is the world
    /// remembering that.
    #[serde(default)]
    pub where_it_was_worked_out: std::collections::BTreeSet<Position>,

    /// What has dried out in the sun since anybody last looked, and where.
    ///
    /// The world does the drying; whoever is standing near enough to see it
    /// happen is what turns it into something a person knows. Drained every
    /// tick by the simulation - see `Simulation::who_saw_that_dry`.
    #[serde(default, skip)]
    pub what_dried_in_the_sun: Vec<(Position, String)>,
    /// Food that went off before anybody ate it, where it lay and in the
    /// ground.
    ///
    /// The wasted half of whatever was spent getting it. If half the meat rots
    /// before it is eaten then half the hunt was wasted, and until this was
    /// counted every preservation change in this project had to be judged on
    /// how much was *in* the store rather than on how much of what was got was
    /// ever any use to anybody.
    #[serde(default)]
    pub food_that_rotted_where_it_lay: u64,
    #[serde(default)]
    pub food_that_rotted_in_the_ground: u64,

    /// Which sorts of strange plant feed a person in this world, by kind.
    ///
    /// Drawn once when the country is made and never shown to anybody living
    /// in it. `true` at index `k` means a strange plant of kind `k` is supper;
    /// `false` means it is not, and finding out which costs somebody their
    /// health or their life. See `ResourceType::StrangePlant`.
    #[serde(default)]
    pub what_the_strange_plants_are: Vec<bool>,
}

/// Something lying on the ground where somebody left it.
///
/// A thing put down, dropped out of a full pack, thrown and not recovered, or
/// left where its owner died. It is the same item it was in the pack: a worn
/// axe on the ground is still a worn axe when the next person picks it up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dropped {
    pub item: crate::agents::InventoryItem,
    pub where_it_is: Position,
    /// The tick it was left, which is what the weather counts from
    pub since: u32,
    /// How much extra ageing the weather has done to it, over and above the
    /// passing of time.
    ///
    /// Kept as its own count rather than by winding the food's `created_tick`
    /// backwards, which was the first attempt and silently did nothing: a
    /// thing dropped at tick zero has a `created_tick` of zero, and
    /// `saturating_sub` on a `u32` at zero is a very quiet no-op.
    #[serde(default)]
    pub weathered: u32,
    /// And how much sun it has had, which is the other thing the sky does to
    /// a thing lying in it.
    #[serde(default)]
    pub dried_in_the_sun: u32,
}

/// A hole in the ground with food in it.
///
/// Covered or open. Covered is what does the work: earth over the top keeps
/// the sun and the air off, and what is in there ages at a quarter the rate
/// it would in somebody's pack. Open, it is a hole with food in it, which is
/// to say it is much the same as leaving it on the grass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pit {
    pub where_it_is: Position,
    pub holds: Vec<crate::agents::InventoryItem>,
    pub covered: bool,
    /// The tick it was dug, which is what the ground counts from
    pub dug: u32,
}

impl Pit {
    /// How much a pit holds, in the units a pack is weighed in.
    ///
    /// Generous against a pack, which is the point of digging one - a person
    /// carries fifty and a hole in the ground takes six times that.
    pub const WHAT_A_PIT_TAKES: u32 = 300;

    /// What is in there to eat, counted. The lining is not stores.
    pub fn how_much_is_in_it(&self) -> u32 {
        self.holds
            .iter()
            .filter(|item| Self::is_it_food(item))
            .map(|item| item.quantity)
            .sum()
    }

    /// Whether there is room for more.
    pub fn has_room(&self) -> bool {
        self.how_much_is_in_it() < Self::WHAT_A_PIT_TAKES
    }

    /// Whether there is anything in it worth walking to.
    ///
    /// The vessel it is lined with does not count, and neither does an uncut
    /// haunch or a stack that has gone over. A hungry man who walks to a
    /// store and comes back with the bowl has not eaten, and no more has one
    /// who comes back with something he cannot put in his mouth.
    pub fn has_food(&self) -> bool {
        self.holds.iter().any(|item| Self::is_it_a_meal(item))
    }

    /// How many ticks apart two lots of the same thing were laid down.
    ///
    /// Anything without a clock counts as of an age with anything else, so
    /// materials still stack the way they always did.
    fn how_far_apart_in_age(
        one: &crate::agents::InventoryItem,
        other: &crate::agents::InventoryItem,
    ) -> u32 {
        match (&one.food_data, &other.food_data) {
            (Some(mine), Some(theirs)) => mine.created_tick.abs_diff(theirs.created_tick),
            _ => 0,
        }
    }

    /// How close in age two lots have to be before they go in as one.
    ///
    /// A few days. Loads put by in the same week are the same load; loads put
    /// by a season apart are not, and pretending otherwise throws the older
    /// one's clock over the newer.
    const CLOSE_ENOUGH_IN_AGE_TO_JOIN: u32 =
        crate::environment::seasons::TICKS_PER_DAY * 4;

    /// And how many separate lots of one thing a hole keeps before it starts
    /// joining them up. A store is a hole in the ground, not a ledger.
    const AS_MANY_SEPARATE_LOTS_AS_A_PIT_KEEPS: usize = 6;

    /// Whether a thing in the pit is something to eat rather than the vessel
    /// it is kept in.
    ///
    /// This was the blocklist "anything that is not a bowl or a basket", which
    /// counted a spear buried by mistake as a winter's eating. It asks the
    /// same question as everything else now - see `ItemType::is_it_food`.
    fn is_it_food(item: &crate::agents::InventoryItem) -> bool {
        item.quantity > 0 && crate::world::nutrition::is_this_food(&item.item_id)
    }

    /// What is in there to eat, by name.
    ///
    /// A meal, not merely a thing that is not a basket. This used to answer
    /// with whatever was nearest the top, and a pit holding an uncut haunch
    /// or a stack that had gone over would offer it over and over to somebody
    /// who could not eat it: they picked it up, were no better fed for it,
    /// and picked it up again. One settlement in sixteen starved to death
    /// standing on its own larder doing exactly that. See ISSUES_FOUND #43.
    pub fn something_to_eat(&self) -> Option<&str> {
        self.holds
            .iter()
            .find(|item| Self::is_it_a_meal(item))
            .map(|item| item.item_id.as_str())
    }

    /// Whether a thing in the pit is something somebody could actually make a
    /// meal of, on the same terms `Agent::how_many_meals_i_have` counts by.
    fn is_it_a_meal(item: &crate::agents::InventoryItem) -> bool {
        if !Self::is_it_food(item) {
            return false;
        }
        if !crate::world::nutrition::Piece::of(&item.item_id).can_it_be_eaten() {
            return false;
        }
        match item.food_data {
            Some(ref food) => !food.is_spoiled() && !food.is_harmful(),
            None => false,
        }
    }

    /// Whether somebody put a vessel in it before they filled it.
    ///
    /// A bowl or a basket between the food and the damp is worth as much as
    /// the hole is: see `World::what_is_buried_keeps`.
    pub fn is_lined(&self) -> bool {
        self.holds
            .iter()
            .any(|item| matches!(item.item_id.as_str(), "bowl" | "basket"))
    }

    /// One tick in this many is the only one that tells on what is buried
    /// here.
    ///
    /// Bare earth is twice as long as a pack, which is what cool and dark are
    /// worth on their own. Earth with a vessel between the food and the
    /// ground is four times: what actually gets at buried food is the ground
    /// itself, and a bowl or a basket in the way of it is the difference
    /// between a store and a hole full of rot.
    ///
    /// The same number that ages what is in the pit and that answers how long
    /// a thing would keep if it went in - see `how_long_this_would_keep`. Two
    /// spellings of that would drift, and the second would be the one the
    /// decision to bury was made on.
    pub fn how_much_slower_things_age(&self) -> u32 {
        if self.is_lined() {
            Self::EARTH_WITH_SOMETHING_BETWEEN
        } else {
            Self::BARE_EARTH
        }
    }

    const BARE_EARTH: u32 = 2;
    const EARTH_WITH_SOMETHING_BETWEEN: u32 = 4;

    /// How many days this would still be food for, if it went in here now.
    ///
    /// What is left of its own clock, at the pace this hole lets it run. The
    /// question nobody was asking: **a settlement buried 512 units a year and
    /// ate four of them**, because raw greens keep six days in bare earth and
    /// the land gives nothing for seventy-five. See ISSUES_FOUND.md #124.
    ///
    /// `None` for a thing with no clock on it at all, which keeps for ever.
    pub fn how_long_this_would_keep(
        &self,
        item: &crate::agents::InventoryItem,
        now: u32,
    ) -> Option<f32> {
        use crate::environment::seasons::TICKS_PER_DAY;

        let food = item.food_data.as_ref()?;
        let _ = now;

        // What is left of its own clock - see `FoodData::how_long_this_has_left`
        // - at the pace this hole lets it run.
        let left = food.how_long_this_has_left();

        Some(left * self.how_much_slower_things_age() as f32 / TICKS_PER_DAY as f32)
    }

    /// Take some of a thing out.
    pub fn take_out(&mut self, what: &str, how_many: u32) -> u32 {
        let Some(held) = self
            .holds
            .iter_mut()
            .find(|item| item.item_id == what && item.quantity > 0)
        else {
            return 0;
        };

        let taken = how_many.min(held.quantity);
        held.quantity -= taken;
        self.holds.retain(|item| item.quantity > 0);
        taken
    }

    /// Put something in.
    pub fn put_in(&mut self, item: crate::agents::InventoryItem) {
        // A pit is not a pack, and this is where the difference tells.
        //
        // A pack has one slot per name and has to merge; `holds` is a list and
        // does not. What a person actually does with a store is put this
        // autumn's load in beside last autumn's, not tip it in on top - and
        // making a pit merge everything under one name meant a fresh load
        // inherited the clock of whatever had been down there since the last
        // harvest, went off almost at once, and the store collapsed: measured,
        // a settlement held **130 units** against 937 and rotted nearly twice
        // as much in the ground. See ISSUES_FOUND #65.
        //
        // So a load goes in beside what is already there unless there is
        // something of the same age to join, and a pit that has collected too
        // many separate lots joins the nearest rather than growing without
        // bound.
        let same_name: Vec<usize> = self
            .holds
            .iter()
            .enumerate()
            .filter(|(_, held)| held.item_id == item.item_id)
            .map(|(at, _)| at)
            .collect();

        let of_an_age = same_name.iter().copied().min_by_key(|at| {
            Self::how_far_apart_in_age(&self.holds[*at], &item)
        });

        if let Some(at) = of_an_age {
            let apart = Self::how_far_apart_in_age(&self.holds[at], &item);

            if apart <= Self::CLOSE_ENOUGH_IN_AGE_TO_JOIN
                || same_name.len() >= Self::AS_MANY_SEPARATE_LOTS_AS_A_PIT_KEEPS
            {
                self.holds[at].absorb(item);
                return;
            }
        }

        self.holds.push(item);
    }
}

/// World configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub size: (usize, usize), // Width, Height (no Z for simplicity)
    pub initial_resources: ResourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    // Basic resources
    pub wood_nodes: usize,
    pub stone_nodes: usize,
    pub iron_nodes: usize,
    pub food_nodes: usize,
    pub water_sources: usize, // Rivers, wells, springs

    // Mineral resources (for technology progression)
    pub clay_clusters: usize,
    pub sand_clusters: usize,
    pub coal_clusters: usize,

    // Agricultural resources
    pub grain_patches: usize,
    pub flax_patches: usize,
    pub herb_patches: usize,
    pub cotton_patches: usize,

    // Gatherable resources
    pub honey_locations: usize,
    pub fish_areas: usize,

    // Whether to use naturalistic spawning (terrain-appropriate, clustered)
    pub use_naturalistic_spawning: bool,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            // Basic resources
            wood_nodes: 20,
            stone_nodes: 15,
            iron_nodes: 8,
            food_nodes: 25,
            water_sources: 15, // Rivers, wells, springs - critical for survival

            // Minerals
            clay_clusters: 4,
            sand_clusters: 3,
            coal_clusters: 3,

            // Agricultural
            grain_patches: 5,
            flax_patches: 3,
            herb_patches: 6,
            cotton_patches: 2,

            // Gatherable
            honey_locations: 4,
            // A river is a river along its length, not in five places. At five
            // areas the generator put six or seven reaches of fish on three
            // hundred and seventy-odd water tiles - too thin for anybody to
            // build a living on, and two of every three nodes were lost anyway
            // to cluster offsets landing on dry ground.
            fish_areas: 14,

            // Use naturalistic spawning by default
            use_naturalistic_spawning: true,
        }
    }
}

impl ResourceConfig {
    /// The map these counts were written for.
    ///
    /// Every number in this config is a number for a map of this many tiles.
    /// What makes a country liveable is how much wood there is within a walk
    /// of you, not how much wood there is in it altogether, so a map sixteen
    /// times the size gets sixteen times as many of everything. Without this
    /// a hundred square kilometres came out with the same three hundred and
    /// sixty-odd nodes a quarter of a square kilometre had, spread over four
    /// hundred times the ground, and a man could walk all day between bushes.
    pub const THE_MAP_THESE_WERE_WRITTEN_FOR: usize = 50 * 50;

    /// This config as it applies to a map of the given size.
    ///
    /// Rounds up rather than down, so that a small map still gets one of each
    /// thing rather than none: a map with no water on it is not a small map,
    /// it is a dead one.
    pub fn spread_over(&self, tiles: usize) -> Self {
        let over = |count: usize| -> usize {
            if count == 0 {
                return 0;
            }
            let scaled =
                (count * tiles).div_ceil(Self::THE_MAP_THESE_WERE_WRITTEN_FOR);
            scaled.max(1)
        };

        Self {
            wood_nodes: over(self.wood_nodes),
            stone_nodes: over(self.stone_nodes),
            iron_nodes: over(self.iron_nodes),
            food_nodes: over(self.food_nodes),
            water_sources: over(self.water_sources),
            clay_clusters: over(self.clay_clusters),
            sand_clusters: over(self.sand_clusters),
            coal_clusters: over(self.coal_clusters),
            grain_patches: over(self.grain_patches),
            flax_patches: over(self.flax_patches),
            herb_patches: over(self.herb_patches),
            cotton_patches: over(self.cotton_patches),
            honey_locations: over(self.honey_locations),
            fish_areas: over(self.fish_areas),
            use_naturalistic_spawning: self.use_naturalistic_spawning,
        }
    }
}

impl Default for WorldConfig {
    /// A corner of a country: a quarter of a square kilometre.
    ///
    /// Small on purpose. This is the map a test builds, and a test that has to
    /// tick a hundred square kilometres to find out whether one man ate is a
    /// test nobody runs. For the map an ecology actually needs, see
    /// [`WorldConfig::big_enough_for_an_ecology`].
    fn default() -> Self {
        Self {
            size: (50, 50),
            initial_resources: ResourceConfig::default(),
        }
    }
}

impl WorldConfig {
    /// A map big enough for the ecology on it to stand up on its own.
    ///
    /// A hundred square kilometres, which is [`Grid::METRES_PER_CELL`] into a
    /// thousand cells each way. That is the size at which a wolf pack, the
    /// deer it lives on and the grass the deer live on can each hold a
    /// population without any of them being one bad winter from gone - a
    /// quarter of a square kilometre cannot, however carefully it is tuned.
    ///
    /// A square metre a cell was the other way of getting there and does not
    /// fit: a hundred million tiles at forty bytes apiece is four gigabytes
    /// before anything happens in them. Ten metres is also the unit the rest
    /// of the model already thinks in - a forage radius of 25 cells is a
    /// quarter-kilometre walk, which is about right for a morning's gathering
    /// and nonsense as 25 metres.
    pub fn big_enough_for_an_ecology() -> Self {
        let side = Grid::HOW_MANY_CELLS_ACROSS_A_COUNTRY;
        Self {
            size: (side, side),
            initial_resources: ResourceConfig::default(),
        }
    }

    /// Set world size
    pub fn with_size(mut self, width: usize, height: usize) -> Self {
        self.size = (width, height);
        self
    }

    /// Set resource configuration
    pub fn with_resources(mut self, resources: ResourceConfig) -> Self {
        self.initial_resources = resources;
        self
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        let (width, height) = self.size;

        // Check for zero dimensions
        if width == 0 {
            return Err("World width must be greater than 0".to_string());
        }
        if height == 0 {
            return Err("World height must be greater than 0".to_string());
        }

        // Check minimum size (must be large enough for agents to move)
        const MIN_SIZE: usize = 10;
        if width < MIN_SIZE || height < MIN_SIZE {
            return Err(format!("World dimensions must be at least {}x{} (minimum playable size)", MIN_SIZE, MIN_SIZE));
        }

        // Check maximum size (prevent memory issues)
        const MAX_SIZE: usize = 2000;
        if width > MAX_SIZE || height > MAX_SIZE {
            return Err(format!("World dimensions must not exceed {}x{} (maximum supported size)", MAX_SIZE, MAX_SIZE));
        }

        // Validate resource counts don't exceed world tiles
        let total_tiles = width * height;
        let total_resources = self.initial_resources.wood_nodes
            + self.initial_resources.stone_nodes
            + self.initial_resources.iron_nodes
            + self.initial_resources.food_nodes;

        if total_resources > total_tiles {
            return Err(format!(
                "Total resource nodes ({}) exceeds world tiles ({})",
                total_resources, total_tiles
            ));
        }

        Ok(())
    }
}

impl World {
    /// Somebody put this down, or dropped it, or died holding it.
    pub fn somebody_left_this(&mut self, item: crate::agents::InventoryItem, where_it_is: Position, tick: u32) {
        if item.quantity == 0 {
            return;
        }

        self.dropped.push(Dropped {
            item,
            where_it_is,
            since: tick,
            weathered: 0,
            dried_in_the_sun: 0,
        });
    }

    /// What is lying on a given tile, newest first.
    pub fn what_is_lying_at(&self, where_it_is: &Position) -> Vec<&Dropped> {
        let mut here: Vec<&Dropped> = self
            .dropped
            .iter()
            .filter(|left| left.where_it_is == *where_it_is)
            .collect();

        here.reverse();
        here
    }

    /// Take a named thing off the ground here, if it is there.
    pub fn take_off_the_ground(
        &mut self,
        where_it_is: &Position,
        called: &str,
    ) -> Option<crate::agents::InventoryItem> {
        let which = self
            .dropped
            .iter()
            .rposition(|left| left.where_it_is == *where_it_is && left.item.item_id == called)?;

        Some(self.dropped.remove(which).item)
    }

    /// How long a thing lies where it was left before the weather has it.
    ///
    /// A season and a half for anything: long enough that somebody walking the
    /// same country again finds it, short enough that a world does not silt up
    /// with everything anybody ever put down.
    ///
    /// This read 432, which *was* a season and a half - back when a season was
    /// twenty-four days. A season became ninety days and this did not follow,
    /// so it had quietly meant thirty-six days for a while, and food, which
    /// gets a quarter of it, nine. Derived from the season now, so the comment
    /// and the number cannot part company again. The same shape as
    /// `patterns::STILL_WORTH_THE_WALK`, which read 288 against a comment
    /// saying "a season".
    pub const HOW_LONG_A_THING_LIES_THERE: u32 =
        crate::environment::seasons::DAYS_PER_SEASON * 3 / 2 * crate::environment::seasons::TICKS_PER_DAY;

    /// What the weather does to what is lying about.
    ///
    /// Food goes first and goes into the ground, which is where food goes.
    /// Everything else weathers away in its own time.
    /// Cold ground with the earth back over it keeps food.
    ///
    /// `FoodData::update_freshness` works off elapsed time since the thing was
    /// made, so the way to make a pit keep something is to hold that clock
    /// back: on three ticks in every four the buried food's `created_tick` is
    /// pushed forward with the world, so a season underground costs it what a
    /// fortnight in a pack would. An open pit is a hole with food in it and
    /// keeps nothing at all.
    ///
    /// What has gone off in there rots away like anything else.
    fn what_is_buried_keeps(&mut self) {
        let now = self.tick;
        let mut buried_and_lost = 0u64;

        for pit in self.pits.iter_mut() {
            // Bare earth is cool and dark and keeps a thing rather better
            // than a pack does. Earth with a vessel in it keeps it better
            // again: what actually gets at buried food is the ground itself -
            // damp, and everything that lives in it - and a bowl or a basket
            // between the two is the difference between a store and a hole
            // full of rot.
            let ageing = now % pit.how_much_slower_things_age() == 0;

            for item in pit.holds.iter_mut() {
                if let Some(food) = item.food_data.as_mut() {
                    if pit.covered && !ageing {
                        food.created_tick = food.created_tick.saturating_add(1);
                    }
                    food.update_freshness(now);
                }
            }

            pit.holds.retain(|item| {
                let keeps = item.quantity > 0
                    && item
                        .food_data
                        .as_ref()
                        .is_none_or(|food| food.freshness > 0.0);

                if !keeps && item.food_data.is_some() {
                    buried_and_lost += item.quantity as u64;
                }

                keeps
            });
        }

        self.food_that_rotted_in_the_ground =
            self.food_that_rotted_in_the_ground.saturating_add(buried_and_lost);
    }

    /// Whether a thing laid out will dry through before it turns.
    ///
    /// Strips cut off a carcass will. A berry will - it is mostly skin. A
    /// whole fish will not: the outside dries and the inside goes on being a
    /// fish, and by evening the whole of it is carrion. That is the
    /// difference a people here can actually see, and it is the only route
    /// they have to anything that keeps.
    pub fn will_this_dry(what: &str) -> bool {
        // Flesh dries once somebody has cut it, and not before. A whole beast
        // laid in the sun goes off; a joint of it takes most of a week and a
        // strip of it two days - see `nutrition::Piece`.
        if crate::world::nutrition::Piece::is_it_flesh(what) {
            return crate::world::nutrition::Piece::of(what) != crate::world::nutrition::Piece::Whole;
        }

        what.ends_with("strips")
            || what.ends_with("portions")
            || matches!(what, "food" | "berries" | "greens" | "grain" | "roots")
    }

    /// How much faster a thing goes off lying in the open than it does in a
    /// pack.
    const WHAT_THE_WEATHER_ADDS: u32 = 3;

    /// And in the shade, out of both sun and rain.
    ///
    /// Still worse than a pack, because a pack is carried indoors and out of
    /// the way of everything that eats carrion.
    const WHAT_SHADE_ADDS: u32 = 2;

    /// How much sun a thing has to sit in before it is dried through.
    ///
    /// Two days of clear weather, which on this calendar is a real wait: rain
    /// in the middle of it does not undo the drying but it does stop it, so a
    /// wet fortnight is a fortnight nothing gets preserved.
    /// Superseded by `nutrition::Piece::how_long_it_takes_to_dry`, which asks
    /// the question this constant could not: how big is the piece.
    #[allow(dead_code)]
    const HOW_LONG_DRYING_TAKES: u32 = 2 * crate::environment::seasons::TICKS_PER_DAY;

    /// How often the weathering pass runs, which is what the extra ageing is
    /// reckoned against.
    ///
    /// The fourth spelling of one cadence, and the plainest: it sat next to a
    /// `% 10` in the same file that it had to agree with, and said so in its
    /// own doc comment. Derived now, so the ageing and the running of it
    /// cannot drift apart.
    const HOW_OFTEN_THE_WEATHER_GETS_AT_IT: u32 = crate::environment::seasons::ONCE_A_DAY;

    /// What share of what a plant is carrying comes off it each pass, once
    /// the season it bears in has passed.
    ///
    /// The plant pass runs every ten ticks, so at a quarter a hedgerow is
    /// four fifths bare within five days of the season turning and all but
    /// empty inside a fortnight. That is what fruit does.
    ///
    /// A first cut used a twentieth and left 472 units of berries hanging on
    /// bushes in midwinter - most of a season's crop still on the branch in
    /// the snow, which is not a lean season, it is autumn with worse weather.
    const WHAT_FALLS_OFF_A_TICK: f32 = 0.25;

    /// The pit dug on this tile, if there is one.
    pub fn pit_at(&self, where_it_is: Position) -> Option<&Pit> {
        self.pits.iter().find(|pit| pit.where_it_is == where_it_is)
    }

    /// The same, to put something in or take something out of.
    pub fn pit_at_mut(&mut self, where_it_is: Position) -> Option<&mut Pit> {
        self.pits
            .iter_mut()
            .find(|pit| pit.where_it_is == where_it_is)
    }

    /// The nearest pit with anything in it, and how far off it is.
    pub fn nearest_full_pit(&self, from: Position, within: u32) -> Option<(&Pit, u32)> {
        self.pits
            .iter()
            .filter(|pit| pit.has_food())
            .map(|pit| {
                let paces = from.distance_to(&pit.where_it_is);
                (pit, paces)
            })
            .filter(|(_, paces)| *paces <= within)
            .min_by_key(|(_, paces)| *paces)
    }

    /// How much food is in the ground about here.
    ///
    /// The whole larder within walking distance, not one hole. A person can
    /// see the pits round their own camp, and how full a store is, is a
    /// question about the store and not about whichever hole they happen to
    /// be standing over.
    pub fn how_much_is_in_the_ground_near(&self, from: Position, within: u32) -> u32 {
        self.pits
            .iter()
            .filter(|pit| from.distance_to(&pit.where_it_is) <= within)
            .map(|pit| pit.how_much_is_in_it())
            .sum()
    }

    /// The nearest pit with room in it.
    pub fn nearest_pit_with_room(&self, from: Position, within: u32) -> Option<(&Pit, u32)> {
        self.pits
            .iter()
            .filter(|pit| pit.has_room())
            .map(|pit| {
                let paces = from.distance_to(&pit.where_it_is);
                (pit, paces)
            })
            .filter(|(_, paces)| *paces <= within)
            .min_by_key(|(_, paces)| *paces)
    }

    fn what_is_lying_about_weathers(&mut self) {
        let now = self.tick;
        let mut back_to_the_ground: Vec<(Position, f32)> = Vec::new();
        let mut dried: Vec<(Position, String)> = Vec::new();

        // What is lying out in the weather goes off faster than what is in
        // somebody's pack. Sun, rain and flies get at it, and until now they
        // did not: a thing picked up off the grass a fortnight after it was
        // dropped was exactly as fresh as the day it fell.
        // What the sky is doing to whatever is lying in it.
        //
        // Rain rots anything. Sun dries what is thin enough to dry - strips
        // cut off a carcass, berries - and rots what is not: a whole fish
        // left out in the sun is carrion by evening, and the same fish cut
        // down and laid out keeps for a season. That difference is the whole
        // of what a people here can learn about preserving anything, and it
        // is the world that teaches it rather than anything written down.
        // How hard it is coming down, rather than whether it is. A drizzle
        // and a thunderstorm were the same event: `WHAT_THE_WEATHER_ADDS` was
        // a constant and the intensity - which the weather has always
        // reported - was thrown away at the first comparison.
        let how_hard_it_rains = self.climate.weather.weather_type.precipitation_intensity();
        let sunny = matches!(
            self.climate.weather.weather_type,
            crate::environment::WeatherType::Clear
                | crate::environment::WeatherType::PartlyCloudy
        );

        // What is lying under a roof, which the sky cannot get at either way.
        //
        // "Nothing yet distinguishes food under a roof from food in the
        // open": shade was a constant rather than a question about where the
        // thing was lying. It is a question now, and it cuts both ways - a
        // thing under a roof does not rot in the rain and does not dry in the
        // sun either.
        let under_a_roof: std::collections::BTreeSet<Position> = self
            .buildings
            .iter()
            .map(|building| building.position)
            .collect();

        for left in self.dropped.iter_mut() {
            if left.item.food_data.is_none() {
                continue;
            }

            let sheltered = under_a_roof.contains(&left.where_it_is);
            let thin_enough_to_dry = Self::will_this_dry(&left.item.item_id);
            let drying = sunny && thin_enough_to_dry && !sheltered;

            if drying {
                left.dried_in_the_sun += Self::HOW_OFTEN_THE_WEATHER_GETS_AT_IT;
            } else {
                // Rain gets at everything, and how much depends on how hard
                // it is coming down. Sun gets at anything too thick to dry
                // through before it turns, and at full strength - there is
                // nothing gentle about a hot afternoon and a whole carcass.
                let in_the_open = if sheltered {
                    // The sky gets at nothing under a roof. What is left is
                    // the shade case, which still costs something, because
                    // nothing keeps out of doors.
                    0.0
                } else if sunny && !thin_enough_to_dry {
                    1.0
                } else {
                    how_hard_it_rains
                };

                // Shade is the floor and the open sky is the ceiling; a
                // drizzle sits between them rather than at the top of the
                // range with a thunderstorm.
                let shade = Self::WHAT_SHADE_ADDS as f32;
                let worst = Self::WHAT_THE_WEATHER_ADDS as f32;
                let adds = shade + (worst - shade) * in_the_open.clamp(0.0, 1.0);

                left.weathered +=
                    ((adds - 1.0) * Self::HOW_OFTEN_THE_WEATHER_GETS_AT_IT as f32) as u32;
            }

            let weathered = left.weathered;

            // How long it has to lie there is a question about how small it
            // was cut. This was one flat number for everything, so a joint
            // dried as fast as a strip and there was no reason on earth to
            // cut a thing into strips.
            let long_enough = left.dried_in_the_sun
                >= crate::world::nutrition::Piece::of(&left.item.item_id)
                    .how_long_it_takes_to_dry();

            if let Some(food) = left.item.food_data.as_mut() {
                if long_enough && food.preparation == crate::world::nutrition::PreparationState::Raw
                {
                    food.set_preparation(
                        crate::world::nutrition::PreparationState::Dried,
                        now,
                    );
                    dried.push((left.where_it_is, left.item.item_id.clone()));
                }

                food.update_freshness(now + weathered);
            }
        }

        self.what_dried_in_the_sun.extend(dried);

        let mut wasted = 0u64;

        self.dropped.retain(|left| {
            // Food that has gone off entirely is not food any more, whatever
            // the clock says about how long it has lain there
            if left
                .item
                .food_data
                .as_ref()
                .is_some_and(|food| food.freshness <= 0.0)
            {
                back_to_the_ground.push((left.where_it_is, left.item.quantity as f32 * 0.05));
                wasted += left.item.quantity as u64;
                return false;
            }

            let lain = now.saturating_sub(left.since);

            let gone = if left.item.food_data.is_some() {
                lain >= Self::HOW_LONG_A_THING_LIES_THERE / 4
            } else {
                lain >= Self::HOW_LONG_A_THING_LIES_THERE
            };

            if gone && left.item.food_data.is_some() {
                back_to_the_ground.push((left.where_it_is, left.item.quantity as f32 * 0.05));
                wasted += left.item.quantity as u64;
            }

            !gone
        });

        self.food_that_rotted_where_it_lay =
            self.food_that_rotted_where_it_lay.saturating_add(wasted);

        for (where_it_is, worth) in back_to_the_ground {
            if let Some(tile) = self.grid.get_tile_mut(&where_it_is) {
                tile.soil.add_leaf_litter(worth);
            }
        }
    }

    /// How many different unknown plants grow in a world.
    ///
    /// Few enough that a people can get through them in a few generations,
    /// and enough that getting through them costs somebody.
    pub const HOW_MANY_STRANGE_PLANTS: u8 = 4;

    /// Which of them turn out to be food, drawn fresh for each world.
    ///
    /// Always at least one of each, so no world is a world where curiosity is
    /// simply free and none is a world where it is simply fatal.
    fn draw_the_strange_plants() -> Vec<bool> {
        use rand::seq::SliceRandom;

        let how_many = Self::HOW_MANY_STRANGE_PLANTS as usize;
        let mut what_they_are: Vec<bool> = (0..how_many).map(|i| i % 2 == 0).collect();
        what_they_are.shuffle(&mut crate::core::dice::roll());
        what_they_are
    }

    /// Whether a strange plant of this kind feeds a person.
    ///
    /// The world knows. Nobody living in it does.
    pub fn does_this_one_feed_you(&self, kind: u8) -> bool {
        self.what_the_strange_plants_are
            .get(kind as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn new(config: WorldConfig) -> Self {
        Self::made_with(config, Some(AnimalSpawnConfig::default()))
    }

    /// A world, and what fauna it opens with.
    ///
    /// `None` means a country with its foliage and its assumed lower tiers
    /// and nothing standing on it, which is what
    /// [`World::let_the_country_come_up`] starts from.
    fn made_with(config: WorldConfig, fauna: Option<AnimalSpawnConfig>) -> Self {
        let mut grid = Grid::new(config.size.0, config.size.1);
        grid.generate_terrain();

        let mut world = Self {
            grid,
            resources: Vec::new(),
            buildings: Vec::new(),
            storehouse_inventory: Inventory::new(10000), // Large capacity
            marketplace: Marketplace::new(),
            tech_tree: TechnologyTree::new(),
            climate: ClimateManager::default(),
            heat_sources: HeatSourceRegistry::new(),
            // What a map will hold at the very outside. These are not the
            // carrying capacity - what a country will feed is a question for
            // the grass on it - they are the point past which the vectors
            // stop growing, and so they have to be a question about area
            // rather than a number somebody picked for a fifty by fifty map.
            animals: AnimalManager::new(Grid::at_the_very_outside(
                config.size.0 * config.size.1,
                Self::MOST_ANIMALS_A_SMALL_MAP_HOLDS,
            )),
            plants: PlantManager::new(Grid::at_the_very_outside(
                config.size.0 * config.size.1,
                Self::MOST_PLANTS_A_SMALL_MAP_HOLDS,
            )),
            combat_manager: combat::CombatManager::new(),
            crafting_manager: crafting::CraftingManager::new(),
            tick: 0,
            config: config.clone(),
            resource_nodes: std::collections::BTreeMap::new(),
            zone_manager: zoning::ZoneManager::new(),
            road_network: path_planning::RoadNetwork::new(),
            territory_manager: territory::TerritoryManager::new(),
            what_the_strange_plants_are: Self::draw_the_strange_plants(),
            dropped: Vec::new(),
            snares: Vec::new(),
            pits: Vec::new(),
            where_it_was_worked_out: std::collections::BTreeSet::new(),
            what_dried_in_the_sun: Vec::new(),
            food_that_rotted_where_it_lay: 0,
            food_that_rotted_in_the_ground: 0,
        };

        // The ground under the terrain that was just generated
        world.grid.settle_soil();


        // Place initial resources, as many of them as this much ground
        // should carry rather than as many as the config names - see
        // `ResourceConfig::spread_over`.
        let for_this_map = config
            .initial_resources
            .spread_over(config.size.0 * config.size.1);
        world.generate_resources(&for_this_map);

        // Build initial longhouse at center
        let center = (config.size.0 / 2, config.size.1 / 2);
        world.add_building(Building::new(
            BuildingType::Longhouse,
            Position::new(center.0 as i32, center.1 as i32),
        ));

        // Stock the country with what grows on it
        world.prime_the_springs();

        world.plants.spawn_naturalistic(&world.grid);

        // Spawn initial wildlife based on terrain
        if let Some(spawn_config) = fauna {
            world.animals.spawn_naturalistic(&world.grid, &spawn_config);
        } else {
            // A country with nothing on it still has to know how big it is,
            // which is otherwise `spawn_naturalistic`'s doing.
            world.animals.knows_how_big_the_world_is(&world.grid);
        }

        // And stock the lower tiers, which are a population rather than
        // records - see `SmallLife`. A country is not empty of rabbits on the
        // morning it is made, and until this ran a world had none until its
        // first tick: anything that asked what was living on a piece of
        // ground before then was told nothing was.
        world
            .animals
            .stock_the_small_life(&world.grid, world.climate.current_season());

        world
    }

    /// The order a country comes up in: what goes on the map at each stage,
    /// and what band of grazers goes with it.
    ///
    /// The chain from the bottom, each tier admitted onto ground that will
    /// already feed it. The small predators go on first because what they
    /// live on - the assumed grazers and rodents - is a population on every
    /// hunting ground from the morning the world is made, so they need
    /// nothing put down for them. The medium browsers and the middle
    /// predators go on together, then the large herbivores, and the wolves
    /// and the lions last, onto a country that has herds in it.
    /// The size band only bites on a stage that admits `PrimaryConsumer`;
    /// the other two carry the whole band and are decided by their tier list
    /// alone, which is clearer than an inverted range doing the work
    /// silently.
    pub const THE_ORDER_A_COUNTRY_COMES_UP_IN: [(&'static [TrophicRole], (AnimalSize, AnimalSize)); 4] = [
        // The small predators, onto the assumed layers and nothing else.
        (
            &[TrophicRole::SmallPredator],
            (AnimalSize::Tiny, AnimalSize::Huge),
        ),
        // The browsers up to a deer, and what lives off them and off the
        // small life.
        (
            &[TrophicRole::PrimaryConsumer, TrophicRole::MidPredator],
            (AnimalSize::Tiny, AnimalSize::Medium),
        ),
        // Then the cattle, the elk and the mammoth.
        (
            &[TrophicRole::PrimaryConsumer],
            (AnimalSize::Large, AnimalSize::Huge),
        ),
        // And the wolves and the lions, onto a country with herds in it.
        (
            &[TrophicRole::TopPredator],
            (AnimalSize::Tiny, AnimalSize::Huge),
        ),
    ];

    /// Let a country come up in the order a country comes up in, instead of
    /// arriving whole on one morning.
    ///
    /// The specification: "start with the foliage and let it spread out,
    /// colonizing the map. Once it is established, add the assumed small
    /// creatures and small predators until they get established. Then add
    /// the medium assumed creatures and predators and let them get
    /// established before introducing the large herbivores and eventually
    /// the large predators."
    ///
    /// **What this buys is a legible failure, and that is worth more here
    /// than the realism.** A world stocked all at once and left alone for two
    /// years tells you that its kestrels are gone; it does not tell you
    /// whether they starved because the layer under them was too thin,
    /// because they were put on ground that never suited them, or because
    /// something ate them first. A tier that arrives on its own, onto a
    /// country that is already standing still, fails visibly and alone.
    ///
    /// It also gives the model a definition of *settled*, which it has never
    /// had: each stage is admitted onto ground that held its numbers through
    /// the last one, and the report says which stages did.
    ///
    /// It is not a fix for a country that will not carry a tier. If the
    /// steady state cannot feed a kestrel, this reaches nought kestrels more
    /// slowly and more legibly, and the arithmetic of what a kestrel eats is
    /// still where the answer is.
    pub fn let_the_country_come_up(
        config: WorldConfig,
        days_a_tier_gets: usize,
    ) -> (Self, Vec<HowATierCameUp>) {
        // The foliage and the assumed layers, and nothing standing on them.
        let mut world = Self::made_with(config, None);
        world.let_it_stand(days_a_tier_gets);

        let mut how_it_went = Vec::new();
        for (tiers, grazers) in Self::THE_ORDER_A_COUNTRY_COMES_UP_IN {
            let before = world.animals.how_many_are_alive();
            world
                .animals
                .spawn_naturalistic(&world.grid, &AnimalSpawnConfig::only(tiers, grazers));
            let put_down = world.animals.how_many_are_alive().saturating_sub(before);

            world.let_it_stand(days_a_tier_gets);

            how_it_went.push(HowATierCameUp {
                tiers: tiers.to_vec(),
                grazers,
                put_down,
                standing_after: world.animals.how_many_are_alive(),
            });
        }

        (world, how_it_went)
    }

    /// Leave the country to itself for a while.
    fn let_it_stand(&mut self, days: usize) {
        for _ in 0..days * crate::environment::seasons::TICKS_PER_DAY as usize {
            self.tick();
        }
    }

    /// The most animals a fifty by fifty map will hold, scaled up from there.
    ///
    /// Room, not carrying capacity. A thousand animals on a quarter of a
    /// square kilometre is already far more than the ground would feed; what
    /// this stops is a vector growing without bound if something upstream
    /// goes wrong.
    const MOST_ANIMALS_A_SMALL_MAP_HOLDS: usize = 1000;

    /// The most plants a fifty by fifty map will hold, scaled up from there.
    ///
    /// A `Plant` here is the standing growth on its cell rather than one
    /// stem - a hundred square metres of hazel is one of these - so two per
    /// cell is generous.
    const MOST_PLANTS_A_SMALL_MAP_HOLDS: usize = 5000;

    /// How many patches of each unknown plant a world carries.
    const PATCHES_OF_EACH_STRANGE_PLANT: u32 = 4;

    /// How much stands in one of them.
    const WHAT_A_STRANGE_PATCH_CARRIES: u32 = 30;

    /// Put the unknown plants about the country.
    ///
    /// On open ground, like anything else that grows, and scattered rather than
    /// clustered: the point is that a people walking about its own country
    /// keeps coming across them, and has to decide each time whether today is
    /// the day somebody tries one.
    fn scatter_the_strange_plants(
        &mut self,
        today: u32,
        taken: &mut std::collections::BTreeSet<(i32, i32)>,
    ) {
        use rand::Rng;

        let mut rng = crate::core::dice::roll();
        let width = self.grid.width as i32;
        let height = self.grid.height as i32;

        for kind in 0..Self::HOW_MANY_STRANGE_PLANTS {
            let mut placed = 0;
            let mut tries = 0;

            while placed < Self::PATCHES_OF_EACH_STRANGE_PLANT && tries < 400 {
                tries += 1;

                let where_it_is = Position::new(
                    rng.gen_range(0..width),
                    rng.gen_range(0..height),
                );

                let will_grow = self
                    .grid
                    .get_tile(&where_it_is)
                    .map(|tile| tile.terrain.can_be_tilled())
                    .unwrap_or(false);

                if !will_grow {
                    continue;
                }

                // This had a fourth spelling of "is anything standing here"
                // and walked the whole resource list to answer it, four
                // hundred times per kind of plant. It asks the register the
                // other spawners use now.
                if taken.contains(&(where_it_is.x, where_it_is.y)) {
                    continue;
                }
                taken.insert((where_it_is.x, where_it_is.y));

                let mut patch = ResourceNode::of_kind(
                    ResourceType::StrangePlant,
                    where_it_is,
                    Self::WHAT_A_STRANGE_PATCH_CARRIES,
                    kind,
                );

                // Nobody knows what these do, including when they bear - but
                // they do bear, and out of season there is nothing on them.
                // This is the *third* spawner in this project and it had its
                // own vocabulary too: it does not go through
                // `what_this_ground_carries`, so the seeding fix reached the
                // hedgerows and not these. A test written for the fix is what
                // found it.
                if !ResourceType::StrangePlant.is_it_bearing(today) {
                    patch.amount = 0;
                }

                self.resources.push(patch);
                placed += 1;
            }
        }
    }

    fn generate_resources(&mut self, config: &ResourceConfig) {
        let mut rng = crate::core::dice::roll();

        // What is standing on the plants depends on the date the world opens,
        // not only on the ground - see `what_this_ground_carries`.
        let today = self.climate.calendar.day_of_year;

        // The ground already spoken for, asked once and carried through all
        // three spawners rather than re-derived per node - see
        // `World::what_ground_is_taken`.
        let mut taken = self.what_ground_is_taken();

        // Generate basic resources (legacy method for backward compatibility)
        self.generate_basic_resources(config, today, &mut rng, &mut taken);

        // Generate additional resources using naturalistic spawning
        if config.use_naturalistic_spawning {
            self.generate_naturalistic_resources(config, today, &mut taken);
        }

        // And the things nobody has tried
        self.scatter_the_strange_plants(today, &mut taken);

        // Update resource_nodes map for spatial queries
        self.update_resource_node_map();
    }

    /// Generate basic resources (wood, stone, iron, food)
    fn generate_basic_resources(
        &mut self,
        config: &ResourceConfig,
        today: u32,
        rng: &mut impl Rng,
        taken: &mut std::collections::BTreeSet<(i32, i32)>,
    ) {
        // Generate wood nodes (in forest areas)
        for _ in 0..config.wood_nodes {
            let pos = self.find_random_terrain_position(TerrainType::Forest, taken);
            self.resources.push(ResourceNode::new(
                ResourceType::Wood,
                pos,
                rng.gen_range(50..150),
            ));
        }

        // Generate stone nodes (in mountain and hills areas)
        for _ in 0..config.stone_nodes {
            let terrain = if rng.gen::<f32>() < 0.7 {
                TerrainType::Mountain
            } else {
                TerrainType::Hills
            };
            let pos = self.find_random_terrain_position(terrain, taken);
            self.resources.push(ResourceNode::new(
                ResourceType::Stone,
                pos,
                rng.gen_range(80..200),
            ));
        }

        // Generate iron nodes (rare, in mountains)
        for _ in 0..config.iron_nodes {
            let pos = self.find_random_terrain_position(TerrainType::Mountain, taken);
            self.resources.push(ResourceNode::new(
                ResourceType::Iron,
                pos,
                rng.gen_range(30..100),
            ));
        }

        // Generate food nodes (in plains and meadows).
        //
        // How much a bush carries comes from the same table the clustered
        // resources use, and how much *this* bush carries comes from the
        // ground it is rooted in. Both of those used to be a hard-coded
        // `gen_range(20..60)` sitting here, out of reach of anything that
        // thought it was setting the world's food supply - see
        // ISSUES_FOUND #57.
        for _ in 0..config.food_nodes {
            let terrain = if rng.gen::<f32>() < 0.6 {
                TerrainType::Plains
            } else {
                TerrainType::Meadow
            };
            let pos = self.find_random_terrain_position(terrain, taken);
            let (thin, heavy) =
                resource_spawning::TerrainResourceMapper::amount_range(ResourceType::Food);
            self.resources
                .push(resource_spawning::what_this_ground_carries(
                    &self.grid,
                    ResourceType::Food,
                    pos,
                    rng.gen_range(thin..=heavy),
                    today,
                ));
        }

        // Wild leaf and shoot, and the first roots. What a hedgerow gives
        // before anything has ripened - thin, plentiful and only there for
        // its own few weeks of the year. Rather more patches than there are
        // berry bushes, because a person living on greens has to pick a great
        // many of them.
        for (what, how_many) in [
            (ResourceType::Greens, config.food_nodes * 3),
            (ResourceType::Roots, config.food_nodes * 2),
            // Wild vetch and pea. Thin on the ground compared to the leaf -
            // a wild legume is not a crop until somebody sows it - but there
            // has to be some of it somewhere for anybody to find out what it
            // is, which is the same reason there is wild grain.
            (ResourceType::Legumes, config.food_nodes),
        ] {
            let (thin, heavy) = resource_spawning::TerrainResourceMapper::amount_range(what);

            for _ in 0..how_many {
                let terrain = if rng.gen::<f32>() < 0.5 {
                    TerrainType::Meadow
                } else {
                    TerrainType::Plains
                };
                let pos = self.find_random_terrain_position(terrain, taken);
                self.resources
                    .push(resource_spawning::what_this_ground_carries(
                        &self.grid,
                        what,
                        pos,
                        rng.gen_range(thin..=heavy),
                        today,
                    ));
            }
        }

        // The mast, under the trees that drop it.
        //
        // Fewer stands than there are berry bushes and far heavier ones: a
        // wood in October is one place where a great deal of food is on the
        // ground at once, for a few weeks, and how much depends on whether
        // this is a mast year - see `ResourceType::how_heavy_the_mast_is`.
        {
            let (thin, heavy) =
                resource_spawning::TerrainResourceMapper::amount_range(ResourceType::Nuts);
            let this_year = self.climate.calendar.year;

            for _ in 0..config.food_nodes {
                let terrain = if rng.gen::<f32>() < 0.75 {
                    TerrainType::Forest
                } else {
                    TerrainType::Hills
                };
                let pos = self.find_random_terrain_position(terrain, taken);
                self.resources
                    .push(resource_spawning::what_this_ground_carries_in(
                        &self.grid,
                        ResourceType::Nuts,
                        pos,
                        rng.gen_range(thin..=heavy),
                        today,
                        this_year,
                    ));
            }
        }

        // Salt: rare, and in two quite different places.
        //
        // On a flat, where a shallow sea dried up and left what was in it, it
        // can be picked up off the ground. In a seam in the hills it has to
        // be broken out. Both are scarce on purpose - a settlement that has
        // neither has to boil the sea for it or go without, and going without
        // is what most inland peoples actually did.
        for (terrain, how_many, carrying) in [
            (TerrainType::SaltFlat, Self::HOW_MANY_SALT_FLATS_CARRY, (40, 110)),
            (TerrainType::Mountain, Self::HOW_MANY_SEAMS_IN_THE_HILLS, (15, 45)),
        ] {
            for _ in 0..how_many {
                // Only where the ground for it actually exists. A world with
                // no coast has no flats, and asking for a position on terrain
                // that is not there would put salt in the middle of a wood.
                if !self.is_there_any_of_this_terrain(terrain) {
                    continue;
                }
                let pos = self.find_random_terrain_position(terrain, taken);
                self.resources.push(ResourceNode::new(
                    ResourceType::Salt,
                    pos,
                    rng.gen_range(carrying.0..carrying.1),
                ));
            }
        }

        // Generate water sources (rivers, wells, springs)
        // Water is critical for survival - place near various terrains
        for _ in 0..config.water_sources {
            // Water can be found in various locations
            let terrain = match rng.gen_range(0..4) {
                0 => TerrainType::Plains,   // River in plains
                1 => TerrainType::Meadow,   // Stream in meadow
                2 => TerrainType::Forest,   // Spring in forest
                _ => TerrainType::Hills,    // Well in hills
            };
            let pos = self.find_random_terrain_position(terrain, taken);
            // Water sources are renewable and have high capacity
            self.resources.push(ResourceNode::new(
                ResourceType::Water,
                pos,
                rng.gen_range(200..500), // High capacity, water is abundant at source
            ));
        }

        // And the sea, which is water and is not a drink.
        //
        // It has to be here as a water source or nobody could ever make the
        // mistake: a thing an agent cannot reach is a thing an agent cannot
        // learn about. What stops them drinking it is not that it is
        // unreachable, it is that they know better - see
        // `Agent::would_i_drink_the_sea`, which is false for everybody who is
        // not already dying of thirst.
        for terrain in [TerrainType::Sea, TerrainType::SaltMarsh] {
            if !self.is_there_any_of_this_terrain(terrain) {
                continue;
            }
            for _ in 0..Self::HOW_MANY_PLACES_THE_SEA_CAN_BE_REACHED {
                let pos = self.find_random_terrain_position(terrain, taken);
                self.resources.push(ResourceNode::new(
                    ResourceType::Water,
                    pos,
                    // As much as any one source in this world carries. There
                    // is no running the sea dry, but a node is a node and
                    // nothing else in the world is allowed to be bigger.
                    rng.gen_range(400..500),
                ));
            }
        }
    }

    /// How many places along a coast a person can get down to the water.
    const HOW_MANY_PLACES_THE_SEA_CAN_BE_REACHED: u32 = 3;

    /// Work out what every spring in this world puts out, before anybody
    /// drinks from one.
    ///
    /// `regenerate_resources` sets this, and it does not run until the tenth
    /// tick. A source with no flow on it yet has no floor under it, so the
    /// founders could drink one dry in the first morning of the world - which
    /// is the whole failure this is meant to prevent, arriving ten ticks early.
    fn prime_the_springs(&mut self) {
        let precipitation = self.climate.weather.wetness_per_tick() * 100.0;

        for resource in &mut self.resources {
            if resource.resource_type != ResourceType::Water {
                continue;
            }

            let terrain_type = self
                .grid
                .get_tile(&resource.position)
                .map(|tile| tile.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            // What stops a spring is the ground freezing, not the air being
            // cold for an hour - see `ClimateManager::water_temperature`.
            let frozen = self.climate.is_the_water_frozen(resource.position, terrain_type);
            let inflow = resource.water_inflow(terrain_type, precipitation, frozen);

            resource.flow = inflow;
        }
    }

    /// Generate naturalistic resources for technology progression
    fn generate_naturalistic_resources(
        &mut self,
        config: &ResourceConfig,
        today: u32,
        taken: &mut std::collections::BTreeSet<(i32, i32)>,
    ) {
        use resource_spawning::{NaturalisticResourceConfig, NaturalisticSpawner};

        // Convert ResourceConfig to NaturalisticResourceConfig
        let nat_config = NaturalisticResourceConfig {
            clay_clusters: config.clay_clusters,
            sand_clusters: config.sand_clusters,
            coal_clusters: config.coal_clusters,
            grain_patches: config.grain_patches,
            flax_patches: config.flax_patches,
            herb_patches: config.herb_patches,
            cotton_patches: config.cotton_patches,
            honey_locations: config.honey_locations,
            fish_areas: config.fish_areas,
            nodes_per_cluster: 3,
            cluster_radius: 5,
        };

        // Use naturalistic spawner
        let mut spawner = NaturalisticSpawner::new(&self.grid, today);
        let new_resources = spawner.spawn_all(&nat_config);

        // Add spawned resources. The spawner chooses its own ground and does
        // not ask about what is already there, so the register hears about
        // what it put down rather than the other way about.
        for resource in &new_resources {
            taken.insert((resource.position.x, resource.position.y));
        }
        self.resources.extend(new_resources);

        log::info!(
            "Generated naturalistic resources: {} clay, {} sand, {} coal, {} grain, {} flax, {} herbs clusters",
            config.clay_clusters, config.sand_clusters, config.coal_clusters,
            config.grain_patches, config.flax_patches, config.herb_patches
        );
    }

    /// Update the resource_nodes map for efficient spatial queries
    fn update_resource_node_map(&mut self) {
        self.resource_nodes.clear();

        for resource in &self.resources {
            let type_name = format!("{:?}", resource.resource_type);
            let pos_tuple = (resource.position.x, resource.position.y, 0);

            self.resource_nodes
                .entry(type_name)
                .or_insert_with(Vec::new)
                .push(pos_tuple);
        }
    }

    /// Whether this world has any of a given ground in it at all.
    ///
    /// `find_random_terrain_position` falls back to *any* free tile when it
    /// cannot find the ground it was asked for, which is right for wood in a
    /// world short of forest and quite wrong for salt: it would put a salt
    /// flat in the middle of a wood. A world with no coast should simply have
    /// no flats.
    fn is_there_any_of_this_terrain(&self, terrain_type: TerrainType) -> bool {
        (0..self.grid.height).any(|y| {
            (0..self.grid.width).any(|x| {
                self.grid
                    .get_tile(&Position::new(x as i32, y as i32))
                    .is_some_and(|tile| tile.terrain.terrain_type == terrain_type)
            })
        })
    }

    /// How many patches of salt a world's flats carry.
    const HOW_MANY_SALT_FLATS_CARRY: u32 = 4;

    /// And how many seams there are in the hills, for a people with no coast.
    const HOW_MANY_SEAMS_IN_THE_HILLS: u32 = 2;

    /// The ground that already has something standing on it.
    ///
    /// The same question [`World::is_position_occupied`] answers, asked once
    /// for the whole map instead of once for every node placed on it.
    ///
    /// Stocking a map places about one node per seven tiles and each placement
    /// walked the whole resource list to find out whether its spot was taken,
    /// so the cost of building a world was the square of the world. A quarter
    /// of a square kilometre took a millisecond; twenty-five square kilometres
    /// took five and a half seconds; a hundred would not finish.
    pub fn what_ground_is_taken(&self) -> std::collections::BTreeSet<(i32, i32)> {
        self.buildings
            .iter()
            .map(|building| (building.position.x, building.position.y))
            .chain(
                self.resources
                    .iter()
                    .map(|resource| (resource.position.x, resource.position.y)),
            )
            .collect()
    }

    /// Somewhere with this ground on it that nothing is standing on yet.
    ///
    /// `taken` is the ground already spoken for, and this adds to it before
    /// returning, so a caller placing a great many things in a row asks the
    /// map once rather than once a thing. It has to give the same answers as
    /// asking `is_position_occupied` afresh every time, which means every
    /// caller that puts something down at a position this did not choose has
    /// to say so - see `land_tests::stocking_a_map_leaves_no_two_things_on_a
    /// _tile`.
    fn find_random_terrain_position(
        &self,
        terrain_type: TerrainType,
        taken: &mut std::collections::BTreeSet<(i32, i32)>,
    ) -> Position {
        use rand::seq::SliceRandom;
        let mut rng = crate::core::dice::roll();

        let mut claim = |pos: Position, taken: &mut std::collections::BTreeSet<(i32, i32)>| {
            taken.insert((pos.x, pos.y));
            pos
        };

        // First, try random sampling (efficient for common terrain types)
        for _ in 0..100 {
            let x = rng.gen_range(0..self.grid.width) as i32;
            let y = rng.gen_range(0..self.grid.height) as i32;
            let pos = Position::new(x, y);

            if let Some(tile) = self.grid.get_tile(&pos) {
                if tile.terrain.terrain_type == terrain_type {
                    // Check if position is not occupied
                    if !taken.contains(&(pos.x, pos.y)) {
                        return claim(pos, taken);
                    }
                }
            }
        }

        // Fallback: collect ALL valid positions and randomly select
        // This guarantees we find a valid position if one exists
        let valid_positions: Vec<Position> = (0..self.grid.width)
            .flat_map(|x| (0..self.grid.height).map(move |y| Position::new(x as i32, y as i32)))
            .filter(|pos| {
                if let Some(tile) = self.grid.get_tile(pos) {
                    tile.terrain.terrain_type == terrain_type
                        && !taken.contains(&(pos.x, pos.y))
                } else {
                    false
                }
            })
            .collect();

        if let Some(pos) = valid_positions.choose(&mut rng) {
            return claim(*pos, taken);
        }

        // Last resort: if no valid terrain exists, find ANY unoccupied position
        // of the requested type, allowing overlap with resources
        let any_matching: Vec<Position> = (0..self.grid.width)
            .flat_map(|x| (0..self.grid.height).map(move |y| Position::new(x as i32, y as i32)))
            .filter(|pos| {
                if let Some(tile) = self.grid.get_tile(pos) {
                    tile.terrain.terrain_type == terrain_type
                } else {
                    false
                }
            })
            .collect();

        if let Some(pos) = any_matching.choose(&mut rng) {
            return claim(*pos, taken);
        }

        // Absolute last resort: return center position (should never happen in a valid world)
        log::warn!(
            "Could not find any {:?} terrain in world, placing resource at center",
            terrain_type
        );
        claim(
            Position::new(self.grid.width as i32 / 2, self.grid.height as i32 / 2),
            taken,
        )
    }

    pub fn is_position_occupied(&self, pos: &Position) -> bool {
        // Check buildings
        if self.buildings.iter().any(|b| &b.position == pos) {
            return true;
        }

        // Check resources
        if self.resources.iter().any(|r| &r.position == pos) {
            return true;
        }

        false
    }

    pub fn add_building(&mut self, building: Building) {
        self.buildings.push(building);
    }

    pub fn get_resource_at(&self, pos: &Position) -> Option<&ResourceNode> {
        self.resources.iter().find(|r| &r.position == pos)
    }

    pub fn get_resource_at_mut(&mut self, pos: &Position) -> Option<&mut ResourceNode> {
        self.resources.iter_mut().find(|r| &r.position == pos)
    }

    pub fn get_building_at(&self, pos: &Position) -> Option<&Building> {
        self.buildings.iter().find(|b| &b.position == pos)
    }

    pub fn remove_depleted_resources(&mut self) {
        // A renewable node stays on the map when emptied so it can regrow;
        // deleting it would make berry patches and fish runs single-use and
        // drain the world of food permanently. Mined-out mineral deposits are
        // genuinely gone and are removed - but the ground remembers being
        // worked, which is what tells a stripped seam from a spot somebody
        // made up.
        let worked_out = &mut self.where_it_was_worked_out;
        self.resources.retain(|r| {
            let keeping = r.amount > 0 || r.is_renewable();
            if !keeping {
                worked_out.insert(r.position);
            }
            keeping
        });
    }

    // ===== Heat Source Management =====

    /// Build a new heat source at a position
    pub fn build_heat_source(
        &mut self,
        heat_source_type: crate::environment::HeatSourceType,
        position: (i32, i32, i32),
        builder_id: Option<uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        let (x, y, _z) = position;
        if x < 0 || y < 0 || x >= self.grid.width as i32 || y >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        // Check if there's already a heat source at this position
        if self.heat_sources.get_at_position(position).is_some() {
            return Err("Heat source already exists at this position".to_string());
        }

        // Create the heat source
        let mut heat_source = crate::environment::HeatSource::new(
            heat_source_type,
            position,
            self.tick as u64,
        );

        if let Some(builder) = builder_id {
            heat_source = heat_source.with_builder(builder);
        }

        let id = heat_source.id;
        self.heat_sources.add(heat_source);

        Ok(id)
    }

    /// Add fuel to a heat source
    pub fn add_fuel_to_heat_source(
        &mut self,
        heat_source_id: &uuid::Uuid,
        material_id: String,
        amount: f32,
    ) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            // Default burn time based on material (could be expanded)
            let burn_time = match material_id.as_str() {
                "wood" => 100,
                "charcoal" => 200,
                "coal" => 300,
                _ => 50,
            };

            heat_source.add_fuel(material_id, amount, burn_time);
            Ok(())
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Light a heat source
    pub fn light_heat_source(&mut self, heat_source_id: &uuid::Uuid) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            if heat_source.light() {
                Ok(())
            } else {
                Err("Cannot light heat source (no fuel)".to_string())
            }
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Extinguish a heat source
    pub fn extinguish_heat_source(&mut self, heat_source_id: &uuid::Uuid) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            heat_source.extinguish();
            Ok(())
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Add materials to heat/smelt
    pub fn add_to_heat_source(
        &mut self,
        heat_source_id: &uuid::Uuid,
        material_id: String,
        quantity: u32,
    ) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            heat_source.add_contents(material_id, quantity);
            Ok(())
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Get heat source at position (2D, assumes z=0)
    pub fn get_heat_source_at(&self, x: i32, y: i32) -> Option<&crate::environment::HeatSource> {
        self.heat_sources.get_at_position((x, y, 0))
    }

    /// Get all heat sources within range of a position
    pub fn get_heat_sources_in_range(
        &self,
        position: (i32, i32, i32),
        range: f32,
    ) -> Vec<&crate::environment::HeatSource> {
        self.heat_sources.in_range(position, range)
    }

    /// Get temperature contribution from nearby heat sources
    pub fn environmental_temperature(&self, position: (i32, i32, i32), range: f32) -> f32 {
        let nearby_sources = self.get_heat_sources_in_range(position, range);

        let mut total_heat_contribution = 0.0;

        for source in nearby_sources {
            if source.is_lit {
                let dx = (source.position.0 - position.0) as f32;
                let dy = (source.position.1 - position.1) as f32;
                let dz = (source.position.2 - position.2) as f32;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(1.0);

                // Heat contribution falls off with distance
                let contribution = (source.current_temperature - 20.0) / distance;
                total_heat_contribution += contribution;
            }
        }

        // Base environmental temp + heat contribution
        20.0 + total_heat_contribution
    }

    // ===== Animal Management =====

    /// Spawn a wild animal at a position
    pub fn spawn_animal(
        &mut self,
        species_id: String,
        position: (i32, i32),
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        if position.0 < 0 || position.1 < 0 ||
           position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.animals.spawn_animal(species_id, position)
            .ok_or_else(|| "Failed to spawn animal (max population reached or invalid species)".to_string())
    }

    /// Spawn a group/herd of animals
    pub fn spawn_animal_group(
        &mut self,
        species_id: String,
        center: (i32, i32),
        count: u32,
    ) -> Result<uuid::Uuid, String> {
        // Check if center position is valid
        if center.0 < 0 || center.1 < 0 ||
           center.0 >= self.grid.width as i32 || center.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.animals.spawn_group(species_id, center, count)
            .ok_or_else(|| "Failed to spawn animal group".to_string())
    }

    /// Get animals within radius of a position
    pub fn get_animals_in_radius(
        &self,
        center: (i32, i32),
        radius: f32,
    ) -> Vec<&crate::environment::Animal> {
        self.animals.get_in_radius(center, radius)
    }


    /// Tame an animal (increase tame level)
    pub fn tame_animal(&mut self, animal_id: &uuid::Uuid, amount: f32) -> Result<(), String> {
        if let Some(animal) = self.animals.get_mut(animal_id) {
            animal.tame(amount);
            Ok(())
        } else {
            Err("Animal not found".to_string())
        }
    }

    /// Feed an animal (restores stamina and health)
    pub fn feed_animal(&mut self, animal_id: &uuid::Uuid, amount: f32) -> Result<(), String> {
        if let Some(animal) = self.animals.get_mut(animal_id) {
            animal.stamina = (animal.stamina + amount).min(animal.max_stamina);
            animal.heal(amount * 0.5); // Restore some health too
            Ok(())
        } else {
            Err("Animal not found".to_string())
        }
    }

    /// Damage an animal
    pub fn damage_animal(&mut self, animal_id: &uuid::Uuid, damage: f32) -> Result<bool, String> {
        if let Some(animal) = self.animals.get_mut(animal_id) {
            animal.take_damage(damage);
            let is_dead = !animal.is_alive();
            Ok(is_dead)
        } else {
            Err("Animal not found".to_string())
        }
    }

    /// Get all animals of a specific species
    pub fn get_animals_by_species(&self, species_id: &str) -> Vec<&crate::environment::Animal> {
        self.animals.get_all()
            .iter()
            .filter(|a| a.species_id == species_id)
            .collect()
    }

    /// Get all domesticated animals
    pub fn get_domesticated_animals(&self) -> Vec<&crate::environment::Animal> {
        self.animals.get_all()
            .iter()
            .filter(|a| a.is_domesticated)
            .collect()
    }

    // ===== Plant Management =====

    /// Plant a crop at a position (cultivated)
    pub fn plant_crop(
        &mut self,
        species_id: String,
        position: (i32, i32),
        planter_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        if position.0 < 0 || position.1 < 0 ||
           position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.plants.plant_crop(species_id, position, planter_id, self.tick)
            .ok_or_else(|| "Failed to plant crop (max population reached or invalid species)".to_string())
    }

    /// Spawn a wild plant at a position
    pub fn spawn_plant(
        &mut self,
        species_id: String,
        position: (i32, i32),
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        if position.0 < 0 || position.1 < 0 ||
           position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.plants.spawn_plant(species_id, position, self.tick)
            .ok_or_else(|| "Failed to spawn plant (max population reached or invalid species)".to_string())
    }

    /// Spawn a patch of plants (forest, field, etc.)
    pub fn spawn_plant_patch(
        &mut self,
        species_id: String,
        center: (i32, i32),
        radius: u32,
        density: f32,
    ) -> Vec<uuid::Uuid> {
        self.plants.spawn_patch(species_id, center, radius, density, self.tick)
    }

    /// Harvest a plant
    pub fn harvest_plant(
        &mut self,
        plant_id: &uuid::Uuid,
    ) -> Result<Vec<crate::environment::PlantDrop>, String> {
        self.plants.harvest_plant(plant_id)
            .ok_or_else(|| "Failed to harvest plant (not found or not harvestable)".to_string())
    }

    /// Get harvestable plants in radius
    pub fn get_harvestable_plants(
        &self,
        center: (i32, i32),
        radius: f32,
    ) -> Vec<&crate::environment::Plant> {
        self.plants.get_harvestable_in_radius(center, radius)
    }

    /// Get all plants in radius
    pub fn get_plants_in_radius(
        &self,
        center: (i32, i32),
        radius: f32,
    ) -> Vec<&crate::environment::Plant> {
        self.plants.get_in_radius(center, radius)
    }


    /// Get all plants of a specific species
    pub fn get_plants_by_species(&self, species_id: &str) -> Vec<&crate::environment::Plant> {
        self.plants.all_plants()
            .iter()
            .filter(|p| p.species_id == species_id)
            .collect()
    }

    /// Get all cultivated plants
    pub fn get_cultivated_plants(&self) -> Vec<&crate::environment::Plant> {
        self.plants.all_plants()
            .iter()
            .filter(|p| p.is_cultivated)
            .collect()
    }

    // ===== Combat System =====





    /// Get combat statistics for an entity
    pub fn get_combat_stats(&self, entity_id: &uuid::Uuid) -> combat::CombatStatistics {
        self.combat_manager.get_combat_stats(entity_id)
    }

    /// Get recent combat log
    pub fn get_recent_combat(&self, count: usize) -> Vec<&combat::CombatResult> {
        self.combat_manager.get_recent_combat(count)
    }

    // ===== Crafting System =====

    /// Get a crafting recipe
    pub fn get_recipe(&self, recipe_id: &str) -> Option<&crafting::CraftingRecipe> {
        self.crafting_manager.get_recipe(recipe_id)
    }

    /// Get all recipes in a category
    pub fn get_recipes_by_category(&self, category: crafting::CraftingCategory) -> Vec<&crafting::CraftingRecipe> {
        self.crafting_manager.get_recipes_by_category(category)
    }




    /// Get active crafting jobs for a crafter
    pub fn get_crafter_jobs(&self, crafter_id: &uuid::Uuid) -> Vec<&crafting::CraftingJob> {
        self.crafting_manager.get_crafter_jobs(crafter_id)
    }


    // ===== Smelting System =====

    /// Get smelting recipes for a material
    pub fn get_smelting_recipes(&self, material_id: &str) -> Vec<&crate::environment::smelting::SmeltingRecipe> {
        self.heat_sources.get_smelting_recipes(material_id)
    }

    /// Check if a material can be smelted
    pub fn can_smelt_material(&self, material_id: &str) -> bool {
        self.heat_sources.can_smelt_material(material_id)
    }


    pub fn tick(&mut self) {
        self.tick += 1;

        // Update climate (weather, seasons, time)
        self.climate.tick();

        // Update buildings
        for building in &mut self.buildings {
            building.tick();
        }

        // Update heat sources (fuel consumption, heating)
        self.heat_sources.tick_all();

        // And the weather gets at whatever is lying about
        if self.tick % crate::environment::seasons::ONCE_A_DAY == 0 {
            self.what_is_lying_about_weathers();
        }

        // What is under the earth keeps
        self.what_is_buried_keeps();

        // Update animals (AI, movement, aging), and what they take off the
        // ground and put back onto it. Grazing runs on the vegetation's own
        // ten-tick cadence - see `AnimalManager::tick_in_world` - so a
        // grazing pass stands for ten ticks of feeding.
        // The cadence and the amount are one number. A pass stands for
        // exactly as long as it is since the last pass, and reading that off
        // two separate literals is how a herd ends up eating a tenth or ten
        // times what it should the moment the turn length changes.
        let how_often_the_ground_is_grazed = crate::environment::seasons::ONCE_A_DAY;
        let grazing_ticks = if self.tick % how_often_the_ground_is_grazed == 0 {
            how_often_the_ground_is_grazed as f32
        } else {
            0.0
        };
        let weather = crate::environment::GrazingWeather {
            precipitation: self.climate.weather.wetness_per_tick() * 100.0,
            now: self.tick,
            season: self.climate.current_season(),
        };
        self.animals.tick_in_world(
            &mut self.grid,
            &mut self.plants,
            grazing_ticks,
            weather,
        );

        // And what has gone into the snares, and what has come out of them
        // again. Taken and put back because the snares are the world's and
        // the small life is the fauna's, and the pass needs both.
        if !self.snares.is_empty() {
            let mut snares = std::mem::take(&mut self.snares);
            let mut rng = crate::core::dice::roll();
            self.animals.small_life.tick_the_snares(
                &mut snares,
                self.tick,
                crate::environment::fauna::AnimalManager::whose_ground,
                &mut rng,
            );
            self.snares = snares;
        }

        // Update plants: growth on what the ground and sky give them, and the
        // leaf fall that in time becomes more of it.
        //
        // One zone of the map in twenty-four, one of them every sixty ticks,
        // so any given plant is worked out once in fourteen hundred and forty
        // ticks - four months - and no single tick carries more than a
        // twenty-fourth of the map. This is the most expensive thing in a tick
        // and the one that least needs doing often: a hundred square
        // kilometres carries a quarter of a million plants and nothing a plant
        // does on its own happens inside four months.
        //
        // Ground something is standing on does not wait for its zone.
        // `AnimalManager` brings a plant up to date before it takes a bite out
        // of it - see `PlantManager::catch_up_one` - because a grazed plant
        // would otherwise lose condition a hundred and forty-four times for
        // every time it gained any.
        // The one spelling, and it lives with the plants because the plants
        // are what it is about - see `PlantManager::HOW_OFTEN_A_ZONE_COMES_ROUND`.
        use crate::environment::flora::PlantManager;
        const HOW_OFTEN_A_ZONE_COMES_ROUND: u32 = PlantManager::HOW_OFTEN_A_ZONE_COMES_ROUND;

        if self.tick % HOW_OFTEN_A_ZONE_COMES_ROUND == 0 {
            let precipitation = self.climate.weather.wetness_per_tick() * 100.0;
            let season = self.climate.current_season();
            let zone = (self.tick / HOW_OFTEN_A_ZONE_COMES_ROUND) as usize
                % crate::environment::PlantManager::HOW_MANY_ZONES;

            self.plants
                .grow_a_zone(&mut self.grid, precipitation, self.tick, season, zone);
        }

        // Regenerate resources based on climate conditions (every 10 ticks to reduce overhead)
        if self.tick % crate::environment::seasons::ONCE_A_DAY == 0 {
            self.rot_what_is_lying_about();
            self.regenerate_resources();
        }

        // Update crafting jobs (progress crafting)
        // Completed crafts are tracked but not auto-distributed - agents poll for their completed jobs
        // via World::get_completed_crafts_for_agent() to add items to their inventories
        self.crafting_manager.tick();

        // Remove depleted resources
        self.remove_depleted_resources();

        // And drop the ground that has gone bare again off the visiting list.
        // Once a tick rather than on every read, so a reader may see a tile
        // that has just finished - which is why every reader asks its own
        // question of the tile as well.
        self.grid.forget_bare_ground();
    }

    /// Regenerate renewable resources based on climate and weather conditions
    /// Break down everything lying on the ground into nutrient.
    ///
    /// The rate is the ground's to decide: wet country turns leaf fall into
    /// soil inside a season, and a desert holds what falls on it more or less
    /// forever. Density does the rest - the leaves that come off a tree are
    /// gone long before the tree is.
    fn rot_what_is_lying_about(&mut self) {
        use crate::world::soil::Soil;

        // Rain reaches everywhere; the ground decides what it does with it
        let precipitation = self.climate.weather.wetness_per_tick() * 100.0;

        // A pass stands for however long it has been since the last one, which
        // is one number and not two. This read ten while the trigger in
        // `World::tick` read ten separately, in another function - two
        // spellings of one cadence, and shortening the turn would have moved
        // one and not the other.
        const TICKS_PER_PASS: f32 = crate::environment::seasons::ONCE_A_DAY as f32;

        // Every tile in the world, because every tile in the world has litter
        // on it - `Soil::for_terrain` gives a forest floor 1.5 and a desert
        // 0.02, and rot never quite takes the last of it. There is nothing to
        // narrow here and the register would hold the whole map. One pass in
        // ten ticks over a million tiles is about half a millisecond, which is
        // a twentieth of what the two sweeps that *could* be narrowed were
        // costing. See ISSUES_FOUND.md #128.
        for row in &mut self.grid.tiles {
            for tile in row.iter_mut() {
                if tile.soil.litter() <= 0.0 {
                    continue;
                }

                let humidity = Soil::humidity(tile.terrain.terrain_type, precipitation);
                tile.soil.decay(humidity, TICKS_PER_PASS);
            }
        }
    }

    fn regenerate_resources(&mut self) {
        let current_season = self.climate.current_season();
        let today = self.climate.calendar.day_of_year;
        let this_year = self.climate.calendar.year;
        let season_modifier = current_season.plant_growth_modifier();
        let precipitation = self.climate.weather.wetness_per_tick() * 100.0; // Scale to 0-1 range

        for resource in &mut self.resources {
            // Get temperature at resource position
            let terrain_type = self.grid.get_tile(&resource.position)
                .map(|t| t.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            let temperature = self.climate.get_temperature(resource.position, terrain_type);
            // And how warm the water is, which is a different question and
            // the one that decides ice. See `ClimateManager::water_temperature`.
            let frozen_water = self.climate.is_the_water_frozen(resource.position, terrain_type);

            // Water is fed by the ground it sits on and the weather over it,
            // not by growing back the way a berry patch does
            if resource.resource_type == ResourceType::Water {
                let inflow = resource.water_inflow(
                    terrain_type,
                    precipitation,
                    frozen_water,
                );

                // The rate is also the floor. What is standing in a spring is
                // this pass's flow arriving, not a barrel somebody filled, so
                // it is the one resource in this world that cannot be taken
                // away - see `ResourceNode::what_can_be_taken`.
                resource.flow = inflow;
                resource.take_inflow(inflow);
                continue;
            }

            // Fish come up the river rather than growing back out of what is
            // left of them. What arrives is what the season is running, not
            // what last year's fishing left behind, so a reach that was taken
            // down to nothing fills again - see `fish_run`.
            if resource.resource_type.grows_in_water() {
                let run = resource.fish_run(terrain_type, current_season, frozen_water);
                resource.take_inflow(run);
                continue;
            }

            // Everything else grows out of the ground it is standing in, and
            // takes what it grows with. Broken ground gets at more of what is
            // there and carries a heavier crop; it does not grow faster than
            // the plant's kind can grow.
            let cultivated = terrain_type == TerrainType::Farmland;

            // What a plant drinks is what the ground holds, not whether it
            // happens to be raining on it this hour
            let ground_water =
                crate::world::soil::Soil::humidity(terrain_type, precipitation);

            let soil = match self.grid.get_tile_mut(&resource.position) {
                Some(tile) => &mut tile.soil,
                None => continue,
            };

            // A field nobody has been near comes on in weeds and vermin. This
            // runs on the same weather the crop wants, because weeds do best
            // exactly when the wheat does.
            if cultivated {
                let growing = (season_modifier * ground_water).clamp(0.0, 1.0);
                soil.nobody_weeded_this(growing, 1.0);
            }

            // A hedgerow out of season carries nothing. Growth was seasonal
            // from the beginning and what was *standing* was not, so a berry
            // bush that had grown all summer still had its berries on it in
            // February - and a settlement that could pick fruit in the snow
            // had no reason to put anything by, no lean season to be lean in,
            // and no use for a store. What is on the plant now falls off it
            // outside the weeks it bears, which is what fruit does.
            if !resource.resource_type.is_it_bearing(today) {
                resource.what_it_carries_falls_off(Self::WHAT_FALLS_OFF_A_TICK, soil);
                continue;
            }

            // A pass stands for exactly the ground it covers: however long it
            // has been since the last one. The rates inside are per-pass
            // numbers fitted when a pass was ten ticks, and they are read
            // against that - see `ResourceNode::WHAT_THESE_RATES_WERE_FITTED_TO`.
            let _regen_amount = resource.regenerate_in_ground(
                temperature,
                ground_water,
                season_modifier,
                cultivated,
                soil,
                crate::environment::seasons::ONCE_A_DAY as f32,
            );

            // And how good a year it is, which only the mast asks. A wood
            // that stood full last autumn stands nearly bare this one, and
            // that is the first thing in this model that makes one year
            // different from another - see `how_heavy_the_mast_is`.
            if resource.resource_type.does_it_have_mast_years() {
                let mast = ResourceType::how_heavy_the_mast_is(this_year);
                let this_autumn = ((resource.max_amount as f32 * mast).round() as u32).max(1);
                resource.amount = resource.amount.min(this_autumn);
            }

            // Debug log significant regeneration
            // if regen_amount > 0 {
            //     debug!("Resource {:?} at ({}, {}) regenerated {} units",
            //         resource.resource_type, resource.position.x, resource.position.y, regen_amount);
            // }
        }
    }

    /// Get statistics about the world
    pub fn stats(&self) -> WorldStats {
        let mut stats = WorldStats::default();

        stats.total_resources = self.resources.len();
        stats.total_buildings = self.buildings.len();

        for resource in &self.resources {
            match resource.resource_type {
                // Basic resources
                ResourceType::Wood => stats.wood_available += resource.amount,
                ResourceType::Stone => stats.stone_available += resource.amount,
                ResourceType::Iron => stats.iron_available += resource.amount,
                ResourceType::Food => stats.food_available += resource.amount,
                // Agricultural resources
                ResourceType::Grain => stats.grain_available += resource.amount,
                ResourceType::Flax => stats.flax_available += resource.amount,
                ResourceType::Herbs => stats.herbs_available += resource.amount,
                // Animal resources
                ResourceType::Hides => stats.hides_available += resource.amount,
                ResourceType::Wool => stats.wool_available += resource.amount,
                ResourceType::Meat => stats.meat_available += resource.amount,
                ResourceType::Fish => stats.fish_available += resource.amount,
                // Mineral resources
                ResourceType::Clay => stats.clay_available += resource.amount,
                ResourceType::Coal => stats.coal_available += resource.amount,
                // Other types not individually tracked
                _ => {}
            }
        }

        // Count storehouse inventory - basic resources
        stats.wood_stored = self.storehouse_inventory.count_item(&ItemType::Wood);
        stats.stone_stored = self.storehouse_inventory.count_item(&ItemType::Stone);
        stats.iron_stored = self.storehouse_inventory.count_item(&ItemType::Iron);
        stats.food_stored = self.storehouse_inventory.count_item(&ItemType::Food);
        // Agricultural
        stats.grain_stored = self.storehouse_inventory.count_item(&ItemType::Grain);
        // Processed materials
        stats.flour_stored = self.storehouse_inventory.count_item(&ItemType::Flour);
        stats.leather_stored = self.storehouse_inventory.count_item(&ItemType::Leather);
        stats.cloth_stored = self.storehouse_inventory.count_item(&ItemType::Cloth);
        // Finished goods
        stats.bread_stored = self.storehouse_inventory.count_item(&ItemType::Bread);
        // Count tools (any tool type)
        stats.tools_stored = self.storehouse_inventory.count_item(&ItemType::WoodenAxe)
            + self.storehouse_inventory.count_item(&ItemType::StoneAxe)
            + self.storehouse_inventory.count_item(&ItemType::IronAxe);

        // Count buildings by type
        for building in &self.buildings {
            match building.building_type {
                BuildingType::Longhouse => stats.longhouses += 1,
                BuildingType::UpgradedLonghouse => stats.longhouses += 1, // Count as longhouse
                BuildingType::SmallHouse => stats.small_houses += 1,
                BuildingType::MediumHouse => stats.medium_houses += 1,
                BuildingType::LargeHouse => stats.large_houses += 1,
                BuildingType::Manor => stats.large_houses += 1, // Count as large house
                BuildingType::Storehouse | BuildingType::TownStorage => stats.storehouses += 1,
                BuildingType::Workshop => stats.workshops += 1,
                BuildingType::Smithy | BuildingType::Forge => stats.smithies += 1,
                BuildingType::Farm => stats.farms += 1,
                // All other building types are tracked in total_buildings but not individually
                _ => {}
            }
        }

        stats
    }

    /// Process exploration for an agent at a position
    /// Returns number of new tiles discovered
    pub fn process_exploration(
        &mut self,
        agent_exploration: &mut crate::agents::ExplorationKnowledge,
        agent_position: &Position,
        vision_range: u32,
        current_tick: u32,
    ) -> usize {
        let mut new_discoveries = 0;
        let range = vision_range as i32;

        // Explore all tiles in vision range
        for dx in -range..=range {
            for dy in -range..=range {
                // Check if within circular vision range (not square)
                if (dx * dx + dy * dy) as f32 > (range * range) as f32 {
                    continue;
                }

                let explore_pos = Position::new(
                    agent_position.x + dx,
                    agent_position.y + dy,
                );

                // Check if position is valid
                if !self.grid.is_valid_position(&explore_pos) {
                    continue;
                }

                // Mark tile as explored if new
                if agent_exploration.explore_tile(explore_pos, current_tick) {
                    new_discoveries += 1;

                    // Mark tile as globally explored
                    if let Some(tile) = self.grid.get_tile_mut(&explore_pos) {
                        tile.mark_explored();

                        // Discover terrain type
                        agent_exploration.encounter_terrain(
                            tile.terrain.terrain_type,
                            explore_pos,
                            current_tick,
                        );
                    }

                    // Check for resources at this position
                    for resource in &self.resources {
                        if resource.position == explore_pos && resource.amount > 0 {
                            agent_exploration.discover_resource(
                                explore_pos,
                                resource.resource_type,
                                current_tick,
                            );
                        }
                    }

                    // Check for buildings at this position
                    for building in &self.buildings {
                        if building.position == explore_pos {
                            agent_exploration.discover_building(
                                explore_pos,
                                building.building_type,
                                current_tick,
                            );
                        }
                    }
                }
            }
        }

        // Record milestone discoveries
        if new_discoveries >= 10 {
            agent_exploration.discoveries.push(crate::agents::Discovery {
                discovery_type: crate::agents::DiscoveryType::AreaExplored {
                    tiles_count: new_discoveries,
                },
                tick: current_tick,
                position: *agent_position,
            });
        }

        new_discoveries
    }

    /// Get total number of tiles in the world
    pub fn total_tiles(&self) -> usize {
        self.grid.width * self.grid.height
    }

    // ===== Helper Methods for Spatial Planning and Testing =====

    /// Place a resource node at a specific position (for testing and spatial planning)
    pub fn place_resource_node(&mut self, resource_type: &str, position: (i32, i32, i32)) {
        self.resource_nodes
            .entry(resource_type.to_string())
            .or_insert_with(Vec::new)
            .push(position);
    }

    /// Add a building at a specific position (for testing and spatial planning)
    pub fn add_building_at(&mut self, building_type: BuildingType, position: (i32, i32, i32)) {
        use crate::world::buildings::Building;
        let pos = Position::new(position.0, position.1);
        let building = Building::new(building_type, pos);
        self.buildings.push(building);
    }

    /// Check if terrain at position is passable
    pub fn is_terrain_passable(&self, position: (i32, i32, i32)) -> bool {
        // Check bounds
        if position.0 < 0 || position.1 < 0 {
            return false;
        }
        if position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return false;
        }

        // Check actual terrain walkability
        let pos = Position::new(position.0, position.1);
        if let Some(tile) = self.grid.get_tile(&pos) {
            tile.terrain.is_walkable()
        } else {
            false
        }
    }

    /// Mark an area as impassable (for testing terrain constraints)
    /// Sets tiles within the radius to Water terrain (which is not walkable)
    pub fn set_terrain_impassable(&mut self, center: (i32, i32, i32), radius: i32) {
        let (cx, cy, _) = center;

        // Mark all tiles within radius as impassable (Water terrain)
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let pos = Position::new(cx + dx, cy + dy);

                if let Some(tile) = self.grid.get_tile_mut(&pos) {
                    tile.terrain = Terrain::new(TerrainType::Water);
                }
            }
        }
    }

    // ===== Building Production and Maintenance =====


    /// Collect production from a specific building at a position
    /// Returns the resources collected, or empty vec if no building or no production
    pub fn collect_building_production_at(&mut self, position: Position) -> Vec<resources::Resource> {
        for building in &mut self.buildings {
            if building.position == position && building.is_completed() {
                return building.collect_production();
            }
        }
        Vec::new()
    }

    /// Get list of buildings that need maintenance (condition below 50%)
    /// Returns tuples of (position, building_type, condition)
    pub fn get_buildings_needing_maintenance(&self) -> Vec<(Position, BuildingType, f32)> {
        self.buildings
            .iter()
            .filter(|b| b.needs_maintenance())
            .map(|b| (b.position, b.building_type, b.condition))
            .collect()
    }

    /// Get list of buildings in critical condition (below 25%)
    /// Returns tuples of (position, building_type, condition)
    pub fn get_critical_buildings(&self) -> Vec<(Position, BuildingType, f32)> {
        self.buildings
            .iter()
            .filter(|b| b.is_critical_condition())
            .map(|b| (b.position, b.building_type, b.condition))
            .collect()
    }


    /// Get pending production info for display (without collecting)
    /// Returns map of position -> (building_type, resource_count)
    pub fn get_pending_production_info(&self) -> BTreeMap<Position, (BuildingType, usize)> {
        let mut info = BTreeMap::new();

        for building in &self.buildings {
            if building.is_completed() && !building.pending_production.is_empty() {
                info.insert(
                    building.position,
                    (building.building_type, building.pending_production.len())
                );
            }
        }

        info
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStats {
    pub total_resources: usize,
    pub total_buildings: usize,
    // Basic resources
    pub wood_available: u32,
    pub stone_available: u32,
    pub iron_available: u32,
    pub food_available: u32,
    // Extended resources
    pub clay_available: u32,
    pub coal_available: u32,
    pub grain_available: u32,
    pub herbs_available: u32,
    pub hides_available: u32,
    // Storage
    pub wood_stored: u32,
    pub stone_stored: u32,
    pub iron_stored: u32,
    pub food_stored: u32,
    // Agricultural resources
    pub grain_stored: u32,
    // Agricultural resources
    pub flax_available: u32,
    // Animal resources
    pub wool_available: u32,
    pub meat_available: u32,
    pub fish_available: u32,
    // Processed materials
    pub flour_stored: u32,
    pub leather_stored: u32,
    pub cloth_stored: u32,
    // Finished goods
    pub bread_stored: u32,
    pub tools_stored: u32,
    // Buildings
    pub longhouses: usize,
    pub small_houses: usize,
    pub medium_houses: usize,
    pub large_houses: usize,
    pub storehouses: usize,
    pub workshops: usize,
    pub smithies: usize,
    pub farms: usize,
}

// Legacy types for compatibility
pub struct GridConfig {
    /// World dimensions (width, depth, height)
    pub size: (i32, i32, i32),
    /// Size of chunks for spatial optimization
    pub chunk_size: u32,
}

impl GridConfig {
    /// Create a new grid configuration with custom size
    pub fn new(size: (i32, i32, i32)) -> Self {
        Self {
            size,
            chunk_size: 16,
        }
    }

    /// Create from a WorldSize preset
    pub fn from_world_size(world_size: WorldSize) -> Self {
        Self::new(world_size.dimensions())
    }

    /// Validate configuration
    pub fn is_valid(&self) -> bool {
        self.size.0 > 0
            && self.size.1 > 0
            && self.size.2 > 0
            && self.chunk_size > 0
            && self.chunk_size <= 64
    }

    /// Check if a position is within bounds
    pub fn is_in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        let (width, depth, height) = self.size;
        x >= -width/2 && x < width/2
            && z >= -depth/2 && z < depth/2
            && y >= 0 && y < height
    }

    /// Get total world volume
    pub fn volume(&self) -> i64 {
        self.size.0 as i64 * self.size.1 as i64 * self.size.2 as i64
    }
}

impl Default for GridConfig {
    fn default() -> Self {
        Self::from_world_size(WorldSize::Medium)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_size_presets() {
        assert_eq!(WorldSize::Tiny.dimensions(), (64, 64, 64));
        assert_eq!(WorldSize::Small.dimensions(), (128, 128, 96));
        assert_eq!(WorldSize::Medium.dimensions(), (256, 256, 128));
        assert_eq!(WorldSize::Large.dimensions(), (512, 512, 160));
        assert_eq!(WorldSize::Huge.dimensions(), (1024, 1024, 192));
    }

    #[test]
    fn test_custom_world_size() {
        let custom = WorldSize::Custom(100, 200, 300);
        assert_eq!(custom.dimensions(), (100, 200, 300));
    }

    #[test]
    fn test_world_size_validation() {
        assert!(WorldSize::Medium.is_valid());
        assert!(WorldSize::Custom(256, 256, 128).is_valid());
        assert!(!WorldSize::Custom(0, 256, 128).is_valid());
        assert!(!WorldSize::Custom(-10, 256, 128).is_valid());
        assert!(!WorldSize::Custom(10000, 256, 128).is_valid());
    }

    #[test]
    fn test_grid_config_creation() {
        let config = GridConfig::new((256, 256, 128));
        assert_eq!(config.size, (256, 256, 128));
        assert_eq!(config.chunk_size, 16);
    }

    #[test]
    fn test_grid_config_from_world_size() {
        let config = GridConfig::from_world_size(WorldSize::Large);
        assert_eq!(config.size, (512, 512, 160));
    }

    #[test]
    fn test_bounds_checking() {
        let config = GridConfig::new((100, 100, 64));

        // Within bounds (range is [-50, 50) for x and z, [0, 64) for y)
        assert!(config.is_in_bounds(0, 0, 0));
        assert!(config.is_in_bounds(49, 32, 49));
        assert!(config.is_in_bounds(-50, 63, -50)); // -50 is included (lower bound)
        assert!(config.is_in_bounds(-49, 0, -49));

        // Out of bounds
        assert!(!config.is_in_bounds(50, 0, 0));   // 50 is excluded (upper bound)
        assert!(!config.is_in_bounds(-51, 0, 0));  // -51 is out of bounds
        assert!(!config.is_in_bounds(0, -1, 0));   // negative y
        assert!(!config.is_in_bounds(0, 64, 0));   // y at max is excluded
    }

    #[test]
    fn test_world_volume() {
        let config = GridConfig::new((100, 100, 64));
        assert_eq!(config.volume(), 640000);
    }

    #[test]
    fn test_world_creation() {
        let world = World::new(WorldConfig::default());
        // Verify grid dimensions match config
        assert_eq!(world.grid.width, 50);
        assert_eq!(world.grid.height, 50);
        // Verify resources were generated
        assert!(!world.resources.is_empty());
    }

    #[test]
    fn test_world_position_validation() {
        let world = World::new(WorldConfig::default());
        // Valid position
        assert!(world.grid.is_valid_position(&Position::new(25, 25)));
        // Out of bounds positions
        assert!(!world.grid.is_valid_position(&Position::new(-1, 0)));
        assert!(!world.grid.is_valid_position(&Position::new(50, 25)));
        assert!(!world.grid.is_valid_position(&Position::new(25, 50)));
    }

    #[test]
    fn test_world_config() {
        let config = WorldConfig::default();
        assert_eq!(config.size, (50, 50));
        // Verify default resource counts
        assert_eq!(config.initial_resources.wood_nodes, 20);
        assert_eq!(config.initial_resources.stone_nodes, 15);
        assert_eq!(config.initial_resources.iron_nodes, 8);
        assert_eq!(config.initial_resources.food_nodes, 25);
    }

    #[test]
    fn test_custom_world_config() {
        let config = WorldConfig {
            size: (100, 80),
            initial_resources: ResourceConfig {
                wood_nodes: 30,
                stone_nodes: 20,
                iron_nodes: 10,
                food_nodes: 40,
                ..Default::default()
            },
        };
        assert_eq!(config.size, (100, 80));
        assert_eq!(config.initial_resources.wood_nodes, 30);

        // Create world with custom config and verify
        let world = World::new(config);
        assert_eq!(world.grid.width, 100);
        assert_eq!(world.grid.height, 80);
    }

    #[test]
    fn test_position_distance() {
        // Position is now 2D (x, y)
        let p1 = Position::new(0, 0);
        let p2 = Position::new(3, 4);
        // distance_to uses Manhattan distance: |3-0| + |4-0| = 7
        assert_eq!(p1.distance_to(&p2), 7);
        // For Euclidean distance (sqrt(3^2 + 4^2) = 5.0):
        assert_eq!(p1.euclidean_distance_to(&p2), 5.0);
    }

    #[test]
    fn test_estimated_memory() {
        let tiny = WorldSize::Tiny;
        let huge = WorldSize::Huge;

        assert!(tiny.estimated_memory_mb() < huge.estimated_memory_mb());
        assert!(tiny.estimated_memory_mb() < 50.0); // Tiny should be small
    }
}

// External TDD test modules
#[cfg(test)]
mod tdd_tests;
