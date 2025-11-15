// src/agents/reproduction.rs
//! Reproduction and genetic inheritance system.

use uuid::Uuid;
use rand::Rng;
use crate::agents::{Agent, AgentConfig};
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

/// Check if two agents can mate
pub fn can_mate(agent1: &Agent, agent2: &Agent, criteria: &MateSelectionCriteria) -> bool {
    // Both must be alive and able to reproduce
    if !agent1.can_reproduce() || !agent2.can_reproduce() {
        return false;
    }

    // Check fertility levels
    if agent1.fertility() < criteria.min_fertility || agent2.fertility() < criteria.min_fertility {
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

/// Calculate Euclidean distance between two positions
fn calculate_distance(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> f32 {
    let dx = (pos1.0 - pos2.0) as f32;
    let dy = (pos1.1 - pos2.1) as f32;
    let dz = (pos1.2 - pos2.2) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Create offspring from two parent agents
pub fn reproduce(parent1: &Agent, parent2: &Agent, current_tick: u32) -> Agent {
    let parent_ids = vec![parent1.id, parent2.id];

    // Create offspring with inherited traits
    let mut offspring = Agent::with_parents(AgentConfig { random_weights: false }, parent_ids, current_tick);

    // Inherit drives from parents with mutation
    offspring.drives = inherit_drives(&parent1.drives, &parent2.drives);

    // Inherit behavior trees from parents with pruning and mutation
    offspring.behavior_trees = inherit_behavior_trees(&parent1.behavior_trees, &parent2.behavior_trees);

    // Inherit traits from parents (mix of both with some variation)
    offspring.traits = inherit_traits(&parent1.traits, &parent2.traits);

    // Start with neutral emotions
    offspring.emotions = crate::agents::EmotionState::default();

    // Generate random preferences
    offspring.preferences = crate::core::Preferences::generate_random();

    // Place offspring near parents
    offspring.state.position = offspring_position(parent1.state.position, parent2.state.position);

    // Establish family relationships in memory
    offspring.memory.mark_as_parent(parent1.id);
    offspring.memory.mark_as_parent(parent2.id);

    offspring
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

/// Inherit traits from two parents
fn inherit_traits(traits1: &crate::agents::TraitSet, traits2: &crate::agents::TraitSet) -> crate::agents::TraitSet {
    // For now, just return a default TraitSet
    // TODO: Implement proper trait inheritance when TraitSet API is stable
    crate::agents::TraitSet::default()
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

    #[test]
    fn test_can_mate_basic() {
        let mut agent1 = Agent::new(AgentConfig::default());
        let mut agent2 = Agent::new(AgentConfig::default());

        // Set to adult stage
        agent1.state.age = 3000;
        agent1.state.life_stage = crate::agents::LifeStage::Adult;
        agent2.state.age = 3000;
        agent2.state.life_stage = crate::agents::LifeStage::Adult;

        // Set positions close together
        agent1.state.position = (0, 0, 0);
        agent2.state.position = (10, 10, 0);

        let criteria = MateSelectionCriteria::default();
        assert!(can_mate(&agent1, &agent2, &criteria));
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
        let agent1 = Agent::new(AgentConfig::default()); // Infant by default
        let mut agent2 = Agent::new(AgentConfig::default());
        agent2.state.age = 3000;
        agent2.state.life_stage = crate::agents::LifeStage::Adult;

        let criteria = MateSelectionCriteria::default();
        assert!(!can_mate(&agent1, &agent2, &criteria));
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
    }

    #[test]
    fn test_distance_calculation() {
        let pos1 = (0, 0, 0);
        let pos2 = (3, 4, 0);
        let distance = calculate_distance(pos1, pos2);
        assert!((distance - 5.0).abs() < 0.001); // 3-4-5 triangle
    }
}
