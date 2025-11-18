// src/world/territory.rs
//! Territory claiming and ownership system for agents
//!
//! This module allows agents to claim land areas for their exclusive use,
//! organizing settlement development and preventing building conflicts.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub type Position = (i32, i32, i32);
pub type AgentId = u32;
pub type TerritoryId = u32;

/// Maximum radius for a single territory claim
const MAX_TERRITORY_RADIUS: i32 = 50;

/// Maximum number of territories an agent can own
const MAX_TERRITORIES_PER_AGENT: usize = 5;

/// Result of attempting to claim territory
#[derive(Debug, Clone, PartialEq)]
pub enum TerritoryClaimResult {
    /// Successfully claimed territory with the given ID
    Success(TerritoryId),
    /// Claim conflicts with existing territory
    Conflict(TerritoryId),
    /// Requested territory is too large
    TooLarge,
    /// Agent already owns too many territories
    TooManyTerritories,
}

/// A claimed territory owned by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Territory {
    id: TerritoryId,
    owner: AgentId,
    center: Position,
    radius: i32,
}

impl Territory {
    pub fn new(id: TerritoryId, owner: AgentId, center: Position, radius: i32) -> Self {
        Self {
            id,
            owner,
            center,
            radius,
        }
    }

    pub fn id(&self) -> TerritoryId {
        self.id
    }

    pub fn owner(&self) -> AgentId {
        self.owner
    }

    pub fn center(&self) -> Position {
        self.center
    }

    pub fn radius(&self) -> i32 {
        self.radius
    }

    pub fn set_owner(&mut self, new_owner: AgentId) {
        self.owner = new_owner;
    }

    pub fn set_radius(&mut self, new_radius: i32) {
        self.radius = new_radius;
    }

    /// Check if a position is within this territory (using circular boundary)
    pub fn contains(&self, position: Position) -> bool {
        let dx = (position.0 - self.center.0) as f32;
        let dy = (position.1 - self.center.1) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        distance <= self.radius as f32
    }

    /// Check if this territory overlaps with another
    pub fn overlaps(&self, other: &Territory) -> bool {
        let dx = (self.center.0 - other.center.0) as f32;
        let dy = (self.center.1 - other.center.1) as f32;
        let center_distance = (dx * dx + dy * dy).sqrt();
        center_distance < (self.radius + other.radius) as f32
    }
}

/// Manages all territories in the world
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TerritoryManager {
    territories: Vec<Territory>,
    next_id: TerritoryId,
    // Cache for quick ownership lookups
    owner_cache: HashMap<Position, AgentId>,
}

impl TerritoryManager {
    pub fn new() -> Self {
        Self {
            territories: Vec::new(),
            next_id: 0,
            owner_cache: HashMap::new(),
        }
    }

    /// Attempt to claim a new territory
    pub fn claim_territory(
        &mut self,
        agent_id: AgentId,
        center: Position,
        radius: i32,
    ) -> TerritoryClaimResult {
        // Check size limits
        if radius > MAX_TERRITORY_RADIUS {
            return TerritoryClaimResult::TooLarge;
        }

        // Check agent's territory count
        let agent_territory_count = self.territories.iter()
            .filter(|t| t.owner == agent_id)
            .count();

        if agent_territory_count >= MAX_TERRITORIES_PER_AGENT {
            return TerritoryClaimResult::TooManyTerritories;
        }

        // Create temporary territory to check for conflicts
        let new_territory = Territory::new(self.next_id, agent_id, center, radius);

        // Check for overlaps with existing territories
        for existing in &self.territories {
            if new_territory.overlaps(existing) {
                return TerritoryClaimResult::Conflict(existing.id);
            }
        }

        // Claim is valid - add territory
        let territory_id = self.next_id;
        self.next_id += 1;

        self.territories.push(new_territory);
        self.rebuild_owner_cache();

        TerritoryClaimResult::Success(territory_id)
    }

    /// Get all territories
    pub fn get_all_territories(&self) -> &[Territory] {
        &self.territories
    }

    /// Get all territories owned by a specific agent
    pub fn get_territories_for_agent(&self, agent_id: AgentId) -> Vec<&Territory> {
        self.territories.iter()
            .filter(|t| t.owner == agent_id)
            .collect()
    }

    /// Get the owner of a specific position
    pub fn get_owner_at(&self, position: Position) -> Option<AgentId> {
        // First check cache
        if let Some(&owner) = self.owner_cache.get(&position) {
            return Some(owner);
        }

        // Check all territories
        for territory in &self.territories {
            if territory.contains(position) {
                return Some(territory.owner);
            }
        }

        None
    }

    /// Get a specific territory by ID
    pub fn get_territory(&self, territory_id: TerritoryId) -> Option<&Territory> {
        self.territories.iter().find(|t| t.id == territory_id)
    }

    /// Get a mutable reference to a specific territory
    fn get_territory_mut(&mut self, territory_id: TerritoryId) -> Option<&mut Territory> {
        self.territories.iter_mut().find(|t| t.id == territory_id)
    }

    /// Expand a territory's radius
    pub fn expand_territory(&mut self, territory_id: TerritoryId, additional_radius: i32) -> Result<(), String> {
        // First, get the territory data without holding a mutable reference
        let (current_radius, owner, center) = {
            let territory = self.get_territory(territory_id)
                .ok_or_else(|| "Territory not found".to_string())?;
            (territory.radius, territory.owner, territory.center)
        };

        let new_radius = current_radius + additional_radius;

        if new_radius > MAX_TERRITORY_RADIUS {
            return Err("Expansion would exceed maximum radius".to_string());
        }

        // Check if expansion would cause overlaps
        let expanded = Territory::new(territory_id, owner, center, new_radius);

        for existing in &self.territories {
            if existing.id != territory_id && expanded.overlaps(existing) {
                return Err("Expansion would overlap with another territory".to_string());
            }
        }

        // Expansion is valid - now mutate
        let territory = self.get_territory_mut(territory_id).unwrap();
        territory.set_radius(new_radius);
        self.rebuild_owner_cache();

        Ok(())
    }

    /// Abandon a territory (remove it)
    pub fn abandon_territory(&mut self, territory_id: TerritoryId) -> Result<(), String> {
        let index = self.territories.iter()
            .position(|t| t.id == territory_id)
            .ok_or_else(|| "Territory not found".to_string())?;

        self.territories.remove(index);
        self.rebuild_owner_cache();

        Ok(())
    }

    /// Transfer ownership of a territory to another agent
    pub fn transfer_territory(&mut self, territory_id: TerritoryId, new_owner: AgentId) -> Result<(), String> {
        let territory = self.get_territory_mut(territory_id)
            .ok_or_else(|| "Territory not found".to_string())?;

        territory.set_owner(new_owner);
        self.rebuild_owner_cache();

        Ok(())
    }

    /// Check if an agent can build at a specific position
    pub fn can_build_at(&self, agent_id: AgentId, position: Position) -> bool {
        match self.get_owner_at(position) {
            None => true, // Unowned territory - anyone can build
            Some(owner) => owner == agent_id, // Can only build in own territory
        }
    }

    /// Get the territory bonus for placing a building at a position
    /// Returns a bonus score if the agent owns the territory
    pub fn get_territory_bonus(&self, position: Position, agent_id: AgentId) -> f32 {
        match self.get_owner_at(position) {
            Some(owner) if owner == agent_id => {
                // Strong bonus for building in own territory
                100.0
            }
            Some(_) => {
                // Penalty for building in someone else's territory
                -200.0
            }
            None => {
                // Neutral for unowned territory
                0.0
            }
        }
    }

    /// Rebuild the owner cache for faster lookups
    fn rebuild_owner_cache(&mut self) {
        self.owner_cache.clear();

        // Cache key positions for each territory (center and cardinal points)
        for territory in &self.territories {
            let center = territory.center;
            let radius = territory.radius;
            let owner = territory.owner;

            // Cache center
            self.owner_cache.insert(center, owner);

            // Cache cardinal directions at various distances
            for dist in (1..=radius).step_by(2) {
                self.owner_cache.insert((center.0 + dist, center.1, center.2), owner);
                self.owner_cache.insert((center.0 - dist, center.1, center.2), owner);
                self.owner_cache.insert((center.0, center.1 + dist, center.2), owner);
                self.owner_cache.insert((center.0, center.1 - dist, center.2), owner);
            }
        }
    }

    /// Clear all territories
    pub fn clear(&mut self) {
        self.territories.clear();
        self.owner_cache.clear();
        self.next_id = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_territory_creation() {
        let territory = Territory::new(0, 1, (10, 10, 0), 5);
        assert_eq!(territory.id(), 0);
        assert_eq!(territory.owner(), 1);
        assert_eq!(territory.center(), (10, 10, 0));
        assert_eq!(territory.radius(), 5);
    }

    #[test]
    fn test_territory_contains() {
        let territory = Territory::new(0, 1, (10, 10, 0), 5);

        assert!(territory.contains((10, 10, 0)), "Center should be in territory");
        assert!(territory.contains((13, 10, 0)), "Point within radius should be in territory");
        assert!(!territory.contains((20, 10, 0)), "Point outside radius should not be in territory");
    }

    #[test]
    fn test_territory_overlap() {
        let territory1 = Territory::new(0, 1, (10, 10, 0), 5);
        let territory2 = Territory::new(1, 2, (20, 10, 0), 5);
        let territory3 = Territory::new(2, 3, (12, 10, 0), 5);

        assert!(!territory1.overlaps(&territory2), "Separate territories should not overlap");
        assert!(territory1.overlaps(&territory3), "Close territories should overlap");
    }

    #[test]
    fn test_manager_claim_territory() {
        let mut manager = TerritoryManager::new();

        let result = manager.claim_territory(1, (10, 10, 0), 5);
        assert!(matches!(result, TerritoryClaimResult::Success(_)));
    }

    #[test]
    fn test_manager_prevent_overlap() {
        let mut manager = TerritoryManager::new();

        manager.claim_territory(1, (10, 10, 0), 5);
        let result = manager.claim_territory(2, (12, 10, 0), 5);

        assert!(matches!(result, TerritoryClaimResult::Conflict(_)));
    }

    #[test]
    fn test_manager_size_limit() {
        let mut manager = TerritoryManager::new();

        let result = manager.claim_territory(1, (10, 10, 0), 1000);
        assert!(matches!(result, TerritoryClaimResult::TooLarge));
    }
}
