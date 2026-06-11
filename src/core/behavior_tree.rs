// src/core/behavior_tree.rs
//! Behavior tree implementation with learning and weighting.
//!
//! Behavior trees represent decision-making logic where each node is either:
//! - A composite node (sequence, selector)
//! - An action node (executes an action)
//! - A condition node (checks a condition)
//!
//! Each branch has a weight that increases with successful outcomes.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of behavior tree nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    /// Execute children in sequence until one fails
    Sequence,
    /// Execute children until one succeeds (priority selector)
    Selector,
    /// Execute a specific action
    Action(String),
    /// Check a condition
    Condition(String),
}

/// Execution result of a behavior tree node
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionResult {
    Success,
    Failure,
    Running,
}

/// Context for behavior tree execution that provides actual action/condition logic
pub trait BehaviorContext {
    /// Execute an action and return the result
    fn execute_action(&mut self, action: &str) -> ExecutionResult;

    /// Evaluate a condition and return whether it's true
    fn evaluate_condition(&self, condition: &str) -> bool;
}

/// Default behavior context that uses historical success rates for actions
/// and evaluates conditions based on common patterns
#[derive(Debug, Default)]
pub struct DefaultBehaviorContext {
    /// Cached condition states that can be set externally
    pub condition_states: std::collections::HashMap<String, bool>,
    /// Cached action results for testing
    pub action_results: std::collections::HashMap<String, ExecutionResult>,
}

impl DefaultBehaviorContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a condition state
    pub fn set_condition(&mut self, condition: &str, value: bool) {
        self.condition_states.insert(condition.to_string(), value);
    }

    /// Set an action result
    pub fn set_action_result(&mut self, action: &str, result: ExecutionResult) {
        self.action_results.insert(action.to_string(), result);
    }
}

impl BehaviorContext for DefaultBehaviorContext {
    fn execute_action(&mut self, action: &str) -> ExecutionResult {
        // Check if we have a preset result for this action
        if let Some(&result) = self.action_results.get(action) {
            return result;
        }

        // Default behavior based on action type patterns
        match action {
            // Actions that typically succeed if attempted
            a if a.contains("rest") || a.contains("idle") || a.contains("wait") => {
                ExecutionResult::Success
            }
            // Actions that require resources/conditions - return Running to indicate in-progress
            a if a.contains("hunt") || a.contains("gather") || a.contains("craft")
              || a.contains("build") || a.contains("find") => {
                ExecutionResult::Running
            }
            // Consumption actions succeed if we got here (preconditions passed)
            a if a.contains("eat") || a.contains("drink") || a.contains("consume")
              || a.contains("use") => {
                ExecutionResult::Success
            }
            // Storage/transfer actions
            a if a.contains("store") || a.contains("deposit") || a.contains("take") => {
                ExecutionResult::Success
            }
            // Movement actions
            a if a.contains("move") || a.contains("go") || a.contains("travel")
              || a.contains("seek") => {
                ExecutionResult::Running
            }
            // Social actions
            a if a.contains("talk") || a.contains("greet") || a.contains("trade") => {
                ExecutionResult::Success
            }
            // Unknown actions - return Running to indicate needs evaluation
            _ => ExecutionResult::Running,
        }
    }

    fn evaluate_condition(&self, condition: &str) -> bool {
        // Check if we have a preset state for this condition
        if let Some(&state) = self.condition_states.get(condition) {
            return state;
        }

        // Default condition evaluation based on common patterns
        // Note: More specific patterns checked first
        match condition {
            // Safety checks - check before generic is_ pattern
            c if c.contains("safe") => true,
            c if c.contains("danger") || c.contains("threat") => false,
            // Hunger/thirst/tiredness - default to true (needs are present)
            c if c.contains("hungry") || c.contains("thirsty") || c.contains("tired") => true,
            // Time-based conditions
            c if c.contains("day") => true,
            c if c.contains("night") => false,
            // Existence checks - default to false (conservative)
            c if c.contains("has_") || c.contains("have_") => false,
            // Proximity checks - default to false
            c if c.contains("nearby") || c.contains("close") || c.contains("near_") => false,
            // Resource availability - default to false
            c if c.contains("available") || c.contains("enough") => false,
            // Generic status checks - need actual state (default to false)
            c if c.contains("is_") => false,
            // Unknown conditions - default to false (be conservative)
            _ => false,
        }
    }
}

/// A node in a behavior tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorNode {
    pub id: Uuid,
    pub node_type: NodeType,
    pub weight: f32,
    pub children: Vec<BehaviorNode>,
    pub execution_count: u32,
    pub success_count: u32,
}

impl BehaviorNode {
    /// Create a new behavior node
    pub fn new(node_type: NodeType) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_type,
            weight: 1.0,
            children: Vec::new(),
            execution_count: 0,
            success_count: 0,
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: BehaviorNode) {
        self.children.push(child);
    }

    /// Update weight based on execution result
    pub fn update_weight(&mut self, result: ExecutionResult) {
        self.execution_count += 1;
        
        match result {
            ExecutionResult::Success => {
                self.success_count += 1;
                self.weight *= 1.1; // Increase weight by 10%
            }
            ExecutionResult::Failure => {
                self.weight *= 0.9; // Decrease weight by 10%
            }
            ExecutionResult::Running => {
                // No weight change for running state
            }
        }
        
        // Clamp weight between 0.1 and 10.0
        self.weight = self.weight.clamp(0.1, 10.0);
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.execution_count == 0 {
            return 0.0;
        }
        self.success_count as f32 / self.execution_count as f32
    }

    /// Prune low-weight children
    pub fn prune(&mut self, min_weight: f32) {
        self.children.retain(|child| child.weight >= min_weight);
        
        // Recursively prune children
        for child in &mut self.children {
            child.prune(min_weight);
        }
    }
}

/// A complete behavior tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorTree {
    pub id: Uuid,
    pub root: BehaviorNode,
    pub name: String,
    pub total_executions: u32,
    pub total_successes: u32,
}

impl BehaviorTree {
    /// Create a new behavior tree
    pub fn new(name: String, root: BehaviorNode) -> Self {
        Self {
            id: Uuid::new_v4(),
            root,
            name,
            total_executions: 0,
            total_successes: 0,
        }
    }

    /// Execute the behavior tree with the default context
    pub fn execute(&mut self) -> ExecutionResult {
        let mut context = DefaultBehaviorContext::new();
        self.execute_with_context(&mut context)
    }

    /// Execute the behavior tree with a custom context
    pub fn execute_with_context<C: BehaviorContext>(&mut self, context: &mut C) -> ExecutionResult {
        self.total_executions += 1;
        let result = self.execute_node(&mut self.root.clone(), context);

        if result == ExecutionResult::Success {
            self.total_successes += 1;
        }

        self.root.update_weight(result);
        result
    }

    /// Execute a single node with context
    fn execute_node<C: BehaviorContext>(&self, node: &mut BehaviorNode, context: &mut C) -> ExecutionResult {
        match &node.node_type {
            NodeType::Sequence => self.execute_sequence(node, context),
            NodeType::Selector => self.execute_selector(node, context),
            NodeType::Action(action) => {
                // Execute the action through the context
                let result = context.execute_action(action);
                node.update_weight(result);
                result
            }
            NodeType::Condition(condition) => {
                // Evaluate the condition through the context
                if context.evaluate_condition(condition) {
                    ExecutionResult::Success
                } else {
                    ExecutionResult::Failure
                }
            }
        }
    }

    /// Execute a sequence node
    fn execute_sequence<C: BehaviorContext>(&self, node: &mut BehaviorNode, context: &mut C) -> ExecutionResult {
        for child in &mut node.children {
            match self.execute_node(child, context) {
                ExecutionResult::Failure => return ExecutionResult::Failure,
                ExecutionResult::Running => return ExecutionResult::Running,
                ExecutionResult::Success => continue,
            }
        }
        ExecutionResult::Success
    }

    /// Execute a selector node
    fn execute_selector<C: BehaviorContext>(&self, node: &mut BehaviorNode, context: &mut C) -> ExecutionResult {
        for child in &mut node.children {
            match self.execute_node(child, context) {
                ExecutionResult::Success => return ExecutionResult::Success,
                ExecutionResult::Running => return ExecutionResult::Running,
                ExecutionResult::Failure => continue,
            }
        }
        ExecutionResult::Failure
    }

    /// Prune low-weight branches
    pub fn prune(&mut self, min_weight: f32) {
        self.root.prune(min_weight);
    }

    /// Get overall success rate
    pub fn success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.total_successes as f32 / self.total_executions as f32
    }

    /// Clone with weight threshold (genetic inheritance)
    pub fn clone_with_pruning(&self, min_weight: f32) -> Self {
        let mut cloned = self.clone();
        cloned.id = Uuid::new_v4(); // New ID for offspring
        cloned.prune(min_weight);
        cloned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_node_creation() {
        let node = BehaviorNode::new(NodeType::Sequence);
        assert_eq!(node.weight, 1.0);
        assert_eq!(node.execution_count, 0);
        assert_eq!(node.success_count, 0);
    }

    #[test]
    fn test_weight_update() {
        let mut node = BehaviorNode::new(NodeType::Action("test".to_string()));
        
        node.update_weight(ExecutionResult::Success);
        assert!(node.weight > 1.0);
        
        node.update_weight(ExecutionResult::Failure);
        assert!(node.weight < 1.1);
    }

    #[test]
    fn test_behavior_tree_execution() {
        let root = BehaviorNode::new(NodeType::Selector);
        let mut tree = BehaviorTree::new("test_tree".to_string(), root);
        
        let _result = tree.execute();
        assert_eq!(tree.total_executions, 1);
    }

    #[test]
    fn test_pruning() {
        let mut root = BehaviorNode::new(NodeType::Sequence);

        let mut child1 = BehaviorNode::new(NodeType::Action("action1".to_string()));
        child1.weight = 0.05; // Below threshold

        let mut child2 = BehaviorNode::new(NodeType::Action("action2".to_string()));
        child2.weight = 2.0; // Above threshold

        root.add_child(child1);
        root.add_child(child2);

        root.prune(0.1);

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].weight, 2.0);
    }

    #[test]
    fn test_context_condition_evaluation() {
        let mut context = DefaultBehaviorContext::new();

        // Default behavior for various condition patterns
        assert!(!context.evaluate_condition("has_food"));
        assert!(!context.evaluate_condition("food_nearby"));
        assert!(context.evaluate_condition("is_safe"));
        assert!(context.evaluate_condition("is_hungry"));

        // Custom condition states override defaults
        context.set_condition("has_food", true);
        assert!(context.evaluate_condition("has_food"));
    }

    #[test]
    fn test_context_action_execution() {
        let mut context = DefaultBehaviorContext::new();

        // Test default action behaviors
        assert_eq!(context.execute_action("rest"), ExecutionResult::Success);
        assert_eq!(context.execute_action("eat_food"), ExecutionResult::Success);
        assert_eq!(context.execute_action("hunt_food"), ExecutionResult::Running);
        assert_eq!(context.execute_action("find_shelter"), ExecutionResult::Running);

        // Custom action results override defaults
        context.set_action_result("hunt_food", ExecutionResult::Success);
        assert_eq!(context.execute_action("hunt_food"), ExecutionResult::Success);
    }

    #[test]
    fn test_execute_with_context() {
        let mut context = DefaultBehaviorContext::new();

        // Create a sequence: check condition -> execute action
        let mut root = BehaviorNode::new(NodeType::Sequence);
        root.add_child(BehaviorNode::new(NodeType::Condition("has_food".to_string())));
        root.add_child(BehaviorNode::new(NodeType::Action("eat_food".to_string())));

        let mut tree = BehaviorTree::new("eat_sequence".to_string(), root);

        // Without food, sequence should fail at condition
        let result = tree.execute_with_context(&mut context);
        assert_eq!(result, ExecutionResult::Failure);

        // With food, sequence should succeed
        context.set_condition("has_food", true);
        let result = tree.execute_with_context(&mut context);
        assert_eq!(result, ExecutionResult::Success);
    }

    #[test]
    fn test_selector_with_context() {
        let mut context = DefaultBehaviorContext::new();

        // Create a selector: try eat stored food OR hunt for food
        let mut root = BehaviorNode::new(NodeType::Selector);

        // First option: check if has food and eat
        let mut eat_sequence = BehaviorNode::new(NodeType::Sequence);
        eat_sequence.add_child(BehaviorNode::new(NodeType::Condition("has_stored_food".to_string())));
        eat_sequence.add_child(BehaviorNode::new(NodeType::Action("eat_stored".to_string())));

        // Second option: hunt for food
        let hunt_action = BehaviorNode::new(NodeType::Action("hunt_food".to_string()));

        root.add_child(eat_sequence);
        root.add_child(hunt_action);

        let mut tree = BehaviorTree::new("find_food_selector".to_string(), root);

        // Without stored food, should fall through to hunt (Running)
        let result = tree.execute_with_context(&mut context);
        assert_eq!(result, ExecutionResult::Running);

        // With stored food, should eat (Success)
        context.set_condition("has_stored_food", true);
        let result = tree.execute_with_context(&mut context);
        assert_eq!(result, ExecutionResult::Success);
    }
}
