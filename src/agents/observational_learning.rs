// src/agents/observational_learning.rs
//! Observational learning system where agents learn behaviors by watching others.
//!
//! Key features:
//! - Children learn more easily from parents
//! - Learning rate based on relationship strength and trust
//! - Successful actions are more likely to be adopted
//! - Repeated observations increase learning probability

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Category of observable activity that can be learned through observation.
/// Each category represents a distinct type of activity with different
/// learning difficulties and observation requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservableActionType {
    /// Mining/harvesting resources
    Mining,
    /// Crafting items
    Crafting,
    /// Building structures
    Building,
    /// Combat actions
    Combat,
    /// Food preparation
    Cooking,
    /// Tool use
    ToolUse,
    /// Social interaction
    Social,
    /// Exploration/navigation
    Navigation,
    /// Problem solving
    ProblemSolving,
    /// Working the land - breaking ground, sowing, harvesting
    Farming,
}

/// Type alias for backwards compatibility
pub type ActionType = ObservableActionType;

impl ObservableActionType {
    /// Get learning difficulty (0.0 = easy, 1.0 = hard)
    pub fn learning_difficulty(&self) -> f32 {
        match self {
            ObservableActionType::Mining => 0.2,        // Simple, repetitive
            ObservableActionType::Crafting => 0.5,      // Requires planning
            ObservableActionType::Building => 0.6,      // Complex coordination
            ObservableActionType::Combat => 0.7,        // Dangerous, requires practice
            ObservableActionType::Cooking => 0.3,       // Moderate complexity
            ObservableActionType::ToolUse => 0.4,       // Requires understanding
            ObservableActionType::Social => 0.5,        // Context-dependent
            ObservableActionType::Navigation => 0.3,    // Learning paths
            ObservableActionType::ProblemSolving => 0.8, // Very complex
            ObservableActionType::Farming => 0.4,       // Patient work, plainly done
        }
    }

    /// Base number of observations needed to learn (before modifiers)
    pub fn observations_to_learn(&self) -> u32 {
        match self {
            ObservableActionType::Mining => 3,
            ObservableActionType::Crafting => 5,
            ObservableActionType::Building => 7,
            ObservableActionType::Combat => 10,
            ObservableActionType::Cooking => 4,
            ObservableActionType::ToolUse => 5,
            ObservableActionType::Social => 6,
            ObservableActionType::Navigation => 4,
            ObservableActionType::ProblemSolving => 12,
            ObservableActionType::Farming => 5,
        }
    }
}

/// Record of an observed action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedAction {
    /// Who performed the action
    pub performer: Uuid,
    /// What type of action
    pub action_type: ActionType,
    /// Whether the action succeeded
    pub success: bool,
    /// Specific details about the action (e.g., "crafted stone_axe")
    pub details: String,
    /// When this was observed
    pub timestamp: u64,
    /// How close the observer was (affects learning quality)
    pub observation_distance: f32,
}

impl ObservedAction {
    pub fn new(
        performer: Uuid,
        action_type: ActionType,
        success: bool,
        details: String,
        timestamp: u64,
        observation_distance: f32,
    ) -> Self {
        Self {
            performer,
            action_type,
            success,
            details,
            timestamp,
            observation_distance,
        }
    }

    /// Calculate observation quality (0.0 to 1.0)
    pub fn observation_quality(&self) -> f32 {
        // Closer observations are higher quality
        let distance_factor = (1.0 / (1.0 + self.observation_distance / 10.0)).min(1.0);

        // Successful actions are easier to learn from
        let success_factor = if self.success { 1.0 } else { 0.5 };

        distance_factor * success_factor
    }
}

/// Learning progress for a specific action type from a specific performer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningProgress {
    /// How many times this action was observed
    pub observation_count: u32,
    /// How many times it succeeded
    pub success_count: u32,
    /// Combined quality of all observations
    pub total_quality: f32,
    /// Whether the learner has adopted this behavior
    pub adopted: bool,
    /// Confidence in this learned behavior (0.0 to 1.0)
    pub confidence: f32,
}

impl LearningProgress {
    pub fn new() -> Self {
        Self {
            observation_count: 0,
            success_count: 0,
            total_quality: 0.0,
            adopted: false,
            confidence: 0.0,
        }
    }

    /// Record a new observation
    pub fn record_observation(&mut self, action: &ObservedAction) {
        self.observation_count += 1;
        if action.success {
            self.success_count += 1;
        }
        self.total_quality += action.observation_quality();

        // Update confidence
        self.update_confidence();
    }

    /// Update confidence based on observations
    fn update_confidence(&mut self) {
        if self.observation_count == 0 {
            self.confidence = 0.0;
            return;
        }

        // Success rate component
        let success_rate = self.success_count as f32 / self.observation_count as f32;

        // Average quality component
        let avg_quality = self.total_quality / self.observation_count as f32;

        // Number of observations component (diminishing returns)
        let observation_factor = (self.observation_count as f32 / 10.0).min(1.0);

        self.confidence = (success_rate * 0.5 + avg_quality * 0.3 + observation_factor * 0.2).clamp(0.0, 1.0);
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.observation_count == 0 {
            return 0.0;
        }
        self.success_count as f32 / self.observation_count as f32
    }

    /// Get average observation quality
    pub fn avg_quality(&self) -> f32 {
        if self.observation_count == 0 {
            return 0.0;
        }
        self.total_quality / self.observation_count as f32
    }
}

impl Default for LearningProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Observational learning system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationalLearning {
    /// Observations organized by (performer, action_type)
    observations: HashMap<(Uuid, ActionType), LearningProgress>,
    /// Recent observations (last 100)
    recent_observations: Vec<ObservedAction>,
    /// Maximum number of recent observations to keep
    max_recent: usize,
    /// Learning rate modifier (child = 1.5, adult = 1.0, elder = 0.7)
    learning_rate: f32,
}

impl ObservationalLearning {
    pub fn new(learning_rate: f32) -> Self {
        Self {
            observations: HashMap::new(),
            recent_observations: Vec::new(),
            max_recent: 100,
            learning_rate: learning_rate.clamp(0.1, 2.0),
        }
    }

    /// Record observing an action
    pub fn observe_action(&mut self, action: ObservedAction) {
        let key = (action.performer, action.action_type);

        let progress = self.observations.entry(key).or_insert_with(LearningProgress::new);
        progress.record_observation(&action);

        // Add to recent observations
        self.recent_observations.push(action);

        // Trim if too many
        if self.recent_observations.len() > self.max_recent {
            self.recent_observations.remove(0);
        }
    }

    /// Check if agent should adopt a behavior from observations
    ///
    /// # Arguments
    /// * `performer` - Who to learn from
    /// * `action_type` - What action type
    /// * `relationship_strength` - Bond strength with performer (-1.0 to 1.0)
    /// * `trust` - Trust in performer (0.0 to 1.0)
    ///
    /// Returns (should_adopt, confidence)
    pub fn should_adopt_behavior(
        &self,
        performer: &Uuid,
        action_type: ActionType,
        relationship_strength: f32,
        trust: f32,
    ) -> (bool, f32) {
        let key = (*performer, action_type);

        let Some(progress) = self.observations.get(&key) else {
            return (false, 0.0);
        };

        if progress.adopted {
            return (false, progress.confidence); // Already adopted
        }

        // Calculate learning threshold based on action difficulty
        let base_observations_needed = action_type.observations_to_learn();

        // Relationship modifier (parents/family = faster learning)
        let relationship_modifier = if relationship_strength > 0.6 {
            0.5 // 50% fewer observations needed for loved ones
        } else if relationship_strength > 0.3 {
            0.7 // 30% fewer for friends
        } else if relationship_strength > 0.0 {
            0.9 // 10% fewer for acquaintances
        } else {
            1.2 // 20% more for neutral/negative relationships
        };

        // Trust modifier
        let trust_modifier = 0.7 + (trust * 0.3); // 0.7 to 1.0 multiplier

        // Learning rate modifier (children learn faster)
        let adjusted_observations_needed = (base_observations_needed as f32
            * relationship_modifier
            * trust_modifier
            / self.learning_rate) as u32;

        // Check if enough observations
        let enough_observations = progress.observation_count >= adjusted_observations_needed;

        // Check if quality is high enough
        let high_quality = progress.avg_quality() > 0.5;

        // Check if success rate is reasonable
        let successful = progress.success_rate() > 0.6;

        let should_adopt = enough_observations && high_quality && successful;

        (should_adopt, progress.confidence)
    }

    /// Mark a behavior as adopted
    pub fn adopt_behavior(&mut self, performer: &Uuid, action_type: ActionType) {
        let key = (*performer, action_type);
        if let Some(progress) = self.observations.get_mut(&key) {
            progress.adopted = true;
        }
    }

    /// Get learning progress for a specific performer and action
    pub fn get_progress(&self, performer: &Uuid, action_type: ActionType) -> Option<&LearningProgress> {
        self.observations.get(&(*performer, action_type))
    }

    /// Get all adopted behaviors
    pub fn get_adopted_behaviors(&self) -> Vec<(Uuid, ActionType, f32)> {
        self.observations
            .iter()
            .filter(|(_, progress)| progress.adopted)
            .map(|((performer, action_type), progress)| (*performer, *action_type, progress.confidence))
            .collect()
    }


    /// Get all performers being learned from
    pub fn get_all_teachers(&self) -> Vec<Uuid> {
        self.observations
            .keys()
            .map(|(performer, _)| *performer)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }


    /// Get count of recent observations of a specific action type
    pub fn count_recent_observations_of_type(&self, action_type: ActionType, tick_window: u32, current_tick: u32) -> usize {
        self.recent_observations.iter()
            .filter(|obs| {
                obs.action_type == action_type &&
                current_tick.saturating_sub(obs.timestamp as u32) <= tick_window
            })
            .count()
    }

    /// Set learning rate (for age-based changes)
    pub fn set_learning_rate(&mut self, rate: f32) {
        self.learning_rate = rate.clamp(0.1, 2.0);
    }

    /// Get learning rate
    pub fn learning_rate(&self) -> f32 {
        self.learning_rate
    }
}

impl Default for ObservationalLearning {
    fn default() -> Self {
        Self::new(1.0) // Adult learning rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_difficulty() {
        assert!(ActionType::Mining.learning_difficulty() < ActionType::Combat.learning_difficulty());
        assert!(ActionType::ProblemSolving.learning_difficulty() > ActionType::Cooking.learning_difficulty());
    }

    #[test]
    fn test_observed_action_quality() {
        let performer = Uuid::new_v4();

        // Close, successful observation
        let good_obs = ObservedAction::new(
            performer,
            ActionType::Mining,
            true,
            "mined stone".to_string(),
            0,
            5.0,
        );
        assert!(good_obs.observation_quality() > 0.6); // Distance 5 gives ~0.666

        // Distant, failed observation
        let bad_obs = ObservedAction::new(
            performer,
            ActionType::Mining,
            false,
            "failed mining".to_string(),
            0,
            50.0,
        );
        assert!(bad_obs.observation_quality() < 0.3);
    }

    #[test]
    fn test_learning_progress() {
        let mut progress = LearningProgress::new();
        let performer = Uuid::new_v4();

        assert_eq!(progress.observation_count, 0);
        assert_eq!(progress.confidence, 0.0);

        // Record successful observation
        let obs = ObservedAction::new(
            performer,
            ActionType::Mining,
            true,
            "mined stone".to_string(),
            0,
            5.0,
        );
        progress.record_observation(&obs);

        assert_eq!(progress.observation_count, 1);
        assert_eq!(progress.success_count, 1);
        assert!(progress.confidence > 0.0);
    }

    #[test]
    fn test_observational_learning_basic() {
        let mut learning = ObservationalLearning::new(1.0);
        let performer = Uuid::new_v4();

        let obs = ObservedAction::new(
            performer,
            ActionType::Mining,
            true,
            "mined stone".to_string(),
            0,
            5.0,
        );

        learning.observe_action(obs);

        assert_eq!(learning.recent_observations.len(), 1);
        assert!(learning.get_progress(&performer, ActionType::Mining).is_some());
    }

    #[test]
    fn test_should_adopt_from_parent() {
        let mut learning = ObservationalLearning::new(1.5); // Child learning rate
        let parent_id = Uuid::new_v4();

        // Observe parent mining several times
        for i in 0..5 {
            let obs = ObservedAction::new(
                parent_id,
                ActionType::Mining,
                true,
                format!("mined stone {}", i),
                i,
                3.0,
            );
            learning.observe_action(obs);
        }

        // High relationship strength (parent) and trust
        let (should_adopt, confidence) = learning.should_adopt_behavior(
            &parent_id,
            ActionType::Mining,
            0.9, // Strong parent bond
            0.8, // High trust
        );

        assert!(should_adopt, "Child should learn mining from parent after 5 observations");
        assert!(confidence > 0.5);
    }

    #[test]
    fn test_should_not_adopt_from_stranger() {
        let mut learning = ObservationalLearning::new(1.0);
        let stranger_id = Uuid::new_v4();

        // Observe stranger once
        let obs = ObservedAction::new(
            stranger_id,
            ActionType::Combat,
            true,
            "won fight".to_string(),
            0,
            10.0,
        );
        learning.observe_action(obs);

        // Low relationship and neutral trust
        let (should_adopt, _) = learning.should_adopt_behavior(
            &stranger_id,
            ActionType::Combat,
            0.0, // No relationship
            0.5, // Neutral trust
        );

        assert!(!should_adopt, "Should not learn combat from stranger after 1 observation");
    }

    #[test]
    fn test_adopt_behavior() {
        let mut learning = ObservationalLearning::new(1.0);
        let teacher_id = Uuid::new_v4();

        // Observe and adopt
        for i in 0..10 {
            let obs = ObservedAction::new(
                teacher_id,
                ActionType::Crafting,
                true,
                format!("crafted item {}", i),
                i,
                5.0,
            );
            learning.observe_action(obs);
        }

        learning.adopt_behavior(&teacher_id, ActionType::Crafting);

        let progress = learning.get_progress(&teacher_id, ActionType::Crafting).unwrap();
        assert!(progress.adopted);

        // Should not adopt again
        let (should_adopt, _) = learning.should_adopt_behavior(
            &teacher_id,
            ActionType::Crafting,
            0.5,
            0.7,
        );
        assert!(!should_adopt);
    }

    #[test]
    fn test_get_adopted_behaviors() {
        let mut learning = ObservationalLearning::new(1.0);
        let teacher1 = Uuid::new_v4();
        let teacher2 = Uuid::new_v4();

        // Need to observe first before adopting
        learning.observe_action(ObservedAction::new(
            teacher1,
            ActionType::Mining,
            true,
            "mine".to_string(),
            0,
            5.0,
        ));

        learning.observe_action(ObservedAction::new(
            teacher2,
            ActionType::Crafting,
            true,
            "craft".to_string(),
            0,
            5.0,
        ));

        learning.adopt_behavior(&teacher1, ActionType::Mining);
        learning.adopt_behavior(&teacher2, ActionType::Crafting);

        let adopted = learning.get_adopted_behaviors();
        assert_eq!(adopted.len(), 2);
    }

    #[test]
    fn test_learning_rate_affects_adoption() {
        // Child learning (faster)
        let mut child_learning = ObservationalLearning::new(1.5);

        // Adult learning (normal)
        let mut adult_learning = ObservationalLearning::new(1.0);

        let parent_id = Uuid::new_v4();

        // Both observe same action 3 times
        for i in 0..3 {
            let obs = ObservedAction::new(
                parent_id,
                ActionType::Mining,
                true,
                format!("mined stone {}", i),
                i,
                5.0,
            );
            child_learning.observe_action(obs.clone());
            adult_learning.observe_action(obs);
        }

        // Child should be able to adopt (faster learning)
        let (child_adopts, _) = child_learning.should_adopt_behavior(
            &parent_id,
            ActionType::Mining,
            0.9, // Parent bond
            0.8,
        );

        // Adult might not be able to adopt yet
        let (adult_adopts, _) = adult_learning.should_adopt_behavior(
            &parent_id,
            ActionType::Mining,
            0.9,
            0.8,
        );

        assert!(child_adopts || !adult_adopts, "Child should learn faster than adult");
    }

    #[test]
    fn test_recent_observations_limit() {
        let mut learning = ObservationalLearning::new(1.0);
        learning.max_recent = 5;

        let performer = Uuid::new_v4();

        // Add 10 observations
        for i in 0..10 {
            let obs = ObservedAction::new(
                performer,
                ActionType::Mining,
                true,
                format!("action {}", i),
                i,
                5.0,
            );
            learning.observe_action(obs);
        }

        // Should only keep last 5
        assert_eq!(learning.recent_observations.len(), 5);
        assert_eq!(learning.recent_observations[0].details, "action 5");
    }

    #[test]
    fn test_get_all_teachers() {
        let mut learning = ObservationalLearning::new(1.0);
        let teacher1 = Uuid::new_v4();
        let teacher2 = Uuid::new_v4();

        learning.observe_action(ObservedAction::new(
            teacher1,
            ActionType::Mining,
            true,
            "mine".to_string(),
            0,
            5.0,
        ));

        learning.observe_action(ObservedAction::new(
            teacher2,
            ActionType::Crafting,
            true,
            "craft".to_string(),
            0,
            5.0,
        ));

        let teachers = learning.get_all_teachers();
        assert_eq!(teachers.len(), 2);
        assert!(teachers.contains(&teacher1));
        assert!(teachers.contains(&teacher2));
    }
}
