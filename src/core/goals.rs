// src/core/goals.rs
//! Goal system for agents.
//!
//! Goals are divided into two types:
//! - Internal Goals: Satisfy emotional needs (invisible, psychological)
//! - External Goals: Achieve tangible objectives (visible, material)

use serde::{Deserialize, Serialize};
use crate::core::{EmotionType, DriveType};

/// Type of goal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoalType {
    Internal,  // Emotional needs
    External,  // Material objectives
}

/// Internal goals focus on satisfying emotional needs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InternalGoal {
    /// Increase a specific emotion
    IncreaseEmotion(EmotionType, f32), // (emotion_type, target_value)
    /// Decrease a specific emotion
    DecreaseEmotion(EmotionType, f32), // (emotion_type, target_value)
    /// Maintain emotional well-being above threshold
    MaintainWellBeing(f32), // (minimum_well_being)
    /// Reduce stress (combination of fear, anger, sadness)
    ReduceStress,
    /// Find entertainment/happiness
    SeekEntertainment,
}

/// External goals focus on tangible, observable objectives
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExternalGoal {
    /// Own a house
    OwnHouse,
    /// Stock house with sufficient food
    StockHouseFood(u32), // (target_amount)
    /// Stock town storehouse with food
    ContributeFoodToStorehouse(u32), // (target_amount)
    /// Have sufficient protection gear
    ObtainProtection,
    /// Stock storehouse with materials
    ContributeMaterialsToStorehouse(u32), // (target_amount)
    /// Ensure sufficient tools in storehouse
    EnsureToolsAvailable(u32), // (target_count)
    /// Craft a specific item
    CraftItem(String), // (item_name)
    /// Build a structure
    BuildStructure(String), // (structure_type)
    /// Gather specific resource
    GatherResource(String, u32), // (resource_type, amount)
    /// Learn a recipe or skill
    LearnSkill(String), // (skill_name)
    /// Form a relationship
    FormRelationship(String), // (relationship_type: friend, partner, etc.)
    /// Complete a job/task
    CompleteJob(String), // (job_name)
}

/// A goal with progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: uuid::Uuid,
    pub goal_type: GoalType,
    pub internal: Option<InternalGoal>,
    pub external: Option<ExternalGoal>,
    pub progress: f32, // 0.0 to 1.0
    pub priority: f32, // 0.0 to 1.0, how urgent/important
    pub created_at: u32, // Tick when goal was created
    pub completed: bool,
}

impl Goal {
    /// Create a new internal goal
    pub fn new_internal(internal_goal: InternalGoal, priority: f32, tick: u32) -> Self {
        Self {
            id: crate::core::dice::name(),
            goal_type: GoalType::Internal,
            internal: Some(internal_goal),
            external: None,
            progress: 0.0,
            priority,
            created_at: tick,
            completed: false,
        }
    }

    /// Create a new external goal
    pub fn new_external(external_goal: ExternalGoal, priority: f32, tick: u32) -> Self {
        Self {
            id: crate::core::dice::name(),
            goal_type: GoalType::External,
            internal: None,
            external: Some(external_goal),
            progress: 0.0,
            priority,
            created_at: tick,
            completed: false,
        }
    }

    /// Update progress towards goal
    pub fn update_progress(&mut self, amount: f32) {
        self.progress = (self.progress + amount).min(1.0);
        if self.progress >= 1.0 {
            self.completed = true;
        }
    }

    /// Check if goal aligns with a specific action
    pub fn aligns_with_action(&self, action: &str) -> bool {
        match &self.external {
            Some(ExternalGoal::CraftItem(item)) => action.contains(item),
            Some(ExternalGoal::BuildStructure(structure)) => action.contains(structure),
            Some(ExternalGoal::GatherResource(resource, _)) => action.contains(resource),
            Some(ExternalGoal::LearnSkill(skill)) => action.contains(skill),
            Some(ExternalGoal::CompleteJob(job)) => action.contains(job),
            _ => false,
        }
    }

    /// Get age of goal in ticks
    pub fn age(&self, current_tick: u32) -> u32 {
        current_tick.saturating_sub(self.created_at)
    }

    /// Check if this goal is already satisfied given current world state
    ///
    /// Used to interrupt plans when new information indicates the goal is complete.
    /// For example, if another agent already restocked the storehouse, this goal
    /// should be marked as satisfied so the agent can stop pursuing it.
    pub fn is_satisfied(&self, world_state: &GoalWorldState) -> bool {
        if self.completed {
            return true;
        }

        // Check internal goals first
        if let Some(internal) = &self.internal {
            return match internal {
                InternalGoal::IncreaseEmotion(emotion, target) => {
                    let emotion_key = format!("{:?}", emotion);
                    world_state.emotion_levels
                        .get(&emotion_key)
                        .map(|level| *level >= *target)
                        .unwrap_or(false)
                }
                InternalGoal::DecreaseEmotion(emotion, target) => {
                    let emotion_key = format!("{:?}", emotion);
                    world_state.emotion_levels
                        .get(&emotion_key)
                        .map(|level| *level <= *target)
                        .unwrap_or(false)
                }
                InternalGoal::MaintainWellBeing(threshold) => {
                    world_state.well_being >= *threshold
                }
                InternalGoal::ReduceStress => {
                    world_state.stress_level < 0.3 // Stress is considered reduced below 30%
                }
                InternalGoal::SeekEntertainment => {
                    // Entertainment is satisfied when happiness is above 0.6
                    world_state.emotion_levels
                        .get("Happiness")
                        .map(|level| *level >= 0.6)
                        .unwrap_or(false)
                }
            };
        }

        // Check external goals
        match &self.external {
            Some(ExternalGoal::ContributeFoodToStorehouse(target)) => {
                world_state.storehouse_food >= *target
            }
            Some(ExternalGoal::ContributeMaterialsToStorehouse(target)) => {
                world_state.storehouse_materials >= *target
            }
            Some(ExternalGoal::StockHouseFood(target)) => {
                world_state.personal_food >= *target
            }
            Some(ExternalGoal::GatherResource(_resource, target)) => {
                world_state.gathered_resources >= *target
            }
            Some(ExternalGoal::EnsureToolsAvailable(target)) => {
                world_state.storehouse_tools >= *target
            }
            Some(ExternalGoal::OwnHouse) => {
                world_state.owns_house
            }
            Some(ExternalGoal::ObtainProtection) => {
                world_state.has_protection
            }
            Some(ExternalGoal::CraftItem(item)) => {
                world_state.crafted_items
                    .get(item)
                    .map(|count| *count > 0)
                    .unwrap_or(false)
            }
            Some(ExternalGoal::BuildStructure(structure)) => {
                world_state.built_structures
                    .get(structure)
                    .map(|count| *count > 0)
                    .unwrap_or(false)
            }
            Some(ExternalGoal::LearnSkill(skill)) => {
                // Skill is learned if level is > 0
                world_state.learned_skills
                    .get(skill)
                    .map(|level| *level > 0)
                    .unwrap_or(false)
            }
            Some(ExternalGoal::FormRelationship(rel_type)) => {
                world_state.relationships_formed
                    .get(rel_type)
                    .map(|count| *count > 0)
                    .unwrap_or(false)
            }
            Some(ExternalGoal::CompleteJob(job)) => {
                world_state.jobs_completed
                    .get(job)
                    .map(|count| *count > 0)
                    .unwrap_or(false)
            }
            None => false,
        }
    }
}

/// World state information for checking goal satisfaction
///
/// This allows goals to be re-evaluated when new information arrives,
/// enabling agents to abandon plans that are no longer necessary.
#[derive(Debug, Clone, Default)]
pub struct GoalWorldState {
    /// Current food in the communal storehouse
    pub storehouse_food: u32,
    /// Current materials in the communal storehouse
    pub storehouse_materials: u32,
    /// Current tools in the communal storehouse
    pub storehouse_tools: u32,
    /// Food in agent's personal inventory
    pub personal_food: u32,
    /// Resources gathered by agent
    pub gathered_resources: u32,
    /// Whether agent owns a house
    pub owns_house: bool,
    /// Whether agent has protection equipment
    pub has_protection: bool,
    /// Items the agent has crafted (item name -> count)
    pub crafted_items: std::collections::BTreeMap<String, u32>,
    /// Structures the agent has built (structure name -> count)
    pub built_structures: std::collections::BTreeMap<String, u32>,
    /// Skills the agent has learned (skill name -> level)
    pub learned_skills: std::collections::BTreeMap<String, i32>,
    /// Relationships formed (relationship type -> count)
    pub relationships_formed: std::collections::BTreeMap<String, u32>,
    /// Jobs completed (job name -> count)
    pub jobs_completed: std::collections::BTreeMap<String, u32>,
    /// Current emotion levels (emotion type -> value 0.0-1.0)
    pub emotion_levels: std::collections::BTreeMap<String, f32>,
    /// Current overall well-being (0.0-1.0)
    pub well_being: f32,
    /// Current stress level (0.0-1.0)
    pub stress_level: f32,
}

/// Agent's goal manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalManager {
    pub goals: Vec<Goal>,
    pub max_goals: usize,
}

impl GoalManager {
    pub fn new(max_goals: usize) -> Self {
        Self {
            goals: Vec::new(),
            max_goals,
        }
    }

    /// Add a new goal if there's room
    pub fn add_goal(&mut self, goal: Goal) -> bool {
        if self.goals.len() >= self.max_goals {
            return false;
        }
        self.goals.push(goal);
        true
    }

    /// Remove completed goals
    pub fn cleanup_completed(&mut self) {
        self.goals.retain(|g| !g.completed);
    }

    /// Get highest priority active goal
    pub fn highest_priority_goal(&self) -> Option<&Goal> {
        self.goals
            .iter()
            .filter(|g| !g.completed)
            .max_by(|a, b| a.priority.partial_cmp(&b.priority).unwrap())
    }


    /// Update progress for a specific goal by ID
    pub fn update_goal_progress(&mut self, goal_id: uuid::Uuid, amount: f32) {
        if let Some(goal) = self.goals.iter_mut().find(|g| g.id == goal_id) {
            goal.update_progress(amount);
        }
    }

    /// Check if action aligns with any goals
    pub fn action_aligns_with_goals(&self, action: &str) -> bool {
        self.goals
            .iter()
            .filter(|g| !g.completed)
            .any(|g| g.aligns_with_action(action))
    }

    /// Generate common goals based on drives and emotions
    pub fn generate_common_goals(
        drives: &[DriveType],
        emotions: &[(EmotionType, f32)],
        tick: u32,
    ) -> Vec<Goal> {
        let mut goals = Vec::new();

        // Generate external goals based on high drives
        for drive in drives {
            match drive {
                DriveType::Hunger => {
                    goals.push(Goal::new_external(
                        ExternalGoal::StockHouseFood(20),
                        0.8,
                        tick,
                    ));
                }
                DriveType::Shelter => {
                    goals.push(Goal::new_external(
                        ExternalGoal::OwnHouse,
                        0.7,
                        tick,
                    ));
                }
                DriveType::Safety => {
                    goals.push(Goal::new_external(
                        ExternalGoal::ObtainProtection,
                        0.75,
                        tick,
                    ));
                }
                DriveType::Preparedness => {
                    goals.push(Goal::new_external(
                        ExternalGoal::ContributeMaterialsToStorehouse(50),
                        0.5,
                        tick,
                    ));
                }
                DriveType::Utility => {
                    goals.push(Goal::new_external(
                        ExternalGoal::EnsureToolsAvailable(10),
                        0.6,
                        tick,
                    ));
                }
                DriveType::Social => {
                    goals.push(Goal::new_external(
                        ExternalGoal::FormRelationship("friend".to_string()),
                        0.55,
                        tick,
                    ));
                }
                _ => {}
            }
        }

        // Generate internal goals based on emotions
        for (emotion, value) in emotions {
            match emotion {
                EmotionType::Happiness if *value < 0.3 => {
                    goals.push(Goal::new_internal(
                        InternalGoal::IncreaseEmotion(EmotionType::Happiness, 0.6),
                        0.7,
                        tick,
                    ));
                }
                EmotionType::Fear if *value > 0.7 => {
                    goals.push(Goal::new_internal(
                        InternalGoal::DecreaseEmotion(EmotionType::Fear, 0.4),
                        0.8,
                        tick,
                    ));
                }
                EmotionType::Anger if *value > 0.7 => {
                    goals.push(Goal::new_internal(
                        InternalGoal::DecreaseEmotion(EmotionType::Anger, 0.4),
                        0.65,
                        tick,
                    ));
                }
                EmotionType::Sadness if *value > 0.7 => {
                    goals.push(Goal::new_internal(
                        InternalGoal::DecreaseEmotion(EmotionType::Sadness, 0.4),
                        0.7,
                        tick,
                    ));
                }
                _ => {}
            }
        }

        goals
    }
}

impl Default for GoalManager {
    fn default() -> Self {
        Self::new(10) // Default max of 10 goals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_goal_creation() {
        let goal = Goal::new_internal(
            InternalGoal::IncreaseEmotion(EmotionType::Happiness, 0.8),
            0.7,
            0,
        );
        assert_eq!(goal.goal_type, GoalType::Internal);
        assert!(!goal.completed);
    }

    #[test]
    fn test_external_goal_creation() {
        let goal = Goal::new_external(
            ExternalGoal::OwnHouse,
            0.8,
            0,
        );
        assert_eq!(goal.goal_type, GoalType::External);
        assert!(!goal.completed);
    }

    #[test]
    fn test_goal_progress() {
        let mut goal = Goal::new_external(ExternalGoal::OwnHouse, 0.8, 0);

        goal.update_progress(0.5);
        assert_eq!(goal.progress, 0.5);
        assert!(!goal.completed);

        goal.update_progress(0.6);
        assert_eq!(goal.progress, 1.0);
        assert!(goal.completed);
    }

    #[test]
    fn test_goal_manager() {
        let mut manager = GoalManager::new(5);

        let goal1 = Goal::new_external(ExternalGoal::OwnHouse, 0.8, 0);
        let goal2 = Goal::new_internal(
            InternalGoal::IncreaseEmotion(EmotionType::Happiness, 0.7),
            0.6,
            0,
        );

        assert!(manager.add_goal(goal1));
        assert!(manager.add_goal(goal2));
        assert_eq!(manager.goals.len(), 2);
    }

    #[test]
    fn test_highest_priority() {
        let mut manager = GoalManager::new(5);

        manager.add_goal(Goal::new_external(ExternalGoal::OwnHouse, 0.5, 0));
        manager.add_goal(Goal::new_external(ExternalGoal::ObtainProtection, 0.9, 0));
        manager.add_goal(Goal::new_external(ExternalGoal::StockHouseFood(10), 0.3, 0));

        let highest = manager.highest_priority_goal().unwrap();
        assert_eq!(highest.priority, 0.9);
    }

    #[test]
    fn test_action_alignment() {
        let goal = Goal::new_external(
            ExternalGoal::CraftItem("sword".to_string()),
            0.7,
            0,
        );

        assert!(goal.aligns_with_action("crafting sword"));
        assert!(!goal.aligns_with_action("building house"));
    }
}
