// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, DriveState, Memory, EmotionalState, TraitSet, GoalManager, Preferences};
use crate::world::{Inventory, ItemType, Position, ResourceType};
use crate::agents::{PersonalKnowledge, SocialNetwork, Profession};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub random_weights: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { random_weights: true }
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
    pub parent_ids: Vec<Uuid>,
    pub emotions: EmotionalState,
    pub traits: TraitSet,
    pub goals: GoalManager,
    pub preferences: Preferences,
    pub inventory: Inventory, // Personal inventory for carrying food and resources
    pub knowledge: PersonalKnowledge, // Personal knowledge about world (resources, etc.)
    pub social_network: SocialNetwork, // Relationships and trust with other agents
    pub profession: Profession, // Job/profession and skill level
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
            parent_ids: Vec::new(),
            emotions: EmotionalState::new(),
            traits: TraitSet::generate_random(3), // 3 random traits
            goals: GoalManager::default(),
            preferences: Preferences::generate_random(),
            inventory: Inventory::new(20), // Can carry up to 20 items
            knowledge: PersonalKnowledge::new(),
            social_network: SocialNetwork::new(),
            profession: Profession::default(), // Starts unemployed
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
}

