// src/agents/relationships.rs
//! Social relationship and trust system for agents.
//!
//! Tracks relationship status (how much they like each other) and trust
//! (how much they believe information from each other) separately.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Relationship status levels - how much an agent likes another
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipLevel {
    // Hates: -30 to -16
    Hates(i8),        // Levels -5 to -1 (mapped to -30 to -16)
    // Dislikes: -15 to -3
    Dislikes(i8),     // Levels -5 to -1 (mapped to -15 to -3)
    // Neutral: -2 to +2
    Neutral(i8),      // Levels -2, -1, 0, +1, +2
    // Likes: +3 to +15
    Likes(i8),        // Levels +1 to +5 (mapped to +3 to +15)
    // Loves: +16 to +30
    Loves(i8),        // Levels +1 to +5 (mapped to +16 to +30)
}

impl RelationshipLevel {
    /// Create default neutral relationship (0)
    pub fn neutral() -> Self {
        RelationshipLevel::Neutral(0)
    }

    /// Create parent-child relationship (starts at Likes +3)
    pub fn parent_child() -> Self {
        RelationshipLevel::Likes(3)
    }

    /// Get numeric value for calculations
    pub fn value(&self) -> i8 {
        match self {
            RelationshipLevel::Hates(level) => -30 + (5 + level) * 3,     // -30 to -16
            RelationshipLevel::Dislikes(level) => -15 + (5 + level) * 3, // -15 to -3
            RelationshipLevel::Neutral(level) => *level,                  // -2 to +2
            RelationshipLevel::Likes(level) => 3 + (level - 1) * 3,     // +3 to +15
            RelationshipLevel::Loves(level) => 16 + (level - 1) * 3,    // +16 to +30
        }
    }

    /// Create from numeric value
    pub fn from_value(value: i8) -> Self {
        match value {
            v if v <= -16 => {
                let level = ((v + 30) / 3 - 5).max(-5).min(-1);
                RelationshipLevel::Hates(level)
            }
            v if v <= -3 => {
                let level = ((v + 15) / 3 - 5).max(-5).min(-1);
                RelationshipLevel::Dislikes(level)
            }
            v if v <= 2 => RelationshipLevel::Neutral(v.max(-2).min(2)),
            v if v <= 15 => {
                let level = ((v - 3) / 3 + 1).max(1).min(5);
                RelationshipLevel::Likes(level)
            }
            _ => {
                let level = ((value - 16) / 3 + 1).max(1).min(5);
                RelationshipLevel::Loves(level)
            }
        }
    }

    /// Adjust relationship by delta, transitioning between levels
    pub fn adjust(&mut self, delta: i8) {
        let new_value = (self.value() + delta).max(-30).min(30);
        *self = RelationshipLevel::from_value(new_value);
    }

    /// Decay relationship towards neutral over time
    pub fn decay_towards_neutral(&mut self, decay_rate: f32) {
        let current = self.value();
        if current == 0 {
            return; // Already neutral
        }

        let decay = if current > 0 {
            -(decay_rate.max(0.1) as i8).max(-current)
        } else {
            (decay_rate.max(0.1) as i8).min(-current)
        };

        self.adjust(decay);
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            RelationshipLevel::Hates(_) => "Hates",
            RelationshipLevel::Dislikes(_) => "Dislikes",
            RelationshipLevel::Neutral(_) => "Neutral",
            RelationshipLevel::Likes(_) => "Likes",
            RelationshipLevel::Loves(_) => "Loves",
        }
    }
}

/// Trust level - how much an agent believes information from another
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    // Distrust: -12 to -1
    DistrustCompletely(i8),  // -3 to -1 (mapped to -12 to -10)
    MostlyDistrusts(i8),     // -3 to -1 (mapped to -9 to -7)
    GenerallyDistrusts(i8),  // -3 to -1 (mapped to -6 to -4)
    SlightlyDistrusts(i8),   // -3 to -1 (mapped to -3 to -1)
    // Neutral: 0
    Neutral,
    // Trust: +1 to +12
    SlightlyTrusts(i8),      // +1 to +3 (mapped to +1 to +3)
    GenerallyTrusts(i8),     // +1 to +3 (mapped to +4 to +6)
    MostlyTrusts(i8),        // +1 to +3 (mapped to +7 to +9)
    TrustsCompletely(i8),    // +1 to +3 (mapped to +10 to +12)
}

impl TrustLevel {
    pub fn neutral() -> Self {
        TrustLevel::Neutral
    }

    /// Get numeric value for calculations
    pub fn value(&self) -> i8 {
        match self {
            TrustLevel::DistrustCompletely(level) => -12 + (3 + level),
            TrustLevel::MostlyDistrusts(level) => -9 + (3 + level),
            TrustLevel::GenerallyDistrusts(level) => -6 + (3 + level),
            TrustLevel::SlightlyDistrusts(level) => -3 + (3 + level),
            TrustLevel::Neutral => 0,
            TrustLevel::SlightlyTrusts(level) => *level,
            TrustLevel::GenerallyTrusts(level) => 3 + *level,
            TrustLevel::MostlyTrusts(level) => 6 + *level,
            TrustLevel::TrustsCompletely(level) => 9 + *level,
        }
    }

    /// Create from numeric value
    pub fn from_value(value: i8) -> Self {
        match value {
            v if v <= -10 => TrustLevel::DistrustCompletely(((v + 12).max(-3).min(-1))),
            v if v <= -7 => TrustLevel::MostlyDistrusts((v + 9).max(-3).min(-1)),
            v if v <= -4 => TrustLevel::GenerallyDistrusts((v + 6).max(-3).min(-1)),
            v if v <= -1 => TrustLevel::SlightlyDistrusts((v + 3).max(-3).min(-1)),
            0 => TrustLevel::Neutral,
            v if v <= 3 => TrustLevel::SlightlyTrusts(v.max(1).min(3)),
            v if v <= 6 => TrustLevel::GenerallyTrusts((v - 3).max(1).min(3)),
            v if v <= 9 => TrustLevel::MostlyTrusts((v - 6).max(1).min(3)),
            _ => TrustLevel::TrustsCompletely((value - 9).max(1).min(3)),
        }
    }

    /// Adjust trust by delta
    pub fn adjust(&mut self, delta: i8) {
        let new_value = (self.value() + delta).max(-12).min(12);
        *self = TrustLevel::from_value(new_value);
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            TrustLevel::DistrustCompletely(_) => "Distrusts Completely",
            TrustLevel::MostlyDistrusts(_) => "Mostly Distrusts",
            TrustLevel::GenerallyDistrusts(_) => "Generally Distrusts",
            TrustLevel::SlightlyDistrusts(_) => "Slightly Distrusts",
            TrustLevel::Neutral => "Neutral Trust",
            TrustLevel::SlightlyTrusts(_) => "Slightly Trusts",
            TrustLevel::GenerallyTrusts(_) => "Generally Trusts",
            TrustLevel::MostlyTrusts(_) => "Mostly Trusts",
            TrustLevel::TrustsCompletely(_) => "Trusts Completely",
        }
    }

    /// Get belief weight (0.0 to 1.0) for information from this source
    pub fn belief_weight(&self) -> f32 {
        // Map -12 to +12 onto 0.0 to 1.0 scale
        // Negative trust = lower belief, positive trust = higher belief
        (self.value() as f32 + 12.0) / 24.0
    }
}

/// Complete relationship state between two agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub other_agent_id: Uuid,
    pub relationship_level: RelationshipLevel,
    pub trust_level: TrustLevel,
    pub last_interaction_tick: u32,
    pub total_interactions: u32,
}

impl Relationship {
    pub fn new(other_agent_id: Uuid, current_tick: u32) -> Self {
        Self {
            other_agent_id,
            relationship_level: RelationshipLevel::neutral(),
            trust_level: TrustLevel::neutral(),
            last_interaction_tick: current_tick,
            total_interactions: 0,
        }
    }

    pub fn parent_child(other_agent_id: Uuid, current_tick: u32) -> Self {
        Self {
            other_agent_id,
            relationship_level: RelationshipLevel::parent_child(),
            trust_level: TrustLevel::neutral(),
            last_interaction_tick: current_tick,
            total_interactions: 0,
        }
    }

    /// Record a positive interaction
    pub fn positive_interaction(&mut self, delta: i8, current_tick: u32) {
        self.relationship_level.adjust(delta);
        self.last_interaction_tick = current_tick;
        self.total_interactions += 1;
    }

    /// Record a negative interaction
    pub fn negative_interaction(&mut self, delta: i8, current_tick: u32) {
        self.relationship_level.adjust(-delta);
        self.last_interaction_tick = current_tick;
        self.total_interactions += 1;
    }

    /// Information was verified as correct - increase trust
    pub fn verify_information(&mut self, info_age_ticks: u32, current_tick: u32) {
        // Recent info = more trust gain
        let trust_gain = if info_age_ticks < 100 {
            3 // "Just saw it" - big trust gain
        } else if info_age_ticks < 500 {
            2 // Recent - moderate trust gain
        } else {
            1 // Old info - small trust gain
        };

        self.trust_level.adjust(trust_gain);
        self.last_interaction_tick = current_tick;
        self.total_interactions += 1;
    }

    /// Information was proven wrong - decrease trust
    pub fn incorrect_information(&mut self, info_age_ticks: u32, current_tick: u32) {
        // Calculate base trust penalty
        let base_penalty = if info_age_ticks < 100 {
            5 // "Just saw it" but was wrong - big penalty
        } else if info_age_ticks < 500 {
            3 // Recent but wrong - moderate penalty
        } else {
            1 // Old info wrong - small penalty (expected)
        };

        // Reduce penalty based on relationship level
        // Friends get forgiveness
        let relationship_value = self.relationship_level.value();
        let forgiveness_factor = if relationship_value > 15 {
            0.2 // Loves: 80% forgiveness
        } else if relationship_value > 5 {
            0.4 // Strong Likes: 60% forgiveness
        } else if relationship_value > 0 {
            0.6 // Likes: 40% forgiveness
        } else if relationship_value >= -2 {
            1.0 // Neutral: No forgiveness
        } else {
            1.2 // Dislikes/Hates: Extra penalty
        };

        let adjusted_penalty = (base_penalty as f32 * forgiveness_factor) as i8;
        self.trust_level.adjust(-adjusted_penalty.max(1));
        self.last_interaction_tick = current_tick;
        self.total_interactions += 1;
    }

    /// Decay relationship towards neutral if no recent interaction
    pub fn decay_if_no_interaction(&mut self, current_tick: u32, decay_rate: f32) {
        let ticks_since_interaction = current_tick.saturating_sub(self.last_interaction_tick);

        // Start decaying after 500 ticks of no interaction
        if ticks_since_interaction > 500 {
            // Faster decay for longer periods
            let decay_multiplier = (ticks_since_interaction / 500) as f32;
            self.relationship_level.decay_towards_neutral(decay_rate * decay_multiplier);
        }
    }
}

/// Agent's social network - all relationships with other agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialNetwork {
    relationships: HashMap<Uuid, Relationship>,
}

impl SocialNetwork {
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
        }
    }

    /// Get or create relationship with another agent
    pub fn get_or_create_relationship(
        &mut self,
        other_agent_id: Uuid,
        current_tick: u32,
    ) -> &mut Relationship {
        self.relationships
            .entry(other_agent_id)
            .or_insert_with(|| Relationship::new(other_agent_id, current_tick))
    }

    /// Get relationship if it exists
    pub fn get_relationship(&self, other_agent_id: Uuid) -> Option<&Relationship> {
        self.relationships.get(&other_agent_id)
    }

    /// Get mutable relationship if it exists
    pub fn get_relationship_mut(&mut self, other_agent_id: Uuid) -> Option<&mut Relationship> {
        self.relationships.get_mut(&other_agent_id)
    }

    /// Add parent-child relationship
    pub fn add_parent_relationship(&mut self, parent_id: Uuid, current_tick: u32) {
        self.relationships
            .insert(parent_id, Relationship::parent_child(parent_id, current_tick));
    }

    /// Decay all relationships towards neutral over time
    pub fn decay_all_relationships(&mut self, current_tick: u32, base_decay_rate: f32) {
        for relationship in self.relationships.values_mut() {
            relationship.decay_if_no_interaction(current_tick, base_decay_rate);
        }
    }

    /// Get belief weight for information from a specific agent
    pub fn belief_weight_for(&self, agent_id: Uuid) -> f32 {
        self.get_relationship(agent_id)
            .map(|r| r.trust_level.belief_weight())
            .unwrap_or(0.5) // Default neutral trust
    }

    /// Get all relationships
    pub fn all_relationships(&self) -> Vec<&Relationship> {
        self.relationships.values().collect()
    }
}

impl Default for SocialNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_levels() {
        let mut rel = RelationshipLevel::neutral();
        assert_eq!(rel.value(), 0);

        // Move to Likes
        rel.adjust(5);
        assert!(matches!(rel, RelationshipLevel::Likes(_)));

        // Move to Loves
        rel.adjust(15);
        assert!(matches!(rel, RelationshipLevel::Loves(_)));

        // Adjusting by -8 brings us back to Likes
        rel.adjust(-8);
        assert!(matches!(rel, RelationshipLevel::Likes(_)));

        // Adjusting by -5 more brings us to Neutral
        rel.adjust(-5);
        assert!(matches!(rel, RelationshipLevel::Neutral(_)));

        // Move to Dislikes
        rel.adjust(-5);
        assert!(matches!(rel, RelationshipLevel::Dislikes(_)));

        // Move to Hates
        rel.adjust(-15);
        assert!(matches!(rel, RelationshipLevel::Hates(_)));
    }

    #[test]
    fn test_trust_levels() {
        let mut trust = TrustLevel::neutral();
        assert_eq!(trust.value(), 0);

        // Build trust
        trust.adjust(3);
        assert!(matches!(trust, TrustLevel::SlightlyTrusts(_)));

        trust.adjust(6);
        assert!(matches!(trust, TrustLevel::MostlyTrusts(_)));

        // Lose trust - need to go below -10 for DistrustCompletely
        trust.adjust(-20);
        assert!(matches!(trust, TrustLevel::DistrustCompletely(_)));
    }

    #[test]
    fn test_information_verification() {
        let mut rel = Relationship::new(Uuid::new_v4(), 0);

        // Correct recent information builds trust quickly
        rel.verify_information(50, 100);
        assert!(rel.trust_level.value() > 0);

        // Wrong recent information hurts trust
        let mut rel2 = Relationship::new(Uuid::new_v4(), 0);
        rel2.incorrect_information(50, 100);
        assert!(rel2.trust_level.value() < 0);
    }

    #[test]
    fn test_friendship_forgiveness() {
        // Friend with high relationship
        let mut friend = Relationship::new(Uuid::new_v4(), 0);
        friend.relationship_level = RelationshipLevel::Loves(3);

        // Give wrong info
        friend.incorrect_information(50, 100);

        // Stranger with neutral relationship
        let mut stranger = Relationship::new(Uuid::new_v4(), 0);
        stranger.incorrect_information(50, 100);

        // Friend should have less trust penalty than stranger
        assert!(friend.trust_level.value() > stranger.trust_level.value());
    }

    #[test]
    fn test_relationship_decay() {
        let mut rel = RelationshipLevel::Likes(4);

        // Decay towards neutral
        rel.decay_towards_neutral(1.0);

        // Should be closer to neutral
        assert!(rel.value() < RelationshipLevel::Likes(4).value());
    }
}
