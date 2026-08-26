// src/agents/mod.rs
//! Agent implementation and population management.

pub mod agent;
pub mod population;
pub mod senses;
pub mod body;
pub mod skills;
pub mod emotions;
pub mod gossip;
pub mod equipment;
pub mod temperature;
pub mod observational_learning;
pub mod transport;
pub mod drive_satisfaction;
pub mod gender;
pub mod pregnancy;
pub mod childcare;
pub mod fatigue;
pub mod practices;
pub mod patterns;

pub use agent::{Agent, AgentConfig, AgentState, Ailment, Inventory, InventoryItem, LifeStage};
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
    EmotionState, EmotionSource, Relationship, RelationshipType,
    RelationshipMap, ThreatAssessment,
};
pub use crate::core::traits::{Trait, TraitSet};
pub use crate::core::EmotionType; // EmotionType now unified in core
pub use gossip::{
    Information, InformationType, InformationDistortion, DistortionType,
    Belief, TrustRating, KnowledgeBase, OpinionTransferResult,
    calculate_opinion_transfer_chance, attempt_opinion_transfer,
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
pub use drive_satisfaction::{
    SatisfactionTracker, DriveSatisfactionTracker, SatisfactionRecord,
};
pub use gender::Gender;
pub use pregnancy::PregnancyState;
pub use childcare::{DevelopmentalNutrition, NursingState, StatModifiers};
pub use fatigue::{FatigueState, FatigueSeverity, SleepQualityFactors};

#[cfg(test)]
mod tests;
pub mod reproduction;
pub mod shared_knowledge;
pub mod knowledge;
pub mod relationships;
pub mod social_interactions;
pub mod exploration;
pub mod exploration_behavior;
pub mod storage_management;
pub mod storage_integration;
pub mod sensory_processing;
pub mod observation_processing;
pub mod job_happiness;
pub mod religious_effects;

pub use population::{Population, PopulationConfig, PopulationLearningStats};
pub use reproduction::{can_mate, reproduce, attempt_impregnation, give_birth, MateSelectionCriteria};
pub use shared_knowledge::{SharedKnowledge, DiscoveredResource};
pub use knowledge::{PersonalKnowledge, ResourceKnowledge, KnowledgeSource};
pub use relationships::{SocialNetwork, RelationshipLevel, TrustLevel};
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
pub use job_happiness::{
    JobCategory, calculate_job_happiness, find_preferred_job,
    rank_jobs_by_happiness, calculate_effective_priority,
    should_override_happiness, trait_job_happiness,
};
pub use religious_effects::{
    ReligiousEffect, RELIGIOUS_EFFECT_RADIUS,
    calculate_religious_effects, total_happiness_modifier,
    should_seek_religious_building, should_avoid_religious_building,
    secular_knowledge_bonus,
};
