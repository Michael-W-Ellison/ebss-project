// src/agents/shared_knowledge.rs
//! Shared knowledge system for agent communication.
//!
//! Allows agents to discover and share information about resource locations,
//! threats, and other important world information.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::world::{Position, ResourceType};

/// A discovered resource location with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredResource {
    pub position: Position,
    pub resource_type: ResourceType,
    pub discovered_tick: u32,
    pub last_verified_tick: u32,
    pub estimated_amount: u32, // Last known amount
    pub discoverers: Vec<uuid::Uuid>, // Agents who know about this
}

/// Shared knowledge base for the population
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedKnowledge {
    /// Discovered resource locations indexed by position
    discovered_resources: HashMap<Position, DiscoveredResource>,
    /// Current tick (for aging information)
    current_tick: u32,
}

impl SharedKnowledge {
    pub fn new() -> Self {
        Self {
            discovered_resources: HashMap::new(),
            current_tick: 0,
        }
    }

    /// Update current tick (called each simulation tick)
    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    /// Record a resource discovery or update existing knowledge
    pub fn discover_resource(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        amount: u32,
        discoverer_id: uuid::Uuid,
    ) {
        if let Some(discovered) = self.discovered_resources.get_mut(&position) {
            // Update existing knowledge
            discovered.last_verified_tick = self.current_tick;
            discovered.estimated_amount = amount;
            if !discovered.discoverers.contains(&discoverer_id) {
                discovered.discoverers.push(discoverer_id);
            }
        } else {
            // New discovery
            self.discovered_resources.insert(
                position,
                DiscoveredResource {
                    position,
                    resource_type,
                    discovered_tick: self.current_tick,
                    last_verified_tick: self.current_tick,
                    estimated_amount: amount,
                    discoverers: vec![discoverer_id],
                },
            );
        }
    }

    /// Remove a resource from knowledge (depleted)
    pub fn remove_resource(&mut self, position: &Position) {
        self.discovered_resources.remove(position);
    }

    /// Get all known resources of a specific type
    pub fn get_resources_of_type(&self, resource_type: ResourceType) -> Vec<&DiscoveredResource> {
        self.discovered_resources
            .values()
            .filter(|r| r.resource_type == resource_type && r.estimated_amount > 0)
            .collect()
    }

    /// Find the closest known resource of a specific type to a position
    pub fn find_closest_resource(
        &self,
        from: &Position,
        resource_type: ResourceType,
    ) -> Option<&DiscoveredResource> {
        self.get_resources_of_type(resource_type)
            .into_iter()
            .min_by_key(|r| from.distance_to(&r.position))
    }


    /// Check if a resource at a position is known
    pub fn has_resource_at(&self, position: &Position) -> bool {
        self.discovered_resources.contains_key(position)
    }

    /// Broadcast: Share all resources known by one agent with another
    /// This simulates agents communicating/teaching each other
    pub fn share_knowledge(&mut self, from_agent: uuid::Uuid, to_agent: uuid::Uuid) {
        // Add to_agent to all resources discovered by from_agent
        for resource in self.discovered_resources.values_mut() {
            if resource.discoverers.contains(&from_agent) && !resource.discoverers.contains(&to_agent) {
                resource.discoverers.push(to_agent);
            }
        }
    }

    /// Get resources known by a specific agent
    pub fn get_agent_knowledge(&self, agent_id: uuid::Uuid, resource_type: ResourceType) -> Vec<&DiscoveredResource> {
        self.discovered_resources
            .values()
            .filter(|r| {
                r.resource_type == resource_type
                    && r.estimated_amount > 0
                    && r.discoverers.contains(&agent_id)
            })
            .collect()
    }


    /// Clean up old/stale resource knowledge
    /// Removes resources that haven't been verified in a long time
    pub fn cleanup_stale(&mut self, max_age_ticks: u32) {
        self.discovered_resources
            .retain(|_, r| self.current_tick - r.last_verified_tick <= max_age_ticks);
    }
}

impl Default for SharedKnowledge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_resource() {
        let mut knowledge = SharedKnowledge::new();
        let agent_id = uuid::Uuid::new_v4();
        let pos = Position::new(10, 10);

        knowledge.discover_resource(pos, ResourceType::Food, 50, agent_id);

        assert!(knowledge.has_resource_at(&pos));
        let resources = knowledge.get_resources_of_type(ResourceType::Food);
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].estimated_amount, 50);
    }

    #[test]
    fn test_find_closest_resource() {
        let mut knowledge = SharedKnowledge::new();
        let agent_id = uuid::Uuid::new_v4();

        knowledge.discover_resource(Position::new(10, 10), ResourceType::Food, 50, agent_id);
        knowledge.discover_resource(Position::new(20, 20), ResourceType::Food, 30, agent_id);
        knowledge.discover_resource(Position::new(5, 5), ResourceType::Food, 40, agent_id);

        let from = Position::new(0, 0);
        let closest = knowledge.find_closest_resource(&from, ResourceType::Food);

        assert!(closest.is_some());
        assert_eq!(closest.unwrap().position, Position::new(5, 5));
    }

    #[test]
    fn test_share_knowledge() {
        let mut knowledge = SharedKnowledge::new();
        let agent1 = uuid::Uuid::new_v4();
        let agent2 = uuid::Uuid::new_v4();
        let pos = Position::new(10, 10);

        knowledge.discover_resource(pos, ResourceType::Food, 50, agent1);

        // Agent2 doesn't know about it yet
        let agent2_knowledge = knowledge.get_agent_knowledge(agent2, ResourceType::Food);
        assert_eq!(agent2_knowledge.len(), 0);

        // Agent1 shares with agent2
        knowledge.share_knowledge(agent1, agent2);

        // Now agent2 knows about it
        let agent2_knowledge = knowledge.get_agent_knowledge(agent2, ResourceType::Food);
        assert_eq!(agent2_knowledge.len(), 1);
    }

    #[test]
    fn test_remove_depleted_resource() {
        let mut knowledge = SharedKnowledge::new();
        let agent_id = uuid::Uuid::new_v4();
        let pos = Position::new(10, 10);

        knowledge.discover_resource(pos, ResourceType::Food, 50, agent_id);
        assert!(knowledge.has_resource_at(&pos));

        knowledge.remove_resource(&pos);
        assert!(!knowledge.has_resource_at(&pos));
    }
}
