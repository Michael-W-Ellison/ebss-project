// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, BehaviorNode, NodeType, DriveState, DriveType, Memory, GoalManager, Preferences};
use crate::environment::{Action, ActionResult};
use std::collections::HashMap;

use super::senses::Senses;
use super::body::Body;
use super::skills::Skills;
use super::emotions::{EmotionState, EmotionSource, RelationshipMap};
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
    }
}

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
    /// Recent percepts processed from sensory input (last 20 ticks)
    pub recent_percepts: Vec<(u32, super::sensory_processing::Percept)>, // (tick, percept)
    pub body: Body,
    pub body_temperature: super::BodyTemperature,
    pub exposure_status: crate::environment::ExposureStatus,
    pub skills: Skills,
    pub emotions: EmotionState,
    pub relationships: RelationshipMap,
    pub social_network: super::relationships::SocialNetwork, // Social relationship and trust tracking
    pub traits: TraitSet,
    pub knowledge: KnowledgeBase,
    pub observational_learning: ObservationalLearning,
    pub transport: TransportSystem,
    pub technology_knowledge: TechnologyKnowledge,
    pub exploration_knowledge: super::exploration::ExplorationKnowledge, // Map discovery and exploration
    pub storage_preferences: super::storage_management::StoragePreferences, // Storage management preferences
    pub parent_ids: Vec<Uuid>,
    pub goals: GoalManager,
    pub preferences: Preferences,
    pub equipment: super::equipment::EquipmentManager, // Equipped items (weapons, armor, tools)
    pub satisfaction_tracker: super::drive_satisfaction::SatisfactionTracker, // Tracks who/what satisfies which drives
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
            recent_percepts: Vec::new(),
            body: Body::default(),
            body_temperature: super::BodyTemperature::default(),
            exposure_status: crate::environment::ExposureStatus::default(),
            skills: Skills::default(),
            emotions: EmotionState::default(),
            relationships: RelationshipMap::default(),
            social_network: super::relationships::SocialNetwork::default(),
            traits: TraitSet::default(),
            knowledge: KnowledgeBase::default(),
            observational_learning: ObservationalLearning::default(),
            transport: TransportSystem::default(),
            technology_knowledge: TechnologyKnowledge::default(),
            exploration_knowledge: super::exploration::ExplorationKnowledge::default(),
            storage_preferences: super::storage_management::StoragePreferences::default(),
            parent_ids: Vec::new(),
            goals: GoalManager::new(5), // Max 5 active goals
            preferences: Preferences::default(),
            equipment: super::equipment::EquipmentManager::new(50.0), // 50kg max carry weight
            satisfaction_tracker: super::drive_satisfaction::SatisfactionTracker::new(),
        }
    }

    /// Create an agent with specified parents
    pub fn with_parents(config: AgentConfig, parent_ids: Vec<Uuid>, current_tick: u32) -> Self {
        let mut agent = Self::new(config);
        agent.parent_ids = parent_ids;
        agent.state.last_ate_tick = current_tick;
        agent
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
        self.emotions.tick();
        self.memory.tick();

        // Update emotions based on drive states (every tick)
        self.update_emotions_from_drives();
        self.drives.tick();

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
                        self.memory.record_interaction(*agent_id, true, 0.01);
                    }
                    _ => {}
                }
            }

            self.recent_percepts.push((current_tick, percept));
        }

        // Trim old percepts (keep only last 20 ticks worth)
        self.recent_percepts.retain(|(tick, _)| current_tick.saturating_sub(*tick) <= 20);

        // Sync body health to agent state
        self.state.health = self.body.overall_health() * 100.0;

        // Update energy (basic metabolism)
        self.state.energy = (self.state.energy - 0.1).max(0.0);
    }

    /// Update agent with time progression (includes aging and survival mechanics)
    pub fn tick_with_time(&mut self, current_tick: u32) {
        // First do the regular tick
        self.tick();

        // Then handle aging and survival mechanics
        self.state.age_tick(current_tick);
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

    /// Check if agent can reproduce
    pub fn can_reproduce(&self) -> bool {
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

    /// Process feedback from action execution
    pub fn apply_feedback(&mut self, action_result: &ActionResult, drive_type: DriveType) {
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


    // Helper methods
    fn find_nearest_shelter(&self) -> (i32, i32, i32) {
        // Placeholder: return a position near the agent
        (self.state.position.0, self.state.position.1, self.state.position.2)
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

        // Create equipment item from inventory item
        // For now, we'll create a basic equipment item
        // In a full implementation, this would lookup item stats from a registry
        use super::equipment::{EquipmentType, EquipmentMaterial, WoodMaterial};
        use super::skills::Quality;

        let equipment_type = match slot {
            super::equipment::EquipmentSlot::MainHand | super::equipment::EquipmentSlot::OffHand => EquipmentType::Pickaxe,
            _ => EquipmentType::Clothing,
        };

        let equipment_item = super::equipment::EquipmentItem::new(
            item_id.to_string(),
            equipment_type,
            slot,
            EquipmentMaterial::Wood(WoodMaterial::Oak),
            Quality::Basic,
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
        use crate::world::ItemType;
        use log::debug;

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

    /// Eat food from inventory and satisfy hunger drive
    /// Returns true if food was consumed
    pub fn eat_food(&mut self, amount: u32) -> bool {
        // Try to get food from inventory
        if let Some(food_item) = self.inventory.get_item_mut("food") {
            if food_item.quantity >= amount {
                food_item.quantity -= amount;

                // Restore energy (each food unit restores 20 energy)
                let energy_restored = (amount as f32) * 20.0;
                self.state.energy = (self.state.energy + energy_restored).min(100.0);

                // Reset starvation (use current age as approximation of tick)
                self.state.last_ate_tick = self.state.age;
                self.state.ticks_without_food = 0;

                // Satisfy hunger drive
                let hunger_reduction = (amount as f32) * 0.2; // Each food reduces hunger by 0.2
                if let Some(hunger) = self.drives.get_mut(DriveType::Hunger) {
                    hunger.decrease(hunger_reduction);
                }

                return true;
            }
        }
        false
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

    /// Consume energy from activity
    pub fn consume_energy(&mut self, amount: f32) {
        self.state.energy = (self.state.energy - amount).max(0.0);

        // When energy is depleted, health starts decreasing
        if self.state.energy <= 0.0 {
            self.state.health = (self.state.health - 0.05).max(0.0);
        }
    }

    /// Take damage (wrapper for AgentState method)
    pub fn take_damage(&mut self, amount: f32) {
        self.state.take_damage(amount);
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

    /// Update life stage based on age
    pub fn update_life_stage(&mut self) {
        self.state.life_stage = LifeStage::from_age(self.state.age);
    }

    // ===== Drive-Emotion Feedback System =====

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

    /// Calculate fear from survival drive deprivation
    /// Multiple survival threats compound (with diminishing returns)
    fn calculate_survival_drive_emotion(&self) -> f32 {
        let mut total_fear = 0.0f32;
        let mut count = 0;

        // Check hunger
        if let Some(hunger) = self.drives.get(DriveType::Hunger) {
            if hunger.value > 0.7 {
                // Fear scales with severity above threshold
                let fear = (hunger.value - 0.7) / 0.3 * 0.7; // 0.0 to 0.7 (increased from 0.6)
                total_fear += fear;
                count += 1;
            }
        }

        // Check thirst (even more urgent)
        if let Some(thirst) = self.drives.get(DriveType::Thirst) {
            if thirst.value > 0.7 {
                let fear = (thirst.value - 0.7) / 0.3 * 0.8; // 0.0 to 0.8 (increased from 0.7)
                total_fear += fear;
                count += 1;
            }
        }

        // Check rest
        if let Some(rest) = self.drives.get(DriveType::Rest) {
            if rest.value >= 0.75 {
                let fear = (rest.value - 0.75) / 0.25 * 0.5; // 0.0 to 0.5 (increased from 0.4)
                total_fear += fear;
                count += 1;
            }
        }

        // If multiple drives, they compound but with diminishing returns
        // Use average with bonus for multiple
        if count > 1 {
            let avg = total_fear / count as f32;
            let compound_bonus = (count - 1) as f32 * 0.2; // +0.2 per additional drive (increased from 0.15)
            (avg + compound_bonus).min(1.0)
        } else {
            total_fear.min(1.0)
        }
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
    pub fn record_drive_satisfaction(&mut self, drive_type: DriveType, source_id: Uuid, amount: f32) {
        let current_tick = 0; // TODO: Get actual tick from context
        self.satisfaction_tracker.record(drive_type, source_id, amount, current_tick);

        // Trigger gratitude response (happiness and bond improvement)
        self.process_gratitude(source_id, amount);
    }

    /// Process gratitude when receiving help from another agent
    /// Increases happiness and improves bond with the helper
    fn process_gratitude(&mut self, helper_id: Uuid, help_amount: f32) {
        use super::EmotionSource;

        // Happiness from receiving help (scaled by amount)
        let gratitude_happiness = (help_amount * 0.3).min(0.4);
        self.emotions.add_happiness(EmotionSource::Agent(helper_id), gratitude_happiness);

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
    pub fn process_helper_happiness(&mut self, recipient_id: Uuid, help_amount: f32) {
        use super::EmotionSource;

        // Base happiness from helping (scaled by amount)
        let mut helper_happiness = (help_amount * 0.2).min(0.3);

        // Empathetic trait bonus: extra happiness from helping others
        if self.traits.has_trait(&super::traits::Trait::Empathetic) {
            helper_happiness += 0.15; // Significant bonus for empathetic helpers
        }

        self.emotions.add_happiness(EmotionSource::Agent(recipient_id), helper_happiness);
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

        // Add sadness
        self.emotions.add_sadness(EmotionSource::Agent(source_id), sadness);

        // If there's a cause and it's an agent, add anger
        if let Some(cause_source) = cause {
            match &cause_source {
                EmotionSource::Agent(_) | EmotionSource::Creature(_) => {
                    // Anger at whoever took away our satisfaction source
                    let anger = importance * 0.5; // 0.0 to 0.5 (stronger anger response)
                    self.emotions.add_anger(cause_source, anger);
                }
                EmotionSource::Event(event) => {
                    // Natural causes - less anger, more sadness
                    if !event.contains("old age") && !event.contains("natural") {
                        // Accident or preventable - some anger
                        self.emotions.add_anger(EmotionSource::Event(event.clone()), importance * 0.2);
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
}

/// Population needs data structure for job selection
#[derive(Clone, Debug, Default)]
pub struct PopulationNeeds {
    pub food_shortage: bool,
    pub food_critical: bool,
    pub wood_shortage: bool,
    pub wood_critical: bool,
    pub stone_shortage: bool,
    pub stone_critical: bool,
    pub tools_shortage: bool,
    pub shelter_shortage: bool,
    pub shelter_critical: bool,
    pub food_processing_needed: bool,
}
