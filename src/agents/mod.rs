// src/agents/mod.rs
//! Agent implementation and population management.

pub mod agent;
pub mod population;
pub mod senses;
pub mod body;
pub mod skills;
pub mod emotions;
pub mod traits;
pub mod gossip;

pub use agent::{Agent, AgentConfig, AgentState, Inventory, InventoryItem};
pub use population::Population;
pub use senses::{Senses, Vision, Hearing, Speech, Sound, SoundType, Utterance};
pub use body::{
    Body, BodyPart, BodyPartType, BodyPartStatus, BodySummary, Condition, ConditionType,
    InjuryType, CripplingType, Injury,
};
pub use skills::{
    Skills, Skill, SkillType, SkillCategory, Quality, SkillCheckResult,
    RepairResult, RecycledMaterial, RecycleResult,
};
pub use emotions::{
    EmotionState, EmotionSource, EmotionType, Relationship, RelationshipType,
    RelationshipMap, ThreatAssessment,
};
pub use traits::{Trait, TraitSet};
pub use gossip::{
    Information, InformationType, InformationDistortion, DistortionType,
    Belief, TrustRating, KnowledgeBase,
};
