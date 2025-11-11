// src/core/traits.rs
//! Personality trait system for agents.
//!
//! Traits modify how agents experience emotions and interact with the world.
//! Each trait provides specific modifiers to emotional responses and behaviors.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// All personality traits that can modify agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trait {
    // Emotional Response Modifiers
    Anxious,        // Doubles fear increase, halves fear reduction
    Brave,          // Halves fear gain, doubles fear loss
    HotHeaded,      // Doubles anger gain, halves anger loss
    Calm,           // Halves anger gain, doubles anger loss
    Pacifist,       // Prevents anger gain
    Empathic,       // Doubles sadness loss and gain
    ColdHearted,    // Halves sadness gain, doubles loss
    Resilient,      // Doubles sadness loss
    Clown,          // Doubles happiness gain, halves happiness loss
    Goth,           // Doubles happiness loss, halves happiness gain
    Melancholic,    // Passive slow gain of sadness
    Stoic,          // Decreases emotional reactions by 50%
    Repressed,      // Decreases average emotions, increases extreme

    // Social Traits
    Extrovert,      // Doubles happiness from socializing, double agent count
    Introvert,      // Happiness when alone, decreases with agents nearby
    KindHearted,    // Doubles happiness loss when seeing agent hurt
    Cruel,          // Reverses happiness loss when seeing pain
    Charismatic,    // Increases socialization benefit, doubles nearby emotional affect
    Mute,           // Cannot use word of mouth
    Gossip,         // Halves happiness except from word of mouth
    Intolerant,     // -3 affection modifier on disagreeing traits
    Mediator,       // Decreases nearby negative emotions
    Romantic,       // Additional happiness from relationships
    Insecure,       // Decreased happiness if partner talks to opposite sex
    Manipulator,    // Happiness from convincing agents to act contrary
    Copycat,        // Happiness from copying nearby agent tasks
    Imaginative,    // Increases nearby agent happiness when sharing

    // Work and Achievement Traits
    Handy,          // Doubles happiness from completing jobs
    Lazy,           // Constant happiness decrease when working
    Proud,          // Happiness from accomplishing goals
    Ambitious,      // Increased happiness from completing external goals
    Pragmatist,     // Happiness from survival-increasing goals
    Stubborn,       // Happiness from consistency, sadness from change
    Traditionalist, // Happiness/speed boost from primitive tools
    Rebel,          // Happiness from unpopular jobs/goals
    Builder,        // Happiness from building structures
    CraftObsessed,  // Happiness from crafted items
    Archivist,      // Happiness from keeping records

    // Resource and Material Traits
    Greedy,         // Happiness bonus from supplies in home
    Ascetic,        // No favorite food, decreases happiness with luxury
    Envious,        // Decreases happiness if others have better items
    Frugal,         // Happiness from stored goods, sadness if used by non-family
    Survivalist,    // Happiness when basic needs met from own stores
    Altruist,       // Happiness from helping others

    // Food and Consumption Traits
    Glutton,        // Increases happiness from favorite food

    // Belief and Knowledge Traits
    Believer,       // Boosts happiness from religious buildings
    Atheist,        // Small decrease at religious buildings, twice happiness at museums
    Ignorant,       // Rare, only occurs with Believer trait
    Zealot,         // Additional happiness in religious buildings
    Skeptic,        // Prevents religious traits, only trusts multiple sources
    Bookworm,       // Doubles curiosity decrease at library, happiness gain
    Curious,        // Happiness from refreshing knowledge and learning

    // Investigation and Awareness Traits
    Suspicious,     // Noise curiosity increases at twice rate
    Deaf,           // Immune to noise events
    Uncaring,       // Reduces noise curiosity by half
    Paranoid,       // Doesn't trust word, assumes malice

    // Conflict and Response Traits
    Vengeful,       // Prevents anger loss unless action taken
    Forgiving,      // Anger decreases at twice speed
    Coward,         // Increased fear in danger, happiness escaping
    Protector,      // Happiness from killing dangerous creatures

    // Interaction with Environment Traits
    AnimalLover,    // Happiness near animals
    Allergic,       // Decreases happiness near animals
    Masochist,      // Happiness from damage until 50% health
    Explorer,       // Happiness from exploring new areas
    Caretaker,      // Happiness from helping sick/injured/elderly

    // Obsession Trait
    Obsessive,      // Gains/loses happiness based on proximity to obsession
}

impl Trait {
    /// Get description of what this trait does
    pub fn description(&self) -> &'static str {
        match self {
            Trait::Anxious => "Doubles fear increase, halves fear reduction",
            Trait::Brave => "Halves fear gain, doubles fear loss",
            Trait::HotHeaded => "Doubles anger gain, halves anger loss",
            Trait::Calm => "Halves anger gain, doubles anger loss",
            Trait::Pacifist => "Prevents anger gain",
            Trait::Empathic => "Doubles sadness loss and gain",
            Trait::ColdHearted => "Halves sadness gain, doubles sadness loss",
            Trait::Resilient => "Doubles sadness loss",
            Trait::Clown => "Doubles happiness gain, halves happiness loss",
            Trait::Goth => "Doubles happiness loss, halves happiness gain",
            Trait::Melancholic => "Passive slow gain of sadness",
            Trait::Stoic => "Decreases all emotional reactions by 50%",
            Trait::Repressed => "Decreases average emotions, increases extreme emotions",
            Trait::Extrovert => "Doubles happiness from socializing",
            Trait::Introvert => "Gains happiness when alone, loses it in crowds",
            Trait::KindHearted => "Doubles happiness loss when seeing others hurt",
            Trait::Cruel => "Gains happiness from seeing others in pain",
            Trait::Charismatic => "Increases socialization benefits and emotional influence",
            Trait::Mute => "Cannot use word of mouth",
            Trait::Gossip => "Halves happiness except from word of mouth",
            Trait::Intolerant => "Decreased affection toward different traits",
            Trait::Mediator => "Decreases nearby negative emotions",
            Trait::Romantic => "Additional happiness from relationships",
            Trait::Insecure => "Decreased happiness if partner talks to others",
            Trait::Manipulator => "Happiness from convincing others to act contrary",
            Trait::Copycat => "Happiness from copying nearby agent tasks",
            Trait::Imaginative => "Increases nearby agent happiness when sharing",
            Trait::Handy => "Doubles happiness from completing jobs",
            Trait::Lazy => "Constant happiness decrease when working",
            Trait::Proud => "Happiness from accomplishing goals",
            Trait::Ambitious => "Increased happiness from completing external goals",
            Trait::Pragmatist => "Happiness from survival-increasing goals",
            Trait::Stubborn => "Happiness from consistency, sadness from change",
            Trait::Traditionalist => "Happiness/speed boost from primitive tools",
            Trait::Rebel => "Happiness from unpopular jobs/goals",
            Trait::Builder => "Happiness from building structures",
            Trait::CraftObsessed => "Happiness from crafted items",
            Trait::Archivist => "Happiness from keeping records",
            Trait::Greedy => "Happiness from supplies stored in home",
            Trait::Ascetic => "No favorite food, dislikes luxury",
            Trait::Envious => "Decreased happiness if others have better items",
            Trait::Frugal => "Happiness from stored goods, sadness if used by non-family",
            Trait::Survivalist => "Happiness when basic needs met from own stores",
            Trait::Altruist => "Happiness from helping others",
            Trait::Glutton => "Increased happiness from favorite food",
            Trait::Believer => "Boosts happiness from religious buildings",
            Trait::Atheist => "Small decrease at religious buildings, happiness at museums",
            Trait::Ignorant => "Limited knowledge acceptance",
            Trait::Zealot => "Additional happiness in religious buildings near believers",
            Trait::Skeptic => "Only trusts multiple sources, no religious traits",
            Trait::Bookworm => "Doubles curiosity decrease at library, happiness gain",
            Trait::Curious => "Happiness from learning and discovering",
            Trait::Suspicious => "Noise curiosity increases at twice rate",
            Trait::Deaf => "Immune to noise events",
            Trait::Uncaring => "Reduces noise curiosity by half",
            Trait::Paranoid => "Doesn't trust others, assumes malice",
            Trait::Vengeful => "Prevents anger loss unless action taken against target",
            Trait::Forgiving => "Anger decreases at twice normal speed",
            Trait::Coward => "Increased fear in danger, happiness when escaping",
            Trait::Protector => "Happiness from protecting others and fighting dangers",
            Trait::AnimalLover => "Happiness near animals",
            Trait::Allergic => "Decreases happiness near animals",
            Trait::Masochist => "Happiness from damage until 50% health",
            Trait::Explorer => "Happiness from exploring new areas",
            Trait::Caretaker => "Happiness from helping sick/injured/elderly",
            Trait::Obsessive => "Strong focus on specific obsession target",
        }
    }

    /// Check if two traits are incompatible
    pub fn is_incompatible_with(&self, other: &Trait) -> bool {
        matches!(
            (self, other),
            (Trait::Anxious, Trait::Brave) | (Trait::Brave, Trait::Anxious) |
            (Trait::HotHeaded, Trait::Calm) | (Trait::Calm, Trait::HotHeaded) |
            (Trait::HotHeaded, Trait::Pacifist) | (Trait::Pacifist, Trait::HotHeaded) |
            (Trait::Empathic, Trait::ColdHearted) | (Trait::ColdHearted, Trait::Empathic) |
            (Trait::Extrovert, Trait::Introvert) | (Trait::Introvert, Trait::Extrovert) |
            (Trait::KindHearted, Trait::Cruel) | (Trait::Cruel, Trait::KindHearted) |
            (Trait::Clown, Trait::Goth) | (Trait::Goth, Trait::Clown) |
            (Trait::Believer, Trait::Atheist) | (Trait::Atheist, Trait::Believer) |
            (Trait::Believer, Trait::Skeptic) | (Trait::Skeptic, Trait::Believer) |
            (Trait::Zealot, Trait::Atheist) | (Trait::Atheist, Trait::Zealot) |
            (Trait::Zealot, Trait::Skeptic) | (Trait::Skeptic, Trait::Zealot) |
            (Trait::Greedy, Trait::Ascetic) | (Trait::Ascetic, Trait::Greedy) |
            (Trait::Greedy, Trait::Altruist) | (Trait::Altruist, Trait::Greedy) |
            (Trait::Greedy, Trait::Frugal) | (Trait::Frugal, Trait::Greedy) |
            (Trait::AnimalLover, Trait::Allergic) | (Trait::Allergic, Trait::AnimalLover) |
            (Trait::Vengeful, Trait::Forgiving) | (Trait::Forgiving, Trait::Vengeful) |
            (Trait::Brave, Trait::Coward) | (Trait::Coward, Trait::Brave) |
            (Trait::Handy, Trait::Lazy) | (Trait::Lazy, Trait::Handy) |
            (Trait::Stoic, Trait::Repressed) | (Trait::Repressed, Trait::Stoic) |
            (Trait::Charismatic, Trait::Mute) | (Trait::Mute, Trait::Charismatic)
        )
    }

    /// Get emotion modifier for this trait
    /// Returns (emotion_type, increase_multiplier, decrease_multiplier)
    pub fn emotion_modifiers(&self) -> Vec<(crate::core::EmotionType, f32, f32)> {
        use crate::core::EmotionType;

        match self {
            Trait::Anxious => vec![(EmotionType::Fear, 2.0, 0.5)],
            Trait::Brave => vec![(EmotionType::Fear, 0.5, 2.0)],
            Trait::HotHeaded => vec![(EmotionType::Anger, 2.0, 0.5)],
            Trait::Calm => vec![(EmotionType::Anger, 0.5, 2.0)],
            Trait::Pacifist => vec![(EmotionType::Anger, 0.0, 1.0)],
            Trait::Empathic => vec![(EmotionType::Sadness, 2.0, 2.0)],
            Trait::ColdHearted => vec![(EmotionType::Sadness, 0.5, 2.0)],
            Trait::Resilient => vec![(EmotionType::Sadness, 1.0, 2.0)],
            Trait::Clown => vec![(EmotionType::Happiness, 2.0, 0.5)],
            Trait::Goth => vec![(EmotionType::Happiness, 0.5, 2.0)],
            Trait::Stoic => vec![
                (EmotionType::Fear, 0.5, 0.5),
                (EmotionType::Anger, 0.5, 0.5),
                (EmotionType::Sadness, 0.5, 0.5),
                (EmotionType::Happiness, 0.5, 0.5),
                (EmotionType::Curiosity, 0.5, 0.5),
            ],
            Trait::Forgiving => vec![(EmotionType::Anger, 1.0, 2.0)],
            _ => vec![],
        }
    }
}

/// Agent's trait collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitSet {
    pub traits: Vec<Trait>,
}

impl TraitSet {
    pub fn new() -> Self {
        Self {
            traits: Vec::new(),
        }
    }

    /// Add a trait if compatible with existing traits
    pub fn add_trait(&mut self, trait_to_add: Trait) -> bool {
        // Check if already has this trait
        if self.traits.contains(&trait_to_add) {
            return false;
        }

        // Check incompatibilities
        for existing_trait in &self.traits {
            if trait_to_add.is_incompatible_with(existing_trait) {
                return false;
            }
        }

        self.traits.push(trait_to_add);
        true
    }

    /// Check if agent has a specific trait
    pub fn has(&self, trait_check: Trait) -> bool {
        self.traits.contains(&trait_check)
    }

    /// Get all emotion modifiers from all traits
    pub fn get_combined_emotion_modifiers(&self) -> Vec<(crate::core::EmotionType, f32, f32)> {
        self.traits
            .iter()
            .flat_map(|t| t.emotion_modifiers())
            .collect()
    }

    /// Generate random traits for a new agent
    pub fn generate_random(count: usize) -> Self {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let all_traits = [
            Trait::Anxious, Trait::Brave, Trait::HotHeaded, Trait::Calm,
            Trait::Pacifist, Trait::Empathic, Trait::ColdHearted, Trait::Resilient,
            Trait::Clown, Trait::Goth, Trait::Melancholic, Trait::Stoic,
            Trait::Extrovert, Trait::Introvert, Trait::KindHearted, Trait::Cruel,
            Trait::Charismatic, Trait::Gossip, Trait::Intolerant, Trait::Mediator,
            Trait::Romantic, Trait::Insecure, Trait::Handy, Trait::Lazy,
            Trait::Proud, Trait::Ambitious, Trait::Pragmatist, Trait::Stubborn,
            Trait::Traditionalist, Trait::Rebel, Trait::Builder, Trait::CraftObsessed,
            Trait::Greedy, Trait::Ascetic, Trait::Envious, Trait::Frugal,
            Trait::Survivalist, Trait::Altruist, Trait::Glutton, Trait::Believer,
            Trait::Atheist, Trait::Zealot, Trait::Skeptic, Trait::Bookworm,
            Trait::Curious, Trait::Suspicious, Trait::Uncaring, Trait::Vengeful,
            Trait::Forgiving, Trait::Coward, Trait::Protector, Trait::AnimalLover,
            Trait::Allergic, Trait::Explorer, Trait::Caretaker,
        ];

        let mut rng = thread_rng();
        let mut selected = all_traits.choose_multiple(&mut rng, count * 2).cloned().collect::<Vec<_>>();

        let mut trait_set = TraitSet::new();
        for trait_candidate in selected {
            if trait_set.add_trait(trait_candidate) && trait_set.traits.len() >= count {
                break;
            }
        }

        trait_set
    }
}

impl Default for TraitSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trait_incompatibility() {
        assert!(Trait::Anxious.is_incompatible_with(&Trait::Brave));
        assert!(Trait::Brave.is_incompatible_with(&Trait::Anxious));
        assert!(!Trait::Anxious.is_incompatible_with(&Trait::Calm));
    }

    #[test]
    fn test_trait_set_creation() {
        let trait_set = TraitSet::new();
        assert_eq!(trait_set.traits.len(), 0);
    }

    #[test]
    fn test_add_compatible_trait() {
        let mut trait_set = TraitSet::new();
        assert!(trait_set.add_trait(Trait::Brave));
        assert!(trait_set.add_trait(Trait::Calm));
        assert_eq!(trait_set.traits.len(), 2);
    }

    #[test]
    fn test_reject_incompatible_trait() {
        let mut trait_set = TraitSet::new();
        assert!(trait_set.add_trait(Trait::Brave));
        assert!(!trait_set.add_trait(Trait::Anxious)); // Incompatible
        assert_eq!(trait_set.traits.len(), 1);
    }

    #[test]
    fn test_has_trait() {
        let mut trait_set = TraitSet::new();
        trait_set.add_trait(Trait::Brave);

        assert!(trait_set.has(Trait::Brave));
        assert!(!trait_set.has(Trait::Anxious));
    }

    #[test]
    fn test_emotion_modifiers() {
        let modifiers = Trait::Anxious.emotion_modifiers();
        assert!(!modifiers.is_empty());
    }

    #[test]
    fn test_generate_random_traits() {
        let trait_set = TraitSet::generate_random(3);
        assert!(trait_set.traits.len() >= 1 && trait_set.traits.len() <= 3);
    }
}
