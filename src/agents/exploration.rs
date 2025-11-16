// src/agents/exploration.rs
//! Agent exploration and map discovery system.
//!
//! Tracks what each agent has discovered about the world including:
//! - Explored tiles (fog of war)
//! - Discovered resources
//! - Discovered buildings
//! - Terrain types encountered

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use uuid::Uuid;
use crate::world::{Position, TerrainType, ResourceType, BuildingType};

/// Types of discoveries agents can make
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryType {
    /// Discovered a new terrain type
    Terrain(TerrainType),
    /// Discovered a resource node
    Resource {
        resource_type: ResourceType,
        position: Position,
    },
    /// Discovered a building
    Building {
        building_type: BuildingType,
        position: Position,
    },
    /// Explored a new area (milestone)
    AreaExplored {
        tiles_count: usize,
    },
}

/// A single discovery event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discovery {
    pub discovery_type: DiscoveryType,
    pub tick: u32,
    pub position: Position,
}

/// Agent's exploration knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationKnowledge {
    /// Set of explored tile positions
    pub explored_tiles: HashSet<Position>,
    /// Discovered resource positions (position -> resource type)
    pub known_resources: HashMap<Position, ResourceType>,
    /// Discovered building positions (position -> building type)
    pub known_buildings: HashMap<Position, BuildingType>,
    /// Terrain types encountered
    pub encountered_terrains: HashSet<TerrainType>,
    /// History of discoveries
    pub discoveries: Vec<Discovery>,
    /// Total tiles explored
    pub total_tiles_explored: usize,
    /// Last exploration tick
    pub last_exploration_tick: u32,
}

impl ExplorationKnowledge {
    pub fn new() -> Self {
        Self {
            explored_tiles: HashSet::new(),
            known_resources: HashMap::new(),
            known_buildings: HashMap::new(),
            encountered_terrains: HashSet::new(),
            discoveries: Vec::new(),
            total_tiles_explored: 0,
            last_exploration_tick: 0,
        }
    }

    /// Mark a tile as explored and return true if it's a new discovery
    pub fn explore_tile(&mut self, position: Position, current_tick: u32) -> bool {
        self.last_exploration_tick = current_tick;
        if self.explored_tiles.insert(position) {
            self.total_tiles_explored += 1;
            true
        } else {
            false
        }
    }

    /// Discover a resource at a position
    pub fn discover_resource(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        current_tick: u32,
    ) -> bool {
        if !self.known_resources.contains_key(&position) {
            self.known_resources.insert(position, resource_type);

            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Resource {
                    resource_type,
                    position,
                },
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Discover a building at a position
    pub fn discover_building(
        &mut self,
        position: Position,
        building_type: BuildingType,
        current_tick: u32,
    ) -> bool {
        if !self.known_buildings.contains_key(&position) {
            self.known_buildings.insert(position, building_type);

            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Building {
                    building_type,
                    position,
                },
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Encounter a new terrain type
    pub fn encounter_terrain(
        &mut self,
        terrain_type: TerrainType,
        position: Position,
        current_tick: u32,
    ) -> bool {
        if self.encountered_terrains.insert(terrain_type) {
            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Terrain(terrain_type),
                tick: current_tick,
                position,
            });

            true
        } else {
            false
        }
    }

    /// Check if a tile has been explored
    pub fn is_explored(&self, position: &Position) -> bool {
        self.explored_tiles.contains(position)
    }

    /// Get number of unexplored neighbors around a position
    pub fn count_unexplored_neighbors(&self, position: &Position) -> usize {
        position.neighbors_8()
            .iter()
            .filter(|p| !self.is_explored(p))
            .count()
    }

    /// Find the nearest unexplored position from a given position
    pub fn find_nearest_unexplored(
        &self,
        from: &Position,
        search_radius: u32,
    ) -> Option<Position> {
        let mut nearest: Option<(Position, u32)> = None;

        // Search in expanding radius
        for radius in 1..=search_radius {
            for dx in -(radius as i32)..=(radius as i32) {
                for dy in -(radius as i32)..=(radius as i32) {
                    if dx.abs() + dy.abs() > radius as i32 {
                        continue;
                    }

                    let pos = Position::new(from.x + dx, from.y + dy);

                    if !self.is_explored(&pos) {
                        let distance = from.distance_to(&pos);

                        match nearest {
                            None => nearest = Some((pos, distance)),
                            Some((_, current_dist)) if distance < current_dist => {
                                nearest = Some((pos, distance));
                            }
                            _ => {}
                        }
                    }
                }
            }

            // If we found something in this radius, return it
            if let Some((pos, _)) = nearest {
                return Some(pos);
            }
        }

        nearest.map(|(pos, _)| pos)
    }

    /// Get exploration percentage (requires world size)
    pub fn exploration_percentage(&self, total_world_tiles: usize) -> f32 {
        if total_world_tiles == 0 {
            return 0.0;
        }
        (self.total_tiles_explored as f32 / total_world_tiles as f32) * 100.0
    }

    /// Get recent discoveries (last N)
    pub fn recent_discoveries(&self, count: usize) -> Vec<&Discovery> {
        let start = if self.discoveries.len() > count {
            self.discoveries.len() - count
        } else {
            0
        };

        self.discoveries[start..].iter().collect()
    }
}

impl Default for ExplorationKnowledge {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate exploration reward (curiosity satisfaction) based on discovery
pub fn calculate_exploration_reward(discovery: &DiscoveryType) -> f32 {
    match discovery {
        DiscoveryType::Terrain(_) => 0.1,  // New terrain type
        DiscoveryType::Resource { .. } => 0.3,  // Resource discovery is very rewarding
        DiscoveryType::Building { .. } => 0.2,  // Building discovery
        DiscoveryType::AreaExplored { tiles_count } => {
            // Scale reward with area size
            (*tiles_count as f32 * 0.01).min(0.5)
        }
    }
}

/// Determine if an agent should explore based on their state
pub fn should_explore(
    curiosity_drive: f32,
    unexplored_nearby: usize,
    last_exploration_ticks_ago: u32,
) -> bool {
    // High curiosity drive makes exploration more likely
    if curiosity_drive > 0.6 {
        return true;
    }

    // Many unexplored tiles nearby and moderate curiosity
    if unexplored_nearby > 5 && curiosity_drive > 0.3 {
        return true;
    }

    // Haven't explored in a while and some curiosity
    if last_exploration_ticks_ago > 1000 && curiosity_drive > 0.2 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exploration_knowledge_creation() {
        let knowledge = ExplorationKnowledge::new();
        assert_eq!(knowledge.total_tiles_explored, 0);
        assert_eq!(knowledge.explored_tiles.len(), 0);
    }

    #[test]
    fn test_explore_tile() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(5, 5);

        // First exploration should be new
        assert!(knowledge.explore_tile(pos, 0));
        assert_eq!(knowledge.total_tiles_explored, 1);

        // Second exploration of same tile should not be new
        assert!(!knowledge.explore_tile(pos, 1));
        assert_eq!(knowledge.total_tiles_explored, 1);
    }

    #[test]
    fn test_discover_resource() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(10, 10);

        // First discovery should be new
        assert!(knowledge.discover_resource(pos, ResourceType::Wood, 0));
        assert_eq!(knowledge.known_resources.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);

        // Second discovery at same position should not be new
        assert!(!knowledge.discover_resource(pos, ResourceType::Wood, 1));
        assert_eq!(knowledge.known_resources.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);
    }

    #[test]
    fn test_terrain_encounter() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(0, 0);

        // First encounter should be new
        assert!(knowledge.encounter_terrain(TerrainType::Forest, pos, 0));
        assert_eq!(knowledge.encountered_terrains.len(), 1);

        // Second encounter of same terrain should not be new
        assert!(!knowledge.encounter_terrain(TerrainType::Forest, pos, 1));
        assert_eq!(knowledge.encountered_terrains.len(), 1);

        // Different terrain should be new
        assert!(knowledge.encounter_terrain(TerrainType::Mountain, pos, 2));
        assert_eq!(knowledge.encountered_terrains.len(), 2);
    }

    #[test]
    fn test_is_explored() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(3, 7);

        assert!(!knowledge.is_explored(&pos));
        knowledge.explore_tile(pos, 0);
        assert!(knowledge.is_explored(&pos));
    }

    #[test]
    fn test_count_unexplored_neighbors() {
        let mut knowledge = ExplorationKnowledge::new();
        let center = Position::new(5, 5);

        // All 8 neighbors should be unexplored initially
        assert_eq!(knowledge.count_unexplored_neighbors(&center), 8);

        // Explore one neighbor
        knowledge.explore_tile(Position::new(6, 5), 0);
        assert_eq!(knowledge.count_unexplored_neighbors(&center), 7);

        // Explore all neighbors
        for neighbor in center.neighbors_8() {
            knowledge.explore_tile(neighbor, 0);
        }
        assert_eq!(knowledge.count_unexplored_neighbors(&center), 0);
    }

    #[test]
    fn test_exploration_percentage() {
        let mut knowledge = ExplorationKnowledge::new();

        // Explore 50 tiles in a 100-tile world
        for i in 0..50 {
            knowledge.explore_tile(Position::new(i, 0), 0);
        }

        assert_eq!(knowledge.exploration_percentage(100), 50.0);
        assert_eq!(knowledge.exploration_percentage(200), 25.0);
    }

    #[test]
    fn test_should_explore() {
        // High curiosity should trigger exploration
        assert!(should_explore(0.7, 0, 0));

        // Moderate curiosity with many unexplored tiles
        assert!(should_explore(0.4, 10, 0));

        // Low curiosity but haven't explored in a while
        assert!(should_explore(0.3, 0, 1500));

        // Low curiosity, few unexplored, recent exploration
        assert!(!should_explore(0.1, 2, 50));
    }
}
