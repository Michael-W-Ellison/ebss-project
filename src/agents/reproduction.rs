// src/agents/reproduction.rs
//! Reproduction and genetic inheritance: pairing, pregnancy, and nursing.

use rand::Rng;
use uuid::Uuid;
use crate::agents::{Agent, AgentConfig};
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
    /// Mating succeeded; `mother_id` is whoever came away carrying
    PregnancyStarted { mother_id: Uuid, father_id: Uuid },
    /// Mating failed due to infertility or chance
    Failed(String),
}

/// Check if two agents can mate
///
/// Requirements:
/// - Two different agents
/// - Neither already carrying a pregnancy
/// - Both must be capable of reproduction AND have their survival needs met
/// - Agents that are hungry or thirsty will not attempt reproduction
pub fn can_mate(agent1: &Agent, agent2: &Agent, criteria: &MateSelectionCriteria) -> bool {
    // Both must be alive, able to reproduce, AND have survival needs met
    if !agent1.should_attempt_reproduction() || !agent2.should_attempt_reproduction() {
        return false;
    }

    // Two different people, and neither of them already carrying.
    //
    // "Agents are gender neutral. There are no male/female agents, merely
    // child and adult agents." This required one of each, so a pair drawn
    // from a settlement had about an even chance of being refused before
    // anything else was asked - in a model that manages two births in 308,000
    // turns of action, half of every candidate pair was thrown away on a
    // distinction the specification does not have.
    if agent1.id == agent2.id {
        return false;
    }

    if agent1.is_pregnant() || agent2.is_pregnant() {
        return false;
    }

    // Check fertility levels
    if agent1.fertility() < criteria.min_fertility
        || agent2.fertility() < criteria.min_fertility
    {
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



/// Whether a pairing takes, and the pregnancy it leaves on the one carrying.
///
/// The names were `male` and `female`; there is no gender in this model, so
/// they are `carrier` - whichever of the two will be carrying it - and
/// `other`. Which of a pair carries is the caller's to decide and is
/// deliberately not a property of either of them.
pub fn attempt_impregnation(
    carrier: &Agent,
    other: &Agent,
    current_tick: u32,
) -> Option<PregnancyState> {
    let mut rng = crate::core::dice::roll();

    // Check basic requirements
    if !carrier.can_carry_a_child() || !other.can_reproduce() {
        return None;
    }

    // Calculate conception probability based on fertility.
    //
    // Clamped because this is handed straight to the sampler, which panics on
    // anything outside 0.0 to 1.0 rather than saturating.
    let conception_chance = (carrier.fertility() * other.fertility()).clamp(0.0, 1.0);

    if rng.gen_bool(conception_chance as f64) {
        Some(PregnancyState::new(current_tick, other.id))
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
    // Whichever of them is carrying it is the one whose nutrition it grew on.
    // Either may be - there is no gender in this model, so the carrier is
    // simply the one with a pregnancy on them.
    let prenatal_nutrition = parent1
        .pregnancy
        .as_ref()
        .or(parent2.pregnancy.as_ref())
        .map(|p| p.nutrition_quality)
        .unwrap_or(0.8);

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

    // Inherit traits from parents (mix of both with mutation).
    //
    // What the child was born with rather than born to survives this: the
    // congenital rolls happen in `with_parents`, before we get here, and
    // assigning straight over the top used to throw them away - so congenital
    // infertility, the one trait anything ever assigned, never once survived a
    // live birth.
    let born_with: Vec<crate::core::traits::Trait> =
        offspring.traits.get_traits().iter().copied()
            .filter(|t| matches!(t, crate::core::traits::Trait::Infertile))
            .collect();

    offspring.traits = inherit_traits(&parent1.traits, &parent2.traits);

    for born_with in born_with {
        offspring.traits.add_trait(born_with);
    }

    // And what the child is wary of, which it cannot have earned yet. Only
    // the worries pass, not the trails - a child inherits its parents' fears
    // and not their map. See `Patterns::what_the_child_takes_from`.
    offspring
        .patterns
        .what_the_child_takes_from(&[&parent1.patterns, &parent2.patterns]);

    // A child's own personality bends its own drives. This has to come after
    // both the drives and the traits are settled - the weights are inherited
    // above and the traits just now - and it is written to be safe to repeat,
    // so ordering here is a matter of correctness rather than luck.
    offspring.drives.lean_towards(&offspring.traits);

    // Traits are assigned after construction, so let the inherited ones reach
    // the senses: a child born blind or deaf must actually be so
    offspring.apply_trait_sensory_modifications();

    // Inherit reproduction drive modifier from parents with mutation
    offspring.reproduction_drive_modifier = inherit_reproduction_modifier(
        parent1.reproduction_drive_modifier,
        parent2.reproduction_drive_modifier,
    );

    // Start with neutral emotions
    offspring.emotions = crate::agents::EmotionState::default();

    // Generate random preferences
    offspring.preferences = crate::core::Preferences::default();

    // Placed beside whoever carried it, which is whichever of them has the
    // pregnancy on them. Falling back to the first parent rather than to a
    // gender, because there is no gender to fall back to.
    let mother_pos = if parent1.pregnancy.is_some() {
        parent1.state.position
    } else if parent2.pregnancy.is_some() {
        parent2.state.position
    } else {
        parent1.state.position
    };
    // Born where its mother is standing, and not a pace to the left.
    //
    // The pace to the left had no idea where the edge of the world was.
    // A mother standing on the first column put one child in three at
    // x = -1, which is off the map: `is_passable_tile` refuses every tile
    // outside the grid, so such a child had **no passable neighbour in any
    // direction** and could never take a step again. It could not walk to
    // food or to water, and it starved where it lay, returning `Move: No
    // passable route toward destination` every turn of its short life.
    // Measured over eight seeded world-years that refusal was 63,922 - **half
    // of every refusal left in the model** - and every single one of them
    // reported "standing off the map, with 0 ways out".
    //
    // Nothing here knows how big the world is, and it does not need to: a
    // baby is born where its mother is. Whatever moves it afterwards is
    // bounds-checked already.
    offspring.state.position = mother_pos;

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
    let mut rng = crate::core::dice::roll();

    // Average parent modifiers
    let base = (parent1_mod + parent2_mod) / 2.0;

    // Add mutation: ±30% variation
    let mutation = rng.gen_range(-0.3..0.3);
    (base * (1.0 + mutation)).clamp(0.3, 1.8)
}

/// Inherit drives from two parents with genetic variation
fn inherit_drives(drives1: &DriveState, drives2: &DriveState) -> DriveState {
    let mut rng = crate::core::dice::roll();
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
    let mut rng = crate::core::dice::roll();
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
    let mut rng = crate::core::dice::roll();

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
        Trait::Repressed, Trait::Mute, Trait::Deaf, Trait::Blind, Trait::Ignorant,
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
    let mut rng = crate::core::dice::roll();

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

    /// Two grown people who could pair, and nothing else to say about them.
    ///
    /// The two are interchangeable. Which of them carries a child is decided
    /// by whoever calls `attempt_impregnation` - the first argument is the
    /// carrier - and not by anything either of them is.
    fn create_mating_pair() -> (Agent, Agent) {
        use crate::core::DriveType;

        let mut other = Agent::new(AgentConfig::default());
        let mut carrier = Agent::new(AgentConfig::default());

        other.state.now_this_many_years_old(30);
        carrier.state.now_this_many_years_old(30);

        // Set positions close together
        other.state.position = (0, 0, 0);
        carrier.state.position = (10, 10, 0);

        // Ensure both are well-fed (low survival drives)
        for agent in [&mut other, &mut carrier] {
            if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
                hunger.value = 0.2;
            }
            if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                thirst.value = 0.2;
            }

            // Traits and the personal reproduction modifier are rolled at
            // random on creation, and an infertile or low-drive pair cannot
            // mate whatever else the test sets up. Pin both so these tests
            // assert on the behaviour they name instead of on the dice.
            agent
                .traits
                .traits
                .retain(|t| *t != crate::core::traits::Trait::Infertile);
            agent.reproduction_drive_modifier = 1.0;
            if let Some(reproduction) = agent.drives.get_mut(DriveType::Reproduction) {
                reproduction.value = 0.5;
            }
        }

        with_a_full_larder(&mut other);
        with_a_full_larder(&mut carrier);

        (other, carrier)
    }

    /// Fertility is handed to the sampler as a probability, so it has to keep
    /// to its documented range.
    ///
    /// The personal reproduction modifier goes as high as 1.8 and the
    /// developmental one to 1.1, so an unclamped agent in its prime multiplied
    /// out to nearly 2.0.

    /// Give an agent food in hand, which reproduction now requires: being
    /// un-hungry for a moment says nothing about whether the next meal exists.
    /// A camp with a lean season's eating in the ground for two.
    ///
    /// Breeding waits on a real surplus now, and a surplus is the camp's
    /// stores rather than anything a person carries - a pack holds twelve
    /// weight, which is about two days' food. Tests here are about *pairing*,
    /// so they stock the larder and let the food gate pass; the gate itself is
    /// tested in `a_child_waits_on_a_surplus_and_not_on_a_full_stomach`.
    fn with_a_full_larder(agent: &mut Agent) {
        let a_day = agent.state.physiology.what_i_burn_in_a_day;
        let gap = crate::agents::provision::how_long_the_land_gives_nothing() as f32;
        agent.state.what_the_larder_says = Some(
            crate::agents::provision::WhatIsPutBy::reckon(a_day * 2.0 * gap, a_day, 90.0, 0),
        );
    }

    fn give_food(agent: &mut Agent, quantity: u32) {
        use crate::agents::InventoryItem;
        use crate::world::nutrition::FoodDatabase;
        use crate::world::ItemType;

        let database = FoodDatabase::new();
        let mut item = InventoryItem::new_with_weight("food".to_string(), quantity, 0.1);
        item.food_data = database.create_food_data(&ItemType::Food, 0);
        agent.inventory.add_item(item);
    }

    #[test]
    fn test_fertility_stays_within_probability_range() {
        let mut agent = Agent::new(AgentConfig::default());

        agent.state.age = 3000;
        agent.state.life_stage = crate::agents::LifeStage::Adult;
        agent.state.health = 100.0;
        agent
            .traits
            .traits
            .retain(|t| *t != crate::core::traits::Trait::Infertile);

        // Every multiplier at its most generous
        agent.reproduction_drive_modifier = 1.8;
        agent.developmental_nutrition.finalized = true;
        agent.developmental_nutrition.stat_modifiers.fertility = 1.1;
        if let Some(drive) = agent.drives.get_mut(DriveType::Reproduction) {
            drive.value = 1.0;
        }

        let fertility = agent.fertility();

        assert!(
            (0.0..=1.0).contains(&fertility),
            "fertility should stay a probability, got {fertility}"
        );
    }

    /// Two unusually fertile agents must not crash the conception roll.
    ///
    /// Their fertilities multiplied to roughly 4.0, and the sampler panics on
    /// anything outside 0.0 to 1.0 rather than saturating. Reproduction only
    /// started running once agents could keep themselves fed and watered, so
    /// this surfaced as a rare crash a few thousand ticks into a run.
    #[test]
    fn test_impregnation_survives_maximum_fertility() {
        let (mut other, mut carrier) = create_mating_pair();

        for agent in [&mut other, &mut carrier] {
            agent.state.health = 100.0;
            agent.reproduction_drive_modifier = 1.8;
            agent.developmental_nutrition.finalized = true;
            agent.developmental_nutrition.stat_modifiers.fertility = 1.1;
            if let Some(drive) = agent.drives.get_mut(DriveType::Reproduction) {
                drive.value = 1.0;
            }
        }

        assert!(
            other.fertility() * carrier.fertility() <= 1.0,
            "the conception roll needs a probability, got {}",
            other.fertility() * carrier.fertility()
        );

        // Would panic outright before the clamp
        let _ = attempt_impregnation(&other, &carrier, 100);
    }

    #[test]
    fn test_can_mate_basic() {
        let (mut other, mut carrier) = create_mating_pair();
        give_food(&mut other, 12);
        give_food(&mut carrier, 12);

        let criteria = MateSelectionCriteria::default();
        assert!(can_mate(&other, &carrier, &criteria));
    }

    /// A pair with nothing put by will not have a child, however full they are
    /// at this moment.
    #[test]
    fn a_pair_with_nothing_put_by_do_not_have_a_child() {
        let (mut other, mut carrier) = create_mating_pair();

        // Nothing put by: no pack, and an empty camp. This is the one test
        // here that is about the food gate, so it takes back what
        // `create_mating_pair` stocked.
        for agent in [&mut other, &mut carrier] {
            agent.state.what_the_larder_says = None;
        }

        let criteria = MateSelectionCriteria::default();
        assert!(
            !can_mate(&other, &carrier, &criteria),
            "an empty pack is not a plan for feeding a child"
        );
    }

    /// Any two grown people can pair.
    ///
    /// This was `test_cannot_mate_same_gender` and it asserted the opposite:
    /// two males or two females were refused. "Agents are gender neutral.
    /// There are no male/female agents, merely child and adult agents", so
    /// there is nothing to refuse them for - and refusing them threw away
    /// about half of every candidate pairing in a settlement that manages two
    /// births in 308,000 turns.
    #[test]
    fn any_two_grown_people_can_pair() {
        // Seeded: `Agent::new` draws a random weight per drive, so how many
        // drives there are decides what personality these two get, and the
        // pairing rules read personality. See ISSUES_FOUND.md #132.
        crate::core::dice::seed(4_300);

        let mut agent1 = Agent::new(AgentConfig::default());
        let mut agent2 = Agent::new(AgentConfig::default());

        agent1.state.now_this_many_years_old(30);
        agent2.state.now_this_many_years_old(30);
        agent1.state.position = (0, 0, 0);
        agent2.state.position = (10, 10, 0);

        with_a_full_larder(&mut agent1);
        with_a_full_larder(&mut agent2);

        let criteria = MateSelectionCriteria::default();
        assert!(
            can_mate(&agent1, &agent2, &criteria),
            "two grown people, and nothing else to say about them"
        );
    }

    /// But not two of the same person.
    #[test]
    fn nobody_pairs_with_themselves() {
        let mut agent = Agent::new(AgentConfig::default());
        agent.state.now_this_many_years_old(30);

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&agent, &agent, &criteria));
    }

    /// And nobody already carrying one starts another.
    #[test]
    fn nobody_already_carrying_starts_another() {
        // Seeded: `Agent::new` draws a random weight per drive, so how many
        // drives there are decides what personality these two get, and the
        // pairing rules read personality. See ISSUES_FOUND.md #132.
        crate::core::dice::seed(4_300);

        let mut agent1 = Agent::new(AgentConfig::default());
        let mut agent2 = Agent::new(AgentConfig::default());
        agent1.state.now_this_many_years_old(30);
        agent2.state.now_this_many_years_old(30);

        with_a_full_larder(&mut agent1);
        with_a_full_larder(&mut agent2);

        let criteria = MateSelectionCriteria::default();
        assert!(can_mate(&agent1, &agent2, &criteria));

        agent1.pregnancy = Some(PregnancyState::new(0, agent2.id));
        assert!(
            !can_mate(&agent1, &agent2, &criteria),
            "and it does not matter which of the two it is"
        );

        agent1.pregnancy = None;
        agent2.pregnancy = Some(PregnancyState::new(0, agent1.id));
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
        // In years. `Agent::new` makes a grown person now, so an infant has to
        // say so - it used to be one by default, which is the trap that fix
        // removed.
        infant.state.now_this_many_years_old(2);

        let mut adult = Agent::new(AgentConfig::default());
        adult.state.now_this_many_years_old(30);
        

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&infant, &adult, &criteria));
    }

    #[test]
    fn test_cannot_mate_when_pregnant() {
        let (other, mut carrier) = create_mating_pair();

        // Female is pregnant
        carrier.pregnancy = Some(PregnancyState::new(0, other.id));

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&other, &carrier, &criteria));
    }

    #[test]
    fn test_reproduce_creates_offspring() {
        let mut parent1 = Agent::new(AgentConfig::default());
        let mut parent2 = Agent::new(AgentConfig::default());

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

        let (mut other, carrier) = create_mating_pair();

        // Set other as hungry (drive active)
        if let Some(hunger) = other.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.9; // Above threshold (0.7)
        }

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - other is hungry
        assert!(!can_mate(&other, &carrier, &criteria));
    }

    #[test]
    fn test_cannot_mate_when_thirsty() {
        use crate::core::DriveType;

        let (other, mut carrier) = create_mating_pair();

        // Set carrier as thirsty (drive active)
        if let Some(thirst) = carrier.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.9; // Above threshold (0.75)
        }

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - carrier is thirsty
        assert!(!can_mate(&other, &carrier, &criteria));
    }

    #[test]
    fn test_can_mate_when_well_fed() {
        use crate::core::DriveType;

        let (mut other, mut carrier) = create_mating_pair();

        // Ensure both are well-fed (low hunger/thirst)
        if let Some(hunger) = other.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.2; // Well below threshold
        }
        if let Some(thirst) = other.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.2;
        }
        if let Some(hunger) = carrier.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.2;
        }
        if let Some(thirst) = carrier.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.2;
        }

        give_food(&mut other, 12);
        give_food(&mut carrier, 12);

        let criteria = MateSelectionCriteria::default();
        // Should be able to mate - both are well-fed and have food in hand
        assert!(can_mate(&other, &carrier, &criteria));
    }

    #[test]
    fn test_impregnation() {
        // The first argument is the one carrying it and the second is the
        // other parent, which is what the pregnancy records. These were
        // `other` and `carrier`, and the assertion below read the first as the
        // father - there is no gender in this model and which of a pair
        // carries is the caller's to decide, so the names had to go.
        let (carrier, other) = create_mating_pair();

        // Try multiple times since it's probabilistic
        let mut success = false;
        for _ in 0..100 {
            if let Some(pregnancy) = attempt_impregnation(&carrier, &other, 100) {
                assert_eq!(pregnancy.father_id, other.id);
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
        // Years, not ticks - 3,000 ticks is most of one year, and a year is
        // 4,320. The life stage was then set by hand to paper over it.
        agent.state.now_this_many_years_old(30);
        with_a_full_larder(&mut agent);

        // Fed, watered, and a lean season in the ground: should attempt
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

        let (mut other, carrier) = create_mating_pair();

        // Make other infertile
        other.traits.add_trait(Trait::Infertile);

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - other is infertile
        assert!(!can_mate(&other, &carrier, &criteria));
        assert!(!other.can_reproduce());
        assert!(other.is_infertile());
    }

    #[test]
    fn test_infertile_female_cannot_mate() {
        use crate::core::traits::Trait;

        let (other, mut carrier) = create_mating_pair();

        // Make carrier infertile
        carrier.traits.add_trait(Trait::Infertile);

        let criteria = MateSelectionCriteria::default();
        // Should NOT be able to mate - carrier is infertile
        assert!(!can_mate(&other, &carrier, &criteria));
        assert!(!carrier.can_reproduce());
        assert!(carrier.is_infertile());
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
