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

    /// Execute the behavior tree
    pub fn execute(&mut self) -> ExecutionResult {
        self.total_executions += 1;
        let result = self.execute_node(&mut self.root.clone());
        
        if result == ExecutionResult::Success {
            self.total_successes += 1;
        }
        
        self.root.update_weight(result);
        result
    }

    /// Execute a single node
    fn execute_node(&self, node: &mut BehaviorNode) -> ExecutionResult {
        match &node.node_type {
            NodeType::Sequence => self.execute_sequence(node),
            NodeType::Selector => self.execute_selector(node),
            NodeType::Action(_action) => {
                // Action execution would be handled by the agent
                // For now, return success with probability based on weight
                if rand::random::<f32>() < node.success_rate() {
                    ExecutionResult::Success
                } else {
                    ExecutionResult::Failure
                }
            }
            NodeType::Condition(_condition) => {
                // Condition checking would be handled by the agent
                // For now, return success with 50% probability
                if rand::random::<bool>() {
                    ExecutionResult::Success
                } else {
                    ExecutionResult::Failure
                }
            }
        }
    }

    /// Execute a sequence node
    fn execute_sequence(&self, node: &mut BehaviorNode) -> ExecutionResult {
        for child in &mut node.children {
            match self.execute_node(child) {
                ExecutionResult::Failure => return ExecutionResult::Failure,
                ExecutionResult::Running => return ExecutionResult::Running,
                ExecutionResult::Success => continue,
            }
        }
        ExecutionResult::Success
    }

    /// Execute a selector node
    fn execute_selector(&self, node: &mut BehaviorNode) -> ExecutionResult {
        for child in &mut node.children {
            match self.execute_node(child) {
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
        
        let result = tree.execute();
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
}
