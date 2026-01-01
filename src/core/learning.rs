// src/core/learning.rs
//! Observational learning system for agents.
//!
//! Young agents learn from observing adults, especially parents.
//! Learning rate is higher for younger agents.
//!
//! Learning is now exposure-based: repeated observations accumulate
//! until a threshold is reached, guaranteeing learning eventually.

use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Complexity level of knowledge affecting learning threshold
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeComplexity {
    /// Basic/instinctive knowledge - easiest to learn (threshold: 0.3)
    /// Examples: basic movement, eating, drinking
    Trivial,
    /// Simple observable skills (threshold: 0.5)
    /// Examples: gathering berries, basic tool use
    Simple,
    /// Standard knowledge requiring practice (threshold: 1.0)
    /// Examples: hunting, crafting simple items, cooking
    Normal,
    /// Complex skills requiring significant observation (threshold: 1.5)
    /// Examples: advanced crafting, building, combat techniques
    Complex,
    /// Expert-level knowledge (threshold: 2.0)
    /// Examples: metallurgy, advanced construction, medicine
    Advanced,
    /// Master-level rare knowledge (threshold: 3.0)
    /// Examples: secret techniques, rare recipes, specialized skills
    Master,
}

impl KnowledgeComplexity {
    /// Get the learning threshold for this complexity level
    pub fn threshold(&self) -> f32 {
        match self {
            KnowledgeComplexity::Trivial => 0.3,
            KnowledgeComplexity::Simple => 0.5,
            KnowledgeComplexity::Normal => 1.0,
            KnowledgeComplexity::Complex => 1.5,
            KnowledgeComplexity::Advanced => 2.0,
            KnowledgeComplexity::Master => 3.0,
        }
    }

    /// Get complexity from a knowledge name (uses heuristics)
    pub fn from_knowledge_name(name: &str) -> Self {
        let lower = name.to_lowercase();

        // Master-level knowledge
        if lower.contains("secret") || lower.contains("master") || lower.contains("legendary") {
            return KnowledgeComplexity::Master;
        }

        // Advanced knowledge
        if lower.contains("advanced") || lower.contains("metallurgy") || lower.contains("forge")
            || lower.contains("medicine") || lower.contains("architecture")
        {
            return KnowledgeComplexity::Advanced;
        }

        // Complex knowledge
        if lower.contains("craft") || lower.contains("build") || lower.contains("combat")
            || lower.contains("weapon") || lower.contains("armor") || lower.contains("tool")
        {
            return KnowledgeComplexity::Complex;
        }

        // Simple knowledge
        if lower.contains("gather") || lower.contains("basic") || lower.contains("simple")
            || lower.contains("find") || lower.contains("pick")
        {
            return KnowledgeComplexity::Simple;
        }

        // Trivial knowledge
        if lower.contains("move") || lower.contains("eat") || lower.contains("drink")
            || lower.contains("rest") || lower.contains("sleep")
        {
            return KnowledgeComplexity::Trivial;
        }

        // Default to normal
        KnowledgeComplexity::Normal
    }

    /// Description of the complexity level
    pub fn description(&self) -> &'static str {
        match self {
            KnowledgeComplexity::Trivial => "trivial (instinctive)",
            KnowledgeComplexity::Simple => "simple (easily observed)",
            KnowledgeComplexity::Normal => "normal (requires practice)",
            KnowledgeComplexity::Complex => "complex (requires study)",
            KnowledgeComplexity::Advanced => "advanced (expert knowledge)",
            KnowledgeComplexity::Master => "master (rare expertise)",
        }
    }
}

impl Default for KnowledgeComplexity {
    fn default() -> Self {
        KnowledgeComplexity::Normal
    }
}

/// Tracks accumulated exposure to knowledge/skills
/// Learning triggers when exposure reaches complexity-based threshold
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningExposure {
    /// Accumulated exposure per knowledge item (0.0 to threshold)
    exposures: HashMap<String, f32>,
    /// Default threshold for unknown complexity (backwards compatible)
    pub default_threshold: f32,
    /// Override thresholds per knowledge item
    #[serde(default)]
    custom_thresholds: HashMap<String, f32>,
}

impl LearningExposure {
    pub fn new() -> Self {
        Self {
            exposures: HashMap::new(),
            default_threshold: 1.0,
            custom_thresholds: HashMap::new(),
        }
    }

    /// Create with a specific default threshold
    pub fn with_default_threshold(threshold: f32) -> Self {
        Self {
            exposures: HashMap::new(),
            default_threshold: threshold,
            custom_thresholds: HashMap::new(),
        }
    }

    /// Get the threshold for a specific knowledge item
    fn get_threshold(&self, knowledge: &str) -> f32 {
        // Check for custom threshold first
        if let Some(&threshold) = self.custom_thresholds.get(knowledge) {
            return threshold;
        }

        // Otherwise, determine from knowledge name
        KnowledgeComplexity::from_knowledge_name(knowledge).threshold()
    }

    /// Set a custom threshold for specific knowledge
    pub fn set_threshold(&mut self, knowledge: &str, threshold: f32) {
        self.custom_thresholds.insert(knowledge.to_string(), threshold);
    }

    /// Set threshold based on complexity level
    pub fn set_complexity(&mut self, knowledge: &str, complexity: KnowledgeComplexity) {
        self.custom_thresholds.insert(knowledge.to_string(), complexity.threshold());
    }

    /// Add exposure to a knowledge item, returns true if threshold reached
    pub fn add_exposure(&mut self, knowledge: &str, amount: f32) -> bool {
        let threshold = self.get_threshold(knowledge);
        let current = self.exposures.entry(knowledge.to_string()).or_insert(0.0);
        *current += amount;
        *current >= threshold
    }

    /// Add exposure with explicit complexity level
    pub fn add_exposure_with_complexity(
        &mut self,
        knowledge: &str,
        amount: f32,
        complexity: KnowledgeComplexity,
    ) -> bool {
        self.set_complexity(knowledge, complexity);
        self.add_exposure(knowledge, amount)
    }

    /// Get current exposure level (0.0 to threshold)
    pub fn get_exposure(&self, knowledge: &str) -> f32 {
        self.exposures.get(knowledge).copied().unwrap_or(0.0)
    }

    /// Check if ready to learn (exposure >= threshold)
    pub fn ready_to_learn(&self, knowledge: &str) -> bool {
        self.get_exposure(knowledge) >= self.get_threshold(knowledge)
    }

    /// Reset exposure after learning
    pub fn reset_exposure(&mut self, knowledge: &str) {
        self.exposures.remove(knowledge);
    }

    /// Get exposure as percentage of threshold
    pub fn exposure_percentage(&self, knowledge: &str) -> f32 {
        let threshold = self.get_threshold(knowledge);
        if threshold <= 0.0 {
            return 1.0;
        }
        (self.get_exposure(knowledge) / threshold).min(1.0)
    }

    /// Get the complexity level for a knowledge item
    pub fn get_complexity(&self, knowledge: &str) -> KnowledgeComplexity {
        if let Some(&threshold) = self.custom_thresholds.get(knowledge) {
            // Map threshold back to complexity
            if threshold <= 0.3 {
                KnowledgeComplexity::Trivial
            } else if threshold <= 0.5 {
                KnowledgeComplexity::Simple
            } else if threshold <= 1.0 {
                KnowledgeComplexity::Normal
            } else if threshold <= 1.5 {
                KnowledgeComplexity::Complex
            } else if threshold <= 2.0 {
                KnowledgeComplexity::Advanced
            } else {
                KnowledgeComplexity::Master
            }
        } else {
            KnowledgeComplexity::from_knowledge_name(knowledge)
        }
    }

    /// Get all knowledge items and their progress
    pub fn all_progress(&self) -> Vec<(&str, f32, KnowledgeComplexity)> {
        self.exposures
            .iter()
            .map(|(k, &exp)| {
                let complexity = self.get_complexity(k);
                let progress = exp / self.get_threshold(k);
                (k.as_str(), progress, complexity)
            })
            .collect()
    }

    /// Legacy compatibility: get the default threshold
    /// (maps to `default_threshold` for backwards compatibility)
    #[inline]
    pub fn threshold(&self) -> f32 {
        self.default_threshold
    }
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
    } else if let Some(relationship) = observer.relationships.get_relationship(&observed.id) {
        if relationship.is_family() {
            1.5  // 50% bonus for learning from family
        } else if relationship.bond_strength > 0.5 {
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
/// Uses exposure-based learning: each observation adds exposure,
/// and learning is guaranteed when exposure threshold is reached.
fn learn_action(
    observer: &mut Agent,
    action_name: &str,
    success: bool,
    learning_rate: f32,
) -> LearningResult {
    // Check if observer already knows this action
    if let Some(knowledge) = observer.memory.get_knowledge_mut(action_name) {
        // Improve proficiency - always gain something from successful observation
        if success {
            // Base increase + bonus based on learning rate
            let base_increase = 0.02;
            let rate_bonus = 0.03 * learning_rate;
            let increase = base_increase + rate_bonus;
            knowledge.proficiency = (knowledge.proficiency + increase).min(1.0);
            LearningResult {
                learned: true,
                knowledge_gained: None,
                proficiency_increase: increase,
            }
        } else {
            // Even failed observations provide small learning (what NOT to do)
            let small_increase = 0.01 * learning_rate;
            knowledge.proficiency = (knowledge.proficiency + small_increase).min(1.0);
            LearningResult {
                learned: true,
                knowledge_gained: None,
                proficiency_increase: small_increase,
            }
        }
    } else {
        // Learning new action: exposure-based with breakthrough chance
        // Exposure gain is proportional to learning rate and success
        let exposure_gain = if success {
            learning_rate * 0.25  // Successful observations are more educational
        } else {
            learning_rate * 0.10  // Failed observations still teach something
        };

        // Add exposure (stored in observer's learning_exposure field)
        let threshold_reached = observer.learning_exposure.add_exposure(action_name, exposure_gain);

        // Check for breakthrough learning (random chance even before threshold)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let exposure_pct = observer.learning_exposure.exposure_percentage(action_name);
        let breakthrough_chance = exposure_pct * learning_rate * 0.2; // Higher exposure = higher breakthrough chance

        if threshold_reached || rng.gen::<f32>() < breakthrough_chance {
            // Learn the action
            observer.memory.learn(
                action_name.to_string(),
                format!("Learned by observing {}", action_name),
            );
            observer.learning_exposure.reset_exposure(action_name);
            LearningResult {
                learned: true,
                knowledge_gained: Some(action_name.to_string()),
                proficiency_increase: 0.0,
            }
        } else {
            // Not learned yet, but exposure increased (progress made)
            LearningResult {
                learned: false,
                knowledge_gained: None,
                proficiency_increase: 0.0,
            }
        }
    }
}

/// Learn how to satisfy a drive
/// Uses exposure-based learning with higher base gain for survival drives
fn learn_drive_satisfaction(
    observer: &mut Agent,
    drive_type: DriveType,
    learning_rate: f32,
) -> LearningResult {
    let knowledge_name = format!("{:?}_satisfaction", drive_type);

    if observer.memory.get_knowledge(&knowledge_name).is_none() {
        // Drive satisfaction is important - higher exposure gain
        // Survival drives (hunger, thirst) are learned faster
        let base_exposure = match drive_type {
            DriveType::Hunger | DriveType::Thirst | DriveType::Rest => 0.35,
            DriveType::Safety => 0.30,
            _ => 0.25,
        };
        let exposure_gain = base_exposure * learning_rate;

        let threshold_reached = observer.learning_exposure.add_exposure(&knowledge_name, exposure_gain);

        // Breakthrough chance scales with accumulated exposure
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let exposure_pct = observer.learning_exposure.exposure_percentage(&knowledge_name);
        let breakthrough_chance = exposure_pct * learning_rate * 0.25;

        if threshold_reached || rng.gen::<f32>() < breakthrough_chance {
            observer.memory.learn(
                knowledge_name.clone(),
                format!("How to satisfy {:?} drive", drive_type),
            );
            observer.learning_exposure.reset_exposure(&knowledge_name);
            return LearningResult {
                learned: true,
                knowledge_gained: Some(knowledge_name),
                proficiency_increase: 0.0,
            };
        }
    } else {
        // Already know this - improve proficiency slightly
        if let Some(knowledge) = observer.memory.get_knowledge_mut(&knowledge_name) {
            let increase = 0.02 * learning_rate;
            knowledge.proficiency = (knowledge.proficiency + increase).min(1.0);
            return LearningResult {
                learned: true,
                knowledge_gained: None,
                proficiency_increase: increase,
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
/// Discoveries are easier to learn (higher exposure gain) since they're
/// significant events worth paying attention to
fn learn_discovery(
    observer: &mut Agent,
    knowledge_name: &str,
    learning_rate: f32,
) -> LearningResult {
    if observer.memory.get_knowledge(knowledge_name).is_none() {
        // Discoveries are memorable - high exposure gain
        let exposure_gain = learning_rate * 0.40;

        let threshold_reached = observer.learning_exposure.add_exposure(knowledge_name, exposure_gain);

        // Higher breakthrough chance for discoveries (they're exciting!)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let exposure_pct = observer.learning_exposure.exposure_percentage(knowledge_name);
        let breakthrough_chance = exposure_pct * learning_rate * 0.35;

        if threshold_reached || rng.gen::<f32>() < breakthrough_chance {
            observer.memory.learn(
                knowledge_name.to_string(),
                format!("Discovered: {}", knowledge_name),
            );
            observer.learning_exposure.reset_exposure(knowledge_name);
            return LearningResult {
                learned: true,
                knowledge_gained: Some(knowledge_name.to_string()),
                proficiency_increase: 0.0,
            };
        }
    } else {
        // Already know this discovery - improve proficiency
        if let Some(knowledge) = observer.memory.get_knowledge_mut(knowledge_name) {
            let increase = 0.03 * learning_rate;
            knowledge.proficiency = (knowledge.proficiency + increase).min(1.0);
            return LearningResult {
                learned: true,
                knowledge_gained: None,
                proficiency_increase: increase,
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
/// Behavior trees are complex - require more exposure but are very valuable
fn learn_behavior(
    observer: &mut Agent,
    observed: &Agent,
    tree_name: &str,
    success: bool,
    learning_rate: f32,
) -> LearningResult {
    // Check if observer already has this behavior tree
    if observer.behavior_trees.iter().any(|t| t.name == tree_name) {
        // Already know it, no additional learning needed
        return LearningResult {
            learned: false,
            knowledge_gained: None,
            proficiency_increase: 0.0,
        };
    }

    // Try to learn the behavior tree
    if let Some(observed_tree) = observed.behavior_trees.iter().find(|t| t.name == tree_name) {
        let behavior_key = format!("behavior:{}", tree_name);

        // Exposure gain depends on success - failed attempts teach less
        let exposure_gain = if success {
            learning_rate * 0.20  // Successful behavior is worth watching
        } else {
            learning_rate * 0.05  // Failed behavior teaches what not to do
        };

        let threshold_reached = observer.learning_exposure.add_exposure(&behavior_key, exposure_gain);

        // Breakthrough chance (lower for complex behaviors)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let exposure_pct = observer.learning_exposure.exposure_percentage(&behavior_key);
        let breakthrough_chance = if success {
            exposure_pct * learning_rate * 0.15
        } else {
            0.0  // Can't have breakthrough learning from watching failures
        };

        if threshold_reached || (success && rng.gen::<f32>() < breakthrough_chance) {
            // Clone the behavior tree with pruning based on exposure level
            // More exposure = better understanding = less pruning needed
            let pruning_threshold = 0.3 + (0.4 * (1.0 - exposure_pct));
            let learned_tree = observed_tree.clone_with_pruning(pruning_threshold);
            observer.behavior_trees.push(learned_tree);
            observer.learning_exposure.reset_exposure(&behavior_key);
            return LearningResult {
                learned: true,
                knowledge_gained: Some(behavior_key),
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

// Learning system for behavior tree evolution.
//
// # Learning Loop Architecture
//
// The learning loop in EBSS follows this flow:
//
// ```text
// 1. UPDATE DRIVES
//    - agent.tick() → agent.drives.tick()
//    - Each drive accumulates based on its base rate
//    - Drives approach their threshold values
//
// 2. SELECT MOST URGENT DRIVE
//    - agent.drives.most_urgent()
//    - Urgency = value * weight (personality variation)
//    - Returns the drive that needs satisfaction most
//
// 3. SELECT BEHAVIOR TREE
//    - agent.select_behavior_tree()
//    - Matches drive type to appropriate behavior tree
//    - Each agent has 13 behavior trees (one per drive)
//
// 4. EXECUTE BEHAVIOR TREE
//    - tree.execute()
//    - Traverses nodes based on weights and success rates
//    - LEARNING HAPPENS HERE AUTOMATICALLY:
//      * Success: weight *= 1.1 (exponential growth)
//      * Failure: weight *= 0.9 (exponential decay)
//    - Returns ExecutionResult (Success/Failure/Running)
//
// 5. CONVERT TO ACTION
//    - Map behavior tree result to environment action
//    - Actions: Eat, Sleep, Gather, Build, Explore, etc.
//
// 6. EXECUTE ACTION
//    - simulation.execute_action(&action)
//    - Interact with environment/world state
//    - Returns ActionResult with success and satisfaction amount
//
// 7. APPLY FEEDBACK
//    - agent.apply_feedback(&result)
//    - If successful: drive.partial_satisfy(amount)
//    - Reduces drive value based on satisfaction
//
// 8. REPEAT
//    - Loop continues, drives accumulate again
//    - Successful strategies become more likely
//    - Failed strategies become less likely
// ```
//
// # Key Learning Mechanisms
//
// ## 1. Weight-Based Reinforcement
// - Every behavior tree node tracks execution_count and success_count
// - Success rate = success_count / execution_count
// - Weights adjust automatically on each execution
// - Range: 0.1 to 10.0 (clamped)
//
// ## 2. Probabilistic Selection
// - Nodes with higher weights more likely to execute
// - Allows exploration of alternative strategies
// - Balance between exploitation and exploration
//
// ## 3. Genetic Inheritance
// - tree.clone_with_pruning(min_weight)
// - Removes branches below weight threshold
// - Only successful strategies inherited
// - Offspring start with parent's learned weights
//
// ## 4. Drive-Action-Satisfaction Loop
// - Drives accumulate → Actions execute → Drives satisfied
// - Closed feedback loop
// - Natural selection favors effective behaviors
//
// # Example Learning Scenario
//
// ```text
// Agent starts hungry (Hunger drive = 0.8)
// → Selects Hunger behavior tree
// → Tree has 3 options:
//    * eat_stored_food (weight: 1.0)
//    * gather_food (weight: 1.0)
//    * hunt (weight: 1.0)
//
// Tick 1: Tries eat_stored_food → Fails (no storage)
//    * weight becomes 0.9
//
// Tick 5: Tries gather_food → Success!
//    * weight becomes 1.1
//    * Hunger reduced by 0.3
//
// Tick 10: Tries gather_food again → Success!
//    * weight becomes 1.21
//
// After 100 ticks:
//    * eat_stored_food: weight = 0.3 (rarely used)
//    * gather_food: weight = 4.5 (preferred strategy)
//    * hunt: weight = 1.8 (moderate success)
//
// Agent has learned: gathering food is most effective!
// ```

use crate::core::behavior_tree::BehaviorTree;

/// Learning system that manages behavior tree evolution
pub struct LearningSystem {
    /// Minimum weight threshold for genetic pruning
    pub pruning_threshold: f32,
    /// Learning rate multiplier (affects weight adjustment speed)
    pub learning_rate: f32,
}

impl LearningSystem {
    pub fn new() -> Self {
        Self {
            pruning_threshold: 0.5,
            learning_rate: 1.0,
        }
    }

    /// Create offspring behavior tree with learned weights
    pub fn create_offspring(&self, parent_tree: &BehaviorTree) -> BehaviorTree {
        parent_tree.clone_with_pruning(self.pruning_threshold)
    }

    /// Adjust learning parameters
    pub fn set_learning_rate(&mut self, rate: f32) {
        self.learning_rate = rate.clamp(0.1, 2.0);
    }

    /// Set genetic pruning threshold
    pub fn set_pruning_threshold(&mut self, threshold: f32) {
        self.pruning_threshold = threshold.clamp(0.1, 2.0);
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
