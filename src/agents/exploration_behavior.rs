// src/agents/exploration_behavior.rs
//! Exploration behavior and decision-making for agents.
//!
//! This module contains logic for agents to decide when and where to explore.

use crate::world::Position;
use crate::core::DriveType;
use super::Agent;

/// Exploration decision result
#[derive(Debug, Clone)]
pub enum ExplorationDecision {
    /// Should explore towards this position
    Explore { target: Position, reason: String },
    /// Should not explore right now
    NoExploration { reason: String },
}

impl Agent {
    /// Decide if and where the agent should explore
    ///
    /// Exploration is suppressed when survival drives (hunger/thirst) are active.
    /// A starving agent should focus on finding food, not wandering into unknown territory.
    pub fn decide_exploration(&self, current_tick: u32) -> ExplorationDecision {
        // Check if survival drives are active - survival takes priority over exploration
        let hunger_active = self.drives.get(DriveType::Hunger)
            .map(|d| d.is_active())
            .unwrap_or(false);
        let thirst_active = self.drives.get(DriveType::Thirst)
            .map(|d| d.is_active())
            .unwrap_or(false);

        if hunger_active || thirst_active {
            return ExplorationDecision::NoExploration {
                reason: "Survival drives active - must address hunger/thirst before exploring".to_string(),
            };
        }

        // Get curiosity drive
        let curiosity = self.drives.get(DriveType::Curiosity)
            .map(|d| d.value)
            .unwrap_or(0.0);

        // Check current position
        let current_pos = Position::new(self.state.position.0, self.state.position.1);

        // Count unexplored neighbors
        let unexplored_nearby = self.exploration_knowledge.count_unexplored_neighbors(&current_pos);

        // Calculate ticks since last exploration
        let ticks_since_exploration = current_tick.saturating_sub(
            self.exploration_knowledge.last_exploration_tick
        );

        // Determine if should explore
        if !super::exploration::should_explore(curiosity, unexplored_nearby, ticks_since_exploration) {
            return ExplorationDecision::NoExploration {
                reason: format!(
                    "Low curiosity ({:.2}), few unexplored tiles ({}), recent exploration ({} ticks ago)",
                    curiosity, unexplored_nearby, ticks_since_exploration
                ),
            };
        }

        // Find nearest unexplored tile
        let search_radius = if curiosity > 0.7 {
            20 // High curiosity = willing to travel far
        } else if curiosity > 0.4 {
            10 // Medium curiosity = moderate travel
        } else {
            5  // Low curiosity = only nearby
        };

        if let Some(target) = self.exploration_knowledge.find_nearest_unexplored(&current_pos, search_radius) {
            ExplorationDecision::Explore {
                target,
                reason: format!(
                    "High curiosity ({:.2}), unexplored tiles nearby ({})",
                    curiosity, unexplored_nearby
                ),
            }
        } else {
            ExplorationDecision::NoExploration {
                reason: "No unexplored tiles within search radius".to_string(),
            }
        }
    }

    /// Get exploration priority (0.0 to 1.0)
    ///
    /// Returns 0.0 when survival drives are active - agents should not explore when starving.
    /// Higher values mean exploration is more important right now.
    pub fn exploration_priority(&self) -> f32 {
        // Survival drives suppress exploration priority entirely
        let hunger_active = self.drives.get(DriveType::Hunger)
            .map(|d| d.is_active())
            .unwrap_or(false);
        let thirst_active = self.drives.get(DriveType::Thirst)
            .map(|d| d.is_active())
            .unwrap_or(false);

        if hunger_active || thirst_active {
            return 0.0; // No exploration when survival is threatened
        }

        let curiosity = self.drives.get(DriveType::Curiosity)
            .map(|d| d.value)
            .unwrap_or(0.0);

        let current_pos = Position::new(self.state.position.0, self.state.position.1);
        let unexplored_nearby = self.exploration_knowledge.count_unexplored_neighbors(&current_pos);

        // Base priority on curiosity drive
        let mut priority = curiosity;

        // Boost if many unexplored tiles nearby
        if unexplored_nearby > 5 {
            priority += 0.2;
        }

        // Boost if haven't explored in a long time
        let ticks_since_exploration = self.exploration_knowledge.last_exploration_tick;
        if ticks_since_exploration > 1000 {
            priority += 0.1;
        }

        priority.min(1.0)
    }
}

/// Generate a random exploration direction
pub fn random_exploration_direction() -> (i32, i32) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let directions = [
        (1, 0),   // East
        (-1, 0),  // West
        (0, 1),   // South
        (0, -1),  // North
        (1, 1),   // Southeast
        (1, -1),  // Northeast
        (-1, 1),  // Southwest
        (-1, -1), // Northwest
    ];

    directions[rng.gen_range(0..directions.len())]
}

/// Calculate the best direction to move for exploration
pub fn calculate_exploration_direction(
    current_pos: &Position,
    target_pos: &Position,
) -> (i32, i32) {
    let dx = (target_pos.x - current_pos.x).signum();
    let dy = (target_pos.y - current_pos.y).signum();

    (dx, dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentConfig;

    #[test]
    fn test_exploration_priority() {
        let mut agent = Agent::new(AgentConfig::default());

        // Set high curiosity
        if let Some(drive) = agent.drives.get_mut(DriveType::Curiosity) {
            drive.value = 0.8;
        }

        let priority = agent.exploration_priority();
        assert!(priority > 0.7); // Should be high with high curiosity
    }

    #[test]
    fn test_exploration_decision_low_curiosity() {
        let agent = Agent::new(AgentConfig::default());

        // Low curiosity should not explore
        let decision = agent.decide_exploration(0);
        assert!(matches!(decision, ExplorationDecision::NoExploration { .. }));
    }

    #[test]
    fn test_random_exploration_direction() {
        let (dx, dy) = random_exploration_direction();

        // Should be a valid direction
        assert!((-1..=1).contains(&dx));
        assert!((-1..=1).contains(&dy));
        assert!(dx != 0 || dy != 0); // Not both zero
    }

    #[test]
    fn test_calculate_exploration_direction() {
        let current = Position::new(5, 5);
        let target = Position::new(10, 8);

        let (dx, dy) = calculate_exploration_direction(&current, &target);

        // Should move towards target
        assert_eq!(dx, 1);  // East
        assert_eq!(dy, 1);  // South
    }
}
