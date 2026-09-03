// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, BehaviorNode, NodeType, DriveState, DriveType, Memory, GoalManager, Preferences, GoalWorldState};
use crate::core::planning::{ActionPlan, PlanActionType, Planner, PlanStep, ActionOutcome};
use crate::environment::{Action, ActionResult};
use std::collections::BTreeMap;

use super::senses::Senses;
use super::body::Body;
use super::physiology;
use super::provision;
use super::skills::Skills;
use super::emotions::{EmotionState, EmotionSource, RelationshipMap};
use crate::core::traits::TraitSet;
use super::gossip::KnowledgeBase;
use super::observational_learning::ObservationalLearning;
use super::transport::TransportSystem;
use crate::environment::TechnologyKnowledge;
use crate::world::nutrition::{FoodData, NutritionalState, EatResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub random_weights: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { random_weights: true }
    }
}

/// Item stored in inventory with quantity and optional fill level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: String,
    pub quantity: u32,
    /// Weight per unit (in kg)
    pub weight_per_unit: f32,
    /// For containers: current fill level (e.g., water in waterskin)
    pub fill_level: Option<f32>,
    /// For containers: maximum capacity
    pub max_capacity: Option<f32>,
    /// Current durability (0.0 to max_durability, None = no durability tracking)
    pub current_durability: Option<f32>,
    /// Maximum durability for this item
    pub max_durability: Option<f32>,
    /// Quality level of this item
    pub quality: Option<super::Quality>,
    /// Food-specific data (nutrition, freshness, preparation state)
    pub food_data: Option<FoodData>,
}

impl InventoryItem {
    pub fn new(item_id: String, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            weight_per_unit: 1.0, // Default weight
            fill_level: None,
            max_capacity: None,
            current_durability: None,
            max_durability: None,
            quality: None,
            food_data: None,
        }
    }

    pub fn new_with_weight(item_id: String, quantity: u32, weight_per_unit: f32) -> Self {
        Self {
            item_id,
            quantity,
            weight_per_unit,
            fill_level: None,
            max_capacity: None,
            current_durability: None,
            max_durability: None,
            quality: None,
            food_data: None,
        }
    }

    pub fn new_container(item_id: String, quantity: u32, capacity: f32) -> Self {
        Self {
            item_id,
            quantity,
            weight_per_unit: 0.5, // Containers are typically lighter
            fill_level: Some(0.0),
            max_capacity: Some(capacity),
            current_durability: None,
            max_durability: None,
            quality: None,
            food_data: None,
        }
    }

    /// Create a new tool/item with durability and quality
    pub fn new_with_durability(
        item_id: String,
        quantity: u32,
        durability: f32,
        quality: super::Quality,
    ) -> Self {
        Self {
            item_id,
            quantity,
            weight_per_unit: 2.0, // Tools are typically heavier
            fill_level: None,
            max_capacity: None,
            current_durability: Some(durability),
            max_durability: Some(durability),
            quality: Some(quality),
            food_data: None,
        }
    }

    /// Create a food item with nutritional data
    pub fn new_food(
        item_id: String,
        quantity: u32,
        weight_per_unit: f32,
        food_data: FoodData,
    ) -> Self {
        Self {
            item_id,
            quantity,
            weight_per_unit,
            fill_level: None,
            max_capacity: None,
            current_durability: None,
            max_durability: None,
            quality: None,
            food_data: Some(food_data),
        }
    }

    /// Check if this is a food item
    /// Take another lot of the same thing onto this stack.
    ///
    /// A stack that has food data keeps it when something without any is
    /// added, and picks one up if it had none. An item with no `food_data`
    /// **never rots**, so letting a dataless lot swallow a real stack made
    /// food that could sit in a pit for the life of the world without ever
    /// going off - which is where the several hundred units of immortal food
    /// in ISSUES_FOUND #45 came from.
    ///
    /// # What this deliberately does not do
    ///
    /// It does not merge the two clocks. It keeps the clock of whichever
    /// stack was already there, which is what the bare
    /// `quantity += other.quantity` it replaces did.
    ///
    /// That is **not** because the clock rule is wrong. Fresh food tipped
    /// into a basket that has been going over ought to come down to meet it -
    /// mould spreads - and `FoodData` had a `the_older_clock` written and unit
    /// tested for exactly that. It was measured at thirty-two worlds a side
    /// and held back:
    ///
    /// | | before | with the clock rule |
    /// |---|---|---|
    /// | food eaten | 9,703 | **4,638** (t = -8.4) |
    /// | people alive | 55.5 | 48.0 (t = -2.2) |
    /// | winter store | 320 | 105 |
    ///
    /// A settlement ate **less than half as much**, and the loss does not
    /// turn up in any waste column - eaten plus waste falls from 12,874 to
    /// 6,692, so something like six thousand units leave the ledger without
    /// being eaten, rotting or being left anywhere. Every other change in that
    /// batch was measured null with the clock rule off, so the rule is
    /// responsible and the hole is not explained. Shipping a rule that loses
    /// half a settlement's food to an unexplained sink is worse than shipping
    /// a stack that lies about its age.
    ///
    /// See ISSUES_FOUND #61, and the task that goes with it.
    pub fn absorb(&mut self, other: InventoryItem) {
        let mine = self.quantity;
        let theirs = other.quantity;
        self.quantity += other.quantity;

        self.food_data = match (self.food_data.take(), other.food_data) {
            (Some(clock), Some(other_clock)) => {
                Some(clock.the_older_clock(other_clock, mine, theirs))
            }
            (Some(only), None) | (None, Some(only)) => Some(only),
            (None, None) => None,
        };
    }

    /// Whether this is something to eat.
    ///
    /// Asked of what the thing *is*, not of whether anybody happened to record
    /// its freshness. This was `food_data.is_some()`, which made a traded
    /// stack of grain - rebuilt without its nutrition, see #232 - not food,
    /// while the pack count beside it said it was.
    pub fn is_food(&self) -> bool {
        crate::world::nutrition::is_this_food(&self.item_id)
    }

    /// Check if food is spoiled
    pub fn is_spoiled(&self) -> bool {
        self.food_data.as_ref().map(|f| f.is_spoiled()).unwrap_or(false)
    }

    /// Update food freshness based on current tick
    pub fn update_food_freshness(&mut self, current_tick: u32) {
        if let Some(ref mut food) = self.food_data {
            food.update_freshness(current_tick);
        }
    }

    /// Check if this is a container
    pub fn is_container(&self) -> bool {
        self.max_capacity.is_some()
    }

    /// Get fill percentage (0.0 to 1.0)
    pub fn fill_percentage(&self) -> f32 {
        match (self.fill_level, self.max_capacity) {
            (Some(fill), Some(max)) if max > 0.0 => fill / max,
            _ => 0.0,
        }
    }

    /// Add liquid to container, returns amount actually added
    pub fn fill(&mut self, amount: f32) -> f32 {
        match (self.fill_level.as_mut(), self.max_capacity) {
            (Some(fill), Some(max)) => {
                let space_available = max - *fill;
                let amount_to_add = amount.min(space_available);
                *fill += amount_to_add;
                amount_to_add
            }
            _ => 0.0,
        }
    }

    /// Remove liquid from container, returns amount actually removed
    pub fn drain(&mut self, amount: f32) -> f32 {
        match self.fill_level.as_mut() {
            Some(fill) => {
                let amount_to_remove = amount.min(*fill);
                *fill -= amount_to_remove;
                amount_to_remove
            }
            _ => 0.0,
        }
    }

    /// Get durability as percentage (0.0 to 1.0)
    pub fn durability_percentage(&self) -> f32 {
        match (self.current_durability, self.max_durability) {
            (Some(current), Some(max)) if max > 0.0 => current / max,
            _ => 1.0, // No durability tracking = always "full"
        }
    }

    /// Check if item is broken (0 durability)
    pub fn is_broken(&self) -> bool {
        match self.current_durability {
            Some(dur) => dur <= 0.0,
            None => false, // No durability = never broken
        }
    }


    /// Repair item to full durability
    pub fn repair(&mut self) {
        if let (Some(current), Some(max)) = (self.current_durability.as_mut(), self.max_durability) {
            *current = max;
        }
    }

    /// Damage item by amount
    pub fn damage(&mut self, amount: f32) {
        if let Some(current) = self.current_durability.as_mut() {
            *current = (*current - amount).max(0.0);
        }
    }

    /// Get total weight of this item stack
    /// Includes container contents (water, etc.) if applicable
    pub fn total_weight(&self) -> f32 {
        // What has been done to it tells on what it weighs. Drying takes the
        // water out and water is most of what meat weighs, so a hunter who
        // dries a kill before walking home carries more of the animal home -
        // see `PreparationState::what_it_does_to_the_weight`.
        let each = self.weight_per_unit * self.how_much_lighter_it_is();
        let base_weight = each * self.quantity as f32;

        // Add liquid weight if this is a filled container
        // Water weighs ~1 kg per liter
        let liquid_weight = self.fill_level.unwrap_or(0.0);

        base_weight + liquid_weight
    }

    /// What being dried or cooked or smoked has taken off this thing's weight.
    pub fn how_much_lighter_it_is(&self) -> f32 {
        self.food_data
            .as_ref()
            .map(|food| food.preparation.what_it_does_to_the_weight())
            .unwrap_or(1.0)
    }
}

/// Agent inventory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// How much food this pack has refused for want of room, in units.
    ///
    /// A refused `add_item` returns `false` and almost every caller ignores
    /// it, so the food simply stops existing. That is a real sink and it had
    /// no counter anywhere: a whole batch's worth of missing food was traced
    /// to it. See ISSUES_FOUND #65.
    #[serde(default)]
    pub what_would_not_go_in: u32,

    /// Items stored by item_id
    /// What is in it, in a stable order.
    ///
    /// A `BTreeMap` rather than a `BTreeMap`, and not for speed. Twenty-two
    /// places iterate this map and several of them pick a *best* - the best
    /// food to eat, the tool that helps most, what can be spared - so when two
    /// candidates tie, the winner is whichever the iterator reached first. A
    /// `BTreeMap` orders by a hash seeded per process, so that answer changed
    /// between runs of the same binary on the same seed.
    ///
    /// Measured: with the dice seeded, five tests still came and went across
    /// three runs. An inventory has no business having an order that depends
    /// on which process is looking at it.
    items: std::collections::BTreeMap<String, InventoryItem>,
    /// Maximum number of item stacks
    pub max_slots: usize,
    /// Maximum weight that can be carried
    pub max_weight: f32,
    /// Current total weight
    pub current_weight: f32,
}

impl Inventory {
    pub fn new(max_slots: usize, max_weight: f32) -> Self {
        Self {
            items: std::collections::BTreeMap::new(),
            max_slots,
            max_weight,
            current_weight: 0.0,
            what_would_not_go_in: 0,
        }
    }

    /// Add an item to inventory
    /// Returns true if successful, false if no room or too heavy
    pub fn add_item(&mut self, item: InventoryItem) -> bool {
        // Check slot limit
        if self.items.len() >= self.max_slots && !self.items.contains_key(&item.item_id) {
            return false; // No room for new item type
        }

        // Check weight limit
        let item_weight = item.total_weight();
        if self.current_weight + item_weight > self.effective_max_weight() {
            // What will not go in is not carried, and something ought to know
            // it happened - see `what_would_not_go_in`.
            if item.is_food() {
                self.what_would_not_go_in += item.quantity;
            }
            return false; // Too heavy
        }

        // Add or stack item, and weigh the pack by what is actually in it.
        //
        // The weight added is **not** the incoming item's weight when the two
        // stacks merge, because merging can change what the whole stack is:
        // `absorb` settles the preparation, and preparation is what decides
        // weight - a dried stack weighs a third of the same thing raw. Adding
        // only the newcomer's weight left `current_weight` reading low, and
        // the next `recalculate_weight` corrected it in one jump. If that jump
        // put the pack over its limit, **every subsequent `add_item` returned
        // false and the food was silently destroyed**, because almost every
        // caller ignores the bool. See ISSUES_FOUND #65.
        if let Some(existing) = self.items.get_mut(&item.item_id) {
            let before = existing.total_weight();
            existing.absorb(item);
            self.current_weight += existing.total_weight() - before;
        } else {
            self.current_weight += item_weight;
            self.items.insert(item.item_id.clone(), item);
        }

        true
    }

    /// Remove an item from inventory
    pub fn remove_item(&mut self, item_id: &str, quantity: u32) -> Option<InventoryItem> {
        if let Some(item) = self.items.get_mut(item_id) {
            if item.quantity >= quantity {
                item.quantity -= quantity;
                let removed = InventoryItem {
                    item_id: item_id.to_string(),
                    quantity,
                    weight_per_unit: item.weight_per_unit,
                    fill_level: item.fill_level,
                    max_capacity: item.max_capacity,
                    current_durability: item.current_durability,
                    max_durability: item.max_durability,
                    quality: item.quality,
                    food_data: item.food_data.clone(),
                };

                // Update weight
                self.current_weight -= removed.total_weight();

                if item.quantity == 0 {
                    self.items.remove(item_id);
                }

                return Some(removed);
            }
        }
        None
    }

    /// Get an item from inventory
    pub fn get_item(&self, item_id: &str) -> Option<&InventoryItem> {
        self.items.get(item_id)
    }

    /// Get a mutable item from inventory
    pub fn get_item_mut(&mut self, item_id: &str) -> Option<&mut InventoryItem> {
        self.items.get_mut(item_id)
    }

    /// Get total water available from all containers
    pub fn get_total_water(&self) -> f32 {
        self.items.values()
            .filter(|item| item.is_container())
            .filter_map(|item| item.fill_level)
            .sum()
    }

    /// Drink water from any available container
    pub fn drink_water(&mut self, amount: f32) -> f32 {
        let mut remaining = amount;

        for item in self.items.values_mut() {
            if item.is_container() && remaining > 0.0 {
                let drained = item.drain(remaining);
                remaining -= drained;
            }
        }

        let drunk = amount - remaining;

        // Update weight (water weighs 1kg per liter)
        self.current_weight -= drunk;

        drunk // Return amount actually drunk
    }

    /// Fill containers from a water source
    /// Total water currently carried in containers, in litres
    pub fn available_water(&self) -> f32 {
        self.items
            .values()
            .filter(|item| item.is_container())
            .filter_map(|item| item.fill_level)
            .sum()
    }

    pub fn fill_containers(&mut self, available_water: f32) -> f32 {
        let mut remaining = available_water;

        for item in self.items.values_mut() {
            if item.is_container() && remaining > 0.0 {
                let filled = item.fill(remaining);
                remaining -= filled;
            }
        }

        let filled = available_water - remaining;

        // Update weight (water weighs 1kg per liter)
        self.current_weight += filled;

        filled // Return amount actually filled
    }

    /// Check if inventory has item with minimum quantity
    pub fn has_item(&self, item_id: &str, min_quantity: u32) -> bool {
        self.items.get(item_id)
            .map(|item| item.quantity >= min_quantity)
            .unwrap_or(false)
    }

    /// Check if inventory has food items (used by Survivalist trait)
    /// Food items are identified by containing "food", "meat", "bread", etc. in their ID
    pub fn has_food(&self, min_quantity: u32) -> bool {
        let food_keywords = ["food", "meat", "bread", "fish", "fruit", "vegetable", "grain", "meal"];
        self.items.iter()
            .filter(|(id, _)| {
                let lower_id = id.to_lowercase();
                food_keywords.iter().any(|kw| lower_id.contains(kw))
            })
            .map(|(_, item)| item.quantity)
            .sum::<u32>() >= min_quantity
    }

    /// Count quantity of a specific item by id
    pub fn count_item(&self, item_id: &str) -> u32 {
        self.items.get(item_id)
            .map(|item| item.quantity)
            .unwrap_or(0)
    }

    /// Get total count of all items in inventory
    pub fn total_items(&self) -> u32 {
        self.items.values().map(|item| item.quantity).sum()
    }

    /// Get all items
    pub fn get_all_items(&self) -> &std::collections::BTreeMap<String, InventoryItem> {
        &self.items
    }

    /// The same, to be changed rather than read. Draining a vessel is the
    /// caller this was wanting.
    pub fn get_all_items_mut(&mut self) -> &mut std::collections::BTreeMap<String, InventoryItem> {
        &mut self.items
    }

    /// Recalculate total weight from all items
    pub fn recalculate_weight(&mut self) {
        self.current_weight = self.items.values()
            .map(|item| item.total_weight())
            .sum();
    }

    /// Check if inventory is overweight
    pub fn is_overweight(&self) -> bool {
        self.current_weight > self.effective_max_weight()
    }

    /// How much this pack can hold, baskets and all.
    ///
    /// A person carries what their arms hold. A person with a basket carries
    /// what their arms hold and what the basket holds, which is most of the
    /// reason anybody ever wove one - see `making::WEAVE_A_BASKET`.
    /// What a basket holds is counted once, in `max_weight`.
    ///
    /// This used to add baskets and leather bags *again*, on top of what
    /// `Agent::take_up_the_cart` had already put into `max_weight` through the
    /// transport system - which maps a basket to `TransportType::Backpack` at
    /// thirty. So one basket was worth thirty as a thing on your back and
    /// another twenty as a thing in your pack: fifty from one basket, and two
    /// owners for the question "what does a container add".
    ///
    /// Measured across six worlds, agents carried 43.5 kg against a stated
    /// capacity of 34.7 - permanently over their own limit - because
    /// `add_item` gated on this figure while `weight_percentage` and every
    /// report read `max_weight`. Two answers, and the looser one held the
    /// door.
    ///
    /// `Transport` owns it now: it has the whole table - capacity, speed,
    /// durability, twenty-odd kinds of carrier - and `take_up_the_cart` puts
    /// what is in the pack onto the back each turn.
    pub fn effective_max_weight(&self) -> f32 {
        self.max_weight
    }

    /// Get weight capacity remaining
    pub fn weight_capacity_remaining(&self) -> f32 {
        (self.effective_max_weight() - self.current_weight).max(0.0)
    }

    /// Get weight as percentage of max (0.0 to 1.0+)
    pub fn weight_percentage(&self) -> f32 {
        if self.max_weight == 0.0 {
            0.0
        } else {
            self.current_weight / self.max_weight
        }
    }

}

impl Default for Inventory {
    fn default() -> Self {
        // Twenty slots and a nominal allowance. What an *agent* can carry is
        // not this: it is what two hands hold plus what it has to put things
        // in, worked out by `update_inventory_capacity_from_transport` and
        // brought up to date every turn by `take_up_the_cart`. A bare
        // `Inventory` has no body and no basket, so it has nothing to work it
        // out from.
        Self::new(20, 100.0)
    }
}

/// What share of a grown appetite a body of this many years wants.
///
/// The specification's own table, year by year: a fifth of an adult's food and
/// water until four, then rising a twentieth a year to ten, then faster, and a
/// full share from sixteen.
///
/// This replaces a guess. The reserve used to be sized by life stage in five
/// crude bands, and what a body burned was the three-quarter power of that -
/// which is the right shape for real animals and is not what was asked for.
/// Here the figure is the food and water a body of that age needs, so it sizes
/// the burn directly, and the reserve and the stomach with it. Everybody still
/// starves in three weeks, whatever size they are; a small body simply has
/// less to go without.
pub fn what_a_body_this_age_eats(years: u32) -> f32 {
    match years {
        0..=3 => 0.20,
        4 => 0.25,
        5 => 0.30,
        6 => 0.35,
        7 => 0.40,
        8 => 0.45,
        9 => 0.50,
        10 => 0.55,
        11 => 0.60,
        12 => 0.70,
        13 => 0.80,
        // "Age 14-15: 90%" and then "Age 16+: 100%", so fifteen falls in a gap
        // between the last child band and the first adult one. The last child
        // band runs to the adult boundary: a fifteen-year-old is nine tenths
        // of a grown worker on the capability table and is fed as one.
        14..=15 => 0.90,
        _ => 1.00,
    }
}


/// What a body of this many years can bring to moving, carrying and working.
///
/// The specification's table, as a share of a grown adult's ten: one at two
/// years, climbing to ten at sixteen, holding until forty and falling away
/// after. At seventy it is over.
///
/// This was written once and hung on nothing, and was then deleted as dead
/// code in the sweep of #93 - which was the right call for the code and left
/// the model with a six-year-old who carried what a grown man carried, walked
/// as fast, worked as hard and hit as heavily. The *only* thing age decided
/// was appetite, and a body that eats a fifth as much while doing a full day's
/// work is not a child, it is a bargain.
///
/// It is hung on four things now, which are the three the sentence above names
/// and the one it implies: what two hands hold, how fast a body walks, what a
/// trip brings back, and what a blow is worth.
pub fn what_a_body_this_age_can_do(years: u32) -> f32 {
    let out_of_ten = match years {
        0..=1 => 0,
        2..=3 => 1,
        4..=5 => 2,
        6..=7 => 3,
        8..=9 => 4,
        10..=11 => 5,
        12 => 6,
        13 => 7,
        14 => 8,
        15 => 9,
        16..=39 => 10,
        40..=49 => 9,
        50..=54 => 8,
        55..=59 => 7,
        60..=64 => 6,
        _ => 5,
    };
    out_of_ten as f32 / 10.0
}

/// Life stages of an agent, in years.
///
/// The bands are the ones the lifecycle is specified in, and what separates
/// them is how far from a grown person somebody of that age may be:
///
/// - **0-5** must be with a parent at all times. Under two the parent has a
///   hand occupied carrying them.
/// - **6-10** must stay within sight of the camp or of some adult.
/// - **11-15** must stay within an hour's walk of the camp or of some adult.
/// - **16+** is a functional adult with no restrictions.
/// - **70** is death from old age.
///
/// These used to be counted in turns - infancy to five hundred of them,
/// adulthood at two and a half thousand - on a calendar where a year was
/// eleven hundred turns and a whole life eight of them. A year is
/// `TICKS_PER_YEAR` turns now and a life is seventy years.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifeStage {
    /// Under six: with a parent at all times
    Infant,
    /// Six to ten: within sight of the camp or an adult
    Child,
    /// Eleven to fifteen: within an hour's walk
    Adolescent,
    /// Sixteen to forty-nine: a functional adult
    Adult,
    /// Fifty and over, when what a body can do starts falling away
    Elderly,
}

impl LifeStage {
    /// The year of life each stage begins in.
    pub const KEPT_IN_ARMS_UNTIL: u32 = 6;
    pub const KEPT_IN_SIGHT_UNTIL: u32 = 11;
    pub const KEPT_WITHIN_AN_HOUR_UNTIL: u32 = 16;
    pub const STRENGTH_STARTS_GOING_AT: u32 = 50;

    /// Get life stage based on age in turns.
    pub fn from_age(age: u32) -> Self {
        Self::from_years(age / crate::environment::seasons::TICKS_PER_YEAR)
    }

    /// The same, from years already counted.
    pub fn from_years(years: u32) -> Self {
        if years < Self::KEPT_IN_ARMS_UNTIL {
            LifeStage::Infant
        } else if years < Self::KEPT_IN_SIGHT_UNTIL {
            LifeStage::Child
        } else if years < Self::KEPT_WITHIN_AN_HOUR_UNTIL {
            LifeStage::Adolescent
        } else if years < Self::STRENGTH_STARTS_GOING_AT {
            LifeStage::Adult
        } else {
            LifeStage::Elderly
        }
    }

    /// Check if agent can reproduce at this stage
    pub fn can_reproduce(&self) -> bool {
        matches!(self, LifeStage::Adolescent | LifeStage::Adult | LifeStage::Elderly)
    }

    /// Whether somebody at this stage of life could stand and fight a wolf.
    ///
    /// The very young cannot, and that is the commonest reason in the world
    /// for fighting not to be an option. An old one can - not well, and
    /// `Agent::own_strength` already knows what their body is worth - but
    /// they can raise a hand.
    pub fn can_fight(&self) -> bool {
        matches!(
            self,
            LifeStage::Adolescent | LifeStage::Adult | LifeStage::Elderly
        )
    }

    /// Get learning rate multiplier for this stage
    pub fn learning_rate(&self) -> f32 {
        match self {
            LifeStage::Infant => 2.0,
            LifeStage::Child => 1.5,
            LifeStage::Adolescent => 1.2,
            LifeStage::Adult => 1.0,
            LifeStage::Elderly => 0.8,
        }
    }

    /// Get fertility multiplier for this stage
    pub fn fertility_multiplier(&self) -> f32 {
        match self {
            LifeStage::Infant | LifeStage::Child => 0.0,
            LifeStage::Adolescent => 0.7,
            LifeStage::Adult => 1.0,
            LifeStage::Elderly => 0.3,
        }
    }

    /// What a body of this size leaves on the ground when it stops.
    ///
    /// Soft matter and bone, as a share of what a grown adult leaves. Measured
    /// against a meal: a person is worth some tens of meals of matter, and
    /// most of what a settlement grows passes through people, so this is a
    /// smaller return than eating but not a negligible one.
    pub fn body_left_behind(&self) -> (f32, f32) {
        let share = match self {
            LifeStage::Infant => 0.15,
            LifeStage::Child => 0.4,
            LifeStage::Adolescent => 0.75,
            LifeStage::Adult => 1.0,
            LifeStage::Elderly => 0.85,
        };

        // Forty meals' worth of soft matter and half as much again in bone
        let soft = 40.0 * crate::world::Soil::WASTE_PER_SPOILED * share;
        (soft, soft * 0.5)
    }

    /// How long this body can go on what it has stored, as a share of what a
    /// grown adult can manage.
    ///
    /// A famine does not take a settlement evenly. A grown adult carries fat
    /// and muscle worth weeks; a small child carries days of it, and burns
    /// through what there is faster for its size. The old and the very young
    /// go first, and they go long before anybody in their prime, which is why
    /// a hungry year shows up as a missing generation rather than as a smaller
    /// one.
    pub fn hunger_reserve(&self) -> f32 {
        // The middle of each band, for the places that still only know a
        // stage. `what_a_body_this_age_eats` is the real answer and takes the
        // years - a five-year-old and a one-year-old are not the same size.
        match self {
            LifeStage::Infant => what_a_body_this_age_eats(3),
            LifeStage::Child => what_a_body_this_age_eats(8),
            LifeStage::Adolescent => what_a_body_this_age_eats(13),
            LifeStage::Adult => 1.0,
            LifeStage::Elderly => 1.0,
        }
    }
}

/// What is wrong with somebody, and until when.
///
/// "Eating raw meat, spending time near dead bodies or fresh waste, and eating
/// spoiling food should have a chance to cause sickness."
///
/// There was no illness at all in this model before this. The only health
/// consequence anywhere in it was a flat ten damage for eating something past
/// `is_harmful`, taken in one tick and over with, so a settlement could live
/// on raw flesh and sleep in its own midden and never know the difference.
///
/// An ailment is deliberately a thing that *lasts*. What makes sickness matter
/// in a settlement is not the damage, it is the days: somebody laid up is
/// somebody not gathering, not building, and eating anyway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ailment {
    /// What brought it on, in the agent's own terms - which is also the key it
    /// learns against, so that somebody who has been ill twice off raw flesh
    /// can decide to stop eating raw flesh.
    pub from: String,
    /// When it started.
    pub since: u32,
    /// And when it will have run its course.
    pub until: u32,
    /// How badly, from nought to one.
    ///
    /// This is the live figure and a remedy moves it - see
    /// `Agent::take_a_remedy`. What it started at is `at_its_worst`.
    pub severity: f32,

    /// What it was at its worst, before anybody treated it.
    ///
    /// Kept so that easing can be capped against the illness itself rather
    /// than against whatever it has already been eased to, which is the
    /// difference between a remedy that helps and a remedy that, taken often
    /// enough, cures. Nothing in this model cures anything.
    #[serde(default)]
    pub at_its_worst: f32,
}

impl Ailment {
    /// Whether this has run its course by now.
    pub fn is_over(&self, now: u32) -> bool {
        now >= self.until
    }

    /// How long somebody is laid up for, at the mildest and the worst.
    ///
    /// Two days to a week and a half, on a calendar of twelve ticks to the
    /// day. Long enough to cost a settlement work, short enough that it is
    /// not simply a slower way of dying.
    pub const THE_SHORTEST_IT_LASTS: u32 = 2 * crate::environment::seasons::TICKS_PER_DAY;
    pub const THE_LONGEST_IT_LASTS: u32 = 10 * crate::environment::seasons::TICKS_PER_DAY;

    /// How bad it was before anybody did anything about it.
    ///
    /// A saved game from before remedies existed has nought here, and nought
    /// means "as bad as it is now" rather than "no illness at all".
    pub fn at_its_worst(&self) -> f32 {
        if self.at_its_worst > 0.0 {
            self.at_its_worst
        } else {
            self.severity
        }
    }

    /// What sort of trouble this is, so a remedy can be right or wrong for it.
    ///
    /// **Every illness in this model is a bad gut** - raw flesh, food on the
    /// turn, foul ground - which is not a shortcut so much as a fact about
    /// what laid people up before anybody boiled water. The match is written
    /// out rather than defaulted so that the day something else makes
    /// somebody ill, this is a place that has to be looked at.
    pub fn what_sort_it_is(&self) -> crate::environment::remedies::WhatARemedyEases {
        use crate::environment::remedies::WhatARemedyEases as Eases;

        match self.from.as_str() {
            // What people were actually ill with before anybody boiled
            // water: it went in at one end.
            Agent::OFF_RAW_FLESH | Agent::OFF_FOOD_ON_THE_TURN | Agent::OFF_FOUL_GROUND => {
                Eases::TheGut
            }
            // A soaking in the cold that turned into something.
            Agent::OFF_A_SOAKING => Eases::TheChest,
            // And the pre-antibiotic killer: a wound that did not close.
            Agent::OFF_A_WOUND_THAT_TURNED => Eases::TheSkin,
            // Anything new: the gut, because most of it is.
            _ => Eases::TheGut,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub health: f32,
    pub energy: f32, // 0.0 to 100.0, depletes without food
    pub position: (i32, i32, i32),
    pub age: u32,
    pub life_stage: LifeStage,
    pub max_age: u32,
    pub is_alive: bool,
    pub last_ate_tick: u32, // Track when agent last ate
    pub ticks_without_food: u32, // Count starvation duration
    pub last_drank_tick: u32, // Track when agent last drank water

    /// The body: water, stomach, gut and reserve, on a clock of minutes.
    ///
    /// This is the truth about whether an agent is hungry, thirsty, weakened
    /// or dead of either. The two turn counters above are kept only so the
    /// interface and older tests have something to read; they decide nothing.
    /// See `agents::physiology`.
    #[serde(default)]
    pub physiology: physiology::Physiology,

    /// Winters this agent has counted its way through.
    #[serde(default)]
    pub winters_seen: provision::WintersSeen,

    /// What this agent last made of its own provisions, and the winter coming.
    #[serde(default)]
    pub what_the_larder_says: Option<provision::WhatIsPutBy>,

    /// What the last turn's work cost, which is what the body burned doing it.
    ///
    /// "Increased physical activity should increase the rate at which hunger
    /// and thirst increase." The action matrix already prices every action in
    /// energy; this carries that figure through to the body, and is spent when
    /// the body reads it.
    #[serde(default)]
    pub effort_this_turn: f32,
    pub ticks_without_water: u32, // Count dehydration duration
    /// What this body has to pass, waiting to be left on the ground.
    ///
    /// Everything eaten used to leave the world for good, so a settlement was
    /// a one-way pump from the soil into nothing. What a body takes in, most
    /// of it comes out again somewhere.
    #[serde(default)]
    pub waste_carried: f32,
    /// What is wrong with this one, if anything.
    #[serde(default)]
    pub ailing: Option<Ailment>,
    /// What last took health off this one, in a word.
    ///
    /// Causes of death used to be worked out *after* the fact, by asking a
    /// corpse whether it was hungry - and by then the hunger has been eaten
    /// away, the exposure has cleared, and the answer is no. Measured over
    /// eight worlds, **70% of every death in this model came out as "unknown
    /// cause"**: a settlement could not say what killed its people.
    ///
    /// So each thing that takes health says so as it takes it, and the
    /// reckoning reads what is written rather than guessing from what is left.
    #[serde(default)]
    pub what_last_took_health: Option<String>,
    /// How much salt this one has drunk and not yet got rid of.
    ///
    /// "If they do so it should increase their hydration drive more over time
    /// even if it seems to temporarily satiate it." So it is not the drink
    /// that costs, it is the days after it.
    #[serde(default)]
    pub salt_in_me: f32,

    /// A wound that has not closed, nought to one.
    ///
    /// Opened by `take_damage` and closing on its own clock. While it is
    /// open it can turn - see `Agent::OFF_A_WOUND_THAT_TURNED` - which is
    /// what actually killed people who survived the animal.
    #[serde(default)]
    pub an_open_wound: f32,
}

impl AgentState {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();
        // Max age varies between 9000-11000 ticks
        // Seventy years, and that is the end of it. There is no spread: the
        // specification says "Age 70: Death from old age", and everything
        // before it - the strength curve, the appetite curve - is written
        // against that one figure.
        let max_age = crate::environment::seasons::YEARS_BEFORE_OLD_AGE_TAKES_YOU
            * crate::environment::seasons::TICKS_PER_YEAR;

        Self {
            health: 100.0,
            energy: 100.0,
            position: (0, 0, 0),
            age: 0,
            life_stage: LifeStage::Infant,
            max_age,
            is_alive: true,
            last_ate_tick: 0,
            ticks_without_food: 0,
            last_drank_tick: 0,
            physiology: physiology::Physiology::new(),
            winters_seen: provision::WintersSeen::default(),
            what_the_larder_says: None,
            effort_this_turn: 0.0,
            ticks_without_water: 0,
            waste_carried: 0.0,
            ailing: None,
            what_last_took_health: None,
            salt_in_me: 0.0,
            an_open_wound: 0.0,
        }
    }

    /// Age the agent by one tick
    pub fn age_tick(&mut self, current_tick: u32) {
        self.age_tick_with_modifier(current_tick, 1.0);
    }

    /// Age the agent by one tick with an energy multiplier (e.g., for pregnancy)
    pub fn age_tick_with_modifier(&mut self, current_tick: u32, energy_multiplier: f32) {
        if !self.is_alive {
            return;
        }

        self.age += 1;
        self.life_stage = LifeStage::from_age(self.age);

        // === SURVIVAL MECHANICS ===
        // Track starvation
        self.ticks_without_food = current_tick.saturating_sub(self.last_ate_tick);

        // Track dehydration (faster than starvation - 3 days vs 7 days)
        self.ticks_without_water = current_tick.saturating_sub(self.last_drank_tick);

        // What this body has stored to go on. A grown adult carries three weeks
        // of it; a small child carries days. A famine therefore takes the young
        // and the old first and the people in their prime last, without anybody
        // having written that down.
        let reserve = self.what_i_eat_for_my_age();
        self.physiology.now_a_body_of(reserve);

        // Two hours of living, at whatever the last turn's work cost. Water,
        // the stomach, the gut and the reserve all move on the body's own
        // clock; see `agents::physiology`. The turn counters above are kept
        // only for the interface and for older tests to read, and are derived
        // rather than counted so they cannot disagree with the body.
        let effort = std::mem::take(&mut self.effort_this_turn);
        self.physiology
            .advance(physiology::MINUTES_PER_TURN, effort * energy_multiplier);

        // Going short of water is felt before it is fatal: the bands in
        // `Physiology::capability` take a quarter off everything the agent can
        // do at each of three-quarters, half and a quarter. Nought is death.
        if self.physiology.died_of_thirst() {
            self.lose_health(self.health, "dehydration");
        } else if self.physiology.is_parched() {
            // Not damage so much as the body starting to fail at the edges
            self.lose_health(0.15 * (1.0 - self.physiology.capability()), "thirst");
        }

        // And the reserve running out is starvation. Three weeks for an adult.
        if self.physiology.starved() {
            self.lose_health(self.health, "starvation");
        } else if self.physiology.is_wasting() {
            self.lose_health(0.1 / reserve, "hunger");
        }

        // Energy depletion (normal metabolism), made worse by working thirsty
        let base_energy_loss = 0.05 * energy_multiplier;
        let energy_loss = base_energy_loss / self.physiology.capability().max(0.25);
        self.energy = (self.energy - energy_loss).max(0.0);

        // When energy is depleted, health starts decreasing too
        if self.energy <= 0.0 {
            self.lose_health(0.05, "exhaustion");
        }

        // Check for death from old age
        if self.age >= self.max_age {
            self.what_last_took_health = Some("old age".to_string());
            self.is_alive = false;
        }

        // Check for death from injury/starvation/dehydration
        if self.health <= 0.0 {
            self.is_alive = false;
        }
    }

    /// Take damage
    pub fn take_damage(&mut self, amount: f32) {
        // A blow leaves something open, and how open depends on how hard it
        // was. This is the one place a wound is opened, because
        // `take_damage` is the one place a blow lands - hunger and cold go
        // through `lose_health` and leave nothing to fester.
        let opened = (amount / Self::WHAT_A_BLOW_HAS_TO_BE_TO_LEAVE_A_WOUND).clamp(0.0, 1.0);
        self.an_open_wound = self.an_open_wound.max(opened);

        self.lose_health(amount, "a blow");
    }

    /// Lose health to a named thing.
    ///
    /// One place, so that every drain says what it was as it happens. Working
    /// the cause out afterwards, by asking a corpse whether it was hungry,
    /// gave **"unknown cause" for 70% of every death in this model** - by the
    /// time anybody asks, the hunger has been eaten away and the cold has
    /// worn off, and the honest answer to every question is no.
    /// How hard a blow has to be before it leaves anything worth calling a
    /// wound.
    ///
    /// A blow of this size leaves one that is as open as they come. A scratch
    /// leaves a scratch.
    pub const WHAT_A_BLOW_HAS_TO_BE_TO_LEAVE_A_WOUND: f32 = 25.0;

    /// How much of an open wound closes in a tick.
    ///
    /// A fortnight to close the worst of them, on twelve ticks to the day,
    /// which is about right for something nobody stitched.
    pub const HOW_FAST_A_WOUND_CLOSES: f32 =
        1.0 / (14.0 * crate::environment::seasons::TICKS_PER_DAY as f32);

    pub fn lose_health(&mut self, amount: f32, to: &str) {
        if amount <= 0.0 {
            return;
        }

        self.health = (self.health - amount).max(0.0);
        self.what_last_took_health = Some(to.to_string());

        if self.health <= 0.0 {
            self.is_alive = false;
        }
    }

    /// Heal
    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(100.0);
    }

    /// Eat food and restore energy
    pub fn eat(&mut self, current_tick: u32, energy_restored: f32) {
        self.energy = (self.energy + energy_restored).min(100.0);
        self.took_a_meal(current_tick, crate::world::Soil::WASTE_PER_MEAL);
    }

    /// Record a meal: the clocks reset, and the body has something to pass.
    ///
    /// What it has to pass depends on what went in. A turnip gives the ground
    /// back some part of what growing that turnip took out of it, so a meal of
    /// turnips is at best a slow loss. A fish was grown at sea, so a meal of
    /// fish is the ground gaining something it never had - which is the whole
    /// reason a people beside a river can farm the same fields for ever.
    pub fn took_a_meal(&mut self, current_tick: u32, waste: f32) {
        self.last_ate_tick = current_tick;
        self.ticks_without_food = 0;
        self.waste_carried += waste;
    }

    /// Leave what the body has to leave, and report how much.
    pub fn void_waste(&mut self) -> f32 {
        std::mem::take(&mut self.waste_carried)
    }

    /// Drink water and reset dehydration
    pub fn drink(&mut self, current_tick: u32) {
        self.last_drank_tick = current_tick;
        self.ticks_without_water = 0;
    }

    /// How much of the life is left, if nothing answers this need.
    ///
    /// The number the whole hierarchy turns on. "The drive which will result
    /// in death the fastest has the highest priority" is not a ladder somebody
    /// wrote down - it falls out of the clocks each need actually runs on, so
    /// a child with a quarter of an adult's reserves orders its needs
    /// differently from its mother without anybody having decided that.
    ///
    /// Reckoned in ticks, from what is true of this body now. `None` means
    /// this need does not kill: it may make a life shorter or poorer, but not
    /// end it, and it takes its place by tier instead.
    pub fn ticks_before_this_kills_me(&self, drive_type: DriveType) -> Option<f32> {
        // What is left to lose, at the rate it is being lost
        fn once_health_goes(health: f32, per_tick: f32) -> f32 {
            if per_tick <= 0.0 {
                f32::INFINITY
            } else {
                health / per_tick
            }
        }

        let health = self.health.max(0.0);

        match drive_type {
            // Thirst is the fast one. Health starts going at a day and a half
            // and goes fifteen times faster after three days, which is why a
            // thirsty agent should stop whatever it is doing even if that
            // thing is fetching food.
            DriveType::Thirst => Some(
                self.physiology.minutes_before_thirst_kills_me()
                    / physiology::MINUTES_PER_TURN as f32,
            ),

            // Starvation is slower and scales with what the body has put by
            DriveType::Hunger => Some(
                self.physiology.minutes_before_hunger_kills_me()
                    / physiology::MINUTES_PER_TURN as f32,
            ),

            // Exhaustion is only a death clock once the energy is nearly
            // gone. Reckoning it from a full tank the way thirst is reckoned
            // from a full skin says every agent alive is a couple of thousand
            // ticks from dying of tiredness, which had Rest winning four turns
            // in five and a settlement doing nothing but sleep and forage.
            // Energy is topped up by every meal; it is not a clock that only
            // runs down.
            DriveType::Rest => {
                const NEARLY_SPENT: f32 = 25.0;

                if self.energy > NEARLY_SPENT {
                    return None;
                }

                let until_spent = once_health_goes(self.energy.max(0.0), 0.05);
                Some(until_spent + once_health_goes(health, 0.05))
            }

            // Whatever is trying to kill you is measured in minutes rather
            // than days, and only while it is actually there
            DriveType::Safety => None,

            _ => None,
        }
    }

    /// Check if agent is starving (critical survival state)
    pub fn is_starving(&self) -> bool {
        self.physiology.is_starving() || self.energy < 20.0
    }

    /// How old this body is, in years.
    pub fn years_old(&self) -> u32 {
        self.age / crate::environment::seasons::TICKS_PER_YEAR
    }

    /// What share of a grown appetite this body wants, for its age.
    pub fn what_i_eat_for_my_age(&self) -> f32 {
        what_a_body_this_age_eats(self.years_old()).max(0.05)
    }

    /// Make this body this many years old, in both of the places that say so.
    ///
    /// Age and life stage are two spellings of one fact, and the stage is a
    /// *stored* field: a dozen places set `life_stage` directly and leave
    /// `age` where it was, which makes a body whose stage and years disagree.
    /// Everything that actually reads a body's age reads the years -
    /// `what_i_eat_for_my_age` and `what_i_can_do_for_my_age` both count them
    /// - so such a body is an adult wearing a child's label.
    ///
    /// That is not a hypothetical. `a_child_and_an_adult_do_not_rank_the_same
    /// _needs_the_same_way` set `life_stage = Child`, asked how long hunger
    /// left the body, and got **47 against 47** - the same answer twice,
    /// because both bodies were the same age. Two more tests in two other
    /// modules reported the same thing, and the project status report listed
    /// "a child and an adult come out identical" as one of three blocking
    /// failures on the strength of them. It was one line in a fixture.
    ///
    /// Sizes the body as well as setting the number, because that is what a
    /// body of a given age *is*: `what_a_body_this_age_eats` decides the
    /// reserve, the stomach and the burn.
    pub fn now_this_many_years_old(&mut self, years: u32) {
        self.age = years * crate::environment::seasons::TICKS_PER_YEAR;
        self.life_stage = LifeStage::from_age(self.age);
        self.physiology.now_a_body_of(self.what_i_eat_for_my_age());
    }

    /// And what it can bring to moving, carrying, working and fighting.
    ///
    /// Floored a little above nothing rather than at nothing: an infant is
    /// worth nought out of ten on the specification's table, and a zero here
    /// would divide the pack capacity to nothing and make every arithmetic
    /// downstream of it a special case. Somebody in arms carries what an
    /// infant carries, which is not nothing and is not much.
    pub fn what_i_can_do_for_my_age(&self) -> f32 {
        what_a_body_this_age_can_do(self.years_old()).max(0.05)
    }

    /// Put this body where it would be after this long without food.
    ///
    /// Minutes, which is the scale the old `ticks_without_food` figures were
    /// always written on. Sizes the body to its life stage first, so a child
    /// set to two days empty is two days into a *child's* reserve.
    pub fn gone_without_food_for(&mut self, minutes: u32) {
        self.physiology.now_a_body_of(self.what_i_eat_for_my_age());
        self.physiology.gone_without_food_for(minutes);
        self.ticks_without_food = minutes;
    }

    /// Likewise, without water.
    pub fn gone_without_water_for(&mut self, minutes: u32) {
        self.physiology.now_a_body_of(self.what_i_eat_for_my_age());
        self.physiology.gone_without_water_for(minutes);
        self.ticks_without_water = minutes;
    }

    /// What share of itself this body can bring to anything.
    ///
    /// A quarter comes off at each of three-quarters, half and a quarter of a
    /// full body of water. See `Physiology::capability`.
    pub fn capability(&self) -> f32 {
        self.physiology.capability()
    }

    /// Check if agent is dehydrated (critical survival state)
    /// Dehydration is more urgent than starvation (720 ticks = 12 hours)
    pub fn is_dehydrated(&self) -> bool {
        self.physiology.is_parched()
    }

    /// Check if agent is in critical survival state
    pub fn is_survival_critical(&self) -> bool {
        self.is_starving() || self.is_dehydrated() || self.health < 30.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,

    /// Whether one of this agent's hands has a child in it.
    ///
    /// "Age 0-2: must remain with a parent agent at all times. Parent agent
    /// has one *hand* occupied with the child." Worked out once a turn in the
    /// kin phase, which is where the caregivers and their charges are already
    /// walked, and read by `update_inventory_capacity_from_transport`.
    ///
    /// A field rather than a question, because an agent cannot see the rest of
    /// the population from inside itself and what it can carry is asked of it
    /// alone, every turn, from four places.
    #[serde(default)]
    pub hands_full_of_child: bool,

    pub state: AgentState,
    pub drives: DriveState,
    pub behavior_trees: Vec<BehaviorTree>,
    pub memory: Memory,
    pub inventory: Inventory,
    pub senses: Senses,
    /// Recent percepts processed from sensory input (last 20 ticks)
    pub recent_percepts: Vec<(u32, super::sensory_processing::Percept)>, // (tick, percept)
    pub body: Body,
    pub body_temperature: super::BodyTemperature,
    pub exposure_status: crate::environment::ExposureStatus,
    pub skills: Skills,
    pub emotions: EmotionState,
    pub relationships: RelationshipMap,
    pub traits: TraitSet,
    pub knowledge: KnowledgeBase,
    pub observational_learning: ObservationalLearning,
    pub transport: TransportSystem,
    pub technology_knowledge: TechnologyKnowledge,
    pub exploration_knowledge: super::exploration::ExplorationKnowledge, // Map discovery and exploration
    /// How many times this one has been laid up by each thing.
    ///
    /// Deliberately not routed through `lessons`. An illness is not an
    /// attempt that did not work - it is days of your life - and the ordinary
    /// belief machinery cannot represent that: at any realistic rate of
    /// getting ill off raw flesh the successes outnumber the failures and the
    /// belief saturates *positive*, so an agent would eat raw flesh happily
    /// for ever. Equilibrium there sits at a 37 per cent illness rate, which
    /// is poison rather than a gamble.
    #[serde(default)]
    pub times_laid_up: std::collections::BTreeMap<String, u32>,
    /// The steps this agent has found out how to do that it was not born
    /// knowing - see `environment::making::Making::obvious`.
    #[serde(default)]
    found_out: std::collections::BTreeSet<String>,
    /// What has answered which need, and where it answered it.
    #[serde(default)]
    pub patterns: super::patterns::Patterns,
    pub storage_preferences: super::storage_management::StoragePreferences, // Storage management preferences
    pub parent_ids: Vec<Uuid>,

    /// Ways of working the agent has picked up rather than been born knowing.
    /// Nothing tells an agent to spread muck on a field: it tries it, sees what
    /// happens, and watches its neighbours.
    #[serde(default)]
    pub practices: super::practices::Practices,
    /// What the agent has found out about the kinds of thing it does. Every
    /// attempt is counted and what came of it shifts what the agent will try
    /// next, which is what makes a hunter who never catches anything stop
    /// hunting.
    #[serde(default)]
    pub lessons: super::practices::Lessons,
    /// Questions this one has put to the world and is waiting on the answer
    /// to - see [`super::wondering::Wondering`].
    ///
    /// Curiosity was a thing that answered in the same turn it was spent, and
    /// most of what a stone-age people has to find out does not answer for
    /// three days and does not answer where you are standing.
    #[serde(default)]
    pub wonderings: Vec<super::wondering::Wondering>,
    /// How much food this one has actually eaten, and how much went off on it
    /// before it could.
    ///
    /// The whole point of preserving anything is that the time spent getting
    /// it was not wasted. If half the meat rots before it is eaten, half the
    /// hunt was wasted - the hours are gone either way and only one of them
    /// fed anybody. Nothing in this project had ever counted that, so every
    /// preservation change had to be judged on how much was *in* the store
    /// rather than on how much of what was got was ever any use.
    #[serde(default)]
    pub food_i_ate: u32,
    #[serde(default)]
    pub food_that_rotted_on_me: u32,
    /// What is actually in this agent's hands, as against what is in the pack.
    ///
    /// A pair of them, which is what `verbs::A_PAIR_OF_HANDS` has always said
    /// and nothing had ever made true. Before this a tool in the pack was a
    /// tool in the hand: an axe helped you the moment you owned one, whether
    /// or not you had got it out, and "a free hand" could only be guessed at
    /// from how loaded the pack was. Taking a thing in hand is a turn's work
    /// now, it is worth doing because a tool already out does more, and it
    /// costs you the hand until you put it away.
    #[serde(default)]
    pub hands: [Option<String>; 2],
    /// What the world around this agent is doing, as far as its drives care.
    /// Filled in by the simulation once a tick; empty for an agent ticked
    /// without a world, which is the right answer for a world that is not
    /// there.
    #[serde(default)]
    pub surroundings: crate::core::Surroundings,
    pub goals: GoalManager,
    pub preferences: Preferences,
    pub equipment: super::equipment::EquipmentManager, // Equipped items (weapons, armor, tools)
    pub satisfaction_tracker: super::drive_satisfaction::SatisfactionTracker, // Tracks who/what satisfies which drives
    /// Current active plan being executed
    /// What this one set out to do, and has not finished doing.
    ///
    /// "Once an agent plans an action, it would not change its mind unless its
    /// situation changed in some manner. For example, an agent wants to walk
    /// to get a drink of water and the trip takes an estimated 10 minutes
    /// one-way. The agent begins walking and for the next ten ticks no new
    /// decisions need be made."
    ///
    /// See `Errand`.
    pub errand: Option<Errand>,
    pub current_plan: Option<ActionPlan>,
    /// Planning engine for generating and learning from plans
    pub planner: Planner,
    /// Ticks spent on current plan step (for timeout detection)
    pub plan_step_ticks: u32,
    /// Accumulated learning exposure for various knowledge/skills
    pub learning_exposure: crate::core::learning::LearningExposure,
    /// Nutritional state (energy, protein, micronutrients)
    pub nutrition: NutritionalState,
    /// Pregnancy state, on whichever of the pair is carrying
    pub pregnancy: Option<super::pregnancy::PregnancyState>,
    /// Nursing state (for infants)
    pub nursing: Option<super::childcare::NursingState>,
    /// Developmental nutrition tracking (affects adult stats)
    pub developmental_nutrition: super::childcare::DevelopmentalNutrition,
    /// Base reproduction drive modifier (personality-based, 0.5 to 1.5)
    pub reproduction_drive_modifier: f32,
    /// Fatigue state tracking tiredness, sleep debt, and penalties
    pub fatigue: super::fatigue::FatigueState,
    /// Cached healing bonus from nearby religious buildings
    pub cached_healing_bonus: f32,
    /// Cached defense bonus from nearby religious buildings
    pub cached_defense_bonus: f32,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        let mut agent = Self {
            id: crate::core::dice::name(),
            hands_full_of_child: false,
            state: AgentState::new(),
            drives: if config.random_weights {
                DriveState::with_random_weights()
            } else {
                DriveState::new()
            },
            behavior_trees: Vec::new(),
            memory: Memory::new(),
            inventory: Inventory::default(),
            senses: Senses::default(),
            recent_percepts: Vec::new(),
            body: Body::default(),
            body_temperature: super::BodyTemperature::default(),
            exposure_status: crate::environment::ExposureStatus::default(),
            skills: Skills::default(),
            emotions: EmotionState::default(),
            relationships: RelationshipMap::default(),
            traits: TraitSet::default(),
            knowledge: KnowledgeBase::default(),
            observational_learning: ObservationalLearning::default(),
            transport: TransportSystem::default(),
            technology_knowledge: TechnologyKnowledge::default(),
            exploration_knowledge: super::exploration::ExplorationKnowledge::default(),
            times_laid_up: std::collections::BTreeMap::new(),
            found_out: Self::what_anybody_is_born_knowing(),
            wonderings: Vec::new(),
            food_i_ate: 0,
            food_that_rotted_on_me: 0,
            patterns: super::patterns::Patterns::default(),
            storage_preferences: super::storage_management::StoragePreferences::default(),
            parent_ids: Vec::new(),
            practices: super::practices::Practices::new(),
            lessons: super::practices::Lessons::new(),
            hands: [None, None],
            surroundings: crate::core::Surroundings::default(),
            goals: GoalManager::new(5), // Max 5 active goals
            preferences: Preferences::default(),
            equipment: super::equipment::EquipmentManager::new(50.0), // 50kg max carry weight
            satisfaction_tracker: super::drive_satisfaction::SatisfactionTracker::new(),
            errand: None,
            current_plan: None,
            planner: Planner::new(),
            plan_step_ticks: 0,
            learning_exposure: crate::core::learning::LearningExposure::new(),
            nutrition: NutritionalState::new(),
            pregnancy: None,
            nursing: None,
            developmental_nutrition: super::childcare::DevelopmentalNutrition::default(),
            reproduction_drive_modifier: Self::generate_reproduction_modifier(),
            fatigue: super::fatigue::FatigueState::new(),
            cached_healing_bonus: 1.0,
            cached_defense_bonus: 1.0,
        };

        // Initialize default behavior trees for each drive type
        agent.initialize_behavior_trees();

        // No personality here. `Agent::new` builds a body; who that body turns
        // out to be is settled when somebody enters a world, by
        // `Population::spawn_agent` for the founding generation and by
        // inheritance for everybody born afterwards. Keeping the draw out of
        // the constructor means a bare `Agent::new` is the same agent every
        // time, which is what several dozen tests of other machinery rely on.

        // Let the traits reach the senses. Without this a Deaf agent hears
        // normally and a Blind one sees normally, because nothing else calls
        // this - it had no callers at all.
        agent.apply_trait_sensory_modifications();

        // A bare `Agent::new` is a grown person.
        //
        // It was a body of age nought, which `LifeStage::from_age` calls an
        // infant - and nothing minded while nothing read a body's age for
        // anything but its appetite. The moment the age capability curve was
        // hung on what two hands hold, every fixture in the project that says
        // `Agent::new` and means "a person" was carrying a twentieth of a
        // pack: **eighty-seven tests failed and one hung**, across carrying,
        // bartering, portioning, the larder and tool wear.
        //
        // This is defect #74 one layer down. That entry found founders
        // spawned at nought - "every world began with twelve newborns and
        // nobody to feed them" - and fixed it in `spawn_agent`, which
        // overrides this. The constructor underneath still made newborns, and
        // every caller that was not `spawn_agent` got one.
        //
        // A newborn now says so: `with_parents` is the birth path and sets the
        // age back to nought itself.
        agent.state.now_this_many_years_old(Self::WHAT_AGE_A_PERSON_IS_UNLESS_TOLD);

        // And what this body can actually carry, which is what two hands hold
        // until it has something to put things in. A bare `Inventory` has no
        // body and no basket and cannot work this out for itself; an agent
        // can, and does again every turn - see `take_up_the_cart`.
        agent.update_inventory_capacity_from_transport();

        agent
    }

    /// What age a body is when nobody has said.
    ///
    /// Grown, and old enough that the capability curve is at its full ten out
    /// of ten, so that a fixture which does not care about age gets a person
    /// rather than a baby. `spawn_agent` rolls a real age over the top of it
    /// and `with_parents` sets it back to nought.
    pub const WHAT_AGE_A_PERSON_IS_UNLESS_TOLD: u32 = 25;

    /// Generate a personality-based reproduction drive modifier
    fn generate_reproduction_modifier() -> f32 {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();
        // Range from 0.5 (low drive) to 1.5 (high drive)
        // Normal distribution centered at 1.0
        let base: f32 = rng.gen_range(0.5..1.5);
        // Add some variance
        (base + rng.gen_range(-0.2_f32..0.2_f32)).clamp(0.3, 1.8)
    }

    /// Create an agent with specified parents
    pub fn with_parents(config: AgentConfig, parent_ids: Vec<Uuid>, current_tick: u32) -> Self {
        use rand::Rng;

        let mut agent = Self::new(config);
        agent.parent_ids = parent_ids.clone();
        agent.state.last_ate_tick = current_tick;

        // Set up infant as newborn.
        //
        // In years as well as in the stored stage: `Agent::new` makes a grown
        // person now, so a birth has to say that this one is not.
        agent.state.now_this_many_years_old(0);
        agent.state.age = 0;

        // A newborn has just been fed and watered by being born.
        //
        // Both clocks are kept as "ticks since", worked out from a tick the
        // agent last ate or drank on, and both start at zero. For the founding
        // generation that is correct; for anybody born later it means a
        // newborn arrives having last drunk at the beginning of the world. An
        // infant born after about four thousand ticks was therefore two days
        // past the point where dehydration takes health, lost 1.65 a tick from
        // its first breath, and was dead at sixty-one - which is what a
        // settlement's whole second generation was quietly doing.
        agent.state.last_ate_tick = current_tick;
        agent.state.last_drank_tick = current_tick;
        agent.state.ticks_without_food = 0;
        agent.state.ticks_without_water = 0;

        // Rare chance of congenital infertility (~1.5% chance)
        let mut rng = crate::core::dice::roll();
        if rng.gen_bool(0.015) {
            agent.traits.add_trait(crate::core::traits::Trait::Infertile);
        }

        // Set up nursing - primary caregiver is first parent (usually mother)
        if let Some(&mother_id) = parent_ids.first() {
            agent.nursing = Some(super::childcare::NursingState::new(current_tick, mother_id));
            // Add second parent as secondary caregiver
            if let Some(&father_id) = parent_ids.get(1) {
                if let Some(ref mut nursing) = agent.nursing {
                    nursing.add_caregiver(father_id);
                }
            }
        }

        agent
    }

    /// Create a newborn with prenatal nutrition data from mother's pregnancy
    pub fn with_parents_and_prenatal(
        config: AgentConfig,
        parent_ids: Vec<Uuid>,
        current_tick: u32,
        prenatal_nutrition: f32,
    ) -> Self {
        let mut agent = Self::with_parents(config, parent_ids, current_tick);
        agent.developmental_nutrition = super::childcare::DevelopmentalNutrition::with_prenatal(prenatal_nutrition);
        agent
    }

    /// Whether the agent has nothing pressing on it right now.
    ///
    /// Fed, watered, rested and warm. This is what separates an agent that can
    /// afford to think about next winter from one that cannot.
    pub fn immediate_needs_met(&self) -> bool {
        use crate::core::DriveType;

        let quiet = |drive_type: DriveType| {
            self.drives
                .get(drive_type)
                .map(|drive| !drive.is_active())
                .unwrap_or(true)
        };

        quiet(DriveType::Hunger)
            && quiet(DriveType::Thirst)
            && quiet(DriveType::Rest)
            && !self.state.is_starving()
            && !self.state.is_dehydrated()
            && !self.body_temperature.is_too_cold()
            && self.exposure_status.active_exposures.is_empty()
    }

    /// What the agent's own means and condition are asking of its drives.
    ///
    /// The world-side half of the picture - what is prowling about, whether it
    /// is dark, who else is building - is filled in by the simulation and kept
    /// on `surroundings`; this folds in the half the agent knows for itself.
    pub fn what_the_situation_asks(&self) -> crate::core::DriveContext {
        use crate::core::DriveContext;

        let mut materials = 0u32;
        let mut tools = 0u32;
        let mut broken = 0u32;
        let mut finery = 0u32;

        for item in self.inventory.get_all_items().values() {
            if item.quantity == 0 {
                continue;
            }

            let name = item.item_id.as_str();

            if Self::MATERIALS.iter().any(|kind| name.contains(kind)) {
                materials += item.quantity;
            }

            if Self::TOOLS.iter().any(|kind| name.contains(kind)) {
                // A tool worn through is a reason to want a new one, which is
                // the specification's "broken tools" for Utility
                let worn_out = item
                    .current_durability
                    .map(|left| left <= 0.0)
                    .unwrap_or(false);

                if worn_out {
                    broken += item.quantity;
                } else {
                    tools += item.quantity;
                }
            }

            if Self::FINERY.iter().any(|kind| name.contains(kind)) {
                finery += item.quantity;
            }
        }

        // Anything equipped counts too: a hafted axe in the hand is a tool
        tools += self
            .equipment
            .get_all_equipped()
            .iter()
            .filter(|item| item.equipment_type.is_tool())
            .count() as u32;

        // What this agent is afraid of that has nothing to round on, and
        // which need it is about. See `what_i_dread`.
        let (dread, dread_of) = self.what_i_dread();

        DriveContext {
            around: self.surroundings.clone(),
            food_put_by: self.food_put_by(),
            materials_put_by: materials,
            tools_to_hand: tools,
            broken_tools: broken,
            fine_things: finery,
            armed: self.equipment.get_weapon().is_some(),
            exposed: !self.exposure_status.active_exposures.is_empty()
                || self.body_temperature.is_too_cold(),
            chilly: self.body_temperature.current
                < self.body_temperature.ideal - self.body_temperature.tolerance * 0.4,
            shelter_pressing: 0.0,
            dread,
            dread_of,
            at_leisure: false,
        }
    }

    /// What counts as material to work or build with


    /// How much of a thing an agent keeps on its person before the rest is
    /// spare.
    pub const ENOUGH_TO_HAND: u32 = 6;

    /// And how much food, which is a different question.
    ///
    /// Food is not flint: six armfuls of berries is not a sensible thing to
    /// be carrying about once berries actually go off. Anything past a meal
    /// is worth putting by rather than walking around with, and a settlement
    /// living hand to mouth rarely holds even that - which is why this was
    /// three and had to come down. At three, `Cover` was refused 1,513 times
    /// out of 1,525 for want of anything to bury.
    pub const WHAT_IS_NOT_WORTH_A_TRIP: u32 = 1;

    /// The thing this agent has most of beyond what it needs about it, if any.
    ///
    /// The Preparedness drive named `item_type: "resource"` - a placeholder
    /// string the storehouse does not recognise - so putting something by
    /// could not work and never once did. Measured, `Store` was 4.7% of
    /// everything a settlement did and failed 100.0% of the time, thirteen
    /// thousand times in four thousand ticks, every one of them
    /// `Unknown item type: resource`.
    ///
    /// Food is left where it is: what is in the pack is what an agent eats
    /// from, and putting the last of it in a storehouse across the settlement
    /// is not thrift.
    pub fn what_i_can_spare(&self) -> Option<(String, u32)> {
        self.inventory
            .get_all_items()
            .iter()
            .filter(|(name, item)| {
                item.quantity > Self::ENOUGH_TO_HAND
                    && item.food_data.is_none()
                    && !name.contains("food")
            })
            .max_by_key(|(_, item)| item.quantity)
            .map(|(name, item)| (name.clone(), item.quantity - Self::ENOUGH_TO_HAND))
    }

    /// How much more weight this pack is holding than its owner can carry.
    ///
    /// Not `weight_capacity_remaining`, which is clamped at nought and so says
    /// the same thing about a pack that is exactly full and one that is half
    /// as much again over its limit. Both happen: a body that weakens carries
    /// less than it did, and until now nothing took the load off it when it
    /// did.
    ///
    /// Down to the limit and not a pound further. The first cut shed down to
    /// the limit *less a day's food*, on the reasoning that a forager loaded
    /// to the last ounce cannot pick anything up - which is true, and cost
    /// **five per cent of the settlement's person-days** against shedding to
    /// the limit alone, measured over 160 worlds. What a person is willing to
    /// walk about carrying is a decision, and dressing one up as a law made it
    /// worse. This is only the law: what cannot be carried is not carried.
    pub fn how_much_too_much_i_am_carrying(&self) -> f32 {
        (self.inventory.current_weight - self.inventory.effective_max_weight()).max(0.0)
    }

    /// What a person sets down when the pack will not take any more food.
    ///
    /// Nothing in this model ever put a load down. An agent picked up wood
    /// for a fire, iron because it glittered and stone out of a hole it dug,
    /// and carried all of it for the rest of its life - so measured across
    /// eight worlds an autumn pack held **38.9 units against a capacity of
    /// 26.0**, half as much again as it could take, and **97% of autumn
    /// agent-ticks had not room in it for a single handful of food**. Twenty
    /// eight thousand units of food a year went back on the bush for want of
    /// anywhere to put them, against two and a half thousand carried home.
    ///
    /// What goes down is the heaviest thing that is none of: food, a tool
    /// this one works with, or the thing it carries its load in. Weight
    /// rather than count, because the question is room and a stack of forty
    /// berries is not a stack of forty logs. The first cut of this filtered
    /// on `ENOUGH_TO_HAND`, which is a count, and it never fired once: wood
    /// weighs two a stick, so the ten units of firewood filling every pack in
    /// the world were five sticks and five is not more than six. A reserve
    /// counted in things cannot answer a question asked in weight.
    pub fn what_i_would_set_down(&self) -> Option<String> {
        use crate::environment::making;

        self.inventory
            .get_all_items()
            .iter()
            .filter(|(_, item)| item.quantity > 0)
            .filter(|(_, item)| item.food_data.is_none() && !item.is_food())
            .filter(|(name, _)| {
                !making::EVERY_TOOL.iter().any(|tool| tool.called == name.as_str())
            })
            .filter(|(name, _)| {
                !Self::WHAT_CARRIES.iter().any(|(called, _)| *called == name.as_str())
            })
            .max_by(|a, b| {
                let load = |item: &InventoryItem| item.quantity as f32 * item.weight_per_unit;
                load(a.1)
                    .partial_cmp(&load(b.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(name, _)| name.clone())
    }

    /// How much of this one goes on the grass, once it has been decided that
    /// some of it should.
    ///
    /// Enough to bring the load back inside the limit and not a stick more. A
    /// man who tips his whole bundle of firewood on the grass because he is
    /// carrying three sticks too many has to go and cut more tomorrow.
    ///
    /// Derived from the shortfall rather than from a reserve, so it answers
    /// the question that was actually asked. Where one stack is not enough -
    /// a pack half as much again over its limit is not emptied by three
    /// sticks - what is left is still short, and the next turn sets down the
    /// next heaviest thing.
    pub fn how_much_of_this_i_would_set_down(&self, what: &str) -> u32 {
        let Some(item) = self.inventory.get_item(what) else {
            return 0;
        };

        let each = item.weight_per_unit * item.how_much_lighter_it_is();
        if each <= 0.0 {
            return item.quantity;
        }

        let wanted = (self.how_much_too_much_i_am_carrying() / each).ceil() as u32;
        wanted.clamp(1, item.quantity)
    }

    /// Food this agent has more of than it is going to eat.
    ///
    /// Deliberately separate from `what_i_can_spare`, which excludes anything
    /// anybody eats: that one is about materials for the chain and it was
    /// right to leave food out of it while there was nowhere to put food. A
    /// hole in the ground is somewhere to put food.
    pub fn what_food_i_can_spare(&self) -> Option<(String, u32)> {
        self.inventory
            .get_all_items()
            .iter()
            .filter(|(name, item)| {
                item.quantity > Self::WHAT_IS_NOT_WORTH_A_TRIP && item.is_food()
            })
            // What keeps worst is what most wants burying
            .max_by_key(|(_, item)| item.quantity)
            .map(|(name, item)| (name.clone(), item.quantity - Self::WHAT_IS_NOT_WORTH_A_TRIP))
    }

    /// The best thing in the pack that is worth laying out to dry.
    ///
    /// `what_food_i_can_spare` picks the *largest* stack and every branch of
    /// the provisioning decision then uses that one item, which is fine when
    /// everything keeps the same way and wrong the moment it does not. What
    /// it did in practice was hand a whole fish to the drying branch, because
    /// whole fish is what a settlement carries most of - and a whole fish
    /// laid in the sun turns, it does not dry.
    ///
    /// So this asks the question the drying branch actually wants: of
    /// everything I could spare, what would keep if I laid it out?
    pub fn what_i_could_dry(&self) -> Option<(String, u32)> {
        self.inventory
            .get_all_items()
            .iter()
            .filter(|(_, item)| item.quantity > Self::WHAT_IS_NOT_WORTH_A_TRIP)
            .filter(|(name, _)| self.is_it_worth_drying(name))
            // Most of it first: one turn dries a stack, so the biggest stack
            // is the best use of the turn.
            .max_by_key(|(_, item)| item.quantity)
            .map(|(name, item)| (name.clone(), item.quantity - Self::WHAT_IS_NOT_WORTH_A_TRIP))
    }

    /// The best thing in the pack that salt would keep.
    ///
    /// The same question as `what_i_could_dry` and it has to be asked
    /// separately, because the two do not accept the same things: drying
    /// wants a thing cut thin and a fortnight of weather, and salt does not
    /// care about either. What they share is that neither will do anything
    /// for a whole carcass or for something already turned.
    pub fn what_i_could_salt(&self) -> Option<(String, u32)> {
        use crate::world::nutrition::{Piece, PreparationState};

        self.inventory
            .get_all_items()
            .iter()
            .filter(|(_, item)| item.quantity > Self::WHAT_IS_NOT_WORTH_A_TRIP)
            .filter(|(name, _)| Piece::of(name) != Piece::Whole)
            .filter(|(_, item)| {
                item.food_data.as_ref().is_some_and(|food| {
                    food.preparation == PreparationState::Raw
                        && food.freshness >= Self::WORTH_PUTTING_BY
                })
            })
            .max_by_key(|(_, item)| item.quantity)
            .map(|(name, item)| (name.clone(), item.quantity - Self::WHAT_IS_NOT_WORTH_A_TRIP))
    }

    /// This one drank out of the sea.
    ///
    /// It slakes the thirst on the tick - that is the trap, and it is why
    /// people do it - and then costs more than it gave, over the days it
    /// takes the body to get rid of it again.
    pub fn drank_salt_water(&mut self, now: u32) {
        use super::EmotionSource;

        self.state.salt_in_me =
            (self.state.salt_in_me + Self::WHAT_ONE_DRINK_OF_THE_SEA_LEAVES).min(Self::AS_SALT_AS_ANYBODY_GETS);

        // And it is a thing that happened, recorded against the doing of it,
        // so that somebody who has done it twice knows better.
        self.lessons.record_particular(Self::DRINKING_THE_SEA, false);

        if self.state.salt_in_me >= Self::AS_SALT_AS_ANYBODY_GETS {
            self.emotions.add_fear_with_traits(
                EmotionSource::Event("salt".to_string()),
                Self::WHAT_BEING_SALT_IS_WORTH_IN_WORRY,
                &self.traits,
            );
            let _ = now;
        }
    }

    /// Whether this one knows better than to drink the sea.
    ///
    /// Everybody does. This is not a discovery - a mouthful of sea water
    /// tells you what it is the moment it is in your mouth, and every people
    /// that ever lived beside one knew. What it is not is a rule that holds
    /// when somebody is three days dry.
    pub fn would_i_drink_the_sea(&self) -> bool {
        self.state.is_dehydrated()
    }

    /// A tick of having drunk the sea: the thirst comes back worse, and the
    /// salt slowly goes.
    fn tick_salt(&mut self) {
        if self.state.salt_in_me <= 0.0 {
            return;
        }

        let salt = self.state.salt_in_me;

        // **Out of the body, not off the drive.**
        //
        // This raised the Thirst drive directly, and the drive is *assigned*
        // from the body a few hundred lines up - `drive.value =
        // body_wants_water` - so every unit of thirst the salt added was wiped
        // the same tick it was added. Measured: a man given a drink of the sea
        // and then followed for six days came out at exactly the thirst he
        // went in with, 0.3 against 0.3. The whole of ISSUES_FOUND.md #155 -
        // salt water "drinkable, tempting, and worse than nothing" - did
        // nothing whatever.
        //
        // Which is also the right model rather than a way round the
        // assignment. Sea water does not make you feel thirstier; it makes you
        // *drier*, because the kidney spends more water getting the salt out
        // than the drink brought in. Taking it off the hydration is the fact,
        // and the thirst then follows on its own like every other thirst in
        // the model.
        self.state.physiology.hydration =
            (self.state.physiology.hydration - salt * Self::WHAT_THE_SALT_COSTS_IN_WATER)
                .clamp(0.0, 1.0);

        self.state.salt_in_me = (salt - Self::HOW_FAST_SALT_GOES).max(0.0);
    }

    /// What one drink of sea water leaves behind.
    const WHAT_ONE_DRINK_OF_THE_SEA_LEAVES: f32 = 0.35;

    /// And the most anybody can be carrying at once.
    const AS_SALT_AS_ANYBODY_GETS: f32 = 1.0;

    /// How much water a full load of salt costs the body every tick.
    ///
    /// The drink itself is worth `physiology::A_DRINK_IS_WORTH`, a third of a
    /// skin. One drink of the sea leaves 0.35 of salt, which goes at
    /// `HOW_FAST_SALT_GOES` a tick and so takes about twenty-nine ticks to
    /// clear, and over those ticks the salt in it costs about 0.35 of a skin
    /// in water at this rate.
    ///
    /// So a drink of the sea gives a third and takes rather more than a third
    /// back over the two and a half days it takes to be rid of, which is what
    /// "worse than nothing" means and is why it is a thing a desperate man
    /// does and a sensible one does not.
    const WHAT_THE_SALT_COSTS_IN_WATER: f32 = 0.0007;

    /// And how fast the body gets rid of it.
    const HOW_FAST_SALT_GOES: f32 = 0.012;

    /// What being full of salt is worth in worry.
    const WHAT_BEING_SALT_IS_WORTH_IN_WORRY: f32 = 0.2;

    /// What an agent calls drinking the sea, for the record it keeps.
    pub const DRINKING_THE_SEA: &'static str = "the sea";

    /// This one has come down with something.
    ///
    /// Nothing here stacks: somebody already ill does not get a second
    /// ailment on top, they simply stay ill. What a second exposure does is
    /// nothing at all, which is a simplification and a deliberate one - the
    /// alternative is a settlement that catches four things in a bad week and
    /// dies of arithmetic.
    pub fn taken_ill_with(&mut self, from: &str, severity: f32, now: u32) {
        use super::EmotionSource;
        use rand::Rng;

        if self.state.ailing.is_some() {
            return;
        }

        let severity = severity.clamp(0.05, 1.0);

        let spread = Ailment::THE_LONGEST_IT_LASTS - Ailment::THE_SHORTEST_IT_LASTS;
        let how_long = Ailment::THE_SHORTEST_IT_LASTS
            + (spread as f32 * severity) as u32
            + crate::core::dice::roll().gen_range(0..=Ailment::THE_SHORTEST_IT_LASTS);

        self.state.ailing = Some(Ailment {
            from: from.to_string(),
            since: now,
            until: now + how_long,
            severity,
            at_its_worst: severity,
        });

        // And it is a thing that happened for a reason. This is the whole of
        // the learning: somebody who has been ill twice off raw flesh has a
        // record saying so, and can decline the third helping.
        *self.times_laid_up.entry(from.to_string()).or_insert(0) += 1;

        self.emotions.add_fear_with_traits(
            EmotionSource::Event(format!("ill from {from}")),
            severity * Self::WHAT_BEING_ILL_IS_WORTH_IN_WORRY,
            &self.traits,
        );
    }

    /// Whether this one is ill.
    pub fn is_ailing(&self) -> bool {
        self.state.ailing.is_some()
    }

    /// What it is down with, if anything.
    pub fn what_ails_me(&self) -> Option<&Ailment> {
        self.state.ailing.as_ref()
    }

    /// How often an open wound turns, in a tick, at its worst.
    ///
    /// About one in three hundred, which over the fortnight a bad wound takes
    /// to close comes to rather better than an even chance of getting away
    /// with it. That is the shape of the thing: most people were all right,
    /// and the ones who were not died of it.
    const HOW_OFTEN_A_WOUND_TURNS: f64 = 0.0035;

    /// And how often a soaking in the cold turns into a chill.
    ///
    /// Read against how much the weather is actually taking out of somebody,
    /// so a mild damp day is nothing and a January night in the open is not.
    const HOW_OFTEN_A_SOAKING_TELLS: f64 = 0.02;

    /// A wound closes, or it turns.
    fn tick_the_wound(&mut self, now: u32) {
        use rand::Rng;

        if self.state.an_open_wound <= 0.0 {
            return;
        }

        // Only an open wound can turn, and only somebody not already ill can
        // come down with it - nothing in this model stacks.
        if self.state.ailing.is_none() {
            let odds = Self::HOW_OFTEN_A_WOUND_TURNS * self.state.an_open_wound as f64;
            if crate::core::dice::roll().gen_bool(odds.clamp(0.0, 1.0)) {
                let how_bad = 0.4 + 0.5 * self.state.an_open_wound;
                self.taken_ill_with(Self::OFF_A_WOUND_THAT_TURNED, how_bad, now);
            }
        }

        self.state.an_open_wound =
            (self.state.an_open_wound - AgentState::HOW_FAST_A_WOUND_CLOSES).max(0.0);
    }

    /// Cold and wet, for long enough, comes to something.
    ///
    /// Called with what the weather is costing this tick, which is already
    /// the answer to "how cold, how wet, how sheltered" - see
    /// `update_exposure`. Nothing here needs to ask those three again.
    pub fn a_soaking_may_tell(&mut self, what_the_weather_costs: f32, now: u32) {
        use rand::Rng;

        if what_the_weather_costs <= 0.0 || self.state.ailing.is_some() {
            return;
        }

        let odds = Self::HOW_OFTEN_A_SOAKING_TELLS * what_the_weather_costs.min(1.0) as f64;
        if crate::core::dice::roll().gen_bool(odds.clamp(0.0, 1.0)) {
            let how_bad = (0.2 + what_the_weather_costs).clamp(0.2, 0.8);
            self.taken_ill_with(Self::OFF_A_SOAKING, how_bad, now);
        }
    }

    /// A tick of being ill: it costs, and then it is over.
    fn tick_ailment(&mut self, now: u32) {
        let Some(ailing) = self.state.ailing.as_ref() else {
            return;
        };

        if ailing.is_over(now) {
            // Come through it. Getting better is not a point in favour of
            // whatever made you ill, so nothing is recorded here.
            self.state.ailing = None;
            return;
        }

        let severity = ailing.severity;
        self.state.health =
            (self.state.health - severity * Self::WHAT_A_TICK_OF_ILLNESS_COSTS).max(0.0);
        self.state.energy =
            (self.state.energy - severity * Self::WHAT_ILLNESS_TAKES_OUT_OF_YOU).max(0.0);
    }

    /// Take something for it.
    ///
    /// **This is the whole of the treatment in this model, and it is
    /// deliberately not very much.** A remedy takes something off how badly
    /// somebody is laid up; it never shortens the illness by a single tick,
    /// and no amount of it can take off more than
    /// `THE_MOST_A_HERBAL_CAN_DO`. That cap is the line between easing and
    /// curing, and every caveat in the specification is on the easing side of
    /// it: aloe is "not a replacement for burn or wound care", echinacea's
    /// "clinical benefits remain uncertain", garlic is not "an antibiotic
    /// substitute". A settlement can have the whole hedgerow and still bury
    /// people.
    ///
    /// The wrong remedy is still worth something - somebody has been looked
    /// after - but only a quarter, which is what makes knowing one herb from
    /// another worth having.
    ///
    /// Returns how much came off, or `None` if there was nothing to treat or
    /// the thing was not a remedy at all.
    pub fn take_a_remedy(&mut self, item_id: &str, now: u32) -> Option<f32> {
        use crate::environment::remedies;

        let remedy = remedies::what_this_is_good_for(item_id)?;
        let ailing = self.state.ailing.as_ref()?;

        if ailing.is_over(now) {
            return None;
        }

        let worst = ailing.at_its_worst();
        let sort = ailing.what_sort_it_is();

        // A practised hand gets more out of the same handful: knowing when to
        // pick it, how much to use, and what to do with it. The untaught get
        // rather less and never nothing.
        let hand = self
            .skills
            .get_skill_if_exists(super::SkillType::Herbalism)
            .map(|skill| skill.level)
            .unwrap_or(0)
            .clamp(0, 10) as f32
            / 10.0;
        let by_hand = Self::WHAT_AN_UNTAUGHT_HAND_GETS
            + (1.0 - Self::WHAT_AN_UNTAUGHT_HAND_GETS) * hand;

        let for_the_right_thing = if remedy.eases == sort {
            1.0
        } else {
            remedies::WHAT_THE_WRONG_REMEDY_IS_STILL_WORTH
        };

        // Against the illness at its worst, never against what it has already
        // been eased to: this is what stops a second dose taking a second
        // third off, and a sixth dose curing.
        let already_off = (worst - ailing.severity).max(0.0);
        let room = (worst * remedies::THE_MOST_A_HERBAL_CAN_DO - already_off).max(0.0);
        let eased = (worst * remedy.takes_off * by_hand * for_the_right_thing).min(room);

        if eased <= 0.0 {
            return Some(0.0);
        }

        if let Some(ailing) = self.state.ailing.as_mut() {
            ailing.severity = (ailing.severity - eased).max(0.0);
        }

        Some(eased)
    }

    /// What somebody who has never been taught gets out of a remedy.
    ///
    /// Half. The plants do what the plants do; what a herbalist adds is
    /// knowing which one, when it was picked and how much of it - real, and
    /// not the difference between life and death.
    const WHAT_AN_UNTAUGHT_HAND_GETS: f32 = 0.5;

    /// Whether this one would be glad of something for it.
    pub fn wants_something_for_it(&self) -> bool {
        self.state.ailing.is_some()
    }

    /// The first thing in the pack that is any use as a remedy, best first.
    ///
    /// Best for what actually ails them, so a herbalist reaches past the aloe
    /// for the mint. Somebody with no Herbalism at all reaches for whatever
    /// is nearest, which is what `WHAT_AN_UNTAUGHT_HAND_GETS` is about at the
    /// other end.
    pub fn what_i_have_for_it(&self) -> Option<String> {
        use crate::environment::remedies;

        let sort = self.state.ailing.as_ref()?.what_sort_it_is();
        let taught = self
            .skills
            .get_skill_if_exists(super::SkillType::Herbalism)
            .map(|skill| skill.level > 0)
            .unwrap_or(false);

        let mut best: Option<(f32, String)> = None;
        for (id, item) in self.inventory.get_all_items().iter() {
            if item.quantity == 0 {
                continue;
            }
            let Some(remedy) = remedies::what_this_is_good_for(id) else {
                continue;
            };

            // Somebody who has been taught knows which is which. Somebody who
            // has not takes the first thing that is called medicine.
            let worth = if taught && remedy.eases != sort {
                remedy.takes_off * remedies::WHAT_THE_WRONG_REMEDY_IS_STILL_WORTH
            } else {
                remedy.takes_off
            };

            if best.as_ref().map(|(so_far, _)| worth > *so_far).unwrap_or(true) {
                best = Some((worth, id.clone()));
            }
        }

        best.map(|(_, id)| id)
    }

    /// Whether there is anything in the pack worth carrying to somebody
    /// else who is ill.
    ///
    /// Anything at all that is a remedy: what is right for them depends on
    /// what ails *them*, which this cannot see. It is the cheap check that
    /// stops a healthy agent walking across the camp with an empty hand.
    pub fn what_i_have_for_it_for_somebody_else(&self) -> Option<String> {
        use crate::environment::remedies;

        self.inventory
            .get_all_items()
            .iter()
            .find(|(id, item)| item.quantity > 0 && remedies::is_a_remedy(id))
            .map(|(id, _)| id.clone())
    }

    /// Whether this one has learned, the hard way, to leave a thing alone.
    ///
    /// Reads the same record `taken_ill_with` writes. It is the ordinary
    /// lessons machinery - nothing here knows what "raw" means, only that
    /// this agent has a bad history with something by that name.
    pub fn has_this_made_me_ill(&self, from: &str) -> bool {
        self.how_often_this_has_laid_me_up(from) >= Self::TWICE_IS_A_PATTERN
    }

    /// How many times this one has been laid up by a given thing.
    pub fn how_often_this_has_laid_me_up(&self, from: &str) -> u32 {
        self.times_laid_up.get(from).copied().unwrap_or(0)
    }

    /// How many times a thing has to have laid somebody up before it is
    /// worth treating as a rule rather than as bad luck.
    ///
    /// Twice. A week in bed is a great deal of evidence, and a person who has
    /// spent two of them off the same thing does not need a third.
    pub const TWICE_IS_A_PATTERN: u32 = 2;

    /// What a tick of being ill takes off the body.
    ///
    /// Small on purpose. A week of it at full severity comes to about a
    /// quarter of a healthy body, which is a bad illness and not a sentence.
    const WHAT_A_TICK_OF_ILLNESS_COSTS: f32 = 0.25;

    /// And what it takes out of somebody's day.
    ///
    /// This is the part that costs a settlement rather than the agent: an
    /// agent with no energy sleeps, and somebody asleep for four days in the
    /// autumn is four days of nobody gathering.
    const WHAT_ILLNESS_TAKES_OUT_OF_YOU: f32 = 0.8;

    /// What coming down with something is worth in worry.
    const WHAT_BEING_ILL_IS_WORTH_IN_WORRY: f32 = 0.3;

    /// How often eating raw flesh makes somebody ill.
    ///
    /// About one meal in twelve. Raw flesh is not poison - people have lived
    /// on it - it is a gamble, and the point of a fire is that it stops being
    /// one. Cooking was worth 2.7 times the nutrition and nothing else before
    /// this, so there was no reason on earth to light a fire you had to fetch
    /// wood for.
    pub const HOW_OFTEN_RAW_FLESH_TELLS: f64 = 0.08;

    /// And how often food that is on the turn does.
    ///
    /// Scaled by how far gone it is, so this is the rate at the point where
    /// it is barely edible. Food past `is_harmful` is a separate and worse
    /// matter that was already handled.
    pub const HOW_OFTEN_FOOD_ON_THE_TURN_TELLS: f64 = 0.35;

    /// Below this, food is on the turn: still edible, no longer safe.
    pub const ON_THE_TURN: f32 = 0.5;

    /// What this agent calls being ill off raw flesh, which is the key it
    /// learns against.
    pub const OFF_RAW_FLESH: &'static str = "raw flesh";

    /// And off food that had started to go.
    pub const OFF_FOOD_ON_THE_TURN: &'static str = "food on the turn";

    /// And off living on fouled ground.
    pub const OFF_FOUL_GROUND: &'static str = "foul ground";

    /// A chill: cold and wet, for long enough, with no roof.
    ///
    /// The winter had nothing to do with illness in this model until the
    /// thermometer started reading below freezing - see ISSUES_FOUND.md #161.
    /// A person soaked through in a January wind gets ill, and that is most
    /// of what a shelter and a coat are *for* beyond the exposure damage
    /// itself.
    pub const OFF_A_SOAKING: &'static str = "a soaking";

    /// A wound that did not close.
    ///
    /// The thing that killed people who survived the bear. Nothing in this
    /// model has ever cared what happened to a wound after the blow landed:
    /// health came back at a flat rate and that was the end of it. A wound
    /// that turns is why a man with aloe is better off than a man without.
    pub const OFF_A_WOUND_THAT_TURNED: &'static str = "a wound that turned";

    /// Food went off in this agent's own hands.
    ///
    /// Worry rather than grief. What has been lost is not the meal so much as
    /// the certainty of the next one, and the specification is explicit that
    /// it should land as a threat to the hunger drive rather than as a bad
    /// mood: an agent that has watched its supper turn twice this week is an
    /// agent that has a reason to dig a hole and lay things out in the sun.
    pub fn watched_food_go_off(&mut self, what: &str, how_much: u32) {
        use super::EmotionSource;

        // The wasted half of whatever was spent getting it
        self.food_that_rotted_on_me = self.food_that_rotted_on_me.saturating_add(how_much);

        let worth_minding = (how_much as f32 * Self::WHAT_A_LOST_MEAL_IS_WORTH)
            .min(Self::AS_MUCH_AS_ONE_LOT_CAN_COST);

        self.emotions.add_fear_with_traits(
            EmotionSource::Event(format!("{what} went off")),
            worth_minding,
            &self.traits,
        );

        // And it is a thing that happened, so it is a thing that can be
        // learned from: whatever this agent was doing with that food, it did
        // not work
        self.lessons.record_particular("keeping food", false);
    }

    /// What one unit of food turning is worth in worry.
    const WHAT_A_LOST_MEAL_IS_WORTH: f32 = 0.03;

    /// And the most any single lot of it can come to, so that losing a
    /// basketful is bad rather than paralysing.
    const AS_MUCH_AS_ONE_LOT_CAN_COST: f32 = 0.25;

    /// Whether a thing in the pack is raw food still good enough to be worth
    /// preserving.
    ///
    /// Raw, because anything already dried or salted or fermented is done.
    /// Still sound, because preserving does not undo what has already
    /// happened to a thing: all you get from drying carrion is dry carrion.
    pub fn is_it_worth_drying(&self, what: &str) -> bool {
        use crate::world::nutrition::PreparationState;

        // And only if this one knows what laying a thing out would do. The
        // first cut left this out, so agents chose to dry things they had no
        // idea how to dry, the action came back refused, and the turn was
        // gone - which cost a settlement more than half of what it had in the
        // ground by winter.
        if !self.found_out.contains(Self::THAT_LAYING_IT_OUT_KEEPS_IT) {
            return false;
        }

        // And not a whole beast. Laying a carcass out does not dry it, it
        // turns it - which is the one thing about preserving that this world
        // teaches rather than hands down.
        if crate::world::nutrition::Piece::of(what) == crate::world::nutrition::Piece::Whole {
            return false;
        }

        self.inventory
            .get_item(what)
            .and_then(|item| item.food_data.as_ref())
            .is_some_and(|food| {
                food.preparation == PreparationState::Raw
                    && food.freshness >= Self::WORTH_PUTTING_BY
            })
    }

    /// How sound a thing has to still be before anybody bothers preserving
    /// it.
    const WORTH_PUTTING_BY: f32 = 0.5;

    /// What somebody has to have watched happen before they will lay food out
    /// on purpose.
    ///
    /// The same string the simulation records when a thing dries in the sun
    /// with somebody standing over it - see
    /// `Simulation::THAT_LAYING_IT_OUT_KEEPS_IT`.
    pub const THAT_LAYING_IT_OUT_KEEPS_IT: &'static str = "drying";

    /// Everything a person could use and has next to none of.
    ///
    /// "The agents should also use a barter system if they have an abundance of
    /// something another agent wants and that agent has an abundance of
    /// something they want." This is the second half of that: the things this
    /// agent would take if somebody offered them, being the raw stuff that
    /// every step and every working in the chain asks for and that this pack
    /// is short of.
    pub fn what_i_am_short_of(&self) -> Vec<&'static str> {
        use crate::environment::making;
        use std::collections::BTreeSet;

        let mut wanted: BTreeSet<&'static str> = BTreeSet::new();

        for step in making::EVERY_STEP {
            for (what, _) in step.needs {
                wanted.insert(what);
            }
        }
        for working in making::EVERY_WORKING {
            wanted.insert(working.to);
        }

        let mut short: Vec<&'static str> = wanted
            .into_iter()
            .filter(|what| self.how_many_i_have(what) < Self::ENOUGH_TO_HAND)
            .collect();

        // Stable, so two agents looking at the same pack agree about it
        short.sort_unstable();
        short
    }

    /// Somebody did this agent a kindness, and it counts.
    ///
    /// The gratitude machinery was written for one caller and had none. A
    /// thing handed over is exactly what it was for.
    pub fn they_did_me_a_good_turn(&mut self, who: Uuid, how_much: f32) {
        self.process_gratitude(who, how_much);
    }

    /// What a grown person already knows how to do when a world opens.
    ///
    /// Every skill started at -10, the floor, for everybody. That is not a
    /// people arriving somewhere; it is a people who have never done anything.
    /// And it deadlocks: the one thing the Utility drive reaches for is a
    /// wooden axe needing Crafting at -5, skill rises only by doing, so nobody
    /// could make their first axe and the settlement never held a single tool.
    /// Measured, Craft failed 99.3% of the time on that one gate.
    ///
    /// These are the hands of people who lived somewhere before they came
    /// here: enough to feed themselves, put up a tent, work a hide and knap a
    /// stone, and no more. It is a floor to build on rather than a gift - the
    /// climb from here to mastery is untouched, and a founder is still nearer
    /// the bottom of it than the top.
    const WHAT_A_GROWN_PERSON_ARRIVES_KNOWING: [(super::SkillType, i32); 8] = [
        (super::SkillType::Herbalism, -4),
        (super::SkillType::Hunting, -5),
        (super::SkillType::Fishing, -6),
        (super::SkillType::Cooking, -5),
        (super::SkillType::Crafting, -4),
        (super::SkillType::Construction, -5),
        (super::SkillType::Leatherworking, -5),
        (super::SkillType::Woodcutting, -4),
    ];

    /// And what they carry: what you can knap, cut and stitch with.
    ///
    /// Tools and nothing else. Giving founders the hides and poles for a tent
    /// as well seemed obviously right and was measurably ruinous: twenty-five
    /// people who can all raise a tent on the first tick all try to, crowd the
    /// same ground - `No suitable building location found (all positions
    /// occupied)` - and spend the rest of their lives walking about looking
    /// for somewhere to put one instead of feeding themselves. Measured
    /// against the same commit, two worlds a side: 136 alive at the baseline,
    /// 134 with the skills alone, and 36 with the materials in the pack.
    ///
    /// So they arrive knowing how to raise a tent and having to gather the
    /// hides for it, which is a stone-age start rather than a stone-age
    /// stockpile.
    ///
    /// They are the same named things the chain in `environment::making`
    /// turns out, so that what a founder wears through is a thing his people
    /// know how to replace.
    /// And a basket, because a people that walked in carrying two days of food
    /// carried it in something. Without one an agent holds what two hands hold
    /// and nothing else - see `WHAT_TWO_HANDS_HOLD`.
    const WHAT_THEY_CARRY: [(&'static str, u32, f32); 3] = [
        ("handaxe", 1, 2.0),
        ("stoneknife", 1, 0.5),
        ("basket", 1, 1.0),
    ];

    /// And what they arrive with in the way of food.
    ///
    /// A people that walks into a valley has been eating on the way. Founders
    /// arrived with an empty pack and had to find their first meal before they
    /// had found the water, the wood or anywhere to sleep, which is not a
    /// stone-age start - it is a shipwreck.
    ///
    /// Two days of it, and no more. A stone-age start rather than a stone-age
    /// stockpile is the rule these founders are set up by, and giving them a
    /// winter's worth would answer the question this model exists to ask. Two
    /// days is enough to be looking for the good ground rather than for
    /// tonight's supper.
    const DAYS_OF_FOOD_THEY_WALK_IN_WITH: f32 = 2.0;

    /// Whether this agent can do a step at all.
    ///
    /// Everything a stone-age people arrives knowing is `obvious`; everything
    /// past that is a thing somebody had to find out, and only the people who
    /// found it out - or were shown - can do it.
    pub fn knows_how_to(&self, step: &crate::environment::making::Making) -> bool {
        step.obvious || self.found_out.contains(step.makes)
    }

    /// Whether this agent knows any way at all of making a named thing.
    pub fn knows_how_to_make(&self, what: &str) -> bool {
        crate::environment::making::every_way_to_make(what).any(|step| self.knows_how_to(step))
    }

    /// Write down that this agent has found out how to do something.
    ///
    /// Returns false if it already knew.
    /// Put a question to the world and start waiting on the answer.
    pub fn now_i_wonder(&mut self, wondering: super::wondering::Wondering) {
        if self.am_i_wondering_about(&wondering.did, &wondering.what) {
            return;
        }

        // Nobody holds more than a few questions open at once. The oldest
        // goes, which is also the one most likely to have been answered by
        // somebody walking off with the thing.
        while self.wonderings.len() >= Self::AS_MANY_QUESTIONS_AS_ANYBODY_HOLDS {
            self.wonderings.remove(0);
        }

        self.wonderings.push(wondering);
    }

    /// Whether this one already has this question open.
    pub fn am_i_wondering_about(&self, did: &str, what: &str) -> bool {
        self.wonderings
            .iter()
            .any(|wondering| wondering.did == did && wondering.what == what)
    }

    /// Whether this one has put this question often enough to have a view.
    ///
    /// Not "has ever got an answer", which was the first cut of this and made
    /// the whole mechanism useless: one answer is one afternoon, and one
    /// afternoon can no more tell you what becomes of meat left out than one
    /// throw can tell you what a dice does. What is being found out here is
    /// that it depends on the weather, and *that* cannot be found out at all
    /// without leaving meat out in several sorts of weather.
    ///
    /// Measured, the first cut asked and answered sixty-five questions a world
    /// and drew exactly nought conclusions from them, because no agent ever
    /// held more than a single instance of any of them.
    pub fn do_i_know_what_becomes_of(&self, did: &str, what: &str) -> bool {
        self.lessons.tried_this(&format!("{did}:{what}")) >= Self::ENOUGH_TIMES_TO_HAVE_A_VIEW
    }

    /// How many times somebody has to have left a thing out before they stop
    /// wondering what becomes of it.
    ///
    /// Enough that both the wet afternoons and the dry ones have a run behind
    /// them, because the answer this is reaching for is not "it goes off" but
    /// "it goes off *in the rain*".
    const ENOUGH_TIMES_TO_HAVE_A_VIEW: u32 = 20;

    /// Something in the pack this one would leave somewhere to see what
    /// becomes of it.
    ///
    /// The whole of "what happens if I leave meat in the rain". Three things
    /// have to be true and all three matter: it has to be something whose
    /// fate this one has never watched, there has to be more than one of it
    /// so the experiment does not cost the experimenter its dinner, and it
    /// has to be food, because a lump of flint left in a field is a lump of
    /// flint in a field a week later and nobody learns anything.
    ///
    /// What it emphatically does *not* check is the weather. The branch this
    /// replaces would only put something down under a clear sky, which is to
    /// say the code already knew the answer and only let anybody run the
    /// experiment on the days it comes out well. Finding out that meat left
    /// in the rain is ruined is the *same discovery* as finding out that meat
    /// left in the sun keeps, and a people that can only make the second one
    /// has not found anything out at all.
    pub fn what_i_would_leave_out(&self) -> Option<String> {
        self.inventory
            .get_all_items()
            .iter()
            .map(|(_, item)| item)
            .filter(|item| Self::is_it_worth_watching(item))
            .filter(|item| item.quantity > Self::MORE_THAN_ANYBODY_WOULD_RISK)
            .filter(|item| !self.am_i_wondering_about(Self::LEAVING_IT_OUT, &item.item_id))
            .filter(|item| !self.do_i_know_what_becomes_of(Self::LEAVING_IT_OUT, &item.item_id))
            .map(|item| item.item_id.clone())
            .next()
    }

    /// Whether this one would put a question to the world about this thing,
    /// having done this to it.
    ///
    /// The same three conditions for every verb: not already asking it, not
    /// already sure of the answer, and something whose fate is worth watching.
    pub fn would_i_wonder_what_becomes_of(&self, did: &str, what: &str) -> bool {
        !self.am_i_wondering_about(did, what) && !self.do_i_know_what_becomes_of(did, what)
    }

    /// What asking after this thing would actually teach somebody.
    ///
    /// Not the thing itself - you cannot be handed a pot by being told about
    /// one. What passes between two people is the *name of the discovery* the
    /// thing depends on, which is exactly the gate the making machinery
    /// already checks: an agent who has that name can attempt the working, and
    /// an agent who has not cannot.
    ///
    /// Which means being told does not make anybody believe anything. It lets
    /// them go and try it, and what happens when they try it is what decides
    /// whether they believe it - which is how it should be, and is the whole
    /// difference between being told a thing works and finding out.
    pub fn what_asking_about_this_would_teach(item_id: &str) -> Option<String> {
        use crate::environment::making;

        // Something somebody worked out how to make
        if making::EVERY_WORKING
            .iter()
            .any(|working| working.makes == item_id && !working.obvious)
        {
            return Some(item_id.to_string());
        }

        if making::how_to_make(item_id).is_some_and(|step| !step.obvious) {
            return Some(item_id.to_string());
        }

        None
    }

    /// Whether leaving this somewhere could come to anything.
    ///
    /// Food, because the weather and the ground get at it. And clay, because
    /// a fire does - a lump left in the embers is not a lump of clay in the
    /// morning, which is the one thing in this world that a *material* left
    /// lying about turns into something else. A flint left in a field is a
    /// flint in a field a week later and nobody learns anything.
    fn is_it_worth_watching(item: &InventoryItem) -> bool {
        item.food_data.is_some() || item.item_id == Self::THE_ONE_MATERIAL_A_FIRE_CHANGES
    }

    /// What an experiment is called when it is somebody leaving a thing
    /// somewhere and coming back to look.
    pub const LEAVING_IT_OUT: &'static str = "leave";

    /// Clay, and nothing else so far.
    pub const THE_ONE_MATERIAL_A_FIRE_CHANGES: &'static str = "clay";

    /// How many of a thing somebody has to have before they will spare one to
    /// find something out. Curiosity is not hunger and must not act like it.
    const MORE_THAN_ANYBODY_WOULD_RISK: u32 = 2;

    /// And how many questions anybody keeps open at once.
    const AS_MANY_QUESTIONS_AS_ANYBODY_HOLDS: usize = 4;

    pub fn found_out_how_to(&mut self, what: &str) -> bool {
        self.found_out.insert(what.to_string())
    }

    /// Everything this agent has found out that it was not born knowing.
    pub fn what_i_found_out(&self) -> &std::collections::BTreeSet<String> {
        &self.found_out
    }

    /// What a person does not have to be shown.
    ///
    /// **Laying food out to keep it.** It had to be watched happening before
    /// anybody would do it on purpose, and the only route to watching it was a
    /// branch that fired when somebody happened to put food down on a clear
    /// day. That made preserving a thing a settlement stumbled into rather
    /// than a thing it did, and it is why 86% of what went into the ground
    /// went in raw and 98.4% of it rotted - see ISSUES_FOUND.md #124.
    ///
    /// Drying is not a discovery on the scale of smelting. Every people that
    /// has ever had a summer has known that a thing left in the sun goes hard
    /// rather than green, and a model in which a settlement can fail to work
    /// it out for a generation is not modelling ignorance, it is modelling an
    /// accident of where the branch sat. What is still discovered is
    /// everything the making chain calls `Making::obvious == false`.
    pub fn what_anybody_is_born_knowing() -> std::collections::BTreeSet<String> {
        [Self::THAT_LAYING_IT_OUT_KEEPS_IT.to_string()]
            .into_iter()
            .collect()
    }

    /// How an opinion about one of the strange plants is written down
    fn what_i_call_that_plant(kind: u8, good: bool) -> String {
        format!("plant:{kind}:{}", if good { "good" } else { "bad" })
    }

    /// Whether this agent has any opinion at all about that plant.
    ///
    /// Nobody is born with one. What settles it is somebody eating one and
    /// either being fed by it or being ill.
    pub fn have_i_tried_that_plant(&self, kind: u8) -> bool {
        self.found_out
            .contains(&Self::what_i_call_that_plant(kind, true))
            || self
                .found_out
                .contains(&Self::what_i_call_that_plant(kind, false))
    }

    /// Whether this agent believes that plant is food.
    pub fn is_that_plant_food(&self, kind: u8) -> bool {
        self.found_out
            .contains(&Self::what_i_call_that_plant(kind, true))
    }

    /// Write down what that plant turned out to be.
    pub fn now_i_know_that_plant(&mut self, kind: u8, good: bool) {
        self.found_out
            .insert(Self::what_i_call_that_plant(kind, good));
    }


    /// The work a pair of hands wants to be equipped for, in the order it
    /// wants them.
    ///
    /// Hunting first: a spear is the difference between eating meat and not.
    /// Then cutting wood, then cutting meat, then something to carve and
    /// something to dig with.
    ///
    /// This is stated as the work rather than as the tool because what the
    /// best tool for a job *is* changes as a people finds things out. A man
    /// who has never seen metal wants a stone knife; a man who has wants a
    /// metal one, and the same line of code asks for both.
    pub const WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO: [super::SkillType; 5] = [
        super::SkillType::Hunting,
        super::SkillType::Woodcutting,
        super::SkillType::Leatherworking,
        // Crafting and mining were not on this list, and nothing else in the
        // model ever wanted a tool for either. Measured directly: of thirty-one
        // people, **twenty-six wanted a vessel and could make one, twenty-eight
        // held the wood for it, and four owned anything to carve with.** The
        // block on the whole fluid family was never the order of the working
        // table or where the branch sat in the decision - it was that a pair of
        // hands never once thought to want a knife for carving.
        //
        // The same for mining: "nothing in hand that is any use for Mining" was
        // six hundred refused turns a world at the digging alone.
        super::SkillType::Crafting,
        super::SkillType::Mining,
    ];

    /// The best tool this agent knows how to make for a kind of work, if it
    /// would be an improvement on what it already carries.
    pub fn what_i_would_rather_have(
        &self,
        trade: super::SkillType,
    ) -> Option<&'static crate::environment::making::Tool> {
        let good_enough = self
            .what_i_have_to_work_with(trade)
            .map(|tool| tool.how_much_better)
            .unwrap_or(1.0);

        crate::environment::making::what_helps_with(trade)
            .filter(|tool| self.knows_how_to_make(tool.called))
            .filter(|tool| tool.how_much_better > good_enough)
            .max_by(|a, b| {
                a.how_much_better
                    .partial_cmp(&b.how_much_better)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Something this agent has found out how to do and could do right now.
    ///
    /// A man who has just worked out what a fire does to a bright stone will
    /// do it again to see it happen, with no use in mind for what comes out.
    /// That is how the next link in the chain gets into his hands at all:
    /// nobody can want a metal knife before anybody has seen a metal blade.
    pub fn what_i_would_try_out(&self) -> Option<String> {
        let holding = |what: &str| self.how_many_i_have(what);

        crate::environment::making::everything_to_find_out()
            .filter(|step| self.knows_how_to(step))
            .filter(|step| holding(step.makes) < crate::environment::making::A_FEW_SPARE)
            .filter(|step| {
                step.wants_in_hand
                    .is_none_or(|wanted| self.how_many_i_have(wanted) > 0)
            })
            .find(|step| step.makings_to_hand(&holding))
            .map(|step| step.makes.to_string())
    }

    /// Something in the pack worth turning over in the hands.
    ///
    /// A thing this agent is carrying that goes into a step or a working
    /// nobody here has worked out. Looking closely at it costs a turn and no
    /// materials, which makes it the cheapest way into the chain and the one
    /// that has to pay off least often - see
    /// `Simulation::WHAT_LOOKING_CLOSELY_IS_WORTH`.
    pub fn what_i_would_look_at(&self) -> Option<String> {
        use crate::environment::making;

        let unfamiliar = |what: &str| {
            // A thing that is already part of something everybody understands
            // raises no questions, however much else it goes into
            if making::is_a_familiar_thing(what) {
                return false;
            }

            making::everything_to_find_out()
                .filter(|step| step.needs.iter().any(|(needs, _)| *needs == what))
                .map(|step| step.makes)
                .chain(
                    making::every_working_to_find_out()
                        .filter(|working| working.to == what)
                        .map(|working| working.makes),
                )
                .any(|makes| !self.found_out.contains(makes))
        };

        self.inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .map(|item| item.item_id.as_str())
            .filter(|what| unfamiliar(what))
            .filter(|what| {
                self.lessons
                    .will_try_this_again(&format!("examine:{what}"))
            })
            .map(|what| what.to_string())
            .next()
    }

    /// Something in the pack worth breaking down, and what to break it with.
    ///
    /// Returns the verb and the thing it is done to. Only workings this agent
    /// knows, only where there is enough to work with, and only where there is
    /// not already a pile of what it produces — a man does not spend his life
    /// smashing cores when he has three flakes he has not used.
    ///
    /// What the verb wants in the hand is not asked here. That is the verb
    /// matrix's business, and it is asked once, before the action runs.
    pub fn what_i_would_work_on(&self) -> Option<(String, String)> {
        use crate::environment::making;

        let could: Vec<&'static making::Working> = making::EVERY_WORKING
            .iter()
            .filter(|working| working.obvious || self.found_out.contains(working.makes))
            .filter(|working| self.how_many_i_have(working.to) >= working.how_much)
            .filter(|working| self.how_much_water_i_carry() >= working.wants_water)
            .filter(|working| self.how_many_i_have(working.makes) < making::A_FEW_SPARE)
            .filter(|working| {
                self.lessons
                    .will_try_this_again(&format!("{}:{}", working.verb, working.to))
            })
            .collect();

        if could.is_empty() {
            return None;
        }

        // Where a man starts in the list is his own business.
        //
        // This took the *first* thing in the table it could do and stopped, so
        // the order of a hand-written list decided what a whole people ever
        // made. Carving a bowl sits late in that table: anything earlier with
        // materials to hand won every turn, and measured, a settlement made
        // essentially no vessels at all however badly it wanted one - no
        // carried water, no boiling, no salt.
        //
        // `what_working_i_would_try_out` hit exactly this and fixed it exactly
        // this way, for exactly this reason: retting flax sits above fermenting
        // fruit, so over eight worlds nobody ever fermented anything, because
        // somebody always had flax. The fix did not get carried across to its
        // neighbour.
        let mine = (self.id.as_u128() % could.len() as u128) as usize;

        could
            .get(mine)
            .map(|working| (working.verb.to_string(), working.to.to_string()))
    }

    /// A carcass in the pack that has to come apart before it is supper.
    ///
    /// "How are agents eating meat? Can they just absorb an entire side of
    /// beef? Should they not have to cut it into smaller pieces so they can
    /// cook and eat it?" - and until this existed they could and they did: a
    /// kill dropped two-kilo lumps of raw beast and one `Eat` swallowed one.
    ///
    /// This is deliberately not `what_i_would_work_on`. That one is about
    /// Utility, waits on the lessons, and stops once there are a few spare;
    /// this is a step on the way to a meal and answers to Hunger, so it does
    /// not wait on anything. Everybody is born knowing a carcass comes apart.
    pub fn what_flesh_i_should_cut_up(&self) -> Option<(String, String)> {
        use crate::environment::making;
        use crate::world::nutrition::Piece;

        // Only bother if there is nothing already cut that would do. A man
        // with a joint in his hand does not stop to quarter the rest of the
        // deer before he eats.
        if self.find_best_food_to_eat().is_some() {
            return None;
        }

        // And only with something to do it with. `cut` wants an edge, and the
        // matrix enforces that before the action runs - so choosing this
        // without one spends the turn and comes straight back refused, which
        // is exactly what cost a settlement half its winter store last time.
        if self
            .what_i_have_to_work_with(super::SkillType::Leatherworking)
            .is_none()
        {
            return None;
        }

        self.inventory
            .items
            .iter()
            .filter(|(_, item)| item.quantity > 0)
            .filter(|(id, _)| Piece::of(id) == Piece::Whole)
            .filter(|(_, item)| {
                // Not one that has already turned. Cutting up carrion is a
                // waste of an edge and a turn.
                item.food_data
                    .as_ref()
                    .is_none_or(|food| !food.is_spoiled() && !food.is_harmful())
            })
            .find_map(|(id, item)| {
                making::how_to_work("cut", id)
                    .filter(|working| working.obvious || self.found_out.contains(working.makes))
                    .filter(|working| item.quantity >= working.how_much)
                    .map(|working| (working.verb.to_string(), working.to.to_string()))
            })
    }

    /// A joint in the pack worth cutting down into strips, because it is not
    /// going to be eaten today.
    ///
    /// The counterpart to `what_flesh_i_should_cut_up`, and the difference
    /// between them is what the food is *for*. A hungry man quarters a deer
    /// so he can eat it. A man laying in for the winter cuts the joint down
    /// thin, because a strip is dry in two days and a joint takes most of a
    /// week - and a thing that is dry keeps twenty times as long as a thing
    /// that is not.
    ///
    /// Only for somebody who knows what laying a thing out would do. Cutting
    /// flesh into strips is a great deal of work for no reason at all if you
    /// have never seen what the sun does to it afterwards.
    pub fn what_i_would_cut_down_for_keeping(&self) -> Option<(String, String)> {
        use crate::environment::making;
        use crate::world::nutrition::Piece;

        if !self.found_out.contains(Self::THAT_LAYING_IT_OUT_KEEPS_IT) {
            return None;
        }

        if self
            .what_i_have_to_work_with(super::SkillType::Leatherworking)
            .is_none()
        {
            return None;
        }

        self.inventory
            .items
            .iter()
            .filter(|(id, _)| Piece::of(id) == Piece::Portion)
            .filter(|(id, _)| Piece::is_it_flesh(id))
            .filter(|(_, item)| {
                // Still worth keeping. There is no sense drying carrion.
                item.food_data.as_ref().is_some_and(|food| {
                    food.preparation == crate::world::nutrition::PreparationState::Raw
                        && food.freshness >= Self::WORTH_PUTTING_BY
                })
            })
            .find_map(|(id, item)| {
                making::how_to_work("cut", id)
                    .filter(|working| working.obvious || self.found_out.contains(working.makes))
                    .filter(|working| item.quantity >= working.how_much)
                    .map(|working| (working.verb.to_string(), working.to.to_string()))
            })
    }

    /// And something in the pack nobody here has ever thought to break down.
    ///
    /// The cheapest experiment a person can run: the materials are in the pack
    /// and the tool is in the hand whatever happens, so what it costs is one
    /// stick and an afternoon.
    pub fn what_working_i_would_try_out(&self, a_fire_is_to_hand: bool) -> Option<(String, String)> {
        use crate::environment::making;

        let could: Vec<&'static making::Working> = making::every_working_to_find_out()
            .filter(|working| !self.found_out.contains(working.makes))
            // Not something that wants a fire, when there is no fire. The
            // executor refuses those, and an experiment that comes straight
            // back refused is a turn gone - which is exactly what cost a
            // settlement half its winter store two batches ago.
            .filter(|working| !working.over_a_fire || a_fire_is_to_hand)
            .filter(|working| self.how_many_i_have(working.to) >= working.how_much)
            .filter(|working| self.how_much_water_i_carry() >= working.wants_water)
            .filter(|working| {
                self.lessons
                    .will_try_this_again(&format!("{}:{}", working.verb, working.to))
            })
            .collect();

        if could.is_empty() {
            return None;
        }

        // Which of them this particular person is the one to try. Taking the
        // first of the list meant the order of the table decided what a whole
        // people ever found out: retting flax sits above fermenting fruit, so
        // over eight worlds nobody ever fermented anything, because somebody
        // always had flax. Where a man starts in the list is his own business.
        let mine = (self.id.as_u128() % could.len() as u128) as usize;

        could
            .get(mine)
            .map(|working| (working.verb.to_string(), working.to.to_string()))
    }

    /// A step it knows, with the wrong thing where a part should go.
    ///
    /// "Knowing that a stone tool requires the use of specific sub-components,
    /// an agent might substitute known sub-components for new/random things."
    ///
    /// Returns what was being attempted, the part left out, and what went in
    /// instead. It picks a step whose other parts are all to hand, so the man
    /// is genuinely one component short and has genuinely got something else,
    /// and it will not offer a substitution this agent has already tried and
    /// found useless - see `Lessons::will_try_this_again`.
    pub fn what_i_would_swap(&self) -> Option<(String, String, String)> {
        use crate::environment::making;

        for step in making::EVERY_STEP.iter().filter(|step| self.knows_how_to(step)) {
            if self.how_many_i_have(step.makes) >= making::A_FEW_SPARE {
                continue;
            }

            if step
                .wants_in_hand
                .is_some_and(|wanted| self.how_many_i_have(wanted) == 0)
            {
                continue;
            }

            for (left_out, _) in step.needs {
                // Everything else the step wants has to be in the pack, or
                // this is not a substitution, it is a wish
                let rest_to_hand = step.needs.iter().all(|(what, how_many)| {
                    what == left_out || self.how_many_i_have(what) >= *how_many
                });

                if !rest_to_hand {
                    continue;
                }

                for stack in self.inventory.get_all_items().values() {
                    if stack.quantity == 0 {
                        continue;
                    }

                    let put_in = stack.item_id.as_str();

                    // Not a thing the step already wants, and not the part
                    // that is missing
                    if step.needs.iter().any(|(what, _)| *what == put_in) {
                        continue;
                    }

                    let called =
                        making::what_that_swap_is_called(step.makes, left_out, put_in);

                    if !self.lessons.will_try_this_again(&called) {
                        continue;
                    }

                    return Some((
                        step.makes.to_string(),
                        left_out.to_string(),
                        put_in.to_string(),
                    ));
                }
            }
        }

        None
    }

    /// The thing this agent would put its hands to now, if anything.
    ///
    /// Not the thing it wants - the step towards it that today's pack allows.
    /// A man who wants a spear and holds flax and stone is told to twist
    /// cordage, and holds a spear three turns later. Nothing here asks for a
    /// thing already carried: two spears are no better than one until the
    /// first one breaks.
    /// A vessel this agent would rather have than not, if it has none.
    ///
    /// **Nothing in this world had ever wanted one.** `what_i_would_make` asks
    /// only after tools - something to hunt with, something to cut wood with,
    /// something to work a hide with - so a bowl and a fired pot both declared
    /// what they hold and neither was ever made by anybody. Which meant no
    /// agent could carry water, so every drink was a walk to the river and
    /// back; and it meant `Boil` was refused for want of something to hold the
    /// sea in **two hundred and forty-seven times a world**, so salt was
    /// effectively unreachable too.
    ///
    /// A vessel is the plainest preparation there is. The trip to the water is
    /// the cost and the water is free, so a person who owns something to carry
    /// it in pays that cost once and drinks for days.
    pub fn what_vessel_i_would_rather_have(&self) -> Option<(&'static str, &'static str)> {
        if self.what_i_can_hold_water_in() > 0 {
            return None;
        }

        crate::environment::making::EVERY_WORKING
            .iter()
            .filter(|working| working.holds.is_some_and(|held| held > 0.0))
            .filter(|working| working.obvious || self.found_out.contains(working.makes))
            .filter(|working| self.how_many_i_have(working.to) >= working.how_much)
            .max_by(|a, b| {
                a.holds
                    .unwrap_or(0.0)
                    .partial_cmp(&b.holds.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|working| (working.verb, working.to))
    }

    /// How many things this agent has that will hold a liquid.
    pub fn what_i_can_hold_water_in(&self) -> u32 {
        self.inventory
            .get_all_items()
            .values()
            .filter(|item| item.is_container())
            .map(|item| item.quantity)
            .sum()
    }

    pub fn what_i_would_make(&self, a_fire_is_to_hand: bool) -> Option<String> {
        let holding = |what: &str| self.how_many_i_have(what);

        let knows = |step: &crate::environment::making::Making| self.knows_how_to(step);

        // Owning it, not holding it: the executor asks for ownership and the
        // tool gets taken in hand on the way - see `get_the_tool_out_for`.
        let in_hand = |what: &str| self.how_many_i_have(what) > 0;

        Self::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO
            .iter()
            .filter_map(|trade| self.what_i_would_rather_have(*trade))
            .find_map(|want| {
                // Only a step that could actually be carried out. Asking for
                // one that could not is worse than a wasted turn: the refusal
                // goes into the record, and a man learns from it that making
                // knives does not work.
                crate::environment::making::what_to_do_first_that_can_be_done(
                    want.called,
                    &holding,
                    &knows,
                    &in_hand,
                    a_fire_is_to_hand,
                )
            })
            .map(|step| step.makes.to_string())
    }

    /// How many usable ones of a named thing are in the pack.
    ///
    /// A worn-through tool does not count. A broken axe is not an axe: it is
    /// carried about as a reason to make another one, and every question of
    /// the form "have I got one of these" should answer no.
    pub fn how_many_i_have(&self, what: &str) -> u32 {
        self.inventory
            .get_item(what)
            .filter(|item| item.durability_percentage() > 0.0)
            .map(|item| item.quantity)
            .unwrap_or(0)
    }

    /// The tool in this agent's pack that helps most with a kind of work.
    ///
    /// A worn-through tool is no tool: it stays in the pack as a reminder
    /// that a new one is wanted - see the `broken` count in `read_the_room` -
    /// but it does no work.
    pub fn what_i_have_to_work_with(
        &self,
        trade: super::SkillType,
    ) -> Option<&'static crate::environment::making::Tool> {
        crate::environment::making::what_helps_with(trade)
            .filter(|tool| {
                self.inventory
                    .get_item(tool.called)
                    .is_some_and(|item| item.quantity > 0 && item.durability_percentage() > 0.0)
            })
            .max_by(|a, b| {
                a.how_much_better
                    .partial_cmp(&b.how_much_better)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// How much better a well made tool is than a badly made one, at the
    /// extremes.
    ///
    /// `Quality::modifier` runs from 0.5 to 2.0, which put on top of the
    /// tool's own multiplier would make an expert's axe worth two and a half
    /// men. This is the band it is squeezed into: the tenth spear a man makes
    /// is half again the spear his first was, and no more than that.
    const WHAT_GOOD_WORK_IS_WORTH: (f32, f32) = (0.7, 1.5);

    /// What having the right tool multiplies a piece of work by.
    ///
    /// One if there is nothing to hand, which is what a pair of bare hands
    /// gets. A tool most of the way through its life is most of the way back
    /// towards bare hands, and a tool that was badly made was never much of
    /// one - "repeating the action increases the quality of the outcome",
    /// which is only true of anything if the quality is worth having.
    /// How much of a blow gets through what is in this agent's hands.
    ///
    /// "Defend with" is a verb nobody chooses. It is what happens when
    /// something comes at you and there is a shaft in your hand, and it is why
    /// carrying a spear is worth something to a man who never hunts. What it
    /// wants in the hand is declared in the verb matrix like everything else;
    /// a man with nothing in his hands simply does not do it, and takes the
    /// whole of what is coming.
    ///
    /// Putting a shaft in the way of something is hard on the shaft, so the
    /// caller is expected to wear it afterwards.
    pub fn what_a_blow_costs_me(&self, coming: f32) -> f32 {
        let turned = self.how_much_my_tools_help(super::SkillType::MeleeCombat);
        coming / turned.max(1.0)
    }

    /// How much liquid this agent is carrying in vessels.
    ///
    /// What the fluid family runs on. Nobody could carry any of it until
    /// somebody worked out how to hollow out a bowl.
    pub fn how_much_water_i_carry(&self) -> f32 {
        self.inventory
            .get_all_items()
            .values()
            .filter(|item| item.is_container())
            .filter_map(|item| item.fill_level)
            .sum()
    }

    /// Take that much liquid out of what is being carried, and say how much
    /// actually came out.
    pub fn draw_from_what_i_carry(&mut self, wanted: f32) -> f32 {
        let mut still_wanting = wanted;
        let mut got = 0.0;

        for item in self.inventory.get_all_items_mut().values_mut() {
            if still_wanting <= 0.0 {
                break;
            }
            if !item.is_container() {
                continue;
            }

            let came = item.drain(still_wanting);
            still_wanting -= came;
            got += came;
        }

        got
    }

    /// Whether there is a hand free to take hold of something with.
    ///
    /// The first cut of this counted a hand as full for every kind of tool in
    /// the pack, so a man who owned an axe and a spear had no hands at all and
    /// could never stitch a coat again. The second guessed at it from how
    /// loaded the pack was, which was a fudge and was written down as one.
    ///
    /// There are two hands now and they hold particular things, so this is
    /// simply a question about them.
    ///
    /// The pack does not come into it, and the third cut of this was the one
    /// that found out why. A settlement lives at or over the limit of what it
    /// can carry - measured mean load 70 against a capacity of 50 - so a rule
    /// that wanted spare capacity meant nobody in the model ever had a hand
    /// free for anything. What a heavy pack costs is paid in the walking, by
    /// `Simulation::what_this_load_costs`, which is where it belongs.
    pub fn a_hand_to_spare(&self) -> bool {
        self.hands.iter().any(|hand| hand.is_none())
    }

    /// Whether this particular thing is out and in a hand.
    pub fn is_in_my_hand(&self, what: &str) -> bool {
        self.hands
            .iter()
            .any(|hand| hand.as_deref() == Some(what))
    }

    /// What is out, in whichever order the hands happen to be in.
    pub fn what_is_in_my_hands(&self) -> impl Iterator<Item = &str> {
        self.hands.iter().filter_map(|hand| hand.as_deref())
    }

    /// Take something out of the pack and into a hand.
    ///
    /// Refuses what is not in the pack, what is already out, and anything at
    /// all when both hands are full. Nothing leaves the pack: a hand is a
    /// claim on a thing rather than a second place to keep it, which is what
    /// keeps a thing from existing twice.
    pub fn take_in_hand(&mut self, what: &str) -> bool {
        if self.is_in_my_hand(what) {
            return false;
        }

        if self.how_many_i_have(what) == 0 {
            return false;
        }

        let Some(free) = self.hands.iter().position(|hand| hand.is_none()) else {
            return false;
        };

        self.hands[free] = Some(what.to_string());
        true
    }

    /// Put a thing back in the pack, freeing the hand.
    pub fn put_away(&mut self, what: &str) -> bool {
        let Some(held) = self
            .hands
            .iter()
            .position(|hand| hand.as_deref() == Some(what))
        else {
            return false;
        };

        self.hands[held] = None;
        true
    }

    /// A hand does not go on holding what the owner no longer has.
    ///
    /// Anything given away, traded, stolen, worn out or eaten leaves the pack
    /// through the inventory, which knows nothing about hands - so the hands
    /// are checked against the pack rather than being told.
    pub fn let_go_of_what_i_no_longer_have(&mut self) {
        for hand in 0..self.hands.len() {
            let gone = self.hands[hand]
                .as_deref()
                .is_some_and(|what| self.inventory.get_item(what).is_none_or(|item| item.quantity == 0));

            if gone {
                self.hands[hand] = None;
            }
        }
    }

    /// What a pair of bare hands manages at a trade, against a whole one.
    ///
    /// "Many actions can be completed by the agent, but without tools, these
    /// actions are not very efficient." This was one for every trade, so a man
    /// with nothing was a fully competent workman and every tool in the model
    /// was a bonus on top of competence. That is why the ladder measured null:
    /// there was nothing wrong with the bottom of it.
    ///
    /// The figures are the specification's own reading of each job. Fishing
    /// "can be accomplished by hand but is highly inefficient" - a man standing
    /// in a river grabbing at trout. Digging without a tool "should take a
    /// significant amount of time". Butchering is the hard one: "killing any
    /// animal without at least a stone hand axe makes it nearly impossible to
    /// eat the dead animal", so bare hands get almost nothing off a carcass.
    ///
    /// Picking is the exception and is nearly whole, because hands are what
    /// picking is *for*; what a digging stick adds is roots, not berries.
    pub fn what_bare_hands_manage(trade: super::SkillType) -> f32 {
        use super::SkillType;

        match trade {
            // Hands were made for this
            SkillType::Herbalism => 0.85,

            // Grabbing at fish in a river
            SkillType::Fishing => 0.25,

            // Throwing stones at something that runs faster than you
            SkillType::Hunting => 0.3,

            // Tearing at a carcass: nearly impossible
            SkillType::Leatherworking => 0.15,

            // Scraping a hole out of the ground, breaking wood by hand
            SkillType::Mining => 0.3,
            SkillType::Woodcutting => 0.25,

            // Work that is mostly the hands anyway, hindered rather than
            // stopped by having nothing in them
            SkillType::Crafting | SkillType::Construction | SkillType::Farming => 0.6,

            // Everything with no tool in the world behind it is unchanged, or
            // this would quietly tax half the model for no stated reason
            _ => 1.0,
        }
    }

    pub fn how_much_my_tools_help(&self, trade: super::SkillType) -> f32 {
        let bare_hands = Self::what_bare_hands_manage(trade);

        let Some(tool) = self.what_i_have_to_work_with(trade) else {
            return bare_hands;
        };

        let Some(carried) = self.inventory.get_item(tool.called) else {
            return bare_hands;
        };

        let left = carried.durability_percentage();
        let (worst, best) = Self::WHAT_GOOD_WORK_IS_WORTH;
        let how_well_made = carried
            .quality
            .map(|quality| quality.modifier().clamp(worst, best))
            .unwrap_or(1.0);

        // An axe in the pack is an axe you have to stop and dig out. It still
        // works - a person is not helpless because the thing is in the bag -
        // but a tool already in the hand is worth appreciably more, and that
        // difference is the whole reason anybody bothers to take one out.
        let out = if self.is_in_my_hand(tool.called) {
            1.0
        } else {
            Self::WHAT_A_TOOL_STILL_IN_THE_PACK_IS_WORTH
        };

        // A blunt axe is still an axe, so half the gain survives to the end
        // of its life and the other half wears away with it.
        1.0 + (tool.how_much_better - 1.0) * (0.5 + 0.5 * left) * how_well_made * out
    }

    /// What a tool you have not got out is worth against one you have.
    ///
    /// Not nothing: you can reach into a bag mid-job. The first cut put it at
    /// two thirds and that was much too harsh - a person can only hold two
    /// things and a working settlement owns four or five, so most work is
    /// done with something fetched out of the bag, and taxing all of it a
    /// sixth cost the settlement a quarter of its standing crop and 40 per
    /// cent of its tools. It is a small edge, not a penalty for being
    /// organised.
    pub const WHAT_A_TOOL_STILL_IN_THE_PACK_IS_WORTH: f32 = 0.9;

    /// What to let go of when both hands are full and the job wants one free.
    ///
    /// The least useful of what is being held, by the same reckoning that
    /// decided to pick it up.
    pub fn what_i_would_put_away(&self) -> Option<String> {
        use crate::environment::making;

        let worth = |what: &str| {
            making::EVERY_TOOL
                .iter()
                .find(|tool| tool.called == what)
                .map(|tool| self.skills.hand_for(tool.helps) * tool.how_much_better)
                // Something in the hand that is not a tool at all is the
                // first thing to go
                .unwrap_or(0.0)
        };

        self.what_is_in_my_hands()
            .min_by(|a, b| {
                worth(a)
                    .partial_cmp(&worth(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|what| what.to_string())
    }

    /// Wear the tool used for a piece of work, and say if it broke.
    ///
    /// Stone and wood go quickly. A spear is twenty-five or so hunts and a
    /// handaxe forty trips out for timber, less if it was badly made, which
    /// is why a people that cannot make tools well stays a people that cannot
    /// do very much.
    pub fn wear_what_i_worked_with(&mut self, trade: super::SkillType) -> Option<String> {
        let tool = *self.what_i_have_to_work_with(trade)?;
        let hand = self.skills.hand_for(tool.helps);

        let (left, how_many) = {
            let item = self.inventory.get_item(tool.called)?;
            (item.current_durability.unwrap_or(0.0) - 1.0, item.quantity)
        };

        if left > 0.0 {
            self.inventory.get_item_mut(tool.called)?.current_durability = Some(left);
            return None;
        }

        // One of them is finished. If there is another in the pack it comes
        // out fresh; if there is not, what is left is a broken tool. The
        // finished one goes through `remove_item` rather than having its
        // count knocked down in place, so that what the agent is carrying
        // gets lighter along with it.
        if how_many > 1 {
            self.inventory.remove_item(tool.called, 1);
            let fresh = crate::environment::making::how_long_this_one_lasts(&tool, hand);
            if let Some(item) = self.inventory.get_item_mut(tool.called) {
                item.current_durability = Some(fresh);
                item.max_durability = Some(fresh);
            }
            None
        } else {
            self.inventory.get_item_mut(tool.called)?.current_durability = Some(0.0);
            Some(tool.called.to_string())
        }
    }

    /// A newly made tool, as good as the hands that made it.
    ///
    /// "Repeating the action increases the quality of the outcome and the
    /// skill of the agent accomplishing the action": the same man's tenth
    /// spear outlasts his first, because he is better at making spears.
    pub fn a_tool_fresh_from_these_hands(
        &self,
        called: &str,
        how_many: u32,
        weight: f32,
    ) -> super::InventoryItem {
        let mut made = super::InventoryItem::new_with_weight(called.to_string(), how_many, weight);

        if let Some(tool) = crate::environment::making::EVERY_TOOL
            .iter()
            .find(|tool| tool.called == called)
        {
            // The hand that matters is the one that does the making, not the
            // one that will use the thing: a spear is only as good as the man
            // who lashed it, whoever ends up throwing it.
            let trade = crate::environment::making::how_to_make(called)
                .map(|step| step.hands)
                .unwrap_or(tool.helps);
            let hand = self.skills.hand_for(trade);

            let lasts = crate::environment::making::how_long_this_one_lasts(tool, hand);
            made.current_durability = Some(lasts);
            made.max_durability = Some(lasts);
            made.quality = Some(super::skills::Quality::from_hand(hand));
        }

        made
    }

    /// Whether this agent has seen a named thing anywhere with its own eyes.
    ///
    /// Hearsay does not count here: it is about deciding to walk somewhere,
    /// and a man walks to a meadow he remembers.
    pub fn have_i_seen(&self, what: &str) -> bool {
        let Some(kind) = crate::world::ResourceType::called(what) else {
            return false;
        };

        self.exploration_knowledge
            .known_resources
            .iter()
            .any(|(where_it_is, found)| {
                *found == kind && !self.exploration_knowledge.who_told_me.contains_key(where_it_is)
            })
    }

    /// The raw thing this agent would go out and fetch, if anything.
    ///
    /// The other half of `what_i_would_make`. A man who wants a spear and can
    /// take no step towards one is not stuck: he is short of wood, or stone,
    /// or something fibrous, and the ground has all three on it.
    ///
    /// Which of them he goes after is decided by what he has actually seen.
    /// Taking the first thing the table named instead sent a whole people
    /// after flax whether or not any grew here: two thirds of every failed
    /// action in a settlement was `No flax sources nearby`.
    pub fn what_i_must_find(&self) -> Option<String> {
        let holding = |what: &str| self.how_many_i_have(what);

        let knows = |step: &crate::environment::making::Making| self.knows_how_to(step);

        let wanting: Vec<&'static str> = Self::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO
            .iter()
            .filter_map(|trade| self.what_i_would_rather_have(*trade))
            .flat_map(|want| {
                crate::environment::making::everything_wanting_knowing(
                    want.called,
                    &holding,
                    &knows,
                )
            })
            .collect();

        wanting
            .iter()
            .find(|what| self.have_i_seen(what))
            .or_else(|| wanting.first())
            .map(|what| what.to_string())
    }

    /// Set a founder up as somebody who has lived a life before this one.
    pub fn give_them_a_stone_age_start(&mut self) {
        use super::InventoryItem;

        for (trade, hand) in Self::WHAT_A_GROWN_PERSON_ARRIVES_KNOWING {
            // Never take a skill *down*: an agent that has somehow already
            // learned better keeps what it has
            let already = self
                .skills
                .get_skill_if_exists(trade)
                .map(|s| s.level)
                .unwrap_or(i32::MIN);
            if already < hand {
                self.skills.set_skill_level(trade, hand);
            }
        }

        for (what, how_many, each) in Self::WHAT_THEY_CARRY {
            let carried = self.a_tool_fresh_from_these_hands(what, how_many, each);
            self.inventory.add_item(carried);
        }

        // And the food they have been walking on. Counted off the body's own
        // arithmetic rather than a number picked here, so that if what a body
        // burns in a day changes, what a founder walks in with changes with it.
        let handfuls = (Self::DAYS_OF_FOOD_THEY_WALK_IN_WITH
            * super::physiology::UNITS_BURNED_IN_AN_ORDINARY_DAY
            / super::provision::UNITS_IN_ONE_STORED_ITEM)
            .round() as u32;
        let mut travelling_food = InventoryItem::new_with_weight("food".to_string(), handfuls, 0.5);
        travelling_food.food_data = crate::world::FoodDatabase::new()
            .create_food_data(&crate::world::ItemType::Food, 0);
        self.inventory.add_item(travelling_food);

        // And the basket goes on the back on the way in, rather than on the
        // first turn after arriving.
        self.take_up_the_cart();
    }

    pub const MATERIALS: [&'static str; 7] = [
        "wood", "stone", "iron", "clay", "sand", "coal", "brick",
    ];

    /// What counts as a tool
    const TOOLS: [&'static str; 9] = [
        "axe", "pick", "hoe", "shovel", "spade", "knife", "hammer", "tool", "spear",
    ];

    /// What counts as a fine or decorative thing
    const FINERY: [&'static str; 5] = ["jewel", "gold", "gem", "pottery", "ornament"];

    /// How far the agent can make out detail on the ground, in tiles.
    ///
    /// This is shorter than `Vision::detection_range`, which is about spotting
    /// movement at a distance: recognising a berry bush or a seam of ore is
    /// nearer work. Acuity scales it, so a blind agent gets zero and sees
    /// nothing of the world around it.
    ///
    /// It is set to outrange every smell food gives off where it lies - a
    /// berry carries two tiles, flesh six - because looking, not sniffing, is
    /// how a person finds dinner. The one thing that beats an eye is a cooking
    /// fire, which reaches just as far. See `ResourceType::raw_scent_strength`.
    pub fn sight_range(&self) -> u32 {
        /// How far an unimpaired agent recognises what it is looking at
        const BASE_SIGHT_RANGE: f32 = 25.0;

        if self.traits.has(crate::core::traits::Trait::Blind)
            || self.senses.vision.impaired
        {
            return 0;
        }

        (BASE_SIGHT_RANGE * self.senses.vision.acuity.clamp(0.0, 1.0)).round() as u32
    }

    /// Whether the agent can see at all
    pub fn can_see(&self) -> bool {
        self.sight_range() > 0
    }

    /// Apply trait-based sensory and physical modifications
    /// Should be called after traits are set/modified
    pub fn apply_trait_sensory_modifications(&mut self) {
        use crate::core::traits::Trait;

        // Deaf trait: Set hearing sensitivity to 0
        if self.traits.has(Trait::Deaf) {
            self.senses.hearing.sensitivity = 0.0;
            self.senses.hearing.set_impaired(true);
        }

        // Blind trait: no sight at all
        if self.traits.has(Trait::Blind) {
            self.senses.vision.acuity = 0.0;
            self.senses.vision.set_impaired(true);
        }

        // Suspicious trait: Increase noise curiosity rate (already handled in emotion_modifiers)
        // but we can also increase hearing sensitivity slightly
        if self.traits.has(Trait::Suspicious) {
            self.senses.hearing.sensitivity = (self.senses.hearing.sensitivity * 1.2).min(1.0);
        }

        // Uncaring trait: Reduce noise curiosity (reduce hearing sensitivity slightly)
        if self.traits.has(Trait::Uncaring) && !self.traits.has(Trait::Deaf) {
            self.senses.hearing.sensitivity *= 0.7;
        }
    }

    /// Check if agent is near a known religious building
    /// Returns the distance squared to the nearest religious building, or None if none known
    pub fn distance_to_nearest_religious_building(&self) -> Option<f32> {
        let pos = self.state.position;
        let mut nearest_dist_sq: Option<f32> = None;

        for (building_pos, building_type) in &self.exploration_knowledge.known_buildings {
            if building_type.is_religious() {
                let dx = (pos.0 - building_pos.x) as f32;
                let dy = (pos.1 - building_pos.y) as f32;
                let dist_sq = dx * dx + dy * dy;

                match nearest_dist_sq {
                    Some(current) if dist_sq < current => nearest_dist_sq = Some(dist_sq),
                    None => nearest_dist_sq = Some(dist_sq),
                    _ => {}
                }
            }
        }

        nearest_dist_sq
    }

    /// Apply location-based trait effects
    /// Called during tick to grant happiness based on agent's location
    pub fn apply_location_trait_effects(&mut self) {
        use crate::core::traits::Trait;
        use super::EmotionSource;

        const RELIGIOUS_PROXIMITY_SQ: f32 = 225.0; // 15 tiles squared

        // Zealot/Believer traits: happiness near religious buildings
        if self.traits.has(Trait::Zealot) || self.traits.has(Trait::Believer) {
            if let Some(dist_sq) = self.distance_to_nearest_religious_building() {
                if dist_sq <= RELIGIOUS_PROXIMITY_SQ {
                    // Closer = more happiness (max 0.05 at building, 0.01 at 15 tiles)
                    let proximity_factor = 1.0 - (dist_sq / RELIGIOUS_PROXIMITY_SQ);
                    let base_happiness = if self.traits.has(Trait::Zealot) { 0.05 } else { 0.03 };
                    let happiness = base_happiness * proximity_factor;

                    self.emotions.add_happiness(
                        EmotionSource::Event("religious_building_proximity".to_string()),
                        happiness
                    );
                }
            }
        }

        // Atheist trait: slight discomfort near religious buildings
        if self.traits.has(Trait::Atheist) {
            if let Some(dist_sq) = self.distance_to_nearest_religious_building() {
                if dist_sq <= 100.0 { // 10 tiles - closer range for discomfort
                    self.emotions.add_sadness(
                        EmotionSource::Event("religious_discomfort".to_string()),
                        0.01
                    );
                }
            }
        }

        // Greedy trait: happiness from having large personal inventory
        // "Happiness from supplies stored in home" - represented by personal inventory
        if self.traits.has(Trait::Greedy) {
            let item_count = self.inventory.total_items();
            if item_count >= 20 {
                // Scale happiness with inventory size (capped)
                let happiness = (item_count as f32 / 100.0).min(0.05);
                self.emotions.add_happiness(
                    EmotionSource::Event("wealth_satisfaction".to_string()),
                    happiness
                );
            }
        }

        // Survivalist trait: happiness when basic needs are well-met
        if self.traits.has(Trait::Survivalist) {
            // Check if energy, health, and food supplies are good
            let has_food = self.inventory.has_food(5); // Has at least 5 food items
            let well_fed = self.state.energy > 70.0;
            let healthy = self.state.health > 80.0;

            if has_food && well_fed && healthy {
                self.emotions.add_happiness(
                    EmotionSource::Event("self_sufficiency".to_string()),
                    0.02
                );
            }
        }

        // Frugal trait: contentment from having stored goods nearby
        if self.traits.has(Trait::Frugal) {
            // Similar to greedy but more modest - happiness from savings
            let item_count = self.inventory.total_items();
            if item_count >= 10 {
                self.emotions.add_happiness(
                    EmotionSource::Event("savings_contentment".to_string()),
                    0.01
                );
            }
        }
    }

    /// Apply building proximity effects (morale, healing, defense awareness)
    /// Called during tick to grant bonuses based on nearby buildings
    pub fn apply_building_proximity_effects(&mut self) {
        use super::EmotionSource;

        let pos = self.state.position;

        // Track cumulative effects
        let mut total_morale_bonus: f32 = 0.0;
        let mut healing_multiplier: f32 = 1.0;
        let mut defense_multiplier: f32 = 1.0;

        // Check all known buildings for proximity effects
        for (building_pos, building_type) in &self.exploration_knowledge.known_buildings {
            let dx = (pos.0 - building_pos.x) as f32;
            let dy = (pos.1 - building_pos.y) as f32;
            let distance = (dx * dx + dy * dy).sqrt();

            // Morale bonus from nearby buildings
            let morale_radius = building_type.morale_radius();
            if morale_radius > 0.0 && distance <= morale_radius {
                let proximity_factor = 1.0 - (distance / morale_radius);
                total_morale_bonus += building_type.morale_bonus() * proximity_factor;
            }

            // Healing bonus from medical buildings
            let healing_radius = building_type.healing_radius();
            if healing_radius > 0.0 && distance <= healing_radius {
                let bonus = building_type.healing_bonus();
                if bonus > healing_multiplier {
                    healing_multiplier = bonus; // Use best healing bonus, don't stack
                }
            }

            // Defense bonus from defensive buildings
            let defense_radius = building_type.defense_radius();
            if defense_radius > 0.0 && distance <= defense_radius {
                let bonus = building_type.defense_bonus();
                if bonus > defense_multiplier {
                    defense_multiplier = bonus; // Use best defense bonus, don't stack
                }
            }
        }

        // Apply morale bonus (capped at 0.05 per tick to avoid runaway happiness)
        if total_morale_bonus > 0.0 {
            let capped_bonus = total_morale_bonus.min(0.05);
            self.emotions.add_happiness(
                EmotionSource::Event("building_comfort".to_string()),
                capped_bonus
            );
        }

        // Store building bonuses for use by other systems
        // (healing bonus applied in regenerate_health, defense bonus applied in take_damage)
        self.cached_healing_bonus = healing_multiplier;
        self.cached_defense_bonus = defense_multiplier;
    }





    /// Update agent state (tick senses, body, emotions, memory, and drives)
    pub fn tick(&mut self) {
        self.tick_with_percepts(0); // Default tick uses tick 0
    }

    /// Tick with percept processing (requires current tick for timestamping)
    pub fn tick_with_percepts(&mut self, current_tick: u32) {
        // Update subsystems
        self.senses.tick();
        self.body.tick();
        // Use trait-aware emotion decay (traits affect how quickly emotions fade)
        self.emotions.tick_with_traits(&self.traits);
        // Apply passive trait effects (e.g., Melancholic slowly gains sadness)
        self.emotions.apply_passive_trait_effects(&self.traits);
        // Apply location-based trait effects (e.g., Zealot near religious buildings)
        self.apply_location_trait_effects();
        // Apply building proximity effects (morale, healing bonus, defense bonus)
        self.apply_building_proximity_effects();
        self.memory.tick();

        // And the trails fade. Charged by the day inside `fade`, so calling
        // it every turn costs a subtraction and takes nothing until a day
        // has actually gone by. A path nobody walks grows over, which is what
        // keeps an agent holding the corner of the world that has paid it
        // rather than all of the world it has ever seen.
        self.patterns.fade(current_tick);

        // Check for stale storage knowledge and trigger curiosity
        self.update_storage_curiosity(current_tick);

        // A cart in the pack is a cart in the hand. See `take_up_the_cart`.
        self.take_up_the_cart();

        // Update emotions based on drive states (every tick)
        self.update_emotions_from_drives();
        self.feel_what_the_habits_are_costing();
        // Drives rise differently depending on whether the agent has anything
        // more pressing on. See `DriveType::is_long_term`.
        let secure = self.immediate_needs_met();
        let situation = self.what_the_situation_asks();
        self.drives.tick_in(&situation, secure);

        // And worry presses on whatever it is worried for. A man who expects
        // his standing to suffer for what he has been doing attends to his
        // standing - which is worry making somebody act rather than merely
        // decline, and is the half of it that a subtraction in the pattern
        // layer cannot do. Added after the ordinary rise so that it is a
        // push on top of the need and not a replacement for it.
        for drive in self.drives.drives.iter_mut() {
            let worried = self
                .patterns
                .how_much_i_fear_for(drive.drive_type)
                .clamp(0.0, Self::THE_MOST_WORRY_CAN_ADD);
            if worried > 0.0 {
                drive.value = (drive.value + worried).min(1.0);
            }
        }

        // Hunger and thirst are not accumulated; they are read off the body.
        //
        // Every other drive builds at a rate somebody chose. These two do not
        // need to, because there is a stomach, a gut and a reserve to ask, and
        // asking them is the only way the drive and the body can agree about
        // when an agent is in trouble. Four separate spellings of that clock
        // disagreed before this - see ISSUES #73 - and the agent starved
        // holding a drive that had not yet noticed.
        // Hunger rises at the rate the three tables give, rather than being
        // read straight off the body: the tables are headed "Hunger Drive
        // Increase" and that is what they are. Thirst has no such table and is
        // still read directly.
        let how_fast_hunger_rises = self.state.physiology.how_fast_hunger_rises();
        let body_wants_water = self.state.physiology.thirst();
        if let Some(drive) = self.drives.get_mut(DriveType::Hunger) {
            let a_turn_of_it =
                DriveType::Hunger.base_accumulation_rate() * how_fast_hunger_rises;
            drive.value = (drive.value + a_turn_of_it).clamp(0.0, 1.0);
        }
        if let Some(drive) = self.drives.get_mut(DriveType::Thirst) {
            drive.value = body_wants_water;
        }

        // Process sensory input into percepts and store them
        let new_percepts = super::sensory_processing::process_sensory_input(&self.senses, self.state.position);

        // Store percepts with timestamp and integrate important ones into long-term memory
        for percept in new_percepts {
            // Calculate salience to determine if worth remembering
            let salience = super::sensory_processing::calculate_salience(&percept, &self.drives);

            // Store important percepts (> 0.5 salience) in long-term memory
            if salience > 0.5 {
                use super::sensory_processing::Percept;
                use crate::core::memory::SpatialMemoryType;

                match &percept {
                    Percept::ResourceDetected { resource_type, position, .. } => {
                        // Remember resource locations
                        let mem_type = match resource_type.as_str() {
                            "Food" => SpatialMemoryType::Food,
                            "Water" => SpatialMemoryType::Water,
                            _ => SpatialMemoryType::Resource,
                        };
                        self.memory.remember_location(mem_type, *position);
                    }
                    Percept::DangerDetected { position: Some(pos), .. } => {
                        // Remember danger locations
                        self.memory.remember_location(SpatialMemoryType::Danger, *pos);
                    }
                    Percept::AgentDetected { agent_id, .. } => {
                        // Update social relationship (neutral interaction for just seeing them)
                        let rel = self.relationships.get_or_create_relationship(*agent_id, current_tick);
                        rel.strengthen(0.01);
                    }
                    _ => {}
                }
            }

            self.recent_percepts.push((current_tick, percept));
        }

        // Trim old percepts (keep only last 20 ticks worth)
        self.recent_percepts.retain(|(tick, _)| current_tick.saturating_sub(*tick) <= 20);

        // Body condition caps overall health rather than setting it.
        //
        // Starvation, dehydration and exposure damage the same field, and
        // overwriting it from the body every tick threw all of that away: an
        // agent could go six thousand ticks without water and still read as
        // near perfect health, because the only harm that survived the tick
        // was a broken bone.
        let body_condition = self.body.overall_health() * 100.0;
        self.state.health = self.state.health.min(body_condition);

        // Update energy (basic metabolism)
        self.state.energy = (self.state.energy - 0.1).max(0.0);
    }

    /// Update agent with time progression (includes aging and survival mechanics)
    pub fn tick_with_time(&mut self, current_tick: u32) {
        // First do the regular tick
        self.tick();

        self.process_survival_tick(current_tick);
    }

    /// Run one tick of survival mechanics: aging, metabolism, food spoilage and fatigue.
    ///
    /// Split out of `tick_with_time` so that callers which drive agents through
    /// `tick_with_percepts` instead (notably `Population::tick`) run the same
    /// survival mechanics rather than aging alone.
    /// How near a need has to be to killing somebody before it starts to
    /// shout over everything else.
    ///
    /// Half a day. It has to be well inside the shortest of the clocks or a
    /// need that is entirely answered still reads as urgent: thirst kills at
    /// about 4,380 ticks from a full skin, so at a three-day horizon a
    /// perfectly watered agent scored 0.99 and was one sip from outranking a
    /// settlement's whole want of a harvest. At half a day a satisfied need
    /// scores about a seventh, a need a day out scores a half, and one twelve
    /// hours off starts taking the agent over.
    ///
    /// Every figure in that paragraph is still exactly right; only the unit
    /// was stale. It was written when a tick was a minute, so half a day was
    /// seven hundred and twenty of them. `ticks_before_this_kills_me` now
    /// answers in turns off a real body - thirst at thirty-six turns from a
    /// full skin rather than four thousand - and against 720 that read as
    /// twenty, so a fully watered agent was permanently in mortal danger and
    /// went to the water on nine turns in ten. Derived from the calendar now,
    /// so it cannot fall behind it again. See ISSUES #74.
    const A_LONG_WAY_OFF: f32 = crate::environment::seasons::TICKS_PER_DAY as f32 / 2.0;

    /// How much any one drive may press, before its band is applied.
    ///
    /// Just under the ratio between one band and the next, so a need in a
    /// lower band can approach a need in a higher one and never pass it.
    const AS_MUCH_AS_A_BAND_ALLOWS: f32 = 9.0;

    /// How hard this need is pressing on this agent, right now.
    ///
    /// Two things decide it. The tier says how much a need of this kind is
    /// allowed to interrupt - no amount of wanting a fine coat outweighs being
    /// thirsty. Within that, a need that kills presses in proportion to how
    /// soon it would: that is what makes an agent break off hunting to drink,
    /// having resolved nothing about its hunger, because the water runs out
    /// first.
    ///
    /// Nothing here is a written-down ladder. Thirst outranks hunger because
    /// dehydration takes health at 2,160 ticks and starvation at 4,320 times
    /// whatever the body has put by, and a child with a quarter of an adult's
    /// reserves reorders its own needs without anybody having decided that.
    pub fn how_hard_it_presses(&self, drive_type: crate::core::DriveType) -> f32 {
        let Some(drive) = self.drives.get(drive_type) else {
            return 0.0;
        };

        // A chain that has not opened yet is not pressing at all
        if !self.drives.is_unlocked(drive_type) {
            return 0.0;
        }

        // A band is a band.
        //
        // "Wide enough that no amount of wanting a fine coat outweighs being
        // thirsty" is what the hundred against ten is for, and an unbounded
        // `pressure()` was quietly defeating it: Preparedness on a settlement
        // that can never quite lay a week by goes unanswered for thousands of
        // turns, and the pressure of that carried its urgency past ten, at
        // which point a secondary need outranked a primary one that was
        // actively asking. Agents walked away from the water to go on
        // gathering and died of thirst with a full larder in front of them.
        let wanting = drive.urgency().min(Self::AS_MUCH_AS_A_BAND_ALLOWS);

        let deadly = self
            .state
            .ticks_before_this_kills_me(drive_type)
            .map(|left| Self::A_LONG_WAY_OFF / left.max(1.0))
            .unwrap_or(0.0);

        // The rank says what a need of this kind may interrupt - but only once
        // it is actually asking for something. A drive still under its own
        // threshold, on a body in no danger, is a preference rather than a
        // need, and gets no precedence for the band it belongs to.
        //
        // Without this a primary drive at any value at all outranked every
        // secondary one: an agent four tenths of the way to hungry, days from
        // any harm, beat a settlement's whole want of a harvest, and the only
        // thing anybody ever did was forage.
        let asking = drive.is_active() || deadly >= 1.0;

        if !asking {
            return wanting;
        }

        // Among the needs that kill, nearness of death decides and the drive's
        // own value does not. That is the whole of "an agent will not continue
        // hunting if it will die from dehydration, even if it resolves its
        // hunger drive": taking the larger of the two let a big appetite
        // outrank a nearer death, because how much somebody wants a thing and
        // how soon the want of it kills them are different questions.
        let pressing = if self.state.ticks_before_this_kills_me(drive_type).is_some() {
            1.0 + deadly * Self::SOONER_IS_WORSE
        } else {
            wanting
        };

        drive_type.rank().precedence() * pressing
    }

    /// How much worse a death that is twice as near counts as being.
    ///
    /// Enough that the ordering inside the primary band is settled by the
    /// clocks rather than by how much anybody happens to want the thing.
    const SOONER_IS_WORSE: f32 = 10.0;

    /// How many turns before this need starts asking.
    ///
    /// The only forward-looking question in the model, and every term in it is
    /// a figure the drive already keeps: how far it is below its threshold,
    /// how fast it builds, and how much the weight of having been ignored is
    /// making it build faster. Nothing here is invented and nothing is shared
    /// between agents - two people standing in the same field get two
    /// different answers, because they are carrying different values, have
    /// been denied for different lengths of time, and their bodies burn at
    /// different rates.
    ///
    /// `Some(0)` for a need already asking. `None` for one that cannot ask at
    /// all yet, because nothing before it in its chain has been answered.
    pub fn how_long_before_this_asks(&self, drive_type: crate::core::DriveType) -> Option<u32> {
        let drive = self.drives.get(drive_type)?;

        if !self.drives.is_unlocked(drive_type) {
            return None;
        }

        if drive.is_active() {
            return Some(0);
        }

        let climbing = drive_type.base_accumulation_rate() * drive.pressure();
        if climbing <= 0.0 {
            return None;
        }

        Some(((drive.threshold - drive.value) / climbing).ceil().max(0.0) as u32)
    }

    /// The need that will take the turn off this one before it is finished,
    /// and how long there is before it does.
    ///
    /// "The planner should attempt to anticipate drive demand increase so that
    /// actions can be efficiently executed, reducing the odds of tasks being
    /// dropped mid-completion." This is the question that asks: I am about to
    /// spend `turns` on something - is there a need that outranks it and is
    /// going to start asking before I am done?
    ///
    /// Rank decides what may interrupt what, which is the model's own answer
    /// and not a second one: a primary need takes the turn from a secondary
    /// one, and no amount of wanting a coat takes it from being thirsty. A
    /// need in the same band as the one being served does not count, because
    /// two needs of a kind trading places is the ordinary business of a day
    /// and turning round for it is how an agent gets nothing done at all -
    /// see `what_it_takes_to_turn_me_round`.
    ///
    /// **What counts as not waiting is the body's clock, not the threshold.**
    /// Written against `how_long_before_this_asks` it fired on nearly every
    /// job anybody ever started: hunger is a few turns off its threshold most
    /// of the time and outranks everything that is not itself primary, so a
    /// settlement stopped provisioning, stopped building and stopped making
    /// tools, and did nothing but eat. Measured over 160 worlds that cost
    /// between four and fifteen per cent of every block. A need being about to
    /// *ask* is ordinary; a need that will have killed you before the job is
    /// done is the one worth turning round for, and
    /// `ticks_before_this_kills_me` is what the model already reckons the
    /// primaries by.
    pub fn what_will_not_wait_for(
        &self,
        this: crate::core::DriveType,
        turns: u32,
    ) -> Option<crate::core::DriveType> {
        let mine = this.rank().precedence();

        crate::core::DriveType::all()
            .into_iter()
            .filter(|other| *other != this)
            .filter(|other| other.rank().precedence() > mine)
            .filter(|other| {
                self.state
                    .ticks_before_this_kills_me(*other)
                    .is_some_and(|left| left < turns as f32)
            })
            // Of the needs the job would outlast, the one that starts asking
            // first, which is what an agent would deal with first anyway.
            .filter_map(|other| Some((other, self.how_long_before_this_asks(other)?)))
            .min_by_key(|(_, soon)| *soon)
            .map(|(other, _)| other)
    }

    /// What this agent most needs to do something about.
    ///
    /// `DriveState::most_urgent` compares drives against each other as though
    /// they were all the same kind of thing. They are not: a need that kills
    /// in a day and a wish for a better axe are not on one scale, and reading
    /// them off one was why an agent would go on hunting while it died of
    /// thirst.
    pub fn what_presses_hardest(&self) -> Option<crate::core::DriveType> {
        crate::core::DriveType::all()
            .into_iter()
            .map(|drive_type| (drive_type, self.how_hard_it_presses(drive_type)))
            .filter(|(_, pressing)| *pressing > 0.0)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(drive_type, _)| drive_type)
    }

    /// How often a hand is tested against what it has not been doing.
    ///
    /// A season. Rust is reckoned in years, so checking more often buys
    /// nothing and costs a walk over every skill of every agent.
    const HOW_OFTEN_A_HAND_IS_TESTED: u32 = 288;

    pub fn process_survival_tick(&mut self, current_tick: u32) {
        // Calculate pregnancy energy multiplier (if pregnant)
        let energy_multiplier = self.pregnancy.as_ref()
            .map(|p| p.energy_multiplier(current_tick))
            .unwrap_or(1.0);

        // Then handle aging and survival mechanics with pregnancy modifier
        self.state.age_tick_with_modifier(current_tick, energy_multiplier);

        // Process nutrition metabolism
        self.tick_nutrition(current_tick);

        // Process food spoilage in inventory
        self.tick_food_spoilage(current_tick);

        // And whatever this one has come down with, which costs a little
        // every tick and then is over
        self.tick_ailment(current_tick);
        self.tick_the_wound(current_tick);

        // And the salt, if this one has been drinking out of the sea
        self.tick_salt();

        // And let go of trades that have not been practised in a long time.
        // Once a season is often enough for something measured in years, and a
        // settlement of two hundred is not worth walking every skill of every
        // agent every tick for.
        if current_tick % Self::HOW_OFTEN_A_HAND_IS_TESTED == 0 {
            self.skills.let_unused_skills_rust(current_tick);
        }

        // Recover condition when nothing is wrong. `regenerate_health` had no
        // callers at all, so agents only ever lost health over a lifetime.
        let suffering = self.state.is_starving()
            || self.state.is_dehydrated()
            || !self.exposure_status.active_exposures.is_empty();

        if !suffering {
            let resting = self.fatigue.is_sleeping;
            let body_condition = self.body.overall_health() * 100.0;

            self.regenerate_health(resting);
            self.state.health = self.state.health.min(body_condition);
        }

        // Process fatigue (awake state)
        if !self.fatigue.is_sleeping {
            // Activity level based on current drive urgency and recent actions
            let activity_level = self.calculate_activity_level();
            self.fatigue.tick_awake(activity_level, current_tick);

            // Update rest drive based on fatigue
            if let Some(rest_drive) = self.drives.get_mut(DriveType::Rest) {
                // Rest drive increases with fatigue level
                let rest_increase = self.fatigue.level * 0.02;
                rest_drive.increase(rest_increase);
            }

            // Apply fatigue-induced happiness penalty
            let happiness_penalty = self.fatigue.happiness_modifier();
            if happiness_penalty < 0.0 {
                self.emotions.happiness = (self.emotions.happiness + happiness_penalty).max(0.0);
            }
        }
    }

    /// Calculate activity level based on current state (0.0 = resting, 1.0 = strenuous)
    fn calculate_activity_level(&self) -> f32 {
        // Base on energy expenditure indicators
        let mut activity: f32 = 0.3; // Base activity

        // Higher if carrying heavy load
        if self.inventory.weight_percentage() > 0.5 {
            activity += 0.2;
        }

        // Higher if in dangerous situation
        if self.emotions.fear > 0.5 {
            activity += 0.2; // Stress/alertness
        }

        // Higher if low energy (struggling)
        if self.state.energy < 30.0 {
            activity += 0.1;
        }

        activity.min(1.0)
    }

    /// Tick nutrition metabolism and apply deficiency effects
    pub fn tick_nutrition(&mut self, _current_tick: u32) {
        // Calculate activity level from energy expenditure
        let activity_level = if self.state.energy < 30.0 {
            0.2 // Low energy = low activity
        } else if self.state.energy > 70.0 {
            0.8 // High energy = high activity
        } else {
            0.5 // Moderate
        };

        // Tick metabolism (depletes nutrients)
        self.nutrition.tick_metabolism(activity_level);

        // Apply deficiency health penalties
        let penalty = self.nutrition.deficiency_health_penalty();
        if penalty > 0.0 {
            self.state.lose_health(penalty, "a poor diet");
        }

        // Couple state energy to nutritional reserves.
        //
        // Reserves are the long-term store; `state.energy` is short-term felt
        // energy that eating and sleeping move directly. Drifting gradually
        // toward reserves keeps the two systems consistent without erasing the
        // effect of a meal or a rest on the very next tick, which is what a
        // straight average between the two used to do.
        const ENERGY_SYNC_RATE: f32 = 0.02;
        let reserve_delta = self.nutrition.energy_reserves - self.state.energy;
        self.state.energy =
            (self.state.energy + reserve_delta * ENERGY_SYNC_RATE).clamp(0.0, 100.0);
    }

    /// Update food freshness in inventory and remove spoiled items
    pub fn tick_food_spoilage(&mut self, current_tick: u32) {
        // Update freshness for all food items
        for item in self.inventory.items.values_mut() {
            item.update_food_freshness(current_tick);
        }

        // Remove completely spoiled food (freshness <= 0)
        let spoiled_items: Vec<String> = self.inventory.get_all_items().iter()
            .filter(|(_, item)| {
                item.food_data.as_ref()
                    .map(|f| f.freshness <= 0.0)
                    .unwrap_or(false)
            })
            .map(|(id, _)| id.clone())
            .collect();

        for item_id in spoiled_items {
            // What has gone off is still matter. Deleting it outright made a
            // pack the one place in the world where things could rot to
            // nothing, which is half of why the ground never got anything
            // back.
            if let Some(item) = self.inventory.items.remove(&item_id) {
                self.state.waste_carried += item.quantity as f32
                    * crate::world::Soil::waste_from_spoilage(&item_id);

                // And it is worth minding. Food going off in your own pack is
                // a threat to the one drive that kills you soonest, and until
                // now it was the one kind of loss that cost an agent nothing
                // at all to watch: the meal simply stopped existing and
                // nobody felt anything about it.
                self.watched_food_go_off(&item_id, item.quantity);
            }
        }
    }


    /// Update body temperature, accounting for whether the agent is under cover
    ///
    /// Shelter has to reach the body to be worth seeking: without this an
    /// agent inside a building cools exactly as fast as one standing in the
    /// open, so sheltering never resolves the exposure that sent it indoors.
    pub fn update_temperature_with_shelter(&mut self, climate: &super::Climate, has_shelter: bool) {
        let cold_insulation = self.body.total_cold_insulation();
        let heat_resistance = self.body.total_heat_resistance();

        let effective_temp = if has_shelter {
            climate.sheltered_effective_temperature()
        } else {
            climate.effective_temperature()
        };

        self.body_temperature.update(effective_temp, cold_insulation, heat_resistance);
    }

    /// Update exposure status based on environmental conditions
    ///
    /// # Arguments
    /// * `weather` - Current weather conditions
    /// * `environmental_temp` - Ambient temperature
    /// * `has_shelter` - Whether the agent is in shelter
    /// * `has_water_access` - Whether the agent has access to water
    /// * `time_of_day` - Current time of day (0-24)
    ///
    /// Returns the amount of exposure damage taken this tick
    pub fn update_exposure(
        &mut self,
        weather: &crate::environment::Weather,
        environmental_temp: super::temperature::Temperature,
        has_shelter: bool,
        has_water_access: bool,
        time_of_day: f32,
        now: u32,
    ) -> f32 {
        let damage = self.exposure_status.update(
            &self.body_temperature,
            environmental_temp,
            weather,
            has_shelter,
            has_water_access,
            time_of_day,
        );

        // Apply exposure damage to health
        if damage > 0.0 {
            self.state.lose_health(damage * 10.0, "the weather");
            // And a soaking in the cold is a thing people came down with,
            // rather than only a thing that wore them down.
            self.a_soaking_may_tell(damage, now);
        }

        damage
    }

    /// Check if agent needs shelter based on current exposure
    pub fn needs_shelter(&self) -> bool {
        // Seek shelter if exposure is getting dangerous
        self.exposure_status.is_critical() ||
        !self.exposure_status.active_exposures.is_empty()
    }


    /// Respond emotionally to a threat
    ///
    /// # Arguments
    /// * `threat_strength` - Strength of the threat (e.g., enemy agent's combat power)
    /// * `source` - Source of the threat
    ///
    /// Returns the emotional response triggered
    /// What this agent is worth in a fight, as it reckons itself.
    ///
    /// Health, what the body can still do, armour, a weapon in the hand, a
    /// practised arm - and what happened the last few times it stood its
    /// ground. That last is the part the specification asks for: an agent that
    /// has fought and won finds fighting a more attractive option, and one
    /// that has fought and lost finds running one.
    ///
    /// It is what the agent *believes*, not what is true. Two agents of
    /// identical build appraise the same wolf differently if one of them has
    /// beaten a wolf before, and that is the point.
    /// Whether this agent could raise a hand to anything at all.
    ///
    /// Not whether it would win - that is `ThreatAssessment` - but whether
    /// fighting is available to it as a thing to do. Two ways it is not: no
    /// arm that works, and a body with so little left in it that the first
    /// blow returned would finish the job.
    pub fn could_i_fight_at_all(&self, coming: f32) -> bool {
        use super::body::BodyPartType;

        // A child does not fight a wolf. This is not a judgement about how
        // brave it is - it is the commonest way in the world for fighting to
        // not be an option, and leaving it out made freezing unreachable:
        // measured over eight worlds, not one agent ever froze, because
        // anybody with an arm and more health than one bite would take had
        // fighting available to them.
        if !self.state.life_stage.can_fight() {
            return false;
        }

        let an_arm = [BodyPartType::LeftArm, BodyPartType::RightArm]
            .iter()
            .any(|part| {
                self.body
                    .get_part(*part)
                    .is_some_and(|arm| arm.is_functional())
            });

        if !an_arm {
            return false;
        }

        // A man with four points of health left does not trade blows with a
        // wolf. What one would cost him is what he already knows from having
        // stood his ground before - see `what_a_blow_costs_me`.
        self.state.health > self.what_a_blow_costs_me(coming)
    }

    /// Whether this agent could run anywhere.
    ///
    /// Running is not walking: it costs `WHAT_RUNNING_COSTS` and a body with
    /// nothing left in it cannot pay. Legs that do not work stop it too.
    /// Whether there is anywhere to run *to* is a question about the ground
    /// and belongs to the simulation - see `Simulation::is_there_anywhere_to_run`.
    pub fn could_i_run_at_all(&self, what_it_takes: f32) -> bool {
        use super::body::BodyPartType;

        let a_leg = [BodyPartType::LeftLeg, BodyPartType::RightLeg]
            .iter()
            .any(|part| {
                self.body
                    .get_part(*part)
                    .is_some_and(|leg| leg.is_functional())
            });

        a_leg && self.state.energy > what_it_takes
    }

    /// What this agent has to lose - the drive demand still standing between
    /// it and being satisfied, which is what a thing that would kill it
    /// actually takes away.
    ///
    /// The specification says a threat is a threat *to the agent's ability to
    /// satisfy future drive demand*, and that is a different quantity from
    /// how large the animal's teeth are. A wolf does not take an agent's
    /// health; it takes every meal, every drink and every night's sleep that
    /// agent had left. So what it costs is measured on the drives.
    ///
    /// Every drive that is asking contributes what it is asking for, ranked:
    /// the ones that kill you count for most, the ones that decide whether
    /// there is anybody here in ten years next, and the ones that decide what
    /// sort of place it is least. A person with nothing pressing still has
    /// something to lose - `WHAT_BEING_ALIVE_IS_WORTH` - because being alive
    /// is itself the thing that makes tomorrow's dinner possible.
    pub fn what_i_stand_to_lose(&self) -> f32 {
        use crate::core::drives::DriveRank;

        let asked: f32 = self
            .drives
            .drives
            .iter()
            .map(|drive| {
                let weight = match drive.drive_type.rank() {
                    DriveRank::Primary => 1.0,
                    DriveRank::Secondary => 0.5,
                    DriveRank::Tertiary => 0.2,
                };
                drive.urgency().clamp(0.0, 1.0) * weight
            })
            .sum();

        // Four primaries at full cry is the most anybody carries, so that is
        // what the scale is divided by
        let carried = (asked / Self::WHAT_A_LIFE_FULL_OF_WANT_COMES_TO).clamp(0.0, 1.0);

        Self::WHAT_BEING_ALIVE_IS_WORTH
            + carried * (1.0 - Self::WHAT_BEING_ALIVE_IS_WORTH)
    }

    /// What a person with nothing pressing still stands to lose, against
    /// somebody who is starving, parched and worn out at once.
    ///
    /// Not far off the whole of it: a comfortable man does not shrug at a
    /// wolf. What the rest of the scale buys is that a desperate one minds
    /// rather more.
    const WHAT_BEING_ALIVE_IS_WORTH: f32 = 0.75;

    /// The most drive demand anybody carries at once, in the units
    /// `what_i_stand_to_lose` sums.
    const WHAT_A_LIFE_FULL_OF_WANT_COMES_TO: f32 = 4.0;

    pub fn own_strength(&self) -> f32 {
        let health_factor = self.state.health / 100.0;
        let body_factor = self.body.movement_speed_multiplier();

        let armor_bonus = self.equipment.total_armor() / 100.0;
        let weapon_bonus = if self.equipment.get_weapon().is_some() { 0.3 } else { 0.0 };

        let combat_skill = self
            .skills
            .get_skill_if_exists(crate::agents::SkillType::MeleeCombat)
            .map(|skill| skill.level)
            .unwrap_or(-10);
        let skill_bonus = (combat_skill as f32 + 10.0) / 20.0;

        let bravery_modifier = if self.traits.has(crate::core::Trait::Brave) {
            1.3
        } else if self.traits.has(crate::core::Trait::Anxious) {
            0.7
        } else {
            1.0
        };

        // And the years. `LifeStage::can_fight` already keeps the very young
        // out of a fight altogether; this is what separates a thirteen-year-old
        // from his father, and his father from his grandfather.
        let base_strength = health_factor * body_factor * self.state.what_i_can_do_for_my_age();
        let equipment_bonus = armor_bonus * 0.3 + weapon_bonus;
        let built = (base_strength + equipment_bonus + skill_bonus * 0.2) * bravery_modifier;

        built * self.what_fighting_has_taught_me()
    }

    /// How much what happened last time is worth, at the widest and narrowest.
    ///
    /// Somebody who has never fought reckons themselves at face value; a
    /// proven fighter half again; somebody beaten every time they tried, half.
    /// Wide enough to turn a fight an agent would have run from into one it
    /// stands for, and narrow enough that a coward with an axe is still worth
    /// more than a hero without one.
    const BEATEN_EVERY_TIME: f32 = 0.6;
    const NEVER_YET_LOST: f32 = 1.5;

    /// What this agent's own record tells it about standing its ground.
    ///
    /// `Lessons` already keeps a running belief per undertaking, moved by
    /// every outcome and weighted so failures count for more than successes.
    /// Fighting is one more of those.
    pub fn what_fighting_has_taught_me(&self) -> f32 {
        use super::practices::{Lessons, Undertaking};

        // Nobody learns anything from one scrap
        if self.lessons.attempts(Undertaking::Fighting) == 0 {
            return 1.0;
        }

        let believes = self.lessons.belief(Undertaking::Fighting);
        let from_nothing = (believes - Lessons::UNTRIED) / Lessons::UNTRIED;

        (1.0 + from_nothing * (Self::NEVER_YET_LOST - 1.0))
            .clamp(Self::BEATEN_EVERY_TIME, Self::NEVER_YET_LOST)
    }

    /// Feel about something that is simply *there*, rather than something that
    /// has just happened.
    ///
    /// The same appraisal as [`Self::respond_to_threat`] - can I fight this,
    /// and so is it anger or is it fear - but it *sets* the feeling rather than
    /// adding to it. A wolf standing ten paces off is one wolf however many
    /// ticks it stands there; adding a fresh helping of anger every tick it
    /// remained in sight ran every agent in the world up to the ceiling, and
    /// left three in five of them ready to attack something at any moment.
    pub fn appraise_what_is_there(
        &mut self,
        threat_strength: f32,
        source: super::EmotionSource,
    ) -> super::EmotionType {
        use super::ThreatAssessment;

        // What the thing would end, which is what makes it a threat rather
        // than merely a large animal - see `what_i_stand_to_lose`
        let assessment = ThreatAssessment::assess_against_what_is_at_stake(
            self.own_strength(),
            threat_strength,
            self.what_i_stand_to_lose(),
            source.clone(),
        );

        let emotion_type = assessment.emotion_type();
        let emotion_amount = assessment.emotion_amount();

        match emotion_type {
            super::EmotionType::Anger => {
                self.emotions.set_anger(source.clone(), emotion_amount);
                self.emotions.set_fear(source, 0.0);
            }
            super::EmotionType::Fear => {
                self.emotions.set_fear(source.clone(), emotion_amount);
                self.emotions.set_anger(source, 0.0);
            }
            _ => {}
        }

        emotion_type
    }

    pub fn respond_to_threat(&mut self, threat_strength: f32, source: super::EmotionSource) -> super::EmotionType {
        use super::ThreatAssessment;

        let assessment =
            ThreatAssessment::assess(self.own_strength(), threat_strength, source.clone());

        let emotion_type = assessment.emotion_type();
        let emotion_amount = assessment.emotion_amount();

        match emotion_type {
            super::EmotionType::Anger => {
                self.emotions.add_anger_with_traits(source, emotion_amount, &self.traits);
            }
            super::EmotionType::Fear => {
                self.emotions.add_fear_with_traits(source, emotion_amount, &self.traits);
            }
            _ => {}
        }

        emotion_type
    }


    /// Respond emotionally to death of a loved one
    ///
    /// # Arguments
    /// * `deceased_id` - UUID of the deceased
    /// * `source` - Source of the death (what killed them)
    pub fn respond_to_loved_one_death(&mut self, deceased_id: &Uuid, source: super::EmotionSource) {
        // Maximum sadness for death of loved one
        if let Some(relationship) = self.relationships.get_relationship(deceased_id) {
            if relationship.is_loved_one() {
                let sadness_amount = relationship.bond_strength * 0.9;
                self.emotions.add_sadness_with_traits(EmotionSource::Agent(*deceased_id), sadness_amount, &self.traits);

                // Fear of the source that killed them
                self.emotions.add_fear_with_traits(source, 0.4, &self.traits);
            }
        }
    }



    /// Get agent's dominant emotion
    pub fn dominant_emotion(&self) -> Option<super::EmotionType> {
        self.emotions.dominant_emotion()
    }



    /// Check if agent believes specific information
    pub fn believes(&self, info_id: &Uuid) -> bool {
        self.knowledge.believes(info_id)
    }


    /// Observe a resource at a position
    /// Note: Resource knowledge is tracked separately from gossip knowledge
    pub fn observe_resource(
        &mut self,
        _position: crate::world::Position,
        _resource_type: crate::world::ResourceType,
        _amount: u32,
    ) {
        // Resource observation is handled by the simulation tick loop
        // This method exists for API compatibility
    }




    /// Regenerate preferences based on current traits
    ///
    /// Call this after modifying agent traits to update their job preferences
    pub fn update_preferences_from_traits(&mut self) {
        self.preferences = Preferences::from_traits(&self.traits);
        // Also update storage preferences
        let trait_vec: Vec<_> = self.traits.get_traits().iter().copied().collect();
        self.storage_preferences = super::storage_management::StoragePreferences::from_traits(&trait_vec);
    }

    /// Calculate happiness for doing a specific job category
    pub fn get_job_happiness(&self, job: super::job_happiness::JobCategory) -> f32 {
        super::job_happiness::calculate_job_happiness(&self.traits, job)
    }

    /// Get the job that would make this agent happiest
    pub fn get_preferred_job(&self) -> (super::job_happiness::JobCategory, f32) {
        super::job_happiness::find_preferred_job(&self.traits)
    }

    /// Get all jobs ranked by happiness preference
    pub fn get_job_rankings(&self) -> Vec<(super::job_happiness::JobCategory, f32)> {
        super::job_happiness::rank_jobs_by_happiness(&self.traits)
    }

    /// Check if survival needs should override happiness-based job selection
    pub fn should_prioritize_survival(&self) -> bool {
        let hunger = self.drives.get(crate::core::DriveType::Hunger)
            .map(|d| d.value)
            .unwrap_or(0.0);
        let thirst = self.drives.get(crate::core::DriveType::Thirst)
            .map(|d| d.value)
            .unwrap_or(0.0);
        let health_percent = self.state.health / 100.0;

        super::job_happiness::should_override_happiness(hunger, thirst, health_percent)
    }

    /// Calculate effective priority for an action considering both drive urgency and happiness
    ///
    /// This is used when selecting between multiple possible work actions.
    /// Higher values = more preferred action.
    pub fn calculate_action_priority(
        &self,
        drive_urgency: f32,
        job: super::job_happiness::JobCategory,
    ) -> f32 {
        // If survival is threatened, ignore happiness
        if self.should_prioritize_survival() {
            return drive_urgency;
        }

        let job_happiness = self.get_job_happiness(job);
        // Use 0.3 weight - happiness is noticeable but doesn't dominate
        super::job_happiness::calculate_effective_priority(drive_urgency, job_happiness, 0.3)
    }

    /// Map a drive type to a job category for happiness calculation
    fn drive_to_job_category(drive_type: crate::core::DriveType) -> Option<super::job_happiness::JobCategory> {
        use crate::core::DriveType;
        use super::job_happiness::JobCategory;

        match drive_type {
            DriveType::Industry => Some(JobCategory::Mining),
            DriveType::Construction => Some(JobCategory::Building),
            DriveType::Utility => Some(JobCategory::Crafting),
            DriveType::Sustenance => Some(JobCategory::Gathering),
            DriveType::Social => Some(JobCategory::Social),
            DriveType::Curiosity => Some(JobCategory::Exploring),
            DriveType::Preparedness => Some(JobCategory::Labor),
            // Survival drives don't map to happiness-influenced jobs
            DriveType::Hunger | DriveType::Thirst | DriveType::Rest |
            DriveType::Safety | DriveType::Aggression | DriveType::Shelter |
            DriveType::Reproduction | DriveType::Protection | DriveType::Luxury => None,
        }
    }

    /// Select the best drive considering both urgency and happiness
    ///
    /// For survival-critical drives (hunger, thirst, rest, safety), returns the most urgent.
    /// For work-related drives, considers happiness when drives are similarly urgent.
    pub fn select_drive_with_happiness(&self) -> Option<crate::core::DriveType> {
        use crate::core::DriveType;

        // First check if survival is threatened - if so, use pure urgency
        if self.should_prioritize_survival() {
            return self.drives.most_urgent().map(|d| d.drive_type);
        }

        // Get all active drives
        let mut drive_scores: Vec<(DriveType, f32)> = Vec::new();

        for drive_type in DriveType::all() {
            if let Some(drive) = self.drives.get(drive_type) {
                if drive.is_active() {
                    // Through `bare_urgency` rather than by hand, so that
                    // what the agent's personality makes of this drive counts
                    // here as it does everywhere else
                    let base_urgency = drive.bare_urgency();

                    // For work-related drives, factor in happiness
                    let effective_priority = if let Some(job) = Self::drive_to_job_category(drive_type) {
                        self.calculate_action_priority(base_urgency, job)
                    } else {
                        // Survival drives use pure urgency
                        base_urgency
                    };

                    drive_scores.push((drive_type, effective_priority));
                }
            }
        }

        // Sort by effective priority (descending)
        drive_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        drive_scores.first().map(|(drive_type, _)| *drive_type)
    }

    /// Observe another agent performing an action
    ///
    /// # Arguments
    /// * `performer_id` - UUID of agent performing the action
    /// * `performer_position` - Position of the performer
    /// * `action_type` - Type of action being performed
    /// * `success` - Whether the action succeeded
    /// * `details` - Specific details about the action
    /// * `timestamp` - Current simulation time
    pub fn observe_action(
        &mut self,
        performer_id: &Uuid,
        performer_position: (i32, i32, i32),
        action_type: super::ActionType,
        success: bool,
        details: String,
        timestamp: u64,
    ) {
        // Check if agent can see the performer
        if !self.senses.vision.visible_agents.contains(performer_id) {
            return; // Can't learn if you can't see them
        }

        // Calculate distance to performer
        let dx = (performer_position.0 - self.state.position.0) as f32;
        let dy = (performer_position.1 - self.state.position.1) as f32;
        let dz = (performer_position.2 - self.state.position.2) as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Create observation record
        let observation = super::ObservedAction::new(
            *performer_id,
            action_type,
            success,
            details,
            timestamp,
            distance,
        );

        // Record the observation
        self.observational_learning.observe_action(observation);
    }

    /// Check if agent should adopt behaviors from observations
    ///
    /// Returns list of (performer, action_type, confidence) for behaviors ready to adopt
    pub fn check_learning_opportunities(&self) -> Vec<(Uuid, super::ActionType, f32)> {
        let mut opportunities = Vec::new();

        for teacher_id in self.observational_learning.get_all_teachers() {
            // Get relationship and trust
            let relationship_strength = self.relationships
                .get_relationship(&teacher_id)
                .map(|r| r.bond_strength)
                .unwrap_or(0.0);

            let trust = self.knowledge.get_trust(&teacher_id);

            // Check each action type
            for action_type in [
                super::ActionType::Mining,
                super::ActionType::Crafting,
                super::ActionType::Building,
                super::ActionType::Combat,
                super::ActionType::Cooking,
                super::ActionType::ToolUse,
                super::ActionType::Social,
                super::ActionType::Navigation,
                super::ActionType::ProblemSolving,
            ] {
                let (should_adopt, confidence) = self.observational_learning.should_adopt_behavior(
                    &teacher_id,
                    action_type,
                    relationship_strength,
                    trust,
                );

                if should_adopt {
                    opportunities.push((teacher_id, action_type, confidence));
                }
            }
        }

        opportunities
    }

    /// Adopt a learned behavior
    ///
    /// # Arguments
    /// * `teacher_id` - Who to learn from
    /// * `action_type` - What action to adopt
    ///
    /// Returns true if successfully adopted
    pub fn adopt_learned_behavior(
        &mut self,
        teacher_id: &Uuid,
        action_type: super::ActionType,
    ) -> bool {
        // Get relationship and trust
        let relationship_strength = self.relationships
            .get_relationship(teacher_id)
            .map(|r| r.bond_strength)
            .unwrap_or(0.0);

        let trust = self.knowledge.get_trust(teacher_id);

        // Check if ready to adopt
        let (should_adopt, _) = self.observational_learning.should_adopt_behavior(
            teacher_id,
            action_type,
            relationship_strength,
            trust,
        );

        if should_adopt {
            self.observational_learning.adopt_behavior(teacher_id, action_type);
            true
        } else {
            false
        }
    }


    /// Get learning progress from a specific teacher
    pub fn get_learning_from(
        &self,
        teacher_id: &Uuid,
        action_type: super::ActionType,
    ) -> Option<&super::LearningProgress> {
        self.observational_learning.get_progress(teacher_id, action_type)
    }

    /// Get all adopted behaviors
    pub fn get_adopted_behaviors(&self) -> Vec<(Uuid, super::ActionType, f32)> {
        self.observational_learning.get_adopted_behaviors()
    }

    /// Check if this agent is learning from parents
    ///
    /// Returns list of (parent_id, action_types_being_learned)
    pub fn learning_from_parents(&self) -> Vec<(Uuid, Vec<super::ActionType>)> {
        let mut parent_learning = Vec::new();

        // Get all parents
        let parents: Vec<Uuid> = self.relationships
            .get_all()
            .iter()
            .filter(|(_, rel)| rel.relationship_type == super::RelationshipType::Parent)
            .map(|(id, _)| *id)
            .collect();

        for parent_id in parents {
            let mut learning_actions = Vec::new();

            // Check all action types
            for action_type in [
                super::ActionType::Mining,
                super::ActionType::Crafting,
                super::ActionType::Building,
                super::ActionType::Combat,
                super::ActionType::Cooking,
                super::ActionType::ToolUse,
                super::ActionType::Social,
                super::ActionType::Navigation,
                super::ActionType::ProblemSolving,
            ] {
                if let Some(progress) = self.observational_learning.get_progress(&parent_id, action_type) {
                    if progress.observation_count > 0 {
                        learning_actions.push(action_type);
                    }
                }
            }

            if !learning_actions.is_empty() {
                parent_learning.push((parent_id, learning_actions));
            }
        }

        parent_learning
    }

    /// Initialize default behavior trees for each drive type
    fn initialize_behavior_trees(&mut self) {
        for drive_type in DriveType::all() {
            let tree = Self::create_default_tree_for_drive(drive_type);
            self.behavior_trees.push(tree);
        }
    }

    /// Set age-based learning rate (child = 1.5, adult = 1.0, elder = 0.7)
    pub fn set_learning_rate(&mut self, rate: f32) {
        self.observational_learning.set_learning_rate(rate);
    }

    /// Get current learning rate (base rate modified by fatigue)
    pub fn learning_rate(&self) -> f32 {
        self.observational_learning.learning_rate() * self.fatigue.learning_modifier()
    }




    /// Equip a transport (activate it)
    /// Updates inventory capacity automatically
    pub fn equip_transport(&mut self, transport_id: &Uuid) -> bool {
        if self.transport.activate(transport_id) {
            // Update inventory capacity
            self.update_inventory_capacity_from_transport();
            true
        } else {
            false
        }
    }

    /// Unequip a transport (deactivate it)
    /// Updates inventory capacity automatically
    pub fn unequip_transport(&mut self, transport_id: &Uuid) {
        self.transport.deactivate(transport_id);
        self.update_inventory_capacity_from_transport();
    }

    /// Add a new transport to agent's possession
    pub fn add_transport(&mut self, transport: super::Transport) {
        self.transport.add_transport(transport);
    }

    /// Take up whatever this one has to carry things in, or put it down.
    ///
    /// `TransportSystem` has been able to model a basket, a travois and a cart
    /// since it was written - capacity, speed, durability, twenty-odd kinds of
    /// vehicle and pack animal - and **nothing has ever put a transport into
    /// it**, so the whole of it was tables with no caller.
    /// `total_additional_capacity` is already added into `max_weight` and
    /// `speed_modifier` is already multiplied into `movement_speed_at_tick`;
    /// the only missing link was somebody actually owning one.
    ///
    /// So: what is in the pack is what is on the back. Called each turn,
    /// because a thing to carry with can arrive by making it, by trade or by
    /// inheriting it, and can leave by wearing out.
    ///
    /// This is the largest waste in the model at the far end of it. Measured,
    /// nearly nine thousand items of gathered food went back on the bush in one
    /// run because packs were full.
    /// The things a person carries other things in, best first.
    ///
    /// Nobody drags a travois while pushing a cart. Read by
    /// `take_up_the_cart`, which puts the best of them on the back, and by
    /// `how_many_of_this_i_should_keep`, which will not have somebody set down
    /// the thing everything else is in.
    pub const WHAT_CARRIES: [(&'static str, super::transport::TransportType); 4] = [
        ("handcart", super::transport::TransportType::Handcart),
        ("travois", super::transport::TransportType::Travois),
        // A leather bag holds a good deal more than a flax basket, which is
        // what being a leatherworker is worth - see `making::SEW_A_BAG`. It
        // reached carrying capacity through `effective_max_weight` and not
        // through here, so it was a second way of holding things that the
        // transport system knew nothing about. See ISSUES #116.
        ("leatherbag", super::transport::TransportType::LargeBackpack),
        ("basket", super::transport::TransportType::Backpack),
    ];

    pub fn take_up_the_cart(&mut self) {
        use super::transport::Transport;

        let best = Self::WHAT_CARRIES
            .iter()
            .find(|(called, _)| self.how_many_i_have(called) > 0)
            .map(|(_, kind)| *kind);

        let already: Vec<_> = self
            .transport
            .get_active()
            .iter()
            .map(|t| (t.id, t.transport_type))
            .collect();

        if already.len() == 1 && Some(already[0].1) == best {
            return;
        }

        for (id, _) in already {
            self.unequip_transport(&id);
        }
        if let Some(kind) = best {
            let carrier = Transport::new(kind);
            let id = carrier.id;
            self.add_transport(carrier);
            self.equip_transport(&id);
        } else {
            // Nothing to carry with: the capacity still has to be recomputed,
            // or an agent that loses its basket goes on carrying as though it
            // had one.
            self.update_inventory_capacity_from_transport();
        }
    }

    /// What a pair of hands and a strong back hold with nothing to put things
    /// in.
    ///
    /// This wants to be much smaller. "An agent can eat from a berry bush but
    /// cannot carry additional berries unless they are carrying a pack or
    /// container" asks for a figure around a dozen, so that a basket is the
    /// difference between an armful and a load - and at twelve it is, and
    /// forty tests across barter, larder, sprouting, theft, working and
    /// portioning fall over, because every fixture in the suite was built when
    /// a pair of bare hands held a hundredweight.
    ///
    /// That sweep is its own piece of work and its own commit; doing it inside
    /// this one would make a change touching forty unrelated tests
    /// unattributable. Filed as #216. What is here now is the *shape* -
    /// carrying is hands plus containers, and containers are things you make -
    /// on the old number.
    ///
    /// **And the old number turns out to be right, which is not luck but is
    /// not reasoning either - it was measured.** Swept against survival over
    /// three blocks of thirty-two seeded worlds, person-days total:
    ///
    /// | what two hands hold | person-days |
    /// |---|---|
    /// | 6 | 97,217 |
    /// | **12** | **95,371** |
    /// | 120 | 75,081 |
    ///
    /// Flat from six to twelve - the difference is inside the block-to-block
    /// noise of ten per cent - and **twenty-one per cent worse at ten times**.
    /// A bigger pack is not a kindness. What it buys is turns: at 120 the
    /// share of the settlement's turns spent on `Work` rises by twenty-seven
    /// per cent and the share spent on `Eat` falls, because a person with
    /// materials in hand has something to make and making competes with
    /// eating.
    ///
    /// So carrying is not what limits this settlement, at this figure or any
    /// figure near it. See ISSUES #119, which closes #236 on that evidence.
    pub const WHAT_TWO_HANDS_HOLD: f32 = 12.0;

    /// Update inventory max_weight from what this one has to carry things in.
    ///
    /// Two things were wrong here. The base was a hundred - so a pair of hands
    /// carried more than a handcart adds, and no container was worth having.
    /// And it was scaled by `body.movement_speed_multiplier()`, with a comment
    /// calling that a strength: it is the leg-health figure, so how much
    /// somebody could carry was decided by how well they walked, and taking up
    /// a cart recomputed it. See ISSUES #87.
    ///
    /// What carrying actually depends on is the body's own strength and what
    /// there is to put things in.
    fn update_inventory_capacity_from_transport(&mut self) {
        let how_strong = self.body.how_much_this_body_can_lift();

        // And how old it is. A six-year-old's two hands are not a grown man's
        // two hands, and until now they were: this is the carrying half of
        // `what_a_body_this_age_can_do`. What goes in a basket is not scaled
        // - a travois drags the same load whoever is pulling it, and what
        // stops a child using one is the pulling, which is the movement half.
        let years = self.state.what_i_can_do_for_my_age();

        // And whether one of those hands has a child in it.
        //
        // "Age 0-2: must remain with a parent agent at all times. Parent agent
        // has one *hand* occupied with the child, limiting the types of work
        // the parent agent can accomplish." One hand, so half of what two of
        // them hold; the basket on the back is unaffected, which is exactly
        // why somebody carrying a baby wants one.
        let hands = if self.hands_full_of_child { 0.5 } else { 1.0 };

        let in_hand = Self::WHAT_TWO_HANDS_HOLD * how_strong * years * hands;
        let in_something = self.transport.total_additional_capacity();

        self.inventory.max_weight = in_hand + in_something;
    }

    /// Get movement speed including transport, fatigue, and pregnancy penalties
    pub fn movement_speed(&self) -> f32 {
        self.movement_speed_at_tick(0) // Default for backward compatibility
    }

    /// Get movement speed at a specific tick (includes pregnancy modifier)
    pub fn movement_speed_at_tick(&self, current_tick: u32) -> f32 {
        let body_speed = self.body.movement_speed_multiplier();
        let transport_speed = self.transport.speed_modifier();
        let weight_penalty = if self.inventory.is_overweight() {
            0.5 // 50% speed when overweight
        } else {
            1.0 - (self.inventory.weight_percentage() * 0.3) // Up to 30% slower at max weight
        };
        let fatigue_penalty = self.fatigue.movement_speed_modifier();
        let pregnancy_penalty = self.pregnancy.as_ref()
            .map(|p| p.speed_modifier(current_tick))
            .unwrap_or(1.0);

        // A short pair of legs covers less ground. The movement half of
        // `what_a_body_this_age_can_do`, and the reason a five-year-old is not
        // simply a small adult who eats less.
        body_speed
            * self.state.what_i_can_do_for_my_age()
            * transport_speed
            * weight_penalty
            * fatigue_penalty
            * pregnancy_penalty
    }

    /// Check if agent can carry additional weight
    pub fn can_carry(&self, additional_weight: f32) -> bool {
        self.inventory.current_weight + additional_weight <= self.inventory.max_weight
    }

    /// Get total carrying capacity (base + transport)
    pub fn total_carrying_capacity(&self) -> f32 {
        self.inventory.max_weight
    }

    /// Check if agent can reproduce (basic capability check)
    pub fn can_reproduce(&self) -> bool {
        if !self.state.is_alive || !self.state.life_stage.can_reproduce() {
            return false;
        }

        // Infertile agents cannot reproduce
        if self.traits.has(crate::core::traits::Trait::Infertile) {
            return false;
        }

        // Nobody already carrying a child starts another
        if self.pregnancy.is_some() {
            return false;
        }

        true
    }

    /// Check if agent is infertile
    pub fn is_infertile(&self) -> bool {
        self.traits.has(crate::core::traits::Trait::Infertile)
    }

    /// Whether this agent could carry a child.
    ///
    /// Anybody grown who is not already carrying one. There is no gender in
    /// this model - "agents are gender neutral; there are no male/female
    /// agents, merely child and adult agents" - so this replaces both
    /// `can_become_pregnant`, which asked whether somebody was female, and
    /// `can_impregnate`, which asked whether they were not.
    pub fn can_carry_a_child(&self) -> bool {
        self.can_reproduce() && self.pregnancy.is_none()
    }

    /// Check if this agent is currently pregnant
    pub fn is_pregnant(&self) -> bool {
        self.pregnancy.is_some()
    }



    /// How much of a stretch of going short counts against it.
    ///
    /// Not zero: hunger crossing its threshold is the ordinary signal that
    /// sends an agent to eat, so every well-fed agent trips it regularly. What
    /// matters is whether the asking went unanswered for any length of time.
    pub const GOING_SHORT: u32 = 12;

    /// Whether this agent has been going hungry rather than merely getting
    /// hungry.
    pub fn has_not_been_going_short(&self) -> bool {
        self.drives
            .get(DriveType::Hunger)
            .map(|drive| drive.denied_ticks() < Self::GOING_SHORT)
            .unwrap_or(true)
    }


    /// What the agent is carrying that it or a child could eat.
    ///
    /// Counts untracked stacks as well as ones with nutrition data on them:
    /// most of what an agent picks up off the land arrives as a plain "food"
    /// stack with no freshness attached, and a count that ignored those would
    /// report an empty pack for an agent carrying a fortnight's eating.
    ///
    /// What counts is `nutrition::is_this_food`, the same question
    /// `find_best_food_to_eat` asks, so that "I am provisioned" and "I can eat
    /// this" cannot disagree. They did: this used to match substrings from a
    /// list of six - `LOOKS_EDIBLE` - which counted an untracked stack of
    /// grain that nothing could eat, and counted untracked greens and roots as
    /// nothing at all, those being the whole of what a hedgerow gives for half
    /// the year and neither word being on the list.
    pub fn food_put_by(&self) -> u32 {
        self.inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter(|item| Self::is_this_worth_eating(item))
            .map(|item| item.quantity)
            .sum()
    }

    /// Whether this stack is food, and food that would not do harm.
    ///
    /// One place, because everything that asks about a pack asks this: what is
    /// put by, what there is to eat, and what the verb will accept.
    pub fn is_this_worth_eating(item: &InventoryItem) -> bool {
        if !crate::world::nutrition::is_this_food(&item.item_id) {
            return false;
        }
        match &item.food_data {
            Some(food) => !food.is_spoiled() && !food.is_harmful(),
            // Nothing known about it beyond what it is. An untracked stack has
            // no freshness to be suspicious of, so it is taken at face value.
            None => true,
        }
    }

    /// Whether this agent has any reason to think a child could be fed.
    ///
    /// Not "am I hungry this minute". That is answered by the last meal and
    /// says nothing about the next one, and a model that asks only that
    /// produces settlements which double in size while the crop halves - which
    /// is what thirty thousand ticks of tracing showed. A person who ate today
    /// but has nothing put by, on ground that has stopped giving, is in no
    /// position to raise a child.
    ///
    /// So: fed, watered and warm right now, no stretch of going short behind
    /// them, and food actually in hand for two.
    pub fn expects_to_be_able_to_feed_a_child(&self) -> bool {
        if !self.immediate_needs_met() || !self.has_not_been_going_short() {
            return false;
        }

        // There has to be food actually put by. Not "and also, alternatively,
        // a full belly" - which is what the second half of this used to say,
        // and which made the first half dead letter.
        //
        // `food_has_been_easy` reads the body's reserve, so it is true of
        // every healthy agent in the model: measured, a fed agent sits at
        // eighty-five to ninety-nine per cent of reserve, and the threshold is
        // eighty-five. Behind an `||` that meant the pack was never once the
        // binding question, and a settlement bred on the strength of having
        // eaten that morning - the exact reading the paragraph above this one
        // says is not good enough, written into the line underneath it.
        //
        // A full belly is not a surplus. A surplus is food that is still there
        // tomorrow.
        self.enough_put_by_for_a_child()
    }

    /// Whether there is enough put by to see this agent and a newborn through
    /// the stretch of the year the land gives nothing.
    ///
    /// What is put by, not what has been eaten: the pack and this agent's
    /// share of the camp's stores, with the stomach and the gut taken back off
    /// - see `WhatIsPutBy::units_put_by`. Against what the two of them would
    /// get through in that stretch, a newborn counting for a fifth of a grown
    /// appetite on the specification's own table.
    ///
    /// This is the whole of "do not breed until there is a surplus", and it is
    /// deliberately a hard number rather than a feeling. The settlement store
    /// is sized at exactly this stretch for one mouth (see the store's
    /// `what_one_mouth_wants_put_by`), so the gate says: breed when you have
    /// more put by than you need for yourself.
    ///
    /// Falls back to the pack alone before the first reckoning of the year has
    /// run, which is the only time `what_the_larder_says` is empty for a live
    /// agent.
    pub fn enough_put_by_for_a_child(&self) -> bool {
        let gap = super::provision::how_long_the_land_gives_nothing() as f32;
        let for_the_two_of_them = self.state.physiology.what_i_burn_in_a_day
            * (1.0 + what_a_body_this_age_eats(0));

        let put_by = match self.state.what_the_larder_says.as_ref() {
            Some(larder) => larder.units_put_by(),
            None => self.food_put_by() as f32 * super::provision::UNITS_IN_ONE_STORED_ITEM,
        };

        put_by >= for_the_two_of_them * gap
    }

    /// Check if agent should attempt reproduction given current survival state
    ///
    /// Returns false unless the agent is fed, watered and warm now, has not
    /// been going short, and is carrying enough food to feed a child as well
    /// as itself. Being un-hungry for a moment is not enough: it says nothing
    /// about whether the next meal exists.
    pub fn should_attempt_reproduction(&self) -> bool {
        if !self.can_reproduce() {
            return false;
        }

        self.expects_to_be_able_to_feed_a_child()
    }

    /// Get fertility level (0.0 to 1.0)
    pub fn fertility(&self) -> f32 {
        if !self.can_reproduce() {
            return 0.0;
        }

        // Base fertility from life stage
        let base_fertility = self.state.life_stage.fertility_multiplier();

        // Modified by health
        let health_factor = self.state.health / 100.0;

        // Modified by reproduction drive and personal modifier
        let reproduction_drive = self.drives.get(crate::core::DriveType::Reproduction)
            .map(|d| d.value)
            .unwrap_or(0.0);

        // Apply developmental nutrition modifier if finalized
        let dev_modifier = if self.developmental_nutrition.finalized {
            self.developmental_nutrition.stat_modifiers.fertility
        } else {
            1.0
        };

        // Fatigue reduces fertility (tired agents are less likely to reproduce)
        let fatigue_factor = match self.fatigue.severity() {
            super::fatigue::FatigueSeverity::None => 1.0,
            super::fatigue::FatigueSeverity::Mild => 0.9,
            super::fatigue::FatigueSeverity::Moderate => 0.6,
            super::fatigue::FatigueSeverity::Severe => 0.2,
        };

        // Clamped to keep the documented 0.0 to 1.0 contract. The personal
        // modifier reaches 1.8 and the developmental one 1.1, so an agent in
        // its prime can multiply out to nearly 2.0 - and callers treat this as
        // a probability. Multiplying two such values gave conception odds near
        // 4.0, which panics the sampler.
        let fertility = base_fertility
            * health_factor
            * (0.5 + reproduction_drive * 0.5)
            * self.reproduction_drive_modifier
            * dev_modifier
            * fatigue_factor;

        fertility.clamp(0.0, 1.0)
    }


    /// Create a default behavior tree for a specific drive
    fn create_default_tree_for_drive(drive_type: DriveType) -> BehaviorTree {
        let root = match drive_type {
            DriveType::Hunger => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("eat_stored_food".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("gather_food".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("hunt".to_string())));
                selector
            }
            DriveType::Thirst => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("drink_water".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("find_water".to_string())));
                selector
            }
            DriveType::Rest => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_shelter".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("sleep".to_string())));
                sequence
            }
            DriveType::Shelter => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("find_shelter".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("build_shelter".to_string())));
                selector
            }
            DriveType::Construction => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_materials".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("build_structure".to_string())));
                sequence
            }
            DriveType::Protection => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("go_to_child".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("seek_shelter".to_string())));
                selector
            }
            DriveType::Industry => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("mine_resources".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("process_materials".to_string())));
                selector
            }
            DriveType::Curiosity => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("explore".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("experiment".to_string())));
                selector
            }
            DriveType::Social => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("find_agents".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("socialize".to_string())));
                selector
            }
            DriveType::Utility => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_resources".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("craft_tools".to_string())));
                sequence
            }
            DriveType::Preparedness => {
                BehaviorNode::new(NodeType::Action("store_resources".to_string()))
            }
            DriveType::Sustenance => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("plant_crops".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("harvest".to_string())));
                selector
            }
            // The fear drive: get away from it, get behind something, or
            // failing both arm yourself against the next time.
            DriveType::Safety => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("flee".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("seek_shelter".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("craft_weapon".to_string())));
                selector
            }
            // And the anger drive, which has one answer.
            DriveType::Aggression => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("attack".to_string())));
                selector
            }
            DriveType::Reproduction => {
                let mut sequence = BehaviorNode::new(NodeType::Sequence);
                sequence.add_child(BehaviorNode::new(NodeType::Condition("has_resources".to_string())));
                sequence.add_child(BehaviorNode::new(NodeType::Action("reproduce".to_string())));
                sequence
            }
            DriveType::Luxury => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("seek_luxury".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("decorate".to_string())));
                selector
            }
            // Thirst is already handled at line 1230 above
        };

        BehaviorTree::new(format!("{:?}_tree", drive_type), root)
    }

    /// Select the most appropriate behavior tree based on current drive state
    pub fn select_behavior_tree(&mut self) -> Option<&mut BehaviorTree> {
        // Get the most urgent drive
        let most_urgent_drive = self.drives.most_urgent()?;

        // Find the behavior tree for this drive type
        self.behavior_trees
            .iter_mut()
            .find(|tree| tree.name.starts_with(&format!("{:?}", most_urgent_drive.drive_type)))
    }


    /// Note how an attempt turned out, so the agent can stop doing what does
    /// not work and do more of what does.
    ///
    /// Called for every action an agent takes. Most actions teach it nothing
    /// worth remembering - walking somewhere either works or the ground was in
    /// the way - so only the undertakings an agent could sensibly form an
    /// opinion about are recorded.
    /// The particular thing an action attempts, named finely enough to learn
    /// about: `gather:water` rather than `foraging`.
    pub fn what_was_tried(action: &Action) -> String {
        match action {
            Action::Gather { resource_type } => format!("gather:{resource_type}"),
            Action::Craft { item_type } => format!("craft:{item_type}"),
            // Digging yourself in is not framing. `build` wants poles in the
            // hand - right for a tent, wrong for a hole - so the matrix has
            // to be asked a different question about a burrow, and there has
            // always been a `burrow` verb in it to ask.
            Action::Build { structure_type, .. } if structure_type == "burrow" => {
                format!("burrow:{structure_type}")
            }
            Action::Build { structure_type, .. } => format!("build:{structure_type}"),
            Action::Store { item_type, .. } => format!("store:{item_type}"),
            Action::Eat { food_type } => format!("eat:{food_type}"),
            Action::Mate { .. } => "mate".to_string(),
            Action::Fish => "fish".to_string(),
            Action::SetSnare => "setsnare".to_string(),
            Action::CheckSnares => "checksnares".to_string(),
            Action::Hunt { .. } => "hunt".to_string(),
            Action::MakeClothing { garment } => format!("makeclothing:{garment}"),
            Action::Cook { .. } => "cook".to_string(),
            Action::LightFire => "lightfire".to_string(),
            Action::TillSoil => "tillsoil".to_string(),
            Action::TendField => "tendfield".to_string(),
            Action::TakeFrom { .. } => "takefrom".to_string(),
            Action::FleeFrom { .. } => "fleefrom".to_string(),
            Action::Freeze => "freeze".to_string(),
            Action::Examine { what } => format!("examine:{what}"),
            Action::Equip { .. } => "equip".to_string(),
            Action::Unequip { .. } => "unequip".to_string(),
            Action::Dry { .. } => "dry".to_string(),
            Action::Boil => "boil".to_string(),
            Action::Salt { .. } => "salt".to_string(),
            Action::Excavate => "excavate".to_string(),
            Action::Cover { .. } => "cover".to_string(),
            Action::PickUp { .. } => "pickup".to_string(),
            Action::PutDown { .. } => "putdown".to_string(),
            Action::Trade { .. } => "trade".to_string(),
            Action::GiveTo { .. } => "giveto".to_string(),
            Action::GoWithout { .. } => "gowithout".to_string(),
            Action::Work { verb, to } => format!("{verb}:{to}"),
            Action::Taste => "taste".to_string(),
            Action::TrySwapping {
                instead_of_making,
                instead_of,
                put_in,
            } => crate::environment::making::what_that_swap_is_called(
                instead_of_making,
                instead_of,
                put_in,
            ),
            Action::TakeCutting => "takecutting".to_string(),
            Action::PlantCutting => "plantcutting".to_string(),
            Action::SpreadMuck => "spreadmuck".to_string(),
            Action::Treat { .. } => "treat".to_string(),
            Action::Socialize { .. } => "socialize".to_string(),
            Action::AskAbout { what, .. } => format!("ask:{what}"),
            Action::ShareInformation { .. } => "shareinformation".to_string(),
            other => format!("{:?}", other)
                .split(|c: char| c == ' ' || c == '{' || c == '(')
                .next()
                .unwrap_or("")
                .to_lowercase(),
        }
    }

    pub fn learn_from(&mut self, action: &Action, worked: bool) {
        self.learn_from_this_here(action, worked, &[]);
    }

    /// The same, with what the world was doing at the time written down
    /// alongside it.
    ///
    /// Nobody names the situation. The circumstances are whatever the sky, the
    /// season and the ground happened to be, gathered by the simulation and
    /// attached to the attempt without either the agent or the code that chose
    /// the action having an opinion about which of them matter. Which of them
    /// matter is the thing the agent works out - see
    /// [`super::practices::Lessons::what_this_changes`].
    pub fn learn_from_this_here(
        &mut self,
        action: &Action,
        worked: bool,
        here: &[super::practices::Circumstance],
    ) {
        use super::practices::Undertaking;

        // The fine record, which is what decides whether this exact thing is
        // worth trying again. The coarse one below answers a different
        // question - what sort of person this is - and both are wanted.
        self.lessons
            .record_particular_here(&Self::what_was_tried(action), worked, here);

        let undertaking = match action {
            Action::Hunt { .. } => Undertaking::Hunting,
            Action::Fight { .. } => Undertaking::Fighting,
            // Running is the other answer to the same question, but it is not
            // the same lesson. Getting away teaches you that getting away
            // works; it must not teach you that you can win, or a man who has
            // outrun four wolves goes and picks a fight with the fifth
            Action::FleeFrom { .. } => Undertaking::Fleeing,
            // Freezing is not an attempt at anything and teaches nothing: an
            // agent that lived through it did not do so by freezing well
            Action::Freeze => return,
            // Doctoring is its own lesson. A man who has dosed four people
            // and watched them all get better anyway must not learn that he
            // is a hunter, and folding it into `Foraging` would teach him
            // about picking herbs rather than about giving them.
            Action::Treat { .. } => Undertaking::Healing,
            Action::Fish => Undertaking::Fishing,
            Action::SetSnare | Action::CheckSnares => Undertaking::Trapping,
            Action::Cook { .. } | Action::LightFire => Undertaking::Cooking,
            Action::TillSoil
            | Action::SpreadMuck
            | Action::TendField
            | Action::TakeCutting
            | Action::PlantCutting => Undertaking::Farming,
            // Digging a hole and filling it is putting something by, which is
            // the same undertaking as any other kind of husbandry
            Action::Excavate | Action::Cover { .. } => Undertaking::Farming,
            // Making food outlast the week it was got in is cookery, which is
            // where the rest of what a fire is for already lives
            Action::Dry { .. } => Undertaking::Cooking,
            Action::MakeClothing { .. } | Action::WearClothing { .. } => Undertaking::Clothing,
            Action::Gather { .. } => Undertaking::Foraging,
            Action::Build { .. } => Undertaking::Building,
            Action::Craft { .. } | Action::TrySwapping { .. } | Action::Work { .. } => {
                Undertaking::Crafting
            }
            Action::Socialize { .. }
            | Action::ShareInformation { .. }
            | Action::Trade { .. }
            | Action::GiveTo { .. }
            | Action::GoWithout { .. } => Undertaking::Dealing,
            _ => return,
        };

        self.lessons.record(undertaking, worked);

        // And drive the behaviour-tree weights from the same outcome. Those
        // weights were built to be the record of what works and never had a
        // caller, so nothing an agent did had ever changed what it did next.
        let action_name = format!("{:?}", undertaking).to_lowercase();
        for tree in &mut self.behavior_trees {
            if worked {
                tree.reinforce_action(&action_name, 0.05);
            } else {
                tree.penalize_action(&action_name, 0.05);
            }
        }
    }

    /// Link what was just done to the need it answered.
    ///
    /// The specification's pattern formation: "when an agent satisfies drive
    /// demand, it links its previous actions taken to the drive satisfaction
    /// to form a pattern". The action and the ground it was done on go down
    /// against every need the doing of it actually eased.
    ///
    /// A drive that barely moved is not evidence of anything - joining that to
    /// whatever the agent happened to be doing is how a superstition gets
    /// made - so only a real fall counts. And an action that was *aimed* at a
    /// need and did not answer it counts against the pattern, so that ground
    /// which has stopped working stops being worth the walk back.
    pub fn link_what_worked(
        &mut self,
        action: &Action,
        action_result: &ActionResult,
        aimed_at: DriveType,
        where_it_was: (i32, i32, i32),
        now: u32,
    ) {
        use super::patterns::Patterns;

        let elements = self.what_this_episode_was_made_of(action, where_it_was, now);
        let turns = self.how_long_that_took();
        let mut answered_anything = false;

        for (need, change) in &action_result.drive_changes {
            if *change <= -Patterns::ENOUGH_TO_NOTICE {
                // Efficiency, not the bare fact of it: how much demand came
                // off per turn spent getting it off. Somebody who can answer
                // a need quickly has the rest of the day for the other ones,
                // and that is what makes one way of doing it better than
                // another rather than merely possible.
                let efficiency = -*change / turns as f32;
                self.patterns.it_worked(*need, &elements, efficiency, now);
                if *need == aimed_at {
                    answered_anything = true;
                }
            }
        }

        if !answered_anything {
            self.patterns.it_did_not(aimed_at, &elements);
        }
    }

    /// The elements of what just happened: everything that was true of it
    /// which some later episode might also be true of.
    ///
    /// This is the whole of the generalising. Two hunts that fed somebody
    /// share `Did("hunt")` and `On("Deer")` and differ in `At` and `Toward`,
    /// so the doing outgrows the direction without anybody deciding that it
    /// should.
    pub fn what_this_episode_was_made_of(
        &self,
        action: &Action,
        where_it_was: (i32, i32, i32),
        now: u32,
    ) -> Vec<super::patterns::Element> {
        use super::patterns::{Bearing, Element};
        use crate::environment::seasons::{Season, DAYS_PER_YEAR, TICKS_PER_DAY};

        let tried = Self::what_was_tried(action);
        let mut elements = Vec::with_capacity(5);

        // `what_was_tried` writes "gather:Berries" - the verb and the thing it
        // was done to, glued. Split, they are two elements that vary
        // independently, which is what lets an agent learn that gathering
        // pays without concluding that berries are the only thing worth
        // gathering.
        match tried.split_once(':') {
            Some((verb, subject)) => {
                elements.push(Element::Did(verb.to_string()));
                elements.push(Element::On(subject.to_string()));
            }
            None => elements.push(Element::Did(tried)),
        }

        elements.push(Element::At(where_it_was));

        if let Some(errand) = &self.errand {
            if let Some(bearing) = Bearing::from_home(errand.set_out_from, where_it_was) {
                elements.push(Element::Toward(bearing));
            }
        }

        let day_of_year = (now / TICKS_PER_DAY) % DAYS_PER_YEAR;
        elements.push(Element::When(Season::from_day_of_year(day_of_year)));

        elements
    }

    /// Read the felt total of what this one expects its habits to cost it.
    ///
    /// Worry is not accumulated here; it is accumulated against the elements
    /// of the things that earned it, and this is the sum of that. Doing it the
    /// other way round would give an agent two records of the same fear which
    /// would drift apart, and the one in the pattern layer is the one that can
    /// actually be acted on.
    fn feel_what_the_habits_are_costing(&mut self) {
        self.emotions.worry = self.patterns.everything_i_dread().clamp(0.0, 1.0);
    }

    /// Something has cost this one future satisfaction of a drive.
    ///
    /// Whatever it has been doing lately takes the blame, against the drive
    /// that took the loss - see `Patterns::it_cost_me`. What sorts out which
    /// of those things actually caused it is repetition and nothing else.
    pub fn this_cost_me(&mut self, cost_to: DriveType, how_much: f32, now: u32) {
        self.patterns.it_cost_me(cost_to, how_much, now);
        self.feel_what_the_habits_are_costing();
    }

    /// How much worry is pressing on a particular drive.
    ///
    /// This is worry feeding the drive layer: a man who expects his standing
    /// to suffer attends to his standing. It is what turns "I am wary of this"
    /// into a reason to go and do something about it, which is the whole
    /// point - worry has to make somebody *act*, not merely refuse.
    pub fn what_worry_adds_to(&self, drive: DriveType) -> f32 {
        self.patterns.how_much_i_fear_for(drive).clamp(0.0, Self::THE_MOST_WORRY_CAN_ADD)
    }

    /// How far a worry can push a drive on its own.
    ///
    /// Well short of the thing actually going wrong. Being worried about your
    /// standing is not the same as being friendless, and an agent that treated
    /// them alike would spend its life mending fences nobody had broken.
    pub const THE_MOST_WORRY_CAN_ADD: f32 = 0.25;

    /// How many turns went into what was just finished.
    ///
    /// The walk as well as the work: an errand that took nine turns to reach a
    /// bush and one to strip it cost ten, and pricing it at one is what makes
    /// a far-off meal look as cheap as a near one.
    fn how_long_that_took(&self) -> u32 {
        self.errand
            .as_ref()
            .map(|errand| errand.turns_on_it.max(1))
            .unwrap_or(1)
    }

    /// Ground this agent would walk back to for a need, if any.
    ///
    /// Not the tile it is standing on: this answers "where do I go", and the
    /// answer "here" is no answer.
    pub fn somewhere_that_answered(
        &self,
        need: DriveType,
        from: (i32, i32, i32),
        now: u32,
    ) -> Option<(i32, i32, i32)> {
        let there = self.patterns.where_it_worked(need, now)?;

        if (there.0 - from.0).abs() + (there.1 - from.1).abs() <= 1 {
            None
        } else {
            Some(there)
        }
    }

    /// Process feedback from action execution
    pub fn apply_feedback(&mut self, action_result: &ActionResult, _drive_type: DriveType) {
        // Apply all drive changes from the action result
        for (affected_drive, change_amount) in &action_result.drive_changes {
            if let Some(drive) = self.drives.get_mut(*affected_drive) {
                if *change_amount < 0.0 {
                    // Negative value = satisfaction (decrease drive)
                    drive.partial_satisfy(change_amount.abs());
                } else {
                    // Positive value = increase drive
                    drive.increase(*change_amount);
                }
            }
        }
    }

    /// Apply trait-based happiness rewards based on completed actions
    /// This is called after successful actions to give trait holders bonus happiness
    pub fn apply_trait_action_rewards(&mut self, action: &crate::environment::Action) {
        use crate::core::traits::Trait;
        use super::EmotionSource;

        let mut happiness_bonus = 0.0;
        let mut reward_reason = String::new();

        match action {
            // Building actions reward Builder trait
            crate::environment::Action::Build { .. } => {
                if self.traits.has(Trait::Builder) {
                    happiness_bonus += Trait::Builder.happiness_gain() * 0.02;
                    reward_reason = "building_satisfaction".to_string();
                }
                // Proud trait holders gain happiness from accomplishments
                if self.traits.has(Trait::Proud) {
                    happiness_bonus += Trait::Proud.happiness_gain() * 0.01;
                }
                // Ambitious trait holders gain happiness from external goals
                if self.traits.has(Trait::Ambitious) {
                    happiness_bonus += 0.05;
                }
            }

            // Crafting actions reward CraftObsessed trait
            crate::environment::Action::Craft { .. } => {
                if self.traits.has(Trait::CraftObsessed) {
                    happiness_bonus += 0.08; // Significant happiness from crafting
                    reward_reason = "craft_satisfaction".to_string();
                }
                // Handy trait holders gain happiness from completing tasks
                if self.traits.has(Trait::Handy) {
                    happiness_bonus += Trait::Handy.happiness_gain() * 0.02;
                }
                if self.traits.has(Trait::Proud) {
                    happiness_bonus += Trait::Proud.happiness_gain() * 0.01;
                }
                // Ascetic trait: slight discomfort crafting non-essential items
                if self.traits.has(Trait::Ascetic) {
                    happiness_bonus -= 0.02;
                }
            }

            // Gathering/working actions reward Diligent trait, penalize Lazy
            crate::environment::Action::Gather { .. } => {
                if self.traits.has(Trait::Diligent) {
                    happiness_bonus += Trait::Diligent.happiness_gain() * 0.02;
                    reward_reason = "work_satisfaction".to_string();
                }
                // Lazy trait holders lose happiness from work
                if self.traits.has(Trait::Lazy) {
                    happiness_bonus -= 0.03; // Constant happiness decrease when working
                }
                // Traditionalist trait: happiness from using primitive (wood/stone) tools
                if self.traits.has(Trait::Traditionalist) && self.equipment.has_primitive_tool() {
                    happiness_bonus += 0.04;
                    reward_reason = "traditional_tools_satisfaction".to_string();
                }
            }

            // Exploring actions reward Explorer and Curious traits
            crate::environment::Action::Move { .. } |
            crate::environment::Action::Explore { .. } => {
                if self.traits.has(Trait::Explorer) {
                    happiness_bonus += Trait::Explorer.happiness_gain() * 0.01;
                    reward_reason = "exploration_joy".to_string();
                }
                if self.traits.has(Trait::Curious) {
                    happiness_bonus += Trait::Curious.happiness_gain() * 0.005;
                }
            }

            // Social interactions reward Extrovert, penalize Introvert
            crate::environment::Action::Socialize { .. } |
            crate::environment::Action::ShareInformation { .. } => {
                if self.traits.has(Trait::Extrovert) || self.traits.has(Trait::Sociable) {
                    happiness_bonus += Trait::Extrovert.happiness_gain() * 0.03;
                    reward_reason = "social_joy".to_string();
                }
                if self.traits.has(Trait::Charismatic) {
                    happiness_bonus += 0.02;
                }
                // Introverts lose happiness from socializing
                if self.traits.has(Trait::Introvert) || self.traits.has(Trait::Introverted) {
                    happiness_bonus -= 0.02;
                }
            }

            // Sleeping rewards introverts who enjoy solitude
            crate::environment::Action::Sleep { .. } => {
                // Introverts gain slight happiness from being alone/resting
                if self.traits.has(Trait::Introvert) || self.traits.has(Trait::Introverted) {
                    happiness_bonus += 0.01;
                    reward_reason = "peaceful_solitude".to_string();
                }
            }

            // Eating rewards Glutton trait
            crate::environment::Action::Eat { .. } => {
                if self.traits.has(Trait::Glutton) {
                    happiness_bonus += 0.05; // Extra happiness from eating
                    reward_reason = "food_enjoyment".to_string();
                }
                // Ascetic trait: no extra happiness from food, slight penalty for elaborate meals
                if self.traits.has(Trait::Ascetic) {
                    // Negate glutton bonus if somehow both traits exist
                    happiness_bonus -= 0.03; // Prefers simple sustenance
                }
                // Survivalist trait: happiness from eating own stored food
                // (They're happy just knowing they're self-sufficient)
                if self.traits.has(Trait::Survivalist) {
                    happiness_bonus += 0.03;
                    reward_reason = "self_sufficient_meal".to_string();
                }
            }

            // Animal interactions reward AnimalLover, penalize Allergic
            crate::environment::Action::Tame { .. } |
            crate::environment::Action::CollectAnimalProduct { .. } => {
                if self.traits.has(Trait::AnimalLover) {
                    happiness_bonus += 0.06;
                    reward_reason = "animal_joy".to_string();
                }
                if self.traits.has(Trait::Allergic) {
                    happiness_bonus -= 0.03; // Discomfort from animal proximity
                }
            }

            // Hunting can reward Protector trait (protecting community from threats)
            crate::environment::Action::Hunt { .. }
            | crate::environment::Action::Fight { .. } => {
                if self.traits.has(Trait::Protector) {
                    happiness_bonus += 0.04;
                    reward_reason = "protector_satisfaction".to_string();
                }
                // AnimalLover may feel conflicted about hunting
                if self.traits.has(Trait::AnimalLover) {
                    happiness_bonus -= 0.02;
                }
            }

            // Attack actions - check for aggressive vs peaceful traits
            crate::environment::Action::Attack { .. } => {
                if self.traits.has(Trait::Aggressive) {
                    happiness_bonus += 0.03;
                    reward_reason = "combat_thrill".to_string();
                }
                if self.traits.has(Trait::Peaceful) || self.traits.has(Trait::Pacifist) {
                    happiness_bonus -= 0.05; // Distress from violence
                }
            }

            // Store/retrieve items - check for Frugal/Greedy
            crate::environment::Action::Store { .. } => {
                if self.traits.has(Trait::Frugal) {
                    happiness_bonus += 0.02; // Satisfaction from saving
                    reward_reason = "saving_satisfaction".to_string();
                }
            }

            crate::environment::Action::Retrieve { .. } => {
                if self.traits.has(Trait::Greedy) {
                    happiness_bonus += 0.02; // Joy from acquiring
                    reward_reason = "acquisition_joy".to_string();
                }
            }

            // Mating rewards Romantic trait
            crate::environment::Action::Mate { .. } => {
                if self.traits.has(Trait::Romantic) {
                    happiness_bonus += 0.1; // Significant happiness from romantic interaction
                    reward_reason = "romantic_joy".to_string();
                }
            }

            _ => {}
        }

        // Copycat trait: happiness from mimicking recently observed actions
        if self.traits.has(Trait::Copycat) {
            if let Some(action_type) = self.action_to_observable_type(action) {
                // Check if we've seen this action type recently (within 50 ticks)
                // Note: current tick tracking would be needed, but we use a simpler approach
                // by checking if any recent observations exist
                let observation_count = self.observational_learning.count_recent_observations_of_type(
                    action_type,
                    50, // Within last 50 ticks
                    0   // Will use recent count which doesn't need exact tick
                );

                if observation_count > 0 {
                    // Bonus happiness for mimicking others
                    happiness_bonus += 0.03 * (observation_count as f32).min(3.0);
                    reward_reason = "copycat_satisfaction".to_string();
                }
            }
        }

        // Apply the happiness bonus if any
        if happiness_bonus != 0.0 {
            let source = if reward_reason.is_empty() {
                EmotionSource::Event("trait_reward".to_string())
            } else {
                EmotionSource::Event(reward_reason)
            };

            if happiness_bonus > 0.0 {
                self.emotions.add_happiness_with_traits(source, happiness_bonus, &self.traits);
            } else {
                // Negative bonus means we reduce happiness or add sadness
                self.emotions.add_sadness_with_traits(source, happiness_bonus.abs(), &self.traits);
            }
        }
    }


    // Helper methods

    /// Convert an environment Action to an ObservableActionType for Copycat trait
    fn action_to_observable_type(&self, action: &crate::environment::Action) -> Option<super::ActionType> {
        use crate::environment::Action;
        use super::ActionType;

        match action {
            // Mining/gathering type actions
            Action::Gather { .. } => Some(ActionType::Mining),

            // Crafting actions
            Action::Craft { .. } => Some(ActionType::Crafting),

            // Building actions
            Action::Build { .. } => Some(ActionType::Building),

            // Combat actions
            Action::Attack { .. } | Action::Hunt { .. } | Action::Fight { .. } => {
                Some(ActionType::Combat)
            }

            // Cooking and food preparation
            Action::Eat { .. } => Some(ActionType::Cooking),

            // Social interactions
            Action::Socialize { .. } | Action::ShareInformation { .. } => Some(ActionType::Social),

            // Exploration/navigation
            Action::Move { .. } | Action::Explore { .. } => Some(ActionType::Navigation),

            // Tool use (storing, retrieving, etc.)
            Action::Store { .. } | Action::Retrieve { .. } => Some(ActionType::ToolUse),

            // Animal interactions involve problem solving
            Action::Tame { .. } | Action::CollectAnimalProduct { .. } => Some(ActionType::ProblemSolving),

            // Other actions that don't fit categories
            _ => None,
        }
    }

    /// Find the nearest known shelter (housing building) from the agent's exploration knowledge.
    /// Returns the position of the nearest shelter, or the agent's current position if no shelter is known.
    fn find_nearest_shelter(&self) -> (i32, i32, i32) {
        use crate::world::BuildingType;

        // Housing building types that provide shelter
        let shelter_types = [
            BuildingType::Longhouse,
            BuildingType::UpgradedLonghouse,
            BuildingType::SmallHouse,
            BuildingType::MediumHouse,
            BuildingType::LargeHouse,
            BuildingType::Manor,
        ];

        let current_pos = self.state.position;
        let mut nearest_shelter: Option<(i32, i32, i32)> = None;
        let mut nearest_dist_sq = f32::MAX;

        // Search through known buildings for housing/shelter
        for (position, building_type) in &self.exploration_knowledge.known_buildings {
            if shelter_types.contains(building_type) {
                // Calculate squared distance (avoid sqrt for performance)
                let dx = (position.x - current_pos.0) as f32;
                let dy = (position.y - current_pos.1) as f32;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < nearest_dist_sq {
                    nearest_dist_sq = dist_sq;
                    nearest_shelter = Some((position.x, position.y, 0));
                }
            }
        }

        // Return nearest shelter or current position if none known
        nearest_shelter.unwrap_or(current_pos)
    }


    // ===== Sensory Processing Integration =====

    /// Process current sensory input into meaningful percepts
    pub fn process_percepts(&self) -> Vec<super::sensory_processing::Percept> {
        super::sensory_processing::process_sensory_input(&self.senses, self.state.position)
    }

    /// Get the most salient (attention-grabbing) percept based on current drives
    pub fn most_salient_percept(&self) -> Option<super::sensory_processing::Percept> {
        let percepts = self.process_percepts();
        super::sensory_processing::most_salient_percept(&percepts, &self.drives)
            .cloned()
    }

    /// Get all percepts above a salience threshold
    pub fn filter_percepts_by_salience(&self, threshold: f32) -> Vec<super::sensory_processing::Percept> {
        let percepts = self.process_percepts();
        super::sensory_processing::filter_by_salience(percepts, &self.drives, threshold)
    }

    /// Get percept salience score (0.0 to 1.0) based on current drives
    pub fn percept_salience(&self, percept: &super::sensory_processing::Percept) -> f32 {
        super::sensory_processing::calculate_salience(percept, &self.drives)
    }

    /// Check if any danger percepts are detected
    pub fn senses_danger_percept(&self) -> bool {
        let percepts = self.process_percepts();
        percepts.iter().any(|p| matches!(p, super::sensory_processing::Percept::DangerDetected { .. }))
    }

    /// Get all detected agents from percepts
    pub fn get_detected_agents(&self) -> Vec<Uuid> {
        let percepts = self.process_percepts();
        percepts.iter()
            .filter_map(|p| {
                if let super::sensory_processing::Percept::AgentDetected { agent_id, .. } = p {
                    Some(*agent_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all detected resources from percepts
    pub fn get_detected_resources(&self) -> Vec<(String, (i32, i32, i32))> {
        let percepts = self.process_percepts();
        percepts.iter()
            .filter_map(|p| {
                if let super::sensory_processing::Percept::ResourceDetected { resource_type, position, .. } = p {
                    Some((resource_type.clone(), *position))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the highest-priority threat from danger percepts
    pub fn get_primary_threat(&self) -> Option<(super::sensory_processing::ThreatType, f32)> {
        let percepts = self.process_percepts();
        percepts.iter()
            .filter_map(|p| {
                if let super::sensory_processing::Percept::DangerDetected { threat_type, severity, .. } = p {
                    Some((*threat_type, *severity))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }

    // ===== Equipment Management =====





    /// Get reference to currently equipped item in a slot
    pub fn get_equipped(&self, slot: super::equipment::EquipmentSlot) -> Option<&super::equipment::EquipmentItem> {
        self.equipment.get_equipped(slot)
    }

    /// Check if a specific slot is occupied
    pub fn is_slot_equipped(&self, slot: super::equipment::EquipmentSlot) -> bool {
        self.equipment.get_equipped(slot).is_some()
    }





    /// Get tool efficiency bonus from equipped tools
    pub fn get_tool_efficiency(&self, tool_type: &str) -> f32 {
        // Check main hand for matching tool
        if let Some(item) = self.equipment.get_equipped(super::equipment::EquipmentSlot::MainHand) {
            if item.name.contains(tool_type) {
                // Use average of mining and harvesting speed as tool efficiency
                return (item.effective_mining_speed() + item.effective_harvesting_speed()) / 2.0;
            }
        }
        1.0 // Default efficiency if no tool equipped
    }

    /// Apply durability loss to equipped item in a slot (e.g., when using a tool)
    pub fn damage_equipment(&mut self, slot: super::equipment::EquipmentSlot, damage: f32) -> Result<bool, String> {
        if let Some(item) = self.equipment.get_equipped_mut(slot) {
            item.apply_wear(damage);

            // Check if item broke
            if item.is_broken() {
                // Remove broken item
                self.equipment.unequip(slot);
                return Ok(true); // Item broke
            }
            Ok(false) // Item damaged but not broken
        } else {
            Err(format!("No item equipped in slot {:?}", slot))
        }
    }

    /// Repair equipped item in a slot
    pub fn repair_equipment(&mut self, slot: super::equipment::EquipmentSlot, amount: f32) -> Result<(), String> {
        if let Some(item) = self.equipment.get_equipped_mut(slot) {
            item.repair(amount);
            Ok(())
        } else {
            Err(format!("No item equipped in slot {:?}", slot))
        }
    }

    /// Check if agent is encumbered by equipment weight
    pub fn is_encumbered(&self) -> bool {
        self.equipment.is_encumbered()
    }



    /// Get all equipped items
    pub fn get_all_equipped(&self) -> Vec<&super::equipment::EquipmentItem> {
        self.equipment.get_all_equipped()
    }


    /// Get harvesting speed bonus from equipped tools
    pub fn get_harvesting_speed_bonus(&self) -> f32 {
        self.equipment.harvesting_speed_bonus()
    }

    /// Decide what storage action to take (if any) based on inventory and storage preferences
    ///
    /// When survival drives (hunger/thirst) are active, agents will only retrieve food
    /// from storage - they will NOT deposit resources when their survival is threatened.
    /// Returns Some(Action) if agent should interact with storehouse, None otherwise
    pub fn decide_storage_action(
        &self,
        storehouse_food: u32,
        storehouse_resources: u32,
    ) -> Option<crate::environment::Action> {
        use crate::agents::storage_integration::{
            count_food_in_inventory, count_resources_in_inventory, count_tools_in_inventory,
            item_type_to_id,
        };
        use crate::agents::storage_management::decide_storage_action;
        use crate::environment::Action;

        use log::debug;

        // Check if survival drives are active - if so, only allow food retrieval
        let hunger_active = self.drives.get(DriveType::Hunger)
            .map(|d| d.is_active())
            .unwrap_or(false);
        let thirst_active = self.drives.get(DriveType::Thirst)
            .map(|d| d.is_active())
            .unwrap_or(false);
        let survival_threatened = hunger_active || thirst_active;

        // Count what agent has
        let agent_food = count_food_in_inventory(&self.inventory);
        let agent_resources = count_resources_in_inventory(&self.inventory);
        let agent_tools = count_tools_in_inventory(&self.inventory);

        // Get preparedness drive level
        let preparedness = self.drives.get(crate::core::DriveType::Preparedness)
            .map(|d| d.value)
            .unwrap_or(0.0);

        // Make storage decision
        let decision = decide_storage_action(
            agent_food,
            agent_resources,
            agent_tools,
            storehouse_food,
            storehouse_resources,
            preparedness,
            &self.storage_preferences,
        );

        use crate::agents::storage_management::StorageDecision;
        match decision {
            StorageDecision::Deposit { item_type, quantity, reason } => {
                // When survival is threatened, do NOT deposit anything
                // The agent should focus on their own survival, not community contribution
                if survival_threatened {
                    debug!("Agent {} skipping deposit (survival threatened): {}", self.id, reason);
                    return None;
                }
                debug!("Agent {} storing: {}", self.id, reason);
                Some(Action::Store {
                    item_type: item_type_to_id(item_type),
                    amount: quantity,
                })
            }
            StorageDecision::Retrieve { item_type, quantity, reason } => {
                debug!("Agent {} retrieving: {}", self.id, reason);
                Some(Action::Retrieve {
                    item_type: item_type_to_id(item_type),
                    amount: quantity,
                })
            }
            StorageDecision::NoAction { .. } => None,
        }
    }

    // ========== Survival API Methods (for TDD tests) ==========

    /// Eat food from inventory and satisfy hunger drive (legacy method)
    /// Returns true if food was consumed
    /// Note: Use eat_food_item for nutrition-aware eating
    pub fn eat_food(&mut self, amount: u32) -> bool {
        // Read the food data before consuming, so the item can go through
        // `remove_item` - that drops emptied stacks and keeps carried weight
        // correct, where decrementing in place leaves a zero-quantity entry
        // that still reads as "carrying food"
        let carried = match self.inventory.get_item("food") {
            Some(item) if item.quantity >= amount => Some(item.food_data.clone()),
            _ => None,
        };

        let Some(food_data) = carried else {
            return false;
        };

        self.inventory.remove_item("food", amount);

        // If food has nutrition data, use it
        if let Some(food_data) = food_data {
            let nutrition = food_data.effective_nutrition();
            self.nutrition.consume(&nutrition.scale(amount as f32));

            // Also satisfy thirst from water content
            if nutrition.water_content > 0.3 {
                if let Some(thirst) = self.drives.get_mut(DriveType::Thirst) {
                    thirst.decrease(nutrition.water_content * 0.1 * amount as f32);
                }
            }
        } else {
            // Legacy: no food data, use old flat rate
            let energy_restored = (amount as f32) * 20.0;
            self.state.energy = (self.state.energy + energy_restored).min(100.0);
        }

        // Reset starvation (use current age as approximation of tick)
        self.state.last_ate_tick = self.state.age;
        self.state.ticks_without_food = 0;

        // Satisfy hunger drive
        let hunger_reduction = (amount as f32) * 0.2; // Each food reduces hunger by 0.2
        if let Some(hunger) = self.drives.get_mut(DriveType::Hunger) {
            hunger.decrease(hunger_reduction);
        }

        true
    }

    /// Eat a specific food item from inventory with full nutrition tracking
    /// Returns the result of eating including nutrition gained or problems
    pub fn eat_food_item(&mut self, item_id: &str, current_tick: u32) -> EatResult {
        // Read what we need up front so the item can be consumed through
        // `remove_item`, which drops emptied stacks and keeps carried weight
        // correct. Decrementing the quantity in place leaves a zero-quantity
        // entry behind, and an agent holding one reads as "still carrying
        // food" forever - so it keeps trying to eat nothing instead of going
        // to look for a meal.
        let (has_food, food_data) = match self.inventory.get_item(item_id) {
            Some(item) if item.quantity > 0 => (true, item.food_data.clone()),
            _ => (false, None),
        };

        if !has_food {
            return EatResult::NoFood;
        }

        // You cannot eat a stone. The callers all filter for food before they
        // get here, and that was the whole of the guard: `Piece::can_it_be_eaten`
        // asks only whether a thing is an uncut carcass, so anything that was
        // not a carcass passed. Called with "wood", "stone", "clay", "bowl" or
        // "flax" this returned Success, credited twenty energy, fed
        // `nutrition.consume` and dropped the hunger drive. Nothing reached it
        // that way in a live run - but the rule lived in the callers and not
        // in the verb, which is the shape every defect in this file has had.
        if !crate::world::nutrition::is_this_food(item_id) {
            return EatResult::NoFood;
        }

        // You cannot eat a deer either. The decision layer knows this and cuts
        // one up first, but the executor is the place it has to be true: an
        // agent handed a carcass by any other route would otherwise swallow
        // two kilos of raw beast in a tick.
        if !crate::world::nutrition::Piece::of(item_id).can_it_be_eaten() {
            return EatResult::NoFood;
        }

        let food_data = match food_data {
            Some(data) => data,
            None => {
                // Not a tracked food item - consume 1 with flat nutrition
                self.inventory.remove_item(item_id, 1);
                self.food_i_ate = self.food_i_ate.saturating_add(1);
                let flat_nutrition =
                    crate::world::nutrition::what_an_untracked_mouthful_is_worth();
                self.nutrition.consume(&flat_nutrition);
                self.state.took_a_meal(
                    current_tick,
                    crate::world::Soil::waste_from_eating(item_id),
                );
                if let Some(hunger) = self.drives.get_mut(DriveType::Hunger) {
                    hunger.decrease(0.2);
                }
                return EatResult::Success(flat_nutrition);
            }
        };

        // Check if food is harmful (severely spoiled)
        if food_data.is_harmful() {
            self.inventory.remove_item(item_id, 1);
            self.food_i_ate = self.food_i_ate.saturating_add(1);
            let damage = 10.0;
            self.state.lose_health(damage, "a blow");
            return EatResult::MadeSick(damage);
        }

        // Check if food is spoiled (inedible)
        if food_data.is_spoiled() {
            return EatResult::Spoiled;
        }

        // Consume the food
        self.inventory.remove_item(item_id, 1);
        self.food_i_ate = self.food_i_ate.saturating_add(1);

        // And what it might cost. Two gambles, both of which a fire settles:
        // raw flesh, and food that has started to go but has not gone far
        // enough to be obviously carrion.
        //
        // This is the first illness in the model. Before it, cooking was
        // worth 2.7 times the nutrition and nothing else, so there was no
        // reason to fetch wood for a fire you did not strictly need.
        {
            use rand::Rng;
            let mut rng = crate::core::dice::roll();

            let raw_flesh = food_data.preparation
                == crate::world::nutrition::PreparationState::Raw
                && crate::world::nutrition::Piece::is_it_flesh(item_id);

            if raw_flesh && rng.gen_bool(Self::HOW_OFTEN_RAW_FLESH_TELLS) {
                self.taken_ill_with(Self::OFF_RAW_FLESH, 0.5, current_tick);
            } else if food_data.freshness < Self::ON_THE_TURN {
                // The further gone it is, the likelier it is to tell. At the
                // point where it counts as harmful it is a different and
                // worse question, handled above.
                let how_far_gone =
                    1.0 - (food_data.freshness / Self::ON_THE_TURN).clamp(0.0, 1.0);
                let odds = Self::HOW_OFTEN_FOOD_ON_THE_TURN_TELLS * how_far_gone as f64;

                if rng.gen_bool(odds.clamp(0.0, 1.0)) {
                    self.taken_ill_with(
                        Self::OFF_FOOD_ON_THE_TURN,
                        0.3 + 0.4 * how_far_gone,
                        current_tick,
                    );
                }
            }
        }

        // Get effective nutrition (preparation + freshness factors applied)
        let nutrition = food_data.effective_nutrition();

        // Apply nutrition to agent
        self.nutrition.consume(&nutrition);

        // Satisfy thirst from water content
        if nutrition.water_content > 0.3 {
            if let Some(thirst) = self.drives.get_mut(DriveType::Thirst) {
                thirst.decrease(nutrition.water_content * 0.1);
            }
        }

        // Reset starvation timer, and note what the body will have to pass
        self.state.took_a_meal(
            current_tick,
            crate::world::Soil::waste_from_eating(item_id),
        );

        // Satisfy hunger based on total nutrition
        let hunger_reduction = nutrition.total() / 100.0 * 0.3;
        if let Some(hunger) = self.drives.get_mut(DriveType::Hunger) {
            hunger.decrease(hunger_reduction);
        }

        EatResult::Success(nutrition)
    }

    /// Whether the agent is carrying anything it can safely eat
    /// Whether there is anything in the pack this one could make a meal of.
    ///
    /// Exactly what `find_best_food_to_eat` would return, and nothing else.
    /// There used to be a second clause here reaching for the literal item id
    /// `"food"`, because the search above could not see untracked stacks - so
    /// a pack of untracked grain, berries or fish answered *false* to this
    /// while `food_put_by` counted every one of them. The decision layer asks
    /// this before it chooses to eat, so those agents read as provisioned and
    /// never once ate what they were carrying. The search sees them now.
    pub fn has_edible_food(&self) -> bool {
        self.find_best_food_to_eat().is_some()
    }

    /// Find the best food item to eat based on nutritional needs and freshness
    /// How many meals are actually in the pack.
    ///
    /// Not the same question as how much food is in the pack. A haunch nobody
    /// has taken a knife to, a stack that has gone over, a strip of raw flesh
    /// that made this one ill last time - all of them answer `is_food` and
    /// none of them is supper. Anything deciding whether a person needs to go
    /// and get something wants this count and not the other one, or a man
    /// carrying a rotten carcass reads as provisioned.
    pub fn how_many_meals_i_have(&self) -> u32 {
        self.inventory
            .items
            .iter()
            .filter(|(item_id, _)| self.is_this_a_meal(item_id))
            .map(|(_, item)| item.quantity)
            .sum()
    }

    /// How much food about this person is still worth something to them.
    ///
    /// Not the same question as `how_many_meals_i_have`, and not the same as
    /// how much food is in the pack. A whole fish nobody has taken a knife to
    /// is not a meal, but it is a fish and it will be a meal shortly, so a man
    /// carrying six of them has no business going back to the river. A fish
    /// that has **gone over** is neither.
    ///
    /// Anything deciding whether somebody has enough about them to stop
    /// getting more wants this. Asking `is_food` instead counts the rot, and
    /// the effect of that is savage once food actually keeps an honest clock:
    /// a settlement gathered **a third less** and ate **less than half as
    /// much**, because men stood next to hedges declining to pick anything
    /// with eight units of mould in the pack. See ISSUES_FOUND #65.
    pub fn how_much_good_food_i_have(&self) -> u32 {
        self.inventory
            .items
            .iter()
            .filter(|(_, item)| item.quantity > 0)
            .filter(|(_, item)| match item.food_data {
                Some(ref food) => !food.is_spoiled() && !food.is_harmful(),
                None => false,
            })
            .map(|(_, item)| item.quantity)
            .sum()
    }

    /// Whether one thing in the pack is something this person would eat, on
    /// the same terms `find_best_food_to_eat` picks by.
    fn is_this_a_meal(&self, item_id: &str) -> bool {
        let Some(item) = self.inventory.items.get(item_id) else {
            return false;
        };
        if item.quantity == 0 {
            return false;
        }
        if !crate::world::nutrition::Piece::of(item_id).can_it_be_eaten() {
            return false;
        }
        let Some(ref food_data) = item.food_data else {
            return false;
        };
        if food_data.is_spoiled() || food_data.is_harmful() {
            return false;
        }
        if food_data.preparation == crate::world::nutrition::PreparationState::Raw
            && crate::world::nutrition::Piece::is_it_flesh(item_id)
            && self.has_this_made_me_ill(Self::OFF_RAW_FLESH)
            && !self.state.is_starving()
        {
            return false;
        }
        true
    }

    pub fn find_best_food_to_eat(&self) -> Option<String> {
        let needed = self.nutrition.most_needed_nutrient();

        // What it is, how many whole days it has left, and what it is worth.
        let mut best_item: Option<(String, u32, f32)> = None;

        for (item_id, item) in &self.inventory.items {
            // Skip emptied stacks - an exhausted entry lingering in the
            // inventory would otherwise read as "still carrying food"
            if item.quantity == 0 {
                continue;
            }

            // A carcass is not a meal. Somebody has to take a knife to it
            // first, and until they do it is no more edible than the animal
            // was - see `Piece` and `what_i_could_cut_up`.
            if !crate::world::nutrition::Piece::of(item_id).can_it_be_eaten() {
                continue;
            }

            // Something that is not food at all is not a candidate, whatever
            // else is true of it. This used to be left to the `if let` below -
            // no nutrition data meant "skip" - which quietly also skipped
            // every untracked stack of real food, so a pack of traded grain
            // was invisible here and counted in `food_put_by` at the same
            // time.
            if !crate::world::nutrition::is_this_food(item_id) {
                continue;
            }

            let Some(ref food_data) = item.food_data else {
                // Food with nothing known about it beyond its name. It is
                // scored on what `eat_food_item` will actually credit it with,
                // so the search and the verb agree about what it is worth, and
                // at full freshness because there is nothing to say otherwise.
                let flat = crate::world::nutrition::what_an_untracked_mouthful_is_worth();
                let score = match needed {
                    crate::world::NutrientType::Energy => flat.energy,
                    crate::world::NutrientType::Protein => flat.protein,
                    crate::world::NutrientType::Micronutrients => flat.micronutrients,
                };

                // Nothing is known about how long it has, so it is not urgent
                // and not stale: it sits with the things that will keep.
                let days_left = u32::MAX;
                let better = match best_item.as_ref() {
                    None => true,
                    Some((_, best_days, best_score)) => {
                        days_left < *best_days
                            || (days_left == *best_days && score > *best_score)
                    }
                };
                if better {
                    best_item = Some((item_id.clone(), days_left, score));
                }
                continue;
            };

            {
                // Skip anything that would make the agent sick. Raw food turns
                // harmful before it counts as spoiled, so checking spoilage
                // alone leaves agents eating rot: ten health a bite, one bite
                // a tick, until the stack or the agent runs out.
                if food_data.is_spoiled() || food_data.is_harmful() {
                    continue;
                }

                // And skip raw flesh, if this one has been ill off raw flesh
                // before and is not desperate enough for that to stop
                // mattering. This is the whole point of the illness: a fire
                // used to be worth 2.7 times the nutrition and nothing else.
                //
                // Starving overrides it, as a strong enough survival drive
                // overrides everything: a man three days without food eats
                // what is in front of him and takes his chances.
                if food_data.preparation == crate::world::nutrition::PreparationState::Raw
                    && crate::world::nutrition::Piece::is_it_flesh(item_id)
                    && self.has_this_made_me_ill(Self::OFF_RAW_FLESH)
                    && !self.state.is_starving()
                {
                    continue;
                }

                let nutrition = food_data.effective_nutrition();

                // Score based on what we need most
                let score = match needed {
                    crate::world::NutrientType::Energy => nutrition.energy,
                    crate::world::NutrientType::Protein => nutrition.protein,
                    crate::world::NutrientType::Micronutrients => nutrition.micronutrients,
                };

                // Eat what will be lost first.
                //
                // The rule was `score * freshness * spoilage_multiplier`, and
                // `score` is `effective_nutrition`, which multiplies by
                // freshness already - so freshness went in **twice** and the
                // preference for the fresher of two identical things was
                // squared. A settlement ate this morning's berries and let
                // last week's rot beside them.
                //
                // What matters is not how fresh a thing is but how long it
                // has left, which is one number - see
                // `FoodData::how_long_this_has_left`. It carries what the old
                // expression was reaching for with both its terms: a dried
                // strip has hundreds of days in it and goes to the back, which
                // is what saves a winter store from being eaten in October;
                // and a raw thing three days off turning goes to the front,
                // ahead of the same thing picked this morning.
                //
                // Reckoned in whole days rather than ticks, so that what is
                // *worth* eating still decides between two things that will be
                // lost at about the same time. A strict ordering on the clock
                // alone has somebody eat a crumb with an hour left in front of
                // a good meal with a day, and a turn spent on a crumb is a
                // turn.
                let days_left = (food_data.how_long_this_has_left()
                    / crate::environment::seasons::TICKS_PER_DAY as f32)
                    .floor() as u32;

                let better = match best_item.as_ref() {
                    None => true,
                    Some((_, best_days, best_score)) => {
                        days_left < *best_days
                            || (days_left == *best_days && score > *best_score)
                    }
                };

                if better {
                    best_item = Some((item_id.clone(), days_left, score));
                }
            }
        }

        best_item.map(|(id, _, _)| id)
    }


    /// Drink water from inventory and satisfy thirst drive
    /// Returns true if water was consumed
    pub fn drink_water(&mut self, amount: f32) -> bool {
        let drunk = self.inventory.drink_water(amount);

        if drunk > 0.0 {
            // Satisfy thirst drive
            let thirst_reduction = drunk * 0.2; // Each liter reduces thirst by 0.2
            if let Some(thirst) = self.drives.get_mut(DriveType::Thirst) {
                thirst.decrease(thirst_reduction);
            }
            true
        } else {
            false
        }
    }

    /// Rest and restore energy, satisfy rest drive
    pub fn rest(&mut self, amount: f32) {
        self.state.energy = (self.state.energy + amount).min(100.0);

        // Satisfy rest drive
        let rest_reduction = amount * 0.01; // Resting reduces the rest drive
        if let Some(rest_drive) = self.drives.get_mut(DriveType::Rest) {
            rest_drive.decrease(rest_reduction);
        }
    }

    /// Sleep for one tick with quality factors affecting recovery
    /// Returns the fatigue decrease this tick
    pub fn sleep_tick(&mut self, current_tick: u32, sleep_quality_factors: &super::fatigue::SleepQualityFactors) -> f32 {
        let sleep_quality = sleep_quality_factors.calculate_quality();

        // Apply trait modifiers to recovery rate
        let recovery_modifier = self.sleep_recovery_modifier();
        let fatigue_decrease = self.fatigue.tick_sleeping_with_modifier(sleep_quality, current_tick, recovery_modifier);

        // Also restore energy based on fatigue recovery
        let energy_restored = fatigue_decrease * 30.0;
        self.state.energy = (self.state.energy + energy_restored).min(100.0);

        // Satisfy rest drive
        if let Some(rest_drive) = self.drives.get_mut(DriveType::Rest) {
            rest_drive.decrease(fatigue_decrease * 0.5);
        }

        fatigue_decrease
    }

    /// Get sleep recovery modifier based on traits
    /// Narcoleptic: 0.6 (40% less effective sleep)
    /// Normal: 1.0
    fn sleep_recovery_modifier(&self) -> f32 {
        if self.traits.has(crate::core::traits::Trait::Narcoleptic) {
            0.6 // Sleep is 40% less restorative
        } else {
            1.0
        }
    }

    /// Get sleep need threshold modifier based on traits
    /// SoundSleeper: ~0.75 (needs ~2 hours less sleep, which is ~25% less)
    /// Normal: 1.0
    fn sleep_need_modifier(&self) -> f32 {
        if self.traits.has(crate::core::traits::Trait::SoundSleeper) {
            0.75 // Needs 25% less sleep (~2 hours less of a typical 8-hour night)
        } else {
            1.0
        }
    }

    /// Wake up from sleep
    pub fn wake_up(&mut self, current_tick: u32) {
        self.fatigue.wake_up(current_tick);
    }

    /// Check if agent needs sleep based on fatigue (trait-aware)
    pub fn needs_sleep(&self) -> bool {
        self.fatigue.needs_sleep_with_modifier(self.sleep_need_modifier())
    }

    /// Check if agent desperately needs sleep (trait-aware)
    pub fn desperately_needs_sleep(&self) -> bool {
        self.fatigue.desperately_needs_sleep_with_modifier(self.sleep_need_modifier())
    }

    /// Check if agent should collapse from exhaustion
    pub fn should_collapse(&self) -> bool {
        self.fatigue.should_collapse()
    }



    /// Consume energy from activity
    pub fn consume_energy(&mut self, amount: f32) {
        self.state.energy = (self.state.energy - amount).max(0.0);

        // When energy is depleted, health starts decreasing
        if self.state.energy <= 0.0 {
            self.state.lose_health(0.05, "exhaustion");
        }
    }

    /// Take damage (wrapper for AgentState method)
    /// Defense bonus from nearby buildings reduces damage taken
    /// Masochist trait holders gain happiness from taking damage (up to 50% health)
    pub fn take_damage(&mut self, amount: f32) {
        use crate::core::traits::Trait;
        use super::EmotionSource;

        // Apply defense bonus from nearby buildings (higher bonus = less damage)
        // Defense bonus of 1.25 means 25% damage reduction (amount / 1.25)
        let defense_bonus = self.cached_defense_bonus;
        let reduced_amount = amount / defense_bonus;

        let health_before = self.state.health;
        self.state.take_damage(reduced_amount);

        // Masochist trait: happiness from taking damage while above 50% health
        if self.traits.has(Trait::Masochist) && health_before > 50.0 && self.state.health > 0.0 {
            // Scale happiness by damage taken, capped at 0.1 per hit
            let happiness_amount = (reduced_amount / 20.0).min(0.1);
            self.emotions.add_happiness(
                EmotionSource::Event("masochist_pleasure".to_string()),
                happiness_amount
            );
        }
    }

    /// Heal the agent (wrapper for AgentState method)
    /// Healing bonus from nearby medical buildings increases healing rate
    pub fn heal(&mut self, amount: f32) {
        // Apply healing bonus from nearby medical buildings
        let healing_bonus = self.cached_healing_bonus;
        let boosted_amount = amount * healing_bonus;
        self.state.heal(boosted_amount);
    }

    /// Regenerate health naturally over time
    /// Called during rest or when near medical facilities
    /// Base regeneration rate is 0.1 health per tick when resting
    pub fn regenerate_health(&mut self, is_resting: bool) {
        if self.state.health >= 100.0 {
            return; // Already at full health
        }

        // Base regeneration rate
        let base_rate = if is_resting { 0.1 } else { 0.02 };

        // Apply healing bonus from nearby medical buildings
        let healing_bonus = self.cached_healing_bonus;
        let regeneration = base_rate * healing_bonus;

        self.state.heal(regeneration);
    }

    /// Check if agent is dead
    pub fn is_dead(&self) -> bool {
        !self.state.is_alive || self.state.health <= 0.0
    }

    /// Age the agent by one tick
    pub fn age_tick(&mut self) {
        // Use age as an approximation of tick for the basic test API
        self.state.age_tick(self.state.age);
    }

    /// Update starvation counter (called each tick)
    /// Another turn goes by with nothing eaten.
    pub fn update_starvation(&mut self) {
        self.state
            .physiology
            .advance(physiology::MINUTES_PER_TURN, 5.0);
        self.state.ticks_without_food += physiology::MINUTES_PER_TURN;
    }

    /// Apply damage from starvation
    ///
    /// The clock is the body's, not a counter of turns: what does the harm is
    /// how far into the reserve this body has eaten, and an empty reserve is
    /// death whatever the calendar says. See `agents::physiology`.
    pub fn apply_starvation_damage(&mut self) {
        if self.state.physiology.starved() {
            self.state.lose_health(self.state.health, "starvation");
            return;
        }
        if self.state.physiology.is_wasting() {
            let days_into_the_reserve = (self.state.physiology.reserve_capacity
                - self.state.physiology.reserve)
                / physiology::UNITS_BURNED_IN_AN_ORDINARY_DAY;
            self.state.lose_health(days_into_the_reserve * 0.5, "starvation");
        }
    }

    /// How old this agent is in calendar years.
    pub fn age_in_years(&self) -> f32 {
        self.state.age as f32 / crate::environment::TICKS_PER_YEAR as f32
    }

    /// How long this agent will live, in calendar years, if nothing kills it.
    pub fn lifespan_in_years(&self) -> f32 {
        self.state.max_age as f32 / crate::environment::TICKS_PER_YEAR as f32
    }

    /// Update life stage based on age
    pub fn update_life_stage(&mut self) {
        self.state.life_stage = LifeStage::from_age(self.state.age);
    }

    // ===== Drive-Emotion Feedback System =====

    /// Check for stale storage knowledge and trigger curiosity
    /// Agents become curious about storage containers they haven't checked recently
    fn update_storage_curiosity(&mut self, current_tick: u32) {
        use super::EmotionSource;
        use crate::core::memory::SpatialMemoryType;

        // Threshold for "stale" knowledge (ticks since last seen)
        const STALE_THRESHOLD: u32 = 1000;

        // Check all storage memories
        let storage_memories = self.memory.recall_locations(SpatialMemoryType::Storage);

        for storage_memory in storage_memories {
            let time_since_seen = current_tick.saturating_sub(storage_memory.last_seen);

            // If knowledge is stale, generate curiosity
            if time_since_seen > STALE_THRESHOLD {
                // Base curiosity scales with staleness (0.1 to 0.4)
                let staleness_factor = ((time_since_seen - STALE_THRESHOLD) as f32 / 2000.0).min(1.0);
                let base_curiosity = 0.1 + (staleness_factor * 0.3);

                // Curious trait holders get 50% more curiosity
                let curiosity_bonus = if self.traits.has(crate::core::Trait::Curious) {
                    1.5
                } else {
                    1.0
                };

                let curiosity_amount = base_curiosity * curiosity_bonus;

                // Add curiosity about this specific storage location (with trait modifiers)
                self.emotions.add_curiosity_with_traits(
                    EmotionSource::Location(storage_memory.position),
                    curiosity_amount,
                    &self.traits
                );
            }
        }
    }


    /// Update emotions based on current drive states
    /// High unsatisfied drives trigger appropriate negative emotions
    /// Well-satisfied drives trigger happiness
    /// Uses set_* methods to replace values rather than accumulate
    pub fn update_emotions_from_drives(&mut self) {
        use super::EmotionSource;

        // Survival drives (Hunger, Thirst, Rest) → Fear (threat to survival)
        let survival_fear = self.calculate_survival_drive_emotion();
        self.emotions.set_fear(EmotionSource::Event("unmet survival needs".to_string()), survival_fear);

        // Social drives → Sadness (loneliness, isolation)
        let social_sadness = self.calculate_social_drive_emotion();
        self.emotions.set_sadness(EmotionSource::Event("social isolation".to_string()), social_sadness);

        // Other unfulfilled drives → General frustration (mild sadness)
        let general_frustration = self.calculate_general_drive_frustration();
        self.emotions.set_sadness(EmotionSource::Event("unfulfilled needs".to_string()), general_frustration * 0.5);

        // Well-satisfied drives → Happiness (contentment)
        let contentment = self.calculate_drive_satisfaction_happiness();
        self.emotions.set_happiness(EmotionSource::Event("needs satisfied".to_string()), contentment);
    }

    /// How long a need has to have gone unanswered before it starts to
    /// frighten somebody.
    ///
    /// A missed meal is not frightening. Days of missed meals are.
    const LONG_ENOUGH_TO_FRIGHTEN: f32 = 48.0;

    /// How far ahead dread looks.
    ///
    /// Three days. Not the same horizon as `A_LONG_WAY_OFF`, which is half a
    /// day and is the *urgency* clock - how hard a need should press on what
    /// an agent does this turn. These are two different questions and they
    /// were sharing one number: read at half a day, a man fifteen days without
    /// food and six days from dying of it came out **eight per cent
    /// frightened**, because six days is twelve times half a day.
    ///
    /// Urgency wants a tight horizon or everything is always an emergency -
    /// that is what the comment on `A_LONG_WAY_OFF` is about. Dread wants a
    /// long one, because being a week from starving is frightening and is
    /// meant to be: it is the specification's "I do not have enough food"
    /// raising fear, and fear is what sends somebody looking further afield
    /// than the ground they are standing on.
    const WHAT_DREAD_LOOKS_AHEAD: f32 = crate::environment::seasons::TICKS_PER_DAY as f32 * 3.0;

    /// Fear from a need that something has been preventing this agent from
    /// answering.
    ///
    /// The second half of the specification, and the half with no adversary in
    /// it. A worked-out field, a river the run has left, a winter: these
    /// prevent an agent satisfying its drives exactly as a wolf does, and the
    /// difference is that there is nothing to round on. Nothing to fight
    /// means fear rather than anger, every time.
    ///
    /// It is keyed on how long the need has actually been denied rather than
    /// on how high it stands, because those are different things. A drive can
    /// sit near its threshold all day while being met every time it asks; that
    /// is not being prevented from anything. `denied_ticks` counts only the
    /// ticks it asked and got nothing.
    fn calculate_survival_drive_emotion(&self) -> f32 {
        self.what_i_dread().0
    }

    /// The same reading, and *which need* it is about.
    ///
    /// The name matters as much as the number. "I do not have enough food"
    /// raising fear is only half of what the specification asks for; the
    /// other half is that the fear then helps motivate the drive that would
    /// answer it, and a bare magnitude cannot say which drive that is. So
    /// this returns both, and `DriveType::Safety` carries them: the fear
    /// drive rises on the dread, and when there is nothing in the field to
    /// run from, what it offers to do about it is whatever the dreaded need
    /// offers - which is going for food when the dread is hunger and going
    /// for water when it is thirst. Fear does not displace the need it is
    /// about; it pushes in the same direction.
    ///
    /// Only the needs with a death clock can be dreaded, and
    /// `ticks_before_this_kills_me` answers `None` for Safety itself, so the
    /// fear drive can never end up pointed at its own tail.
    pub fn what_i_dread(&self) -> (f32, Option<crate::core::DriveType>) {
        let mut worst: f32 = 0.0;
        let mut about = None;

        for drive_type in crate::core::DriveType::all() {
            let Some(drive) = self.drives.get(drive_type) else {
                continue;
            };

            if drive.denied_ticks() == 0 {
                continue;
            }

            // How badly this one going unanswered would end. A need that
            // cannot kill is a disappointment; one that can is a danger, and
            // the nearer it is the worse.
            let stakes = match self.state.ticks_before_this_kills_me(drive_type) {
                Some(left) => (Self::WHAT_DREAD_LOOKS_AHEAD / left.max(1.0)).clamp(0.0, 1.0),
                None => continue,
            };

            let how_long = (drive.denied_ticks() as f32 / Self::LONG_ENOUGH_TO_FRIGHTEN)
                .clamp(0.0, 1.0);

            let this_one = stakes * how_long;

            if this_one > worst {
                worst = this_one;
                about = Some(drive_type);
            }
        }

        (worst.min(1.0), about)
    }

    /// Calculate sadness from social drive deprivation
    fn calculate_social_drive_emotion(&self) -> f32 {
        if let Some(social) = self.drives.get(DriveType::Social) {
            if social.value > 0.6 {
                // Sadness scales with loneliness
                return (social.value - 0.6) / 0.4 * 0.6; // 0.0 to 0.6 (increased from 0.5)
            }
        }
        0.0
    }

    /// Calculate general frustration from other drives
    fn calculate_general_drive_frustration(&self) -> f32 {
        let mut total = 0.0;
        let mut count = 0;

        for drive in &self.drives.drives {
            // Skip survival and social drives (handled separately)
            if matches!(drive.drive_type, DriveType::Hunger | DriveType::Thirst | DriveType::Rest | DriveType::Social) {
                continue;
            }

            if drive.value > 0.7 {
                total += (drive.value - 0.7) / 0.3;
                count += 1;
            }
        }

        if count > 0 {
            // Return average frustration (compounds when multiple drives are high)
            total / count as f32 * 0.4
        } else {
            0.0
        }
    }

    /// Calculate happiness from satisfied drives
    /// Returns 0.0 to 1.0 based on how well drives are met
    fn calculate_drive_satisfaction_happiness(&self) -> f32 {
        let mut total = 0.0;
        let mut count = 0;

        // Check all drives - well-satisfied drives contribute to happiness
        for drive in &self.drives.drives {
            // Drives below 0.3 are well-satisfied
            if drive.value < 0.3 {
                // Inverse scaling: lower drive = more happiness
                let satisfaction = (0.3 - drive.value) / 0.3; // 0.0 to 1.0
                total += satisfaction;
                count += 1;
            }
        }

        if count > 0 {
            // Average satisfaction across all drives, capped at reasonable level
            (total / count as f32 * 0.5).min(0.7)
        } else {
            0.0
        }
    }

    /// Record that a source satisfied a drive
    /// Also triggers gratitude (happiness and bond improvement) if source is an agent
    pub fn record_drive_satisfaction(&mut self, drive_type: DriveType, source_id: Uuid, amount: f32, current_tick: u32) {
        self.satisfaction_tracker.record(drive_type, source_id, amount, current_tick);

        // Trigger gratitude response (happiness and bond improvement)
        self.process_gratitude(source_id, amount);
    }

    /// Process gratitude when receiving help from another agent
    /// Increases happiness and improves bond with the helper
    fn process_gratitude(&mut self, helper_id: Uuid, help_amount: f32) {
        use super::EmotionSource;

        // Happiness from receiving help (scaled by amount, with trait modifiers)
        let gratitude_happiness = (help_amount * 0.3).min(0.4);
        self.emotions.add_happiness_with_traits(EmotionSource::Agent(helper_id), gratitude_happiness, &self.traits);

        // Improve bond with helper
        if let Some(relationship) = self.relationships.get_relationship_mut(&helper_id) {
            // Bond improvement scaled by help amount (0.01 to 0.05)
            let bond_increase = (help_amount * 0.1).min(0.05);
            relationship.bond_strength = (relationship.bond_strength + bond_increase).min(1.0);
            relationship.time_together += 1;
        } else {
            // Create new positive relationship if none exists
            use super::emotions::{Relationship, RelationshipType};
            let mut new_relationship = Relationship::new(helper_id, RelationshipType::Acquaintance);
            // Start with slightly higher bond due to the help
            new_relationship.bond_strength = (0.2 + help_amount * 0.1).min(0.4);
            self.relationships.add_relationship(new_relationship);
        }
    }

    /// Process altruistic happiness when providing help to another agent
    /// Empathetic agents get extra happiness from helping
    /// Altruist trait holders also get extra happiness from helping
    pub fn process_helper_happiness(&mut self, recipient_id: Uuid, help_amount: f32) {
        use super::EmotionSource;
        use crate::core::traits::Trait;

        // Base happiness from helping (scaled by amount)
        let mut helper_happiness = (help_amount * 0.2).min(0.3);

        // Empathetic trait bonus: extra happiness from helping others
        if self.traits.has_trait(&Trait::Empathetic) {
            helper_happiness += 0.15; // Significant bonus for empathetic helpers
        }

        // Altruist trait bonus: additional happiness from helping
        if self.traits.has(Trait::Altruist) {
            helper_happiness += Trait::Altruist.happiness_gain() * 0.02;
        }

        // Caretaker trait bonus: happiness from helping sick/injured/elderly
        if self.traits.has(Trait::Caretaker) {
            helper_happiness += Trait::Caretaker.happiness_gain() * 0.02;
        }

        self.emotions.add_happiness_with_traits(EmotionSource::Agent(recipient_id), helper_happiness, &self.traits);
    }

    /// Apply religious happiness effects from nearby religious buildings
    /// Called by simulation tick with pre-calculated effects
    pub fn apply_religious_happiness(&mut self, happiness_modifier: f32, source_description: &str) {
        use super::EmotionSource;

        if happiness_modifier.abs() < 0.001 {
            return; // No effect to apply
        }

        if happiness_modifier > 0.0 {
            self.emotions.add_happiness(
                EmotionSource::Event(source_description.to_string()),
                happiness_modifier,
            );
        } else {
            // Negative effects reduce happiness (or could add sadness/discomfort)
            // For religious discomfort, we reduce happiness rather than add sadness
            self.emotions.happiness = (self.emotions.happiness + happiness_modifier).max(0.0);
        }
    }

    /// Get all sources that satisfy a specific drive
    pub fn get_drive_satisfaction_sources(&self, drive_type: DriveType) -> Vec<Uuid> {
        self.satisfaction_tracker.get_sources(drive_type)
    }

    /// Get the primary (most important) source for a drive
    pub fn get_primary_satisfaction_source(&self, drive_type: DriveType) -> Option<Uuid> {
        self.satisfaction_tracker.get_primary_source(drive_type)
    }

    /// Get importance of a source for a drive (0.0 to 1.0)
    pub fn get_source_importance(&self, drive_type: DriveType, source_id: Uuid) -> f32 {
        self.satisfaction_tracker.get_source_importance(drive_type, source_id)
    }

    /// Process the loss of a drive satisfaction source
    /// Triggers sadness based on source importance and current drive level
    pub fn process_drive_source_loss(&mut self, drive_type: DriveType, source_id: Uuid) {
        self.process_drive_source_loss_with_cause(drive_type, source_id, None);
    }

    /// Process drive source loss with known cause
    /// Can trigger anger at the cause in addition to sadness
    pub fn process_drive_source_loss_with_cause(
        &mut self,
        drive_type: DriveType,
        source_id: Uuid,
        cause: Option<super::EmotionSource>,
    ) {
        use super::EmotionSource;

        // Get source importance before removing
        let importance = self.get_source_importance(drive_type, source_id);

        if importance < 0.05 {
            // Insignificant source, minimal emotional impact
            return;
        }

        // Base sadness from losing the source
        let mut sadness = importance * 0.5; // 0.0 to 0.5

        // Amplify if drive is currently high (functional grief)
        if let Some(drive) = self.drives.get(drive_type) {
            if drive.value > 0.6 {
                // "I was already lonely, now I'm even more alone"
                let amplification = (drive.value - 0.6) / 0.4; // 0.0 to 1.0
                sadness += importance * amplification * 0.6; // Add up to 0.6 more (stronger amplification)
            }
        }

        // Add sadness (with trait modifiers)
        self.emotions.add_sadness_with_traits(EmotionSource::Agent(source_id), sadness, &self.traits);

        // If there's a cause and it's an agent, add anger
        if let Some(cause_source) = cause {
            match &cause_source {
                EmotionSource::Agent(_) | EmotionSource::Creature(_) => {
                    // Anger at whoever took away our satisfaction source
                    let anger = importance * 0.5; // 0.0 to 0.5 (stronger anger response)
                    self.emotions.add_anger_with_traits(cause_source, anger, &self.traits);
                }
                EmotionSource::Event(event) => {
                    // Natural causes - less anger, more sadness
                    if !event.contains("old age") && !event.contains("natural") {
                        // Accident or preventable - some anger
                        self.emotions.add_anger_with_traits(EmotionSource::Event(event.clone()), importance * 0.2, &self.traits);
                    }
                }
                _ => {}
            }
        }

        // Remove the source from tracking
        self.satisfaction_tracker.remove_source(source_id);
    }

    /// Get a functional explanation of grief
    /// Returns a message explaining why losing this agent matters
    pub fn get_grief_reason(&self, deceased_id: Uuid) -> String {
        let mut reasons = Vec::new();

        // Check all drives
        for drive_type in [DriveType::Social, DriveType::Reproduction, DriveType::Safety] {
            let importance = self.get_source_importance(drive_type, deceased_id);
            if importance > 0.3 {
                let drive_name = match drive_type {
                    DriveType::Social => "companionship",
                    DriveType::Reproduction => "partnership",
                    DriveType::Safety => "protection",
                    _ => "support",
                };
                reasons.push(format!("They provided {}", drive_name));
            }
        }

        // Check relationship
        if let Some(relationship) = self.relationships.get_relationship(&deceased_id) {
            if relationship.bond_strength >= 0.6 {
                // Loved one
                reasons.push(format!("I deeply cared about them (bond: {:.1})", relationship.bond_strength));
            } else if relationship.bond_strength > 0.3 {
                // Meaningful positive relationship
                reasons.push(format!("We had a bond"));
            }
        }

        if reasons.is_empty() {
            "I miss them".to_string()
        } else {
            format!("I'm grieving because: {}. I feel lonely and lost without them.", reasons.join(", "))
        }
    }

    // ========== Plan Execution System ==========

    /// Check if agent has an active plan
    pub fn has_active_plan(&self) -> bool {
        self.current_plan.as_ref().map(|p| !p.is_complete()).unwrap_or(false)
    }

    /// Check if agent should use their plan (vs. reactive behavior)
    ///
    /// Returns false if:
    /// - No active plan exists
    /// - Survival drives are active (must address immediate needs first)
    /// - Plan has been stuck on same step too long (timeout)
    pub fn should_execute_plan(&self) -> bool {
        // Must have an active plan
        if !self.has_active_plan() {
            return false;
        }

        // Survival drives override plan execution
        let hunger_active = self.drives.get(DriveType::Hunger)
            .map(|d| d.is_active())
            .unwrap_or(false);
        let thirst_active = self.drives.get(DriveType::Thirst)
            .map(|d| d.is_active())
            .unwrap_or(false);

        if hunger_active || thirst_active {
            return false;
        }

        // Check for plan step timeout (stuck too long on one step)
        if let Some(plan) = &self.current_plan {
            if let Some(step) = plan.current_step() {
                // Allow 3x estimated time before considering it stuck
                let timeout = step.estimated_ticks * 3;
                if self.plan_step_ticks > timeout && timeout > 0 {
                    return false; // Plan is stuck, should abandon
                }
            }
        }

        true
    }

    /// Check if the current plan is still relevant given updated world state
    ///
    /// Returns true if the plan should continue, false if it should be abandoned
    /// because the underlying goal is already satisfied (e.g., someone else restocked
    /// the storehouse while this agent was gathering resources).
    ///
    /// This enables agents to respond to new information and avoid wasted effort.
    pub fn is_plan_still_relevant(&self, world_state: &GoalWorldState) -> bool {
        // No plan = not relevant
        if !self.has_active_plan() {
            return false;
        }

        // Find the goal this plan was created for
        // Check if any active goal matches the plan description and is now satisfied
        if let Some(plan) = &self.current_plan {
            for goal in &self.goals.goals {
                if goal.completed {
                    continue;
                }

                // Check if this goal is now satisfied by world state
                if goal.is_satisfied(world_state) {
                    // Check if plan was for this goal (rough match by description)
                    let goal_matches = match &goal.external {
                        Some(crate::core::ExternalGoal::ContributeFoodToStorehouse(_)) => {
                            plan.goal_description.contains("food") ||
                            plan.goal_description.contains("Food")
                        }
                        Some(crate::core::ExternalGoal::ContributeMaterialsToStorehouse(_)) => {
                            plan.goal_description.contains("wood") ||
                            plan.goal_description.contains("material") ||
                            plan.goal_description.contains("Gather")
                        }
                        Some(crate::core::ExternalGoal::StockHouseFood(_)) => {
                            plan.goal_description.contains("food") ||
                            plan.goal_description.contains("stock")
                        }
                        Some(crate::core::ExternalGoal::GatherResource(resource, _)) => {
                            plan.goal_description.to_lowercase().contains(&resource.to_lowercase())
                        }
                        _ => false,
                    };

                    if goal_matches {
                        log::debug!(
                            "Plan '{}' no longer relevant - goal already satisfied by world state",
                            plan.goal_description
                        );
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Update plan relevance and abandon if goal is already satisfied
    ///
    /// Call this when the agent receives new information about the world
    /// (e.g., learning that the storehouse was restocked via gossip).
    pub fn update_plan_relevance(&mut self, world_state: &GoalWorldState) {
        if !self.is_plan_still_relevant(world_state) {
            if let Some(plan) = &self.current_plan {
                log::info!(
                    "Agent {} abandoning plan '{}' - goal already satisfied",
                    self.id, plan.goal_description
                );
            }
            self.abandon_plan();

            // Also mark the satisfied goal as complete
            for goal in &mut self.goals.goals {
                if !goal.completed && goal.is_satisfied(world_state) {
                    goal.completed = true;
                    goal.progress = 1.0;
                }
            }
        }
    }

    /// Get the next action from the current plan
    ///
    /// Converts the current PlanStep to an environment Action.
    /// Returns None if no plan, plan is complete, or step can't be converted.
    /// Note: Social actions may have nil UUIDs that need resolution at execution time.
    pub fn get_plan_action(&self) -> Option<Action> {
        let plan = self.current_plan.as_ref()?;
        let step = plan.current_step()?;

        self.convert_plan_step_to_action(step, &[])
    }


    /// Find the nearest agent from a list of candidates
    fn find_nearest_agent(&self, candidates: &[(uuid::Uuid, (i32, i32, i32))]) -> Option<uuid::Uuid> {
        let my_pos = self.state.position;
        candidates
            .iter()
            .filter(|(id, _)| *id != self.id) // Exclude self
            .min_by_key(|(_, pos)| {
                let dx = (pos.0 - my_pos.0).abs();
                let dy = (pos.1 - my_pos.1).abs();
                dx + dy // Manhattan distance
            })
            .map(|(id, _)| *id)
    }

    /// Convert a PlanStep's ActionType to an environment Action
    fn convert_plan_step_to_action(
        &self,
        step: &PlanStep,
        nearby_agents: &[(uuid::Uuid, (i32, i32, i32))],
    ) -> Option<Action> {
        match &step.action {
            PlanActionType::MoveTo { location } => {
                Some(Action::Move { target: *location })
            }
            PlanActionType::EquipItem { item: _ } => {
                // Equipment is handled internally, return None to skip
                // The step will be marked complete when equipment is applied
                None
            }
            PlanActionType::GatherResource { resource, amount: _ } => {
                Some(Action::Gather { resource_type: resource.clone() })
            }
            PlanActionType::CraftItem { item, count: _ } => {
                Some(Action::Craft { item_type: item.clone() })
            }
            PlanActionType::BuildStructure { structure } => {
                let pos = step.target_location.unwrap_or(self.state.position);
                Some(Action::Build { structure_type: structure.clone(), position: pos })
            }
            PlanActionType::Deposit { resource, amount } => {
                Some(Action::Store { item_type: resource.clone(), amount: *amount })
            }
            PlanActionType::Retrieve { resource, amount } => {
                Some(Action::Retrieve { item_type: resource.clone(), amount: *amount })
            }
            PlanActionType::Socialize { target_id } => {
                // Resolve nil UUID to nearest agent if possible
                let resolved_id = if target_id.is_nil() {
                    self.find_nearest_agent(nearby_agents).unwrap_or(*target_id)
                } else {
                    *target_id
                };
                Some(Action::Socialize { target_agent_id: resolved_id })
            }
            PlanActionType::Rest { duration } => {
                Some(Action::Sleep { duration: *duration })
            }
            PlanActionType::LearnSkill { skill: _ } => {
                // Learning is passive, skip this step
                None
            }
        }
    }

    /// Advance the plan after a successful action
    ///
    /// Records the outcome for learning, advances to next step,
    /// and clears step ticks counter.
    pub fn advance_plan_step(&mut self, success: bool, actual_ticks: u32) {
        if let Some(plan) = &self.current_plan {
            if let Some(step) = plan.current_step() {
                // Record outcome for learning
                let outcome = ActionOutcome {
                    action_type: step.action.clone(),
                    estimated_ticks: step.estimated_ticks,
                    actual_ticks,
                    success,
                    tool_used: step.required_tool.clone(),
                    tick: self.state.age, // Use age as proxy for current tick
                };
                self.planner.record_outcome(outcome);
            }
        }

        // Advance to next step
        if let Some(plan) = &mut self.current_plan {
            plan.advance_step();
            self.plan_step_ticks = 0;

            // Clear plan if complete
            if plan.is_complete() {
                self.current_plan = None;
            }
        }
    }

    /// Abandon the current plan (e.g., due to failure or priority change)
    pub fn abandon_plan(&mut self) {
        if self.current_plan.is_some() {
            log::debug!("Agent {} abandoning plan", self.id);
        }
        self.current_plan = None;
        self.plan_step_ticks = 0;
    }

    /// Increment the tick counter for current plan step
    pub fn tick_plan_step(&mut self) {
        if self.has_active_plan() {
            self.plan_step_ticks += 1;
        }
    }

    /// Create a plan for gathering resources
    pub fn create_gather_plan(
        &mut self,
        _resource: &str,
        amount: u32,
        resource_location: (i32, i32, i32),
        return_location: (i32, i32, i32),
        current_tick: u32,
    ) {
        // Get available tools from inventory
        let available_tools: Vec<String> = self.inventory.items.values()
            .filter(|item| {
                item.item_id.ends_with("_axe") ||
                item.item_id.ends_with("_pickaxe") ||
                item.item_id.ends_with("_hammer")
            })
            .map(|item| item.item_id.clone())
            .collect();

        // Generate the plan
        let plan = self.planner.plan_gather_wood(
            self.state.position,
            resource_location,
            return_location,
            &available_tools,
            amount,
        );

        // Check if plan is acceptable for this agent's traits
        let traits: Vec<_> = self.traits.get_traits().iter().copied().collect();
        if !plan.exceeds_complexity_limit(&traits) {
            log::debug!(
                "Agent {} created plan: {} ({} steps, est. {} ticks)",
                self.id, plan.goal_description, plan.steps.len(), plan.total_estimated_ticks
            );
            self.current_plan = Some(ActionPlan {
                created_at: current_tick,
                ..plan
            });
            self.plan_step_ticks = 0;
        }
    }

    /// Create a plan for the current highest-priority goal
    ///
    /// Returns true if a plan was created, false otherwise.
    pub fn create_plan_for_goal(
        &mut self,
        resource_location: (i32, i32, i32),
        return_location: (i32, i32, i32),
        current_tick: u32,
    ) -> bool {
        use crate::core::{ExternalGoal, InternalGoal, EmotionType};

        // Get the highest priority goal (internal or external)
        let goal = self.goals.highest_priority_goal();
        if goal.is_none() {
            return false;
        }

        let goal = goal.unwrap();

        // Get traits for complexity check
        let traits: Vec<_> = self.traits.get_traits().iter().copied().collect();

        // Handle internal goals first
        if let Some(internal_goal) = &goal.internal {
            let steps = match internal_goal {
                InternalGoal::IncreaseEmotion(emotion, _target) => {
                    match emotion {
                        EmotionType::Happiness => {
                            // Seek entertainment or social interaction
                            vec![
                                PlanStep {
                                    action: PlanActionType::Socialize { target_id: uuid::Uuid::nil() },
                                    estimated_ticks: 30,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: None,
                                    confidence: 0.5,
                                },
                            ]
                        }
                        EmotionType::Curiosity => {
                            // Explore or learn something new
                            vec![
                                PlanStep {
                                    action: PlanActionType::MoveTo { location: resource_location },
                                    estimated_ticks: 40,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: Some(resource_location),
                                    confidence: 0.7,
                                },
                            ]
                        }
                        _ => {
                            // Default: rest and reflect
                            vec![
                                PlanStep {
                                    action: PlanActionType::Rest { duration: 20 },
                                    estimated_ticks: 20,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: None,
                                    confidence: 0.8,
                                },
                            ]
                        }
                    }
                }
                InternalGoal::DecreaseEmotion(emotion, _target) => {
                    match emotion {
                        EmotionType::Fear => {
                            // Find shelter or safety
                            let shelter = self.find_nearest_shelter();
                            vec![
                                PlanStep {
                                    action: PlanActionType::MoveTo { location: shelter },
                                    estimated_ticks: 30,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: Some(shelter),
                                    confidence: 0.8,
                                },
                                PlanStep {
                                    action: PlanActionType::Rest { duration: 30 },
                                    estimated_ticks: 30,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: Some(shelter),
                                    confidence: 0.9,
                                },
                            ]
                        }
                        EmotionType::Anger => {
                            // Rest to calm down
                            vec![
                                PlanStep {
                                    action: PlanActionType::Rest { duration: 40 },
                                    estimated_ticks: 40,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: None,
                                    confidence: 0.7,
                                },
                            ]
                        }
                        EmotionType::Sadness => {
                            // Seek social support
                            vec![
                                PlanStep {
                                    action: PlanActionType::Socialize { target_id: uuid::Uuid::nil() },
                                    estimated_ticks: 40,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: None,
                                    confidence: 0.5,
                                },
                            ]
                        }
                        _ => {
                            vec![
                                PlanStep {
                                    action: PlanActionType::Rest { duration: 20 },
                                    estimated_ticks: 20,
                                    required_tool: None,
                                    required_resources: vec![],
                                    target_location: None,
                                    confidence: 0.8,
                                },
                            ]
                        }
                    }
                }
                InternalGoal::MaintainWellBeing(_threshold) => {
                    // Balance of rest and social activity
                    vec![
                        PlanStep {
                            action: PlanActionType::Rest { duration: 20 },
                            estimated_ticks: 20,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: None,
                            confidence: 0.85,
                        },
                        PlanStep {
                            action: PlanActionType::Socialize { target_id: uuid::Uuid::nil() },
                            estimated_ticks: 20,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: None,
                            confidence: 0.6,
                        },
                    ]
                }
                InternalGoal::ReduceStress => {
                    // Find shelter and rest
                    let shelter = self.find_nearest_shelter();
                    vec![
                        PlanStep {
                            action: PlanActionType::MoveTo { location: shelter },
                            estimated_ticks: 25,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: Some(shelter),
                            confidence: 0.85,
                        },
                        PlanStep {
                            action: PlanActionType::Rest { duration: 50 },
                            estimated_ticks: 50,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: Some(shelter),
                            confidence: 0.9,
                        },
                    ]
                }
                InternalGoal::SeekEntertainment => {
                    // Explore and socialize
                    vec![
                        PlanStep {
                            action: PlanActionType::MoveTo { location: resource_location },
                            estimated_ticks: 30,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: Some(resource_location),
                            confidence: 0.7,
                        },
                        PlanStep {
                            action: PlanActionType::Socialize { target_id: uuid::Uuid::nil() },
                            estimated_ticks: 30,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: None,
                            confidence: 0.5,
                        },
                    ]
                }
            };

            let plan = ActionPlan::new(
                format!("{:?}", internal_goal),
                steps,
                current_tick,
                "fulfilling emotional need".to_string(),
            );

            if !plan.exceeds_complexity_limit(&traits) {
                self.current_plan = Some(plan);
                self.plan_step_ticks = 0;
                return true;
            }
            return false;
        }

        // Handle external goals
        let external_goal = match &goal.external {
            Some(ext) => ext.clone(),
            None => return false,
        };

        // Generate plan based on external goal type
        match &external_goal {
            ExternalGoal::GatherResource(resource, amount) => {
                self.create_gather_plan(
                    resource,
                    *amount,
                    resource_location,
                    return_location,
                    current_tick,
                );
                self.has_active_plan()
            }
            ExternalGoal::StockHouseFood(amount) => {
                // Food gathering plan
                self.create_gather_plan(
                    "food",
                    *amount,
                    resource_location,
                    return_location,
                    current_tick,
                );
                self.has_active_plan()
            }
            ExternalGoal::ContributeFoodToStorehouse(amount) => {
                // If we have food, create deposit plan
                let food_count = self.inventory.get_item("food")
                    .map(|i| i.quantity)
                    .unwrap_or(0);

                if food_count >= *amount {
                    // Create simple deposit plan
                    let steps = vec![
                        PlanStep {
                            action: PlanActionType::MoveTo { location: return_location },
                            estimated_ticks: 30,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: Some(return_location),
                            confidence: 0.95,
                        },
                        PlanStep {
                            action: PlanActionType::Deposit {
                                resource: "food".to_string(),
                                amount: *amount,
                            },
                            estimated_ticks: 5,
                            required_tool: None,
                            required_resources: vec![("food".to_string(), *amount)],
                            target_location: Some(return_location),
                            confidence: 0.95,
                        },
                    ];

                    let plan = ActionPlan::new(
                        "Contribute food to storehouse".to_string(),
                        steps,
                        current_tick,
                        "carrying food".to_string(),
                    );

                    if !plan.exceeds_complexity_limit(&traits) {
                        self.current_plan = Some(plan);
                        self.plan_step_ticks = 0;
                        return true;
                    }
                } else {
                    // Need to gather food first
                    self.create_gather_plan(
                        "food",
                        *amount,
                        resource_location,
                        return_location,
                        current_tick,
                    );
                }
                self.has_active_plan()
            }
            ExternalGoal::ContributeMaterialsToStorehouse(amount) => {
                self.create_gather_plan(
                    "wood",
                    *amount,
                    resource_location,
                    return_location,
                    current_tick,
                );
                self.has_active_plan()
            }
            ExternalGoal::CraftItem(item) => {
                // Simple craft plan
                let steps = vec![
                    PlanStep {
                        action: PlanActionType::CraftItem {
                            item: item.clone(),
                            count: 1,
                        },
                        estimated_ticks: 30,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: None,
                        confidence: 0.7,
                    },
                ];

                let plan = ActionPlan::new(
                    format!("Craft {}", item),
                    steps,
                    current_tick,
                    "crafting".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::BuildStructure(structure) => {
                // Simple build plan
                let steps = vec![
                    PlanStep {
                        action: PlanActionType::BuildStructure {
                            structure: structure.clone(),
                        },
                        estimated_ticks: 100,
                        required_tool: Some("hammer".to_string()),
                        required_resources: vec![("wood".to_string(), 10)],
                        target_location: Some(self.state.position),
                        confidence: 0.6,
                    },
                ];

                let plan = ActionPlan::new(
                    format!("Build {}", structure),
                    steps,
                    current_tick,
                    "constructing".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::OwnHouse => {
                // Build a small house for the agent
                let steps = vec![
                    PlanStep {
                        action: PlanActionType::GatherResource {
                            resource: "wood".to_string(),
                            amount: 20,
                        },
                        estimated_ticks: 60,
                        required_tool: Some("axe".to_string()),
                        required_resources: vec![],
                        target_location: Some(resource_location),
                        confidence: 0.8,
                    },
                    PlanStep {
                        action: PlanActionType::BuildStructure {
                            structure: "small_house".to_string(),
                        },
                        estimated_ticks: 150,
                        required_tool: Some("hammer".to_string()),
                        required_resources: vec![("wood".to_string(), 20)],
                        target_location: Some(self.state.position),
                        confidence: 0.6,
                    },
                ];

                let plan = ActionPlan::new(
                    "Build own house".to_string(),
                    steps,
                    current_tick,
                    "constructing home".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::ObtainProtection => {
                // Craft basic protection equipment
                let steps = vec![
                    PlanStep {
                        action: PlanActionType::GatherResource {
                            resource: "leather".to_string(),
                            amount: 5,
                        },
                        estimated_ticks: 40,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: Some(resource_location),
                        confidence: 0.7,
                    },
                    PlanStep {
                        action: PlanActionType::CraftItem {
                            item: "leather_armor".to_string(),
                            count: 1,
                        },
                        estimated_ticks: 50,
                        required_tool: None,
                        required_resources: vec![("leather".to_string(), 5)],
                        target_location: None,
                        confidence: 0.6,
                    },
                    PlanStep {
                        action: PlanActionType::EquipItem {
                            item: "leather_armor".to_string(),
                        },
                        estimated_ticks: 2,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: None,
                        confidence: 0.95,
                    },
                ];

                let plan = ActionPlan::new(
                    "Obtain protection".to_string(),
                    steps,
                    current_tick,
                    "crafting armor".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::EnsureToolsAvailable(target_count) => {
                // Craft tools and deposit them
                let steps = vec![
                    PlanStep {
                        action: PlanActionType::GatherResource {
                            resource: "wood".to_string(),
                            amount: 5,
                        },
                        estimated_ticks: 30,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: Some(resource_location),
                        confidence: 0.85,
                    },
                    PlanStep {
                        action: PlanActionType::GatherResource {
                            resource: "stone".to_string(),
                            amount: 3,
                        },
                        estimated_ticks: 25,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: Some(resource_location),
                        confidence: 0.85,
                    },
                    PlanStep {
                        action: PlanActionType::CraftItem {
                            item: "stone_axe".to_string(),
                            count: 1,
                        },
                        estimated_ticks: 20,
                        required_tool: None,
                        required_resources: vec![
                            ("wood".to_string(), 2),
                            ("stone".to_string(), 1),
                        ],
                        target_location: None,
                        confidence: 0.7,
                    },
                    PlanStep {
                        action: PlanActionType::MoveTo { location: return_location },
                        estimated_ticks: 30,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: Some(return_location),
                        confidence: 0.95,
                    },
                    PlanStep {
                        action: PlanActionType::Deposit {
                            resource: "stone_axe".to_string(),
                            amount: *target_count,
                        },
                        estimated_ticks: 5,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: Some(return_location),
                        confidence: 0.9,
                    },
                ];

                let plan = ActionPlan::new(
                    "Ensure tools available".to_string(),
                    steps,
                    current_tick,
                    "crafting tools".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::LearnSkill(skill_name) => {
                // Create a learning plan with practice time
                let steps = vec![
                    PlanStep {
                        action: PlanActionType::LearnSkill {
                            skill: skill_name.clone(),
                        },
                        estimated_ticks: 100,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: None,
                        confidence: 0.5, // Learning outcomes are uncertain
                    },
                ];

                let plan = ActionPlan::new(
                    format!("Learn {}", skill_name),
                    steps,
                    current_tick,
                    "practicing".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::FormRelationship(relationship_type) => {
                // Find nearby agents and socialize
                // Use a placeholder target ID - actual target selection happens during execution
                let target_id = uuid::Uuid::nil(); // Placeholder, will be resolved at execution time

                let steps = vec![
                    PlanStep {
                        action: PlanActionType::Socialize { target_id },
                        estimated_ticks: 30,
                        required_tool: None,
                        required_resources: vec![],
                        target_location: None,
                        confidence: 0.4, // Relationship outcomes are uncertain
                    },
                ];

                let plan = ActionPlan::new(
                    format!("Form {} relationship", relationship_type),
                    steps,
                    current_tick,
                    "socializing".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
            ExternalGoal::CompleteJob(job_name) => {
                // Generic job completion - maps to appropriate actions
                // The job name determines what resources to gather or items to craft
                let steps = match job_name.as_str() {
                    "woodcutting" => vec![
                        PlanStep {
                            action: PlanActionType::GatherResource {
                                resource: "wood".to_string(),
                                amount: 10,
                            },
                            estimated_ticks: 60,
                            required_tool: Some("axe".to_string()),
                            required_resources: vec![],
                            target_location: Some(resource_location),
                            confidence: 0.8,
                        },
                    ],
                    "mining" => vec![
                        PlanStep {
                            action: PlanActionType::GatherResource {
                                resource: "stone".to_string(),
                                amount: 10,
                            },
                            estimated_ticks: 80,
                            required_tool: Some("pickaxe".to_string()),
                            required_resources: vec![],
                            target_location: Some(resource_location),
                            confidence: 0.7,
                        },
                    ],
                    "hunting" => vec![
                        PlanStep {
                            action: PlanActionType::GatherResource {
                                resource: "meat".to_string(),
                                amount: 5,
                            },
                            estimated_ticks: 90,
                            required_tool: Some("bow".to_string()),
                            required_resources: vec![],
                            target_location: Some(resource_location),
                            confidence: 0.5,
                        },
                    ],
                    "crafting" => vec![
                        PlanStep {
                            action: PlanActionType::CraftItem {
                                item: "tool".to_string(),
                                count: 1,
                            },
                            estimated_ticks: 40,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: None,
                            confidence: 0.7,
                        },
                    ],
                    _ => vec![
                        // Generic work task - rest and observe
                        PlanStep {
                            action: PlanActionType::Rest { duration: 10 },
                            estimated_ticks: 10,
                            required_tool: None,
                            required_resources: vec![],
                            target_location: None,
                            confidence: 0.9,
                        },
                    ],
                };

                let plan = ActionPlan::new(
                    format!("Complete {} job", job_name),
                    steps,
                    current_tick,
                    "working".to_string(),
                );

                if !plan.exceeds_complexity_limit(&traits) {
                    self.current_plan = Some(plan);
                    self.plan_step_ticks = 0;
                    return true;
                }
                false
            }
        }
    }

    /// Get current plan progress (0.0 to 1.0), or None if no plan
    pub fn plan_progress(&self) -> Option<f32> {
        self.current_plan.as_ref().map(|p| p.progress())
    }

    /// Get description of current plan step, if any
    pub fn current_plan_step_description(&self) -> Option<String> {
        self.current_plan.as_ref().and_then(|plan| {
            plan.current_step().map(|step| {
                match &step.action {
                    PlanActionType::MoveTo { location } => {
                        format!("Moving to {:?}", location)
                    }
                    PlanActionType::GatherResource { resource, amount } => {
                        format!("Gathering {} {}", amount, resource)
                    }
                    PlanActionType::CraftItem { item, count } => {
                        format!("Crafting {} x{}", item, count)
                    }
                    PlanActionType::BuildStructure { structure } => {
                        format!("Building {}", structure)
                    }
                    PlanActionType::Deposit { resource, amount } => {
                        format!("Depositing {} {}", amount, resource)
                    }
                    PlanActionType::Retrieve { resource, amount } => {
                        format!("Retrieving {} {}", amount, resource)
                    }
                    PlanActionType::EquipItem { item } => {
                        format!("Equipping {}", item)
                    }
                    PlanActionType::Socialize { target_id } => {
                        format!("Socializing with {}", target_id)
                    }
                    PlanActionType::Rest { duration } => {
                        format!("Resting for {} ticks", duration)
                    }
                    PlanActionType::LearnSkill { skill } => {
                        format!("Learning {}", skill)
                    }
                }
            })
        })
    }

    // ===== Trust and Lie Detection System =====

    /// Get lie detection chance based on agent's traits
    /// Returns a multiplier (1.0 = normal, higher = better detection)
    pub fn get_lie_detection_bonus(&self) -> f32 {
        use crate::core::traits::Trait;

        let mut bonus: f32 = 1.0;

        // Suspicious trait: +50% lie detection
        if self.traits.has(Trait::Suspicious) {
            bonus += 0.5;
        }

        // Paranoid trait: +80% lie detection (assumes malice)
        if self.traits.has(Trait::Paranoid) {
            bonus += 0.8;
        }

        // Honest agents are better at spotting dishonesty (they know what truth looks like)
        if self.traits.has(Trait::Honest) {
            bonus += 0.3;
        }

        // Skeptic trait: +40% lie detection
        if self.traits.has(Trait::Skeptic) {
            bonus += 0.4;
        }

        // Trusting trait: -40% lie detection (easily fooled)
        if self.traits.has(Trait::Trusting) {
            bonus -= 0.4;
        }

        bonus.max(0.1) // Minimum 10% detection chance
    }

    /// Verify a resource location claim against what this agent actually knows
    /// Returns Some(true) if verified correct, Some(false) if verified wrong, None if unverifiable
    pub fn verify_resource_claim(
        &self,
        claimed_resource: &str,
        claimed_location: (i32, i32, i32),
    ) -> Option<bool> {
        use crate::world::Position;

        let claimed_pos = Position::new(claimed_location.0, claimed_location.1);

        // Check if we've explored this location
        if !self.exploration_knowledge.is_explored(&claimed_pos) {
            return None; // Can't verify - haven't been there
        }

        // Check what we actually know about this location
        if let Some(actual_resource) = self.exploration_knowledge.known_resources.get(&claimed_pos) {
            // We know what's at this location
            let actual_name = format!("{:?}", actual_resource).to_lowercase();
            let claimed_lower = claimed_resource.to_lowercase();

            // Check if the claimed resource matches what we found
            if actual_name.contains(&claimed_lower) || claimed_lower.contains(&actual_name) {
                return Some(true); // Verified correct
            } else {
                return Some(false); // Verified wrong - different resource type
            }
        }

        // We've been there but didn't find any resource
        // If they claimed a resource exists there, they're likely wrong
        Some(false)
    }

    /// Attempt to detect lies in received information based on personal knowledge
    /// Returns a list of (info_id, source_id, was_lie) for detected lies
    pub fn detect_lies_in_knowledge(&self, _current_tick: u32) -> Vec<(uuid::Uuid, uuid::Uuid, bool)> {
        use super::gossip::InformationType;

        let mut detections = Vec::new();
        let detection_bonus = self.get_lie_detection_bonus();

        // Check each piece of information we've received
        for (info_id, info) in &self.knowledge.known_information {
            // Skip if this is our own information
            if info.original_source == self.id {
                continue;
            }

            // Find the belief for this info to get the source
            let source = self.knowledge.beliefs
                .iter()
                .find(|b| b.info_id == *info_id)
                .map(|b| b.source);

            if let Some(source_id) = source {
                // Try to verify different types of information
                match &info.info_type {
                    InformationType::ResourceLocation { resource, location } => {
                        if let Some(is_correct) = self.verify_resource_claim(resource, *location) {
                            // Apply detection bonus - higher bonus means more likely to catch lies
                            let base_detection_chance = if is_correct { 0.0 } else { 0.5 };
                            let effective_chance = base_detection_chance * detection_bonus;

                            // Simple probability check
                            use rand::Rng;
                            let roll: f32 = crate::core::dice::roll().gen();

                            if roll < effective_chance || !is_correct {
                                // We detected this information as incorrect
                                if !info.ground_truth || !is_correct {
                                    detections.push((*info_id, source_id, true)); // It was a lie
                                } else {
                                    detections.push((*info_id, source_id, false)); // It was true
                                }
                            }
                        }
                    }
                    // Could add more verification types here (accusations, observations, etc.)
                    _ => {}
                }
            }
        }

        detections
    }

    /// Process lie detection and update trust/relationships accordingly
    /// Call this periodically (e.g., every 100 ticks) to verify information
    pub fn process_information_verification(&mut self, current_tick: u32) {
        use super::EmotionSource;

        let detections = self.detect_lies_in_knowledge(current_tick);

        for (info_id, source_id, was_lie) in detections {
            // Calculate info age for trust update (used by SocialNetwork methods)
            let _info_age = if let Some(info) = self.knowledge.known_information.get(&info_id) {
                current_tick.saturating_sub(info.timestamp as u32)
            } else {
                1000 // Default to old if not found
            };

            // Update knowledge base trust
            self.knowledge.verify_information(&info_id, !was_lie);

            // Update relationship
            if was_lie {
                // What it was about, so that a lie which sent this agent
                // somewhere it needed to go costs more than one which did not
                let about = self
                    .knowledge
                    .known_information
                    .get(&info_id)
                    .and_then(|info| match &info.info_type {
                        super::gossip::InformationType::ResourceLocation { resource, .. } => {
                            Some(resource.clone())
                        }
                        _ => None,
                    });

                self.found_out_i_was_lied_to(
                    source_id,
                    about.as_deref().unwrap_or(""),
                    current_tick,
                );
            } else {
                // Truth verified - strengthen trust and relationship
                self.found_out_they_were_right(source_id);
                let rel = self.relationships.get_or_create_relationship(source_id, current_tick);
                rel.strengthen(0.05); // Small positive reinforcement
                rel.settle_what_we_are();

                // Small happiness from receiving accurate information
                self.emotions.add_happiness(
                    EmotionSource::Agent(source_id),
                    0.02
                );
            }
        }
    }







    /// The most places an agent keeps in its head at once.
    ///
    /// Nothing bounded this before. It did not matter while the only way to
    /// learn a place was to walk past it, and it matters a great deal now that
    /// news travels: a settlement that talks carries the whole map in every
    /// head, which is neither true to life nor cheap.
    pub const WHAT_A_MAN_CAN_HOLD_IN_MIND: usize = 96;

    /// Forget what does not matter.
    ///
    /// "Information should ... be retained longer if an agent has an interest
    /// in the topic. A topic an agent cares little for should be quickly
    /// forgotten."
    ///
    /// What is worth keeping is what answers something this agent actually
    /// wants - `how_hard_it_presses` is the same reckoning the drive hierarchy
    /// ranks needs by - and, at the same interest, what was learned most
    /// recently. So a thirsty man holds on to every waterhole he has heard of
    /// and lets the flax go, and a man who wants for nothing keeps whatever he
    /// heard last.
    ///
    /// Hearsay is let go before first-hand knowledge of equal interest, on the
    /// principle that a man is surer of what he saw.
    /// What counts as a place worth carrying about in your head, by how much
    /// is standing on it.
    ///
    /// Anything at or above this is remembered as richly as anything else can
    /// be; the scale only has to separate a seam from the last of one.
    const A_PLACE_WORTH_REMEMBERING: u32 = 12;

    pub fn forget_what_does_not_matter(&mut self, current_tick: u32) {
        if self.exploration_knowledge.known_resources.len()
            <= Self::WHAT_A_MAN_CAN_HOLD_IN_MIND
        {
            return;
        }

        let mut worth: Vec<(crate::world::Position, f32)> = self
            .exploration_knowledge
            .known_resources
            .iter()
            .map(|(where_it_is, what_it_is)| {
                let subject = format!("{:?}", what_it_is).to_lowercase();

                // How much this agent wants the thing at all
                let wanted = Self::what_this_answers(&subject)
                    .map(|need| self.how_hard_it_presses(need))
                    .unwrap_or(0.0);

                // And how fresh the knowledge of it is, which decides between
                // two things wanted equally
                let learned_on = self
                    .exploration_knowledge
                    .when_i_saw_it(where_it_is)
                    .unwrap_or(0);
                let freshness = 1.0
                    - (current_tick.saturating_sub(learned_on) as f32
                        / crate::environment::seasons::TICKS_PER_YEAR as f32)
                        .clamp(0.0, 1.0);

                let heard_not_seen = self
                    .exploration_knowledge
                    .who_told_me
                    .contains_key(where_it_is);

                // And how much was standing there, which decides between two
                // things wanted equally and known equally well. A head only
                // holds so many places; the last handful of a worked-out seam
                // is the one to let go of, and a man who has been told it is
                // the last handful can now know that about it.
                let how_rich = self
                    .exploration_knowledge
                    .how_much_was_there_then(where_it_is)
                    .map(|how_much| {
                        (how_much as f32 / Self::A_PLACE_WORTH_REMEMBERING as f32).clamp(0.0, 1.0)
                    })
                    // Nothing said about it either way is not the same as
                    // being told it is bare
                    .unwrap_or(0.5);

                let keeping = wanted * 4.0 + freshness + how_rich
                    - if heard_not_seen { 0.5 } else { 0.0 };
                (*where_it_is, keeping)
            })
            .collect();

        worth.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let to_forget = worth.len() - Self::WHAT_A_MAN_CAN_HOLD_IN_MIND;
        for (where_it_is, _) in worth.into_iter().take(to_forget) {
            self.exploration_knowledge.known_resources.remove(&where_it_is);
            self.exploration_knowledge.who_told_me.remove(&where_it_is);
            self.exploration_knowledge
                .resource_discovery_ticks
                .remove(&where_it_is);
            self.exploration_knowledge
                .last_seen_ticks
                .remove(&where_it_is);
            self.exploration_knowledge
                .how_much_was_there
                .remove(&where_it_is);
        }
    }

    /// Which need a thing in the ground answers, if any.
    ///
    /// A lie about where the water is and a lie about where the pretty stones
    /// are do not weigh the same, and this is what tells them apart.
    pub fn what_this_answers(subject: &str) -> Option<DriveType> {
        let subject = subject.to_lowercase();
        if subject.contains("water") {
            Some(DriveType::Thirst)
        } else if subject.contains("food")
            || subject.contains("fish")
            || subject.contains("meat")
            || subject.contains("berry")
            || subject.contains("berries")
            || subject.contains("grain")
        {
            Some(DriveType::Hunger)
        } else if subject.contains("wood") || subject.contains("stone") {
            Some(DriveType::Shelter)
        } else if subject.contains("flax")
            || subject.contains("cotton")
            || subject.contains("wool")
            || subject.contains("hide")
        {
            Some(DriveType::Shelter)
        } else if subject.contains("iron") || subject.contains("herb") {
            Some(DriveType::Industry)
        } else {
            None
        }
    }

    /// What being lied to about this costs the liar.
    ///
    /// "If an agent lies to another agent about something they care about or
    /// something which has a detrimental impact on their ability to satisfy a
    /// drive, the amount of anger should be higher."
    ///
    /// Before this it was a flat 0.2 whatever the lie was about, so sending a
    /// thirsty man to a dry riverbed and telling him the wrong thing about a
    /// pile of rocks cost exactly the same.
    ///
    /// Three things decide it. What was lied about, weighed by how hard that
    /// need is pressing on *this* agent right now - which is the same
    /// `how_hard_it_presses` the drive hierarchy ranks needs by, so a lie
    /// about food to a man who is not hungry is a small thing and the same
    /// lie to one who is starving is not. What the two of them were to each
    /// other, because being deceived by somebody you trusted is worse than
    /// being deceived by somebody you did not. And what sort of person is
    /// doing the resenting.
    pub fn what_a_lie_about_this_costs(&self, subject: Option<&str>, liar: uuid::Uuid) -> f32 {
        use crate::core::traits::Trait;

        /// A lie is a lie even when it is about nothing that matters
        const ANY_LIE_AT_ALL: f32 = 0.15;

        /// And this much again on top when it touches something vital
        const AND_THIS_MUCH_FOR_A_VITAL_ONE: f32 = 0.55;

        let about_something_i_need = subject
            .and_then(Self::what_this_answers)
            .map(|need| self.how_hard_it_presses(need))
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);

        let mut cost = ANY_LIE_AT_ALL + AND_THIS_MUCH_FOR_A_VITAL_ONE * about_something_i_need;

        // Being deceived by a friend is worse than being deceived by a
        // stranger, and being deceived by somebody you already had no time for
        // is barely news
        if let Some(bond) = self.relationships.get_relationship(&liar) {
            if bond.bond_strength >= 0.5 {
                cost *= 1.5;
            } else if bond.bond_strength < 0.0 {
                cost *= 0.7;
            }
        }

        if self.traits.has(Trait::Vengeful) {
            cost *= 1.5;
        }
        if self.traits.has(Trait::Forgiving) {
            cost *= 0.5;
        }
        if self.traits.has(Trait::Trusting) {
            // It hurts more when you did not see it coming
            cost *= 1.25;
        }
        if self.traits.has(Trait::Paranoid) || self.traits.has(Trait::Suspicious) {
            // And less when you always half expected it
            cost *= 0.75;
        }

        cost.clamp(0.0, 1.0)
    }


    /// What happens when an agent finds out it was lied to.
    ///
    /// One place, because it is reached from two: walking to a place somebody
    /// named and finding bare ground, and the periodic sweep of remembered
    /// claims. The first is where nearly all of it happens - a lie is found
    /// out standing on the spot, not by review.
    pub fn found_out_i_was_lied_to(
        &mut self,
        liar: uuid::Uuid,
        about: &str,
        current_tick: u32,
    ) {
        use super::EmotionSource;

        let cost = self.what_a_lie_about_this_costs(Some(about), liar);

        // Anger at whoever it was, weighted by what it was about. A grudge
        // then weighs on the bond every tick it is held - see
        // `Relationship::let_it_tell` - but finding out is its own moment and
        // lands on the bond directly as well.
        self.emotions.add_anger(EmotionSource::Agent(liar), cost);

        let bond = self
            .relationships
            .get_or_create_relationship(liar, current_tick);
        bond.weaken(cost);
        bond.settle_what_we_are();

        // And it goes on the record, which is what `how_far_i_trust` reads
        // when this agent is next offered something by the same man
        self.knowledge
            .trust_ratings
            .entry(liar)
            .or_insert_with(|| super::gossip::TrustRating::new(self.id, liar))
            .update_on_verification(false);
    }

    /// Somebody took something of this agent's.
    ///
    /// The same shape as finding out you were lied to, and worse: a lie costs
    /// you a belief and a theft costs you the thing. It lands on the bond, it
    /// lands on the anger, and it goes on the record that decides whose word
    /// this agent will take next time.
    pub fn they_took_something_of_mine(
        &mut self,
        thief: uuid::Uuid,
        what: &str,
        how_many: u32,
        current_tick: u32,
        how_strong_they_are: f32,
    ) {
        use super::EmotionSource;

        // What it costs depends on how much was taken and how much this agent
        // had. Two sticks off a man with forty is a nuisance; two off a man
        // with three is the difference between a spear and no spear.
        let had = self.how_many_i_have(what).max(how_many);
        let share = (how_many as f32 / had as f32).clamp(0.0, 1.0);
        let cost = Self::WHAT_BEING_ROBBED_COSTS_THEM * (0.4 + 0.6 * share);

        // Anger at somebody who can be faced, and fear of somebody who cannot.
        //
        // This was anger every time, whoever took it - so a man robbed by
        // somebody twice his size came away resolved to do something about it,
        // which is not what being robbed by somebody twice your size feels
        // like. It is the same appraisal the wolves get, pointed at a person:
        // see `ThreatAssessment` and `DriveType::Aggression`. What was taken
        // decides how much, and who took it decides which.
        let judged = super::ThreatAssessment::assess(
            self.own_strength(),
            how_strong_they_are,
            EmotionSource::Agent(thief),
        );

        if judged.can_overcome {
            self.emotions.add_anger(EmotionSource::Agent(thief), cost);
        } else {
            self.emotions.add_fear(EmotionSource::Agent(thief), cost);
        }

        let bond = self
            .relationships
            .get_or_create_relationship(thief, current_tick);
        bond.weaken(cost);
        bond.settle_what_we_are();

        // On the record - but in the thief's column, not the liar's. Being
        // honest is about what a man says, and this is about what he takes.
        self.knowledge
            .trust_ratings
            .entry(thief)
            .or_insert_with(|| super::gossip::TrustRating::new(self.id, thief))
            .update_on_theft();
    }

    /// What being robbed costs the man who was robbed, at its worst.
    ///
    /// More than a lie. A lie takes a belief off you and a theft takes the
    /// thing.
    const WHAT_BEING_ROBBED_COSTS_THEM: f32 = 0.55;

    /// How readily this agent would help itself to somebody else's things.
    ///
    /// Nobody steals for the sake of it. What decides it is what sort of
    /// person this is and how badly the want is pressing: an honest man with a
    /// full belly does not, and a starving one might.
    pub fn how_readily_i_would_take_it(&self) -> f32 {
        use crate::core::traits::Trait;

        let mut how_readily: f32 = 0.12;

        if self.traits.has(Trait::Honest) {
            how_readily -= 0.10;
        }
        if self.traits.has(Trait::Greedy) {
            how_readily += 0.12;
        }

        // And need, which is what actually does it
        if self.state.is_starving() || self.nutrition.is_starving() {
            how_readily += 0.35;
        }

        how_readily.clamp(0.0, 1.0)
    }

    /// Which drive a thing answers, as far as this agent is concerned.
    ///
    /// Rough, and it has to be: the model has no table saying what each of a
    /// hundred item names is *for*. What it does have is food that declares
    /// itself food, vessels that hold water, and a chain that says which raw
    /// things it is forever short of. Everything else is something to have
    /// put by.
    pub fn what_this_would_answer(&self, what: &str) -> crate::core::DriveType {
        use crate::core::DriveType;

        if self
            .inventory
            .get_item(what)
            .is_some_and(|item| item.is_food())
            || matches!(what, "food" | "grain" | "flour" | "bread" | "fish" | "meat")
        {
            return DriveType::Hunger;
        }

        if what == "water"
            || self
                .inventory
                .get_item(what)
                .is_some_and(|item| item.is_container())
        {
            return DriveType::Thirst;
        }

        if crate::environment::making::is_a_familiar_thing(what) {
            return DriveType::Utility;
        }

        DriveType::Preparedness
    }

    /// What taking a thing would actually be worth to this agent.
    ///
    /// The specification asks for theft to be decided on drive demand rather
    /// than on temperament, and this is the first half of it: how much of
    /// what this agent is asking for would a handful of that answer. A sack
    /// of grain is worth a great deal to a hungry man and nothing at all to a
    /// full one, and until now the decision could not tell the two apart
    /// because it never looked at what was being taken.
    pub fn what_taking_this_would_answer(&self, what: &str, how_many: u32) -> f32 {
        let answers = self.what_this_would_answer(what);

        let urgency = self
            .drives
            .get(answers)
            .map(|drive| drive.urgency().clamp(0.0, 1.0))
            .unwrap_or(0.0);

        // More of a thing is worth more, and sharply less so: the second
        // armful of grain does not answer hunger twice
        let how_much = (how_many as f32 / Self::WHAT_A_USEFUL_HAUL_IS).clamp(0.0, 1.0);

        urgency * how_much
    }

    /// How many of a thing counts as having got something worth having.
    const WHAT_A_USEFUL_HAUL_IS: f32 = 4.0;

    /// What taking it would cost this agent later.
    ///
    /// The second half: taking threatens *future* drive demand satisfaction,
    /// and in this model it does so through the bonds. Everything a person
    /// gets from other people - gifts, trades, news, somebody to have a child
    /// with, somebody who does not leave them to the wolves - runs on the
    /// bond, and a theft costs it with the victim and with everybody who saw.
    ///
    /// `watching` is how many people are near enough to see, the victim
    /// included. `bonds` is what this agent currently gets from the people it
    /// would be stealing in front of, on the same 0..1 scale a bond is on.
    pub fn what_taking_it_would_cost_me(&self, watching: usize, bonds: f32) -> f32 {
        use crate::core::traits::Trait;

        // What the eyes are worth. Doing it in front of nobody still costs
        // something, because the victim always knows.
        let seen = 1.0 + watching as f32 * Self::WHAT_ANOTHER_PAIR_OF_EYES_ADDS;

        let mut cost = bonds.clamp(0.0, 1.0) * seen * Self::WHAT_A_BOND_IS_WORTH_KEEPING;

        // Temperament does not decide this any more, but it does weigh it: an
        // honest man sees more at stake in being a thief and a greedy one
        // sees less
        if self.traits.has(Trait::Honest) {
            cost *= 1.6;
        }
        if self.traits.has(Trait::Greedy) {
            cost *= 0.6;
        }

        cost
    }

    /// What each further witness adds to what a theft costs.
    const WHAT_ANOTHER_PAIR_OF_EYES_ADDS: f32 = 0.5;

    /// What standing among the people you live with is worth, against the
    /// most a single haul could answer.
    ///
    /// Above 1.0, so that on an ordinary day the sums come out against
    /// stealing: a settlement where theft pays is a settlement that stops
    /// being one.
    const WHAT_A_BOND_IS_WORTH_KEEPING: f32 = 1.4;

    /// Whether this agent would take it, weighing the one against the other.
    ///
    /// > the decision to commit theft should be made if the theft will satisfy
    /// > drive demand and not threaten future drive demand satisfaction. if a
    /// > survival drive is strong enough, it will override the risk to future
    /// > drive satisfaction to ensure immediate survival.
    ///
    /// So: gain against cost, and a primary drive past
    /// `WHEN_TOMORROW_STOPS_MATTERING` sets the cost aside altogether. A man
    /// who will be dead by morning is not weighing his reputation.
    pub fn would_i_take_it(&self, gain: f32, cost: f32) -> bool {
        if self.is_a_survival_drive_past_bearing() {
            return gain > 0.0;
        }

        gain > cost
    }

    /// Whether something that kills you is far enough along to stop an agent
    /// caring what anybody thinks.
    pub fn is_a_survival_drive_past_bearing(&self) -> bool {
        use crate::core::drives::DriveRank;

        self.drives
            .drives
            .iter()
            .filter(|drive| drive.drive_type.rank() == DriveRank::Primary)
            .any(|drive| drive.urgency() >= Self::WHEN_TOMORROW_STOPS_MATTERING)
    }

    /// How hard something that kills you has to be pressing before a person
    /// stops weighing what it will cost them afterwards.
    ///
    /// High on purpose. This is the override, not the ordinary case: it is
    /// the difference between a hungry man and a starving one.
    const WHEN_TOMORROW_STOPS_MATTERING: f32 = 0.85;

    /// And when what it was told turns out to have been true once.
    ///
    /// A place somebody reported a season ago and which is bare now proves
    /// nothing against him except that his news keeps badly. It costs a little
    /// standing and no anger at all, which is the difference between a man who
    /// is out of date and a man who is lying.
    pub fn found_out_they_were_out_of_date(&mut self, them: uuid::Uuid) {
        self.knowledge
            .trust_ratings
            .entry(them)
            .or_insert_with(|| super::gossip::TrustRating::new(self.id, them))
            .update_on_stale_news();
    }

    /// And when what it was told turns out to be so.
    pub fn found_out_they_were_right(&mut self, them: uuid::Uuid) {
        self.knowledge
            .trust_ratings
            .entry(them)
            .or_insert_with(|| super::gossip::TrustRating::new(self.id, them))
            .update_on_verification(true);
    }

    /// Whose word this agent will take, and how readily.
    ///
    /// Trust was kept in three unconnected books. `TrustRating` in the
    /// knowledge base held a verified track record and was read when a belief
    /// was filed and nowhere else. `Relationship::trust_level` mapped the bond
    /// onto an enum and was read in one place, to decide whether a gift would
    /// be accepted. `TraitSet::combined_trust_modifier` summed every
    /// trust-flavoured trait an agent had, which mixes two different things:
    /// Paranoid is about whether *this* agent believes people, and Charismatic
    /// is about whether people believe *them*, and adding them together means
    /// a paranoid charmer trusts everybody slightly less than average for the
    /// wrong reason.
    ///
    /// Meanwhile the channel that actually carries information between agents
    /// - resource and building locations passing into `exploration_knowledge`,
    /// which is what foraging reads - consulted none of the three. An agent
    /// took a place-name from anybody, including somebody it had just named an
    /// enemy.
    ///
    /// Four things decide it, and the specification names three of them:
    /// what the two of them are to each other, whether this one has been right
    /// before, what sort of person is doing the listening, and what sort is
    /// doing the talking.
    pub fn how_far_i_trust(&self, them: uuid::Uuid, what_they_are_like: &TraitSet) -> f32 {
        use crate::core::traits::Trait;

        // A stranger is neither trusted nor distrusted
        const A_STRANGER: f32 = 0.5;

        // What the two of them are to each other. A bond of -1 to 1 becomes
        // nothing to everything, and this is the largest single term: you
        // believe your friends.
        let by_standing = match self.relationships.get_relationship(&them) {
            Some(bond) => (bond.bond_strength + 1.0) / 2.0,
            None => A_STRANGER,
        };

        // Whether they have been right before. Starts at neutral and moves
        // only on something the agent went and checked for itself.
        let by_record = self.knowledge.get_trust(&them);

        // What sort of person is doing the listening
        let my_disposition = if self.traits.has(Trait::Trusting) {
            0.2
        } else if self.traits.has(Trait::Paranoid) {
            -0.35
        } else if self.traits.has(Trait::Suspicious) {
            -0.2
        } else if self.traits.has(Trait::Skeptic) {
            -0.15
        } else {
            0.0
        };

        // And what sort is doing the talking. Bearing, not reputation - an
        // agent cannot read another's traits, but it can be charmed by one and
        // put off by another.
        let their_bearing = if what_they_are_like.has(Trait::Charismatic) {
            0.15
        } else if what_they_are_like.has(Trait::KindHearted) {
            0.1
        } else if what_they_are_like.has(Trait::Cruel) {
            -0.15
        } else {
            0.0
        };

        (by_standing * 0.6 + by_record * 0.4 + my_disposition + their_bearing).clamp(0.0, 1.0)
    }

    /// The point at which an agent will act on what it has been told.
    ///
    /// Below this it hears the claim and does not go and stand on it.
    pub const TAKE_SOMEBODY_AT_THEIR_WORD: f32 = 0.5;

    /// Whether this agent will act on something [`them`] has told it.
    pub fn would_take_their_word(&self, them: uuid::Uuid, what_they_are_like: &TraitSet) -> bool {
        self.how_far_i_trust(them, what_they_are_like) >= Self::TAKE_SOMEBODY_AT_THEIR_WORD
    }


    /// Whether this agent would lie to a room rather than to a man.
    ///
    /// "An agent thinking about lying should not only take into account the
    /// person they are talking to and any other agents who might overhear the
    /// lie."
    ///
    /// `would_lie_to` weighs one listener: how honest the speaker is, and what
    /// he thinks of the man in front of him. That is the right calculation for
    /// a word in somebody's ear and the wrong one for something said out loud.
    /// Three things change when there is a room:
    ///
    /// A lie told in front of somebody who has walked the ground is a lie that
    /// will be contradicted on the spot, and almost nobody tries it.
    ///
    /// Every extra pair of ears is another person who may go and look, and
    /// another mouth to tell everybody else what they found. The risk grows
    /// with the audience.
    ///
    /// And a lie is told because there is somebody in the room worth
    /// deceiving, so the room is weighed at its least friendly face rather
    /// than by whoever happens to be addressed. Weighing it at its
    /// *friendliest* - on the thought that a man will not lie in front of his
    /// friends - reads well and stops lying dead: bonds in a settlement are
    /// mostly warm, so the friendliest face in any room is nearly always a
    /// close one, and the discount for it cancelled the whole temptation.
    pub fn would_lie_to_this_room(&self, room: &[uuid::Uuid], current_tick: u32) -> bool {
        use rand::Rng;

        if room.is_empty() {
            return false;
        }

        // Whoever in the room he would most like to mislead
        let worth_deceiving = room
            .iter()
            .min_by(|a, b| {
                let bond = |who: &uuid::Uuid| {
                    self.relationships
                        .get_relationship(who)
                        .map(|r| r.bond_strength)
                        .unwrap_or(0.0)
                };
                bond(a).partial_cmp(&bond(b)).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(room[0]);

        if !self.would_lie_to(worth_deceiving, current_tick) {
            return false;
        }

        // And every extra pair of ears is another person who may go and look,
        // and another mouth to tell the rest what they found. One listener is
        // a private word and no different from before; a crowd of five is
        // about a third as tempting. Making each extra ear halve it instead
        // abolished lying altogether, which is not what "take into account"
        // means.
        let extra_ears = (room.len() - 1) as f64;
        crate::core::dice::roll().gen_bool((1.0 / (1.0 + 0.5 * extra_ears)).clamp(0.0, 1.0))
    }

    /// Check if this agent would lie when sharing information
    /// Based on traits and relationship with the target
    pub fn would_lie_to(&self, target_id: uuid::Uuid, _current_tick: u32) -> bool {
        use crate::core::traits::Trait;
        use rand::Rng;

        let mut lie_chance: f32 = 0.0;

        // Dishonest trait: 40% base lie chance
        if self.traits.has(Trait::Dishonest) {
            lie_chance += 0.4;
        }

        // Manipulative/Manipulator: 30% lie chance
        if self.traits.has(Trait::Manipulative) || self.traits.has(Trait::Manipulator) {
            lie_chance += 0.3;
        }

        // Honest trait: prevents lying (override)
        if self.traits.has(Trait::Honest) {
            return false;
        }

        // Relationship affects lying
        if let Some(rel) = self.relationships.get_relationship(&target_id) {
            if rel.bond_strength < -0.5 {
                // More likely to lie to enemies
                lie_chance += 0.2;
            } else if rel.bond_strength > 0.6 {
                // Less likely to lie to loved ones
                lie_chance -= 0.3;
            }
        }

        // KindHearted agents avoid harmful lies
        if self.traits.has(Trait::KindHearted) {
            lie_chance -= 0.2;
        }

        let roll: f32 = crate::core::dice::roll().gen();
        roll < lie_chance.clamp(0.0, 0.8) // Max 80% chance to lie
    }


}
/// An errand: somewhere to be, something to do there, and the drive it answers.
///
/// "Once an agent plans an action, it would not change its mind unless its
/// situation changed in some manner. The agent begins walking and for the next
/// ten ticks no new decisions need be made. If during the walk the agent ran
/// into a pack of wolves, it would need to recalculate."
///
/// Before this, every tile of every walk was a fresh decision made from
/// scratch, and the whole decision - which drive, which patch, which route -
/// was re-derived from a world that had moved one step. Measured, `Move` ran
/// at a third of all turns and the trips it was made of mostly did not finish:
/// a walk to a river twenty tiles off is twenty chances for whatever drive is
/// loudest that minute to send the agent somewhere else, so agents ate
/// whatever was underfoot when they gave up. That is why weighting food by
/// what it is worth measured *worse* than picking the nearest thing - the
/// better food was further off, and further off meant never arrived at.
///
/// What ends an errand is a change in what the agent needs, not the passing of
/// a turn: arriving, a threat, a different drive taking the lead, or the walk
/// going on so much longer than it should that the place is plainly not
/// reachable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Errand {
    /// Where it is going
    pub going_to: (i32, i32, i32),
    /// And the ground it set out from.
    ///
    /// There is no settlement object in this model - see ISSUES_FOUND #11 -
    /// so there is no camp to take a bearing from. Where somebody stood when
    /// they set out is the honest origin anyway: the thing being learned is
    /// "going that way answered it", and that way is relative to where the
    /// going started.
    #[serde(default)]
    pub set_out_from: (i32, i32, i32),
    /// Or what it is making, if the errand is a job rather than a journey.
    ///
    /// A tool is not one turn's work. Measured, the tool arithmetic diverted
    /// four turns in a run and produced **nothing**: a diversion buys the next
    /// step in a chain - a length of cordage, a knapped edge - and the turn
    /// after that the whole decision was made again from scratch and went
    /// somewhere else, so the settlement collected half-finished tools it
    /// never picked up again. The same defect as the walk that was re-decided
    /// at every tile, one layer up.
    pub to_make: Option<String>,
    /// Which drive it set out to answer
    pub for_drive: DriveType,
    /// How hard that drive was pressing when it set out, so that a drive going
    /// quiet - because somebody handed this one a meal, say - ends the errand
    pub pressed_this_hard: f32,
    /// How many turns it has been walking
    pub turns_on_it: u32,
    /// And how many turns it has been standing waiting while its owner dealt
    /// with something that would not wait.
    ///
    /// An errand used to be **destroyed** the moment another need took the
    /// turn. Measured over six worlds, 1,717 of 3,047 errands a settlement set
    /// out on ended that way - 56% - and 1,401 of those were a primary need
    /// taking the turn from a secondary one, most often a Preparedness errand
    /// cut short by thirst or hunger. Since a primary drive outranks a
    /// secondary one whatever its clock says, that happened to every single
    /// attempt at putting food by, over and over, and nothing was ever stocked.
    ///
    /// Going for a drink is not a change of mind. The errand waits.
    #[serde(default)]
    pub set_aside: u32,
}

impl Errand {
    /// How much longer than the crow flies a walk is allowed to take.
    ///
    /// A step is a tile, so a place twenty tiles off is twenty turns of
    /// walking at best. Ground is not flat and routes are not straight, so
    /// three times that is generous; past it the place is not reachable and
    /// going on is a way of starving politely.
    pub const HOW_LONG_A_WALK_IS_WORTH: u32 = 3;

    /// The fewest turns any errand is given, so that a short walk is not
    /// abandoned on its first step.
    pub const AT_LEAST_THIS_MANY_TURNS: u32 = 4;

    /// How long an errand keeps while its owner is doing something else.
    ///
    /// Two days of the world's calendar. Long enough to outlast a drink, a
    /// meal, a night's sleep and the walk to each; short enough that a patch
    /// remembered two days ago is not still being walked to a season later,
    /// which is the failure the old behaviour was avoiding by throwing the
    /// errand away.
    pub const HOW_LONG_AN_ERRAND_KEEPS: u32 =
        2 * crate::environment::seasons::TICKS_PER_DAY;

    /// Whether this one has been waiting too long to still be worth resuming.
    pub fn stale(&self) -> bool {
        self.set_aside > Self::HOW_LONG_AN_ERRAND_KEEPS
    }

    /// How far off it was set out from
    pub fn how_far_it_was(&self, from: (i32, i32, i32)) -> u32 {
        (self.going_to.0 - from.0).abs().max((self.going_to.1 - from.1).abs()) as u32
    }

    /// Whether this one has got there.
    pub fn arrived(&self, at: (i32, i32, i32)) -> bool {
        self.going_to.0 == at.0 && self.going_to.1 == at.1
    }

}

