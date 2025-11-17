// src/core/episodic_memory.rs
//! Episodic memory system for remembering specific events and experiences.
//!
//! Episodic memories are autobiographical memories of specific events,
//! situations, and experiences. They include:
//! - What happened (event details)
//! - When it happened (temporal context)
//! - Where it happened (spatial context)
//! - Who was involved (social context)
//! - How it felt (emotional context)

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::VecDeque;

/// Type of episodic event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EpisodeType {
    /// Social interaction
    SocialInteraction,
    /// Resource gathering
    ResourceGathering,
    /// Combat or conflict
    Combat,
    /// Discovery or exploration
    Discovery,
    /// Crafting or building
    Creation,
    /// Eating or drinking
    Consumption,
    /// Birth or death
    LifeEvent,
    /// Learning something new
    Learning,
    /// Emotional experience
    EmotionalEvent,
    /// Threat or danger
    Threat,
    /// Achievement or milestone
    Achievement,
    /// Failure or setback
    Failure,
}

impl EpisodeType {
    /// Get base emotional intensity for this episode type
    pub fn base_emotional_intensity(&self) -> f32 {
        match self {
            EpisodeType::SocialInteraction => 0.3,
            EpisodeType::ResourceGathering => 0.2,
            EpisodeType::Combat => 0.9,
            EpisodeType::Discovery => 0.7,
            EpisodeType::Creation => 0.5,
            EpisodeType::Consumption => 0.1,
            EpisodeType::LifeEvent => 1.0,
            EpisodeType::Learning => 0.6,
            EpisodeType::EmotionalEvent => 0.8,
            EpisodeType::Threat => 0.9,
            EpisodeType::Achievement => 0.8,
            EpisodeType::Failure => 0.7,
        }
    }

    /// Should this type be consolidated into long-term memory?
    pub fn should_consolidate(&self) -> bool {
        matches!(self,
            EpisodeType::LifeEvent |
            EpisodeType::Achievement |
            EpisodeType::Combat |
            EpisodeType::Discovery |
            EpisodeType::Learning
        )
    }
}

/// A single episodic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    pub episode_type: EpisodeType,

    /// When this happened
    pub timestamp: u64,

    /// Where this happened
    pub location: Option<(i32, i32, i32)>,

    /// Who was involved
    pub participants: Vec<Uuid>,

    /// What happened (description)
    pub description: String,

    /// Emotional valence (-1.0 negative to 1.0 positive)
    pub emotional_valence: f32,

    /// Emotional intensity (0.0 to 1.0)
    pub emotional_intensity: f32,

    /// Memory strength (0.0 to 1.0, decays over time)
    pub strength: f32,

    /// How many times this memory has been recalled
    pub recall_count: u32,

    /// Last time this memory was recalled
    pub last_recalled: u64,

    /// Is this a consolidated long-term memory?
    pub consolidated: bool,

    /// Related episode IDs (memories that occurred around the same time)
    pub related_episodes: Vec<Uuid>,

    /// Tags for retrieval
    pub tags: Vec<String>,
}

impl Episode {
    pub fn new(
        episode_type: EpisodeType,
        timestamp: u64,
        description: String,
        emotional_valence: f32,
    ) -> Self {
        let intensity = episode_type.base_emotional_intensity();

        Self {
            id: Uuid::new_v4(),
            episode_type,
            timestamp,
            location: None,
            participants: Vec::new(),
            description,
            emotional_valence: emotional_valence.clamp(-1.0, 1.0),
            emotional_intensity: intensity,
            strength: 1.0,
            recall_count: 0,
            last_recalled: timestamp,
            consolidated: false,
            related_episodes: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add location context
    pub fn with_location(mut self, location: (i32, i32, i32)) -> Self {
        self.location = Some(location);
        self
    }

    /// Add participants
    pub fn with_participants(mut self, participants: Vec<Uuid>) -> Self {
        self.participants = participants;
        self
    }

    /// Add tags for retrieval
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Recall this memory (reinforces it)
    pub fn recall(&mut self, current_time: u64) {
        self.recall_count += 1;
        self.last_recalled = current_time;
        // Recalling strengthens memory
        self.strength = (self.strength + 0.1).min(1.0);
    }

    /// Apply time-based decay
    pub fn decay(&mut self, decay_rate: f32) {
        // More emotional memories decay slower
        let emotional_factor = 1.0 - (self.emotional_intensity * 0.5);
        // Frequently recalled memories decay slower
        let recall_factor = 1.0 / (1.0 + (self.recall_count as f32 * 0.1));

        let effective_decay = decay_rate * emotional_factor * recall_factor;
        self.strength = (self.strength - effective_decay).max(0.0);
    }

    /// Should this memory be forgotten?
    pub fn should_forget(&self) -> bool {
        self.strength < 0.1 && !self.consolidated
    }

    /// Get age of memory in ticks
    pub fn age(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.timestamp)
    }

    /// Calculate retrieval probability based on current context
    pub fn retrieval_probability(
        &self,
        current_emotion: f32,
        current_location: Option<(i32, i32, i32)>,
        present_agents: &[Uuid],
    ) -> f32 {
        let mut prob = self.strength;

        // Emotional congruence increases retrieval
        let emotion_diff = (self.emotional_valence - current_emotion).abs();
        prob *= 1.0 - (emotion_diff * 0.3);

        // Location match increases retrieval
        if let (Some(mem_loc), Some(curr_loc)) = (self.location, current_location) {
            if mem_loc == curr_loc {
                prob *= 1.5;
            }
        }

        // Presence of participants increases retrieval
        for participant in &self.participants {
            if present_agents.contains(participant) {
                prob *= 1.3;
                break;
            }
        }

        // Recent recall increases accessibility
        prob *= 1.0 + (self.recall_count as f32 * 0.05);

        prob.min(1.0)
    }
}

/// Episodic memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    /// All episodes, ordered by timestamp
    episodes: VecDeque<Episode>,

    /// Maximum number of episodes to store
    max_episodes: usize,

    /// Decay rate per tick
    decay_rate: f32,

    /// Current time
    current_time: u64,
}

impl EpisodicMemory {
    pub fn new(max_episodes: usize) -> Self {
        Self {
            episodes: VecDeque::new(),
            max_episodes,
            decay_rate: 0.001, // 0.1% per tick
            current_time: 0,
        }
    }

    /// Add a new episode
    pub fn add_episode(&mut self, episode: Episode) {
        // Remove oldest if at capacity
        if self.episodes.len() >= self.max_episodes {
            self.episodes.pop_front();
        }

        self.episodes.push_back(episode);
    }

    /// Tick the memory system
    pub fn tick(&mut self, current_time: u64) {
        self.current_time = current_time;

        // Decay all episodes
        for episode in &mut self.episodes {
            episode.decay(self.decay_rate);
        }

        // Remove forgotten episodes
        self.episodes.retain(|e| !e.should_forget());
    }

    /// Recall episodes matching criteria
    pub fn recall_episodes(
        &mut self,
        episode_type: Option<EpisodeType>,
        limit: usize,
    ) -> Vec<&Episode> {
        let mut matching: Vec<&Episode> = self.episodes
            .iter()
            .filter(|e| {
                if let Some(etype) = episode_type {
                    e.episode_type == etype
                } else {
                    true
                }
            })
            .collect();

        // Sort by strength (strongest first)
        matching.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap());

        matching.into_iter().take(limit).collect()
    }

    /// Context-based retrieval
    pub fn recall_by_context(
        &mut self,
        current_emotion: f32,
        current_location: Option<(i32, i32, i32)>,
        present_agents: &[Uuid],
        limit: usize,
    ) -> Vec<&Episode> {
        // Calculate retrieval probabilities
        let mut episodes_with_prob: Vec<(usize, f32)> = self.episodes
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let prob = e.retrieval_probability(current_emotion, current_location, present_agents);
                (i, prob)
            })
            .collect();

        // Sort by probability
        episodes_with_prob.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Capture current_time and indices before mutable borrow
        let current_time = self.current_time;
        let indices: Vec<usize> = episodes_with_prob
            .into_iter()
            .take(limit)
            .map(|(idx, _)| idx)
            .collect();

        // Mark episodes as recalled (mutable borrow)
        for idx in &indices {
            if let Some(episode) = self.episodes.get_mut(*idx) {
                episode.recall(current_time);
            }
        }

        // Collect immutable references
        indices
            .iter()
            .filter_map(|idx| self.episodes.get(*idx))
            .collect()
    }

    /// Get recent episodes (last N ticks)
    pub fn recent_episodes(&self, ticks: u64) -> Vec<&Episode> {
        let cutoff = self.current_time.saturating_sub(ticks);
        self.episodes
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .collect()
    }

    /// Get episodes involving a specific agent
    pub fn episodes_with_agent(&self, agent_id: Uuid) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|e| e.participants.contains(&agent_id))
            .collect()
    }

    /// Get episodes at a specific location
    pub fn episodes_at_location(&self, location: (i32, i32, i32)) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|e| e.location == Some(location))
            .collect()
    }

    /// Get strongest memories
    pub fn strongest_memories(&self, limit: usize) -> Vec<&Episode> {
        let mut episodes: Vec<&Episode> = self.episodes.iter().collect();
        episodes.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap());
        episodes.into_iter().take(limit).collect()
    }

    /// Consolidate memories (move important ones to long-term)
    pub fn consolidate_memories(&mut self) {
        for episode in &mut self.episodes {
            if episode.episode_type.should_consolidate() && episode.strength > 0.5 && !episode.consolidated {
                episode.consolidated = true;
                episode.strength = 1.0; // Refresh consolidated memories
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> EpisodicMemoryStats {
        let mut type_counts = std::collections::HashMap::new();
        let mut total_strength = 0.0;
        let mut consolidated_count = 0;

        for episode in &self.episodes {
            *type_counts.entry(episode.episode_type).or_insert(0) += 1;
            total_strength += episode.strength;
            if episode.consolidated {
                consolidated_count += 1;
            }
        }

        EpisodicMemoryStats {
            total_episodes: self.episodes.len(),
            consolidated_episodes: consolidated_count,
            average_strength: if self.episodes.is_empty() {
                0.0
            } else {
                total_strength / self.episodes.len() as f32
            },
            episodes_by_type: type_counts,
        }
    }

    /// Get episode count
    pub fn episode_count(&self) -> usize {
        self.episodes.len()
    }

    /// Clear all episodes
    pub fn clear(&mut self) {
        self.episodes.clear();
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new(500) // Default: remember last 500 episodes
    }
}

/// Statistics about episodic memory
#[derive(Debug, Clone)]
pub struct EpisodicMemoryStats {
    pub total_episodes: usize,
    pub consolidated_episodes: usize,
    pub average_strength: f32,
    pub episodes_by_type: std::collections::HashMap<EpisodeType, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_creation() {
        let episode = Episode::new(
            EpisodeType::SocialInteraction,
            100,
            "Met a friend".to_string(),
            0.5,
        );

        assert_eq!(episode.episode_type, EpisodeType::SocialInteraction);
        assert_eq!(episode.emotional_valence, 0.5);
        assert_eq!(episode.strength, 1.0);
        assert_eq!(episode.recall_count, 0);
    }

    #[test]
    fn test_episode_decay() {
        let mut episode = Episode::new(
            EpisodeType::ResourceGathering,
            0,
            "Found berries".to_string(),
            0.3,
        );

        let initial_strength = episode.strength;
        episode.decay(0.01);

        assert!(episode.strength < initial_strength);
    }

    #[test]
    fn test_episode_recall() {
        let mut episode = Episode::new(
            EpisodeType::Discovery,
            0,
            "Found cave".to_string(),
            0.7,
        );

        episode.recall(100);

        assert_eq!(episode.recall_count, 1);
        assert_eq!(episode.last_recalled, 100);
        assert!(episode.strength > 1.0 - 0.01); // Should be reinforced
    }

    #[test]
    fn test_episodic_memory_add() {
        let mut memory = EpisodicMemory::new(10);

        let episode = Episode::new(
            EpisodeType::Combat,
            0,
            "Fought wolf".to_string(),
            -0.5,
        );

        memory.add_episode(episode);
        assert_eq!(memory.episode_count(), 1);
    }

    #[test]
    fn test_episodic_memory_capacity() {
        let mut memory = EpisodicMemory::new(3);

        for i in 0..5 {
            let episode = Episode::new(
                EpisodeType::ResourceGathering,
                i,
                format!("Event {}", i),
                0.0,
            );
            memory.add_episode(episode);
        }

        assert_eq!(memory.episode_count(), 3); // Should cap at 3
    }

    #[test]
    fn test_recall_by_type() {
        let mut memory = EpisodicMemory::new(10);

        memory.add_episode(Episode::new(
            EpisodeType::Combat,
            0,
            "Fight 1".to_string(),
            -0.5,
        ));
        memory.add_episode(Episode::new(
            EpisodeType::SocialInteraction,
            1,
            "Talk 1".to_string(),
            0.5,
        ));
        memory.add_episode(Episode::new(
            EpisodeType::Combat,
            2,
            "Fight 2".to_string(),
            -0.7,
        ));

        let combat_episodes = memory.recall_episodes(Some(EpisodeType::Combat), 10);
        assert_eq!(combat_episodes.len(), 2);
    }

    #[test]
    fn test_context_based_retrieval() {
        let mut memory = EpisodicMemory::new(10);

        let agent_id = Uuid::new_v4();
        let episode = Episode::new(
            EpisodeType::SocialInteraction,
            0,
            "Met friend".to_string(),
            0.8,
        ).with_participants(vec![agent_id])
         .with_location((10, 10, 0));

        memory.add_episode(episode);

        // Should have high retrieval probability when agent is present
        let recalled = memory.recall_by_context(
            0.8,
            Some((10, 10, 0)),
            &[agent_id],
            5,
        );

        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].recall_count, 1); // Should be marked as recalled
    }

    #[test]
    fn test_recent_episodes() {
        let mut memory = EpisodicMemory::new(10);

        memory.add_episode(Episode::new(
            EpisodeType::Discovery,
            0,
            "Old discovery".to_string(),
            0.5,
        ));
        memory.add_episode(Episode::new(
            EpisodeType::Discovery,
            1000,
            "Recent discovery".to_string(),
            0.5,
        ));

        memory.current_time = 1100;
        let recent = memory.recent_episodes(200);

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].description, "Recent discovery");
    }

    #[test]
    fn test_memory_consolidation() {
        let mut memory = EpisodicMemory::new(10);

        memory.add_episode(Episode::new(
            EpisodeType::LifeEvent,
            0,
            "Important event".to_string(),
            1.0,
        ));
        memory.add_episode(Episode::new(
            EpisodeType::ResourceGathering,
            1,
            "Gathered wood".to_string(),
            0.2,
        ));

        memory.consolidate_memories();

        let stats = memory.stats();
        assert_eq!(stats.consolidated_episodes, 1); // Only LifeEvent should consolidate
    }

    #[test]
    fn test_emotional_memories_decay_slower() {
        let mut emotional = Episode::new(
            EpisodeType::LifeEvent,
            0,
            "Birth of child".to_string(),
            1.0,
        );

        let mut mundane = Episode::new(
            EpisodeType::ResourceGathering,
            0,
            "Found stick".to_string(),
            0.1,
        );

        for _ in 0..100 {
            emotional.decay(0.01);
            mundane.decay(0.01);
        }

        assert!(emotional.strength > mundane.strength);
    }
}
