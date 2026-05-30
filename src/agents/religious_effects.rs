// src/agents/religious_effects.rs
//! Religious building effects on agent happiness.
//!
//! Implements happiness bonuses and penalties based on agent traits
//! and proximity to religious buildings (Shrines, Temples).

use crate::agents::Trait;
use crate::core::traits::TraitSet;
use crate::world::{BuildingType, Position};

/// Distance within which religious buildings affect agents
pub const RELIGIOUS_EFFECT_RADIUS: u32 = 10;

/// Happiness effect from being near a religious building
#[derive(Debug, Clone)]
pub struct ReligiousEffect {
    /// Happiness modifier (-1.0 to 1.0)
    pub happiness_modifier: f32,
    /// Source description
    pub source: String,
    /// Whether this is a positive (blessing) or negative effect
    pub is_positive: bool,
}

impl ReligiousEffect {
    pub fn blessing(amount: f32, source: &str) -> Self {
        Self {
            happiness_modifier: amount.abs(),
            source: source.to_string(),
            is_positive: true,
        }
    }

    pub fn discomfort(amount: f32, source: &str) -> Self {
        Self {
            happiness_modifier: -amount.abs(),
            source: source.to_string(),
            is_positive: false,
        }
    }
}

/// Calculate religious effects for an agent based on their position and nearby buildings
///
/// # Arguments
/// * `agent_position` - The agent's current position
/// * `agent_traits` - The agent's trait set
/// * `buildings` - List of (position, building_type, is_completed) tuples
/// * `nearby_believers` - Number of agents with Believer trait within effect radius
///
/// # Returns
/// A vector of religious effects to apply to the agent
pub fn calculate_religious_effects(
    agent_position: Position,
    agent_traits: &TraitSet,
    buildings: &[(Position, BuildingType, bool)],
    nearby_believers: u32,
) -> Vec<ReligiousEffect> {
    let mut effects = Vec::new();

    // Check agent's religious traits
    let is_believer = agent_traits.has(Trait::Believer);
    let is_atheist = agent_traits.has(Trait::Atheist);
    let is_zealot = agent_traits.has(Trait::Zealot);

    // If agent has no religious traits, no effects apply
    if !is_believer && !is_atheist && !is_zealot {
        return effects;
    }

    // Find nearby religious buildings
    for (building_pos, building_type, is_completed) in buildings {
        // Only completed buildings have effects
        if !is_completed {
            continue;
        }

        // Check if it's a religious building
        if !building_type.is_religious() {
            continue;
        }

        // Calculate distance
        let distance = agent_position.distance_to(building_pos);
        if distance > RELIGIOUS_EFFECT_RADIUS {
            continue;
        }

        // Calculate proximity factor (closer = stronger effect)
        let proximity_factor = 1.0 - (distance as f32 / RELIGIOUS_EFFECT_RADIUS as f32);

        // Building strength factor (Temple > Shrine)
        let building_strength = match building_type {
            BuildingType::Temple => 1.5,
            BuildingType::Shrine => 1.0,
            _ => 0.0,
        };

        // Apply effects based on traits
        if is_believer {
            // Believers gain happiness from religious buildings
            let base_bonus = 0.15 * building_strength * proximity_factor;
            effects.push(ReligiousEffect::blessing(
                base_bonus,
                &format!("Spiritual fulfillment from nearby {:?}", building_type),
            ));
        }

        if is_zealot {
            // Zealots get additional happiness, especially with other believers
            let zealot_bonus = 0.10 * building_strength * proximity_factor;
            effects.push(ReligiousEffect::blessing(
                zealot_bonus,
                &format!("Zealous devotion at {:?}", building_type),
            ));

            // Extra bonus when other believers are nearby
            if nearby_believers > 0 {
                let community_bonus = 0.05 * (nearby_believers as f32).min(5.0) * proximity_factor;
                effects.push(ReligiousEffect::blessing(
                    community_bonus,
                    "Fellowship with other believers",
                ));
            }
        }

        if is_atheist {
            // Atheists feel uncomfortable at religious buildings
            let discomfort = 0.08 * building_strength * proximity_factor;
            effects.push(ReligiousEffect::discomfort(
                discomfort,
                &format!("Discomfort near {:?}", building_type),
            ));
        }
    }

    effects
}

/// Calculate the total happiness modifier from religious effects
pub fn total_happiness_modifier(effects: &[ReligiousEffect]) -> f32 {
    effects.iter().map(|e| e.happiness_modifier).sum()
}

/// Check if an agent should seek out religious buildings (for Believers/Zealots)
pub fn should_seek_religious_building(traits: &TraitSet, current_happiness: f32) -> bool {
    let is_believer = traits.has(Trait::Believer);
    let is_zealot = traits.has(Trait::Zealot);

    // Believers seek religious buildings when happiness is below 0.6
    // Zealots seek them more aggressively (below 0.8)
    if is_zealot && current_happiness < 0.8 {
        return true;
    }
    if is_believer && current_happiness < 0.6 {
        return true;
    }
    false
}

/// Check if an agent should avoid religious buildings (for Atheists)
pub fn should_avoid_religious_building(traits: &TraitSet) -> bool {
    traits.has(Trait::Atheist)
}

/// Get happiness bonus for Atheists at museums/libraries (secular knowledge centers)
pub fn secular_knowledge_bonus(building_type: BuildingType, traits: &TraitSet) -> f32 {
    if !traits.has(Trait::Atheist) {
        return 0.0;
    }

    // Atheists get happiness at knowledge-focused buildings
    match building_type {
        BuildingType::Scriptorium => 0.20,  // Writing/printing center
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position(x: i32, y: i32) -> Position {
        Position::new(x, y)
    }

    #[test]
    fn test_believer_gets_happiness_at_shrine() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Believer);

        let agent_pos = make_position(5, 5);
        let buildings = vec![
            (make_position(5, 5), BuildingType::Shrine, true),
        ];

        let effects = calculate_religious_effects(agent_pos, &traits, &buildings, 0);

        assert!(!effects.is_empty());
        assert!(effects[0].is_positive);
        assert!(effects[0].happiness_modifier > 0.0);
    }

    #[test]
    fn test_atheist_uncomfortable_at_temple() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Atheist);

        let agent_pos = make_position(5, 5);
        let buildings = vec![
            (make_position(5, 5), BuildingType::Temple, true),
        ];

        let effects = calculate_religious_effects(agent_pos, &traits, &buildings, 0);

        assert!(!effects.is_empty());
        assert!(!effects[0].is_positive);
        assert!(effects[0].happiness_modifier < 0.0);
    }

    #[test]
    fn test_zealot_bonus_with_believers() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Zealot);

        let agent_pos = make_position(5, 5);
        let buildings = vec![
            (make_position(5, 5), BuildingType::Temple, true),
        ];

        // Without believers
        let effects_alone = calculate_religious_effects(agent_pos, &traits, &buildings, 0);
        let happiness_alone = total_happiness_modifier(&effects_alone);

        // With believers
        let effects_community = calculate_religious_effects(agent_pos, &traits, &buildings, 3);
        let happiness_community = total_happiness_modifier(&effects_community);

        // Community should provide more happiness
        assert!(happiness_community > happiness_alone);
    }

    #[test]
    fn test_no_effect_for_non_religious_agents() {
        let traits = TraitSet::new(); // No religious traits

        let agent_pos = make_position(5, 5);
        let buildings = vec![
            (make_position(5, 5), BuildingType::Temple, true),
        ];

        let effects = calculate_religious_effects(agent_pos, &traits, &buildings, 0);

        assert!(effects.is_empty());
    }

    #[test]
    fn test_distance_affects_strength() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Believer);

        let buildings = vec![
            (make_position(0, 0), BuildingType::Shrine, true),
        ];

        // Agent right at the shrine
        let effects_close = calculate_religious_effects(
            make_position(0, 0), &traits, &buildings, 0
        );
        let happiness_close = total_happiness_modifier(&effects_close);

        // Agent 5 tiles away
        let effects_mid = calculate_religious_effects(
            make_position(5, 0), &traits, &buildings, 0
        );
        let happiness_mid = total_happiness_modifier(&effects_mid);

        // Agent 9 tiles away (just within range)
        let effects_far = calculate_religious_effects(
            make_position(9, 0), &traits, &buildings, 0
        );
        let happiness_far = total_happiness_modifier(&effects_far);

        assert!(happiness_close > happiness_mid);
        assert!(happiness_mid > happiness_far);
    }

    #[test]
    fn test_temple_stronger_than_shrine() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Believer);

        let agent_pos = make_position(0, 0);

        let shrine_effects = calculate_religious_effects(
            agent_pos,
            &traits,
            &[(make_position(0, 0), BuildingType::Shrine, true)],
            0,
        );

        let temple_effects = calculate_religious_effects(
            agent_pos,
            &traits,
            &[(make_position(0, 0), BuildingType::Temple, true)],
            0,
        );

        let shrine_happiness = total_happiness_modifier(&shrine_effects);
        let temple_happiness = total_happiness_modifier(&temple_effects);

        assert!(temple_happiness > shrine_happiness);
    }

    #[test]
    fn test_incomplete_buildings_no_effect() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Believer);

        let agent_pos = make_position(0, 0);
        let buildings = vec![
            (make_position(0, 0), BuildingType::Temple, false), // Not completed
        ];

        let effects = calculate_religious_effects(agent_pos, &traits, &buildings, 0);

        assert!(effects.is_empty());
    }

    #[test]
    fn test_should_seek_religious_building() {
        let mut believer_traits = TraitSet::new();
        believer_traits.add_trait(Trait::Believer);

        let mut zealot_traits = TraitSet::new();
        zealot_traits.add_trait(Trait::Zealot);

        // Believer with low happiness should seek
        assert!(should_seek_religious_building(&believer_traits, 0.4));
        // Believer with high happiness shouldn't
        assert!(!should_seek_religious_building(&believer_traits, 0.8));

        // Zealot seeks more aggressively
        assert!(should_seek_religious_building(&zealot_traits, 0.7));
    }

    #[test]
    fn test_atheist_scriptorium_bonus() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Atheist);

        let bonus = secular_knowledge_bonus(BuildingType::Scriptorium, &traits);
        assert!(bonus > 0.0);

        // Non-atheist gets no bonus
        let normal_traits = TraitSet::new();
        let no_bonus = secular_knowledge_bonus(BuildingType::Scriptorium, &normal_traits);
        assert_eq!(no_bonus, 0.0);
    }
}
