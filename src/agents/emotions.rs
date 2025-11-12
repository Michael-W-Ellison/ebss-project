// src/agents/emotions.rs
//! Emotion system for agents responding to threats and relationships.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Emotional state tracking anger, fear, and sadness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    /// Anger: response to overcomable threats (0.0 to 1.0)
    pub anger: f32,
    /// Fear: response to overwhelming threats (0.0 to 1.0)
    pub fear: f32,
    /// Sadness: response to harm to loved ones (0.0 to 1.0)
    pub sadness: f32,
    /// Decay rate per tick for each emotion
    pub decay_rate: f32,
    /// Emotion sources: what/who triggered each emotion
    pub anger_sources: HashMap<EmotionSource, f32>,
    pub fear_sources: HashMap<EmotionSource, f32>,
    pub sadness_sources: HashMap<EmotionSource, f32>,
}

impl EmotionState {
    pub fn new() -> Self {
        Self {
            anger: 0.0,
            fear: 0.0,
            sadness: 0.0,
            decay_rate: 0.01, // 1% per tick
            anger_sources: HashMap::new(),
            fear_sources: HashMap::new(),
            sadness_sources: HashMap::new(),
        }
    }

    /// Add anger toward a source
    pub fn add_anger(&mut self, source: EmotionSource, amount: f32) {
        let new_amount = self.anger_sources.get(&source).unwrap_or(&0.0) + amount;
        self.anger_sources.insert(source, new_amount.min(1.0));
        self.update_totals();
    }

    /// Add fear toward a source
    pub fn add_fear(&mut self, source: EmotionSource, amount: f32) {
        let new_amount = self.fear_sources.get(&source).unwrap_or(&0.0) + amount;
        self.fear_sources.insert(source, new_amount.min(1.0));
        self.update_totals();
    }

    /// Add sadness toward a source
    pub fn add_sadness(&mut self, source: EmotionSource, amount: f32) {
        let new_amount = self.sadness_sources.get(&source).unwrap_or(&0.0) + amount;
        self.sadness_sources.insert(source, new_amount.min(1.0));
        self.update_totals();
    }

    /// Update total emotion levels from sources
    fn update_totals(&mut self) {
        self.anger = self.anger_sources.values().sum::<f32>().min(1.0);
        self.fear = self.fear_sources.values().sum::<f32>().min(1.0);
        self.sadness = self.sadness_sources.values().sum::<f32>().min(1.0);
    }

    /// Decay emotions over time
    pub fn tick(&mut self) {
        // Decay each source
        for amount in self.anger_sources.values_mut() {
            *amount = (*amount - self.decay_rate).max(0.0);
        }
        for amount in self.fear_sources.values_mut() {
            *amount = (*amount - self.decay_rate).max(0.0);
        }
        for amount in self.sadness_sources.values_mut() {
            *amount = (*amount - self.decay_rate).max(0.0);
        }

        // Remove sources at 0
        self.anger_sources.retain(|_, v| *v > 0.0);
        self.fear_sources.retain(|_, v| *v > 0.0);
        self.sadness_sources.retain(|_, v| *v > 0.0);

        self.update_totals();
    }

    /// Get dominant emotion
    pub fn dominant_emotion(&self) -> Option<EmotionType> {
        let max_value = self.anger.max(self.fear).max(self.sadness);

        if max_value < 0.1 {
            return None; // No significant emotion
        }

        if self.anger >= max_value {
            Some(EmotionType::Anger)
        } else if self.fear >= max_value {
            Some(EmotionType::Fear)
        } else {
            Some(EmotionType::Sadness)
        }
    }

    /// Check if agent should flee (high fear)
    pub fn should_flee(&self) -> bool {
        self.fear > 0.6
    }

    /// Check if agent should attack (high anger, low fear)
    pub fn should_attack(&self) -> bool {
        self.anger > 0.5 && self.fear < 0.3
    }

    /// Check if agent is emotionally distressed
    pub fn is_distressed(&self) -> bool {
        self.anger + self.fear + self.sadness > 1.5
    }
}

impl Default for EmotionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Source of an emotion
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionSource {
    /// Another agent (by UUID)
    Agent(Uuid),
    /// A creature/animal type
    Creature(String),
    /// An environmental event
    Event(String),
    /// A location
    Location((i32, i32, i32)),
}

/// Type of emotion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionType {
    Anger,
    Fear,
    Sadness,
}

/// Relationship between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// ID of the other agent
    pub other_agent: Uuid,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength of bond (0.0 to 1.0)
    pub bond_strength: f32,
    /// Time together (in ticks)
    pub time_together: u64,
}

impl Relationship {
    pub fn new(other_agent: Uuid, relationship_type: RelationshipType) -> Self {
        let bond_strength = match relationship_type {
            RelationshipType::Parent | RelationshipType::Child => 0.9,
            RelationshipType::Sibling => 0.7,
            RelationshipType::Partner => 0.8,
            RelationshipType::Friend => 0.5,
            RelationshipType::Acquaintance => 0.2,
            RelationshipType::Rival => -0.3,
            RelationshipType::Enemy => -0.7,
        };

        Self {
            other_agent,
            relationship_type,
            bond_strength,
            time_together: 0,
        }
    }

    /// Check if this is a loved one
    pub fn is_loved_one(&self) -> bool {
        self.bond_strength >= 0.6
    }

    /// Check if this is family
    pub fn is_family(&self) -> bool {
        matches!(
            self.relationship_type,
            RelationshipType::Parent | RelationshipType::Child | RelationshipType::Sibling
        )
    }

    /// Strengthen bond
    pub fn strengthen(&mut self, amount: f32) {
        self.bond_strength = (self.bond_strength + amount).min(1.0);
    }

    /// Weaken bond
    pub fn weaken(&mut self, amount: f32) {
        self.bond_strength = (self.bond_strength - amount).max(-1.0);
    }
}

/// Type of relationship between agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Parent,
    Child,
    Sibling,
    Partner,
    Friend,
    Acquaintance,
    Rival,
    Enemy,
}

/// Tracks all relationships for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMap {
    relationships: HashMap<Uuid, Relationship>,
}

impl RelationshipMap {
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
        }
    }

    /// Add or update a relationship
    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.relationships.insert(relationship.other_agent, relationship);
    }

    /// Get relationship with another agent
    pub fn get_relationship(&self, agent_id: &Uuid) -> Option<&Relationship> {
        self.relationships.get(agent_id)
    }

    /// Get mutable relationship with another agent
    pub fn get_relationship_mut(&mut self, agent_id: &Uuid) -> Option<&mut Relationship> {
        self.relationships.get_mut(agent_id)
    }

    /// Get all loved ones
    pub fn get_loved_ones(&self) -> Vec<&Relationship> {
        self.relationships
            .values()
            .filter(|r| r.is_loved_one())
            .collect()
    }

    /// Get all family members
    pub fn get_family(&self) -> Vec<&Relationship> {
        self.relationships
            .values()
            .filter(|r| r.is_family())
            .collect()
    }

    /// Remove a relationship
    pub fn remove_relationship(&mut self, agent_id: &Uuid) {
        self.relationships.remove(agent_id);
    }

    /// Get all relationships
    pub fn get_all(&self) -> &HashMap<Uuid, Relationship> {
        &self.relationships
    }
}

impl Default for RelationshipMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Threat assessment for emotional responses
#[derive(Debug, Clone)]
pub struct ThreatAssessment {
    /// Threat level (0.0 to 1.0)
    pub threat_level: f32,
    /// Can agent overcome this threat?
    pub can_overcome: bool,
    /// Source of threat
    pub source: EmotionSource,
}

impl ThreatAssessment {
    /// Create threat assessment based on agent vs threat strength
    pub fn assess(agent_strength: f32, threat_strength: f32, source: EmotionSource) -> Self {
        let threat_level = (threat_strength / agent_strength.max(0.1)).min(1.0);
        let can_overcome = agent_strength >= threat_strength * 0.8;

        Self {
            threat_level,
            can_overcome,
            source,
        }
    }

    /// Get appropriate emotion for this threat
    pub fn emotion_type(&self) -> EmotionType {
        if self.can_overcome {
            EmotionType::Anger
        } else {
            EmotionType::Fear
        }
    }

    /// Get emotion amount (0.0 to 1.0)
    pub fn emotion_amount(&self) -> f32 {
        if self.can_overcome {
            // Anger scales with threat level
            self.threat_level * 0.5
        } else {
            // Fear scales with overwhelming odds
            self.threat_level * 0.7
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_state_creation() {
        let emotions = EmotionState::new();
        assert_eq!(emotions.anger, 0.0);
        assert_eq!(emotions.fear, 0.0);
        assert_eq!(emotions.sadness, 0.0);
    }

    #[test]
    fn test_add_anger() {
        let mut emotions = EmotionState::new();
        let source = EmotionSource::Creature("rabbit".to_string());

        emotions.add_anger(source.clone(), 0.3);
        assert_eq!(emotions.anger, 0.3);

        emotions.add_anger(source, 0.2);
        assert_eq!(emotions.anger, 0.5);
    }

    #[test]
    fn test_emotion_decay() {
        let mut emotions = EmotionState::new();
        emotions.add_anger(EmotionSource::Creature("rabbit".to_string()), 0.5);

        assert_eq!(emotions.anger, 0.5);

        emotions.tick();
        assert_eq!(emotions.anger, 0.49); // Decayed by 0.01
    }

    #[test]
    fn test_dominant_emotion() {
        let mut emotions = EmotionState::new();

        emotions.add_anger(EmotionSource::Creature("rabbit".to_string()), 0.3);
        emotions.add_fear(EmotionSource::Creature("bear".to_string()), 0.7);

        assert_eq!(emotions.dominant_emotion(), Some(EmotionType::Fear));
    }

    #[test]
    fn test_should_flee() {
        let mut emotions = EmotionState::new();
        emotions.add_fear(EmotionSource::Creature("bear".to_string()), 0.8);

        assert!(emotions.should_flee());
    }

    #[test]
    fn test_should_attack() {
        let mut emotions = EmotionState::new();
        emotions.add_anger(EmotionSource::Creature("rabbit".to_string()), 0.6);

        assert!(emotions.should_attack());
    }

    #[test]
    fn test_relationship_creation() {
        let other_agent = Uuid::new_v4();
        let rel = Relationship::new(other_agent, RelationshipType::Parent);

        assert_eq!(rel.bond_strength, 0.9);
        assert!(rel.is_loved_one());
        assert!(rel.is_family());
    }

    #[test]
    fn test_relationship_map() {
        let mut map = RelationshipMap::new();
        let parent_id = Uuid::new_v4();
        let friend_id = Uuid::new_v4();

        map.add_relationship(Relationship::new(parent_id, RelationshipType::Parent));
        map.add_relationship(Relationship::new(friend_id, RelationshipType::Friend));

        let family = map.get_family();
        assert_eq!(family.len(), 1);

        let loved_ones = map.get_loved_ones();
        assert_eq!(loved_ones.len(), 1); // Only parent (0.9 bond) is loved one
    }

    #[test]
    fn test_threat_assessment_overcomable() {
        let assessment = ThreatAssessment::assess(
            10.0,
            5.0,
            EmotionSource::Creature("rabbit".to_string()),
        );

        assert!(assessment.can_overcome);
        assert_eq!(assessment.emotion_type(), EmotionType::Anger);
    }

    #[test]
    fn test_threat_assessment_overwhelming() {
        let assessment = ThreatAssessment::assess(
            5.0,
            15.0,
            EmotionSource::Creature("bear".to_string()),
        );

        assert!(!assessment.can_overcome);
        assert_eq!(assessment.emotion_type(), EmotionType::Fear);
    }

    #[test]
    fn test_strengthen_relationship() {
        let mut rel = Relationship::new(Uuid::new_v4(), RelationshipType::Friend);
        assert_eq!(rel.bond_strength, 0.5);

        rel.strengthen(0.2);
        assert_eq!(rel.bond_strength, 0.7);
        assert!(rel.is_loved_one());
    }

    #[test]
    fn test_emotion_sources_cleanup() {
        let mut emotions = EmotionState::new();
        emotions.decay_rate = 0.5; // High decay for faster testing

        emotions.add_anger(EmotionSource::Creature("rabbit".to_string()), 0.4);

        emotions.tick();
        assert!(emotions.anger_sources.is_empty()); // Should be removed at 0
    }
}
