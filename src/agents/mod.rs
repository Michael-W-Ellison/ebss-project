// src/agents/mod.rs
//! Agent implementation and population management.

pub mod agent;
pub mod population;
pub mod senses;
pub mod body;
pub mod skills;

pub use agent::{Agent, AgentConfig, AgentState, Inventory, InventoryItem};
pub use population::Population;
pub use senses::{Senses, Vision, Hearing, Speech, Sound, SoundType, Utterance};
pub use body::{Body, BodyPart, BodyPartType, BodyPartStatus, BodySummary, Condition, ConditionType};
pub use skills::{
    Skills, Skill, SkillType, SkillCategory, Quality, SkillCheckResult, InjuryType,
    RepairResult, RecycledMaterial, RecycleResult,
};
