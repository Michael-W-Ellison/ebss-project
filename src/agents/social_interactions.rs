// src/agents/social_interactions.rs
//! Social interaction system for agents.
//!
//! This module handles various social behaviors including:
//! - Greetings when agents meet
//! - Conversations that build relationships
//! - Gift-giving to strengthen bonds
//! - Cooperative actions and helping behavior
//! - Social drive satisfaction

use serde::{Deserialize, Serialize};
use crate::world::ItemType;
use super::relationships::{RelationshipLevel, TrustLevel};
use crate::core::traits::Trait;

/// Types of social interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SocialInteractionType {
    /// Initial greeting when meeting
    Greet,
    /// General conversation
    Converse {
        topic: ConversationTopic,
    },
    /// Give item as gift
    GiveGift {
        item_type: ItemType,
        quantity: u32,
    },
    /// Offer to help with a task
    OfferHelp {
        help_type: HelpType,
    },
    /// Express appreciation
    ThankYou,
    /// Compliment another agent
    Compliment,
    /// Share a meal
    ShareMeal,
}

/// Topics for conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConversationTopic {
    /// Small talk about weather, daily life
    SmallTalk,
    /// Discussing work and tasks
    Work,
    /// Sharing stories and experiences
    Stories,
    /// Talking about family
    Family,
    /// Discussing technologies and discoveries
    Technology,
    /// Philosophical or religious topics
    Beliefs,
}

/// Types of help agents can offer
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HelpType {
    /// Help with resource gathering
    Gathering,
    /// Help with construction
    Building,
    /// Help with crafting
    Crafting,
    /// Help with carrying items
    Transport,
    /// General assistance
    General,
}

/// Result of a social interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialInteractionResult {
    /// Whether the interaction was successful
    pub success: bool,
    /// Change in relationship level
    pub relationship_change: i8,
    /// Change in trust level
    pub trust_change: i8,
    /// Amount of social drive satisfied
    pub social_satisfaction: f32,
    /// Message describing what happened
    pub message: String,
}

impl SocialInteractionResult {
    pub fn success(message: String, relationship_change: i8, social_satisfaction: f32) -> Self {
        Self {
            success: true,
            relationship_change,
            trust_change: 0,
            social_satisfaction,
            message,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            relationship_change: 0,
            trust_change: 0,
            social_satisfaction: 0.0,
            message,
        }
    }
}

/// Calculate relationship change from a social interaction
pub fn calculate_relationship_change(
    interaction_type: &SocialInteractionType,
    initiator_traits: &[Trait],
    recipient_traits: &[Trait],
    current_relationship: &RelationshipLevel,
) -> i8 {
    let base_change = match interaction_type {
        SocialInteractionType::Greet => 1,
        SocialInteractionType::Converse { topic } => match topic {
            ConversationTopic::SmallTalk => 1,
            ConversationTopic::Work => 1,
            ConversationTopic::Stories => 2,
            ConversationTopic::Family => 3,
            ConversationTopic::Technology => 2,
            ConversationTopic::Beliefs => {
                // Beliefs can be positive or negative depending on compatibility
                if has_incompatible_beliefs(initiator_traits, recipient_traits) {
                    -2
                } else if has_compatible_beliefs(initiator_traits, recipient_traits) {
                    3
                } else {
                    1
                }
            }
        },
        SocialInteractionType::GiveGift { quantity, .. } => {
            // More valuable gifts improve relationship more
            ((*quantity as f32 / 10.0).min(5.0) as i8).max(1)
        }
        SocialInteractionType::OfferHelp { .. } => 2,
        SocialInteractionType::ThankYou => 1,
        SocialInteractionType::Compliment => 2,
        SocialInteractionType::ShareMeal => 3,
    };

    // Trait modifiers
    let mut modifier = 1.0;

    // Sociable agents get more benefit from interactions
    if initiator_traits.contains(&Trait::Sociable) {
        modifier += 0.3;
    }

    // Introverted agents get less benefit
    if initiator_traits.contains(&Trait::Introverted) {
        modifier -= 0.2;
    }

    // Aggressive agents have difficulty building positive relationships
    if initiator_traits.contains(&Trait::Aggressive) && base_change > 0 {
        modifier -= 0.3;
    }

    // Peaceful agents build relationships more easily
    if initiator_traits.contains(&Trait::Peaceful) && base_change > 0 {
        modifier += 0.2;
    }

    // Calculate final change
    let mut final_change = (base_change as f32 * modifier) as i8;

    // Diminishing returns for already high relationships
    let current_value = current_relationship.value();
    if current_value > 15 && final_change > 0 {
        final_change = (final_change / 2).max(1);
    }

    final_change
}

/// Calculate social drive satisfaction from interaction
pub fn calculate_social_satisfaction(
    interaction_type: &SocialInteractionType,
    initiator_traits: &[Trait],
    relationship: &RelationshipLevel,
) -> f32 {
    let base_satisfaction = match interaction_type {
        SocialInteractionType::Greet => 0.05,
        SocialInteractionType::Converse { topic } => match topic {
            ConversationTopic::SmallTalk => 0.1,
            ConversationTopic::Work => 0.08,
            ConversationTopic::Stories => 0.15,
            ConversationTopic::Family => 0.2,
            ConversationTopic::Technology => 0.12,
            ConversationTopic::Beliefs => 0.1,
        },
        SocialInteractionType::GiveGift { .. } => 0.15,
        SocialInteractionType::OfferHelp { .. } => 0.1,
        SocialInteractionType::ThankYou => 0.05,
        SocialInteractionType::Compliment => 0.08,
        SocialInteractionType::ShareMeal => 0.25,
    };

    let mut modifier = 1.0_f32;

    // Sociable agents enjoy socializing
    if initiator_traits.contains(&Trait::Sociable) {
        modifier += 0.8;
    }

    // Introverted agents get less satisfaction
    if initiator_traits.contains(&Trait::Introverted) {
        modifier -= 0.4;
    }

    // Better relationships provide more satisfaction
    let relationship_bonus = match relationship {
        RelationshipLevel::Loves(_) => 0.5,
        RelationshipLevel::Likes(_) => 0.3,
        RelationshipLevel::Neutral(_) => 0.0,
        RelationshipLevel::Dislikes(_) => -0.3,
        RelationshipLevel::Hates(_) => -0.5,
    };

    ((base_satisfaction * modifier) + relationship_bonus).max(0.0_f32)
}

/// Determine if two agents should greet each other
pub fn should_greet(
    last_interaction_tick: u32,
    current_tick: u32,
    relationship: &RelationshipLevel,
) -> bool {
    // Greet if haven't interacted in a while
    let ticks_since_interaction = current_tick.saturating_sub(last_interaction_tick);

    // Greet interval depends on relationship
    let greet_interval = match relationship {
        RelationshipLevel::Loves(_) => 500,   // Greet close friends/family more often
        RelationshipLevel::Likes(_) => 1000,
        RelationshipLevel::Neutral(_) => 2000,
        RelationshipLevel::Dislikes(_) => 5000, // Rarely greet those disliked
        RelationshipLevel::Hates(_) => 10000,   // Almost never greet enemies
    };

    ticks_since_interaction >= greet_interval
}

/// Check for incompatible beliefs
fn has_incompatible_beliefs(traits1: &[Trait], traits2: &[Trait]) -> bool {
    (traits1.contains(&Trait::Believer) && traits2.contains(&Trait::Atheist)) ||
    (traits1.contains(&Trait::Atheist) && traits2.contains(&Trait::Believer))
}

/// Check for compatible beliefs
fn has_compatible_beliefs(traits1: &[Trait], traits2: &[Trait]) -> bool {
    (traits1.contains(&Trait::Believer) && traits2.contains(&Trait::Believer)) ||
    (traits1.contains(&Trait::Atheist) && traits2.contains(&Trait::Atheist))
}

/// Determine appropriate conversation topic based on relationship and traits
pub fn select_conversation_topic(
    relationship: &RelationshipLevel,
    _initiator_traits: &[Trait],
    _recipient_traits: &[Trait],
) -> ConversationTopic {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Close relationships can discuss deeper topics
    match relationship {
        RelationshipLevel::Loves(_) => {
            // Family and stories with loved ones
            let topics = vec![
                ConversationTopic::Family,
                ConversationTopic::Stories,
                ConversationTopic::Work,
            ];
            *topics.get(rng.gen_range(0..topics.len())).unwrap()
        }
        RelationshipLevel::Likes(_) => {
            // Varied topics with friends
            let topics = vec![
                ConversationTopic::SmallTalk,
                ConversationTopic::Work,
                ConversationTopic::Stories,
                ConversationTopic::Technology,
            ];
            *topics.get(rng.gen_range(0..topics.len())).unwrap()
        }
        _ => {
            // Stick to safe topics with strangers or enemies
            if rng.gen_bool(0.7) {
                ConversationTopic::SmallTalk
            } else {
                ConversationTopic::Work
            }
        }
    }
}

/// Calculate gift value based on item type and quantity
pub fn calculate_gift_value(item_type: &ItemType, quantity: u32) -> f32 {
    let base_value = match item_type {
        // Basic resources (low value)
        ItemType::Wood | ItemType::Stone => 1.0,
        ItemType::Food => 2.0,

        // Processed materials (medium value)
        ItemType::Iron | ItemType::Cloth | ItemType::Leather => 3.0,

        // Tools and equipment (high value)
        ItemType::WoodenAxe | ItemType::StoneAxe => 5.0,
        ItemType::IronAxe => 8.0,

        // Luxury items (very high value)
        ItemType::Jewelry | ItemType::Pottery => 10.0,
        ItemType::Furniture => 7.0,

        // Food items (medium-high value)
        ItemType::Bread | ItemType::Cheese => 4.0,
        ItemType::Ale => 5.0,

        // Clothing (medium-high value)
        ItemType::Clothing | ItemType::Shoes => 6.0,
        ItemType::LeatherArmor => 8.0,

        // Default
        _ => 2.0,
    };

    base_value * (quantity as f32)
}

/// Determine if an agent would accept a gift
pub fn would_accept_gift(
    relationship: &RelationshipLevel,
    trust: &TrustLevel,
    recipient_traits: &[Trait],
) -> bool {
    // Suspicious agents are less likely to accept gifts
    let suspicion_modifier = if recipient_traits.contains(&Trait::Suspicious) {
        0.5
    } else if recipient_traits.contains(&Trait::Trusting) {
        1.5
    } else {
        1.0
    };

    // Base acceptance rate depends on relationship
    let base_rate = match relationship {
        RelationshipLevel::Loves(_) => 0.95,
        RelationshipLevel::Likes(_) => 0.85,
        RelationshipLevel::Neutral(_) => 0.6,
        RelationshipLevel::Dislikes(_) => 0.3,
        RelationshipLevel::Hates(_) => 0.1,
    };

    // Trust affects acceptance
    let trust_modifier = (trust.value() as f32 / 12.0).max(-0.3).min(0.3);

    let acceptance_rate = (base_rate + trust_modifier) * suspicion_modifier;

    use rand::Rng;
    rand::thread_rng().gen_bool(acceptance_rate.max(0.0).min(1.0) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet_timing() {
        let relationship = RelationshipLevel::neutral();

        // Should greet after long time
        assert!(should_greet(0, 3000, &relationship));

        // Should not greet right after last interaction
        assert!(!should_greet(2900, 3000, &relationship));
    }

    #[test]
    fn test_social_satisfaction() {
        let greet = SocialInteractionType::Greet;
        let traits = vec![Trait::Sociable];
        let relationship = RelationshipLevel::neutral();

        let satisfaction = calculate_social_satisfaction(&greet, &traits, &relationship);
        assert!(satisfaction > 0.0);
    }

    #[test]
    fn test_relationship_change() {
        let greet = SocialInteractionType::Greet;
        let traits1 = vec![Trait::Sociable];
        let traits2 = vec![Trait::Peaceful];
        let relationship = RelationshipLevel::neutral();

        let change = calculate_relationship_change(&greet, &traits1, &traits2, &relationship);
        assert!(change > 0);
    }

    #[test]
    fn test_incompatible_beliefs() {
        let believer = vec![Trait::Believer];
        let atheist = vec![Trait::Atheist];

        assert!(has_incompatible_beliefs(&believer, &atheist));
        assert!(has_incompatible_beliefs(&atheist, &believer));
        assert!(!has_incompatible_beliefs(&believer, &believer));
    }

    #[test]
    fn test_gift_acceptance() {
        let loved = RelationshipLevel::Loves(3);
        let trust = TrustLevel::neutral();
        let traits = vec![Trait::Trusting];

        // Should almost always accept from loved ones
        assert!(would_accept_gift(&loved, &trust, &traits));
    }

    #[test]
    fn test_gift_value() {
        let value = calculate_gift_value(&ItemType::Jewelry, 1);
        assert!(value >= 10.0);

        let wood_value = calculate_gift_value(&ItemType::Wood, 10);
        assert!(wood_value == 10.0);
    }
}
