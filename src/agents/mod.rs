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
    EquipmentItem, EquipmentType, EquipmentMaterial, EquipmentManager,
    MetalMaterial, WoodMaterial, StoneMaterial,
    WeaponTemplate, ToolTemplate, ArmorTemplate,
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
pub mod reproduction;
pub mod shared_knowledge;
pub mod knowledge;
pub mod relationships;
pub mod profession;
pub mod social_interactions;
pub mod exploration;
pub mod exploration_behavior;
pub mod storage_management;
pub mod storage_integration;
pub mod sensory_processing;
pub mod observation_processing;

pub use population::{Population, PopulationConfig, PopulationLearningStats};
pub use reproduction::{can_mate, reproduce, MateSelectionCriteria};
pub use shared_knowledge::{SharedKnowledge, DiscoveredResource};
pub use knowledge::{PersonalKnowledge, ResourceKnowledge, KnowledgeSource};
pub use relationships::{SocialNetwork, RelationshipLevel, TrustLevel};
pub use profession::{Profession, JobType, JobCategory};
pub use social_interactions::{
    SocialInteractionType, ConversationTopic, HelpType, SocialInteractionResult,
    calculate_relationship_change, calculate_social_satisfaction, should_greet,
    select_conversation_topic, would_accept_gift, calculate_gift_value,
};
pub use exploration::{
    ExplorationKnowledge, Discovery, DiscoveryType,
    calculate_exploration_reward, should_explore,
};
pub use exploration_behavior::{
    ExplorationDecision,
    random_exploration_direction,
    calculate_exploration_direction,
};
pub use storage_management::{
    StorageDecision, StoragePreferences,
    decide_storage_action, calculate_storage_priority,
    is_storage_critical, should_prioritize_gathering,
};
pub use storage_integration::{
    take_from_agent_inventory, add_to_agent_inventory,
    count_in_agent_inventory, count_food_in_inventory,
    count_resources_in_inventory, count_tools_in_inventory,
};
pub use sensory_processing::{
    Percept, DetectionMethod, ThreatType, EnvironmentType,
    process_sensory_input, calculate_salience, filter_by_salience,
    most_salient_percept,
};
pub use observation_processing::{
    BroadcastAction, BehaviorContext, NeedType, LearningStats,
    process_observations, auto_adopt_ready_behaviors,
    apply_skill_learning, should_imitate_behavior, get_learning_stats,
};
