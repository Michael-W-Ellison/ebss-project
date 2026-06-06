// src/environment/technology.rs
//! Technology discovery and knowledge propagation system.
//!
//! This system enables emergent technological progression where agents:
//! - Discover new technologies through experimentation, observation, or accident
//! - Share knowledge through social networks (gossip)
//! - Progress from low to high skill levels with each technology
//! - Have technologies gated by prerequisites and discovery conditions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// State of knowledge about a technology
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TechnologyState {
    /// Never heard of this technology
    Unknown,
    /// Heard about it through gossip, low confidence
    Rumored,
    /// Learned from reliable source or discovered it
    Known,
    /// Successfully used this technology at least once
    Practiced,
    /// High skill level (6+), mastered this technology
    Mastered,
}

impl TechnologyState {
    /// Can this agent attempt to use this technology?
    pub fn can_attempt(&self) -> bool {
        matches!(self, TechnologyState::Known | TechnologyState::Practiced | TechnologyState::Mastered)
    }

    /// Confidence level for teaching others
    pub fn teaching_confidence(&self) -> f32 {
        match self {
            TechnologyState::Unknown => 0.0,
            TechnologyState::Rumored => 0.3,
            TechnologyState::Known => 0.6,
            TechnologyState::Practiced => 0.8,
            TechnologyState::Mastered => 1.0,
        }
    }
}

/// How a technology was discovered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// Agent experimented with materials (Curiosity-driven)
    Experimentation,
    /// Watched another agent use this technology
    Observation,
    /// Directly taught by another agent
    Instruction,
    /// Heard about it through word-of-mouth
    Gossip,
    /// Accidental discovery (e.g., ore melting in fire)
    Accident,
    /// Given at start (basic Stone Age knowledge)
    Initial,
}

impl DiscoveryMethod {
    /// Initial confidence when learning via this method
    pub fn initial_confidence(&self) -> f32 {
        match self {
            DiscoveryMethod::Experimentation => 0.7, // You did it yourself
            DiscoveryMethod::Observation => 0.6,     // You saw it work
            DiscoveryMethod::Instruction => 0.8,     // Teacher showed you
            DiscoveryMethod::Gossip => 0.4,          // Just heard about it
            DiscoveryMethod::Accident => 0.5,        // Stumbled upon it
            DiscoveryMethod::Initial => 1.0,         // Starting knowledge
        }
    }
}

/// Record of when and how a technology was discovered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    pub tech_id: String,
    pub discoverer: Uuid,
    pub method: DiscoveryMethod,
    pub timestamp: u64,
    pub confidence: f32,
    pub success_count: u32,
    pub failure_count: u32,
}

impl DiscoveryRecord {
    pub fn new(tech_id: String, discoverer: Uuid, method: DiscoveryMethod, timestamp: u64) -> Self {
        Self {
            tech_id,
            discoverer,
            method,
            timestamp,
            confidence: method.initial_confidence(),
            success_count: 0,
            failure_count: 0,
        }
    }

    /// Update confidence based on success/failure
    pub fn record_attempt(&mut self, success: bool) {
        if success {
            self.success_count += 1;
            self.confidence = (self.confidence + 0.1).min(1.0);
        } else {
            self.failure_count += 1;
            self.confidence = (self.confidence - 0.05).max(0.0);
        }
    }

    /// Get current state based on skill and confidence
    pub fn get_state(&self, skill_level: i32) -> TechnologyState {
        if skill_level >= 6 {
            TechnologyState::Mastered
        } else if self.success_count > 0 {
            TechnologyState::Practiced
        } else if self.confidence >= 0.5 {
            TechnologyState::Known
        } else if self.confidence > 0.0 {
            TechnologyState::Rumored
        } else {
            TechnologyState::Unknown
        }
    }

    /// Success rate
    pub fn success_rate(&self) -> f32 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        self.success_count as f32 / total as f32
    }
}

/// Technology definition with discovery conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technology {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Technologies that must be known first
    pub prerequisites: Vec<String>,

    /// Materials required to exist for discovery
    pub required_materials: Vec<String>,

    /// Minimum Curiosity drive value to attempt experimentation
    pub curiosity_threshold: f32,

    /// Base chance of discovery when experimenting (0.0 to 1.0)
    pub discovery_chance: f32,

    /// Can this be discovered accidentally?
    pub accidental_discovery: bool,

    /// Chance of accidental discovery per tick when conditions met
    pub accident_chance: f32,

    /// Related recipe ID (if any)
    pub recipe_id: Option<String>,

    /// Skill type used for this technology
    pub skill_type: Option<String>,
}

impl Technology {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            prerequisites: Vec::new(),
            required_materials: Vec::new(),
            curiosity_threshold: 0.3,
            discovery_chance: 0.1,
            accidental_discovery: false,
            accident_chance: 0.001,
            recipe_id: None,
            skill_type: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_prerequisites(mut self, prerequisites: Vec<String>) -> Self {
        self.prerequisites = prerequisites;
        self
    }

    pub fn with_required_materials(mut self, materials: Vec<String>) -> Self {
        self.required_materials = materials;
        self
    }

    pub fn with_curiosity_threshold(mut self, threshold: f32) -> Self {
        self.curiosity_threshold = threshold;
        self
    }

    pub fn with_discovery_chance(mut self, chance: f32) -> Self {
        self.discovery_chance = chance;
        self
    }

    pub fn with_accidental_discovery(mut self, accident_chance: f32) -> Self {
        self.accidental_discovery = true;
        self.accident_chance = accident_chance;
        self
    }

    pub fn with_recipe(mut self, recipe_id: String) -> Self {
        self.recipe_id = Some(recipe_id);
        self
    }

    pub fn with_skill_type(mut self, skill_type: String) -> Self {
        self.skill_type = Some(skill_type);
        self
    }

    /// Check if agent meets prerequisites
    pub fn can_discover(&self, known_techs: &HashMap<String, DiscoveryRecord>) -> bool {
        // Check all prerequisites are known
        for prereq in &self.prerequisites {
            if let Some(record) = known_techs.get(prereq) {
                if !record.get_state(0).can_attempt() {
                    return false;
                }
            } else {
                return false; // Prerequisite not known at all
            }
        }
        true
    }
}

/// Agent's personal knowledge of technologies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyKnowledge {
    /// All technologies this agent knows about
    pub known_technologies: HashMap<String, DiscoveryRecord>,

    /// Technologies discovered by this agent (first in world)
    pub original_discoveries: Vec<String>,
}

impl TechnologyKnowledge {
    pub fn new() -> Self {
        Self {
            known_technologies: HashMap::new(),
            original_discoveries: Vec::new(),
        }
    }

    /// Add initial starting knowledge
    pub fn add_initial_technology(&mut self, tech_id: String, agent_id: Uuid, timestamp: u64) {
        let record = DiscoveryRecord::new(tech_id.clone(), agent_id, DiscoveryMethod::Initial, timestamp);
        self.known_technologies.insert(tech_id, record);
    }

    /// Discover a new technology
    pub fn discover_technology(
        &mut self,
        tech_id: String,
        agent_id: Uuid,
        method: DiscoveryMethod,
        timestamp: u64,
        is_world_first: bool,
    ) {
        let record = DiscoveryRecord::new(tech_id.clone(), agent_id, method, timestamp);
        self.known_technologies.insert(tech_id.clone(), record);

        if is_world_first {
            self.original_discoveries.push(tech_id);
        }
    }

    /// Learn about technology from another agent
    pub fn learn_from_agent(
        &mut self,
        tech_id: String,
        agent_id: Uuid,
        method: DiscoveryMethod,
        teacher_confidence: f32,
        trust_in_teacher: f32,
        timestamp: u64,
    ) {
        // Combine teacher confidence and trust
        let initial_confidence = teacher_confidence * trust_in_teacher;

        let mut record = DiscoveryRecord::new(tech_id.clone(), agent_id, method, timestamp);
        record.confidence = initial_confidence;

        self.known_technologies.insert(tech_id, record);
    }

    /// Record attempt to use technology
    pub fn record_attempt(&mut self, tech_id: &str, success: bool) {
        if let Some(record) = self.known_technologies.get_mut(tech_id) {
            record.record_attempt(success);
        }
    }

    /// Get current state of a technology
    pub fn get_state(&self, tech_id: &str, skill_level: i32) -> TechnologyState {
        if let Some(record) = self.known_technologies.get(tech_id) {
            record.get_state(skill_level)
        } else {
            TechnologyState::Unknown
        }
    }

    /// Check if can attempt to use technology
    pub fn can_use(&self, tech_id: &str, skill_level: i32) -> bool {
        self.get_state(tech_id, skill_level).can_attempt()
    }

    /// Get teaching confidence for a technology
    pub fn teaching_confidence(&self, tech_id: &str) -> f32 {
        if let Some(record) = self.known_technologies.get(tech_id) {
            record.confidence
        } else {
            0.0
        }
    }

    /// Get all known technologies at a minimum state
    pub fn get_technologies_at_state(&self, min_state: TechnologyState, skill_level: i32) -> Vec<String> {
        self.known_technologies
            .iter()
            .filter(|(_, record)| {
                let state = record.get_state(skill_level);
                state as u8 >= min_state as u8
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> TechnologyStats {
        let mut by_state = HashMap::new();
        let mut by_method = HashMap::new();

        for record in self.known_technologies.values() {
            let state = record.get_state(0);
            *by_state.entry(state).or_insert(0) += 1;
            *by_method.entry(record.method).or_insert(0) += 1;
        }

        TechnologyStats {
            total_known: self.known_technologies.len(),
            original_discoveries: self.original_discoveries.len(),
            by_state,
            by_method,
        }
    }
}

impl Default for TechnologyKnowledge {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about technology knowledge
#[derive(Debug, Clone)]
pub struct TechnologyStats {
    pub total_known: usize,
    pub original_discoveries: usize,
    pub by_state: HashMap<TechnologyState, usize>,
    pub by_method: HashMap<DiscoveryMethod, usize>,
}

/// Global technology registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyRegistry {
    technologies: HashMap<String, Technology>,

    /// First discoverer of each technology in the world
    pub first_discoverers: HashMap<String, (Uuid, u64)>,
}

impl TechnologyRegistry {
    pub fn new() -> Self {
        Self {
            technologies: HashMap::new(),
            first_discoverers: HashMap::new(),
        }
    }

    /// Register a technology
    pub fn register(&mut self, tech: Technology) {
        self.technologies.insert(tech.id.clone(), tech);
    }

    /// Get a technology
    pub fn get(&self, tech_id: &str) -> Option<&Technology> {
        self.technologies.get(tech_id)
    }

    /// Record first discovery
    pub fn record_first_discovery(&mut self, tech_id: String, discoverer: Uuid, timestamp: u64) -> bool {
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(e) = self.first_discoverers.entry(tech_id) {
            e.insert((discoverer, timestamp));
            true
        } else {
            false // Already discovered
        }
    }

    /// Check if technology has been discovered by anyone
    pub fn is_discovered(&self, tech_id: &str) -> bool {
        self.first_discoverers.contains_key(tech_id)
    }

    /// Get all technologies
    pub fn all_technologies(&self) -> Vec<&Technology> {
        self.technologies.values().collect()
    }

    /// Get technologies that can be discovered with given prerequisites
    pub fn available_for_discovery(&self, known_techs: &HashMap<String, DiscoveryRecord>) -> Vec<&Technology> {
        self.technologies
            .values()
            .filter(|tech| tech.can_discover(known_techs))
            .collect()
    }
}

impl Default for TechnologyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_technology_state_progression() {
        let agent = Uuid::new_v4();
        let mut record = DiscoveryRecord::new("test".to_string(), agent, DiscoveryMethod::Experimentation, 0);

        // Starts as Known (0.7 confidence from experimentation)
        assert_eq!(record.get_state(0), TechnologyState::Known);

        // After success, becomes Practiced
        record.record_attempt(true);
        assert_eq!(record.get_state(0), TechnologyState::Practiced);

        // At skill 6+, becomes Mastered
        assert_eq!(record.get_state(6), TechnologyState::Mastered);
    }

    #[test]
    fn test_discovery_confidence() {
        let agent = Uuid::new_v4();
        let mut record = DiscoveryRecord::new("test".to_string(), agent, DiscoveryMethod::Gossip, 0);

        // Gossip starts with low confidence
        assert_eq!(record.confidence, 0.4);

        // Successes increase confidence
        record.record_attempt(true);
        assert!(record.confidence > 0.4);

        // Failures decrease confidence
        record.record_attempt(false);
        assert!(record.confidence < 0.5);
    }

    #[test]
    fn test_technology_prerequisites() {
        let mut known = HashMap::new();
        let agent = Uuid::new_v4();

        // Add prerequisite technology
        let mut prereq_record = DiscoveryRecord::new("flint_knapping".to_string(), agent, DiscoveryMethod::Initial, 0);
        prereq_record.success_count = 1; // Make it Known
        known.insert("flint_knapping".to_string(), prereq_record);

        let tech = Technology::new("copper_working".to_string(), "Copper Working".to_string())
            .with_prerequisites(vec!["flint_knapping".to_string()]);

        assert!(tech.can_discover(&known));

        // Without prerequisite, cannot discover
        let tech2 = Technology::new("iron_smelting".to_string(), "Iron Smelting".to_string())
            .with_prerequisites(vec!["bloomery".to_string()]);

        assert!(!tech2.can_discover(&known));
    }

    #[test]
    fn test_technology_knowledge() {
        let agent = Uuid::new_v4();
        let mut knowledge = TechnologyKnowledge::new();

        // Initially unknown
        assert_eq!(knowledge.get_state("copper_smelting", 0), TechnologyState::Unknown);
        assert!(!knowledge.can_use("copper_smelting", 0));

        // Discover technology
        knowledge.discover_technology(
            "copper_smelting".to_string(),
            agent,
            DiscoveryMethod::Experimentation,
            0,
            true,
        );

        // Now known
        assert_eq!(knowledge.get_state("copper_smelting", 0), TechnologyState::Known);
        assert!(knowledge.can_use("copper_smelting", 0));

        // Record success
        knowledge.record_attempt("copper_smelting", true);
        assert_eq!(knowledge.get_state("copper_smelting", 0), TechnologyState::Practiced);
    }

    #[test]
    fn test_learning_from_teacher() {
        let agent = Uuid::new_v4();
        let mut knowledge = TechnologyKnowledge::new();

        // Learn from highly trusted, confident teacher
        knowledge.learn_from_agent(
            "bronze_casting".to_string(),
            agent,
            DiscoveryMethod::Instruction,
            0.9, // Teacher confidence
            0.8, // Trust in teacher
            0,
        );

        let record = knowledge.known_technologies.get("bronze_casting").unwrap();
        // Confidence should be product: 0.9 * 0.8 = 0.72
        assert!((record.confidence - 0.72).abs() < 0.01);
    }

    #[test]
    fn test_technology_registry() {
        let mut registry = TechnologyRegistry::new();

        let tech = Technology::new("flint_knapping".to_string(), "Flint Knapping".to_string())
            .with_description("Shaping flint into sharp tools".to_string());

        registry.register(tech);

        assert!(registry.get("flint_knapping").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_first_discovery_tracking() {
        let mut registry = TechnologyRegistry::new();
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();

        // First discovery
        assert!(registry.record_first_discovery("copper_smelting".to_string(), agent1, 100));

        // Second attempt should fail
        assert!(!registry.record_first_discovery("copper_smelting".to_string(), agent2, 200));

        // Check first discoverer
        let (discoverer, time) = registry.first_discoverers.get("copper_smelting").unwrap();
        assert_eq!(*discoverer, agent1);
        assert_eq!(*time, 100);
    }

    #[test]
    fn test_teaching_confidence_levels() {
        assert_eq!(TechnologyState::Unknown.teaching_confidence(), 0.0);
        assert_eq!(TechnologyState::Rumored.teaching_confidence(), 0.3);
        assert_eq!(TechnologyState::Known.teaching_confidence(), 0.6);
        assert_eq!(TechnologyState::Practiced.teaching_confidence(), 0.8);
        assert_eq!(TechnologyState::Mastered.teaching_confidence(), 1.0);
    }

    #[test]
    fn test_success_rate_calculation() {
        let agent = Uuid::new_v4();
        let mut record = DiscoveryRecord::new("test".to_string(), agent, DiscoveryMethod::Experimentation, 0);

        // No attempts yet
        assert_eq!(record.success_rate(), 0.0);

        // 2 successes, 1 failure
        record.record_attempt(true);
        record.record_attempt(true);
        record.record_attempt(false);

        assert!((record.success_rate() - 0.666).abs() < 0.01);
    }
}
