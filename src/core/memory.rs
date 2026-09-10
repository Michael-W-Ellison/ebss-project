// src/core/memory.rs
//! Memory system for agents with dynamic expansion and management.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
            id: crate::core::dice::name(),
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

impl SpatialMemoryType {
    /// How much forgetting this place would cost.
    ///
    /// A spatial memory had no importance at all: `SpatialMemory::decay` took
    /// a flat thousandth a tick, so **the pit a man dug and filled with his
    /// winter food was forgotten at exactly the rate of a bush he once glanced
    /// at**. `MemoryImportance::decay_multiplier` has described five bands of
    /// this since memories were written and only the episodic entries ever
    /// read it - a table with a reader for half its callers, which is this
    /// project's recurring defect wearing its plainest face.
    ///
    /// The arithmetic it was hiding: confidence starts at 1.0, `recall_locations`
    /// wants it above 0.3, and a flat thousandth a tick spends that in 700
    /// ticks - **fourteen and a half days**. The lean season is seventy-five.
    /// So a store laid down in autumn was forgotten a fortnight later and its
    /// owner starved thirty paces from it: measured, of the turns taken by a
    /// body under a quarter of its reserve, **0.6% could remember a store at
    /// all**, while the settlement's pits held two thousand items.
    ///
    /// A store is Critical because it is a thing you made, not a thing you
    /// noticed, and because the whole point of making it was to come back to
    /// it in a season when nothing else will feed you. Water next, because
    /// thirst kills in three days. A bush is ordinary: bushes come and go and
    /// being wrong about one costs a walk.
    pub fn how_much_this_matters(&self) -> MemoryImportance {
        match self {
            // Somewhere you buried food, for a winter you could see coming.
            SpatialMemoryType::Storage => MemoryImportance::Critical,
            // Thirst is the fastest of the slow deaths.
            SpatialMemoryType::Water => MemoryImportance::Important,
            // What has hurt you, and where you sleep.
            SpatialMemoryType::Danger | SpatialMemoryType::Shelter => MemoryImportance::Important,
            // A bush, a seam, a thing left lying. Worth knowing and no
            // disaster to be wrong about.
            SpatialMemoryType::Food | SpatialMemoryType::Resource | SpatialMemoryType::Tool => {
                MemoryImportance::Normal
            }
        }
    }
}

/// A remembered place, and what was there.
///
/// **What a place is remembered *as* is a fact about the rememberer, not about
/// the place.** A man who knows what clay is for remembers a clay bank; a man
/// who does not remembers that there is something on that bend of the river.
/// Both of them saw the same mud. That is what `what_it_is` is for, and it is
/// the whole difference between a map memory that can serve a craft and one
/// that can only serve an appetite.
///
/// It also degrades in the right order. Confidence falls, and the *name* goes
/// before the *place* does: "flax grows at that field edge" becomes "there is
/// something worth having in that valley" becomes nothing. One clock and two
/// thresholds rather than two clocks, so a memory cannot be sure what was
/// there and unsure where.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialMemory {
    pub memory_type: SpatialMemoryType,
    pub position: (i32, i32, i32),
    pub last_seen: u32, // Tick when last observed
    pub confidence: f32, // 0.0 to 1.0, decays over time
    pub value: f32, // Estimated value/usefulness
    /// What was there, where the rememberer knew what it was for.
    ///
    /// `None` is not ignorance of the place - it is a place known without a
    /// name, which is what a resource an agent has no use for comes to. See
    /// `what_i_could_name_it`.
    #[serde(default)]
    pub what_it_is: Option<String>,
}

impl SpatialMemory {
    pub fn new(memory_type: SpatialMemoryType, position: (i32, i32, i32), tick: u32) -> Self {
        Self {
            memory_type,
            position,
            last_seen: tick,
            confidence: 1.0,
            value: 1.0,
            what_it_is: None,
        }
    }

    /// What forgetting this place would cost.
    ///
    /// **A place you have no use for is worth less than a place you have.**
    /// This is the retention half of the specificity rule, and it is the half
    /// that costs nothing: a man who cannot name what was there loses it a
    /// band faster and goes over the side first when the shelf is full, and
    /// nobody walks anywhere on the strength of it.
    ///
    /// The type still decides the band - a store is a store - and the name
    /// decides whether he keeps the whole of it. Wanting it the other way
    /// round was measured: sending a man to fetch the material he remembers
    /// took person-days from 215,333 to 201,777 over two blocks of 32 worlds
    /// and cost four fifths of a settlement's finished burrows, because the
    /// thing worth doing is nearly always the thing under his feet and a walk
    /// displaces it. Knowing where the clay is turns out to be worth having
    /// and not worth crossing a valley for.
    pub fn what_forgetting_this_would_cost(&self) -> MemoryImportance {
        let matters = self.memory_type.how_much_this_matters();

        // **Only where naming is the point.** A pit a man dug, a roof he
        // worked on, a place that hurt him and the water he drinks are not
        // remembered *as* anything - `remember_location` files them without a
        // name because there is nothing to say beyond what they are. Demoting
        // those for want of a name made the winter store fade a band faster,
        // which is precisely the fault `how_much_this_matters` was written to
        // fix: measured, a store forgotten a fortnight after it was buried and
        // its owner starving thirty paces from it. Three tests caught it.
        //
        // A `Resource` is the one kind where the question means something: it
        // is a bank of clay to a potter and a patch of mud to everybody else.
        if !matches!(
            self.memory_type,
            SpatialMemoryType::Resource | SpatialMemoryType::Tool
        ) || self.what_it_is.is_some()
        {
            return matters;
        }

        match matters {
            MemoryImportance::Critical => MemoryImportance::Important,
            MemoryImportance::Important => MemoryImportance::Normal,
            MemoryImportance::Normal => MemoryImportance::Minor,
            MemoryImportance::Minor | MemoryImportance::Trivial => MemoryImportance::Trivial,
        }
    }

    /// What he would call this place now.
    ///
    /// The name is the first thing to go. Above `STILL_KNOWS_WHAT_IT_WAS` he
    /// can still say it was flax; below it he knows only that the valley was
    /// worth something, which is what a category memory is; below the 0.3
    /// `recall_locations` wants, nothing at all.
    ///
    /// Asked rather than stored, so there is one clock and the two tiers
    /// cannot disagree about how long ago this was.
    pub fn what_i_could_name_it(&self) -> Option<&str> {
        (self.confidence > Self::STILL_KNOWS_WHAT_IT_WAS)
            .then(|| self.what_it_is.as_deref())
            .flatten()
    }

    /// How sure a man has to be to still say what was there.
    ///
    /// Set so the name lasts about a fifth as long as the place - a berry
    /// patch named for three days and remembered as somewhere-worth-a-look for
    /// a fortnight, on the ordinary rate. The specification asks for ratios
    /// between one in three and one in twelve depending how the place was
    /// learned; one threshold cannot be all three, and this sits in the middle
    /// of them. Making it depend on *how* the place was learned is the next
    /// piece, not this one.
    pub const STILL_KNOWS_WHAT_IT_WAS: f32 = 0.85;

    /// Decay confidence over time
    pub fn forget_a_little(&mut self, ticks: u32) {
        // A thousandth a tick, at the pace this kind of place is forgotten -
        // see `SpatialMemoryType::how_much_this_matters`. It was flat, and a
        // winter store went the way of a berry bush.
        //
        // Denominated in **ticks since this was last called**, not in the tick
        // it is now. It used to take `current_tick` and read `last_seen`,
        // which is an absolute reading - correct if you call it once, and
        // quadratic if you call it every tick, because each call subtracts the
        // whole elapsed span again from an already-decayed confidence. Both
        // callers existed. `Memory::tick` called it every tick, so a memory
        // was gone in under a minute; `batch_decay_and_prune` called it every
        // hundred with its own copy of the arithmetic. Two callers with
        // opposite contracts and one function to satisfy them, which is why
        // neither was right. This has one meaning and both callers now say how
        // much time has passed.
        let spent = ticks as f32
            * Self::HOW_FAST_AN_ORDINARY_PLACE_IS_FORGOTTEN
            * self.what_forgetting_this_would_cost().decay_multiplier();
        self.confidence = (self.confidence - spent).max(0.0);
    }

    /// What a tick costs an ordinary place's confidence.
    ///
    /// At `MemoryImportance::Normal` this spends a fresh memory's confidence
    /// down to the 0.3 `recall_locations` wants in 700 ticks, which is a
    /// fortnight - about right for a bush somebody walked past once.
    pub const HOW_FAST_AN_ORDINARY_PLACE_IS_FORGOTTEN: f32 = 0.001;

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
                memory.forget_a_little(1);
            }
            // Remove very old, low-confidence memories
            self.spatial_memories.retain(|m| m.confidence > self.config.forget_threshold);
        }
    }

    /// Perform batch decay and pruning (efficient for large populations)
    fn batch_decay_and_prune(&mut self) {
        // One spelling of forgetting, and this is not where it lives.
        //
        // This had its own copy of the thousandth-a-tick rule, ignored
        // `how_much_this_matters`, and then multiplied by `prune_interval` on
        // top of `time_elapsed` - which double-counts, because `decay` already
        // measures from `last_seen` and is not an increment. With the default
        // interval of a hundred that came to **a tenth of confidence per tick
        // elapsed**: a fresh memory fell below the 0.3 `recall_locations` wants
        // in seven ticks and was pruned outright in nine, so **anywhere a
        // person had not looked in the last four hours was gone**.
        //
        // Measured over eight seeded world-years: of the turns taken by a body
        // under a quarter of its reserve, 0.6% could remember a store at all,
        // while the settlement's pits held two thousand items. Every world
        // emptied between day 315 and 350 with a full larder in the ground.
        //
        // `SpatialMemory::decay` computes from `last_seen` absolutely, so
        // calling it on an interval is exactly the same as calling it every
        // tick and there is nothing to accumulate.
        let since = self.config.prune_interval;
        for memory in &mut self.spatial_memories {
            memory.forget_a_little(since);
        }

        // Prune weak memories
        self.spatial_memories.retain(|m| m.confidence > self.config.forget_threshold);

        // Enforce max memory limit if set.
        //
// **What goes off a full shelf is decided by what a place is worth,
        // not only by how lately it was seen.** This sorted on confidence
        // alone, so a berry bush glanced at this morning outranked the pit a
        // man dug in the autumn and filled with his winter food -
        // `how_much_this_matters` set how fast each *fades* and had no say in
        // who was evicted. That is the table-with-half-its-readers fault this
        // project keeps finding, at the call site that most needed it.
        //
        // It does not bind today and this changes nothing measurable: a
        // thousand places is far more than anybody ever remembers, and a run
        // with this fixed came back byte-identical to one without. It was
        // found by filling the store and it is fixed rather than left for
        // whoever fills it next. See ISSUES_FOUND #194.
        if let Some(max) = self.config.max_memories {
            if self.spatial_memories.len() > max {
                self.spatial_memories.sort_by(|a, b| {
                    let worth =
                        |m: &SpatialMemory| m.what_forgetting_this_would_cost().decay_multiplier();

                    // Lower multiplier is a place that matters more, so it
                    // sorts first; confidence breaks the tie within a band.
                    worth(a)
                        .partial_cmp(&worth(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| {
                            b.confidence
                                .partial_cmp(&a.confidence)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                });
                self.spatial_memories.truncate(max);
            }
        }
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
        let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
        for memory in &self.spatial_memories {
            let type_name = format!("{:?}", memory.memory_type);
            *by_type.entry(type_name).or_insert(0) += 1;
        }
        // Add knowledge count
        *by_type.entry("Knowledge".to_string()).or_insert(0) += self.knowledge.len();

        // Categorize by strength/importance
        let mut by_importance: BTreeMap<String, usize> = BTreeMap::new();
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

    /// Add or update a spatial memory, and note how much was standing there.
    ///
    /// `SpatialMemory::value` has existed since the model had memories and was
    /// set to 1.0 for everything, so an agent remembered a spring and a puddle
    /// as the same place. Foraging and migration both read this store, and
    /// both of them chose between remembered places on distance alone.
    pub fn remember_how_much_is_there(
        &mut self,
        memory_type: SpatialMemoryType,
        position: (i32, i32, i32),
        how_much: u32,
    ) {
        self.remember_location(memory_type.clone(), position);

        if let Some(remembered) = self.spatial_memories.iter_mut().find(|m| {
            std::mem::discriminant(&m.memory_type) == std::mem::discriminant(&memory_type)
                && m.position == position
        }) {
            remembered.value = how_much as f32;
        }
    }

    /// Remember a place, what was there, and how much of it.
    ///
    /// The writer the specification asks for. `what_it_is` is what the
    /// *rememberer* could name, so the caller has already asked whether this
    /// agent knows what the stuff is for - see
    /// `Agent::do_i_know_what_this_is_for`. Passing `None` files the place
    /// under its category alone, which is what every memory in this model was
    /// until now.
    ///
    /// A name once learned is not unlearned by seeing the thing again without
    /// knowing it: the `or` below keeps a name a later sighting could not
    /// supply, because forgetting what a thing is called is what confidence is
    /// for and not something a glance should do.
    pub fn remember_this_here(
        &mut self,
        memory_type: SpatialMemoryType,
        position: (i32, i32, i32),
        what_it_is: Option<String>,
        how_much: u32,
    ) {
        self.remember_how_much_is_there(memory_type.clone(), position, how_much);

        if let Some(remembered) = self.spatial_memories.iter_mut().find(|m| {
            std::mem::discriminant(&m.memory_type) == std::mem::discriminant(&memory_type)
                && m.position == position
        }) {
            remembered.what_it_is = what_it_is.or(remembered.what_it_is.take());
        }
    }

    /// Forget a spatial memory that turned out to be wrong.
    ///
    /// Used when an agent travels to a remembered resource and finds nothing
    /// there; without this the agent keeps returning to an exhausted site.
    /// Returns true if a memory was removed.
    pub fn forget_location(
        &mut self,
        memory_type: SpatialMemoryType,
        position: (i32, i32, i32),
    ) -> bool {
        let before = self.spatial_memories.len();
        self.spatial_memories.retain(|m| {
            !(std::mem::discriminant(&m.memory_type) == std::mem::discriminant(&memory_type)
                && m.position == position)
        });
        self.spatial_memories.len() < before
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
    pub by_type: BTreeMap<String, usize>,
    /// Count of memories by strength/importance (e.g., "Strong", "Moderate", "Weak")
    pub by_importance: BTreeMap<String, usize>,
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
