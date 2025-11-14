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
pub mod equipment;
pub mod temperature;
pub mod observational_learning;
pub mod transport;

pub use agent::{Agent, AgentConfig, AgentState, Inventory, InventoryItem, LifeStage};
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
pub use equipment::{
    Equipment, EquipmentSlot, ClothingMaterial, ClothingTemplate,
};
pub use temperature::{
    BodyTemperature, Temperature, Climate,
};
pub use observational_learning::{
    ObservationalLearning, ObservedAction, ActionType, LearningProgress,
};
pub use transport::{
    Transport, TransportType, TransportSystem,
};

#[cfg(test)]
mod tests;
// Temporarily commented out due to API changes
// pub mod reproduction;
pub mod shared_knowledge;
pub mod knowledge;
pub mod relationships;
pub mod profession;

pub use population::{Population, PopulationConfig};
// pub use reproduction::{can_mate, reproduce, MateSelectionCriteria};
pub use shared_knowledge::{SharedKnowledge, DiscoveredResource};
pub use knowledge::{PersonalKnowledge, ResourceKnowledge, KnowledgeSource};
pub use relationships::{SocialNetwork, RelationshipLevel, TrustLevel};
pub use profession::{Profession, JobType, JobCategory};
