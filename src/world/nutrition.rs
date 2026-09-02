// src/world/nutrition.rs
//! Nutrition system for food, preparation states, and spoilage.
//!
//! Implements a realistic nutrition model with:
//! - Three nutrient types: Energy (carbs/fats), Protein, Micronutrients
//! - Food preparation states affecting utilization and spoilage
//! - Time-based food spoilage with preservation methods

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use super::inventory::ItemType;

/// Types of nutrients that agents need
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NutrientType {
    /// Carbohydrates/Fats - powers movement and metabolism
    Energy,
    /// Repairs tissues, enzyme function
    Protein,
    /// Vitamins/Minerals pooled together
    Micronutrients,
}

/// Nutritional content of a food item
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct NutritionalContent {
    /// Energy value (carbs/fats) - satisfies hunger, powers movement
    pub energy: f32,
    /// Protein content - repairs tissues
    pub protein: f32,
    /// Micronutrient content (vitamins/minerals)
    pub micronutrients: f32,
    /// Water content (0.0-1.0) - contributes to thirst satisfaction
    pub water_content: f32,
}

impl NutritionalContent {
    pub fn new(energy: f32, protein: f32, micronutrients: f32, water: f32) -> Self {
        Self {
            energy,
            protein,
            micronutrients,
            water_content: water,
        }
    }

    /// Total nutritional value (for hunger satisfaction calculations)
    pub fn total(&self) -> f32 {
        self.energy + self.protein + self.micronutrients
    }

    /// Scale all values by a factor
    pub fn scale(&self, factor: f32) -> Self {
        Self {
            energy: self.energy * factor,
            protein: self.protein * factor,
            micronutrients: self.micronutrients * factor,
            water_content: self.water_content * factor,
        }
    }

}

/// What a fire does to a particular kind of food
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CookingOutcome {
    /// Raw flesh and hard grain: a fire is what makes these worth eating
    Improves,
    /// Food, but nothing a fire helps. Cooking it destroys it
    Ruins,
    /// Not food at all - there is nothing here to cook
    NotFood,
}

/// How big the piece is.
///
/// "Can they just absorb an entire side of beef? Should they not have to cut
/// it into smaller pieces so they can cook and eat it?" - and they could, and
/// they should. A kill dropped two-kilo lumps of `meat` that an agent ate raw,
/// one lump per bite, with nothing in between the carcass and the mouth.
///
/// A piece is not a property an item carries about with it; it is a thing you
/// can read off what the item *is*, the same way `World::will_this_dry` reads
/// off an id. A carcass is whole until somebody takes a knife to it, and what
/// comes off the knife says so in its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Piece {
    /// A carcass, or a fish out of the water with its head still on. Too much
    /// to put in your mouth and too much to put over a fire.
    Whole,
    /// A joint: what one person takes off a carcass to cook and eat now.
    Portion,
    /// Cut down thin, which is what you do to a thing you mean to keep rather
    /// than to eat.
    Strip,
    /// What arrives at about the size of a mouthful and needs no butchering
    /// at all: a berry, a handful of grain, a root.
    ///
    /// Distinct from a joint because thickness and bulk are two different
    /// questions and the first cut of this merged them. A berry has the bulk
    /// of a mouthful and the thickness of nothing, so it dries as fast as a
    /// strip and goes over a fire in the same quantities everything always
    /// did.
    Small,
}

impl Piece {
    /// What size the thing with this id is.
    ///
    /// Flesh is whole until it is cut. Everything else - a berry, a handful of
    /// grain, a root - arrives at about the size of a mouthful and needs no
    /// butchering, so it counts as a portion from the start.
    pub fn of(item_id: &str) -> Self {
        let id = item_id.to_lowercase();

        if id.ends_with("strips") {
            return Self::Strip;
        }
        if id.ends_with("portions") {
            return Self::Portion;
        }
        if Self::is_it_flesh(&id) {
            return Self::Whole;
        }

        Self::Small
    }

    /// Whether this is the kind of thing that comes off an animal in one
    /// piece and has to be taken apart.
    ///
    /// Kept here, in one place, rather than spread across the dozen call
    /// sites that want to know.
    pub fn is_it_flesh(item_id: &str) -> bool {
        let id = item_id.to_lowercase();
        let base = id
            .strip_prefix("cooked_")
            .or_else(|| id.strip_prefix("burnt_"))
            .unwrap_or(&id);

        // And a joint of it is still flesh, which the first cut of this got
        // wrong: it stripped the cooking prefix and not the cutting suffix,
        // so `meatportions` read as something that had never been an animal
        // and eating it raw carried no risk at all.
        let base = base
            .strip_suffix("portions")
            .or_else(|| base.strip_suffix("strips"))
            .unwrap_or(base);

        matches!(base, "meat" | "fish")
    }

    /// Whether a person can put this in their mouth as it is.
    ///
    /// Only about *shape*: a whole carcass has to come apart first. It is not
    /// the question of whether the thing is food at all - that is
    /// `is_this_food` below - and it was being used as though it were, which
    /// is how `eat_food_item` came to swallow wood.
    pub fn can_it_be_eaten(&self) -> bool {
        !matches!(self, Self::Whole)
    }

    /// Whether this will go over a fire.
    ///
    /// A whole carcass will not: what happens to a beast laid on a fire is
    /// that the outside chars and the inside stays raw, which is the same
    /// thing as not cooking it.
    pub fn can_it_be_cooked(&self) -> bool {
        !matches!(self, Self::Whole)
    }

    /// How many of these fit over the flames at once.
    ///
    /// This is what "smaller portions cook faster" comes to in a model where
    /// an action is a tick: cut small, and more of your supper is ready at the
    /// end of the same turn.
    ///
    /// A portion is deliberately the same five that everything was before
    /// this existed, so that a basket of berries cooks exactly as it always
    /// did and the only thing that has changed is what a carcass costs you.
    pub fn how_many_fit_over_a_fire(&self) -> u32 {
        match self {
            Self::Whole => 0,
            Self::Portion | Self::Small => 5,
            Self::Strip => 10,
        }
    }

    /// How long this has to lie in the sun before it is dry, in weathering
    /// passes.
    ///
    /// A strip is dry in a couple of days and a joint takes most of a week,
    /// which is the whole reason anybody bothers cutting a thing into strips
    /// rather than just quartering it.
    pub fn how_long_it_takes_to_dry(&self) -> u32 {
        match self {
            Self::Whole => u32::MAX,
            Self::Portion => 72,
            Self::Strip | Self::Small => 24,
        }
    }
}

/// What a mouthful of food nobody has recorded anything about is worth.
///
/// Most of what an agent picks up arrives with a full `FoodData` on it, but
/// some paths hand over a bare stack - a trade, an animal product - and a body
/// still has to be able to eat one. This is what `eat_food_item` credits it
/// with, and `find_best_food_to_eat` scores it by, so the two cannot differ:
/// they did, and the difference was that the search could not see such a stack
/// at all.
pub fn what_an_untracked_mouthful_is_worth() -> NutritionalContent {
    NutritionalContent::new(20.0, 5.0, 5.0, 0.3)
}

/// Whether a thing with this name is food at all.
///
/// The one answer for item ids, as `ItemType::is_it_food` is the one answer
/// for types - this is that question asked through `id_to_item_type`, so a
/// cooked joint and a cut portion resolve to what they were cut off. Anything
/// the name does not resolve to is not food: an unknown name is a thing
/// nobody has taught this model about, and guessing that it might be edible
/// from a substring is what `LOOKS_EDIBLE` did.
pub fn is_this_food(item_id: &str) -> bool {
    crate::agents::storage_integration::id_to_item_type(item_id)
        .is_some_and(|kind| kind.is_it_food())
}


/// Preparation state of food affecting utilization and spoilage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PreparationState {
    /// Unprocessed food - low utilization, normal spoilage
    #[default]
    Raw,
    /// Heat-processed - high utilization, slightly slower spoilage
    Cooked,
    /// Dehydrated - good retention, very slow spoilage
    Dried,
    /// Smoke-preserved - good retention, very slow spoilage
    Smoked,
    /// Salt-preserved - moderate retention, slow spoilage
    Salted,
    /// Acid/brine preserved - moderate retention, very slow spoilage
    Pickled,
    /// Mechanically processed (flour, etc.) - high utilization, faster spoilage
    Ground,
    /// Fermented (cheese, ale) - enhanced nutrients, slow spoilage
    Fermented,
    /// Burnt, scorched or curdled past saving - worthless and unsafe to eat
    Ruined,
}

impl PreparationState {
    /// Get utilization multiplier (how much nutrition is absorbed)
    /// Raw foods are hard to digest; cooking/processing improves absorption
    pub fn utilization_multiplier(&self) -> f32 {
        match self {
            Self::Raw => 0.35,       // 30-40% utilization
            Self::Cooked => 0.95,    // 90-100% - cooking unlocks nutrients
            Self::Dried => 0.85,     // Good retention, some loss
            Self::Smoked => 0.80,    // Slight nutrient loss from heat
            Self::Salted => 0.75,    // Some nutrient loss
            Self::Pickled => 0.70,   // Fermentation changes profile
            Self::Ground => 0.90,    // Improved digestibility
            Self::Fermented => 0.85, // Enhanced some nutrients, lost others
            Self::Ruined => 0.0,     // Nothing left to absorb
        }
    }

    /// Get spoilage rate multiplier (lower = longer lasting)
    /// What preparing a thing this way does to what it weighs.
    ///
    /// Drying takes the water out, and water is most of what meat weighs. This
    /// is not decoration: an agent can only carry so much, so a hunter who
    /// dries what he kills before he walks home carries more of the animal
    /// home. Preserving buys carrying capacity as well as time, and the two
    /// are the same thing seen from different ends - a deer left behind at the
    /// kill because it would not fit in the pack is exactly as wasted as a
    /// deer that rotted in it.
    ///
    /// Cooking drives some water off too, and less of it. Salting and pickling
    /// add as much as they take.
    pub fn what_it_does_to_the_weight(&self) -> f32 {
        match self {
            Self::Dried => 0.35,
            Self::Smoked => 0.5,
            Self::Cooked => 0.8,
            Self::Ground => 0.9,
            // Salt and brine put back roughly what they draw out
            Self::Salted | Self::Pickled | Self::Fermented => 1.0,
            Self::Raw | Self::Ruined => 1.0,
        }
    }

    pub fn spoilage_multiplier(&self) -> f32 {
        match self {
            Self::Raw => 1.0,        // Baseline
            Self::Cooked => 0.8,     // Slightly slower than raw
            Self::Dried => 0.05,     // 20x longer - very slow
            Self::Smoked => 0.1,     // 10x longer - very slow
            Self::Salted => 0.15,    // ~7x longer
            Self::Pickled => 0.1,    // 10x longer
            Self::Ground => 1.2,     // Faster (more surface area)
            Self::Fermented => 0.2,  // 5x longer
            Self::Ruined => 1.5,     // Already broken down; goes off fast
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Cooked => "Cooked",
            Self::Dried => "Dried",
            Self::Smoked => "Smoked",
            Self::Salted => "Salted",
            Self::Pickled => "Pickled",
            Self::Ground => "Ground",
            Self::Fermented => "Fermented",
            Self::Ruined => "Ruined",
        }
    }
}

/// Food-specific data attached to inventory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodData {
    /// Base nutritional values (before preparation adjustments)
    pub base_nutrition: NutritionalContent,
    /// Current preparation state
    pub preparation: PreparationState,
    /// Freshness (1.0 = fresh, 0.0 = spoiled)
    pub freshness: f32,
    /// Tick when item was created/harvested
    pub created_tick: u32,
    /// Base spoilage rate (ticks to reach 0 freshness from 1.0 at Raw state)
    pub base_spoilage_ticks: u32,
}

impl FoodData {
    pub fn new(
        base_nutrition: NutritionalContent,
        preparation: PreparationState,
        base_spoilage_ticks: u32,
        created_tick: u32,
    ) -> Self {
        Self {
            base_nutrition,
            preparation,
            freshness: 1.0,
            created_tick,
            base_spoilage_ticks,
        }
    }

    /// Get effective nutrition after preparation and freshness factors
    pub fn effective_nutrition(&self) -> NutritionalContent {
        let utilization = self.preparation.utilization_multiplier();
        let freshness_factor = self.freshness.max(0.0);

        NutritionalContent {
            energy: self.base_nutrition.energy * utilization * freshness_factor,
            protein: self.base_nutrition.protein * utilization * freshness_factor,
            micronutrients: self.base_nutrition.micronutrients * utilization * freshness_factor,
            water_content: self.base_nutrition.water_content * freshness_factor,
        }
    }

    /// Check if food is spoiled (inedible without consequences)
    pub fn is_spoiled(&self) -> bool {
        self.freshness <= 0.1
    }

    /// Whether this food has turned far enough to smell of it
    pub fn is_rotting(&self) -> bool {
        self.freshness < 0.4
    }

    /// How strongly this food gives itself away by smell, as a fraction of an
    /// agent's full smelling range.
    ///
    /// Cooking is the loudest thing a nose ever meets, which is why a camp can
    /// be smelled long before it is seen. Rot is the next loudest and carries
    /// most of the way. Anything raw and whole is close to silent.
    pub fn scent_strength(&self) -> f32 {
        if self.is_rotting() {
            // The further gone it is, the further it carries
            let rottenness = (0.4 - self.freshness.max(0.0)) / 0.4;
            return (0.35 + rottenness * 0.45).clamp(0.0, 0.8);
        }

        match self.preparation {
            // Food over a fire is unmistakable
            PreparationState::Cooked | PreparationState::Smoked => 1.0,
            // Prepared, but not hot
            PreparationState::Dried
            | PreparationState::Fermented
            | PreparationState::Ground
            | PreparationState::Salted
            | PreparationState::Pickled => 0.3,
            // Whole and raw: barely there
            PreparationState::Raw => 0.1,
            // Burnt carries, and is not a smell anyone follows
            PreparationState::Ruined => 0.5,
        }
    }

    /// Whether this was cooked when it should not have been, or burnt
    pub fn is_ruined(&self) -> bool {
        self.preparation == PreparationState::Ruined
    }

    /// Put this over a fire.
    ///
    /// Only raw flesh and grain gain anything from it. Everything else is
    /// spoiled by the heat, and so is anything that was already cooked or
    /// preserved: a second turn over the flames burns it. Returns what
    /// happened, so a caller can tell a meal from a mistake.
    pub fn cook(&mut self, outcome: CookingOutcome) -> CookingOutcome {
        let worth_cooking =
            outcome == CookingOutcome::Improves && self.preparation == PreparationState::Raw;

        if worth_cooking {
            self.preparation = PreparationState::Cooked;
            return CookingOutcome::Improves;
        }

        if outcome == CookingOutcome::NotFood {
            return CookingOutcome::NotFood;
        }

        self.preparation = PreparationState::Ruined;
        CookingOutcome::Ruins
    }

    /// Check if food is harmful (causes sickness if eaten)
    pub fn is_harmful(&self) -> bool {
        self.is_ruined() ||
        self.freshness <= 0.0 ||
        (self.freshness < 0.3 && self.preparation == PreparationState::Raw)
    }

    /// Get freshness description
    pub fn freshness_description(&self) -> &'static str {
        match self.freshness {
            f if f > 0.8 => "Fresh",
            f if f > 0.5 => "Good",
            f if f > 0.3 => "Stale",
            f if f > 0.1 => "Spoiling",
            _ => "Spoiled",
        }
    }

    /// Update freshness based on current tick
    /// How many ticks this has before it stops being food.
    ///
    /// What is left of its own clock: the whole of it at the pace its
    /// preparation lets it run, times how much of that is still to come.
    /// Freshness is *derived* from `created_tick` by `update_freshness`, so
    /// this needs no tick handed to it and cannot disagree with what the
    /// world last worked out.
    ///
    /// The one owner of the question. `Pit::how_long_this_would_keep` asks it
    /// and multiplies by what the hole is worth; `find_best_food_to_eat` asks
    /// it to decide what to eat first.
    pub fn how_long_this_has_left(&self) -> f32 {
        let spoils_in = self.base_spoilage_ticks as f32 / self.preparation.spoilage_multiplier();
        (spoils_in * self.freshness.clamp(0.0, 1.0)).max(0.0)
    }

    pub fn update_freshness(&mut self, current_tick: u32) {
        let elapsed = current_tick.saturating_sub(self.created_tick);
        let spoilage_rate = self.preparation.spoilage_multiplier();
        let effective_spoilage_ticks = (self.base_spoilage_ticks as f32 / spoilage_rate) as u32;

        if effective_spoilage_ticks > 0 {
            self.freshness = (1.0 - (elapsed as f32 / effective_spoilage_ticks as f32)).max(0.0);
        }
    }

    /// What a stack's clock becomes when fresh food is put on top of old.
    ///
    /// The older clock, near enough. Tipping this morning's berries into a
    /// basket that has been going over for a week does not give you a basket
    /// of this morning's berries — mould spreads, and it spreads quickly once
    /// it is there. The new food comes down to meet the old.
    ///
    /// A stack ages as a mixture of what is in it, so that a basket topped up
    /// a hundred times over a world is not pinned for ever at the tick its
    /// very first berry was picked. But once mould has actually manifested it
    /// takes the whole basket outright: nothing rescues a basket that has gone
    /// over by putting good fruit into it.
    ///
    /// Freshness is *derived* from `created_tick` by `update_freshness`, so
    /// the timer is the thing that has to move; the freshness is set as well
    /// only so that the stack reads right before the next pass.
    ///
    /// See ISSUES_FOUND #61 and #65.
    pub fn the_older_clock(self, other: Self, mine: u32, theirs: u32) -> Self {
        // The state that spoils faster, on the same reasoning: a stack is as
        // good as its worst part.
        let preparation = if self.preparation.spoilage_multiplier()
            >= other.preparation.spoilage_multiplier()
        {
            self.preparation
        } else {
            other.preparation
        };

        let gone_over = self.is_spoiled() || other.is_spoiled();

        let created_tick = if gone_over {
            self.created_tick.min(other.created_tick)
        } else {
            Self::weighted(self.created_tick, mine, other.created_tick, theirs)
        };

        let freshness = if gone_over {
            self.freshness.min(other.freshness)
        } else {
            let mine = mine.max(1) as f32;
            let theirs = theirs.max(1) as f32;
            (self.freshness * mine + other.freshness * theirs) / (mine + theirs)
        };

        Self {
            base_nutrition: self.base_nutrition,
            preparation,
            freshness,
            created_tick,
            base_spoilage_ticks: self.base_spoilage_ticks.min(other.base_spoilage_ticks),
        }
    }

    /// One tick blended into another by how much of each there is.
    fn weighted(mine: u32, how_much_of_mine: u32, theirs: u32, how_much_of_theirs: u32) -> u32 {
        let how_much_of_mine = how_much_of_mine.max(1) as u64;
        let how_much_of_theirs = how_much_of_theirs.max(1) as u64;

        let total = how_much_of_mine + how_much_of_theirs;
        let blended =
            (mine as u64 * how_much_of_mine + theirs as u64 * how_much_of_theirs) / total;

        blended as u32
    }

    /// Change preparation state (e.g., cooking raw meat)
    /// Resets created_tick to current tick for spoilage calculations
    pub fn set_preparation(&mut self, new_state: PreparationState, current_tick: u32) {
        self.preparation = new_state;
        self.created_tick = current_tick;
        // Freshness resets when food is prepared
        self.freshness = 1.0;
    }
}

/// Template for creating food items - defines base nutritional values
#[derive(Debug, Clone)]
pub struct FoodTemplate {
    pub base_nutrition: NutritionalContent,
    /// Ticks to spoil at raw state
    pub base_spoilage_ticks: u32,
    /// Default preparation state when created
    pub default_preparation: PreparationState,
}

/// Registry of food nutritional data
pub struct FoodDatabase {
    entries: BTreeMap<ItemType, FoodTemplate>,
}

impl Default for FoodDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl FoodDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            entries: BTreeMap::new(),
        };
        db.register_all_foods();
        db
    }

    pub fn get(&self, item_type: &ItemType) -> Option<&FoodTemplate> {
        self.entries.get(item_type)
    }

    /// Create FoodData for an item type
    pub fn create_food_data(&self, item_type: &ItemType, current_tick: u32) -> Option<FoodData> {
        self.entries.get(item_type).map(|template| {
            FoodData::new(
                template.base_nutrition,
                template.default_preparation,
                template.base_spoilage_ticks,
                current_tick,
            )
        })
    }

    /// Check if an item type is food
    pub fn is_food(&self, item_type: &ItemType) -> bool {
        self.entries.contains_key(item_type)
    }

    /// How many ticks a given number of days is, on the calendar this world
    /// actually keeps.
    ///
    /// Every one of these tables was written as a day-count and stored as a
    /// number of ticks at 1440 ticks to the day. The calendar was later put on
    /// a scale a life fits inside - `TICKS_PER_DAY` is 12, a season is
    /// twenty-four days and a year is 1,152 ticks - and the food tables were
    /// not brought with it. So meat, written down as lasting a day, lasted a
    /// hundred and twenty of them; grain written down as ten days lasted
    /// twelve and a half years.
    ///
    /// Nothing in this world spoiled, and everything downstream followed from
    /// that: nobody ever went hungry, a larder was insurance against nothing,
    /// and six of the nine preparation states had never been reachable
    /// because there was no reason to preserve anything. Stating the intent
    /// in days and converting here is what keeps the two from drifting apart
    /// again.
    /// A first cut of this used the day-counts the tables were written with -
    /// meat a day, berries a day and a half - and that turned out to be a
    /// different thing on this calendar than it was on the old one. A tick
    /// here is an *action*, not a minute: an agent gets twelve of them in a
    /// day, and walking out to a berry patch and back is thirty or forty. Food
    /// that lasts two days lasts less than the trip that fetches it, so
    /// nobody ever held a surplus, nothing was ever dried or buried, and a
    /// settlement lost a fifth of its people. These are on the scale of the
    /// season instead, which is the unit a store is actually against.
    const fn days(how_many: u32) -> u32 {
        how_many * crate::environment::seasons::TICKS_PER_DAY
    }

    fn register_all_foods(&mut self) {
        // === MEAT & FISH (High protein, moderate energy) ===

        // Meat - high protein, moderate energy, low micronutrients
        self.entries.insert(ItemType::Meat, FoodTemplate {
            base_nutrition: NutritionalContent::new(30.0, 50.0, 10.0, 0.6),
            base_spoilage_ticks: Self::days(10), // Under a season raw, and then it is carrion
            default_preparation: PreparationState::Raw,
        });

        // Fish - high protein, moderate energy, good micronutrients (omega-3, etc.)
        self.entries.insert(ItemType::Fish, FoodTemplate {
            base_nutrition: NutritionalContent::new(25.0, 45.0, 20.0, 0.7),
            base_spoilage_ticks: Self::days(6), // Fish spoils faster than anything else anybody catches
            default_preparation: PreparationState::Raw,
        });

        // === WHAT A HEDGEROW GIVES, BY SEASON ===

        // Wild leaf and shoot. Almost nothing in it by way of energy and a
        // great deal of what a body needs a little of, which is exactly what
        // a person who has lived on stored grain all winter is short of. It
        // does not keep at all: greens are a thing you eat where you pick
        // them.
        self.entries.insert(ItemType::Greens, FoodTemplate {
            base_nutrition: NutritionalContent::new(6.0, 3.0, 45.0, 0.9),
            base_spoilage_ticks: Self::days(3),
            default_preparation: PreparationState::Raw,
        });

        // The first roots and pods to come on. Better than greens and nothing
        // like a harvest, and they keep about as well as a berry does.
        self.entries.insert(ItemType::Roots, FoodTemplate {
            base_nutrition: NutritionalContent::new(30.0, 8.0, 20.0, 0.7),
            base_spoilage_ticks: Self::days(14),
            default_preparation: PreparationState::Raw,
        });

        // The mast: acorn, hazel, chestnut, walnut.
        //
        // **The top of the scale, and it is the reason the scale had a top.**
        // `physiology.rs` describes this database as running "from six
        // (spring greens) to eighty (fat and nuts)" - and until now nothing
        // in the world yielded one, so the eighty was a figure in a comment.
        // A nut is a third fat by weight and that is where the energy is.
        //
        // What it does that nothing else does is **keep**. Everything else a
        // settlement puts by has to be dried, salted, smoked or buried, and
        // the throughput of that is what caps the winter store - #241. A nut
        // in its shell wants nothing done to it: gathered in October, still
        // food in March. Two hundred and forty days is eight months, which is
        // an acorn kept dry and is longer than anything else in this table.
        self.entries.insert(ItemType::Nuts, FoodTemplate {
            base_nutrition: NutritionalContent::new(80.0, 20.0, 25.0, 0.05),
            base_spoilage_ticks: Self::days(240),
            default_preparation: PreparationState::Raw,
        });

        // === GRAINS (High energy, low protein) ===

        // Grain - high energy, low protein, moderate micronutrients
        self.entries.insert(ItemType::Grain, FoodTemplate {
            base_nutrition: NutritionalContent::new(60.0, 15.0, 15.0, 0.1),
            base_spoilage_ticks: Self::days(60), // Two seasons and a half, which is what a dry seed does
            default_preparation: PreparationState::Raw,
        });

        // Flour - grain opened up between two stones. A third more of what is
        // in a seed comes out in the eating once the husk is off it, and it
        // keeps rather less well than the whole seed does, which is the whole
        // reason to grind it when you mean to eat it rather than when you
        // bring it in.
        self.entries.insert(ItemType::Flour, FoodTemplate {
            base_nutrition: NutritionalContent::new(80.0, 16.0, 14.0, 0.1),
            base_spoilage_ticks: Self::days(30), // Rather less than the whole seed, which is why you grind it when you mean to eat it
            default_preparation: PreparationState::Raw,
        });

        // Bread - processed grain, already cooked
        self.entries.insert(ItemType::Bread, FoodTemplate {
            base_nutrition: NutritionalContent::new(55.0, 12.0, 10.0, 0.3),
            base_spoilage_ticks: Self::days(20),
            default_preparation: PreparationState::Cooked,
        });

        // === DAIRY (Balanced nutrition) ===

        // Milk - balanced nutrition, high water
        self.entries.insert(ItemType::Milk, FoodTemplate {
            base_nutrition: NutritionalContent::new(25.0, 20.0, 25.0, 0.85),
            base_spoilage_ticks: Self::days(4), // Sours faster than anything but fish
            default_preparation: PreparationState::Raw,
        });

        // Cheese - preserved milk, concentrated nutrients
        self.entries.insert(ItemType::Cheese, FoodTemplate {
            base_nutrition: NutritionalContent::new(40.0, 35.0, 20.0, 0.35),
            base_spoilage_ticks: Self::days(50), // Which is most of the point of making it
            default_preparation: PreparationState::Fermented,
        });

        // === SWEETS & BEVERAGES ===

        // Honey - pure energy, practically never spoils
        self.entries.insert(ItemType::Honey, FoodTemplate {
            base_nutrition: NutritionalContent::new(80.0, 0.0, 5.0, 0.2),
            base_spoilage_ticks: Self::days(3000), // Effectively never, and true of honey
            default_preparation: PreparationState::Raw, // Honey is special - raw but fully usable
        });

        // Ale - fermented grain beverage
        self.entries.insert(ItemType::Ale, FoodTemplate {
            base_nutrition: NutritionalContent::new(45.0, 5.0, 10.0, 0.9),
            base_spoilage_ticks: Self::days(80),
            default_preparation: PreparationState::Fermented,
        });

        // === GENERIC FOOD (Berries, foraged items) ===

        // Generic "Food" - represents berries, foraged items
        // High in micronutrients (vitamins from fruits/vegetables)
        self.entries.insert(ItemType::Food, FoodTemplate {
            base_nutrition: NutritionalContent::new(20.0, 5.0, 35.0, 0.8),
            base_spoilage_ticks: Self::days(12), // Berries off the bush. Half a season and they are jam on the inside of the pack
            default_preparation: PreparationState::Raw,
        });
    }
}

/// Agent's nutritional state - tracks nutrient reserves and deficiencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutritionalState {
    /// Energy reserves (carbs/fats) - 0-100
    /// Depletes with activity, restored by eating energy-rich foods
    pub energy_reserves: f32,

    /// Protein stores - 0-100
    /// Depletes slowly, needed for healing and maintenance
    pub protein_stores: f32,

    /// Micronutrient levels - 0-100
    /// Depletes very slowly, causes deficiency diseases when low
    pub micronutrient_level: f32,

    /// Ticks spent with protein below critical threshold
    pub ticks_protein_deficit: u32,

    /// Ticks spent with micronutrients below critical threshold
    pub ticks_micronutrient_deficit: u32,
}

impl Default for NutritionalState {
    fn default() -> Self {
        Self::new()
    }
}

impl NutritionalState {
    /// Critical threshold below which deficiency effects begin
    pub const DEFICIT_THRESHOLD: f32 = 20.0;
    /// Ticks of protein deficit before wasting begins (1 day)
    pub const PROTEIN_DEFICIT_ONSET: u32 = 1440;
    /// Ticks of micronutrient deficit before scurvy-like symptoms (3 days)
    pub const MICRONUTRIENT_DEFICIT_ONSET: u32 = 4320;

    pub fn new() -> Self {
        Self {
            energy_reserves: 80.0,
            protein_stores: 80.0,
            micronutrient_level: 80.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        }
    }

    /// Create with full nutrition (for well-fed agents)
    pub fn full() -> Self {
        Self {
            energy_reserves: 100.0,
            protein_stores: 100.0,
            micronutrient_level: 100.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        }
    }

    /// Consume nutrition from food
    pub fn consume(&mut self, nutrition: &NutritionalContent) {
        self.energy_reserves = (self.energy_reserves + nutrition.energy).min(100.0);
        self.protein_stores = (self.protein_stores + nutrition.protein).min(100.0);
        self.micronutrient_level = (self.micronutrient_level + nutrition.micronutrients).min(100.0);
    }

    /// Tick metabolism - depletes nutrients over time
    /// activity_level: 0.0 (resting) to 1.0 (intense activity)
    pub fn tick_metabolism(&mut self, activity_level: f32) {
        // Energy depletes faster with activity
        // Base: 0.02/tick at rest, up to 0.05/tick at full activity
        let energy_drain = 0.02 + (activity_level * 0.03);
        self.energy_reserves = (self.energy_reserves - energy_drain).max(0.0);

        // Protein depletes slowly (tissue maintenance)
        // ~100 ticks to deplete 0.5 points
        self.protein_stores = (self.protein_stores - 0.005).max(0.0);

        // Micronutrients deplete very slowly
        // ~100 ticks to deplete 0.2 points
        self.micronutrient_level = (self.micronutrient_level - 0.002).max(0.0);

        // Track deficiency duration
        if self.protein_stores < Self::DEFICIT_THRESHOLD {
            self.ticks_protein_deficit += 1;
        } else {
            self.ticks_protein_deficit = 0;
        }

        if self.micronutrient_level < Self::DEFICIT_THRESHOLD {
            self.ticks_micronutrient_deficit += 1;
        } else {
            self.ticks_micronutrient_deficit = 0;
        }
    }

    /// Check if experiencing protein deficiency (wasting)
    pub fn has_protein_deficiency(&self) -> bool {
        self.protein_stores < Self::DEFICIT_THRESHOLD &&
        self.ticks_protein_deficit > Self::PROTEIN_DEFICIT_ONSET
    }

    /// Check if experiencing micronutrient deficiency (scurvy-like)
    pub fn has_micronutrient_deficiency(&self) -> bool {
        self.micronutrient_level < Self::DEFICIT_THRESHOLD &&
        self.ticks_micronutrient_deficit > Self::MICRONUTRIENT_DEFICIT_ONSET
    }

    /// Get health penalty per tick from deficiencies
    pub fn deficiency_health_penalty(&self) -> f32 {
        let mut penalty = 0.0;

        // Protein deficiency causes wasting (progressive health loss)
        if self.has_protein_deficiency() {
            let days_in_deficit = (self.ticks_protein_deficit - Self::PROTEIN_DEFICIT_ONSET) as f32
                / 1440.0;
            // Scales from 0.05 to 0.15 health/tick over 3 days
            penalty += 0.05 * (1.0 + days_in_deficit.min(2.0));
        }

        // Micronutrient deficiency causes disease symptoms
        if self.has_micronutrient_deficiency() {
            let days_in_deficit = (self.ticks_micronutrient_deficit - Self::MICRONUTRIENT_DEFICIT_ONSET) as f32
                / 1440.0;
            // Scales from 0.02 to 0.06 health/tick over 3 days
            penalty += 0.02 * (1.0 + days_in_deficit.min(2.0));
        }

        penalty
    }


    /// Check if agent is starving (critical energy)
    pub fn is_starving(&self) -> bool {
        self.energy_reserves < 10.0
    }

    /// Get the most deficient nutrient type
    pub fn most_needed_nutrient(&self) -> NutrientType {
        if self.energy_reserves <= self.protein_stores &&
           self.energy_reserves <= self.micronutrient_level {
            NutrientType::Energy
        } else if self.protein_stores <= self.micronutrient_level {
            NutrientType::Protein
        } else {
            NutrientType::Micronutrients
        }
    }

}

/// Result of eating food
#[derive(Debug, Clone)]
pub enum EatResult {
    /// Successfully consumed food with given nutrition
    Success(NutritionalContent),
    /// Food was too spoiled to eat
    Spoiled,
    /// Ate bad food and got sick
    MadeSick(f32), // Health damage taken
    /// No food available
    NoFood,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preparation_utilization() {
        assert!(PreparationState::Cooked.utilization_multiplier() >
                PreparationState::Raw.utilization_multiplier());
        assert_eq!(PreparationState::Raw.utilization_multiplier(), 0.35);
        assert_eq!(PreparationState::Cooked.utilization_multiplier(), 0.95);
    }

    #[test]
    fn test_preparation_spoilage() {
        // Dried food should spoil much slower
        assert!(PreparationState::Dried.spoilage_multiplier() <
                PreparationState::Raw.spoilage_multiplier());
        // Ground food spoils faster
        assert!(PreparationState::Ground.spoilage_multiplier() >
                PreparationState::Raw.spoilage_multiplier());
    }

    #[test]
    fn test_food_data_effective_nutrition() {
        let food = FoodData::new(
            NutritionalContent::new(100.0, 50.0, 25.0, 0.5),
            PreparationState::Raw,
            1000,
            0,
        );

        let effective = food.effective_nutrition();
        // Raw utilization is 35%
        assert!((effective.energy - 35.0).abs() < 0.01);
        assert!((effective.protein - 17.5).abs() < 0.01);
    }

    #[test]
    fn test_food_data_cooked_nutrition() {
        let food = FoodData::new(
            NutritionalContent::new(100.0, 50.0, 25.0, 0.5),
            PreparationState::Cooked,
            1000,
            0,
        );

        let effective = food.effective_nutrition();
        // Cooked utilization is 95%
        assert!((effective.energy - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_food_freshness() {
        let mut food = FoodData::new(
            NutritionalContent::new(100.0, 50.0, 25.0, 0.5),
            PreparationState::Raw,
            1000, // Spoils in 1000 ticks
            0,
        );

        assert_eq!(food.freshness_description(), "Fresh");
        assert!(!food.is_spoiled());

        // Simulate 500 ticks passing (50% fresh)
        food.update_freshness(500);
        assert!((food.freshness - 0.5).abs() < 0.01);
        // At exactly 0.5, it's not > 0.5, so it's "Stale"
        assert_eq!(food.freshness_description(), "Stale");

        // Simulate 800 ticks - should be spoiling (20% fresh)
        food.update_freshness(800);
        assert!(food.freshness < 0.25);
        assert!(food.freshness > 0.1);
        assert_eq!(food.freshness_description(), "Spoiling");

        // Simulate 1000+ ticks - should be spoiled
        food.update_freshness(1100);
        assert!(food.is_spoiled());
    }

    #[test]
    fn test_dried_food_lasts_longer() {
        let mut raw_food = FoodData::new(
            NutritionalContent::new(50.0, 50.0, 10.0, 0.6),
            PreparationState::Raw,
            1000,
            0,
        );

        let mut dried_food = FoodData::new(
            NutritionalContent::new(50.0, 50.0, 10.0, 0.6),
            PreparationState::Dried,
            1000,
            0,
        );

        // After 1000 ticks, raw should be nearly spoiled
        raw_food.update_freshness(1000);
        dried_food.update_freshness(1000);

        // Dried food should still be mostly fresh (20x slower spoilage)
        assert!(dried_food.freshness > 0.9);
        assert!(raw_food.freshness < 0.1);
    }

    #[test]
    fn test_nutritional_state_metabolism() {
        let mut state = NutritionalState::full();

        // Tick 100 times at moderate activity
        for _ in 0..100 {
            state.tick_metabolism(0.5);
        }

        // Energy should have depleted most
        assert!(state.energy_reserves < 100.0);
        assert!(state.energy_reserves < state.protein_stores);
        assert!(state.protein_stores < 100.0);
    }

    #[test]
    fn test_protein_deficiency() {
        let mut state = NutritionalState {
            energy_reserves: 50.0,
            protein_stores: 10.0, // Below threshold
            micronutrient_level: 50.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        };

        // Not deficient yet - need time
        assert!(!state.has_protein_deficiency());

        // Simulate 1.5 days of deficit
        state.ticks_protein_deficit = 2000;
        assert!(state.has_protein_deficiency());
        assert!(state.deficiency_health_penalty() > 0.0);
    }

    #[test]
    fn test_food_database() {
        let db = FoodDatabase::new();

        assert!(db.is_food(&ItemType::Meat));
        assert!(db.is_food(&ItemType::Bread));
        assert!(!db.is_food(&ItemType::Wood));

        let meat = db.get(&ItemType::Meat).unwrap();
        assert!(meat.base_nutrition.protein > meat.base_nutrition.energy);

        let grain = db.get(&ItemType::Grain).unwrap();
        assert!(grain.base_nutrition.energy > grain.base_nutrition.protein);
    }

    #[test]
    fn test_consume_nutrition() {
        let mut state = NutritionalState::new();
        let initial_energy = state.energy_reserves;

        let nutrition = NutritionalContent::new(20.0, 10.0, 5.0, 0.5);
        state.consume(&nutrition);

        assert!((state.energy_reserves - (initial_energy + 20.0)).abs() < 0.01);
        assert!((state.protein_stores - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_most_needed_nutrient() {
        let state = NutritionalState {
            energy_reserves: 30.0,
            protein_stores: 50.0,
            micronutrient_level: 80.0,
            ticks_protein_deficit: 0,
            ticks_micronutrient_deficit: 0,
        };

        assert_eq!(state.most_needed_nutrient(), NutrientType::Energy);
    }

    #[test]
    fn test_honey_never_spoils() {
        let db = FoodDatabase::new();
        let honey = db.get(&ItemType::Honey).unwrap();

        // Honey outlasts the person carrying it, which is what "never spoils"
        // means on a calendar where a life is about eight thousand ticks.
        // This used to be written as a bare number of ticks against the old
        // 1440-tick day - see `FoodDatabase::days`.
        let a_life = 8000;
        assert!(honey.base_spoilage_ticks > a_life * 2);

        let mut food = db.create_food_data(&ItemType::Honey, 0).unwrap();
        food.update_freshness(a_life / 4);

        // Still fresh a couple of years on
        assert!(food.freshness > 0.9);
        assert_eq!(food.freshness_description(), "Fresh");
    }
}

#[cfg(test)]
mod one_answer_to_what_is_food {
    use super::{is_this_food, FoodDatabase};
    use crate::world::ItemType;

    /// `ItemType::is_it_food` is a static list and `FoodDatabase` is a runtime
    /// table, and the first is only trustworthy while it matches the second.
    /// Every resource type in the model is put to both.
    #[test]
    fn every_food_type_has_a_template() {
        let db = FoodDatabase::new();
        for what in crate::world::ResourceType::all() {
            let Some(kind) = crate::agents::storage_integration::id_to_item_type(
                &format!("{what:?}").to_lowercase(),
            ) else {
                continue;
            };
            assert_eq!(
                kind.is_it_food(),
                db.is_food(&kind),
                "{kind:?} is food to one of these and not the other"
            );
        }

        // And the thirteen the database carries, named, so that dropping one
        // from either side fails here rather than quietly starving somebody.
        for kind in [
            ItemType::Food, ItemType::Meat, ItemType::Fish, ItemType::Greens,
            ItemType::Roots, ItemType::Nuts, ItemType::Grain, ItemType::Flour,
            ItemType::Bread, ItemType::Milk, ItemType::Cheese, ItemType::Honey,
            ItemType::Ale,
        ] {
            assert!(kind.is_it_food(), "{kind:?} should be food");
            assert!(db.is_food(&kind), "{kind:?} should have a template");
        }
        for kind in [ItemType::Wood, ItemType::Stone, ItemType::Iron, ItemType::Clay] {
            assert!(!kind.is_it_food(), "{kind:?} is not food");
            assert!(!db.is_food(&kind), "{kind:?} should have no template");
        }
    }

    /// The name-level question agrees with the type-level one, through the
    /// cooking prefix and the cutting suffix alike.
    #[test]
    fn a_cooked_joint_is_still_food_and_a_stone_is_still_not() {
        for id in ["food", "grain", "greens", "roots", "nuts", "fish", "meat",
                   "bread", "cooked_meat", "meatportions", "fishportions"] {
            assert!(is_this_food(id), "{id} should be food");
        }

        // And what the flora system drops, which is where this test earned its
        // keep: it was written asserting that *nothing* is called "berries",
        // on the strength of the word appearing only in prose in the decision
        // layer. `PlantDrop` names sixty-two things a plant can give and
        // "berries" is one of them - so the assertion was wrong, and it failed
        // the moment the name table was taught the drops. A guard that only
        // ever agrees with you is not a guard.
        for id in ["berries", "apples", "potatoes", "wheat", "mushrooms", "cabbage"] {
            assert!(is_this_food(id), "the flora system drops {id}");
        }

        // Not everything a plant gives is supper.
        for id in ["bark", "resin", "straw", "plant_fiber", "rose_petals",
                   "poison_mushrooms", "cotton_seeds"] {
            assert!(!is_this_food(id), "{id} is not a meal");
        }
        for id in ["wood", "stone", "clay", "bowl", "basket", "flax", "iron", "spear"] {
            assert!(!is_this_food(id), "{id} is not food");
        }
    }
}
