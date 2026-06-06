// src/core/mod.rs
//! Core AI systems including behavior trees, drives, and learning algorithms.

pub mod behavior_tree;
pub mod drives;
pub mod events;
pub mod learning;
pub mod memory;
pub mod episodic_memory;
pub mod working_memory;
pub mod memory_manager;
pub mod emotions;
pub mod traits;
pub mod goals;
pub mod preferences;
pub mod drive_progression;
pub mod planning;
pub mod spatial;

#[cfg(test)]
mod tests;

pub use behavior_tree::{BehaviorTree, BehaviorNode, NodeType, ExecutionResult, BehaviorContext, DefaultBehaviorContext};
pub use drives::{Drive, DriveType, DriveState};
pub use events::{SimulationEvent, SimulationEventType, DeathCause};
pub use learning::{ObservableEvent, ObservableEventType, LearningResult, LearningExposure, observe_and_learn, process_population_learning};
pub use memory::{Memory, SpatialMemoryType, KnowledgeMemory};
pub use episodic_memory::{EpisodicMemory, Episode, EpisodeType, EpisodicMemoryStats};
pub use working_memory::{WorkingMemory, WorkingTask, TaskPriority, TaskStatus, WorkingMemoryStats};
pub use memory_manager::{MemoryManager, DecisionContext, MemoryManagerStats};
pub use emotions::{Emotion, EmotionType, EmotionalState};
pub use traits::{Trait, TraitSet};
pub use goals::{Goal, GoalType, InternalGoal, ExternalGoal, GoalManager, GoalWorldState};
pub use preferences::{Preferences, Obsession, ObsessionType};
pub use drive_progression::{DriveProgression, DriveTier, DriveTierRequirement, Requirement};
pub use planning::{Planner, ActionPlan, PlanStep, ActionType, ActionOutcome};
pub use spatial::{SpatialGrid, distance_squared, distance_squared_2d, within_interaction_range, within_close_range};
