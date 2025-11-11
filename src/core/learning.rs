// src/core/learning.rs
//! Learning system for behavior tree evolution.
//!
//! # Learning Loop Architecture
//!
//! The learning loop in EBSS follows this flow:
//!
//! ```text
//! 1. UPDATE DRIVES
//!    - agent.tick() → agent.drives.tick()
//!    - Each drive accumulates based on its base rate
//!    - Drives approach their threshold values
//!
//! 2. SELECT MOST URGENT DRIVE
//!    - agent.drives.most_urgent()
//!    - Urgency = value * weight (personality variation)
//!    - Returns the drive that needs satisfaction most
//!
//! 3. SELECT BEHAVIOR TREE
//!    - agent.select_behavior_tree()
//!    - Matches drive type to appropriate behavior tree
//!    - Each agent has 13 behavior trees (one per drive)
//!
//! 4. EXECUTE BEHAVIOR TREE
//!    - tree.execute()
//!    - Traverses nodes based on weights and success rates
//!    - LEARNING HAPPENS HERE AUTOMATICALLY:
//!      * Success: weight *= 1.1 (exponential growth)
//!      * Failure: weight *= 0.9 (exponential decay)
//!    - Returns ExecutionResult (Success/Failure/Running)
//!
//! 5. CONVERT TO ACTION
//!    - Map behavior tree result to environment action
//!    - Actions: Eat, Sleep, Gather, Build, Explore, etc.
//!
//! 6. EXECUTE ACTION
//!    - simulation.execute_action(&action)
//!    - Interact with environment/world state
//!    - Returns ActionResult with success and satisfaction amount
//!
//! 7. APPLY FEEDBACK
//!    - agent.apply_feedback(&result)
//!    - If successful: drive.partial_satisfy(amount)
//!    - Reduces drive value based on satisfaction
//!
//! 8. REPEAT
//!    - Loop continues, drives accumulate again
//!    - Successful strategies become more likely
//!    - Failed strategies become less likely
//! ```
//!
//! # Key Learning Mechanisms
//!
//! ## 1. Weight-Based Reinforcement
//! - Every behavior tree node tracks execution_count and success_count
//! - Success rate = success_count / execution_count
//! - Weights adjust automatically on each execution
//! - Range: 0.1 to 10.0 (clamped)
//!
//! ## 2. Probabilistic Selection
//! - Nodes with higher weights more likely to execute
//! - Allows exploration of alternative strategies
//! - Balance between exploitation and exploration
//!
//! ## 3. Genetic Inheritance
//! - tree.clone_with_pruning(min_weight)
//! - Removes branches below weight threshold
//! - Only successful strategies inherited
//! - Offspring start with parent's learned weights
//!
//! ## 4. Drive-Action-Satisfaction Loop
//! - Drives accumulate → Actions execute → Drives satisfied
//! - Closed feedback loop
//! - Natural selection favors effective behaviors
//!
//! # Example Learning Scenario
//!
//! ```text
//! Agent starts hungry (Hunger drive = 0.8)
//! → Selects Hunger behavior tree
//! → Tree has 3 options:
//!    * eat_stored_food (weight: 1.0)
//!    * gather_food (weight: 1.0)
//!    * hunt (weight: 1.0)
//!
//! Tick 1: Tries eat_stored_food → Fails (no storage)
//!    * weight becomes 0.9
//!
//! Tick 5: Tries gather_food → Success!
//!    * weight becomes 1.1
//!    * Hunger reduced by 0.3
//!
//! Tick 10: Tries gather_food again → Success!
//!    * weight becomes 1.21
//!
//! After 100 ticks:
//!    * eat_stored_food: weight = 0.3 (rarely used)
//!    * gather_food: weight = 4.5 (preferred strategy)
//!    * hunt: weight = 1.8 (moderate success)
//!
//! Agent has learned: gathering food is most effective!
//! ```

use crate::core::behavior_tree::BehaviorTree;

/// Learning system that manages behavior tree evolution
pub struct LearningSystem {
    /// Minimum weight threshold for genetic pruning
    pub pruning_threshold: f32,
    /// Learning rate multiplier (affects weight adjustment speed)
    pub learning_rate: f32,
}

impl LearningSystem {
    pub fn new() -> Self {
        Self {
            pruning_threshold: 0.5,
            learning_rate: 1.0,
        }
    }

    /// Create offspring behavior tree with learned weights
    pub fn create_offspring(&self, parent_tree: &BehaviorTree) -> BehaviorTree {
        parent_tree.clone_with_pruning(self.pruning_threshold)
    }

    /// Adjust learning parameters
    pub fn set_learning_rate(&mut self, rate: f32) {
        self.learning_rate = rate.clamp(0.1, 2.0);
    }

    /// Set genetic pruning threshold
    pub fn set_pruning_threshold(&mut self, threshold: f32) {
        self.pruning_threshold = threshold.clamp(0.1, 2.0);
    }
}

impl Default for LearningSystem {
    fn default() -> Self {
        Self::new()
    }
}
