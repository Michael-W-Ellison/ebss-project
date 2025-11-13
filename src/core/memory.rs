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
    }
}
