// src/agents/reproduction.rs
//! Reproduction and genetic inheritance system with gender, pregnancy, and nursing.

use rand::Rng;
use uuid::Uuid;
use crate::agents::{Agent, AgentConfig};
use crate::agents::gender::Gender;
use crate::agents::pregnancy::PregnancyState;
use crate::core::{DriveState, DriveType, BehaviorTree};

/// Mate selection criteria
#[derive(Debug, Clone)]
pub struct MateSelectionCriteria {
    /// Minimum distance for mate selection
    pub min_distance: f32,
    /// Maximum distance for mate selection
    pub max_distance: f32,
    /// Minimum fertility for reproduction
    pub min_fertility: f32,
}

impl Default for MateSelectionCriteria {
    fn default() -> Self {
        Self {
            min_distance: 0.0,
            max_distance: 50.0,
            min_fertility: 0.3,
        }
    }
}

/// Result of a mating attempt
#[derive(Debug)]
pub enum MatingResult {
    /// Mating successful, female is now pregnant
    PregnancyStarted { mother_id: Uuid, father_id: Uuid },
    /// Mating failed due to infertility or chance
    Failed(String),
}

/// Check if two agents can mate
///
/// Requirements:
/// - One must be male, one must be female
/// - Female must not already be pregnant
/// - Both must be capable of reproduction AND have their survival needs met
/// - Agents that are hungry or thirsty will not attempt reproduction
pub fn can_mate(agent1: &Agent, agent2: &Agent, criteria: &MateSelectionCriteria) -> bool {
    // Both must be alive, able to reproduce, AND have survival needs met
    if !agent1.should_attempt_reproduction() || !agent2.should_attempt_reproduction() {
        return false;
    }

    // One must be male, one must be female
    let (male, female) = match (agent1.gender, agent2.gender) {
        (Gender::Male, Gender::Female) => (agent1, agent2),
        (Gender::Female, Gender::Male) => (agent2, agent1),
        _ => return false, // Same gender cannot mate
    };

    // Female must not be pregnant
    if female.is_pregnant() {
        return false;
    }

    // Check fertility levels
    if male.fertility() < criteria.min_fertility || female.fertility() < criteria.min_fertility {
        return false;
    }

    // Check distance
    let distance = calculate_distance(agent1.state.position, agent2.state.position);
    if distance < criteria.min_distance || distance > criteria.max_distance {
        return false;
    }

    // Cannot mate with self or direct relatives (parents)
    if agent1.id == agent2.id {
        return false;
    }

    // Check if they are parent-child
    if agent1.parent_ids.contains(&agent2.id) || agent2.parent_ids.contains(&agent1.id) {
        return false;
    }

    true
}

/// Get the female agent from a mating pair (returns None if no valid pair)
pub fn get_female<'a>(agent1: &'a Agent, agent2: &'a Agent) -> Option<&'a Agent> {
    match (agent1.gender, agent2.gender) {
        (Gender::Female, Gender::Male) => Some(agent1),
        (Gender::Male, Gender::Female) => Some(agent2),
        _ => None,
    }
}

/// Get the male agent from a mating pair (returns None if no valid pair)
pub fn get_male<'a>(agent1: &'a Agent, agent2: &'a Agent) -> Option<&'a Agent> {
    match (agent1.gender, agent2.gender) {
        (Gender::Male, Gender::Female) => Some(agent1),
        (Gender::Female, Gender::Male) => Some(agent2),
        _ => None,
    }
}

/// Attempt to impregnate the female agent
/// Returns the pregnancy state if successful
pub fn attempt_impregnation(
    male: &Agent,
    female: &Agent,
    current_tick: u32,
) -> Option<PregnancyState> {
    let mut rng = rand::thread_rng();

    // Check basic requirements
    if !male.can_impregnate() || !female.can_become_pregnant() {
        return None;
    }

    // Calculate conception probability based on fertility
    let conception_chance = male.fertility() * female.fertility();

    if rng.gen_bool(conception_chance as f64) {
        Some(PregnancyState::new(current_tick, male.id))
    } else {
        None
    }
}

/// Calculate Euclidean distance between two positions
fn calculate_distance(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> f32 {
    let dx = (pos1.0 - pos2.0) as f32;
    let dy = (pos1.1 - pos2.1) as f32;
    let dz = (pos1.2 - pos2.2) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Create offspring from two parent agents (used for immediate birth in legacy code)
pub fn reproduce(parent1: &Agent, parent2: &Agent, current_tick: u32) -> Agent {
    // Determine which parent is mother for prenatal nutrition
    let prenatal_nutrition = match (parent1.gender, parent2.gender) {
        (Gender::Female, _) => parent1.pregnancy.as_ref()
            .map(|p| p.nutrition_quality)
            .unwrap_or(0.8),
        (_, Gender::Female) => parent2.pregnancy.as_ref()
            .map(|p| p.nutrition_quality)
            .unwrap_or(0.8),
        _ => 0.8, // Default if no clear mother
    };

    give_birth_internal(parent1, parent2, current_tick, prenatal_nutrition)
}

/// Create offspring when pregnancy reaches term
/// This is the proper way to create offspring - from a completed pregnancy
pub fn give_birth(
    mother: &Agent,
    father: &Agent,
    pregnancy: &PregnancyState,
    current_tick: u32,
) -> Agent {
    give_birth_internal(mother, father, current_tick, pregnancy.nutrition_quality)
}

/// Internal function to create offspring with specified prenatal nutrition
fn give_birth_internal(
    parent1: &Agent,
    parent2: &Agent,
    current_tick: u32,
    prenatal_nutrition: f32,
) -> Agent {
    let parent_ids = vec![parent1.id, parent2.id];

    // Create offspring with inherited traits and prenatal nutrition data
    let mut offspring = Agent::with_parents_and_prenatal(
        AgentConfig { random_weights: false },
        parent_ids,
        current_tick,
        prenatal_nutrition,
    );

    // Inherit drives from parents with mutation
    offspring.drives = inherit_drives(&parent1.drives, &parent2.drives);

    // Inherit behavior trees from parents with pruning and mutation
    offspring.behavior_trees = inherit_behavior_trees(&parent1.behavior_trees, &parent2.behavior_trees);

    // Inherit traits from parents (mix of both with mutation)
    offspring.traits = inherit_traits(&parent1.traits, &parent2.traits);

    // Inherit reproduction drive modifier from parents with mutation
    offspring.reproduction_drive_modifier = inherit_reproduction_modifier(
        parent1.reproduction_drive_modifier,
        parent2.reproduction_drive_modifier,
    );

    // Start with neutral emotions
    offspring.emotions = crate::agents::EmotionState::default();

    // Generate random preferences
    offspring.preferences = crate::core::Preferences::default();

    // Place offspring near mother (parent1 if female, otherwise parent2)
    let mother_pos = if parent1.gender == Gender::Female {
        parent1.state.position
    } else {
        parent2.state.position
    };
    offspring.state.position = (
        mother_pos.0 + rand::thread_rng().gen_range(-1..=1),
        mother_pos.1 + rand::thread_rng().gen_range(-1..=1),
        mother_pos.2,
    );

    // Establish family relationships
    use crate::agents::emotions::{Relationship, RelationshipType};
    offspring.relationships.add_relationship(
        Relationship::new(parent1.id, RelationshipType::Parent)
    );
    offspring.relationships.add_relationship(
        Relationship::new(parent2.id, RelationshipType::Parent)
    );

    offspring
}

/// Inherit reproduction drive modifier from parents with mutation
fn inherit_reproduction_modifier(parent1_mod: f32, parent2_mod: f32) -> f32 {
    let mut rng = rand::thread_rng();

    // Average parent modifiers
    let base = (parent1_mod + parent2_mod) / 2.0;

    // Add mutation: ±30% variation
    let mutation = rng.gen_range(-0.3..0.3);
    (base * (1.0 + mutation)).clamp(0.3, 1.8)
}

/// Inherit drives from two parents with genetic variation
fn inherit_drives(drives1: &DriveState, drives2: &DriveState) -> DriveState {
    let mut rng = rand::thread_rng();
    let mut new_drives = DriveState::new();

    for drive_type in DriveType::all().iter() {
        let parent1_drive = drives1.get(*drive_type).unwrap();
        let parent2_drive = drives2.get(*drive_type).unwrap();

        // Average parent weights with variation
        let base_weight = (parent1_drive.weight + parent2_drive.weight) / 2.0;

        // Add mutation: ±20% variation
        let mutation = rng.gen_range(-0.2..0.2);
        let mutated_weight = (base_weight * (1.0 + mutation)).clamp(0.3, 3.0);

        if let Some(offspring_drive) = new_drives.get_mut(*drive_type) {
            offspring_drive.weight = mutated_weight;
        }
    }

    new_drives
}

/// Inherit behavior trees from two parents
fn inherit_behavior_trees(trees1: &[BehaviorTree], trees2: &[BehaviorTree]) -> Vec<BehaviorTree> {
    let mut rng = rand::thread_rng();
    let mut offspring_trees = Vec::new();

    // Take a mix of trees from both parents
    for tree in trees1 {
        if rng.gen_bool(0.5) {
            // Clone with pruning (remove low-weight branches)
            offspring_trees.push(tree.clone_with_pruning(0.3));
        }
    }

    for tree in trees2 {
        if rng.gen_bool(0.5) {
            offspring_trees.push(tree.clone_with_pruning(0.3));
        }
    }

    offspring_trees
}

/// Inherit traits from two parents with 10% mutation chance
/// Each inherited trait has a 10% chance of being replaced by a completely new trait
fn inherit_traits(traits1: &crate::agents::TraitSet, traits2: &crate::agents::TraitSet) -> crate::agents::TraitSet {
    use crate::agents::Trait;
    use rand::seq::SliceRandom;
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let mut offspring_traits = crate::agents::TraitSet::new();

    // All available traits for mutation
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
        Trait::Allergic, Trait::Explorer, Trait::Caretaker, Trait::Aggressive,
        Trait::Peaceful, Trait::Trusting, Trait::Honest, Trait::Dishonest,
        Trait::Callous, Trait::Diligent, Trait::Manipulator, Trait::Imaginative,
        Trait::Paranoid, Trait::Archivist, Trait::Masochist, Trait::Copycat,
        Trait::Repressed, Trait::Mute, Trait::Deaf, Trait::Ignorant,
    ];

    // Collect all parent traits
    let parent1_traits: Vec<_> = traits1.get_traits().iter().copied().collect();
    let parent2_traits: Vec<_> = traits2.get_traits().iter().copied().collect();

    // Inherit traits from parent 1 (50% chance for each trait)
    for &trait_item in &parent1_traits {
        if rng.gen_bool(0.5) {
            // 10% chance: mutate to a completely new trait instead
            if rng.gen_bool(0.10) {
                // Pick a random trait that isn't from either parent
                let new_trait = all_traits.choose(&mut rng).copied();
                if let Some(t) = new_trait {
                    offspring_traits.add_trait(t);
                }
            } else {
                // Normal inheritance
                offspring_traits.add_trait(trait_item);
            }
        }
    }

    // Inherit traits from parent 2 (50% chance for each trait)
    for &trait_item in &parent2_traits {
        if rng.gen_bool(0.5) {
            // 10% chance: mutate to a completely new trait instead
            if rng.gen_bool(0.10) {
                // Pick a random trait that isn't from either parent
                let new_trait = all_traits.choose(&mut rng).copied();
                if let Some(t) = new_trait {
                    offspring_traits.add_trait(t);
                }
            } else {
                // Normal inheritance
                offspring_traits.add_trait(trait_item);
            }
        }
    }

    offspring_traits
}

/// Calculate offspring position (near parents)
fn offspring_position(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> (i32, i32, i32) {
    let mut rng = rand::thread_rng();

    // Average parent positions
    let avg_x = (pos1.0 + pos2.0) / 2;
    let avg_y = (pos1.1 + pos2.1) / 2;
    let avg_z = (pos1.2 + pos2.2) / 2;

    // Add small random offset
    let offset_x = rng.gen_range(-2..=2);
    let offset_y = rng.gen_range(-2..=2);
    let offset_z = rng.gen_range(-1..=1);

    (avg_x + offset_x, avg_y + offset_y, avg_z + offset_z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentConfig;

    /// Helper to create a mating-ready agent pair (male and female)
    fn create_mating_pair() -> (Agent, Agent) {
        use crate::core::DriveType;

        let mut male = Agent::new(AgentConfig::default());
        let mut female = Agent::new(AgentConfig::default());

        // Set genders
        male.gender = Gender::Male;
        female.gender = Gender::Female;

        // Set to adult stage
        male.state.age = 3000;
        male.state.life_stage = crate::agents::LifeStage::Adult;
        female.state.age = 3000;
        female.state.life_stage = crate::agents::LifeStage::Adult;

        // Set positions close together
        male.state.position = (0, 0, 0);
        female.state.position = (10, 10, 0);

        // Ensure both are well-fed (low survival drives)
        for agent in [&mut male, &mut female] {
            if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
                hunger.value = 0.2;
            }
            if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                thirst.value = 0.2;
            }
        }

        (male, female)
    }

    #[test]
    fn test_can_mate_basic() {
        let (male, female) = create_mating_pair();

        let criteria = MateSelectionCriteria::default();
        assert!(can_mate(&male, &female, &criteria));
    }

    #[test]
    fn test_cannot_mate_same_gender() {
        let mut agent1 = Agent::new(AgentConfig::default());
        let mut agent2 = Agent::new(AgentConfig::default());

        // Both male
        agent1.gender = Gender::Male;
        agent2.gender = Gender::Male;
        agent1.state.age = 3000;
        agent1.state.life_stage = crate::agents::LifeStage::Adult;
        agent2.state.age = 3000;
        agent2.state.life_stage = crate::agents::LifeStage::Adult;
        agent1.state.position = (0, 0, 0);
        agent2.state.position = (10, 10, 0);

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&agent1, &agent2, &criteria));

        // Both female
        agent1.gender = Gender::Female;
        agent2.gender = Gender::Female;
        assert!(!can_mate(&agent1, &agent2, &criteria));
    }

    #[test]
    fn test_cannot_mate_with_self() {
        let mut agent = Agent::new(AgentConfig::default());
        agent.state.age = 3000;
        agent.state.life_stage = crate::agents::LifeStage::Adult;

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&agent, &agent, &criteria));
    }

    #[test]
    fn test_cannot_mate_infant() {
        let mut infant = Agent::new(AgentConfig::default());
        infant.gender = Gender::Male;

        let mut adult = Agent::new(AgentConfig::default());
        adult.gender = Gender::Female;
        adult.state.age = 3000;
        adult.state.life_stage = crate::agents::LifeStage::Adult;

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&infant, &adult, &criteria));
    }

    #[test]
    fn test_cannot_mate_when_pregnant() {
        let (male, mut female) = create_mating_pair();

        // Female is pregnant
        female.pregnancy = Some(PregnancyState::new(0, male.id));

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&male, &female, &criteria));
    }

    #[test]
    fn test_reproduce_creates_offspring() {
        let mut parent1 = Agent::new(AgentConfig::default());
        let mut parent2 = Agent::new(AgentConfig::default());

        parent1.gender = Gender::Male;
        parent2.gender = Gender::Female;
        parent1.state.age = 3000;
        parent1.state.life_stage = crate::agents::LifeStage::Adult;
        parent2.state.age = 3000;
        parent2.state.life_stage = crate::agents::LifeStage::Adult;

        let offspring = reproduce(&parent1, &parent2, 100);

        assert_eq!(offspring.parent_ids.len(), 2);
        assert!(offspring.parent_ids.contains(&parent1.id));
        assert!(offspring.parent_ids.contains(&parent2.id));
        assert_eq!(offspring.state.age, 0);
        assert!(offspring.nursing.is_some()); // Newborn should have nursing state
    }

    #[test]
    fn test_distance_calculation() {
        let pos1 = (0, 0, 0);
        let pos2 = (3, 4, 0);
        let distance = calculate_distance(pos1, pos2);
        assert!((distance - 5.0).abs() < 0.001); // 3-4-5 triangle
    }

    #[test]
    fn test_cannot_mate_when_hungry() {
        use crate::core::DriveType;

        let (mut male, female) = create_mating_pair();

        // Set male as hungry (drive active)
        if let Some(hunger) = male.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.9; // Above threshold (0.7)
        }

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - male is hungry
        assert!(!can_mate(&male, &female, &criteria));
    }

    #[test]
    fn test_cannot_mate_when_thirsty() {
        use crate::core::DriveType;

        let (male, mut female) = create_mating_pair();

        // Set female as thirsty (drive active)
        if let Some(thirst) = female.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.9; // Above threshold (0.75)
        }

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - female is thirsty
        assert!(!can_mate(&male, &female, &criteria));
    }

    #[test]
    fn test_can_mate_when_well_fed() {
        use crate::core::DriveType;

        let (mut male, mut female) = create_mating_pair();

        // Ensure both are well-fed (low hunger/thirst)
        if let Some(hunger) = male.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.2; // Well below threshold
        }
        if let Some(thirst) = male.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.2;
        }
        if let Some(hunger) = female.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.2;
        }
        if let Some(thirst) = female.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.2;
        }

        let criteria = MateSelectionCriteria::default();
        // Should be able to mate - both are well-fed
        assert!(can_mate(&male, &female, &criteria));
    }

    #[test]
    fn test_impregnation() {
        let (male, female) = create_mating_pair();

        // Try multiple times since it's probabilistic
        let mut success = false;
        for _ in 0..100 {
            if let Some(pregnancy) = attempt_impregnation(&male, &female, 100) {
                assert_eq!(pregnancy.father_id, male.id);
                assert_eq!(pregnancy.conception_tick, 100);
                success = true;
                break;
            }
        }
        assert!(success, "Impregnation should succeed at least once in 100 tries");
    }

    #[test]
    fn test_should_attempt_reproduction_respects_survival_drives() {
        use crate::core::DriveType;

        let mut agent = Agent::new(AgentConfig::default());
        agent.state.age = 3000;
        agent.state.life_stage = crate::agents::LifeStage::Adult;

        // Well-fed agent should attempt reproduction
        assert!(agent.should_attempt_reproduction());

        // Hungry agent should NOT attempt reproduction
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.8; // Above threshold
        }
        assert!(!agent.should_attempt_reproduction());

        // Reset hunger, make thirsty
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.1;
        }
        if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.9; // Above threshold
        }
        assert!(!agent.should_attempt_reproduction());
    }

    #[test]
    fn test_trait_inheritance_from_parents() {
        use crate::agents::Trait;

        // Create parents with specific traits
        let mut parent1 = Agent::new(AgentConfig::default());
        let mut parent2 = Agent::new(AgentConfig::default());

        parent1.state.age = 3000;
        parent1.state.life_stage = crate::agents::LifeStage::Adult;
        parent2.state.age = 3000;
        parent2.state.life_stage = crate::agents::LifeStage::Adult;

        // Give parent1 specific traits
        parent1.traits = crate::agents::TraitSet::new();
        parent1.traits.add_trait(Trait::Brave);
        parent1.traits.add_trait(Trait::Diligent);

        // Give parent2 different traits
        parent2.traits = crate::agents::TraitSet::new();
        parent2.traits.add_trait(Trait::Curious);
        parent2.traits.add_trait(Trait::Builder);

        // Reproduce multiple times to test inheritance patterns
        let mut inherited_parental_traits = 0;
        let mut total_traits = 0;

        for _ in 0..100 {
            let offspring = reproduce(&parent1, &parent2, 100);
            for trait_item in offspring.traits.get_traits() {
                total_traits += 1;
                // Check if trait came from either parent
                if parent1.traits.has(*trait_item) || parent2.traits.has(*trait_item) {
                    inherited_parental_traits += 1;
                }
            }
        }

        // Most traits should come from parents (accounting for 10% mutation rate)
        // With 10% mutation, ~90% should be parental traits
        let parental_ratio = inherited_parental_traits as f32 / total_traits as f32;
        assert!(
            parental_ratio > 0.7,
            "Expected most traits to be inherited from parents, got {}%",
            parental_ratio * 100.0
        );
    }

    #[test]
    fn test_trait_mutation_occurs() {
        use crate::agents::Trait;

        // Create parents with limited traits
        let mut parent1 = Agent::new(AgentConfig::default());
        let mut parent2 = Agent::new(AgentConfig::default());

        parent1.state.age = 3000;
        parent1.state.life_stage = crate::agents::LifeStage::Adult;
        parent2.state.age = 3000;
        parent2.state.life_stage = crate::agents::LifeStage::Adult;

        // Give parents just one trait each (non-overlapping)
        parent1.traits = crate::agents::TraitSet::new();
        parent1.traits.add_trait(Trait::Brave);

        parent2.traits = crate::agents::TraitSet::new();
        parent2.traits.add_trait(Trait::Curious);

        // Reproduce many times and check for mutations (new traits)
        let mut mutation_occurred = false;

        for _ in 0..200 {
            let offspring = reproduce(&parent1, &parent2, 100);
            for trait_item in offspring.traits.get_traits() {
                // A mutation is a trait not from either parent
                if !parent1.traits.has(*trait_item) && !parent2.traits.has(*trait_item) {
                    mutation_occurred = true;
                    break;
                }
            }
            if mutation_occurred {
                break;
            }
        }

        // With 10% mutation rate over 200 reproductions, we should see at least one mutation
        assert!(mutation_occurred, "Expected at least one trait mutation to occur over 200 reproductions");
    }

    #[test]
    fn test_infertile_cannot_mate() {
        use crate::core::traits::Trait;

        let (mut male, female) = create_mating_pair();

        // Make male infertile
        male.traits.add_trait(Trait::Infertile);

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - male is infertile
        assert!(!can_mate(&male, &female, &criteria));
        assert!(!male.can_reproduce());
        assert!(male.is_infertile());
    }

    #[test]
    fn test_infertile_female_cannot_mate() {
        use crate::core::traits::Trait;

        let (male, mut female) = create_mating_pair();

        // Make female infertile
        female.traits.add_trait(Trait::Infertile);

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - female is infertile
        assert!(!can_mate(&male, &female, &criteria));
        assert!(!female.can_reproduce());
        assert!(female.is_infertile());
    }

    #[test]
    fn test_severe_malnutrition_can_cause_infertility() {
        use crate::agents::childcare::DevelopmentalNutrition;

        // Run finalize many times with severe malnutrition - should eventually cause infertility
        let mut infertility_occurred = false;
        for _ in 0..100 {
            // Create severely malnourished development
            let mut dev = DevelopmentalNutrition::with_prenatal(0.05);
            // Simulate severe infant malnutrition
            for _ in 0..50 {
                dev.update_infant_nutrition(0.1, false); // Very poor nutrition, not nursed
            }
            // Simulate severe child malnutrition
            for _ in 0..50 {
                dev.update_child_nutrition(0.1, 30.0); // Poor nutrition, low health
            }

            if dev.finalize() {
                infertility_occurred = true;
                break;
            }
        }

        // With severe malnutrition, should eventually cause infertility
        assert!(infertility_occurred, "Severe malnutrition should eventually cause infertility");
    }
}
