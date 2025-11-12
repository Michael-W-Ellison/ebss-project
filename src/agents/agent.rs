// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, DriveState, Memory};
use std::collections::HashMap;

use super::senses::Senses;
use super::body::Body;
use super::skills::Skills;
use super::emotions::{EmotionState, RelationshipMap};

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
}

impl InventoryItem {
    pub fn new(item_id: String, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            fill_level: None,
            max_capacity: None,
            current_durability: None,
            max_durability: None,
            quality: None,
        }
    }

    pub fn new_container(item_id: String, quantity: u32, capacity: f32) -> Self {
        Self {
            item_id,
            quantity,
            fill_level: Some(0.0),
            max_capacity: Some(capacity),
            current_durability: None,
            max_durability: None,
            quality: None,
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
            fill_level: None,
            max_capacity: None,
            current_durability: Some(durability),
            max_durability: Some(durability),
            quality: Some(quality),
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
    pub fn add_item(&mut self, item: InventoryItem) -> bool {
        if self.items.len() >= self.max_slots && !self.items.contains_key(&item.item_id) {
            return false; // No room for new item type
        }

        self.items.insert(item.item_id.clone(), item);
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
                    fill_level: item.fill_level,
                    max_capacity: item.max_capacity,
                    current_durability: item.current_durability,
                    max_durability: item.max_durability,
                    quality: item.quality,
                };

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

        amount - remaining // Return amount actually drunk
    }

    /// Fill containers from a water source
    pub fn fill_containers(&mut self, available_water: f32) -> f32 {
        let mut remaining = available_water;

        for item in self.items.values_mut() {
            if item.is_container() && remaining > 0.0 {
                let filled = item.fill(remaining);
                remaining -= filled;
            }
        }

        available_water - remaining // Return amount actually used
    }

    /// Check if inventory has item with minimum quantity
    pub fn has_item(&self, item_id: &str, min_quantity: u32) -> bool {
        self.items.get(item_id)
            .map(|item| item.quantity >= min_quantity)
            .unwrap_or(false)
    }

    /// Get all items
    pub fn get_all_items(&self) -> &HashMap<String, InventoryItem> {
        &self.items
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(20, 100.0) // Default: 20 slots, 100 weight units
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub health: f32,
    pub position: (i32, i32, i32),
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
    pub body: Body,
    pub skills: Skills,
    pub emotions: EmotionState,
    pub relationships: RelationshipMap,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: AgentState {
                health: 100.0,
                position: (0, 0, 0),
            },
            drives: if config.random_weights {
                DriveState::with_random_weights()
            } else {
                DriveState::new()
            },
            behavior_trees: Vec::new(),
            memory: Memory::new(),
            inventory: Inventory::default(),
            senses: Senses::default(),
            body: Body::default(),
            skills: Skills::default(),
            emotions: EmotionState::default(),
            relationships: RelationshipMap::default(),
        }
    }

    /// Update agent state (tick senses, body, and emotions)
    pub fn tick(&mut self) {
        self.senses.tick();
        self.body.tick();
        self.emotions.tick();

        // Sync body health to agent state
        self.state.health = self.body.overall_health() * 100.0;
    }

    /// Respond emotionally to a threat
    ///
    /// # Arguments
    /// * `threat_strength` - Strength of the threat (e.g., enemy agent's combat power)
    /// * `source` - Source of the threat
    ///
    /// Returns the emotional response triggered
    pub fn respond_to_threat(&mut self, threat_strength: f32, source: super::EmotionSource) -> super::EmotionType {
        use super::ThreatAssessment;

        // Calculate agent strength (simplified: health + body functionality)
        let agent_strength = self.state.health / 100.0 * self.body.movement_speed_multiplier();

        let assessment = ThreatAssessment::assess(agent_strength, threat_strength, source.clone());

        let emotion_type = assessment.emotion_type();
        let emotion_amount = assessment.emotion_amount();

        match emotion_type {
            super::EmotionType::Anger => {
                self.emotions.add_anger(source, emotion_amount);
            }
            super::EmotionType::Fear => {
                self.emotions.add_fear(source, emotion_amount);
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
                self.emotions.add_sadness(source.clone(), sadness_amount);

                // Also potentially add fear or anger based on agent's ability to protect
                // Parents protecting children might feel anger if they can fight back
                if relationship.relationship_type == super::RelationshipType::Child {
                    // Calculate if agent is strong enough to retaliate
                    let agent_strength = self.state.health / 100.0;

                    // Assume medium threat strength for the source
                    let assessment = super::ThreatAssessment::assess(agent_strength, 0.7, source.clone());

                    if assessment.can_overcome {
                        self.emotions.add_anger(source, 0.5);
                    } else {
                        self.emotions.add_fear(source, 0.3);
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
                self.emotions.add_sadness(EmotionSource::Agent(*deceased_id), sadness_amount);

                // Fear of the source that killed them
                self.emotions.add_fear(source, 0.4);
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
}

// Need to import EmotionSource at top
use super::emotions::EmotionSource;
