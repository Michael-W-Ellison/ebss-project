// src/world/soil.rs
//! What is under a plant's feet.
//!
//! Growth in this simulation used to be a number per species multiplied by the
//! weather. Nothing was ever taken out of the ground and nothing was ever put
//! back, so a patch of berries picked bare regrew exactly as fast on bare rock
//! in a drought as in river silt after a wet spring.
//!
//! A tile now carries soil: a stock of nutrients that plants draw on, and two
//! pools of dead matter waiting to become more of it. The two pools are the
//! whole of the decay model - what breaks down fast and what breaks down slowly
//! - and which one a thing lands in is decided by how dense it was when it was
//! alive. Leaves, dung and spoiled food are soft; trunks, branches and bone are
//! woody.
//!
//! How fast either turns into nutrients depends on how wet the ground is. A
//! tree that falls in a swamp is gone in a few years. The same tree in a desert
//! is still lying there.

use serde::{Deserialize, Serialize};

use super::TerrainType;

/// The state of the ground on one tile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Soil {
    /// What a plant can draw on now, 0.0 to 1.0
    pub nutrients: f32,

    /// Leaves, dung, spoiled food: open, wet, and quick to go
    pub leaf_litter: f32,

    /// Trunks, branches, bone: dense, dry inside, and slow
    pub woody_litter: f32,

    /// Fresh dung lying on the surface: what a midden smells of.
    ///
    /// Separate from the litter because it is a different question. Litter is
    /// about what the ground will have to eat next year; this is about what
    /// the ground smells of today, and it goes off much faster than it breaks
    /// down. What it turns into is in the litter already - this is only the
    /// smell of it, and the seeds in it.
    #[serde(default)]
    pub fouling: f32,

    /// Seeds passed through somebody, waiting on the ground they were dropped
    /// on.
    ///
    /// "Seeds from the plants they have eaten should sprout." They come out
    /// with the waste, sit until the fouling has broken down enough to be soil
    /// rather than muck, and then come up.
    #[serde(default)]
    pub seeds_dropped: f32,
}

impl Soil {
    /// The most nutrient any ground holds at once.
    ///
    /// Nutrients above this run off or blow away rather than banking up, which
    /// is what stops a settlement turning one field into an infinite larder by
    /// piling refuse on it for ten thousand ticks.
    pub const MAX_NUTRIENTS: f32 = 1.0;

    /// How much litter one tile can hold before more of it simply will not fit
    pub const MAX_LITTER: f32 = 4.0;

    /// What share of the matter in litter ends up in the ground rather than
    /// going off into the air.
    ///
    /// The rest is lost. This is the number that makes a closed loop
    /// impossible: everything that goes round comes back a little smaller.
    pub const KEPT_FROM_ROT: f32 = 0.6;

    /// What one unit of standing crop takes out of the ground to grow.
    ///
    /// Growth and return are two ends of the same arithmetic, so they live
    /// beside each other. `ResourceNode::regenerate_in_ground` draws this per
    /// unit it grows; everything that puts matter back is measured against it.
    pub const NUTRIENT_PER_UNIT_GROWN: f32 = 0.0015;

    /// The litter left by one unit of food that was eaten.
    ///
    /// A body keeps some of what it eats and passes the rest. Set so that a
    /// meal returns about three fifths of the nutrient that growing it took,
    /// once rot has taken its own cut - the loop turns, and loses on every
    /// turn, which is what a loop of this kind does.
    pub const WASTE_PER_MEAL: f32 = Self::NUTRIENT_PER_UNIT_GROWN;

    /// And by one unit that spoiled before anybody could eat it.
    ///
    /// Nothing took a share of this on the way, so all of what it was grown
    /// with is still in it.
    pub const WASTE_PER_SPOILED: f32 =
        Self::NUTRIENT_PER_UNIT_GROWN / Self::KEPT_FROM_ROT;

    /// What a plant leaves in the ground it grew in, per unit of crop.
    ///
    /// The largest return of the three, and the one that was missing longest.
    /// A plant takes up far more than ends up in the part anybody carries
    /// away: the roots, the stalk and the leaves stay where they grew and go
    /// back into that same tile. Only the grain leaves the field.
    ///
    /// Set so that about half of what the plant took up stays put, which is
    /// roughly where a cereal sits. Without it the model treated every plant
    /// as though the whole of it were carried off, and a settlement's fields
    /// fell from 0.53 fertility to 0.04 inside thirty thousand ticks however
    /// much its people put back at the other end.
    pub const RESIDUE_PER_UNIT_GROWN: f32 =
        Self::NUTRIENT_PER_UNIT_GROWN * 0.5 / Self::KEPT_FROM_ROT;

    /// What one fish is worth to the ground it is buried in.
    ///
    /// This is the number that makes a fishery different in kind from every
    /// other food a settlement has. Everything else on the land is a return:
    /// a crop meal gives back some part of what growing the crop took out of
    /// that same country, and rot takes its cut on the way, so the best the
    /// land can do is lose slowly. A fish was not grown here. It was grown at
    /// sea and fed on a whole catchment, and it walked into the settlement's
    /// reach on its own. Burying it in a field is the one way the country a
    /// settlement farms gets richer than it was.
    ///
    /// Set at forty times what a unit of crop returns, which is about what a
    /// fish is against a turnip and is why people who had rivers buried fish
    /// with the seed corn long before anybody could say what nitrogen was.
    pub const NUTRIENT_PER_FISH: f32 = Self::WASTE_PER_SPOILED * 40.0;

    /// What is left of a fish, eaten, for the ground to have.
    ///
    /// A third of it went as guts at the waterside before anybody carried it
    /// home, and a body keeps a quarter of what it eats. This is the rest.
    pub const WASTE_PER_FISH_EATEN: f32 = Self::NUTRIENT_PER_FISH * 0.4;

    /// And of one that turned before anybody got to it.
    ///
    /// Nothing took a share on the way except the guts, so almost all of what
    /// the sea put into it is still there. A glut of fish nobody could eat or
    /// dry in time is the single richest thing that ever lands on a field.
    pub const WASTE_PER_FISH_SPOILED: f32 = Self::NUTRIENT_PER_FISH * 0.65;

    /// Whether a thing in somebody's pack came out of the water.
    ///
    /// Matched on the name, because that is all an untracked stack carries.
    pub fn came_out_of_the_water(item_id: &str) -> bool {
        let name = item_id.to_lowercase();
        name.contains("fish") || name.contains("salmon") || name.contains("trout")
    }

    /// What one unit of this, eaten, leaves for the ground.
    pub fn waste_from_eating(item_id: &str) -> f32 {
        if Self::came_out_of_the_water(item_id) {
            Self::WASTE_PER_FISH_EATEN
        } else {
            Self::WASTE_PER_MEAL
        }
    }

    /// What one unit of this, spoiled and never eaten, leaves for the ground.
    pub fn waste_from_spoilage(item_id: &str) -> f32 {
        if Self::came_out_of_the_water(item_id) {
            Self::WASTE_PER_FISH_SPOILED
        } else {
            Self::WASTE_PER_SPOILED
        }
    }

    /// The ground as it starts, before anything has lived or died on it.
    ///
    /// River silt and marsh are rich, mountain and sand are all but bare, and
    /// woodland sits somewhere in between with a century of leaf fall already
    /// in it.
    pub fn for_terrain(terrain: TerrainType) -> Self {
        let (nutrients, leaf_litter) = match terrain {
            TerrainType::Wetland => (0.85, 1.2),
            TerrainType::Riverbank => (0.80, 0.6),
            TerrainType::Forest => (0.65, 1.5),
            TerrainType::Meadow => (0.60, 0.4),
            TerrainType::Plains => (0.50, 0.3),
            TerrainType::Farmland => (0.55, 0.2),
            TerrainType::Hills => (0.35, 0.2),
            TerrainType::Beach => (0.15, 0.1),
            TerrainType::Mountain => (0.10, 0.05),
            TerrainType::Desert => (0.08, 0.02),
            TerrainType::Water => (0.30, 0.2),
        };

        Self {
            nutrients,
            leaf_litter,
            woody_litter: match terrain {
                TerrainType::Forest => 0.8,
                TerrainType::Wetland => 0.3,
                _ => 0.05,
            },
            fouling: 0.0,
            seeds_dropped: 0.0,
        }
    }

    /// How wet this ground is, from the country it is in and the weather over
    /// it.
    ///
    /// This is the single thing that decides how fast anything lying on it
    /// breaks down.
    pub fn humidity(terrain: TerrainType, precipitation: f32) -> f32 {
        let ground = match terrain {
            TerrainType::Water | TerrainType::Wetland => 1.0,
            TerrainType::Riverbank => 0.85,
            TerrainType::Forest => 0.7,
            TerrainType::Meadow => 0.55,
            TerrainType::Farmland => 0.5,
            TerrainType::Plains => 0.45,
            TerrainType::Beach => 0.4,
            TerrainType::Hills => 0.35,
            TerrainType::Mountain => 0.25,
            TerrainType::Desert => 0.05,
        };

        (ground + precipitation.clamp(0.0, 1.0) * 0.3).clamp(0.0, 1.0)
    }

    /// Put soft matter on the ground: leaves, dung, spoiled food, offal
    pub fn add_leaf_litter(&mut self, amount: f32) {
        self.leaf_litter = (self.leaf_litter + amount).clamp(0.0, Self::MAX_LITTER);
    }

    /// What somebody has just passed, with whatever was in it.
    ///
    /// Goes into the litter like anything else soft, and additionally leaves
    /// the two things that make a midden a midden: a smell, and seeds.
    pub fn somebody_voided_here(&mut self, amount: f32) {
        self.add_leaf_litter(amount);
        self.fouling = (self.fouling + amount).clamp(0.0, Self::AS_FOUL_AS_IT_GETS);
        self.seeds_dropped =
            (self.seeds_dropped + amount * Self::WHAT_COMES_THROUGH_WHOLE).clamp(0.0, 1.0);
    }

    /// How foul ground can get before more of it makes no difference.
    pub const AS_FOUL_AS_IT_GETS: f32 = 2.0;

    /// Ground fouler than this is ground people will not sit on.
    pub const FOUL_ENOUGH_TO_WALK_AWAY_FROM: f32 = 0.35;

    /// What share of what goes in comes out able to grow.
    ///
    /// Most of a berry is digested. The pips are not, which is the whole
    /// mechanism: a hedge grows where the birds sit.
    pub const WHAT_COMES_THROUGH_WHOLE: f32 = 0.05;

    /// How much seed has to be lying on a tile before anything comes up.
    ///
    /// Two units of waste on the same ground, which is about what a camp
    /// leaves on one tile over a season. This was five times higher to begin
    /// with, and measured over six thousand ticks it meant that of a thousand
    /// tiles carrying seed not one carried enough: people move about, and no
    /// single tile ever caught up.
    pub const ENOUGH_TO_COME_UP: f32 = 0.1;

    /// And how far the fouling has to have gone off before the ground under it
    /// is soil rather than muck.
    ///
    /// This is the wait the specification describes: "over time the waste
    /// should break down and seeds from the plants they have eaten should
    /// sprout". Nothing grows out of a fresh midden.
    pub const BROKEN_DOWN_ENOUGH_TO_GROW_IN: f32 = 0.1;

    /// Whether this ground is fouled enough that people will not stay on it.
    pub fn is_foul(&self) -> bool {
        self.fouling >= Self::FOUL_ENOUGH_TO_WALK_AWAY_FROM
    }

    /// Whether what was dropped here is ready to come up.
    pub fn ready_to_sprout(&self) -> bool {
        self.seeds_dropped >= Self::ENOUGH_TO_COME_UP
            && self.fouling <= Self::BROKEN_DOWN_ENOUGH_TO_GROW_IN
            && self.nutrients > 0.0
    }

    /// Take the seed off the ground, because it has come up.
    pub fn it_came_up(&mut self) -> f32 {
        std::mem::take(&mut self.seeds_dropped)
    }

    /// Put dense matter on the ground: trunks, branches, bone
    pub fn add_woody_litter(&mut self, amount: f32) {
        self.woody_litter = (self.woody_litter + amount).clamp(0.0, Self::MAX_LITTER);
    }

    /// Everything lying on this tile waiting to break down
    pub fn litter(&self) -> f32 {
        self.leaf_litter + self.woody_litter
    }

    /// Break down what is lying here, turning it into nutrient.
    ///
    /// `humidity` runs 0.0 to 1.0 and does most of the work: dry ground barely
    /// rots at all. Density does the rest - soft matter goes an order of
    /// magnitude faster than wood, which is why a fallen tree outlasts the
    /// leaves that fell with it by decades.
    ///
    /// Returns how much nutrient was released, which is mostly of interest to
    /// tests.
    pub fn decay(&mut self, humidity: f32, ticks: f32) -> f32 {
        /// Share of soft litter that goes per tick in ideal conditions
        const LEAF_RATE: f32 = 0.0006;

        /// And of wood, which is dense enough to keep the wet out of its middle
        const WOOD_RATE: f32 = 0.00004;

        // Rot needs water. Bone dry ground holds what falls on it more or less
        // indefinitely, which is why a desert keeps its dead.
        let wetness = humidity.clamp(0.0, 1.0);
        let activity = wetness * wetness;

        if activity <= 0.0 {
            return 0.0;
        }

        // A midden stops smelling long before it stops being there. This runs
        // an order of magnitude faster than the rot underneath it, which is
        // why the ground people walked away from a season ago is ground they
        // will sit on again.
        const FOULING_RATE: f32 = 0.006;
        self.fouling = (self.fouling - self.fouling * FOULING_RATE * activity * ticks).max(0.0);

        let from_leaves = (self.leaf_litter * LEAF_RATE * activity * ticks).min(self.leaf_litter);
        let from_wood = (self.woody_litter * WOOD_RATE * activity * ticks).min(self.woody_litter);

        self.leaf_litter -= from_leaves;
        self.woody_litter -= from_wood;

        // Some of it is lost to the air rather than staying in the ground
        let released = (from_leaves + from_wood) * Self::KEPT_FROM_ROT;
        let before = self.nutrients;
        self.nutrients = (self.nutrients + released).min(Self::MAX_NUTRIENTS);

        self.nutrients - before
    }

    /// Take nutrient out of the ground, returning how much was actually there
    /// to take
    pub fn draw(&mut self, wanted: f32) -> f32 {
        let taken = wanted.min(self.nutrients).max(0.0);
        self.nutrients -= taken;
        taken
    }

    /// How well fed this ground is, as a fraction of what it could hold
    pub fn fertility(&self) -> f32 {
        (self.nutrients / Self::MAX_NUTRIENTS).clamp(0.0, 1.0)
    }
}

impl Default for Soil {
    fn default() -> Self {
        Self::for_terrain(TerrainType::Plains)
    }
}
