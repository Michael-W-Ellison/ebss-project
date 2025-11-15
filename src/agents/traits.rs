// src/agents/traits.rs
//! Personality trait system affecting agent behavior and information sharing.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Personality traits that affect agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trait {
    // Social traits
    /// Exaggerates information for drama, gains happiness from storytelling
    Imaginative,
    /// Lies and manipulates for personal gain, gains happiness from chaos
    Manipulative,
    /// Forgives offenses easily, reduces anger/conflict
    Forgiving,
    /// Trusts others easily
    Trusting,
    /// Distrusts others, questions information
    Suspicious,

    // Belief traits
    /// Religious believer, dislikes atheists
    Believer,
    /// Atheist, may dislike believers
    Atheist,

    // Work ethic
    /// Hard worker, gains satisfaction from labor
    Diligent,
    /// Avoids work when possible
    Lazy,

    // Social behavior
    /// Enjoys social interaction
    Sociable,
    /// Prefers solitude
    Introverted,
    /// Aggressive in conflicts
    Aggressive,
    /// Avoids confrontation
    Peaceful,

    // Honesty
    /// Always tells the truth
    Honest,
    /// Frequently lies
    Dishonest,

    // Emotional
    /// Quick to anger
    Hottempered,
    /// Slow to anger
    Calm,
    /// Feels others' emotions strongly
    Empathetic,
    /// Doesn't care about others' feelings
    Callous,
}

impl Trait {
    /// Get trait name
    pub fn name(&self) -> &'static str {
        match self {
            Trait::Imaginative => "Imaginative",
            Trait::Manipulative => "Manipulative",
            Trait::Forgiving => "Forgiving",
            Trait::Trusting => "Trusting",
            Trait::Suspicious => "Suspicious",
            Trait::Believer => "Believer",
            Trait::Atheist => "Atheist",
            Trait::Diligent => "Diligent",
            Trait::Lazy => "Lazy",
            Trait::Sociable => "Sociable",
            Trait::Introverted => "Introverted",
            Trait::Aggressive => "Aggressive",
            Trait::Peaceful => "Peaceful",
            Trait::Honest => "Honest",
            Trait::Dishonest => "Dishonest",
            Trait::Hottempered => "Hottempered",
            Trait::Calm => "Calm",
            Trait::Empathetic => "Empathetic",
            Trait::Callous => "Callous",
        }
    }

    /// Check if traits are incompatible
    pub fn incompatible_with(&self, other: &Trait) -> bool {
        matches!(
            (self, other),
            (Trait::Believer, Trait::Atheist)
                | (Trait::Atheist, Trait::Believer)
                | (Trait::Trusting, Trait::Suspicious)
                | (Trait::Suspicious, Trait::Trusting)
                | (Trait::Diligent, Trait::Lazy)
                | (Trait::Lazy, Trait::Diligent)
                | (Trait::Sociable, Trait::Introverted)
                | (Trait::Introverted, Trait::Sociable)
                | (Trait::Aggressive, Trait::Peaceful)
                | (Trait::Peaceful, Trait::Aggressive)
                | (Trait::Honest, Trait::Dishonest)
                | (Trait::Dishonest, Trait::Honest)
                | (Trait::Hottempered, Trait::Calm)
                | (Trait::Calm, Trait::Hottempered)
                | (Trait::Empathetic, Trait::Callous)
                | (Trait::Callous, Trait::Empathetic)
        )
    }

    /// Get happiness gain from expressing this trait
    pub fn happiness_gain(&self) -> f32 {
        match self {
            Trait::Imaginative => 5.0,  // From embellishing stories
            Trait::Manipulative => 10.0, // From successful manipulation
            Trait::Forgiving => 3.0,     // From forgiving others
            Trait::Sociable => 5.0,      // From social interaction
            Trait::Diligent => 2.0,      // From hard work
            _ => 0.0,
        }
    }

    /// Anger reduction multiplier for Forgiving trait
    pub fn anger_reduction(&self) -> f32 {
        match self {
            Trait::Forgiving => 0.5, // 50% less anger
            Trait::Calm => 0.7,      // 30% less anger
            Trait::Hottempered => 1.5, // 50% more anger
            _ => 1.0,
        }
    }

    /// Trust modifier for evaluating information
    pub fn trust_modifier(&self) -> f32 {
        match self {
            Trait::Trusting => 0.3,    // +30% trust
            Trait::Suspicious => -0.3, // -30% trust
            Trait::Honest => 0.2,      // Others trust you +20%
            Trait::Dishonest => -0.2,  // Others trust you -20%
            _ => 0.0,
        }
    }
}

/// Collection of traits for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitSet {
    traits: HashSet<Trait>,
}

impl TraitSet {
    pub fn new() -> Self {
        Self {
            traits: HashSet::new(),
        }
    }

    /// Add a trait if compatible with existing traits
    pub fn add_trait(&mut self, trait_to_add: Trait) -> bool {
        // Check for incompatibilities
        for existing_trait in &self.traits {
            if trait_to_add.incompatible_with(existing_trait) {
                return false; // Cannot add incompatible trait
            }
        }

        self.traits.insert(trait_to_add);
        true
    }

    /// Remove a trait
    pub fn remove_trait(&mut self, trait_to_remove: &Trait) {
        self.traits.remove(trait_to_remove);
    }

    /// Check if agent has a trait
    pub fn has_trait(&self, trait_check: &Trait) -> bool {
        self.traits.contains(trait_check)
    }

    /// Get all traits
    pub fn get_traits(&self) -> &HashSet<Trait> {
        &self.traits
    }

    /// Calculate combined anger reduction from all traits
    pub fn combined_anger_reduction(&self) -> f32 {
        let mut multiplier = 1.0;
        for trait_item in &self.traits {
            multiplier *= trait_item.anger_reduction();
        }
        multiplier
    }

    /// Calculate combined trust modifier from all traits
    pub fn combined_trust_modifier(&self) -> f32 {
        self.traits
            .iter()
            .map(|t| t.trust_modifier())
            .sum()
    }

    /// Check if agent would distort information
    pub fn would_distort_info(&self) -> Option<Trait> {
        if self.has_trait(&Trait::Manipulative) {
            Some(Trait::Manipulative)
        } else if self.has_trait(&Trait::Imaginative) {
            Some(Trait::Imaginative)
        } else {
            None
        }
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
        assert!(Trait::Believer.incompatible_with(&Trait::Atheist));
        assert!(Trait::Trusting.incompatible_with(&Trait::Suspicious));
        assert!(!Trait::Imaginative.incompatible_with(&Trait::Sociable));
    }

    #[test]
    fn test_trait_set_add_compatible() {
        let mut traits = TraitSet::new();
        assert!(traits.add_trait(Trait::Imaginative));
        assert!(traits.add_trait(Trait::Sociable));
        assert_eq!(traits.get_traits().len(), 2);
    }

    #[test]
    fn test_trait_set_add_incompatible() {
        let mut traits = TraitSet::new();
        assert!(traits.add_trait(Trait::Believer));
        assert!(!traits.add_trait(Trait::Atheist)); // Should fail
        assert_eq!(traits.get_traits().len(), 1);
    }

    #[test]
    fn test_trait_happiness_gain() {
        assert_eq!(Trait::Imaginative.happiness_gain(), 5.0);
        assert_eq!(Trait::Manipulative.happiness_gain(), 10.0);
    }

    #[test]
    fn test_anger_reduction() {
        assert_eq!(Trait::Forgiving.anger_reduction(), 0.5);
        assert_eq!(Trait::Hottempered.anger_reduction(), 1.5);
    }

    #[test]
    fn test_combined_anger_reduction() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Forgiving);
        traits.add_trait(Trait::Calm);

        // 0.5 * 0.7 = 0.35
        assert_eq!(traits.combined_anger_reduction(), 0.35);
    }

    #[test]
    fn test_would_distort_info() {
        let mut traits = TraitSet::new();
        assert!(traits.would_distort_info().is_none());

        traits.add_trait(Trait::Imaginative);
        assert_eq!(traits.would_distort_info(), Some(Trait::Imaginative));
    }

    #[test]
    fn test_trust_modifier() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Trusting);
        traits.add_trait(Trait::Honest);

        // 0.3 + 0.2 = 0.5
        assert_eq!(traits.combined_trust_modifier(), 0.5);
    }
}
