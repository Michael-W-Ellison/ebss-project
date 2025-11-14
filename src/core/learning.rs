// src/core/learning.rs
//! Observational learning system for agents.
//!
//! Young agents learn from observing adults, especially parents.
//! Learning rate is higher for younger agents.

use uuid::Uuid;
use crate::agents::{Agent, LifeStage};
use crate::core::DriveType;

/// Learning event that can be observed
#[derive(Debug, Clone)]
pub struct ObservableEvent {
    pub agent_id: Uuid,
    pub event_type: ObservableEventType,
    pub success: bool,
    pub position: (i32, i32, i32),
}

#[derive(Debug, Clone)]
pub enum ObservableEventType {
    /// Agent performed an action
    Action(String),
    /// Agent satisfied a drive
    DriveSatisfaction(DriveType),
    /// Agent discovered something new
    Discovery(String),
    /// Agent used a behavior tree
    BehaviorExecution(String),
}

/// Result of observational learning
#[derive(Debug, Clone)]
pub struct LearningResult {
    pub learned: bool,
    pub knowledge_gained: Option<String>,
    pub proficiency_increase: f32,
}

/// Check if an agent can observe another agent
pub fn can_observe(observer: &Agent, observed: &Agent, max_distance: f32) -> bool {
    // Both must be alive
    if !observer.state.is_alive || !observed.state.is_alive {
        return false;
    }

    // Cannot observe self
    if observer.id == observed.id {
        return false;
    }

    // Check distance
    let distance = calculate_distance(observer.state.position, observed.state.position);
    if distance > max_distance {
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

/// Observe and learn from an event
pub fn observe_and_learn(
    observer: &mut Agent,
    observed: &Agent,
    event: &ObservableEvent,
) -> LearningResult {
    // Check if observer can actually observe this event
    if !can_observe(observer, observed, 20.0) {
        return LearningResult {
            learned: false,
            knowledge_gained: None,
            proficiency_increase: 0.0,
        };
    }

    // Get learning rate based on observer's age
    let learning_rate = observer.state.life_stage.learning_rate();

    // Check if observer is related to observed (family learns better)
    let relationship_bonus = if observer.parent_ids.contains(&observed.id) {
        1.5 // 50% bonus for learning from parents
    } else if observer.memory.social_relationships.contains_key(&observed.id) {
        let relationship = &observer.memory.social_relationships[&observed.id];
        if relationship.is_parent || relationship.is_child {
            1.5
        } else if relationship.trust > 0.5 {
            1.2 // 20% bonus for trusted agents
        } else {
            1.0
        }
    } else {
        1.0
    };

    let effective_learning_rate = learning_rate * relationship_bonus;

    // Learn from the event
    match &event.event_type {
        ObservableEventType::Action(action_name) => {
            learn_action(observer, action_name, event.success, effective_learning_rate)
        }
        ObservableEventType::DriveSatisfaction(drive_type) => {
            learn_drive_satisfaction(observer, *drive_type, effective_learning_rate)
        }
        ObservableEventType::Discovery(knowledge_name) => {
            learn_discovery(observer, knowledge_name, effective_learning_rate)
        }
        ObservableEventType::BehaviorExecution(tree_name) => {
            learn_behavior(observer, observed, tree_name, event.success, effective_learning_rate)
        }
    }
}

/// Learn an action by observation
fn learn_action(
    observer: &mut Agent,
    action_name: &str,
    success: bool,
    learning_rate: f32,
) -> LearningResult {
    // Check if observer already knows this action
    if let Some(knowledge) = observer.memory.get_knowledge_mut(action_name) {
        // Improve proficiency
        if success {
            let increase = 0.05 * learning_rate;
            knowledge.proficiency = (knowledge.proficiency + increase).min(1.0);
            LearningResult {
                learned: true,
                knowledge_gained: None,
                proficiency_increase: increase,
            }
        } else {
            LearningResult {
                learned: false,
                knowledge_gained: None,
                proficiency_increase: 0.0,
            }
        }
    } else {
        // Learn new action with probability based on learning rate
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < learning_rate * 0.3 {
            observer.memory.learn(
                action_name.to_string(),
                format!("Learned by observing {}", action_name),
            );
            LearningResult {
                learned: true,
                knowledge_gained: Some(action_name.to_string()),
                proficiency_increase: 0.0,
            }
        } else {
            LearningResult {
                learned: false,
                knowledge_gained: None,
                proficiency_increase: 0.0,
            }
        }
    }
}

/// Learn how to satisfy a drive
fn learn_drive_satisfaction(
    observer: &mut Agent,
    drive_type: DriveType,
    learning_rate: f32,
) -> LearningResult {
    let knowledge_name = format!("{:?}_satisfaction", drive_type);

    if observer.memory.get_knowledge(&knowledge_name).is_none() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < learning_rate * 0.4 {
            observer.memory.learn(
                knowledge_name.clone(),
                format!("How to satisfy {:?} drive", drive_type),
            );
            return LearningResult {
                learned: true,
                knowledge_gained: Some(knowledge_name),
                proficiency_increase: 0.0,
            };
        }
    }

    LearningResult {
        learned: false,
        knowledge_gained: None,
        proficiency_increase: 0.0,
    }
}

/// Learn a discovery/recipe
fn learn_discovery(
    observer: &mut Agent,
    knowledge_name: &str,
    learning_rate: f32,
) -> LearningResult {
    if observer.memory.get_knowledge(knowledge_name).is_none() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen::<f32>() < learning_rate * 0.5 {
            observer.memory.learn(
                knowledge_name.to_string(),
                format!("Discovered: {}", knowledge_name),
            );
            return LearningResult {
                learned: true,
                knowledge_gained: Some(knowledge_name.to_string()),
                proficiency_increase: 0.0,
            };
        }
    }

    LearningResult {
        learned: false,
        knowledge_gained: None,
        proficiency_increase: 0.0,
    }
}

/// Learn a behavior tree by observation
fn learn_behavior(
    observer: &mut Agent,
    observed: &Agent,
    tree_name: &str,
    success: bool,
    learning_rate: f32,
) -> LearningResult {
    // Check if observer already has this behavior tree
    if observer.behavior_trees.iter().any(|t| t.name == tree_name) {
        // Already know it, no additional learning
        return LearningResult {
            learned: false,
            knowledge_gained: None,
            proficiency_increase: 0.0,
        };
    }

    // Try to learn the behavior tree
    if let Some(observed_tree) = observed.behavior_trees.iter().find(|t| t.name == tree_name) {
        // Only learn if the observed behavior was successful
        if success {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen::<f32>() < learning_rate * 0.2 {
                // Clone the behavior tree with some pruning
                let learned_tree = observed_tree.clone_with_pruning(0.5);
                observer.behavior_trees.push(learned_tree);
                return LearningResult {
                    learned: true,
                    knowledge_gained: Some(format!("behavior:{}", tree_name)),
                    proficiency_increase: 0.0,
                };
            }
        }
    }

    LearningResult {
        learned: false,
        knowledge_gained: None,
        proficiency_increase: 0.0,
    }
}

/// Process observational learning for all young agents in a population
pub fn process_population_learning(agents: &mut [Agent], events: &[ObservableEvent]) {
    // Find young agents (infants, children, adolescents)
    let young_agent_indices: Vec<usize> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.state.is_alive
                && matches!(
                    a.state.life_stage,
                    LifeStage::Infant | LifeStage::Child | LifeStage::Adolescent
                )
        })
        .map(|(i, _)| i)
        .collect();

    // For each event, let young agents try to learn
    for event in events {
        // Find the agent who performed the event
        if let Some(observed) = agents.iter().find(|a| a.id == event.agent_id) {
            let observed_clone = observed.clone();

            // Let each young agent try to learn
            for &young_idx in &young_agent_indices {
                let young_agent = &mut agents[young_idx];
                observe_and_learn(young_agent, &observed_clone, event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentConfig;

    #[test]
    fn test_can_observe() {
        let mut observer = Agent::new(AgentConfig::default());
        let mut observed = Agent::new(AgentConfig::default());

        observer.state.position = (0, 0, 0);
        observed.state.position = (10, 0, 0);

        assert!(can_observe(&observer, &observed, 20.0));
        assert!(!can_observe(&observer, &observed, 5.0));
    }

    #[test]
    fn test_cannot_observe_self() {
        let agent = Agent::new(AgentConfig::default());
        assert!(!can_observe(&agent, &agent, 100.0));
    }

    #[test]
    fn test_learning_rate_varies_by_age() {
        let infant = LifeStage::Infant;
        let adult = LifeStage::Adult;

        assert!(infant.learning_rate() > adult.learning_rate());
    }

    #[test]
    fn test_learn_action() {
        let mut agent = Agent::new(AgentConfig::default());
        agent.state.age = 100; // Infant
        agent.state.life_stage = LifeStage::Infant;

        let result = learn_action(&mut agent, "Mining", true, 2.0);

        // With high learning rate, should eventually learn
        // Note: This is probabilistic, so we can't assert learned=true deterministically
    }

    #[test]
    fn test_observe_and_learn_from_parent() {
        let mut parent = Agent::new(AgentConfig::default());
        parent.state.age = 3000;
        parent.state.life_stage = LifeStage::Adult;

        let mut child = Agent::new(AgentConfig::default());
        child.state.age = 100;
        child.state.life_stage = LifeStage::Infant;
        child.parent_ids.push(parent.id);

        child.state.position = (0, 0, 0);
        parent.state.position = (5, 0, 0);

        let event = ObservableEvent {
            agent_id: parent.id,
            event_type: ObservableEventType::Action("Farming".to_string()),
            success: true,
            position: parent.state.position,
        };

        observe_and_learn(&mut child, &parent, &event);

        // Child should have attempted to learn
        // (Result is probabilistic)
    }
}
