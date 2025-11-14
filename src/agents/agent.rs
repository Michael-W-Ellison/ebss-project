// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, DriveState, Memory};
use std::collections::HashMap;

use super::senses::Senses;
use super::body::Body;
use super::skills::Skills;
use super::emotions::{EmotionState, RelationshipMap};
use super::traits::TraitSet;
use super::gossip::KnowledgeBase;
use super::observational_learning::ObservationalLearning;
use super::transport::TransportSystem;
use crate::environment::TechnologyKnowledge;

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
/// Life stages of an agent
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
        }
    }

    /// Age the agent by one tick
    pub fn age_tick(&mut self, current_tick: u32) {
        if !self.is_alive {
            return;
        }

        self.age += 1;
        self.life_stage = LifeStage::from_age(self.age);

        // === SURVIVAL MECHANICS ===
        // Track starvation
        self.ticks_without_food = current_tick.saturating_sub(self.last_ate_tick);

        // Energy depletion (normal metabolism)
        let base_energy_loss = 0.05; // Base energy loss per tick
        let mut energy_loss = base_energy_loss;

        // After 24 hours (1440 ticks) without food: energy depletes faster
        if self.ticks_without_food > 1440 {
            energy_loss *= 2.0; // 2x faster energy depletion
        }

        // After 3 days (4320 ticks) without food: health starts decreasing
        if self.ticks_without_food > 4320 {
            let health_loss = 0.1; // Slow health degradation
            self.health = (self.health - health_loss).max(0.0);
        }

        // After 7 days (10080 ticks) without food: rapid health loss (death imminent)
        if self.ticks_without_food > 10080 {
            let severe_health_loss = 1.0; // Rapid health loss
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

        // Check for death from injury/starvation
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
        self.last_ate_tick = current_tick;
        self.ticks_without_food = 0;
    }

    /// Check if agent is starving (critical survival state)
    pub fn is_starving(&self) -> bool {
        self.ticks_without_food > 1440 || self.energy < 20.0
    }

    /// Check if agent is in critical survival state
    pub fn is_survival_critical(&self) -> bool {
        self.is_starving() || self.health < 30.0
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
    pub body: Body,
    pub body_temperature: super::BodyTemperature,
    pub skills: Skills,
    pub emotions: EmotionState,
    pub relationships: RelationshipMap,
    pub traits: TraitSet,
    pub knowledge: KnowledgeBase,
    pub observational_learning: ObservationalLearning,
    pub transport: TransportSystem,
    pub technology_knowledge: TechnologyKnowledge,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
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
            body: Body::default(),
            body_temperature: super::BodyTemperature::default(),
            skills: Skills::default(),
            emotions: EmotionState::default(),
            relationships: RelationshipMap::default(),
            traits: TraitSet::default(),
            knowledge: KnowledgeBase::default(),
            observational_learning: ObservationalLearning::default(),
            transport: TransportSystem::default(),
            technology_knowledge: TechnologyKnowledge::default(),
        }
    }

    /// Update agent state (tick senses, body, emotions, and memory)
    pub fn tick(&mut self) {
        self.senses.tick();
        self.body.tick();
        self.emotions.tick();
        self.memory.tick();

        // Sync body health to agent state
        self.state.health = self.body.overall_health() * 100.0;
    }

    /// Update body temperature based on environmental conditions
    ///
    /// # Arguments
    /// * `climate` - Environmental climate conditions
    pub fn update_temperature(&mut self, climate: &super::Climate) {
        let cold_insulation = self.body.total_cold_insulation();
        let heat_resistance = self.body.total_heat_resistance();
        let effective_temp = climate.effective_temperature();

        self.body_temperature.update(effective_temp, cold_insulation, heat_resistance);
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
            parent_ids: Vec::new(),
            emotions: EmotionalState::new(),
            traits: TraitSet::generate_random(3), // 3 random traits
            goals: GoalManager::default(),
            preferences: Preferences::generate_random(),
            inventory: Inventory::new(20), // Can carry up to 20 items
            knowledge: PersonalKnowledge::new(),
            social_network: SocialNetwork::new(),
            profession: Profession::default(), // Starts unemployed
            wealth: 100, // Starting currency
            known_technologies: crate::world::KnownTechnologies::new(), // Starts with fire and basic shelter
        }
    }

    /// Create a new agent with specified parent IDs (for reproduction)
    pub fn with_parents(config: AgentConfig, parent_ids: Vec<Uuid>, current_tick: u32) -> Self {
        let mut agent = Self::new(config);
        agent.parent_ids = parent_ids.clone();

        // Add parent relationships (starts at Likes +3)
        for parent_id in parent_ids {
            agent.social_network.add_parent_relationship(parent_id, current_tick);
        }

        agent
    }

    /// Update agent for one tick (backward compatibility - assumes current_tick = age)
    pub fn tick(&mut self) {
        self.tick_with_time(self.state.age);
    }

    /// Update agent for one tick with current simulation time
    pub fn tick_with_time(&mut self, current_tick: u32) {
        if !self.state.is_alive {
            return;
        }

        // Age the agent with survival mechanics
        self.state.age_tick(current_tick);

        // Tick drives normally (accumulate over time)
        self.drives.tick();

        // Apply survival-based urgency adjustments
        self.apply_survival_urgency();

        // Update memory
        self.memory.tick();

        // Update emotions (natural decay)
        self.emotions.tick();

        // Update knowledge (age tracking)
        self.knowledge.tick(current_tick);

        // Decay relationships towards neutral over time (every 10 ticks)
        if current_tick % 10 == 0 {
            self.social_network.decay_all_relationships(current_tick, 0.1);
        }

        // Cleanup completed goals
        self.goals.cleanup_completed();
    }

    /// Apply survival-based urgency adjustments to drives
    /// When survival is threatened, basic needs must override all other drives
    /// NOTE: This does NOT tick drives - that should be done separately
    fn apply_survival_urgency(&mut self) {
        // Apply survival urgency overrides when in critical state
        if self.state.is_survival_critical() {
            // CRITICAL: When survival is threatened, basic needs must come first

            // Massively boost hunger drive if starving
            if self.state.is_starving() {
                if let Some(hunger_drive) = self.drives.get_mut(crate::core::DriveType::Hunger) {
                    // Set hunger to maximum urgency when starving
                    hunger_drive.value = 1.0; // Maximum urgency

                    // Increase weight even more based on how long without food
                    let starvation_multiplier = if self.state.ticks_without_food > 10080 {
                        3.0 // 7+ days: CRITICAL
                    } else if self.state.ticks_without_food > 4320 {
                        2.5 // 3+ days: SEVERE
                    } else if self.state.ticks_without_food > 1440 {
                        2.0 // 1+ day: HIGH
                    } else {
                        1.5 // Energy low: MODERATE
                    };

                    hunger_drive.weight = hunger_drive.weight * starvation_multiplier;
                }
            }

            // Boost safety/shelter if health is critical
            if self.state.health < 30.0 {
                if let Some(safety_drive) = self.drives.get_mut(crate::core::DriveType::Safety) {
                    safety_drive.value = (safety_drive.value + 0.5).min(1.0);
                    safety_drive.weight *= 1.5;
                }

                if let Some(shelter_drive) = self.drives.get_mut(crate::core::DriveType::Shelter) {
                    shelter_drive.value = (shelter_drive.value + 0.3).min(1.0);
                    shelter_drive.weight *= 1.3;
                }
            }

            // SUPPRESS non-survival drives during critical situations
            // Agents should NOT reproduce or build luxury items while starving
            let non_critical_drives = [
                crate::core::DriveType::Reproduction,
                crate::core::DriveType::Luxury,
                crate::core::DriveType::Curiosity,
                crate::core::DriveType::Construction,
            ];

            for drive_type in non_critical_drives.iter() {
                if let Some(drive) = self.drives.get_mut(*drive_type) {
                    // Reduce weight to near zero during survival crisis
                    drive.weight *= 0.1;
                }
            }
        }
    }

    /// Check if agent can reproduce
    pub fn can_reproduce(&self) -> bool {
        // Cannot reproduce if in survival-critical state
        if self.state.is_survival_critical() {
            return false;
        }

        self.state.is_alive && self.state.life_stage.can_reproduce()
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

        // Modified by reproduction drive
        let reproduction_drive = self.drives.get(crate::core::DriveType::Reproduction)
            .map(|d| d.value)
            .unwrap_or(0.0);

        base_fertility * health_factor * (0.5 + reproduction_drive * 0.5)
    }

    /// Try to eat food from inventory to restore energy
    /// Returns true if agent successfully ate
    pub fn try_eat(&mut self, current_tick: u32) -> bool {
        // Check if agent should eat
        // Eat when: energy < 90% OR hunger drive is active (>= threshold)
        let hunger_active = self.drives.get(crate::core::DriveType::Hunger)
            .map(|d| d.is_active())
            .unwrap_or(false);

        let should_eat = self.state.energy < 90.0 || hunger_active;

        if !should_eat {
            return false; // Not hungry
        }

        // Try to consume food from inventory
        if self.inventory.has_item(&ItemType::Food, 1) {
            if self.inventory.remove_item(&ItemType::Food, 1) {
                // Restore energy (1 food = 25 energy)
                self.state.eat(current_tick, 25.0);

                // Satisfy hunger drive
                if let Some(hunger_drive) = self.drives.get_mut(crate::core::DriveType::Hunger) {
                    hunger_drive.satisfy();
                }

                // Trigger positive emotion
                if let Some(happiness) = self.emotions.get_mut(crate::core::EmotionType::Happiness) {
                    happiness.increase(0.3);
                }

                return true;
            }
        }

        false
    }

    /// Check if agent should prioritize gathering food
    pub fn needs_food(&self) -> bool {
        // Need food if:
        // 1. Energy is low (< 50%)
        // 2. No food in inventory
        // 3. Hunger drive is active
        self.state.energy < 50.0 ||
        (self.inventory.count_item(&ItemType::Food) == 0 &&
         self.drives.get(crate::core::DriveType::Hunger).map(|d| d.is_active()).unwrap_or(false))
    }

    // ===== COMMUNICATION METHODS =====

    /// Observe a resource (personal discovery)
    pub fn observe_resource(&mut self, position: Position, resource_type: ResourceType, amount: u32) {
        self.knowledge.observe_resource(position, resource_type, amount);
    }

    /// Request information about a specific resource type from another agent
    /// Returns information if the other agent knows about it
    /// Returns: (position, resource_type, amount, learned_tick)
    pub fn request_info_from(
        &mut self,
        other_agent: &Agent,
        resource_type: ResourceType,
    ) -> Option<(Position, ResourceType, u32, u32)> {
        // Other agent shares their best knowledge about this resource type
        other_agent.knowledge.get_shareable_info(resource_type)
    }

    /// Share knowledge with another agent (direct communication)
    pub fn share_knowledge_with(
        &self,
        other_agent: &mut Agent,
        resource_type: ResourceType,
    ) -> bool {
        if let Some((position, res_type, amount, _learned_tick)) = self.knowledge.get_shareable_info(resource_type) {
            // Other agent learns from us
            other_agent.knowledge.learn_from_agent(position, res_type, amount, self.id);
            true
        } else {
            false // We don't have information to share
        }
    }

    /// Overhear conversation between two agents about a resource
    pub fn overhear_conversation(
        &mut self,
        speaker_id: Uuid,
        position: Position,
        resource_type: ResourceType,
        amount: u32,
    ) {
        self.knowledge.overhear_information(position, resource_type, amount, speaker_id);
    }

    /// Get position of agent for proximity checks
    pub fn position(&self) -> Position {
        Position::new(self.state.position.0, self.state.position.1)
    }

    /// Check if another agent is within communication range
    pub fn can_communicate_with(&self, other_agent: &Agent, communication_range: u32) -> bool {
        self.position().distance_to(&other_agent.position()) <= communication_range
    }

    /// Find the resource type the agent is most interested in based on current drives
    pub fn most_desired_resource(&self) -> Option<ResourceType> {
        let most_urgent = self.drives.most_urgent()?;

        match most_urgent.drive_type {
            crate::core::DriveType::Hunger => Some(ResourceType::Food),
            crate::core::DriveType::Construction | crate::core::DriveType::Shelter => Some(ResourceType::Wood),
            crate::core::DriveType::Industry => Some(ResourceType::Iron),
            crate::core::DriveType::Preparedness => Some(ResourceType::Stone),
            _ => None,
        }
    }

    // ===== RELATIONSHIP & TRUST METHODS =====

    /// Verify that information received from another agent was correct
    /// This increases trust with that agent
    pub fn verify_information_from(
        &mut self,
        source_agent_id: Uuid,
        info_age_ticks: u32,
        current_tick: u32,
    ) {
        let relationship = self.social_network.get_or_create_relationship(source_agent_id, current_tick);
        relationship.verify_information(info_age_ticks, current_tick);
    }

    /// Record that information from another agent was incorrect
    /// This decreases trust with that agent
    pub fn information_was_wrong_from(
        &mut self,
        source_agent_id: Uuid,
        info_age_ticks: u32,
        current_tick: u32,
    ) {
        let relationship = self.social_network.get_or_create_relationship(source_agent_id, current_tick);
        relationship.incorrect_information(info_age_ticks, current_tick);
    }

    /// Record a positive social interaction
    pub fn positive_interaction_with(&mut self, other_agent_id: Uuid, strength: i8, current_tick: u32) {
        let relationship = self.social_network.get_or_create_relationship(other_agent_id, current_tick);
        relationship.positive_interaction(strength, current_tick);
    }

    /// Record a negative social interaction
    pub fn negative_interaction_with(&mut self, other_agent_id: Uuid, strength: i8, current_tick: u32) {
        let relationship = self.social_network.get_or_create_relationship(other_agent_id, current_tick);
        relationship.negative_interaction(strength, current_tick);
    }

    /// Get how much to believe information from a specific agent (0.0 to 1.0)
    pub fn trust_factor_for(&self, agent_id: Uuid) -> f32 {
        self.social_network.belief_weight_for(agent_id)
    }

    /// Decide which source to believe when receiving conflicting information
    /// Returns true if should believe source A, false if should believe source B
    pub fn choose_between_sources(&self, source_a: Uuid, source_b: Uuid) -> bool {
        let trust_a = self.trust_factor_for(source_a);
        let trust_b = self.trust_factor_for(source_b);

        // Believe the more trusted source
        trust_a >= trust_b
    }

    // === Profession Methods ===

    /// Assign a new profession to this agent
    pub fn assign_profession(&mut self, job: crate::agents::JobType) {
        self.profession = Profession::new(job);
    }

    /// Assign profession with specific skill level
    pub fn assign_profession_with_skill(&mut self, job: crate::agents::JobType, skill_level: u8) {
        self.profession = Profession::with_skill(job, skill_level);
    }

    /// Assign agent to a workplace building
    pub fn assign_to_workplace(&mut self, position: Position, building_id: Uuid) {
        self.profession.assign_workplace(position, building_id);
    }

    /// Remove agent from their workplace
    pub fn remove_from_workplace(&mut self) {
        self.profession.remove_workplace();
    }

    /// Agent gains work experience
    pub fn gain_work_experience(&mut self, amount: u16) {
        self.profession.gain_experience(amount);
    }

    /// Agent produces items, gaining experience
    pub fn produce_items(&mut self, quantity: u32) {
        self.profession.record_production(quantity);
    }

    /// Check if agent is employed
    pub fn is_employed(&self) -> bool {
        !matches!(self.profession.job, crate::agents::JobType::Unemployed)
    }

    /// Check if agent has a workplace assigned
    pub fn has_workplace(&self) -> bool {
        self.profession.workplace.is_some()
    }

    /// Get agent's profession description
    pub fn profession_description(&self) -> String {
        format!(
            "{} ({})",
            self.profession.job.description(),
            self.profession.skill_description()
        )
    }

    // === Production Methods ===

    /// Start crafting a recipe (by index from available recipes)
    pub fn start_crafting(&mut self, recipe_index: usize) -> bool {
        use crate::world::get_job_recipes;

        let recipes = get_job_recipes(self.profession.job);
        if recipe_index < recipes.len() {
            self.profession.start_production(recipe_index);
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

    /// Set age-based learning rate (child = 1.5, adult = 1.0, elder = 0.7)
    pub fn set_learning_rate(&mut self, rate: f32) {
        self.observational_learning.set_learning_rate(rate);
    }

    /// Get current learning rate
    pub fn learning_rate(&self) -> f32 {
        self.observational_learning.learning_rate()
    }

    /// Equip a transport (activate it)
    /// Updates inventory capacity automatically
    pub fn equip_transport(&mut self, transport_id: &Uuid) -> bool {
        if self.transport.activate(transport_id) {
            // Update inventory capacity
            self.update_inventory_capacity_from_transport();
    /// Tick production, returns completed items if any
    pub fn tick_production(&mut self) -> Option<Vec<(crate::world::ItemType, u32)>> {
        self.profession.tick_production()
    }

    /// Check if currently crafting
    pub fn is_crafting(&self) -> bool {
        self.profession.is_producing()
    }

    /// Cancel current crafting
    pub fn cancel_crafting(&mut self) {
        self.profession.cancel_production();
    }

    /// Get crafting progress (0-100%)
    pub fn crafting_progress(&self) -> u8 {
        self.profession.production_progress_percent()
    }

    /// Get available recipes for agent's job
    pub fn available_recipes(&self) -> Vec<crate::world::Recipe> {
        use crate::world::get_job_recipes;
        get_job_recipes(self.profession.job)
    }

    /// Get current recipe being worked on
    pub fn current_recipe(&self) -> Option<crate::world::Recipe> {
        self.profession.get_current_recipe()
    }

    // === Trading Methods ===

    /// Create a trade offer
    pub fn create_trade_offer(
        &self,
        offering: Vec<(ItemType, u32)>,
        requesting: Vec<(ItemType, u32)>,
        price: u32,
        current_tick: u32,
        duration: u32,
    ) -> Option<crate::world::TradeOffer> {
        // Check if agent has the items they're offering
        for (item, quantity) in &offering {
            if !self.inventory.has_item(item, *quantity) {
                return None; // Cannot create offer without items
            }
        }

        Some(crate::world::TradeOffer::new(
            self.id,
            offering,
            requesting,
            price,
            current_tick,
            duration,
        ))
    }

    /// Check if agent can afford a trade
    pub fn can_afford_trade(&self, offer: &crate::world::TradeOffer) -> bool {
        offer.can_afford(self.wealth)
    }

    /// Check if agent has requested items for a trade
    pub fn has_requested_items(&self, offer: &crate::world::TradeOffer) -> bool {
        for (item, quantity) in &offer.requesting {
            if !self.inventory.has_item(item, *quantity) {
                return false;
            }
        }
        true
    }

    /// Pay for something (returns true if successful)
    pub fn pay(&mut self, amount: u32) -> bool {
        if self.wealth >= amount {
            self.wealth -= amount;
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

    /// Get movement speed including transport penalties
    pub fn movement_speed(&self) -> f32 {
        let body_speed = self.body.movement_speed_multiplier();
        let transport_speed = self.transport.speed_modifier();
        let weight_penalty = if self.inventory.is_overweight() {
            0.5 // 50% speed when overweight
        } else {
            1.0 - (self.inventory.weight_percentage() * 0.3) // Up to 30% slower at max weight
        };

        body_speed * transport_speed * weight_penalty
    }

    /// Check if agent can carry additional weight
    pub fn can_carry(&self, additional_weight: f32) -> bool {
        self.inventory.current_weight + additional_weight <= self.inventory.max_weight
    }

    /// Get total carrying capacity (base + transport)
    pub fn total_carrying_capacity(&self) -> f32 {
        self.inventory.max_weight
    }
}

// Need to import EmotionSource at top
use super::emotions::EmotionSource;
    /// Receive payment
    pub fn receive_payment(&mut self, amount: u32) {
        self.wealth += amount;
    }

    /// Get agent's wealth
    pub fn get_wealth(&self) -> u32 {
        self.wealth
    }

    /// Check if agent wants to buy an item (based on needs and profession)
    pub fn wants_to_buy(&self, item: ItemType) -> bool {
        // Always want food if low on energy
        if item == ItemType::Food && self.state.energy < 50.0 {
            return true;
        }

        // Want items related to profession
        match self.profession.job {
            crate::agents::JobType::Baker => {
                matches!(item, ItemType::Flour | ItemType::Grain)
            }
            crate::agents::JobType::Carpenter => {
                matches!(item, ItemType::Wood)
            }
            crate::agents::JobType::Blacksmith => {
                matches!(item, ItemType::Iron | ItemType::Coal | ItemType::Charcoal)
            }
            crate::agents::JobType::Tailor => {
                matches!(item, ItemType::Cloth | ItemType::Linen)
            }
            crate::agents::JobType::Cobbler => {
                matches!(item, ItemType::Leather)
            }
            _ => false,
        }
    }

    /// Check if agent wants to sell an item
    pub fn wants_to_sell(&self, item: ItemType) -> bool {
        // Don't sell food if energy is low
        if item == ItemType::Food && self.state.energy < 70.0 {
            return false;
        }

        // Sell items not related to profession if inventory is getting full
        if self.inventory.items.len() >= 15 {
            return !self.wants_to_buy(item);
        }

        false
    }

    /// Determine fair price for buying an item (based on agent's valuation)
    pub fn valuation_for_item(&self, item: ItemType, market_price: u32) -> u32 {
        let mut value = market_price;

        // Increase valuation if agent needs it
        if self.wants_to_buy(item) {
            value = (value as f32 * 1.3).round() as u32;
        }

        // Decrease valuation if agent doesn't need it
        if self.wants_to_sell(item) {
            value = (value as f32 * 0.8).round() as u32;
        }

        // Food is more valuable when starving
        if item == ItemType::Food && self.state.is_starving() {
            value = (value as f32 * 2.0).round() as u32;
        }

        value.max(1)
    }

    // === Technology Discovery Methods ===

    /// Attempt to experiment and discover a new technology
    pub fn attempt_discovery(
        &mut self,
        tech_id: &str,
        tech_tree: &crate::world::TechnologyTree,
    ) -> DiscoveryResult {
        use crate::core::Trait;

        // Get the technology
        let tech = match tech_tree.get(tech_id) {
            Some(t) => t,
            None => return DiscoveryResult::InvalidTechnology,
        };

        // Check prerequisites
        for prereq in &tech.prerequisites {
            if !self.known_technologies.knows(prereq) {
                return DiscoveryResult::PrerequisitesNotMet;
            }
        }

        // Already known?
        if self.known_technologies.knows(tech_id) {
            return DiscoveryResult::AlreadyKnown;
        }

        // Check if agent has required items
        for item in &tech.required_items {
            if !self.inventory.has_item(item, 1) {
                return DiscoveryResult::MissingItems;
            }
        }

        // Get curiosity modifier based on trait
        let curiosity = if self.traits.has(Trait::Curious) {
            5 // Curious agents get +5 bonus
        } else {
            0 // Normal agents have no bonus
        };

        // Calculate discovery chance
        let chance = tech.discovery_chance(curiosity);

        // Roll for discovery
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let roll: f32 = rng.gen();

        if roll < chance {
            // Success! Full discovery
            self.known_technologies.learn(tech_id, true);

            // Consume resources (experimentation uses materials)
            for item in &tech.required_items {
                self.inventory.remove_item(item, 1);
            }

            DiscoveryResult::Discovered
        } else {
            // Partial progress
            let progress = rng.gen_range(5..20);

            // Consume some resources on failed attempts
            if rng.gen_bool(0.3) {
                for item in &tech.required_items {
                    self.inventory.remove_item(item, 1);
                }
            }

            if self.known_technologies.add_experimentation(tech_id, progress) {
                DiscoveryResult::Discovered
            } else {
                DiscoveryResult::ProgressMade(self.known_technologies.get_progress(tech_id))
            }
        }
    }

    /// Learn a technology from another agent (teaching)
    pub fn learn_from(&mut self, tech_id: &str, teacher_id: Uuid) -> bool {
        if self.known_technologies.knows(tech_id) {
            return false; // Already known
        }

        // Learn it (not discovered by self)
        self.known_technologies.learn(tech_id, false);

        // Create positive social interaction (teaching creates bond)
        self.positive_interaction_with(teacher_id, 2, 0);

        true
    }

    /// Check if agent can craft an item based on known technologies
    pub fn can_craft_tech(&self, item: ItemType, tech_tree: &crate::world::TechnologyTree) -> bool {
        self.known_technologies.can_craft(item, tech_tree)
    }

    /// Get all recipes agent can craft based on tech and profession
    pub fn get_available_recipes_tech(&self, tech_tree: &crate::world::TechnologyTree) -> Vec<crate::world::Recipe> {
        use crate::world::get_job_recipes;

        let job_recipes = get_job_recipes(self.profession.job);
        let craftable = self.known_technologies.get_craftable_items(tech_tree);

        // Filter recipes to only those with outputs the agent can craft
        job_recipes
            .into_iter()
            .filter(|recipe| {
                recipe.outputs.iter().all(|output| craftable.contains(&output.item_type))
            })
            .collect()
    }

    /// Get current technological era
    pub fn get_tech_era(&self, tech_tree: &crate::world::TechnologyTree) -> crate::world::TechEra {
        self.known_technologies.current_era(tech_tree)
    }

    /// Get list of discoverable technologies (prerequisites met but not known)
    pub fn get_discoverable_techs<'a>(&self, tech_tree: &'a crate::world::TechnologyTree) -> Vec<&'a crate::world::Technology> {
        tech_tree.all().into_iter()
            .filter(|tech| {
                // Not already known
                if self.known_technologies.knows(tech.id) {
                    return false;
                }

                // Prerequisites met
                tech.prerequisites.iter().all(|prereq| self.known_technologies.knows(prereq))
            })
            .collect()
    }

    /// Check if agent knows a specific technology
    pub fn knows_technology(&self, tech_id: &str) -> bool {
        self.known_technologies.knows(tech_id)
    }
}

/// Result of a technology discovery attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryResult {
    Discovered,                    // Successfully discovered!
    ProgressMade(u8),              // Made progress (0-100)
    AlreadyKnown,                  // Agent already knows this
    PrerequisitesNotMet,           // Missing prerequisite technologies
    MissingItems,                  // Don't have required items
    InvalidTechnology,             // Tech ID doesn't exist
}

