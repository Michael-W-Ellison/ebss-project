// src/core/memory.rs
//! Memory system for agents.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Types of spatial memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialMemoryType {
    Food,
    Water,
    Shelter,
    Danger,
    Resource,
    Tool,
    Storage,
}

/// A spatial memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialMemory {
    pub memory_type: SpatialMemoryType,
    pub position: (i32, i32, i32),
    pub last_seen: u32, // Tick when last observed
    pub confidence: f32, // 0.0 to 1.0, decays over time
    pub value: f32, // Estimated value/usefulness
}

impl SpatialMemory {
    pub fn new(memory_type: SpatialMemoryType, position: (i32, i32, i32), tick: u32) -> Self {
        Self {
            memory_type,
            position,
            last_seen: tick,
            confidence: 1.0,
            value: 1.0,
        }
    }

    /// Decay confidence over time
    pub fn decay(&mut self, current_tick: u32) {
        let ticks_elapsed = current_tick.saturating_sub(self.last_seen);
        // Confidence decays by 0.1% per tick
        let decay = (ticks_elapsed as f32) * 0.001;
        self.confidence = (self.confidence - decay).max(0.0);
    }

    /// Refresh memory (saw it again)
    pub fn refresh(&mut self, tick: u32) {
        self.last_seen = tick;
        self.confidence = (self.confidence + 0.2).min(1.0);
    }
}

/// Relationship with another agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialRelationship {
    pub agent_id: Uuid,
    pub familiarity: f32, // 0.0 to 1.0, how well they know each other
    pub trust: f32, // -1.0 to 1.0, negative = distrust
    pub affection: f32, // -1.0 to 1.0, negative = hostility
    pub last_interaction: u32, // Tick of last interaction
    pub positive_interactions: u32,
    pub negative_interactions: u32,
    pub is_parent: bool,
    pub is_child: bool,
    pub is_mate: bool,
}

impl SocialRelationship {
    pub fn new(agent_id: Uuid, tick: u32) -> Self {
        Self {
            agent_id,
            familiarity: 0.1,
            trust: 0.0,
            affection: 0.0,
            last_interaction: tick,
            positive_interactions: 0,
            negative_interactions: 0,
            is_parent: false,
            is_child: false,
            is_mate: false,
        }
    }

    /// Record a positive interaction
    pub fn positive_interaction(&mut self, tick: u32, strength: f32) {
        self.positive_interactions += 1;
        self.trust = (self.trust + strength * 0.1).min(1.0);
        self.affection = (self.affection + strength * 0.1).min(1.0);
        self.familiarity = (self.familiarity + 0.05).min(1.0);
        self.last_interaction = tick;
    }

    /// Record a negative interaction
    pub fn negative_interaction(&mut self, tick: u32, strength: f32) {
        self.negative_interactions += 1;
        self.trust = (self.trust - strength * 0.1).max(-1.0);
        self.affection = (self.affection - strength * 0.1).max(-1.0);
        self.familiarity = (self.familiarity + 0.02).min(1.0); // Still increases familiarity
        self.last_interaction = tick;
    }

    /// Decay relationship over time without interaction
    pub fn decay(&mut self, current_tick: u32) {
        let ticks_elapsed = current_tick.saturating_sub(self.last_interaction);
        if ticks_elapsed > 1000 {
            // Decay trust and affection towards 0
            let decay_rate = 0.001;
            if self.trust > 0.0 {
                self.trust = (self.trust - decay_rate).max(0.0);
            } else if self.trust < 0.0 {
                self.trust = (self.trust + decay_rate).min(0.0);
            }

            if self.affection > 0.0 {
                self.affection = (self.affection - decay_rate).max(0.0);
            } else if self.affection < 0.0 {
                self.affection = (self.affection + decay_rate).min(0.0);
            }
        }
    }
}

/// Knowledge or recipe memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMemory {
    pub name: String,
    pub description: String,
    pub learned_at: u32, // Tick when learned
    pub proficiency: f32, // 0.0 to 1.0, improves with practice
    pub success_count: u32,
    pub failure_count: u32,
}

impl KnowledgeMemory {
    pub fn new(name: String, description: String, tick: u32) -> Self {
        Self {
            name,
            description,
            learned_at: tick,
            proficiency: 0.1,
            success_count: 0,
            failure_count: 0,
        }
    }

    /// Record successful use
    pub fn success(&mut self) {
        self.success_count += 1;
        self.proficiency = (self.proficiency + 0.05).min(1.0);
    }

    /// Record failed use
    pub fn failure(&mut self) {
        self.failure_count += 1;
        self.proficiency = (self.proficiency - 0.02).max(0.0);
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            return 0.0;
        }
        self.success_count as f32 / total as f32
    }
}

/// Complete memory system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub spatial_memories: Vec<SpatialMemory>,
    pub social_relationships: HashMap<Uuid, SocialRelationship>,
    pub knowledge: Vec<KnowledgeMemory>,
    pub current_tick: u32,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            spatial_memories: Vec::new(),
            social_relationships: HashMap::new(),
            knowledge: Vec::new(),
            current_tick: 0,
        }
    }

    /// Update memory for a new tick
    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay spatial memories
        for memory in &mut self.spatial_memories {
            memory.decay(self.current_tick);
        }

        // Remove very old, low-confidence memories
        self.spatial_memories.retain(|m| m.confidence > 0.1);

        // Decay social relationships
        for relationship in self.social_relationships.values_mut() {
            relationship.decay(self.current_tick);
        }
    }

    /// Add or update spatial memory
    pub fn remember_location(&mut self, memory_type: SpatialMemoryType, position: (i32, i32, i32)) {
        // Check if we already have this memory
        if let Some(existing) = self.spatial_memories.iter_mut().find(|m| {
            matches!(&m.memory_type, mt if std::mem::discriminant(mt) == std::mem::discriminant(&memory_type))
                && m.position == position
        }) {
            existing.refresh(self.current_tick);
        } else {
            self.spatial_memories.push(SpatialMemory::new(memory_type, position, self.current_tick));
        }
    }

    /// Get spatial memories of a specific type
    pub fn recall_locations(&self, memory_type: SpatialMemoryType) -> Vec<&SpatialMemory> {
        self.spatial_memories
            .iter()
            .filter(|m| std::mem::discriminant(&m.memory_type) == std::mem::discriminant(&memory_type))
            .filter(|m| m.confidence > 0.3)
            .collect()
    }

    /// Get or create relationship with another agent
    pub fn get_relationship(&mut self, agent_id: Uuid) -> &mut SocialRelationship {
        self.social_relationships
            .entry(agent_id)
            .or_insert_with(|| SocialRelationship::new(agent_id, self.current_tick))
    }

    /// Record interaction with another agent
    pub fn record_interaction(&mut self, agent_id: Uuid, positive: bool, strength: f32) {
        let current_tick = self.current_tick;
        let relationship = self.get_relationship(agent_id);
        if positive {
            relationship.positive_interaction(current_tick, strength);
        } else {
            relationship.negative_interaction(current_tick, strength);
        }
    }

    /// Mark another agent as family
    pub fn mark_as_parent(&mut self, parent_id: Uuid) {
        let relationship = self.get_relationship(parent_id);
        relationship.is_parent = true;
        relationship.trust = 1.0;
        relationship.affection = 0.8;
        relationship.familiarity = 1.0;
    }

    /// Mark another agent as offspring
    pub fn mark_as_child(&mut self, child_id: Uuid) {
        let relationship = self.get_relationship(child_id);
        relationship.is_child = true;
        relationship.trust = 1.0;
        relationship.affection = 0.9;
        relationship.familiarity = 1.0;
    }

    /// Mark another agent as mate
    pub fn mark_as_mate(&mut self, mate_id: Uuid) {
        let relationship = self.get_relationship(mate_id);
        relationship.is_mate = true;
        relationship.trust = 0.8;
        relationship.affection = 0.8;
        relationship.familiarity = (relationship.familiarity + 0.5).min(1.0);
    }

    /// Get trusted agents
    pub fn trusted_agents(&self) -> Vec<Uuid> {
        self.social_relationships
            .iter()
            .filter(|(_, r)| r.trust > 0.5)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Learn new knowledge
    pub fn learn(&mut self, name: String, description: String) {
        if !self.knowledge.iter().any(|k| k.name == name) {
            self.knowledge.push(KnowledgeMemory::new(name, description, self.current_tick));
        }
    }

    /// Get knowledge by name
    pub fn get_knowledge(&self, name: &str) -> Option<&KnowledgeMemory> {
        self.knowledge.iter().find(|k| k.name == name)
    }

    /// Get mutable knowledge by name
    pub fn get_knowledge_mut(&mut self, name: &str) -> Option<&mut KnowledgeMemory> {
        self.knowledge.iter_mut().find(|k| k.name == name)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_memory() {
        let mut memory = Memory::new();
        memory.remember_location(SpatialMemoryType::Food, (10, 10, 0));

        let food_locations = memory.recall_locations(SpatialMemoryType::Food);
        assert_eq!(food_locations.len(), 1);
        assert_eq!(food_locations[0].position, (10, 10, 0));
    }

    #[test]
    fn test_spatial_memory_decay() {
        let mut memory = Memory::new();
        memory.remember_location(SpatialMemoryType::Food, (10, 10, 0));

        // Fast forward time
        for _ in 0..2000 {
            memory.tick();
        }

        // Low confidence memories should be removed
        let food_locations = memory.recall_locations(SpatialMemoryType::Food);
        assert_eq!(food_locations.len(), 0);
    }

    #[test]
    fn test_social_relationship() {
        let mut memory = Memory::new();
        let other_id = Uuid::new_v4();

        memory.record_interaction(other_id, true, 1.0);

        let relationship = memory.social_relationships.get(&other_id).unwrap();
        assert!(relationship.trust > 0.0);
        assert!(relationship.affection > 0.0);
        assert_eq!(relationship.positive_interactions, 1);
    }

    #[test]
    fn test_family_marking() {
        let mut memory = Memory::new();
        let parent_id = Uuid::new_v4();

        memory.mark_as_parent(parent_id);

        let relationship = memory.social_relationships.get(&parent_id).unwrap();
        assert!(relationship.is_parent);
        assert_eq!(relationship.trust, 1.0);
    }

    #[test]
    fn test_knowledge_learning() {
        let mut memory = Memory::new();
        memory.learn("Farming".to_string(), "How to grow crops".to_string());

        let knowledge = memory.get_knowledge("Farming").unwrap();
        assert_eq!(knowledge.name, "Farming");
        assert_eq!(knowledge.proficiency, 0.1);
    }

    #[test]
    fn test_knowledge_proficiency() {
        let mut memory = Memory::new();
        memory.learn("Mining".to_string(), "How to mine ore".to_string());

        let knowledge = memory.get_knowledge_mut("Mining").unwrap();
        knowledge.success();
        knowledge.success();

        assert!(knowledge.proficiency > 0.1);
        assert_eq!(knowledge.success_count, 2);
    }
}
