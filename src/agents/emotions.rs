// src/agents/emotions.rs
//! Emotion system for agents responding to threats and relationships.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::core::traits::{Trait, TraitSet};

/// Emotional state tracking anger, fear, sadness, happiness, and curiosity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    /// Anger: response to overcomable threats (0.0 to 1.0)
    pub anger: f32,
    /// Fear: response to overwhelming threats (0.0 to 1.0)
    pub fear: f32,
    /// Sadness: response to harm to loved ones (0.0 to 1.0)
    pub sadness: f32,
    /// Happiness: positive emotion from satisfaction and social bonds (0.0 to 1.0)
    pub happiness: f32,
    /// Curiosity: drive to explore and refresh knowledge (0.0 to 1.0)
    pub curiosity: f32,
    /// Decay rate per tick for each emotion
    pub decay_rate: f32,
    /// Emotion sources: what/who triggered each emotion
    pub anger_sources: HashMap<EmotionSource, f32>,
    pub fear_sources: HashMap<EmotionSource, f32>,
    pub sadness_sources: HashMap<EmotionSource, f32>,
    pub happiness_sources: HashMap<EmotionSource, f32>,
    pub curiosity_sources: HashMap<EmotionSource, f32>,
}

impl EmotionState {
    pub fn new() -> Self {
        Self {
            anger: 0.0,
            fear: 0.0,
            sadness: 0.0,
            happiness: 0.0,
            curiosity: 0.0,
            decay_rate: 0.01, // 1% per tick
            anger_sources: HashMap::new(),
            fear_sources: HashMap::new(),
            sadness_sources: HashMap::new(),
            happiness_sources: HashMap::new(),
            curiosity_sources: HashMap::new(),
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

    /// Set sadness level for a source (replaces existing value)
    pub fn set_sadness(&mut self, source: EmotionSource, amount: f32) {
        if amount > 0.0 {
            self.sadness_sources.insert(source, amount.min(1.0));
        } else {
            self.sadness_sources.remove(&source);
        }
        self.update_totals();
    }

    /// Set fear level for a source (replaces existing value)
    pub fn set_fear(&mut self, source: EmotionSource, amount: f32) {
        if amount > 0.0 {
            self.fear_sources.insert(source, amount.min(1.0));
        } else {
            self.fear_sources.remove(&source);
        }
        self.update_totals();
    }

    /// Add happiness from a source
    pub fn add_happiness(&mut self, source: EmotionSource, amount: f32) {
        let new_amount = self.happiness_sources.get(&source).unwrap_or(&0.0) + amount;
        self.happiness_sources.insert(source, new_amount.min(1.0));
        self.update_totals();
    }

    /// Set happiness level for a source (replaces existing value)
    pub fn set_happiness(&mut self, source: EmotionSource, amount: f32) {
        if amount > 0.0 {
            self.happiness_sources.insert(source, amount.min(1.0));
        } else {
            self.happiness_sources.remove(&source);
        }
        self.update_totals();
    }

    /// Add curiosity from a source
    pub fn add_curiosity(&mut self, source: EmotionSource, amount: f32) {
        let new_amount = self.curiosity_sources.get(&source).unwrap_or(&0.0) + amount;
        self.curiosity_sources.insert(source, new_amount.min(1.0));
        self.update_totals();
    }

    /// Set curiosity level for a source (replaces existing value)
    pub fn set_curiosity(&mut self, source: EmotionSource, amount: f32) {
        if amount > 0.0 {
            self.curiosity_sources.insert(source, amount.min(1.0));
        } else {
            self.curiosity_sources.remove(&source);
        }
        self.update_totals();
    }

    /// Update total emotion levels from sources
    fn update_totals(&mut self) {
        self.anger = self.anger_sources.values().sum::<f32>().min(1.0);
        self.fear = self.fear_sources.values().sum::<f32>().min(1.0);
        self.sadness = self.sadness_sources.values().sum::<f32>().min(1.0);
        self.happiness = self.happiness_sources.values().sum::<f32>().min(1.0);
        self.curiosity = self.curiosity_sources.values().sum::<f32>().min(1.0);
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
        for amount in self.happiness_sources.values_mut() {
            *amount = (*amount - self.decay_rate).max(0.0);
        }
        for amount in self.curiosity_sources.values_mut() {
            *amount = (*amount - self.decay_rate * 0.5).max(0.0); // Curiosity decays slower
        }

        // Remove sources at 0
        self.anger_sources.retain(|_, v| *v > 0.0);
        self.fear_sources.retain(|_, v| *v > 0.0);
        self.sadness_sources.retain(|_, v| *v > 0.0);
        self.happiness_sources.retain(|_, v| *v > 0.0);
        self.curiosity_sources.retain(|_, v| *v > 0.0);

        self.update_totals();
    }

    /// Get dominant emotion (including happiness and curiosity)
    pub fn dominant_emotion(&self) -> Option<crate::core::EmotionType> {
        use crate::core::EmotionType;

        let max_value = self.anger.max(self.fear).max(self.sadness).max(self.happiness).max(self.curiosity);

        if max_value < 0.1 {
            return None; // No significant emotion
        }

        if self.happiness >= max_value {
            Some(EmotionType::Happiness)
        } else if self.curiosity >= max_value {
            Some(EmotionType::Curiosity)
        } else if self.anger >= max_value {
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
        let negative_total = self.anger + self.fear + self.sadness;
        negative_total > 1.5 || (negative_total > 0.5 && self.happiness < 0.2)
    }

    /// Calculate overall well-being combining positive and negative emotions
    /// Returns a value from -1.0 (maximum negative) to 1.0 (maximum positive)
    pub fn well_being(&self) -> f32 {
        let negative = (self.anger + self.fear + self.sadness) / 3.0;
        self.happiness - negative
    }

    /// Get emotion value by type (for API compatibility)
    pub fn get(&self, emotion_type: crate::core::EmotionType) -> Option<EmotionValue> {
        use crate::core::EmotionType;

        match emotion_type {
            EmotionType::Happiness => Some(EmotionValue { value: self.happiness }),
            EmotionType::Sadness => Some(EmotionValue { value: -self.sadness }),
            EmotionType::Anger => Some(EmotionValue { value: -self.anger }),
            EmotionType::Fear => Some(EmotionValue { value: -self.fear }),
            EmotionType::Curiosity => Some(EmotionValue { value: self.curiosity }),
        }
    }
}

/// Emotion value wrapper for API compatibility
pub struct EmotionValue {
    pub value: f32,
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

// Note: EmotionType is now unified in core::emotions
// Use crate::core::EmotionType instead of a duplicate enum here

/// Relationship between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// ID of the other agent
    pub other_agent: Uuid,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength of bond (-1.0 to 1.0)
    pub bond_strength: f32,
    /// Time together (in ticks)
    pub time_together: u64,
    /// Last interaction tick (for determining if should greet)
    pub last_interaction_tick: u32,
    /// Total number of interactions
    pub total_interactions: u32,
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
            last_interaction_tick: 0,
            total_interactions: 0,
        }
    }

    /// Create a new neutral relationship (for compatibility with social network system)
    pub fn new_neutral(other_agent: Uuid, current_tick: u32) -> Self {
        Self {
            other_agent,
            relationship_type: RelationshipType::Acquaintance,
            bond_strength: 0.0,
            time_together: 0,
            last_interaction_tick: current_tick,
            total_interactions: 0,
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

    /// Record a positive interaction (for compatibility with social network system)
    /// Delta is converted to bond strength change (typically 0-10 -> 0.0-0.1)
    pub fn positive_interaction(&mut self, delta: i8, current_tick: u32) {
        let bond_change = (delta as f32) * 0.01; // Convert delta to 0.0-1.0 scale
        self.strengthen(bond_change);
        self.last_interaction_tick = current_tick;
        self.total_interactions += 1;
    }

    /// Record a negative interaction (for compatibility with social network system)
    /// Delta is converted to bond strength change (typically 0-10 -> 0.0-0.1)
    pub fn negative_interaction(&mut self, delta: i8, current_tick: u32) {
        let bond_change = (delta as f32) * 0.01; // Convert delta to 0.0-1.0 scale
        self.weaken(bond_change);
        self.last_interaction_tick = current_tick;
        self.total_interactions += 1;
    }

    /// Get relationship level (for compatibility with social network system)
    /// Converts bond_strength to RelationshipLevel enum
    pub fn relationship_level(&self) -> super::relationships::RelationshipLevel {
        use super::relationships::RelationshipLevel;

        if self.bond_strength >= 0.8 {
            RelationshipLevel::Loves(5)  // Very strong positive
        } else if self.bond_strength >= 0.6 {
            RelationshipLevel::Loves(1)  // Strong positive
        } else if self.bond_strength >= 0.4 {
            RelationshipLevel::Likes(5)  // Moderate positive
        } else if self.bond_strength >= 0.2 {
            RelationshipLevel::Likes(1)  // Mild positive
        } else if self.bond_strength >= -0.2 {
            RelationshipLevel::Neutral(0)  // Neutral
        } else if self.bond_strength >= -0.4 {
            RelationshipLevel::Dislikes(1)  // Mild negative
        } else if self.bond_strength >= -0.6 {
            RelationshipLevel::Dislikes(5)  // Moderate negative
        } else if self.bond_strength >= -0.8 {
            RelationshipLevel::Hates(1)  // Strong negative
        } else {
            RelationshipLevel::Hates(5)  // Very strong negative
        }
    }

    /// Get trust level (for compatibility with social network system)
    /// Converts bond_strength to TrustLevel enum
    pub fn trust_level(&self) -> super::relationships::TrustLevel {
        use super::relationships::TrustLevel;

        if self.bond_strength >= 0.8 {
            TrustLevel::TrustsCompletely(3)  // Very high trust
        } else if self.bond_strength >= 0.6 {
            TrustLevel::TrustsCompletely(1)  // High trust
        } else if self.bond_strength >= 0.4 {
            TrustLevel::MostlyTrusts(3)  // Moderate trust
        } else if self.bond_strength >= 0.2 {
            TrustLevel::SlightlyTrusts(1)  // Mild trust
        } else if self.bond_strength >= -0.2 {
            TrustLevel::Neutral  // Neutral trust
        } else if self.bond_strength >= -0.4 {
            TrustLevel::SlightlyDistrusts(1)  // Mild distrust
        } else if self.bond_strength >= -0.6 {
            TrustLevel::MostlyDistrusts(3)  // Moderate distrust
        } else if self.bond_strength >= -0.8 {
            TrustLevel::DistrustCompletely(1)  // High distrust
        } else {
            TrustLevel::DistrustCompletely(3)  // Very high distrust
        }
    }

    /// Update relationship based on trait compatibility
    /// Returns true if relationship changed significantly
    pub fn update_from_trait_interaction(
        &mut self,
        self_traits: &TraitSet,
        other_traits: &TraitSet,
    ) -> bool {
        let mut total_change = 0.0;
        let mut incompatibility_count = 0;
        let mut compatibility_count = 0;

        // Check for incompatibilities
        for self_trait in self_traits.get_traits() {
            for other_trait in other_traits.get_traits() {
                if self_trait.incompatible_with(other_trait) {
                    incompatibility_count += 1;

                    // Different trait pairs have different conflict severity
                    let conflict_severity = match (self_trait, other_trait) {
                        // Major conflicts (religion, honesty)
                        (Trait::Believer, Trait::Atheist) |
                        (Trait::Atheist, Trait::Believer) => 0.03,
                        (Trait::Honest, Trait::Dishonest) |
                        (Trait::Dishonest, Trait::Honest) => 0.025,

                        // Moderate conflicts (personality clashes)
                        (Trait::Aggressive, Trait::Peaceful) |
                        (Trait::Peaceful, Trait::Aggressive) => 0.02,
                        (Trait::Hottempered, Trait::Calm) |
                        (Trait::Calm, Trait::Hottempered) => 0.02,

                        // Minor conflicts (preferences)
                        (Trait::Sociable, Trait::Introverted) |
                        (Trait::Introverted, Trait::Sociable) => 0.01,
                        (Trait::Diligent, Trait::Lazy) |
                        (Trait::Lazy, Trait::Diligent) => 0.015,

                        _ => 0.01,
                    };

                    total_change -= conflict_severity;
                }
            }
        }

        // Check for complementary traits (strengthen relationships)
        if self_traits.has_trait(&Trait::Empathetic) &&
           other_traits.has_trait(&Trait::Empathetic) {
            compatibility_count += 1;
            total_change += 0.02; // Both empathetic
        }

        if self_traits.has_trait(&Trait::Forgiving) {
            // Forgiving trait reduces negative impact of incompatibilities
            total_change *= 0.7;
        }

        if self_traits.has_trait(&Trait::Sociable) &&
           other_traits.has_trait(&Trait::Sociable) {
            compatibility_count += 1;
            total_change += 0.015; // Both sociable
        }

        // Family bonds are more resilient to trait conflicts
        if self.is_family() {
            total_change *= 0.5;
        }

        // Apply the change
        let old_strength = self.bond_strength;
        self.bond_strength = (self.bond_strength + total_change).clamp(-1.0, 1.0);
        self.time_together += 1;

        // Significant change if bond strength changed by more than 0.05
        (self.bond_strength - old_strength).abs() > 0.05
    }

    /// Check if relationship is degrading (negative bond)
    pub fn is_degrading(&self) -> bool {
        self.bond_strength < 0.0
    }

    /// Check if relationship has become hostile
    pub fn is_hostile(&self) -> bool {
        self.bond_strength < -0.4
    }

    /// Get relationship quality descriptor
    pub fn quality_descriptor(&self) -> &'static str {
        match self.bond_strength {
            x if x >= 0.8 => "Excellent",
            x if x >= 0.6 => "Good",
            x if x >= 0.4 => "Friendly",
            x if x >= 0.2 => "Neutral",
            x if x >= 0.0 => "Strained",
            x if x >= -0.3 => "Poor",
            x if x >= -0.6 => "Hostile",
            _ => "Enemy",
        }
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

    /// Update a specific relationship based on trait interaction
    /// Returns true if the relationship changed significantly
    pub fn update_relationship_from_traits(
        &mut self,
        other_agent_id: &Uuid,
        self_traits: &TraitSet,
        other_traits: &TraitSet,
    ) -> bool {
        if let Some(relationship) = self.relationships.get_mut(other_agent_id) {
            relationship.update_from_trait_interaction(self_traits, other_traits)
        } else {
            false
        }
    }

    /// Get all degrading relationships (bond strength < 0)
    pub fn get_degrading_relationships(&self) -> Vec<&Relationship> {
        self.relationships
            .values()
            .filter(|r| r.is_degrading())
            .collect()
    }

    /// Get all hostile relationships (bond strength < -0.4)
    pub fn get_hostile_relationships(&self) -> Vec<&Relationship> {
        self.relationships
            .values()
            .filter(|r| r.is_hostile())
            .collect()
    }

    /// Get relationships by quality
    pub fn get_by_quality(&self, min_bond: f32) -> Vec<&Relationship> {
        self.relationships
            .values()
            .filter(|r| r.bond_strength >= min_bond)
            .collect()
    }

    /// Count incompatible trait conflicts with another agent
    pub fn count_trait_conflicts(
        self_traits: &TraitSet,
        other_traits: &TraitSet,
    ) -> usize {
        let mut count = 0;
        for self_trait in self_traits.get_traits() {
            for other_trait in other_traits.get_traits() {
                if self_trait.incompatible_with(other_trait) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Check if two agents have compatible traits (more compatible than incompatible)
    pub fn are_traits_compatible(
        self_traits: &TraitSet,
        other_traits: &TraitSet,
    ) -> bool {
        let conflicts = Self::count_trait_conflicts(self_traits, other_traits);

        // Check for synergies
        let mut synergies = 0;

        if self_traits.has_trait(&Trait::Empathetic) &&
           other_traits.has_trait(&Trait::Empathetic) {
            synergies += 1;
        }

        if self_traits.has_trait(&Trait::Sociable) &&
           other_traits.has_trait(&Trait::Sociable) {
            synergies += 1;
        }

        if self_traits.has_trait(&Trait::Forgiving) ||
           other_traits.has_trait(&Trait::Forgiving) {
            synergies += 1; // Forgiving helps compatibility
        }

        synergies > conflicts
    }

    /// Get or create a relationship (for compatibility with social network system)
    /// If the relationship doesn't exist, creates a new neutral one
    pub fn get_or_create_relationship(
        &mut self,
        other_agent_id: Uuid,
        current_tick: u32,
    ) -> &mut Relationship {
        self.relationships
            .entry(other_agent_id)
            .or_insert_with(|| Relationship::new_neutral(other_agent_id, current_tick))
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
    pub fn emotion_type(&self) -> crate::core::EmotionType {
        use crate::core::EmotionType;

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
    use crate::core::EmotionType;

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

    #[test]
    fn test_trait_incompatibility_weakens_relationship() {
        let agent1_id = Uuid::new_v4();
        let agent2_id = Uuid::new_v4();

        let mut agent1_traits = TraitSet::new();
        agent1_traits.add_trait(Trait::Believer);

        let mut agent2_traits = TraitSet::new();
        agent2_traits.add_trait(Trait::Atheist);

        let mut rel = Relationship::new(agent2_id, RelationshipType::Friend);
        let initial_strength = rel.bond_strength;

        // Simulate multiple interactions
        for _ in 0..10 {
            rel.update_from_trait_interaction(&agent1_traits, &agent2_traits);
        }

        // Relationship should have degraded
        assert!(rel.bond_strength < initial_strength);
        assert!(rel.bond_strength < 0.2); // Should be significantly weakened
    }

    #[test]
    fn test_compatible_traits_strengthen_relationship() {
        let agent1_id = Uuid::new_v4();
        let agent2_id = Uuid::new_v4();

        let mut agent1_traits = TraitSet::new();
        agent1_traits.add_trait(Trait::Empathetic);
        agent1_traits.add_trait(Trait::Sociable);

        let mut agent2_traits = TraitSet::new();
        agent2_traits.add_trait(Trait::Empathetic);
        agent2_traits.add_trait(Trait::Sociable);

        let mut rel = Relationship::new(agent2_id, RelationshipType::Acquaintance);
        let initial_strength = rel.bond_strength;

        // Simulate multiple interactions
        for _ in 0..10 {
            rel.update_from_trait_interaction(&agent1_traits, &agent2_traits);
        }

        // Relationship should have strengthened
        assert!(rel.bond_strength > initial_strength);
        assert!(rel.bond_strength > 0.4); // Should be significantly strengthened
    }

    #[test]
    fn test_forgiving_trait_reduces_conflict_impact() {
        let agent1_id = Uuid::new_v4();
        let agent2_id = Uuid::new_v4();

        // Agent 1 is forgiving
        let mut agent1_traits = TraitSet::new();
        agent1_traits.add_trait(Trait::Believer);
        agent1_traits.add_trait(Trait::Forgiving);

        // Agent 2 has conflicting trait
        let mut agent2_traits = TraitSet::new();
        agent2_traits.add_trait(Trait::Atheist);

        let mut rel_forgiving = Relationship::new(agent2_id, RelationshipType::Friend);
        let mut rel_normal = Relationship::new(agent2_id, RelationshipType::Friend);

        // Without forgiving trait
        let mut traits_no_forgive = TraitSet::new();
        traits_no_forgive.add_trait(Trait::Believer);

        // Simulate interactions
        for _ in 0..5 {
            rel_forgiving.update_from_trait_interaction(&agent1_traits, &agent2_traits);
            rel_normal.update_from_trait_interaction(&traits_no_forgive, &agent2_traits);
        }

        // Forgiving agent's relationship should degrade less
        assert!(rel_forgiving.bond_strength > rel_normal.bond_strength);
    }

    #[test]
    fn test_family_bonds_more_resilient() {
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let mut parent_traits = TraitSet::new();
        parent_traits.add_trait(Trait::Diligent);

        let mut child_traits = TraitSet::new();
        child_traits.add_trait(Trait::Lazy);

        // Family relationship
        let mut family_rel = Relationship::new(child_id, RelationshipType::Parent);
        let family_initial = family_rel.bond_strength;

        // Friend relationship (non-family)
        let mut friend_rel = Relationship::new(child_id, RelationshipType::Friend);
        let friend_initial = friend_rel.bond_strength;

        // Simulate same conflicts
        for _ in 0..10 {
            family_rel.update_from_trait_interaction(&parent_traits, &child_traits);
            friend_rel.update_from_trait_interaction(&parent_traits, &child_traits);
        }

        // Family bond should degrade less than friend bond
        let family_loss = family_initial - family_rel.bond_strength;
        let friend_loss = friend_initial - friend_rel.bond_strength;

        assert!(family_loss < friend_loss);
    }

    #[test]
    fn test_relationship_quality_descriptor() {
        let other_id = Uuid::new_v4();

        let excellent = Relationship::new(other_id, RelationshipType::Parent);
        assert_eq!(excellent.quality_descriptor(), "Excellent");

        let good = Relationship::new(other_id, RelationshipType::Sibling);
        assert_eq!(good.quality_descriptor(), "Good");

        let enemy = Relationship::new(other_id, RelationshipType::Enemy);
        assert_eq!(enemy.quality_descriptor(), "Enemy"); // -0.7 is below -0.6 threshold
    }

    #[test]
    fn test_relationship_map_trait_update() {
        let mut map = RelationshipMap::new();
        let agent1_id = Uuid::new_v4();
        let agent2_id = Uuid::new_v4();

        map.add_relationship(Relationship::new(agent2_id, RelationshipType::Friend));

        let mut agent1_traits = TraitSet::new();
        agent1_traits.add_trait(Trait::Aggressive);

        let mut agent2_traits = TraitSet::new();
        agent2_traits.add_trait(Trait::Peaceful);

        let initial = map.get_relationship(&agent2_id).unwrap().bond_strength;

        // Update relationship
        map.update_relationship_from_traits(&agent2_id, &agent1_traits, &agent2_traits);

        let updated = map.get_relationship(&agent2_id).unwrap().bond_strength;

        // Should have degraded
        assert!(updated < initial);
    }

    #[test]
    fn test_count_trait_conflicts() {
        let mut traits1 = TraitSet::new();
        traits1.add_trait(Trait::Believer);
        traits1.add_trait(Trait::Aggressive);

        let mut traits2 = TraitSet::new();
        traits2.add_trait(Trait::Atheist);
        traits2.add_trait(Trait::Peaceful);

        let conflicts = RelationshipMap::count_trait_conflicts(&traits1, &traits2);
        assert_eq!(conflicts, 2); // Believer-Atheist and Aggressive-Peaceful
    }

    #[test]
    fn test_are_traits_compatible() {
        // Compatible agents
        let mut compatible1 = TraitSet::new();
        compatible1.add_trait(Trait::Empathetic);
        compatible1.add_trait(Trait::Sociable);

        let mut compatible2 = TraitSet::new();
        compatible2.add_trait(Trait::Empathetic);
        compatible2.add_trait(Trait::Forgiving);

        assert!(RelationshipMap::are_traits_compatible(&compatible1, &compatible2));

        // Incompatible agents
        let mut incompatible1 = TraitSet::new();
        incompatible1.add_trait(Trait::Believer);
        incompatible1.add_trait(Trait::Aggressive);

        let mut incompatible2 = TraitSet::new();
        incompatible2.add_trait(Trait::Atheist);
        incompatible2.add_trait(Trait::Peaceful);

        assert!(!RelationshipMap::are_traits_compatible(&incompatible1, &incompatible2));
    }

    #[test]
    fn test_get_degrading_relationships() {
        let mut map = RelationshipMap::new();

        let friend_id = Uuid::new_v4();
        let enemy_id = Uuid::new_v4();

        let mut friend_rel = Relationship::new(friend_id, RelationshipType::Friend);
        friend_rel.bond_strength = -0.2; // Degraded
        map.add_relationship(friend_rel);

        let enemy_rel = Relationship::new(enemy_id, RelationshipType::Enemy);
        map.add_relationship(enemy_rel);

        let degrading = map.get_degrading_relationships();
        assert_eq!(degrading.len(), 2); // Both have negative bond
    }

    #[test]
    fn test_get_hostile_relationships() {
        let mut map = RelationshipMap::new();

        let rival_id = Uuid::new_v4();
        let enemy_id = Uuid::new_v4();

        let rival_rel = Relationship::new(rival_id, RelationshipType::Rival);
        map.add_relationship(rival_rel); // -0.3, not hostile yet

        let enemy_rel = Relationship::new(enemy_id, RelationshipType::Enemy);
        map.add_relationship(enemy_rel); // -0.7, hostile

        let hostile = map.get_hostile_relationships();
        assert_eq!(hostile.len(), 1); // Only enemy is hostile (< -0.4)
    }

    #[test]
    fn test_relationship_becomes_hostile_from_traits() {
        let other_id = Uuid::new_v4();

        let mut agent1_traits = TraitSet::new();
        agent1_traits.add_trait(Trait::Believer);
        agent1_traits.add_trait(Trait::Honest);
        agent1_traits.add_trait(Trait::Aggressive);

        let mut agent2_traits = TraitSet::new();
        agent2_traits.add_trait(Trait::Atheist);
        agent2_traits.add_trait(Trait::Dishonest);
        agent2_traits.add_trait(Trait::Peaceful);

        let mut rel = Relationship::new(other_id, RelationshipType::Acquaintance);

        // Simulate many conflicted interactions
        for _ in 0..30 {
            rel.update_from_trait_interaction(&agent1_traits, &agent2_traits);
        }

        // Should become hostile due to multiple major conflicts
        assert!(rel.is_hostile());
        assert!(rel.bond_strength < -0.4);
    }
}
