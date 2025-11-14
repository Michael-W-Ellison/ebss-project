// src/core/memory.rs
//! Memory system for agents with dynamic expansion and management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Type of memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryType {
    /// Location memory (where things are)
    Spatial,
    /// Storage location memory (chests, containers)
    Storage,
    /// Social interactions and relationships
    Social,
    /// Known crafting recipes
    Recipe,
    /// Events witnessed or experienced
    Event,
    /// Knowledge about resources
    Resource,
    /// Threats and dangers
    Threat,
}

/// Importance level of a memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MemoryImportance {
    Trivial,
    Minor,
    Normal,
    Important,
    Critical,
}

impl MemoryImportance {
    /// Get decay rate multiplier (lower importance = faster decay)
    pub fn decay_multiplier(&self) -> f32 {
        match self {
            MemoryImportance::Trivial => 2.0,
            MemoryImportance::Minor => 1.5,
            MemoryImportance::Normal => 1.0,
            MemoryImportance::Important => 0.5,
            MemoryImportance::Critical => 0.1,
        }
    }
}

/// A single memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub memory_type: MemoryType,
    pub importance: MemoryImportance,

    /// Strength of the memory (0.0 to 1.0, decays over time)
    pub strength: f32,
    /// When this memory was formed
    pub timestamp: u64,
    /// Last time this memory was accessed/reinforced
    pub last_accessed: u64,
    /// How many times this memory has been accessed
    pub access_count: u32,

    /// The actual memory data
    pub data: MemoryData,
}

/// Memory data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryData {
    /// Location of something
    Spatial {
        subject: String,
        location: (i32, i32, i32),
        notes: Option<String>,
    },
    /// Storage container location and contents
    Storage {
        container_id: String,
        location: (i32, i32, i32),
        known_contents: Vec<String>,
    },
    /// Social interaction
    Social {
        other_agent: Uuid,
        interaction_type: String,
        emotional_valence: f32, // -1.0 (negative) to 1.0 (positive)
    },
    /// Known recipe
    Recipe {
        recipe_id: String,
        success_count: u32,
        failure_count: u32,
    },
    /// Event that occurred
    Event {
        description: String,
        location: Option<(i32, i32, i32)>,
        participants: Vec<Uuid>,
    },
    /// Resource knowledge
    Resource {
        resource_type: String,
        location: (i32, i32, i32),
        abundance: f32, // Estimated abundance (0.0 to 1.0)
    },
    /// Threat/danger
    Threat {
        threat_type: String,
        location: (i32, i32, i32),
        danger_level: f32, // 0.0 to 1.0
        last_encounter: u64,
    },
}

impl MemoryEntry {
    pub fn new(memory_type: MemoryType, importance: MemoryImportance, data: MemoryData, timestamp: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            memory_type,
            importance,
            strength: 1.0,
            timestamp,
            last_accessed: timestamp,
            access_count: 0,
            data,
        }
    }

    /// Access this memory (reinforces it)
    pub fn access(&mut self, current_time: u64) {
        self.last_accessed = current_time;
        self.access_count += 1;
        // Reinforce the memory slightly
        self.strength = (self.strength + 0.1).min(1.0);
    }

    /// Apply time-based decay to memory strength
    pub fn decay(&mut self, decay_amount: f32) {
        let decay = decay_amount * self.importance.decay_multiplier();
        self.strength = (self.strength - decay).max(0.0);
    }

    /// Check if memory is too weak to keep
    pub fn should_forget(&self) -> bool {
        self.strength < 0.1
    }
}

/// Configuration for memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of memories to store (None = unlimited)
    pub max_memories: Option<usize>,
    /// Memory decay rate per tick
    pub decay_rate: f32,
    /// Whether to automatically forget weak memories
    pub auto_forget: bool,
    /// Minimum strength to keep when auto-forgetting
    pub forget_threshold: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memories: Some(1000), // Default limit of 1000 memories
            decay_rate: 0.001,        // Very slow decay (0.1% per tick)
            auto_forget: true,
            forget_threshold: 0.1,
        }
    }
}

/// Agent memory system with dynamic expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// All memories stored
    memories: Vec<MemoryEntry>,
    /// Index by type for fast lookup
    type_index: HashMap<MemoryType, Vec<Uuid>>,
    /// Configuration
    pub config: MemoryConfig,
    /// Current simulation time
    current_time: u64,
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

/// Relationship strength categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipStrength {
    CloseFriend,
    Friend,
    Acquaintance,
    Neutral,
    Disliked,
    Enemy,
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

        // Different decay rates based on relationship type
        let decay_threshold = if self.is_parent || self.is_child || self.is_mate {
            5000 // Family bonds persist 5x longer
        } else {
            1000
        };

        if ticks_elapsed > decay_threshold {
            // Base decay rate
            let mut decay_rate = 0.001;

            // Family relationships decay slower
            if self.is_parent || self.is_child || self.is_mate {
                decay_rate *= 0.2;
            }

            // Negative relationships decay slower (grudges last longer)
            if self.trust < 0.0 || self.affection < 0.0 {
                decay_rate *= 0.5;
            }

            // Decay trust and affection towards 0
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

            // Familiarity decays very slowly
            self.familiarity = (self.familiarity - decay_rate * 0.5).max(0.0);
        }
    }

    /// Get relationship strength category
    pub fn relationship_strength(&self) -> RelationshipStrength {
        let combined = self.trust + self.affection;

        if combined >= 1.5 {
            RelationshipStrength::CloseFriend
        } else if combined >= 0.8 {
            RelationshipStrength::Friend
        } else if combined >= 0.3 {
            RelationshipStrength::Acquaintance
        } else if combined >= -0.3 {
            RelationshipStrength::Neutral
        } else if combined >= -0.8 {
            RelationshipStrength::Disliked
        } else {
            RelationshipStrength::Enemy
        }
    }

    /// Should this agent be avoided?
    pub fn should_avoid(&self) -> bool {
        self.trust < -0.3 || self.affection < -0.3
    }

    /// Is this a strong positive relationship?
    pub fn is_strong_bond(&self) -> bool {
        (self.trust > 0.6 && self.affection > 0.6) || self.is_parent || self.is_child || self.is_mate
    }

    /// Get interaction frequency (interactions per 1000 ticks)
    pub fn interaction_frequency(&self, current_tick: u32) -> f32 {
        let total_interactions = self.positive_interactions + self.negative_interactions;
        if total_interactions == 0 {
            return 0.0;
        }

        let ticks_elapsed = current_tick.saturating_sub(self.last_interaction).max(1);
        (total_interactions as f32 / ticks_elapsed as f32) * 1000.0
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
        Self::with_config(MemoryConfig::default())
    }

    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            memories: Vec::new(),
            type_index: HashMap::new(),
            config,
            current_time: 0,
        }
    }

    /// Add a new memory
    pub fn add_memory(&mut self, memory_type: MemoryType, importance: MemoryImportance, data: MemoryData) {
        let memory = MemoryEntry::new(memory_type, importance, data, self.current_time);
        let id = memory.id;

        // Add to main storage
        self.memories.push(memory);

        // Update type index
        self.type_index
            .entry(memory_type)
            .or_insert_with(Vec::new)
            .push(id);

        // Check if we need to prune
        if let Some(max) = self.config.max_memories {
            if self.memories.len() > max {
                self.prune_memories();
            }
        }
    }

    /// Get all memories of a specific type
    pub fn get_by_type(&self, memory_type: MemoryType) -> Vec<&MemoryEntry> {
        if let Some(ids) = self.type_index.get(&memory_type) {
            ids.iter()
                .filter_map(|id| self.memories.iter().find(|m| &m.id == id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get a specific memory by ID and mark it as accessed
    pub fn get_and_access(&mut self, id: &Uuid) -> Option<&MemoryEntry> {
        if let Some(memory) = self.memories.iter_mut().find(|m| &m.id == id) {
            memory.access(self.current_time);
            Some(memory)
        } else {
            None
        }
    }

    /// Search memories by type and recency
    pub fn get_recent(&self, memory_type: MemoryType, count: usize) -> Vec<&MemoryEntry> {
        let mut memories = self.get_by_type(memory_type);
        memories.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        memories.into_iter().take(count).collect()
    }

    /// Search memories by importance
    pub fn get_important(&self, min_importance: MemoryImportance) -> Vec<&MemoryEntry> {
        self.memories
            .iter()
            .filter(|m| m.importance >= min_importance)
            .collect()
    }

    /// Search for spatial memories near a location
    pub fn find_spatial_near(&self, location: (i32, i32, i32), max_distance: f32) -> Vec<&MemoryEntry> {
        self.memories
            .iter()
            .filter(|m| {
                if let MemoryData::Spatial { location: mem_loc, .. } = m.data {
                    let dx = (location.0 - mem_loc.0) as f32;
                    let dy = (location.1 - mem_loc.1) as f32;
                    let dz = (location.2 - mem_loc.2) as f32;
                    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                    distance <= max_distance
                } else {
                    false
                }
            })
            .collect()
    }

    /// Find resource memories
    pub fn find_resource(&self, resource_type: &str) -> Vec<&MemoryEntry> {
        self.memories
            .iter()
            .filter(|m| {
                if let MemoryData::Resource { resource_type: rt, .. } = &m.data {
                    rt == resource_type
                } else {
                    false
                }
            })
            .collect()
    }

    /// Tick the memory system (decay and cleanup)
    pub fn tick(&mut self) {
        self.current_time += 1;

        // Apply decay to all memories
        for memory in &mut self.memories {
            memory.decay(self.config.decay_rate);
        }

        // Auto-forget weak memories if enabled
        if self.config.auto_forget {
            self.forget_weak_memories();
        }
    }

    /// Remove weak memories
    fn forget_weak_memories(&mut self) {
        // Remove from main storage
        let forgotten_ids: Vec<Uuid> = self.memories
            .iter()
            .filter(|m| m.should_forget() && m.importance != MemoryImportance::Critical)
            .map(|m| m.id)
            .collect();

        self.memories.retain(|m| !forgotten_ids.contains(&m.id));

        // Rebuild type index if we forgot anything
        if !forgotten_ids.is_empty() {
            self.rebuild_type_index();
        }
    }

    /// Prune memories when over limit (removes oldest, least accessed)
    fn prune_memories(&mut self) {
        if let Some(max) = self.config.max_memories {
            if self.memories.len() <= max {
                return;
            }

            // Sort by importance (desc), then access count (desc), then age (oldest first)
            self.memories.sort_by(|a, b| {
                b.importance
                    .cmp(&a.importance)
                    .then(b.access_count.cmp(&a.access_count))
                    .then(a.timestamp.cmp(&b.timestamp))
            });

            // Keep only the top max memories
            self.memories.truncate(max);

            // Rebuild index
            self.rebuild_type_index();
        }
    }

    /// Rebuild the type index
    fn rebuild_type_index(&mut self) {
        self.type_index.clear();
        for memory in &self.memories {
            self.type_index
                .entry(memory.memory_type)
                .or_insert_with(Vec::new)
                .push(memory.id);
        }
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        let mut by_type = HashMap::new();
        let mut by_importance = HashMap::new();

        for memory in &self.memories {
            *by_type.entry(memory.memory_type).or_insert(0) += 1;
            *by_importance.entry(memory.importance).or_insert(0) += 1;
        }

        let avg_strength = if self.memories.is_empty() {
            0.0
        } else {
            self.memories.iter().map(|m| m.strength).sum::<f32>() / self.memories.len() as f32
        };

        MemoryStats {
            total_memories: self.memories.len(),
            by_type,
            by_importance,
            average_strength: avg_strength,
        }
    }

    /// Clear all memories
    pub fn clear(&mut self) {
        self.memories.clear();
        self.type_index.clear();
    }

    /// Get total memory count
    pub fn len(&self) -> usize {
        self.memories.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
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

    /// Get agents to avoid (disliked or distrusted)
    pub fn agents_to_avoid(&self) -> Vec<Uuid> {
        self.social_relationships
            .iter()
            .filter(|(_, r)| r.should_avoid())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get strong bonds (close friends and family)
    pub fn strong_bonds(&self) -> Vec<Uuid> {
        self.social_relationships
            .iter()
            .filter(|(_, r)| r.is_strong_bond())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get agents by relationship strength
    pub fn agents_by_strength(&self, strength: RelationshipStrength) -> Vec<Uuid> {
        self.social_relationships
            .iter()
            .filter(|(_, r)| r.relationship_strength() == strength)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Check if agent should be avoided
    pub fn should_avoid_agent(&self, agent_id: Uuid) -> bool {
        self.social_relationships
            .get(&agent_id)
            .map(|r| r.should_avoid())
            .unwrap_or(false)
    }

    /// Get most liked agent
    pub fn most_liked_agent(&self) -> Option<Uuid> {
        self.social_relationships
            .iter()
            .max_by(|(_, a), (_, b)| {
                let a_score = a.trust + a.affection;
                let b_score = b.trust + b.affection;
                a_score.partial_cmp(&b_score).unwrap()
            })
            .map(|(id, _)| *id)
    }

    /// Get most disliked agent
    pub fn most_disliked_agent(&self) -> Option<Uuid> {
        self.social_relationships
            .iter()
            .min_by(|(_, a), (_, b)| {
                let a_score = a.trust + a.affection;
                let b_score = b.trust + b.affection;
                a_score.partial_cmp(&b_score).unwrap()
            })
            .map(|(id, _)| *id)
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

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub by_type: HashMap<MemoryType, usize>,
    pub by_importance: HashMap<MemoryImportance, usize>,
    pub average_strength: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let memory = Memory::new();
        assert_eq!(memory.len(), 0);
        assert!(memory.is_empty());
    }

    #[test]
    fn test_add_memory() {
        let mut memory = Memory::new();

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "tree".to_string(),
                location: (10, 0, 5),
                notes: None,
            },
        );

        assert_eq!(memory.len(), 1);
    }

    #[test]
    fn test_memory_by_type() {
        let mut memory = Memory::new();

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "tree".to_string(),
                location: (10, 0, 5),
                notes: None,
            },
        );

        memory.add_memory(
            MemoryType::Social,
            MemoryImportance::Important,
            MemoryData::Social {
                other_agent: Uuid::new_v4(),
                interaction_type: "trade".to_string(),
                emotional_valence: 0.8,
            },
        );

        let spatial = memory.get_by_type(MemoryType::Spatial);
        assert_eq!(spatial.len(), 1);

        let social = memory.get_by_type(MemoryType::Social);
        assert_eq!(social.len(), 1);
    }

    #[test]
    fn test_memory_decay() {
        let mut config = MemoryConfig::default();
        config.auto_forget = false; // Disable auto-forget to test pure decay
        let mut memory = Memory::with_config(config);

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "tree".to_string(),
                location: (10, 0, 5),
                notes: None,
            },
        );

        let initial_strength = memory.memories[0].strength;

        // Tick many times to cause decay
        for _ in 0..1000 {
            memory.tick();
        }

        // Memory should still exist but be weaker
        assert_eq!(memory.len(), 1);
        assert!(memory.memories[0].strength < initial_strength);
    }

    #[test]
    fn test_auto_forget() {
        let mut config = MemoryConfig::default();
        config.decay_rate = 0.01; // Faster decay for testing
        config.auto_forget = true;

        let mut memory = Memory::with_config(config);

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Trivial,
            MemoryData::Spatial {
                subject: "rock".to_string(),
                location: (5, 0, 3),
                notes: None,
            },
        );

        // Tick until forgotten
        for _ in 0..200 {
            memory.tick();
        }

        // Trivial memory should be forgotten
        assert_eq!(memory.len(), 0);
    }

    #[test]
    fn test_critical_never_forgotten() {
        let mut config = MemoryConfig::default();
        config.decay_rate = 1.0; // Extreme decay
        config.auto_forget = true;

        let mut memory = Memory::with_config(config);

        memory.add_memory(
            MemoryType::Threat,
            MemoryImportance::Critical,
            MemoryData::Threat {
                threat_type: "bear".to_string(),
                location: (20, 0, 10),
                danger_level: 1.0,
                last_encounter: 0,
            },
        );

        // Tick many times
        for _ in 0..100 {
            memory.tick();
        }

        // Critical memory should still exist
        assert_eq!(memory.len(), 1);
    }

    #[test]
    fn test_memory_limit() {
        let mut config = MemoryConfig::default();
        config.max_memories = Some(10);

        let mut memory = Memory::with_config(config);

        // Add 20 memories
        for i in 0..20 {
            memory.add_memory(
                MemoryType::Spatial,
                MemoryImportance::Normal,
                MemoryData::Spatial {
                    subject: format!("object_{}", i),
                    location: (i as i32, 0, 0),
                    notes: None,
                },
            );
        }

        // Should be capped at 10
        assert_eq!(memory.len(), 10);
    }

    #[test]
    fn test_memory_access_reinforcement() {
        let mut memory = Memory::new();

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "tree".to_string(),
                location: (10, 0, 5),
                notes: None,
            },
        );

        let id = memory.memories[0].id;
        let initial_strength = memory.memories[0].strength;
        let initial_access = memory.memories[0].access_count;

        memory.get_and_access(&id);

        assert!(memory.memories[0].strength >= initial_strength);
        assert_eq!(memory.memories[0].access_count, initial_access + 1);
    }

    #[test]
    fn test_find_spatial_near() {
        let mut memory = Memory::new();

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "tree".to_string(),
                location: (10, 0, 5),
                notes: None,
            },
        );

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "rock".to_string(),
                location: (100, 0, 100),
                notes: None,
            },
        );

        let nearby = memory.find_spatial_near((10, 0, 5), 10.0);
        assert_eq!(nearby.len(), 1);
    }

    #[test]
    fn test_memory_stats() {
        let mut memory = Memory::new();

        memory.add_memory(
            MemoryType::Spatial,
            MemoryImportance::Normal,
            MemoryData::Spatial {
                subject: "tree".to_string(),
                location: (10, 0, 5),
                notes: None,
            },
        );

        memory.add_memory(
            MemoryType::Social,
            MemoryImportance::Important,
            MemoryData::Social {
                other_agent: Uuid::new_v4(),
                interaction_type: "trade".to_string(),
                emotional_valence: 0.8,
            },
        );

        let stats = memory.stats();
        assert_eq!(stats.total_memories, 2);
        assert_eq!(*stats.by_type.get(&MemoryType::Spatial).unwrap(), 1);
        assert_eq!(*stats.by_type.get(&MemoryType::Social).unwrap(), 1);
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

    #[test]
    fn test_relationship_strength_categories() {
        let mut relationship = SocialRelationship::new(Uuid::new_v4(), 0);

        // Start as neutral
        assert_eq!(relationship.relationship_strength(), RelationshipStrength::Neutral);

        // Become friends
        relationship.trust = 0.5;
        relationship.affection = 0.4;
        assert_eq!(relationship.relationship_strength(), RelationshipStrength::Friend);

        // Become close friends
        relationship.trust = 0.8;
        relationship.affection = 0.8;
        assert_eq!(relationship.relationship_strength(), RelationshipStrength::CloseFriend);

        // Become disliked
        relationship.trust = -0.5;
        relationship.affection = -0.3;
        assert_eq!(relationship.relationship_strength(), RelationshipStrength::Disliked);
    }

    #[test]
    fn test_should_avoid() {
        let mut relationship = SocialRelationship::new(Uuid::new_v4(), 0);

        // Neutral relationship should not be avoided
        assert!(!relationship.should_avoid());

        // Negative trust triggers avoidance
        relationship.trust = -0.4;
        assert!(relationship.should_avoid());

        // Negative affection triggers avoidance
        relationship.trust = 0.0;
        relationship.affection = -0.4;
        assert!(relationship.should_avoid());
    }

    #[test]
    fn test_family_bonds_decay_slower() {
        let mut parent_rel = SocialRelationship::new(Uuid::new_v4(), 0);
        parent_rel.is_parent = true;
        parent_rel.trust = 1.0;
        parent_rel.affection = 0.8;

        let mut friend_rel = SocialRelationship::new(Uuid::new_v4(), 0);
        friend_rel.trust = 1.0;
        friend_rel.affection = 0.8;

        // Fast forward 3000 ticks
        for tick in 1..3001 {
            parent_rel.decay(tick);
            friend_rel.decay(tick);
        }

        // Family bond should decay much less than friend
        assert!(parent_rel.trust > friend_rel.trust);
        assert!(parent_rel.affection > friend_rel.affection);
    }

    #[test]
    fn test_negative_relationships_persist() {
        let mut enemy_rel = SocialRelationship::new(Uuid::new_v4(), 0);
        enemy_rel.trust = -0.8;
        enemy_rel.affection = -0.8;

        let initial_trust = enemy_rel.trust;

        // Fast forward 2000 ticks
        for tick in 1..2001 {
            enemy_rel.decay(tick);
        }

        // Negative relationship should decay slower (still somewhat negative)
        // With 0.5x decay rate on negative relationships, it should still be negative
        assert!(enemy_rel.trust < -0.2); // Still negative after 2000 ticks
        assert!(enemy_rel.affection < -0.2);
    }

    #[test]
    fn test_agents_to_avoid() {
        let mut memory = Memory::new();
        let friend_id = Uuid::new_v4();
        let enemy_id = Uuid::new_v4();

        // Create positive relationship
        memory.record_interaction(friend_id, true, 1.0);

        // Create strong negative relationship (needs multiple strong interactions to reach -0.3)
        for _ in 0..5 {
            memory.record_interaction(enemy_id, false, 1.0);
        }

        let avoid_list = memory.agents_to_avoid();
        assert!(avoid_list.contains(&enemy_id));
        assert!(!avoid_list.contains(&friend_id));
    }

    #[test]
    fn test_most_liked_disliked() {
        let mut memory = Memory::new();
        let friend_id = Uuid::new_v4();
        let neutral_id = Uuid::new_v4();
        let enemy_id = Uuid::new_v4();

        // Create varied relationships
        memory.record_interaction(friend_id, true, 1.0);
        memory.record_interaction(friend_id, true, 1.0);
        memory.record_interaction(neutral_id, true, 0.1);
        memory.record_interaction(enemy_id, false, 1.0);
        memory.record_interaction(enemy_id, false, 1.0);

        assert_eq!(memory.most_liked_agent(), Some(friend_id));
        assert_eq!(memory.most_disliked_agent(), Some(enemy_id));
    }

    #[test]
    fn test_strong_bonds() {
        let mut memory = Memory::new();
        let parent_id = Uuid::new_v4();
        let friend_id = Uuid::new_v4();
        let acquaintance_id = Uuid::new_v4();

        memory.mark_as_parent(parent_id);

        // Need many interactions to reach 0.6 trust AND affection (each interaction adds 0.1 * strength)
        for _ in 0..8 {
            memory.record_interaction(friend_id, true, 1.0);
        }
        memory.record_interaction(acquaintance_id, true, 0.3);

        let bonds = memory.strong_bonds();
        assert!(bonds.contains(&parent_id)); // Family always strong
        assert!(bonds.contains(&friend_id)); // High trust/affection from many interactions
        assert!(!bonds.contains(&acquaintance_id)); // Weak relationship
    }
}
