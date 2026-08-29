// src/core/memory_manager.rs
//! Integrated memory management system that coordinates all memory types.
//!
//! This module provides a unified interface to:
//! - Episodic memory (autobiographical events)
//! - Working memory (current tasks and goals)
//! - Long-term memory (spatial, social, knowledge)
//! - Memory consolidation
//! - Memory-based decision making

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::{Memory, EpisodicMemory, WorkingMemory, Episode, EpisodeType, WorkingTask, TaskPriority};

/// Integrated memory manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManager {
    /// Long-term memory (spatial, social, knowledge)
    pub long_term: Memory,

    /// Episodic memory (autobiographical events)
    pub episodic: EpisodicMemory,

    /// Working memory (current tasks)
    pub working: WorkingMemory,

    /// Current time (for synchronization)
    current_time: u64,

    /// Consolidation settings
    consolidation_interval: u64,
    last_consolidation: u64,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            long_term: Memory::new(),
            episodic: EpisodicMemory::default(),
            working: WorkingMemory::default(),
            current_time: 0,
            consolidation_interval: 1000, // Consolidate every 1000 ticks
            last_consolidation: 0,
        }
    }

    /// Tick all memory systems
    pub fn tick(&mut self, current_time: u64) {
        self.current_time = current_time;

        self.long_term.tick();
        self.episodic.tick(current_time);
        self.working.tick(current_time);

        // Perform consolidation if needed
        if current_time.saturating_sub(self.last_consolidation) >= self.consolidation_interval {
            self.consolidate();
            self.last_consolidation = current_time;
        }
    }

    /// Record an event in episodic memory
    pub fn record_event(
        &mut self,
        episode_type: EpisodeType,
        description: String,
        emotional_valence: f32,
        location: Option<(i32, i32, i32)>,
        participants: Vec<Uuid>,
    ) -> Uuid {
        let mut episode = Episode::new(episode_type, self.current_time, description, emotional_valence);

        if let Some(loc) = location {
            episode = episode.with_location(loc);
        }

        if !participants.is_empty() {
            episode = episode.with_participants(participants);
        }

        let episode_id = episode.id;
        self.episodic.add_episode(episode);
        episode_id
    }

    /// Add a task to working memory
    pub fn add_task(
        &mut self,
        description: String,
        priority: TaskPriority,
    ) -> Result<Uuid, String> {
        let task = WorkingTask::new(description, priority, self.current_time);
        self.working.add_task(task)
    }

    /// Remember a location (adds to both episodic and long-term)
    pub fn remember_location(
        &mut self,
        memory_type: super::SpatialMemoryType,
        position: (i32, i32, i32),
        description: String,
    ) {
        // Add to long-term spatial memory
        self.long_term.remember_location(memory_type, position);

        // Create episodic memory of discovery
        self.record_event(
            EpisodeType::Discovery,
            description,
            0.5,
            Some(position),
            vec![],
        );
    }

    /// Record a social interaction
    pub fn remember_interaction(
        &mut self,
        other_agent: Uuid,
        positive: bool,
        strength: f32,
        description: String,
    ) {
        // Note: Social relationships are now tracked in Agent.relationships, not in Memory
        // This method only records the episodic memory of the interaction

        // Add to episodic memory
        let valence = if positive { strength } else { -strength };
        self.record_event(
            EpisodeType::SocialInteraction,
            description,
            valence,
            None,
            vec![other_agent],
        );
    }

    /// Consolidate memories (move important short-term to long-term)
    pub fn consolidate(&mut self) {
        // Consolidate important episodic memories
        self.episodic.consolidate_memories();

        // Archive completed tasks from working memory
        let completed: Vec<_> = self.working.tasks_with_status(super::TaskStatus::Completed)
            .iter()
            .map(|t| (*t).clone())
            .collect();

        for task in completed {
            if self.current_time.saturating_sub(task.created) > 100 {
                // Create episodic memory of completed task
                let valence = if task.status == super::TaskStatus::Completed { 0.3 } else { -0.3 };
                self.record_event(
                    EpisodeType::Achievement,
                    format!("Completed: {}", task.description),
                    valence,
                    task.location,
                    task.collaborators.clone(),
                );

                // Remove from working memory
                self.working.remove_task(task.id);
            }
        }
    }

    /// Retrieve memories relevant to current context
    pub fn recall_relevant(
        &mut self,
        current_emotion: f32,
        current_location: Option<(i32, i32, i32)>,
        present_agents: &[Uuid],
    ) -> Vec<&Episode> {
        self.episodic.recall_by_context(
            current_emotion,
            current_location,
            present_agents,
            10,
        )
    }

    /// Get decision-making context from memories
    pub fn get_decision_context(&self, location: Option<(i32, i32, i32)>) -> DecisionContext {
        // Get recent experiences
        let recent_episodes = self.episodic.recent_episodes(500);

        // Calculate average recent emotion
        let avg_emotion = if !recent_episodes.is_empty() {
            recent_episodes.iter()
                .map(|e| e.emotional_valence)
                .sum::<f32>() / recent_episodes.len() as f32
        } else {
            0.0
        };

        // Check for recent threats
        let recent_threats = recent_episodes.iter()
            .filter(|e| matches!(e.episode_type, EpisodeType::Threat | EpisodeType::Combat))
            .count();

        // Check for location history
        let location_familiarity = if let Some(loc) = location {
            let at_location = self.episodic.episodes_at_location(loc);
            (at_location.len() as f32 / 10.0).min(1.0)
        } else {
            0.0
        };

        // Note: Trusted/avoid agents are now tracked in Agent.relationships, not in Memory
        // DecisionContext consumers should get this from agent.relationships instead
        let trusted_agents = Vec::new();
        let avoid_agents = Vec::new();

        // Current task focus
        let current_task = self.working.get_focus().map(|t| t.description.clone());

        DecisionContext {
            recent_emotion: avg_emotion,
            recent_threats,
            location_familiarity,
            trusted_agents,
            avoid_agents,
            current_task,
            pending_tasks: self.working.pending_tasks().len(),
        }
    }

    /// Check if agent has experienced something similar before
    pub fn has_similar_experience(&mut self, episode_type: EpisodeType) -> bool {
        let episodes = self.episodic.recall_episodes(Some(episode_type), 1);
        !episodes.is_empty()
    }

    /// Get emotional association with an agent
    pub fn emotional_association_with(&self, agent_id: Uuid) -> f32 {
        let episodes = self.episodic.episodes_with_agent(agent_id);

        if episodes.is_empty() {
            return 0.0;
        }

        episodes.iter()
            .map(|e| e.emotional_valence)
            .sum::<f32>() / episodes.len() as f32
    }

    /// Get statistics about all memory systems
    pub fn stats(&self) -> MemoryManagerStats {
        MemoryManagerStats {
            episodic: self.episodic.stats(),
            working: self.working.stats(),
            spatial_locations: self.long_term.spatial_memories.len(),
            social_relationships: 0, // Relationships now tracked in Agent.relationships
            knowledge_items: self.long_term.knowledge.len(),
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for decision-making based on memories
#[derive(Debug, Clone)]
pub struct DecisionContext {
    /// Average emotional valence of recent episodes
    pub recent_emotion: f32,

    /// Number of recent threat/combat episodes
    pub recent_threats: usize,

    /// How familiar is the current location (0.0 to 1.0)
    pub location_familiarity: f32,

    /// List of trusted agent IDs
    pub trusted_agents: Vec<Uuid>,

    /// List of agent IDs to avoid
    pub avoid_agents: Vec<Uuid>,

    /// Current task description (if any)
    pub current_task: Option<String>,

    /// Number of pending tasks
    pub pending_tasks: usize,
}

impl DecisionContext {
    /// Should the agent be cautious in current context?
    pub fn should_be_cautious(&self) -> bool {
        self.recent_threats > 2 || !self.avoid_agents.is_empty()
    }

    /// Is the agent overwhelmed with tasks?
    pub fn is_overwhelmed(&self) -> bool {
        self.pending_tasks > 5
    }

    /// Is the agent in familiar territory?
    pub fn in_familiar_territory(&self) -> bool {
        self.location_familiarity > 0.5
    }

    /// Is the agent in a good emotional state?
    pub fn is_positive_mood(&self) -> bool {
        self.recent_emotion > 0.3
    }

    /// Should the agent seek social support?
    pub fn needs_social_support(&self) -> bool {
        self.recent_emotion < -0.3 && !self.trusted_agents.is_empty()
    }
}

/// Overall memory statistics
#[derive(Debug, Clone)]
pub struct MemoryManagerStats {
    pub episodic: super::EpisodicMemoryStats,
    pub working: super::WorkingMemoryStats,
    pub spatial_locations: usize,
    pub social_relationships: usize,
    pub knowledge_items: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::new();
        assert_eq!(manager.current_time, 0);
    }

    #[test]
    fn test_record_event() {
        let mut manager = MemoryManager::new();

        let episode_id = manager.record_event(
            EpisodeType::Discovery,
            "Found a cave".to_string(),
            0.7,
            Some((10, 20, 0)),
            vec![],
        );

        assert_eq!(manager.episodic.episode_count(), 1);
    }

    #[test]
    fn test_add_task() {
        let mut manager = MemoryManager::new();

        let task_id = manager.add_task(
            "Gather wood".to_string(),
            TaskPriority::Normal,
        ).unwrap();

        let stats = manager.working.stats();
        assert_eq!(stats.total_tasks, 1);
    }

    #[test]
    fn test_remember_location() {
        let mut manager = MemoryManager::new();

        manager.remember_location(
            crate::core::SpatialMemoryType::Food,
            (5, 5, 0),
            "Found berry bush".to_string(),
        );

        // Should be in both spatial and episodic memory
        assert!(!manager.long_term.spatial_memories.is_empty());
        assert_eq!(manager.episodic.episode_count(), 1);
    }

    #[test]
    fn test_remember_interaction() {
        let mut manager = MemoryManager::new();
        let other_agent = crate::core::dice::name();

        manager.remember_interaction(
            other_agent,
            true,
            0.8,
            "Had a nice conversation".to_string(),
        );

        // Note: Social relationships are now tracked in Agent.relationships, not Memory
        // This test now only verifies episodic memory
        assert_eq!(manager.episodic.episode_count(), 1);
    }

    #[test]
    fn test_consolidation() {
        let mut manager = MemoryManager::new();

        // Add an important event
        manager.record_event(
            EpisodeType::LifeEvent,
            "Birth of child".to_string(),
            1.0,
            None,
            vec![],
        );

        manager.consolidate();

        let stats = manager.episodic.stats();
        assert!(stats.consolidated_episodes > 0);
    }

    #[test]
    fn test_decision_context() {
        let mut manager = MemoryManager::new();
        manager.current_time = 0;

        // Add multiple positive experiences at the same location to establish familiarity
        // (location_familiarity = count / 10.0, need > 0.5, so need at least 6 episodes)
        for i in 0..6 {
            manager.record_event(
                EpisodeType::SocialInteraction,
                format!("Good talk {}", i),
                0.8,
                Some((0, 0, 0)),
                vec![],
            );
            manager.current_time += 10;
        }

        manager.current_time = 100;

        let context = manager.get_decision_context(Some((0, 0, 0)));
        assert!(context.is_positive_mood());
        assert!(context.in_familiar_territory());
    }

    #[test]
    fn test_has_similar_experience() {
        let mut manager = MemoryManager::new();

        manager.record_event(
            EpisodeType::Combat,
            "Fought wolf".to_string(),
            -0.7,
            None,
            vec![],
        );

        assert!(manager.has_similar_experience(EpisodeType::Combat));
        assert!(!manager.has_similar_experience(EpisodeType::Discovery));
    }

    #[test]
    fn test_emotional_association() {
        let mut manager = MemoryManager::new();
        let friend = crate::core::dice::name();

        // Multiple positive interactions
        for _ in 0..3 {
            manager.record_event(
                EpisodeType::SocialInteraction,
                "Hung out".to_string(),
                0.7,
                None,
                vec![friend],
            );
        }

        let association = manager.emotional_association_with(friend);
        assert!(association > 0.5);
    }

    #[test]
    fn test_recall_relevant() {
        let mut manager = MemoryManager::new();
        let agent = crate::core::dice::name();

        manager.record_event(
            EpisodeType::SocialInteraction,
            "Met someone".to_string(),
            0.5,
            Some((10, 10, 0)),
            vec![agent],
        );

        let recalled = manager.recall_relevant(
            0.5,
            Some((10, 10, 0)),
            &[agent],
        );

        assert_eq!(recalled.len(), 1);
    }
}
