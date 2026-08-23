// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, BehaviorNode, NodeType, DriveState, DriveType, Memory, GoalManager, Preferences, GoalWorldState};
use crate::core::planning::{ActionPlan, PlanActionType, Planner, PlanStep, ActionOutcome};
use crate::environment::{Action, ActionResult};
use std::collections::HashMap;

use super::senses::Senses;
use super::body::Body;
use super::skills::Skills;
use super::emotions::{EmotionState, EmotionSource, RelationshipMap};
use crate::core::traits::TraitSet;
use super::gossip::KnowledgeBase;
use super::observational_learning::ObservationalLearning;
use super::transport::TransportSystem;
use crate::environment::TechnologyKnowledge;
use crate::world::nutrition::{FoodData, NutritionalState, NutritionalContent, EatResult};

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
    pub fn is_food(&self) -> bool {
        self.food_data.is_some()
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

    /// Check if item can be repaired (not broken, has durability < max)
    pub fn can_be_repaired(&self) -> bool {
        match (self.current_durability, self.max_durability) {
            (Some(current), Some(max)) => current > 0.0 && current < max,
            _ => false,
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
        let base_weight = self.weight_per_unit * self.quantity as f32;

        // Add liquid weight if this is a filled container
        // Water weighs ~1 kg per liter
        let liquid_weight = self.fill_level.unwrap_or(0.0);

        base_weight + liquid_weight
    }
}

/// Agent inventory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Items stored by item_id
    items: HashMap<String, InventoryItem>,
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
            items: HashMap::new(),
            max_slots,
            max_weight,
            current_weight: 0.0,
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
        if self.current_weight + item_weight > self.max_weight {
            return false; // Too heavy
        }

        // Update weight
        self.current_weight += item_weight;

        // Add or stack item
        if let Some(existing) = self.items.get_mut(&item.item_id) {
            existing.quantity += item.quantity;
        } else {
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
    pub fn get_all_items(&self) -> &HashMap<String, InventoryItem> {
        &self.items
    }

    /// Recalculate total weight from all items
    pub fn recalculate_weight(&mut self) {
        self.current_weight = self.items.values()
            .map(|item| item.total_weight())
            .sum();
    }

    /// Check if inventory is overweight
    pub fn is_overweight(&self) -> bool {
        self.current_weight > self.max_weight
    }

    /// Get weight capacity remaining
    pub fn weight_capacity_remaining(&self) -> f32 {
        (self.max_weight - self.current_weight).max(0.0)
    }

    /// Get weight as percentage of max (0.0 to 1.0+)
    pub fn weight_percentage(&self) -> f32 {
        if self.max_weight == 0.0 {
            0.0
        } else {
            self.current_weight / self.max_weight
        }
    }

    /// Increase max weight capacity (from backpack, etc.)
    pub fn add_capacity(&mut self, additional_weight: f32) {
        self.max_weight += additional_weight;
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(20, 100.0) // Default: 20 slots, 100 weight units
    }
}

/// Life stages of an agent
///
/// A year is [`crate::environment::TICKS_PER_YEAR`] ticks, so the boundaries
/// below are roughly: infancy to five months, childhood to a year and a
/// quarter, adolescence to two years, adulthood to seven, old age after that.
/// An agent lives eight or nine years and sees thirty-odd seasons turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifeStage {
    /// 0-500 ticks, cannot reproduce, learns from parents
    Infant,
    /// 500-1500 ticks, cannot reproduce, high learning rate
    Child,
    /// 1500-2500 ticks, can reproduce, still learning
    Adolescent,
    /// 2500-8000 ticks, prime reproduction age
    Adult,
    /// 8000+ ticks, reduced fertility, wisdom phase
    Elderly,
}

impl LifeStage {
    /// Get life stage based on age
    pub fn from_age(age: u32) -> Self {
        match age {
            0..=500 => LifeStage::Infant,
            501..=1500 => LifeStage::Child,
            1501..=2500 => LifeStage::Adolescent,
            2501..=8000 => LifeStage::Adult,
            _ => LifeStage::Elderly,
        }
    }

    /// Check if agent can reproduce at this stage
    pub fn can_reproduce(&self) -> bool {
        matches!(self, LifeStage::Adolescent | LifeStage::Adult | LifeStage::Elderly)
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
        match self {
            LifeStage::Infant => 0.25,
            LifeStage::Child => 0.45,
            LifeStage::Adolescent => 0.75,
            LifeStage::Adult => 1.0,
            LifeStage::Elderly => 0.6,
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
    pub ticks_without_water: u32, // Count dehydration duration
    /// What this body has to pass, waiting to be left on the ground.
    ///
    /// Everything eaten used to leave the world for good, so a settlement was
    /// a one-way pump from the soil into nothing. What a body takes in, most
    /// of it comes out again somewhere.
    #[serde(default)]
    pub waste_carried: f32,
}

impl AgentState {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        // Max age varies between 9000-11000 ticks
        let max_age = rng.gen_range(9000..11000);

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
            ticks_without_water: 0,
            waste_carried: 0.0,
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

        // What this body has stored to go on. A grown adult carries fat and
        // muscle worth weeks of it; a small child carries days. Every
        // threshold below is measured against that, so a famine takes the
        // young and the old first and the people in their prime last.
        let reserve = self.life_stage.hunger_reserve().max(0.05);

        // Energy depletion (normal metabolism)
        let base_energy_loss = 0.05 * energy_multiplier; // Apply pregnancy/other multiplier
        let mut energy_loss = base_energy_loss;

        // After a day without food: energy depletes faster, and a small body
        // with little put by depletes faster still
        if self.ticks_without_food as f32 > 1440.0 * reserve {
            energy_loss *= 1.0 + 1.0 / reserve;
        }

        // Three days on an adult's reserves; sooner on a child's
        if self.ticks_without_food as f32 > 4320.0 * reserve {
            let health_loss = 0.1 / reserve;
            self.health = (self.health - health_loss).max(0.0);
        }

        // A week on an adult's reserves, and death is close
        if self.ticks_without_food as f32 > 10080.0 * reserve {
            let severe_health_loss = 1.0 / reserve;
            self.health = (self.health - severe_health_loss).max(0.0);
        }

        // === DEHYDRATION MECHANICS (faster than starvation) ===
        // After 12 hours (720 ticks) without water: energy depletes faster
        if self.ticks_without_water > 720 {
            energy_loss *= 1.5; // Additional 50% energy depletion
        }

        // After 1.5 days (2160 ticks) without water: health starts decreasing
        if self.ticks_without_water > 2160 {
            let health_loss = 0.15; // Moderate health degradation
            self.health = (self.health - health_loss).max(0.0);
        }

        // After 3 days (4320 ticks) without water: rapid health loss (death imminent)
        if self.ticks_without_water > 4320 {
            let severe_health_loss = 1.5; // Rapid health loss (faster than starvation)
            self.health = (self.health - severe_health_loss).max(0.0);
        }

        // Apply energy loss
        self.energy = (self.energy - energy_loss).max(0.0);

        // When energy is depleted, health starts decreasing too
        if self.energy <= 0.0 {
            self.health = (self.health - 0.05).max(0.0);
        }

        // Check for death from old age
        if self.age >= self.max_age {
            self.is_alive = false;
        }

        // Check for death from injury/starvation/dehydration
        if self.health <= 0.0 {
            self.is_alive = false;
        }
    }

    /// Take damage
    pub fn take_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
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

        let reserve = self.life_stage.hunger_reserve().max(0.05);
        let health = self.health.max(0.0);

        match drive_type {
            // Thirst is the fast one. Health starts going at a day and a half
            // and goes fifteen times faster after three days, which is why a
            // thirsty agent should stop whatever it is doing even if that
            // thing is fetching food.
            DriveType::Thirst => {
                let dry = self.ticks_without_water as f32;
                let until_it_bites = (2_160.0 - dry).max(0.0);
                let until_it_races = (4_320.0 - dry).max(0.0);

                // Slow loss while between the two, then rapid
                let slow_span = (until_it_races - until_it_bites).max(0.0);
                let lost_slowly = slow_span * 0.15;
                let left_for_the_race = (health - lost_slowly).max(0.0);

                Some(until_it_races + once_health_goes(left_for_the_race, 1.5))
            }

            // Starvation is slower and scales with what the body has put by
            DriveType::Hunger => {
                let empty = self.ticks_without_food as f32;
                let until_it_bites = (4_320.0 * reserve - empty).max(0.0);
                let until_it_races = (10_080.0 * reserve - empty).max(0.0);

                let slow_span = (until_it_races - until_it_bites).max(0.0);
                let lost_slowly = slow_span * (0.1 / reserve);
                let left_for_the_race = (health - lost_slowly).max(0.0);

                Some(until_it_races + once_health_goes(left_for_the_race, 1.0 / reserve))
            }

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
        self.ticks_without_food > 1440 || self.energy < 20.0
    }

    /// Check if agent is dehydrated (critical survival state)
    /// Dehydration is more urgent than starvation (720 ticks = 12 hours)
    pub fn is_dehydrated(&self) -> bool {
        self.ticks_without_water > 720
    }

    /// Check if agent is in critical survival state
    pub fn is_survival_critical(&self) -> bool {
        self.is_starving() || self.is_dehydrated() || self.health < 30.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
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
    pub current_plan: Option<ActionPlan>,
    /// Planning engine for generating and learning from plans
    pub planner: Planner,
    /// Ticks spent on current plan step (for timeout detection)
    pub plan_step_ticks: u32,
    /// Accumulated learning exposure for various knowledge/skills
    pub learning_exposure: crate::core::learning::LearningExposure,
    /// Nutritional state (energy, protein, micronutrients)
    pub nutrition: NutritionalState,
    /// Biological gender
    pub gender: super::gender::Gender,
    /// Pregnancy state (for females)
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
            id: Uuid::new_v4(),
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
            storage_preferences: super::storage_management::StoragePreferences::default(),
            parent_ids: Vec::new(),
            practices: super::practices::Practices::new(),
            lessons: super::practices::Lessons::new(),
            surroundings: crate::core::Surroundings::default(),
            goals: GoalManager::new(5), // Max 5 active goals
            preferences: Preferences::default(),
            equipment: super::equipment::EquipmentManager::new(50.0), // 50kg max carry weight
            satisfaction_tracker: super::drive_satisfaction::SatisfactionTracker::new(),
            current_plan: None,
            planner: Planner::new(),
            plan_step_ticks: 0,
            learning_exposure: crate::core::learning::LearningExposure::new(),
            nutrition: NutritionalState::new(),
            gender: super::gender::Gender::random(),
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

        agent
    }

    /// Generate a personality-based reproduction drive modifier
    fn generate_reproduction_modifier() -> f32 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
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

        // Set up infant as newborn
        agent.state.life_stage = LifeStage::Infant;
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
        let mut rng = rand::thread_rng();
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
            at_leisure: false,
        }
    }

    /// What counts as material to work or build with
    pub const MATERIALS: [&'static str; 7] = [
        "wood", "stone", "iron", "clay", "sand", "coal", "brick",
    ];

    /// What counts as a tool
    const TOOLS: [&'static str; 8] = [
        "axe", "pick", "hoe", "shovel", "spade", "knife", "hammer", "tool",
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

    /// Get the current healing multiplier from nearby buildings
    pub fn get_healing_bonus(&self) -> f32 {
        self.cached_healing_bonus
    }

    /// Get the current defense multiplier from nearby buildings
    pub fn get_defense_bonus(&self) -> f32 {
        self.cached_defense_bonus
    }

    /// Get the productivity bonus for crafting/working based on nearby buildings
    /// This considers the agent's current position and the buildings they're at
    /// Returns the best productivity bonus from buildings within working distance (3 tiles)
    pub fn get_productivity_bonus(&self) -> f32 {
        const WORKING_DISTANCE_SQ: f32 = 9.0; // 3 tiles

        let pos = self.state.position;
        let mut best_bonus: f32 = 1.0;

        for (building_pos, building_type) in &self.exploration_knowledge.known_buildings {
            let dx = (pos.0 - building_pos.x) as f32;
            let dy = (pos.1 - building_pos.y) as f32;
            let dist_sq = dx * dx + dy * dy;

            // Only consider buildings within working distance
            if dist_sq <= WORKING_DISTANCE_SQ {
                let bonus = building_type.productivity_bonus();
                if bonus > best_bonus {
                    best_bonus = bonus;
                }
            }
        }

        best_bonus
    }

    /// Get the productivity bonus for a specific skill type based on nearby buildings
    /// Different buildings give bonuses for different types of work
    pub fn get_productivity_bonus_for_skill(&self, skill_type: super::SkillType) -> f32 {
        use crate::world::BuildingType;
        use super::SkillType;

        const WORKING_DISTANCE_SQ: f32 = 9.0;

        let pos = self.state.position;
        let mut best_bonus: f32 = 1.0;

        for (building_pos, building_type) in &self.exploration_knowledge.known_buildings {
            let dx = (pos.0 - building_pos.x) as f32;
            let dy = (pos.1 - building_pos.y) as f32;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= WORKING_DISTANCE_SQ {
                // Check if this building provides bonus for this skill type
                let provides_bonus = match (building_type, skill_type) {
                    // Metalworking buildings
                    (BuildingType::Smithy, SkillType::Metalworking | SkillType::Smelting) => true,
                    (BuildingType::Forge, SkillType::Smelting) => true,
                    // Crafting buildings
                    (BuildingType::Workshop, SkillType::Crafting | SkillType::Carpentry) => true,
                    // Cooking buildings
                    (BuildingType::Bakery | BuildingType::Mill | BuildingType::Butchery |
                     BuildingType::Brewery | BuildingType::Dairy, SkillType::Cooking) => true,
                    // Textile buildings
                    (BuildingType::WeaverHut | BuildingType::TailorShop, SkillType::Crafting) => true,
                    // Leather buildings
                    (BuildingType::Tannery | BuildingType::CobblerShop, SkillType::Leatherworking) => true,
                    // Masonry buildings
                    (BuildingType::PotteryKiln | BuildingType::Brickyard, SkillType::Masonry) => true,
                    // Farming buildings
                    (BuildingType::Farm | BuildingType::AnimalPen, SkillType::Farming) => true,
                    _ => false,
                };

                if provides_bonus {
                    let bonus = building_type.productivity_bonus();
                    if bonus > best_bonus {
                        best_bonus = bonus;
                    }
                }
            }
        }

        best_bonus
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

        // Check for stale storage knowledge and trigger curiosity
        self.update_storage_curiosity(current_tick);

        // Update emotions based on drive states (every tick)
        self.update_emotions_from_drives();
        // Drives rise differently depending on whether the agent has anything
        // more pressing on. See `DriveType::is_long_term`.
        let secure = self.immediate_needs_met();
        let situation = self.what_the_situation_asks();
        self.drives.tick_in(&situation, secure);

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
    const A_LONG_WAY_OFF: f32 = 720.0;

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

        let wanting = drive.urgency();

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
            self.state.health = (self.state.health - penalty).max(0.0);
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
        let spoiled_items: Vec<String> = self.inventory.items.iter()
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
            }
        }
    }

    /// Update body temperature based on environmental conditions
    ///
    /// # Arguments
    /// * `climate` - Environmental climate conditions
    pub fn update_temperature(&mut self, climate: &super::Climate) {
        self.update_temperature_with_shelter(climate, false);
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
            self.state.health = (self.state.health - damage * 10.0).max(0.0);
        }

        damage
    }

    /// Check if agent needs shelter based on current exposure
    pub fn needs_shelter(&self) -> bool {
        // Seek shelter if exposure is getting dangerous
        self.exposure_status.is_critical() ||
        !self.exposure_status.active_exposures.is_empty()
    }

    /// Get recommended shelter-seeking priority (0.0 to 1.0)
    pub fn shelter_priority(&self) -> f32 {
        if self.exposure_status.is_critical() {
            1.0 // Critical - seek shelter immediately
        } else if !self.exposure_status.active_exposures.is_empty() {
            0.5 + (self.exposure_status.total_severity() * 0.5) // Moderate priority
        } else {
            0.0 // No exposure risk
        }
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

        let base_strength = health_factor * body_factor;
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

        let assessment =
            ThreatAssessment::assess(self.own_strength(), threat_strength, source.clone());

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

    /// Respond emotionally to harm to a loved one
    ///
    /// # Arguments
    /// * `loved_one_id` - UUID of the loved one
    /// * `harm_severity` - How severe the harm was (0.0 to 1.0)
    /// * `source` - Source of the harm
    pub fn respond_to_loved_one_harm(&mut self, loved_one_id: &Uuid, harm_severity: f32, source: super::EmotionSource) {
        // Check if this is actually a loved one
        if let Some(relationship) = self.relationships.get_relationship(loved_one_id) {
            if relationship.is_loved_one() {
                // Sadness scales with bond strength and harm severity
                let sadness_amount = relationship.bond_strength * harm_severity * 0.8;
                self.emotions.add_sadness_with_traits(source.clone(), sadness_amount, &self.traits);

                // Also potentially add fear or anger based on agent's ability to protect
                // Parents protecting children might feel anger if they can fight back
                if relationship.relationship_type == super::RelationshipType::Child {
                    // Calculate if agent is strong enough to retaliate
                    let agent_strength = self.state.health / 100.0;

                    // Assume medium threat strength for the source
                    let assessment = super::ThreatAssessment::assess(agent_strength, 0.7, source.clone());

                    if assessment.can_overcome {
                        self.emotions.add_anger_with_traits(source, 0.5, &self.traits);
                    } else {
                        self.emotions.add_fear_with_traits(source, 0.3, &self.traits);
                    }
                }
            }
        }
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

    /// Check if agent would flee from current emotional state
    pub fn would_flee(&self) -> bool {
        self.emotions.should_flee()
    }

    /// Check if agent would attack from current emotional state
    pub fn would_attack(&self) -> bool {
        self.emotions.should_attack()
    }

    /// Get agent's dominant emotion
    pub fn dominant_emotion(&self) -> Option<super::EmotionType> {
        self.emotions.dominant_emotion()
    }

    /// Share information with another agent
    ///
    /// # Arguments
    /// * `info` - The information to share
    /// * `recipient` - The agent receiving the information
    /// * `timestamp` - Current simulation time
    pub fn share_information(&self, mut info: super::Information, recipient: &mut Agent, timestamp: u64) {
        // Check if this agent would distort the information
        if let Some(distortion_trait) = self.traits.would_distort_info() {
            // Apply distortion based on trait
            info = info.distort(distortion_trait, self.id);

            // Gain happiness from distortion
            // This would integrate with a happiness/mood system
        }

        // Recipient receives information
        recipient.knowledge.receive_information(info, self.id, recipient.id, &recipient.traits, timestamp);
    }

    /// Learn information directly (observed firsthand)
    ///
    /// # Arguments
    /// * `info` - The information learned
    /// * `timestamp` - Current simulation time
    pub fn learn_information(&mut self, info: super::Information, timestamp: u64) {
        // When learning firsthand, source is self
        self.knowledge.receive_information(info, self.id, self.id, &self.traits, timestamp);
    }

    /// Check if agent believes specific information
    pub fn believes(&self, info_id: &Uuid) -> bool {
        self.knowledge.believes(info_id)
    }

    /// Get trust level for another agent
    pub fn get_trust_in(&self, other_agent: &Uuid) -> f32 {
        self.knowledge.get_trust(other_agent)
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

    /// Record that information from another agent was verified as correct
    pub fn verify_information_from(&mut self, source_id: uuid::Uuid, _info_age: u32, current_tick: u32) {
        if let Some(rel) = self.relationships.get_relationship_mut(&source_id) {
            rel.positive_interaction(2, current_tick);
        }
    }

    /// Record that information from another agent was incorrect
    pub fn information_was_wrong_from(&mut self, source_id: uuid::Uuid, _info_age: u32, current_tick: u32) {
        if let Some(rel) = self.relationships.get_relationship_mut(&source_id) {
            rel.negative_interaction(3, current_tick);
        }
    }

    /// React to learning about another agent's trait
    ///
    /// # Arguments
    /// * `other_agent` - The other agent's UUID
    /// * `other_trait` - The trait learned about
    ///
    /// Example: Believer learns Atheist has Atheist trait → relationship weakens
    pub fn react_to_trait_info(&mut self, other_agent: &Uuid, other_trait: super::Trait) {
        // Check for trait conflicts
        if self.traits.has_trait(&super::Trait::Believer) && other_trait == super::Trait::Atheist {
            // Believer dislikes Atheist
            if let Some(relationship) = self.relationships.get_relationship_mut(other_agent) {
                relationship.weaken(0.2);
            } else {
                // Create negative relationship
                let mut new_rel = super::Relationship::new(*other_agent, super::RelationshipType::Acquaintance);
                new_rel.bond_strength = -0.2;
                self.relationships.add_relationship(new_rel);
            }
        } else if self.traits.has_trait(&super::Trait::Atheist) && other_trait == super::Trait::Believer {
            // Atheist may dislike Believer
            if let Some(relationship) = self.relationships.get_relationship_mut(other_agent) {
                relationship.weaken(0.1);
            } else {
                let mut new_rel = super::Relationship::new(*other_agent, super::RelationshipType::Acquaintance);
                new_rel.bond_strength = -0.1;
                self.relationships.add_relationship(new_rel);
            }
        }
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
            DriveType::Safety | DriveType::Shelter | DriveType::Reproduction |
            DriveType::Protection | DriveType::Luxury => None,
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

    /// Get effective skill modifier (affected by fatigue)
    pub fn skill_effectiveness(&self) -> f32 {
        self.fatigue.skill_modifier()
    }

    /// Get decision quality modifier (affected by fatigue)
    pub fn decision_quality(&self) -> f32 {
        self.fatigue.decision_modifier()
    }

    /// Get injury chance modifier (affected by fatigue)
    pub fn injury_risk_modifier(&self) -> f32 {
        self.fatigue.injury_chance_modifier()
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

    /// Update inventory max_weight based on active transports and body strength
    fn update_inventory_capacity_from_transport(&mut self) {
        // Base capacity (100kg default)
        let base_capacity = 100.0;

        // Strength modifier from body functionality
        // Stronger/healthier body can carry more
        let strength_modifier = self.body.movement_speed_multiplier(); // 0.0 to 1.0

        // Transport capacity
        let transport_capacity = self.transport.total_additional_capacity();

        // Total capacity
        let total_capacity = (base_capacity * strength_modifier) + transport_capacity;

        self.inventory.max_weight = total_capacity;
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

        body_speed * transport_speed * weight_penalty * fatigue_penalty * pregnancy_penalty
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

        // Females cannot reproduce while pregnant
        if self.gender.can_become_pregnant() && self.pregnancy.is_some() {
            return false;
        }

        true
    }

    /// Check if agent is infertile
    pub fn is_infertile(&self) -> bool {
        self.traits.has(crate::core::traits::Trait::Infertile)
    }

    /// Check if female agent can become pregnant
    pub fn can_become_pregnant(&self) -> bool {
        self.can_reproduce() && self.gender.can_become_pregnant() && self.pregnancy.is_none()
    }

    /// Check if male agent can impregnate
    pub fn can_impregnate(&self) -> bool {
        self.can_reproduce() && self.gender.can_impregnate()
    }

    /// Check if this agent is currently pregnant
    pub fn is_pregnant(&self) -> bool {
        self.pregnancy.is_some()
    }

    /// How much food an agent has to be carrying, over and above its own
    /// needs, before it will consider a child.
    ///
    /// A few days' eating for two. Not a stockpile - just enough that the
    /// answer to "could this child be fed next week" is something other than
    /// "I had a meal this morning".
    pub const FOOD_TO_RAISE_A_CHILD: u32 = 4;

    /// How long hunger has to have been a non-issue before an agent treats
    /// the future as settled.
    ///
    /// Twenty days of never once going short. Long enough that a good week
    /// does not count, short enough that a settlement living well can grow.
    pub const SETTLED_ENOUGH_TO_GROW: u32 = 240;

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

    /// How long hunger has not had to ask at all
    pub fn how_long_food_has_been_easy(&self) -> u32 {
        self.drives
            .get(DriveType::Hunger)
            .map(|drive| drive.answered_ticks())
            .unwrap_or(0)
    }

    /// What the agent is carrying that it or a child could eat.
    ///
    /// Counts untracked stacks as well as ones with nutrition data on them:
    /// most of what an agent picks up off the land arrives as a plain "food"
    /// stack with no freshness attached, and a count that ignored those would
    /// report an empty pack for an agent carrying a fortnight's eating.
    pub fn food_put_by(&self) -> u32 {
        self.inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter(|item| match &item.food_data {
                Some(food) => !food.is_spoiled() && !food.is_harmful(),
                None => Self::LOOKS_EDIBLE
                    .iter()
                    .any(|edible| item.item_id.contains(edible)),
            })
            .map(|item| item.quantity)
            .sum()
    }

    /// Item names that mean food when there is no nutrition data to go on
    const LOOKS_EDIBLE: [&'static str; 6] = ["food", "grain", "meat", "fish", "berr", "bread"];

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

        // Either there is food in hand for two, or feeding themselves has
        // simply not been a problem for a long stretch. The second matters as
        // much as the first: an agent living beside a full field eats as it
        // goes and carries nothing, and it is in a far better position to
        // raise a child than one with a full pack on ground that has stopped
        // giving.
        self.food_put_by() >= Self::FOOD_TO_RAISE_A_CHILD
            || self.how_long_food_has_been_easy() >= Self::SETTLED_ENOUGH_TO_GROW
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

    /// Get effective reproduction drive (base drive * personal modifier)
    pub fn effective_reproduction_drive(&self) -> f32 {
        let base_drive = self.drives.get(DriveType::Reproduction)
            .map(|d| d.value)
            .unwrap_or(0.0);
        (base_drive * self.reproduction_drive_modifier).clamp(0.0, 1.0)
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
            DriveType::Safety => {
                let mut selector = BehaviorNode::new(NodeType::Selector);
                selector.add_child(BehaviorNode::new(NodeType::Action("seek_shelter".to_string())));
                selector.add_child(BehaviorNode::new(NodeType::Action("craft_weapon".to_string())));
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

    /// Convert a behavior tree action into an actual environment action
    pub fn action_from_tree_result(&self, action_name: &str) -> Action {
        match action_name {
            "eat_stored_food" | "gather_food" | "hunt" => Action::Eat { food_type: "generic".to_string() },
            "sleep" => Action::Sleep { duration: 10 },
            "find_shelter" | "seek_shelter" => Action::Move { target: self.find_nearest_shelter() },
            "build_shelter" | "build_structure" => Action::Build {
                structure_type: "shelter".to_string(),
                position: self.state.position
            },
            "mine_resources" | "gather_resources" => Action::Gather { resource_type: "generic".to_string() },
            "process_materials" => Action::Craft { item_type: "processed_material".to_string() },
            "explore" | "experiment" => Action::Explore { direction: self.random_direction() },
            "find_agents" | "socialize" => Action::Socialize { target_agent_id: Uuid::nil() },
            "craft_tools" | "craft_weapon" => Action::Craft { item_type: "tool".to_string() },
            "store_resources" => Action::Store { item_type: "resource".to_string(), amount: 1 },
            "plant_crops" | "harvest" => Action::Gather { resource_type: "food".to_string() },
            "reproduce" => Action::Wait, // Special handling needed
            "seek_luxury" | "decorate" => Action::Gather { resource_type: "luxury".to_string() },
            _ => Action::Wait,
        }
    }

    /// Note how an attempt turned out, so the agent can stop doing what does
    /// not work and do more of what does.
    ///
    /// Called for every action an agent takes. Most actions teach it nothing
    /// worth remembering - walking somewhere either works or the ground was in
    /// the way - so only the undertakings an agent could sensibly form an
    /// opinion about are recorded.
    pub fn learn_from(&mut self, action: &Action, worked: bool) {
        use super::practices::Undertaking;

        let undertaking = match action {
            Action::Hunt { .. } => Undertaking::Hunting,
            Action::Fight { .. } => Undertaking::Fighting,
            Action::Fish => Undertaking::Fishing,
            Action::Cook { .. } | Action::LightFire => Undertaking::Cooking,
            Action::TillSoil | Action::SpreadMuck => Undertaking::Farming,
            Action::MakeClothing { .. } | Action::WearClothing { .. } => Undertaking::Clothing,
            Action::Gather { .. } => Undertaking::Foraging,
            Action::Build { .. } => Undertaking::Building,
            Action::Craft { .. } => Undertaking::Crafting,
            Action::Socialize { .. } | Action::ShareInformation { .. } => Undertaking::Dealing,
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

    fn random_direction(&self) -> (i32, i32, i32) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (
            rng.gen_range(-1..=1),
            rng.gen_range(-1..=1),
            0
        )
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

    /// Equip an item from inventory into a specific slot
    pub fn equip_from_inventory(&mut self, item_id: &str, slot: super::equipment::EquipmentSlot) -> Result<(), String> {
        // Check if item exists in inventory
        if !self.inventory.has_item(item_id, 1) {
            return Err(format!("Item '{}' not found in inventory", item_id));
        }

        // Item registry: maps item_id to (equipment_type, material, quality)
        let (equipment_type, material, quality) = Self::lookup_item_properties(item_id, slot);

        let equipment_item = super::equipment::EquipmentItem::new(
            item_id.to_string(),
            equipment_type,
            slot,
            material,
            quality,
        );

        // Equip the item (this returns the previously equipped item if any)
        match self.equipment.equip(equipment_item) {
            Ok(old_item) => {
                // Remove from inventory
                self.inventory.remove_item(item_id, 1);

                // If there was an old item, add it back to inventory
                if let Some(old) = old_item {
                    self.inventory.add_item(InventoryItem::new(old.name, 1));
                }

                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Lookup equipment properties from item registry
    /// Returns (equipment_type, material, quality) for the given item
    fn lookup_item_properties(
        item_id: &str,
        slot: super::equipment::EquipmentSlot,
    ) -> (super::equipment::EquipmentType, super::equipment::EquipmentMaterial, super::skills::Quality) {
        use super::equipment::{EquipmentType, EquipmentMaterial, WoodMaterial, MetalMaterial, ClothingMaterial};
        use super::skills::Quality;

        // Normalize item_id for matching
        let item_lower = item_id.to_lowercase();

        // Match based on item name patterns
        // Tools
        if item_lower.contains("pickaxe") || item_lower.contains("pick") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Pickaxe, mat, qual);
        }
        if item_lower.contains("axe") && !item_lower.contains("pick") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Hatchet, mat, qual);
        }
        if item_lower.contains("shovel") || item_lower.contains("spade") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Shovel, mat, qual);
        }
        if item_lower.contains("hammer") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Hammer, mat, qual);
        }
        if item_lower.contains("sickle") || item_lower.contains("scythe") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Sickle, mat, qual);
        }
        if item_lower.contains("fishing") || item_lower.contains("rod") {
            return (EquipmentType::FishingRod, EquipmentMaterial::Wood(WoodMaterial::Oak), Quality::Basic);
        }

        // Weapons
        if item_lower.contains("sword") || item_lower.contains("blade") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Sword, mat, qual);
        }
        if item_lower.contains("spear") || item_lower.contains("lance") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Spear, mat, qual);
        }
        if item_lower.contains("mace") || item_lower.contains("club") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Mace, mat, qual);
        }
        if item_lower.contains("dagger") || item_lower.contains("knife") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Dagger, mat, qual);
        }
        if item_lower.contains("bow") && !item_lower.contains("cross") {
            return (EquipmentType::Bow, EquipmentMaterial::Wood(WoodMaterial::Yew), Quality::Basic);
        }
        if item_lower.contains("crossbow") {
            return (EquipmentType::Crossbow, EquipmentMaterial::Wood(WoodMaterial::Oak), Quality::Basic);
        }
        if item_lower.contains("shield") {
            let (mat, qual) = Self::parse_material_quality(&item_lower, true);
            return (EquipmentType::Shield, mat, qual);
        }

        // Armor
        if item_lower.contains("plate") || item_lower.contains("heavy armor") {
            return (EquipmentType::HeavyArmor, EquipmentMaterial::Metal(MetalMaterial::Iron), Quality::Basic);
        }
        if item_lower.contains("chain") || item_lower.contains("mail") {
            return (EquipmentType::MediumArmor, EquipmentMaterial::Metal(MetalMaterial::Iron), Quality::Basic);
        }
        if item_lower.contains("leather armor") || item_lower.contains("hide armor") {
            return (EquipmentType::LightArmor, EquipmentMaterial::Cloth(ClothingMaterial::Leather), Quality::Basic);
        }

        // Clothing
        if item_lower.contains("fur") {
            return (EquipmentType::Clothing, EquipmentMaterial::Cloth(ClothingMaterial::Fur), Quality::Basic);
        }
        if item_lower.contains("wool") {
            return (EquipmentType::Clothing, EquipmentMaterial::Cloth(ClothingMaterial::Wool), Quality::Basic);
        }
        if item_lower.contains("leather") {
            return (EquipmentType::Clothing, EquipmentMaterial::Cloth(ClothingMaterial::Leather), Quality::Basic);
        }
        if item_lower.contains("linen") {
            return (EquipmentType::Clothing, EquipmentMaterial::Cloth(ClothingMaterial::Linen), Quality::Basic);
        }
        if item_lower.contains("cotton") {
            return (EquipmentType::Clothing, EquipmentMaterial::Cloth(ClothingMaterial::Cotton), Quality::Basic);
        }

        // Utility
        if item_lower.contains("torch") {
            return (EquipmentType::Torch, EquipmentMaterial::Wood(WoodMaterial::Oak), Quality::Basic);
        }
        if item_lower.contains("lantern") || item_lower.contains("lamp") {
            return (EquipmentType::Lantern, EquipmentMaterial::Metal(MetalMaterial::Iron), Quality::Basic);
        }

        // Default based on slot type
        match slot {
            super::equipment::EquipmentSlot::MainHand | super::equipment::EquipmentSlot::OffHand => {
                (EquipmentType::Pickaxe, EquipmentMaterial::Wood(WoodMaterial::Oak), Quality::Basic)
            }
            _ => (EquipmentType::Clothing, EquipmentMaterial::Cloth(ClothingMaterial::Linen), Quality::Basic)
        }
    }

    /// Parse material and quality from item name
    fn parse_material_quality(item_name: &str, is_tool_or_weapon: bool) -> (super::equipment::EquipmentMaterial, super::skills::Quality) {
        use super::equipment::{EquipmentMaterial, WoodMaterial, MetalMaterial, ClothingMaterial, StoneMaterial};
        use super::skills::Quality;

        // Parse quality (using Quality enum variants: Pathetic, Crude, Basic, Moderate, Advanced, Expert)
        let quality = if item_name.contains("masterwork") || item_name.contains("master") || item_name.contains("expert") {
            Quality::Expert
        } else if item_name.contains("excellent") || item_name.contains("fine") || item_name.contains("advanced") {
            Quality::Advanced
        } else if item_name.contains("good") || item_name.contains("quality") || item_name.contains("moderate") {
            Quality::Moderate
        } else if item_name.contains("poor") || item_name.contains("crude") {
            Quality::Crude
        } else if item_name.contains("pathetic") || item_name.contains("terrible") {
            Quality::Pathetic
        } else {
            Quality::Basic
        };

        // Parse material
        let material = if is_tool_or_weapon {
            // Tools and weapons use metal or wood
            if item_name.contains("steel") {
                EquipmentMaterial::Metal(MetalMaterial::Steel)
            } else if item_name.contains("iron") {
                EquipmentMaterial::Metal(MetalMaterial::Iron)
            } else if item_name.contains("bronze") {
                EquipmentMaterial::Metal(MetalMaterial::Bronze)
            } else if item_name.contains("copper") {
                EquipmentMaterial::Metal(MetalMaterial::Copper)
            } else if item_name.contains("stone") {
                EquipmentMaterial::Stone(StoneMaterial::Flint)
            } else if item_name.contains("oak") {
                EquipmentMaterial::Wood(WoodMaterial::Oak)
            } else if item_name.contains("birch") {
                EquipmentMaterial::Wood(WoodMaterial::Birch)
            } else if item_name.contains("pine") {
                EquipmentMaterial::Wood(WoodMaterial::Pine)
            } else if item_name.contains("yew") {
                EquipmentMaterial::Wood(WoodMaterial::Yew)
            } else if item_name.contains("wood") {
                EquipmentMaterial::Wood(WoodMaterial::Oak)
            } else {
                // Default to wood for basic tools
                EquipmentMaterial::Wood(WoodMaterial::Oak)
            }
        } else {
            // Armor and clothing
            if item_name.contains("leather") {
                EquipmentMaterial::Cloth(ClothingMaterial::Leather)
            } else if item_name.contains("fur") {
                EquipmentMaterial::Cloth(ClothingMaterial::Fur)
            } else if item_name.contains("wool") {
                EquipmentMaterial::Cloth(ClothingMaterial::Wool)
            } else if item_name.contains("linen") {
                EquipmentMaterial::Cloth(ClothingMaterial::Linen)
            } else if item_name.contains("cotton") {
                EquipmentMaterial::Cloth(ClothingMaterial::Cotton)
            } else if item_name.contains("hide") {
                EquipmentMaterial::Cloth(ClothingMaterial::Hide)
            } else {
                EquipmentMaterial::Cloth(ClothingMaterial::Linen)
            }
        };

        (material, quality)
    }

    /// Unequip an item from a slot and put it in inventory
    pub fn unequip_to_inventory(&mut self, slot: super::equipment::EquipmentSlot) -> Result<(), String> {
        match self.equipment.unequip(slot) {
            Some(item) => {
                // Add to inventory
                if self.inventory.add_item(InventoryItem::new(item.name.clone(), 1)) {
                    Ok(())
                } else {
                    // Inventory full, re-equip the item
                    self.equipment.equip(item).ok();
                    Err("Inventory full, cannot unequip".to_string())
                }
            }
            None => Err(format!("No item equipped in slot {:?}", slot)),
        }
    }

    /// Get reference to currently equipped item in a slot
    pub fn get_equipped(&self, slot: super::equipment::EquipmentSlot) -> Option<&super::equipment::EquipmentItem> {
        self.equipment.get_equipped(slot)
    }

    /// Check if a specific slot is occupied
    pub fn is_slot_equipped(&self, slot: super::equipment::EquipmentSlot) -> bool {
        self.equipment.get_equipped(slot).is_some()
    }

    /// Get total armor rating from all equipped armor
    pub fn get_total_armor(&self) -> f32 {
        self.equipment.total_armor()
    }

    /// Get total cold insulation from all equipped clothing
    pub fn get_total_cold_insulation(&self) -> f32 {
        self.equipment.total_cold_insulation()
    }

    /// Get total heat resistance from all equipped clothing
    pub fn get_total_heat_resistance(&self) -> f32 {
        self.equipment.total_heat_resistance()
    }

    /// Get attack damage bonus from equipped weapons
    pub fn get_weapon_damage(&self) -> f32 {
        self.equipment.weapon_damage()
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

    /// Get encumbrance penalty (0.0 = no penalty, 1.0 = fully encumbered)
    pub fn get_encumbrance_penalty(&self) -> f32 {
        self.equipment.encumbrance_penalty()
    }

    /// Get movement speed multiplier based on equipment weight
    pub fn get_movement_speed_multiplier(&self) -> f32 {
        self.equipment.movement_speed_multiplier()
    }

    /// Get all equipped items
    pub fn get_all_equipped(&self) -> Vec<&super::equipment::EquipmentItem> {
        self.equipment.get_all_equipped()
    }

    /// Get mining speed bonus from equipped tools
    pub fn get_mining_speed_bonus(&self) -> f32 {
        self.equipment.mining_speed_bonus()
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

        let food_data = match food_data {
            Some(data) => data,
            None => {
                // Not a tracked food item - consume 1 with flat nutrition
                self.inventory.remove_item(item_id, 1);
                let flat_nutrition = NutritionalContent::new(20.0, 5.0, 5.0, 0.3);
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
            let damage = 10.0;
            self.state.health = (self.state.health - damage).max(0.0);
            return EatResult::MadeSick(damage);
        }

        // Check if food is spoiled (inedible)
        if food_data.is_spoiled() {
            return EatResult::Spoiled;
        }

        // Consume the food
        self.inventory.remove_item(item_id, 1);

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
    pub fn has_edible_food(&self) -> bool {
        if self.find_best_food_to_eat().is_some() {
            return true;
        }

        // Untracked stacks have no freshness to judge, so they are always safe
        self.inventory
            .get_item("food")
            .map(|item| item.quantity > 0 && item.food_data.is_none())
            .unwrap_or(false)
    }

    /// Find the best food item to eat based on nutritional needs and freshness
    pub fn find_best_food_to_eat(&self) -> Option<String> {
        let needed = self.nutrition.most_needed_nutrient();

        let mut best_item: Option<(String, f32)> = None;

        for (item_id, item) in &self.inventory.items {
            // Skip emptied stacks - an exhausted entry lingering in the
            // inventory would otherwise read as "still carrying food"
            if item.quantity == 0 {
                continue;
            }

            if let Some(ref food_data) = item.food_data {
                // Skip anything that would make the agent sick. Raw food turns
                // harmful before it counts as spoiled, so checking spoilage
                // alone leaves agents eating rot: ten health a bite, one bite
                // a tick, until the stack or the agent runs out.
                if food_data.is_spoiled() || food_data.is_harmful() {
                    continue;
                }

                let nutrition = food_data.effective_nutrition();

                // Score based on what we need most
                let score = match needed {
                    crate::world::NutrientType::Energy => nutrition.energy,
                    crate::world::NutrientType::Protein => nutrition.protein,
                    crate::world::NutrientType::Micronutrients => nutrition.micronutrients,
                };

                // Prefer fresher food (multiply by freshness)
                let adjusted_score = score * food_data.freshness;

                if best_item.is_none() || adjusted_score > best_item.as_ref().unwrap().1 {
                    best_item = Some((item_id.clone(), adjusted_score));
                }
            }
        }

        best_item.map(|(id, _)| id)
    }

    /// Get summary of nutritional status
    pub fn nutrition_status(&self) -> String {
        self.nutrition.status_string()
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

    /// Get current fatigue level (0.0 to 1.0)
    pub fn fatigue_level(&self) -> f32 {
        self.fatigue.level
    }

    /// Get fatigue severity description
    pub fn fatigue_description(&self) -> &'static str {
        self.fatigue.description()
    }

    /// Consume energy from activity
    pub fn consume_energy(&mut self, amount: f32) {
        self.state.energy = (self.state.energy - amount).max(0.0);

        // When energy is depleted, health starts decreasing
        if self.state.energy <= 0.0 {
            self.state.health = (self.state.health - 0.05).max(0.0);
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
    pub fn update_starvation(&mut self) {
        self.state.ticks_without_food += 1;
    }

    /// Apply damage from starvation
    pub fn apply_starvation_damage(&mut self) {
        // Damage is already applied in age_tick, but this is for explicit calls
        if self.state.is_starving() {
            let days_starving = self.state.ticks_without_food / 1440;
            let damage = (days_starving as f32) * 0.5;
            self.state.health = (self.state.health - damage).max(0.0);

            if self.state.health <= 0.0 {
                self.state.is_alive = false;
            }
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

    /// Refresh storage knowledge (called when agent inspects/accesses storage)
    /// Satisfies curiosity and grants happiness to Curious trait holders
    pub fn refresh_storage_knowledge(&mut self, storage_position: (i32, i32, i32), _current_tick: u32) {
        use super::EmotionSource;
        use crate::core::memory::SpatialMemoryType;

        // Update the memory
        self.memory.remember_location(SpatialMemoryType::Storage, storage_position);

        // Satisfy curiosity about this location
        self.emotions.set_curiosity(EmotionSource::Location(storage_position), 0.0);

        // Curious trait holders gain happiness from learning/discovering
        if self.traits.has(crate::core::Trait::Curious) {
            self.emotions.add_happiness_with_traits(
                EmotionSource::Event("satisfied curiosity".to_string()),
                0.15, // Moderate happiness boost
                &self.traits
            );
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
        let mut worst: f32 = 0.0;

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
                Some(left) => (Self::A_LONG_WAY_OFF / left.max(1.0)).clamp(0.0, 1.0),
                None => continue,
            };

            let how_long = (drive.denied_ticks() as f32 / Self::LONG_ENOUGH_TO_FRIGHTEN)
                .clamp(0.0, 1.0);

            worst = worst.max(stakes * how_long);
        }

        worst.min(1.0)
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

    /// Get the next action from the current plan, resolving social targets
    ///
    /// Like get_plan_action but resolves nil UUIDs in social actions to actual
    /// nearby agents. The nearby_agents list should contain (id, position) pairs
    /// for all agents that could be interacted with.
    pub fn get_plan_action_with_nearby(
        &self,
        nearby_agents: &[(uuid::Uuid, (i32, i32, i32))],
    ) -> Option<Action> {
        let plan = self.current_plan.as_ref()?;
        let step = plan.current_step()?;

        self.convert_plan_step_to_action(step, nearby_agents)
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
                            let roll: f32 = rand::thread_rng().gen();

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
                // Lie detected - penalize relationship and trust
                let rel = self.relationships.get_or_create_relationship(source_id, current_tick);
                rel.weaken(0.15); // Significant relationship damage

                // Add negative emotion
                self.emotions.add_anger(
                    EmotionSource::Agent(source_id),
                    0.2 // Anger at being lied to
                );

                // Trait-based response to being lied to
                if self.traits.has(crate::core::traits::Trait::Vengeful) {
                    // Vengeful agents remember and hold grudges
                    rel.weaken(0.1); // Extra relationship damage
                }

                if self.traits.has(crate::core::traits::Trait::Forgiving) {
                    // Forgiving agents don't hold it against them as much
                    rel.strengthen(0.05); // Partial forgiveness
                }
            } else {
                // Truth verified - strengthen trust and relationship
                let rel = self.relationships.get_or_create_relationship(source_id, current_tick);
                rel.strengthen(0.05); // Small positive reinforcement

                // Small happiness from receiving accurate information
                self.emotions.add_happiness(
                    EmotionSource::Agent(source_id),
                    0.02
                );
            }
        }
    }

    /// Handle receiving information from another agent with lie detection
    /// This wraps the knowledge base receive and adds immediate verification
    pub fn receive_information_with_verification(
        &mut self,
        info: super::gossip::Information,
        source: uuid::Uuid,
        current_tick: u32,
    ) {
        use super::gossip::InformationType;

        let info_id = info.id;
        let info_type = info.info_type.clone();
        let _ground_truth = info.ground_truth;

        // Receive the information normally
        self.knowledge.receive_information(
            info,
            source,
            self.id,
            &self.traits,
            current_tick as u64,
        );

        // Immediate verification attempt for resource claims
        if let InformationType::ResourceLocation { resource, location } = &info_type {
            if let Some(is_correct) = self.verify_resource_claim(resource, *location) {
                // We can immediately verify this claim
                let _detection_bonus = self.get_lie_detection_bonus();

                if !is_correct {
                    // They lied about a resource location we know about!
                    self.on_lie_detected(source, &info_id, current_tick);
                } else {
                    // Verified correct
                    self.on_truth_verified(source, &info_id, current_tick);
                }
            } else if self.traits.has(crate::core::traits::Trait::Suspicious) {
                // Suspicious agents are wary of unverifiable claims
                // Slightly reduce confidence in the belief
                if let Some(belief) = self.knowledge.beliefs.iter_mut().find(|b| b.info_id == info_id) {
                    belief.confidence *= 0.9;
                }
            }
        }
    }

    /// Called when a lie is detected from a source
    fn on_lie_detected(&mut self, source: uuid::Uuid, info_id: &uuid::Uuid, current_tick: u32) {
        use super::EmotionSource;

        // Update knowledge trust
        self.knowledge.verify_information(info_id, false);

        // Get relationship and apply penalty
        let rel = self.relationships.get_or_create_relationship(source, current_tick);

        // Calculate penalty based on relationship
        let base_penalty = 0.15;
        let penalty = if rel.bond_strength > 0.5 {
            // Betrayal by a friend hurts more
            base_penalty * 1.5
        } else if rel.bond_strength < -0.3 {
            // Expected from an enemy - less emotional impact
            base_penalty * 0.7
        } else {
            base_penalty
        };

        rel.weaken(penalty);
        rel.total_interactions += 1;
        rel.last_interaction_tick = current_tick;

        // Emotional response
        self.emotions.add_anger(
            EmotionSource::Agent(source),
            0.15
        );

        // Paranoid agents become extra suspicious
        if self.traits.has(crate::core::traits::Trait::Paranoid) {
            self.emotions.add_fear(
                EmotionSource::Agent(source),
                0.1 // Fear of further deception
            );
        }

        // Trusting agents feel hurt/sad when lied to
        if self.traits.has(crate::core::traits::Trait::Trusting) {
            self.emotions.add_sadness(
                EmotionSource::Agent(source),
                0.1
            );
        }
    }

    /// Called when truth is verified from a source
    fn on_truth_verified(&mut self, source: uuid::Uuid, info_id: &uuid::Uuid, current_tick: u32) {
        use super::EmotionSource;

        // Update knowledge trust
        self.knowledge.verify_information(info_id, true);

        // Strengthen relationship slightly
        let rel = self.relationships.get_or_create_relationship(source, current_tick);
        rel.strengthen(0.03);
        rel.total_interactions += 1;
        rel.last_interaction_tick = current_tick;

        // Small happiness from accurate information
        self.emotions.add_happiness(
            EmotionSource::Agent(source),
            0.01
        );
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

        let roll: f32 = rand::thread_rng().gen();
        roll < lie_chance.clamp(0.0, 0.8) // Max 80% chance to lie
    }

    /// Create information to share, potentially distorting based on traits
    /// Returns the information (possibly distorted) and whether it's a lie
    pub fn prepare_information_to_share(
        &self,
        info: super::gossip::Information,
        target_id: uuid::Uuid,
        current_tick: u32,
    ) -> (super::gossip::Information, bool) {
        // Check if we would lie to this target
        if self.would_lie_to(target_id, current_tick) {
            // Apply distortion based on traits
            if let Some(distortion_trait) = self.traits.would_distort_info() {
                let distorted = info.distort(distortion_trait, self.id);
                return (distorted, true);
            }
        }

        // No lying - share truthfully
        (info, false)
    }

    /// Spread reputation damage when caught lying (gossip about the liar)
    /// Other agents who witnessed or heard about the lie will also lose trust
    pub fn spread_liar_reputation(
        &mut self,
        liar_id: uuid::Uuid,
        _witness_ids: &[uuid::Uuid],
        current_tick: u32,
    ) {
        // Create gossip information about the lie
        let gossip_info = super::gossip::Information::new(
            super::gossip::InformationType::AgentTrait {
                agent: liar_id,
                trait_name: "dishonest".to_string(),
            },
            self.id,
            true, // This is true - they did lie
            current_tick as u64,
        );

        // Store this information in our knowledge
        self.knowledge.known_information.insert(gossip_info.id, gossip_info);

        // Witnesses also get reputation update (handled by population system)
        // This method just marks that we're spreading the word
    }
}
