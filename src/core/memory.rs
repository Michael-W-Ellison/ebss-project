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
    /// How often to run batch pruning (in ticks) - optimizes large populations
    pub prune_interval: u32,
    /// Whether to use batch decay (more efficient for large populations)
    pub batch_decay: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memories: Some(1000), // Default limit of 1000 memories
            decay_rate: 0.001,        // Very slow decay (0.1% per tick)
            auto_forget: true,
            forget_threshold: 0.1,
            prune_interval: 100,      // Batch prune every 100 ticks
            batch_decay: true,        // Use batch decay by default
        }
    }
}

impl MemoryConfig {
    /// Create a config optimized for large populations
    pub fn for_large_population() -> Self {
        Self {
            max_memories: Some(500),  // Lower limit per agent
            decay_rate: 0.002,        // Slightly faster decay
            auto_forget: true,
            forget_threshold: 0.15,   // Forget weaker memories sooner
            prune_interval: 50,       // Prune more frequently
            batch_decay: true,
        }
    }

    /// Create a config for small populations with detailed memory
    pub fn for_small_population() -> Self {
        Self {
            max_memories: Some(2000), // Higher limit
            decay_rate: 0.0005,       // Slower decay
            auto_forget: true,
            forget_threshold: 0.05,   // Keep weaker memories longer
            prune_interval: 200,      // Prune less frequently
            batch_decay: false,       // Can afford per-tick decay
        }
    }
}

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

// Note: Relationship tracking has been consolidated into agents/emotions.rs
// All relationship functionality now uses RelationshipMap and Relationship from that module

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
/// Note: Social relationships are tracked in agents::emotions::RelationshipMap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub spatial_memories: Vec<SpatialMemory>,
    pub knowledge: Vec<KnowledgeMemory>,
    pub current_tick: u32,
    /// Configuration for memory behavior
    #[serde(default)]
    pub config: MemoryConfig,
    /// Tick counter for batch operations
    #[serde(default)]
    ticks_since_prune: u32,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            spatial_memories: Vec::new(),
            knowledge: Vec::new(),
            current_tick: 0,
            config: MemoryConfig::default(),
            ticks_since_prune: 0,
        }
    }

    /// Create memory with custom config
    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            spatial_memories: Vec::new(),
            knowledge: Vec::new(),
            current_tick: 0,
            config,
            ticks_since_prune: 0,
        }
    }

    /// Update memory for a new tick (optimized for large populations)
    pub fn tick(&mut self) {
        self.current_tick += 1;
        self.ticks_since_prune += 1;

        if self.config.batch_decay {
            // Batch mode: only decay and prune at intervals
            if self.ticks_since_prune >= self.config.prune_interval {
                self.batch_decay_and_prune();
                self.ticks_since_prune = 0;
            }
        } else {
            // Per-tick mode: decay every tick (original behavior)
            for memory in &mut self.spatial_memories {
                memory.decay(self.current_tick);
            }
            // Remove very old, low-confidence memories
            self.spatial_memories.retain(|m| m.confidence > self.config.forget_threshold);
        }
    }

    /// Perform batch decay and pruning (efficient for large populations)
    fn batch_decay_and_prune(&mut self) {
        let decay_multiplier = self.config.prune_interval as f32; // Accumulate decay

        // Apply accumulated decay to spatial memories
        for memory in &mut self.spatial_memories {
            // Apply decay proportional to ticks elapsed
            let time_elapsed = self.current_tick.saturating_sub(memory.last_seen);
            let decay = (time_elapsed as f32 * 0.001 * decay_multiplier).min(1.0);
            memory.confidence = (memory.confidence - decay).max(0.0);
        }

        // Prune weak memories
        self.spatial_memories.retain(|m| m.confidence > self.config.forget_threshold);

        // Enforce max memory limit if set
        if let Some(max) = self.config.max_memories {
            if self.spatial_memories.len() > max {
                // Sort by confidence and keep the strongest
                self.spatial_memories.sort_by(|a, b| {
                    b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
                });
                self.spatial_memories.truncate(max);
            }
        }
    }

    /// Force immediate pruning (useful when memory pressure is high)
    pub fn force_prune(&mut self) {
        self.batch_decay_and_prune();
        self.ticks_since_prune = 0;
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        let total_memories = self.spatial_memories.len() + self.knowledge.len();
        let average_strength = if self.spatial_memories.is_empty() {
            0.0
        } else {
            self.spatial_memories.iter().map(|m| m.confidence).sum::<f32>()
                / self.spatial_memories.len() as f32
        };

        // Count memories by type
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for memory in &self.spatial_memories {
            let type_name = format!("{:?}", memory.memory_type);
            *by_type.entry(type_name).or_insert(0) += 1;
        }
        // Add knowledge count
        *by_type.entry("Knowledge".to_string()).or_insert(0) += self.knowledge.len();

        // Categorize by strength/importance
        let mut by_importance: HashMap<String, usize> = HashMap::new();
        for memory in &self.spatial_memories {
            let importance = if memory.confidence > 0.8 {
                "Strong"
            } else if memory.confidence > 0.5 {
                "Moderate"
            } else if memory.confidence > 0.2 {
                "Weak"
            } else {
                "Fading"
            };
            *by_importance.entry(importance.to_string()).or_insert(0) += 1;
        }
        // Knowledge memories are assumed to be persistent/strong
        *by_importance.entry("Persistent".to_string()).or_insert(0) += self.knowledge.len();

        MemoryStats {
            total_memories,
            by_type,
            by_importance,
            average_strength,
        }
    }

    /// Check if memory is under pressure (near capacity)
    pub fn is_under_pressure(&self) -> bool {
        if let Some(max) = self.config.max_memories {
            self.spatial_memories.len() + self.knowledge.len() > (max * 9 / 10)
        } else {
            false
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
    /// Count of memories by type (e.g., "Food", "Water", "Knowledge")
    pub by_type: HashMap<String, usize>,
    /// Count of memories by strength/importance (e.g., "Strong", "Moderate", "Weak")
    pub by_importance: HashMap<String, usize>,
    pub average_strength: f32,
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
