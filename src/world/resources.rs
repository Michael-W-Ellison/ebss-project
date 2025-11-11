// src/world/resources.rs
//! Resource nodes and harvestable materials.

use serde::{Deserialize, Serialize};
use crate::world::Position;

/// Types of resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Wood,
    Stone,
    Iron,
    Food,
}

impl ResourceType {
    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            ResourceType::Wood => 't',
            ResourceType::Stone => 's',
            ResourceType::Iron => 'i',
            ResourceType::Food => 'f',
        }
    }

    /// Get color code for terminal rendering
    pub fn color_code(&self) -> &'static str {
        match self {
            ResourceType::Wood => "\x1b[33m",      // Yellow/Brown
            ResourceType::Stone => "\x1b[37;1m",   // Bright White
            ResourceType::Iron => "\x1b[90m",      // Dark Gray
            ResourceType::Food => "\x1b[92m",      // Bright Green
        }
    }

    /// Get gather time per unit (in ticks)
    pub fn gather_time(&self) -> u32 {
        match self {
            ResourceType::Wood => 20,
            ResourceType::Stone => 30,
            ResourceType::Iron => 40,
            ResourceType::Food => 15,
        }
    }
}

/// A resource node in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub position: Position,
    pub amount: u32,
    pub max_amount: u32,
}

impl ResourceNode {
    pub fn new(resource_type: ResourceType, position: Position, amount: u32) -> Self {
        Self {
            resource_type,
            position,
            amount,
            max_amount: amount,
        }
    }

    /// Harvest resource from this node
    pub fn harvest(&mut self, amount: u32) -> u32 {
        let harvested = amount.min(self.amount);
        self.amount -= harvested;
        harvested
    }

    /// Check if node is depleted
    pub fn is_depleted(&self) -> bool {
        self.amount == 0
    }

    /// Get percentage remaining
    pub fn percentage_remaining(&self) -> f32 {
        if self.max_amount == 0 {
            return 0.0;
        }
        (self.amount as f32 / self.max_amount as f32) * 100.0
    }
}

/// Resource for tracking what's needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl Resource {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self {
            resource_type,
            amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_node_creation() {
        let pos = Position::new(5, 5);
        let node = ResourceNode::new(ResourceType::Wood, pos, 100);

        assert_eq!(node.resource_type, ResourceType::Wood);
        assert_eq!(node.position, pos);
        assert_eq!(node.amount, 100);
        assert_eq!(node.max_amount, 100);
    }

    #[test]
    fn test_resource_harvest() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Wood, pos, 100);

        let harvested = node.harvest(30);
        assert_eq!(harvested, 30);
        assert_eq!(node.amount, 70);

        // Try to harvest more than available
        let harvested = node.harvest(100);
        assert_eq!(harvested, 70); // Only 70 left
        assert_eq!(node.amount, 0);
        assert!(node.is_depleted());
    }

    #[test]
    fn test_resource_percentage() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Stone, pos, 100);

        assert!((node.percentage_remaining() - 100.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 50.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 0.0).abs() < 0.1);
    }
}
