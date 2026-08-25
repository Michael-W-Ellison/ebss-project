// src/world/resources.rs
//! Resource nodes and harvestable materials.

use serde::{Deserialize, Serialize};
use crate::environment::seasons::Season;
use crate::world::{Position, Soil, TerrainType};

/// Types of resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    // === Basic Resources (Existing) ===
    Wood,
    Stone,
    Iron,
    Food, // Generic food (berries, generic edibles)
    Water, // Fresh water from rivers, wells, springs

    // === Raw Materials (Agricultural) ===
    Grain,      // Wheat, barley, etc. - for flour, bread, beer
    Flax,       // For linen, rope
    Herbs,      // For medicine, dyes
    Cotton,     // For cloth

    // === Raw Materials (Animal) ===
    Hides,      // Raw animal skins - for leather
    Wool,       // From sheep - for cloth
    Meat,       // Butchered meat
    Milk,       // For cheese, butter
    Fish,       // From fishing
    Honey,      // From beekeeping

    // === Raw Materials (Mineral) ===
    Clay,       // For bricks, pottery
    Sand,       // For glass
    Coal,       // For fuel/charcoal

    // === Processed Materials ===
    Flour,      // Grain → Miller → Flour
    Leather,    // Hides → Tanner → Leather
    Cloth,      // Flax/Wool/Cotton → Weaver → Cloth
    Linen,      // Flax → Weaver → Linen (specific cloth)
    Glass,      // Sand → Glassblower → Glass
    Bricks,     // Clay → Brickmaker → Bricks
    Charcoal,   // Wood → Charcoal Maker → Charcoal
    Rope,       // Flax → Ropemaker → Rope
    Paper,      // Various → Papermaker → Paper
    Dye,        // Herbs → Dyer → Dye

    // === Finished Goods (Food) ===
    Bread,      // Flour → Baker → Bread
    Ale,        // Grain → Brewer → Ale
    Cheese,     // Milk → Cheesemaker → Cheese

    // === Finished Goods (Items) ===
    Clothing,   // Cloth → Tailor → Clothing
    Shoes,      // Leather → Cobbler → Shoes
    Tools,      // Wood + Iron → Carpenter/Blacksmith → Tools
    Weapons,    // Wood + Iron → Bowyer/Blacksmith → Weapons
    Armor,      // Leather/Iron → Leatherworker/Armorer → Armor
    Pottery,    // Clay → Potter → Pottery
    Furniture,  // Wood → Carpenter → Furniture
    Jewelry,    // Iron/Gold → Goldsmith → Jewelry
}

impl ResourceType {
    /// How well this crop repays being sown rather than found.
    ///
    /// "They need to discover which plants are suitable for farming." Not every
    /// plant is. Grain was made out of a grass by people who kept putting the
    /// heaviest heads back in the ground, and it answers a plough with several
    /// times what it gives wild. A berry bush set in rows is still a berry
    /// bush. A stand of herbs is worth nothing more for being weeded.
    ///
    /// Nobody is told which is which. An agent sows what it has in its pack,
    /// walks back at harvest, and finds out.
    pub fn takes_to_the_plough(&self) -> f32 {
        match self {
            ResourceType::Grain => 3.0,
            ResourceType::Flax | ResourceType::Cotton => 1.6,
            ResourceType::Food => 1.15,
            ResourceType::Herbs => 1.0,
            _ => 1.0,
        }
    }

    /// The kind of thing a name asks for.
    ///
    /// The names are the ones an inventory carries and an `Action::Gather`
    /// asks by, so that a chain of makings can say "flax" and have the
    /// gathering path go and find some.
    pub fn called(name: &str) -> Option<Self> {
        Some(match name {
            "wood" => ResourceType::Wood,
            "stone" => ResourceType::Stone,
            "iron" => ResourceType::Iron,
            "food" => ResourceType::Food,
            "water" => ResourceType::Water,
            "flax" => ResourceType::Flax,
            "cotton" => ResourceType::Cotton,
            "hides" => ResourceType::Hides,
            "wool" => ResourceType::Wool,
            _ => return None,
        })
    }

    /// How strongly this gives itself away by smell where it lies untouched,
    /// as a fraction of an agent's full smelling range.
    ///
    /// Human noses are poor. Berries on the bush and standing grain are close
    /// to odourless until you are almost on top of them - they are found by
    /// looking, not by sniffing. Flesh carries further. Nothing raw on the
    /// ground competes with cooking or with rot, which are what a nose is
    /// actually good for.
    pub fn raw_scent_strength(&self) -> f32 {
        match self {
            // Barely detectable: you have to be standing among them
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs => 0.08,

            // Flesh gives itself away from further off
            ResourceType::Meat | ResourceType::Fish => 0.24,

            // Damp ground and vegetation, faintly
            ResourceType::Water => 0.12,

            // Wood, stone and ore have no smell worth the name
            _ => 0.0,
        }
    }

    /// Whether an agent can eat this straight from the land.
    ///
    /// The single answer to "is this food", used by foraging, by what an agent
    /// remembers seeing, and by the scents the world gives off.
    pub fn is_edible(&self) -> bool {
        matches!(
            self,
            ResourceType::Food | ResourceType::Grain | ResourceType::Fish | ResourceType::Meat
        )
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            // Basic
            ResourceType::Wood => 't',
            ResourceType::Stone => 's',
            ResourceType::Iron => 'i',
            ResourceType::Food => 'f',
            ResourceType::Water => 'w',

            // Agricultural
            ResourceType::Grain => 'g',
            ResourceType::Flax => 'x',
            ResourceType::Herbs => 'h',
            ResourceType::Cotton => 'c',

            // Animal
            ResourceType::Hides => 'H',
            ResourceType::Wool => 'W',
            ResourceType::Meat => 'm',
            ResourceType::Milk => 'M',
            ResourceType::Fish => '~',
            ResourceType::Honey => 'y',

            // Mineral
            ResourceType::Clay => 'C',
            ResourceType::Sand => 'd',
            ResourceType::Coal => 'o',

            // Processed
            ResourceType::Flour => 'F',
            ResourceType::Leather => 'L',
            ResourceType::Cloth => 'l',
            ResourceType::Linen => 'n',
            ResourceType::Glass => 'G',
            ResourceType::Bricks => 'B',
            ResourceType::Charcoal => 'k',
            ResourceType::Rope => 'r',
            ResourceType::Paper => 'p',
            ResourceType::Dye => 'D',

            // Finished Food
            ResourceType::Bread => 'b',
            ResourceType::Ale => 'a',
            ResourceType::Cheese => 'e',

            // Finished Items
            ResourceType::Clothing => 'T',
            ResourceType::Shoes => 'S',
            ResourceType::Tools => 'O',
            ResourceType::Weapons => 'w',
            ResourceType::Armor => 'A',
            ResourceType::Pottery => 'P',
            ResourceType::Furniture => 'R',
            ResourceType::Jewelry => 'J',
        }
    }

    /// Get color code for terminal rendering
    pub fn color_code(&self) -> &'static str {
        match self {
            // Basic - Original colors
            ResourceType::Wood => "\x1b[33m",      // Yellow/Brown
            ResourceType::Stone => "\x1b[37;1m",   // Bright White
            ResourceType::Iron => "\x1b[90m",      // Dark Gray
            ResourceType::Food => "\x1b[92m",      // Bright Green
            ResourceType::Water => "\x1b[96m",     // Bright Cyan (water)

            // Agricultural - Green shades
            ResourceType::Grain => "\x1b[93m",     // Bright Yellow (wheat)
            ResourceType::Flax => "\x1b[36m",      // Cyan
            ResourceType::Herbs => "\x1b[32m",     // Green
            ResourceType::Cotton => "\x1b[97m",    // Bright White

            // Animal - Brown/Red shades
            ResourceType::Hides => "\x1b[33m",     // Yellow/Brown
            ResourceType::Wool => "\x1b[37m",      // White
            ResourceType::Meat => "\x1b[31m",      // Red
            ResourceType::Milk => "\x1b[97m",      // Bright White
            ResourceType::Fish => "\x1b[96m",      // Bright Cyan
            ResourceType::Honey => "\x1b[93m",     // Bright Yellow

            // Mineral - Gray/Brown shades
            ResourceType::Clay => "\x1b[33m",      // Yellow/Brown
            ResourceType::Sand => "\x1b[93m",      // Bright Yellow
            ResourceType::Coal => "\x1b[90m",      // Dark Gray

            // Processed - Varied colors
            ResourceType::Flour => "\x1b[97m",     // Bright White
            ResourceType::Leather => "\x1b[33m",   // Yellow/Brown
            ResourceType::Cloth => "\x1b[36m",     // Cyan
            ResourceType::Linen => "\x1b[37m",     // White
            ResourceType::Glass => "\x1b[96m",     // Bright Cyan
            ResourceType::Bricks => "\x1b[31m",    // Red
            ResourceType::Charcoal => "\x1b[90m",  // Dark Gray
            ResourceType::Rope => "\x1b[33m",      // Yellow/Brown
            ResourceType::Paper => "\x1b[97m",     // Bright White
            ResourceType::Dye => "\x1b[35m",       // Magenta

            // Finished Food - Warm colors
            ResourceType::Bread => "\x1b[33m",     // Yellow/Brown
            ResourceType::Ale => "\x1b[93m",       // Bright Yellow
            ResourceType::Cheese => "\x1b[93m",    // Bright Yellow

            // Finished Items - Various
            ResourceType::Clothing => "\x1b[36m",  // Cyan
            ResourceType::Shoes => "\x1b[33m",     // Yellow/Brown
            ResourceType::Tools => "\x1b[37m",     // White
            ResourceType::Weapons => "\x1b[37;1m", // Bright White
            ResourceType::Armor => "\x1b[37;1m",   // Bright White
            ResourceType::Pottery => "\x1b[33m",   // Yellow/Brown
            ResourceType::Furniture => "\x1b[33m", // Yellow/Brown
            ResourceType::Jewelry => "\x1b[93m",   // Bright Yellow
        }
    }

    /// Get gather time per unit (in ticks)
    /// For raw materials: time to harvest/gather
    /// For processed/finished: time to craft (base time, modified by skill)
    pub fn gather_time(&self) -> u32 {
        match self {
            // Basic - gathering
            ResourceType::Wood => 20,
            ResourceType::Stone => 30,
            ResourceType::Iron => 40,
            ResourceType::Food => 15,
            ResourceType::Water => 5, // Very quick to drink/fill containers

            // Agricultural - farming/harvesting
            ResourceType::Grain => 25,
            ResourceType::Flax => 25,
            ResourceType::Herbs => 15,
            ResourceType::Cotton => 25,

            // Animal - from animals/butchering
            ResourceType::Hides => 30,
            ResourceType::Wool => 20,
            ResourceType::Meat => 25,
            ResourceType::Milk => 10,
            ResourceType::Fish => 30,
            ResourceType::Honey => 20,

            // Mineral - mining/gathering
            ResourceType::Clay => 20,
            ResourceType::Sand => 15,
            ResourceType::Coal => 35,

            // Processed - crafting time
            ResourceType::Flour => 10,      // Milling
            ResourceType::Leather => 40,    // Tanning (slow process)
            ResourceType::Cloth => 30,      // Weaving
            ResourceType::Linen => 30,      // Weaving
            ResourceType::Glass => 50,      // Glassblowing (difficult)
            ResourceType::Bricks => 25,     // Brick making
            ResourceType::Charcoal => 35,   // Charcoal burning
            ResourceType::Rope => 20,       // Rope making
            ResourceType::Paper => 30,      // Paper making
            ResourceType::Dye => 15,        // Dye making

            // Finished Food - preparation time
            ResourceType::Bread => 20,      // Baking
            ResourceType::Ale => 30,        // Brewing
            ResourceType::Cheese => 25,     // Cheese making

            // Finished Items - crafting time
            ResourceType::Clothing => 40,   // Tailoring
            ResourceType::Shoes => 35,      // Cobbling
            ResourceType::Tools => 45,      // Tool making
            ResourceType::Weapons => 60,    // Weapon crafting
            ResourceType::Armor => 70,      // Armor crafting
            ResourceType::Pottery => 30,    // Pottery making
            ResourceType::Furniture => 50,  // Furniture making
            ResourceType::Jewelry => 55,    // Jewelry crafting
        }
    }

    /// Check if this is a raw/harvestable resource (found in world)
    pub fn is_harvestable(&self) -> bool {
        matches!(
            self,
            ResourceType::Wood | ResourceType::Stone | ResourceType::Iron | ResourceType::Food |
            ResourceType::Water | // Water from rivers, wells, springs
            ResourceType::Grain | ResourceType::Flax | ResourceType::Herbs | ResourceType::Cotton |
            ResourceType::Clay | ResourceType::Sand | ResourceType::Coal |
            ResourceType::Fish | ResourceType::Honey
        )
    }

    /// Whether this grows in water rather than out of the ground it sits beside.
    ///
    /// A fish is not grown from the bank it is caught on. It is grown in the
    /// sea and in the whole catchment above, and it swims into a settlement's
    /// reach under its own power. That makes a fishery the one food a
    /// settlement can take without the land paying for it - and, once the
    /// leavings go on a field, the one food that brings the land something
    /// from outside.
    ///
    /// Before this, fish drew nutrient out of the riverbank exactly as a crop
    /// draws it out of a field, which had the ground feeding the river.
    pub fn grows_in_water(&self) -> bool {
        matches!(self, ResourceType::Fish)
    }

    /// Check if this is an animal product (requires animals)
    pub fn is_animal_product(&self) -> bool {
        matches!(
            self,
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk
        )
    }

    /// Check if this is a processed material (requires crafting)
    pub fn is_processed(&self) -> bool {
        matches!(
            self,
            ResourceType::Flour | ResourceType::Leather | ResourceType::Cloth |
            ResourceType::Linen | ResourceType::Glass | ResourceType::Bricks |
            ResourceType::Charcoal | ResourceType::Rope | ResourceType::Paper | ResourceType::Dye
        )
    }

    /// Check if this is a finished good (final product)
    pub fn is_finished_good(&self) -> bool {
        matches!(
            self,
            ResourceType::Bread | ResourceType::Ale | ResourceType::Cheese |
            ResourceType::Clothing | ResourceType::Shoes | ResourceType::Tools |
            ResourceType::Weapons | ResourceType::Armor | ResourceType::Pottery |
            ResourceType::Furniture | ResourceType::Jewelry
        )
    }

    /// Check if this is food/consumable
    pub fn is_consumable(&self) -> bool {
        matches!(
            self,
            ResourceType::Food | ResourceType::Bread | ResourceType::Ale |
            ResourceType::Cheese | ResourceType::Meat | ResourceType::Fish | ResourceType::Honey
        )
    }

    /// Get category description
    pub fn category(&self) -> &'static str {
        match self {
            ResourceType::Wood | ResourceType::Stone | ResourceType::Iron | ResourceType::Food | ResourceType::Water => "Basic Resource",
            ResourceType::Grain | ResourceType::Flax | ResourceType::Herbs | ResourceType::Cotton => "Agricultural",
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk => "Animal Product",
            ResourceType::Fish | ResourceType::Honey => "Animal Product",
            ResourceType::Clay | ResourceType::Sand | ResourceType::Coal => "Mineral",
            ResourceType::Flour | ResourceType::Leather | ResourceType::Cloth | ResourceType::Linen |
            ResourceType::Glass | ResourceType::Bricks | ResourceType::Charcoal | ResourceType::Rope |
            ResourceType::Paper | ResourceType::Dye => "Processed Material",
            ResourceType::Bread | ResourceType::Ale | ResourceType::Cheese => "Finished Food",
            ResourceType::Clothing | ResourceType::Shoes | ResourceType::Tools | ResourceType::Weapons |
            ResourceType::Armor | ResourceType::Pottery | ResourceType::Furniture | ResourceType::Jewelry => "Finished Good",
        }
    }
}

/// A resource node in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub position: Position,
    pub amount: u32,
    pub max_amount: u32,

    /// Fraction of a unit carried over between regeneration passes, so a
    /// trickle eventually amounts to something
    #[serde(default)]
    pub inflow_carried: f32,
}

impl ResourceNode {
    pub fn new(resource_type: ResourceType, position: Position, amount: u32) -> Self {
        Self {
            resource_type,
            position,
            amount,
            max_amount: amount,
            inflow_carried: 0.0,
        }
    }

    /// Harvest resource from this node
    pub fn harvest(&mut self, amount: u32) -> u32 {
        let harvested = amount.min(self.amount);
        self.amount -= harvested;
        harvested
    }

    /// Check if node is depleted
    pub fn is_depleted(&self) -> bool {
        self.amount == 0
    }

    /// Get percentage remaining
    pub fn percentage_remaining(&self) -> f32 {
        if self.max_amount == 0 {
            return 0.0;
        }
        (self.amount as f32 / self.max_amount as f32) * 100.0
    }

    /// Whether this resource regrows on its own once harvested
    pub fn is_renewable(&self) -> bool {
        matches!(
            self.resource_type,
            ResourceType::Wood
                | ResourceType::Food
                | ResourceType::Grain
                | ResourceType::Herbs
                | ResourceType::Flax
                | ResourceType::Cotton
                | ResourceType::Honey
                | ResourceType::Fish
                // A river is not used up by the people drinking from it
                | ResourceType::Water
        )
    }

    /// Take in a fractional amount of water, carrying the remainder over.
    ///
    /// Inflow is a rate, not a whole number of units: a pool on open ground
    /// gains a tenth of a unit a pass and would otherwise never gain anything
    /// at all, because resource amounts are integers.
    pub fn take_inflow(&mut self, amount: f32) {
        if self.amount >= self.max_amount {
            self.inflow_carried = 0.0;
            return;
        }

        self.inflow_carried += amount;

        let whole = self.inflow_carried.floor();
        if whole >= 1.0 {
            self.inflow_carried -= whole;
            self.amount = (self.amount + whole as u32).min(self.max_amount);
        }
    }

    /// How readily a plant here can take up what is in the ground.
    ///
    /// This is what breaking ground actually buys. A field is weeded, watered
    /// and worked, so the crop on it gets at far more of what the soil holds
    /// than the same plant would growing wild - it reaches its natural best
    /// pace on ground that would only half feed a hedgerow. What it does not do
    /// is make a plant grow faster than its kind can grow: uptake enters the
    /// rate as a factor that is capped at one.
    pub fn uptake_multiplier(cultivated: bool) -> f32 {
        if cultivated {
            2.5
        } else {
            1.0
        }
    }

    /// The most this patch can be carrying at once, given how well fed the
    /// ground is.
    ///
    /// The other half of what a field buys: yield. Rich ground carries a
    /// heavier crop, thin ground a lighter one, and a field that has had muck
    /// spread on it carries more than one that has not.
    ///
    /// The floor used to be four tenths of the full crop, which meant ground
    /// worked down to nothing still nominally carried nearly half of what it
    /// had when it was rich. Over a long run that hid the cost of farming: a
    /// settlement's fields fell to a twentieth of their fertility while their
    /// stated yield fell by four per cent. A worked-out field now carries
    /// almost nothing, which is what a worked-out field does.
    pub fn standing_capacity(&self, fertility: f32) -> u32 {
        let share = Self::MIN_YIELD_SHARE
            + (1.0 - Self::MIN_YIELD_SHARE) * fertility.clamp(0.0, 1.0);
        ((self.max_amount as f32) * share).round() as u32
    }

    /// What ground with nothing left in it still carries: the odd volunteer
    /// plant living off what blows in, and not a crop.
    const MIN_YIELD_SHARE: f32 = 0.05;

    /// The same, for ground that has been broken and sown.
    ///
    /// Breaking ground buys uptake and it buys this, and nothing else: a field
    /// of grain stands far thicker than the same ground would carry wild,
    /// because it is all one plant and all of it wanted. What it does not buy
    /// is speed - a crop grows at the pace its kind grows at, worked or not.
    ///
    /// How much thicker depends on what was sown. This is where suitability
    /// tells: a field of grain carries three times what the ground would
    /// otherwise, a field of berry bushes barely more than the hedge it came
    /// out of. See [`ResourceType::takes_to_the_plough`].
    pub fn how_heavy_a_crop_it_carries(&self, fertility: f32, cultivated: bool) -> u32 {
        let standing = self.standing_capacity(fertility) as f32;

        if !cultivated {
            return standing.round() as u32;
        }

        (standing * self.resource_type.takes_to_the_plough()).round() as u32
    }

    /// How fast this water refills, given the ground it sits on and the
    /// weather over it.
    ///
    /// Water does not grow back the way berries do. A river carries it in from
    /// somewhere upstream and is effectively bottomless; a spring in the hills
    /// keeps giving whatever the weather does; a pool on open ground is
    /// standing water and lives on the rain. It used to regenerate at nothing
    /// at all and was not counted as renewable, so every drink took a unit out
    /// of the world for good and a lake drunk dry was deleted. A world lost
    /// more than half its water in fifteen thousand ticks.
    ///
    /// Returns units per regeneration pass, which runs every ten world ticks.
    pub fn water_inflow(&self, terrain: TerrainType, precipitation: f32, freezing: bool) -> f32 {
        if self.resource_type != ResourceType::Water {
            return 0.0;
        }

        // What the ground itself brings. Water sources are scattered across
        // the map rather than sitting on water tiles - they are the streams,
        // springs and ponds of the country they are in, and what feeds them
        // depends on which.
        let source = match terrain {
            // Running water: whatever is drawn is replaced from upstream
            TerrainType::Water | TerrainType::Riverbank => 3.0,

            // Springs and snowmelt come off high ground
            TerrainType::Mountain | TerrainType::Hills => 1.5,

            // Seeps and marsh hold what they get
            TerrainType::Wetland | TerrainType::Forest => 1.2,

            // Anywhere else it is open water, and lives mostly on the sky
            _ => 0.8,
        };

        // Rain tops everything up; a dry spell is felt most by the pools
        let rain = precipitation.clamp(0.0, 1.0);
        let from_sky = rain * 0.6;

        // Frozen ground gives nothing up, and the rain falls as snow
        let flow = source + from_sky;
        if freezing {
            flow * 0.25
        } else {
            flow
        }
    }

    /// What the run brings into a reach of water, per pass of the resource
    /// tick (one pass every ten ticks, as `water_inflow` is also reckoned).
    ///
    /// Fish do not grow back the way a berry patch grows back. A berry patch
    /// regrows out of what is left of itself, in the ground it stands in, so
    /// fishing a reach out would end it the way harvesting a field to nothing
    /// ends the field. Fish arrive: they are spawned upstream and fed at sea,
    /// and they come back up the rivers under their own power whatever was
    /// taken out of this particular pool last year.
    ///
    /// So the run does not depend on how many are left here. That is what
    /// makes a fishery worth having and what makes it nearly inexhaustible:
    /// it is fed from outside the country a settlement can see, by water that
    /// is not the settlement's to use up. What bounds a catch is the season,
    /// the reach, and how many hours somebody is willing to stand in a river.
    ///
    /// The runs are the point of the year in a fishing people's calendar.
    /// Spring and autumn are heavy; high summer is thin because the run is
    /// past; winter is thinnest of all, and a frozen river gives up almost
    /// nothing.
    pub fn fish_run(&self, terrain: TerrainType, season: Season, freezing: bool) -> f32 {
        if !self.resource_type.grows_in_water() {
            return 0.0;
        }

        // How much water this reach connects to. A river carries a run; a
        // beach gets what comes along the shore; a pond gets whatever
        // wandered in.
        let reach = match terrain {
            TerrainType::Water | TerrainType::Riverbank => 1.0,
            TerrainType::Beach => 0.7,
            TerrainType::Wetland => 0.4,
            _ => 0.2,
        };

        // The run itself
        let run = match season {
            Season::Spring => 1.0,
            Season::Fall => 0.85,
            Season::Summer => 0.4,
            Season::Winter => 0.15,
        };

        let flow = Self::FISH_PER_PASS_AT_FULL_RUN * reach * run;

        if freezing {
            flow * 0.2
        } else {
            flow
        }
    }

    /// What a full spring run brings into one reach of river in one pass.
    ///
    /// Set so that a reach fished down to nothing is full again inside a year,
    /// most of it arriving in the two runs: a spring season of twenty-four days
    /// is twenty-eight or nine passes, which at this rate is a good half of
    /// what a reach holds. That is the shape of the thing - a river is empty
    /// enough to be worth nobody's time for most of the year and thick with
    /// fish twice in it, and a people who live on one arrange the rest of what
    /// they do around those two stretches.
    const FISH_PER_PASS_AT_FULL_RUN: f32 = 1.0;

    /// Regenerate resources based on climate and weather conditions
    /// Returns the amount regenerated
    pub fn regenerate(&mut self, temperature: f32, precipitation: f32, season_modifier: f32) -> u32 {
        self.regenerate_on(temperature, precipitation, season_modifier, false)
    }

    /// Regenerate, saying whether this is growing on broken ground
    pub fn regenerate_on(
        &mut self,
        temperature: f32,
        precipitation: f32,
        season_modifier: f32,
        cultivated: bool,
    ) -> u32 {
        let mut nowhere = Soil::for_terrain(TerrainType::Plains);
        self.regenerate_from_soil(
            temperature,
            precipitation,
            season_modifier,
            cultivated,
            &mut nowhere,
        )
    }

    /// Regenerate, drawing on the ground it is growing in.
    ///
    /// Growth used to be a number per species multiplied by the weather, with
    /// nothing taken out of the ground and nothing put back: a patch picked
    /// bare regrew as fast on bare rock as in river silt. What it can manage
    /// now is bounded by whichever of warmth, rain and nutrient is scarcest,
    /// and what it grows with, it takes.
    pub fn regenerate_from_soil(
        &mut self,
        temperature: f32,
        precipitation: f32,
        season_modifier: f32,
        cultivated: bool,
        soil: &mut Soil,
    ) -> u32 {
        self.regenerate_in_ground(
            temperature,
            precipitation,
            season_modifier,
            cultivated,
            soil,
        )
    }

    /// Regenerate, drawing on the ground, where `precipitation` is how wet
    /// that ground is rather than whether it happens to be raining.
    ///
    /// Plants drink from the soil, not from the sky directly. Passing the
    /// hour's rainfall in here meant every plant in the world was in drought on
    /// any day it was not actively raining, which cut growth to a fifth
    /// wherever a marsh and a dune were treated alike.
    pub fn regenerate_in_ground(
        &mut self,
        temperature: f32,
        precipitation: f32,
        season_modifier: f32,
        cultivated: bool,
        soil: &mut Soil,
    ) -> u32 {
        if self.amount >= self.how_heavy_a_crop_it_carries(soil.fertility(), cultivated) {
            return 0; // As heavy a crop as this ground will carry
        }

        // Base regeneration rate per tick (0-1 units).
        //
        // Wild food comes back slowly: a hedge of berries feeds a few people
        // and no more, which is what a settlement of a dozen lives on and what
        // a settlement of forty starves against. Ground that has been broken
        // and sown is a different matter - see `cultivated_multiplier`.
        let base_rate = match self.resource_type {
            // Renewable resources
            ResourceType::Wood => 0.01,       // Trees grow slowly
            ResourceType::Food => 0.025,      // Berries and fruit, in their own time
            ResourceType::Grain => 0.015,     // Wild grain is thin stuff
            ResourceType::Herbs => 0.04,      // Herbs grow quickly
            ResourceType::Flax => 0.03,
            ResourceType::Cotton => 0.03,
            ResourceType::Honey => 0.02,      // Bees produce honey steadily

            // Slow renewable
            ResourceType::Fish => 0.02,       // Fish populations regenerate

            // Water is fed by what carries it, which is worked out from the
            // ground it sits on rather than from a flat rate - see
            // `water_inflow`.
            ResourceType::Water => 0.0,

            // Non-renewable (mineral resources don't regenerate)
            ResourceType::Stone |
            ResourceType::Iron |
            ResourceType::Clay |
            ResourceType::Sand |
            ResourceType::Coal => 0.0,

            // Processed/finished goods don't regenerate naturally
            _ => 0.0,
        };

        if base_rate == 0.0 {
            return 0;
        }

        // Apply temperature modifier (most resources prefer moderate temps)
        let temp_modifier = match self.resource_type {
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs => {
                // Plants prefer 15-25°C
                if temperature >= 15.0 && temperature <= 25.0 {
                    1.5 // Ideal conditions
                } else if temperature >= 5.0 && temperature <= 35.0 {
                    1.0 // Acceptable
                } else if temperature < -10.0 || temperature > 40.0 {
                    0.1 // Extreme temps slow growth severely
                } else {
                    0.5 // Suboptimal
                }
            },
            ResourceType::Wood => {
                // Trees are hardier
                if temperature >= -5.0 && temperature <= 30.0 {
                    1.0
                } else {
                    0.3
                }
            },
            ResourceType::Cotton => {
                // Cotton prefers warmer climates
                if temperature >= 20.0 && temperature <= 30.0 {
                    1.5
                } else if temperature >= 15.0 {
                    1.0
                } else {
                    0.3
                }
            },
            _ => 1.0, // No temperature preference
        };

        // Apply precipitation modifier (water availability)
        let precip_modifier = match self.resource_type {
            ResourceType::Food | ResourceType::Grain | ResourceType::Herbs | ResourceType::Flax => {
                // Most crops need moderate precipitation
                if precipitation >= 0.4 && precipitation <= 0.8 {
                    1.5 // Good rainfall
                } else if precipitation >= 0.2 {
                    1.0 // Adequate
                } else if precipitation < 0.1 {
                    0.2 // Drought
                } else {
                    0.7 // Too dry or too wet
                }
            },
            ResourceType::Wood => {
                // Trees need regular water
                if precipitation >= 0.3 {
                    1.2
                } else {
                    0.5
                }
            },
            ResourceType::Cotton => {
                // Cotton prefers drier conditions
                if precipitation >= 0.2 && precipitation <= 0.5 {
                    1.3
                } else if precipitation > 0.8 {
                    0.6 // Too wet
                } else {
                    0.8
                }
            },
            _ => 1.0,
        };

        // What the ground can give, and how well this plant can get at it.
        // Capped at one: uptake helps a crop reach the best pace its kind is
        // capable of, it does not carry it past that.
        let nutrient_factor =
            (soil.fertility() * Self::uptake_multiplier(cultivated)).clamp(0.0, 1.0);

        if nutrient_factor <= 0.0 {
            return 0;
        }

        // And what is taking it before the farmer does. A field is the best
        // ground there is and everything else knows it: what a crop keeps is
        // what the weeds and the vermin leave. On unbroken ground this is one -
        // a meadow cannot get any weedier than it already is.
        let kept = if cultivated {
            soil.what_the_crop_keeps()
        } else {
            1.0
        };


        // Calculate total regeneration
        let regen_amount = base_rate
            * temp_modifier
            * precip_modifier
            * season_modifier
            * nutrient_factor
            * kept;

        // Carry the fraction over rather than rounding it away: wild food
        // regrows slowly enough that rounding to whole units each pass loses most
        // of it
        self.inflow_carried += regen_amount * 100.0;
        let regen_units = self.inflow_carried.floor();
        self.inflow_carried -= regen_units;

        // Add regenerated amount, capped at what this ground will carry
        let capacity = self.how_heavy_a_crop_it_carries(soil.fertility(), cultivated);
        let headroom = capacity.saturating_sub(self.amount);
        let actual_regen = (regen_units as u32).min(headroom);
        self.amount += actual_regen;

        // What grew, came out of the ground - and most of the plant stays in
        // the ground it grew in. Roots, stalk and leaf go back into this same
        // tile; only the part somebody carries away is gone from it.
        //
        // What grew in the water is a different matter: it takes nothing from
        // the bank and leaves nothing on it.
        if actual_regen > 0 && !self.resource_type.grows_in_water() {
            soil.draw(actual_regen as f32 * Soil::NUTRIENT_PER_UNIT_GROWN);
            soil.add_leaf_litter(actual_regen as f32 * Soil::RESIDUE_PER_UNIT_GROWN);
        }

        actual_regen
    }
}

/// Resource for tracking what's needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl Resource {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self {
            resource_type,
            amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_node_creation() {
        let pos = Position::new(5, 5);
        let node = ResourceNode::new(ResourceType::Wood, pos, 100);

        assert_eq!(node.resource_type, ResourceType::Wood);
        assert_eq!(node.position, pos);
        assert_eq!(node.amount, 100);
        assert_eq!(node.max_amount, 100);
    }

    #[test]
    fn test_resource_harvest() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Wood, pos, 100);

        let harvested = node.harvest(30);
        assert_eq!(harvested, 30);
        assert_eq!(node.amount, 70);

        // Try to harvest more than available
        let harvested = node.harvest(100);
        assert_eq!(harvested, 70); // Only 70 left
        assert_eq!(node.amount, 0);
        assert!(node.is_depleted());
    }

    #[test]
    fn test_resource_percentage() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Stone, pos, 100);

        assert!((node.percentage_remaining() - 100.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 50.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 0.0).abs() < 0.1);
    }
}
