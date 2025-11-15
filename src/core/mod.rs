// src/core/mod.rs
//! Core AI systems including behavior trees, drives, and learning algorithms.

pub mod behavior_tree;
pub mod drives;
pub mod learning;
pub mod memory;
pub mod emotions;
pub mod traits;
pub mod goals;
pub mod preferences;
pub mod drive_progression;
pub mod planning;

pub use behavior_tree::{BehaviorTree, BehaviorNode, NodeType, ExecutionResult};
pub use drives::{Drive, DriveType, DriveState};
pub use learning::{ObservableEvent, ObservableEventType, LearningResult, observe_and_learn, process_population_learning};
pub use memory::{Memory, SpatialMemoryType, SocialRelationship, RelationshipStrength, KnowledgeMemory};
pub use emotions::{Emotion, EmotionType, EmotionalState};
pub use traits::{Trait, TraitSet};
pub use goals::{Goal, GoalType, InternalGoal, ExternalGoal, GoalManager};
pub use preferences::{Preferences, Obsession, ObsessionType};
pub use drive_progression::{DriveProgression, DriveTier, DriveTierRequirement, Requirement};
pub use planning::{Planner, ActionPlan, PlanStep, ActionType, ActionOutcome};
