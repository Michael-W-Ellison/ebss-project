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
    /// Discovered a storage container or stockpile
    Storage {
        storage_type: String,
        position: Position,
        capacity: f32,
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
    /// Discovered storage positions (position -> (storage type, capacity))
    pub known_storage: HashMap<Position, (String, f32)>,
    /// Terrain types encountered
    pub encountered_terrains: HashSet<TerrainType>,
    /// History of discoveries
    pub discoveries: Vec<Discovery>,
    /// Total tiles explored
    pub total_tiles_explored: usize,
    /// Last exploration tick
    pub last_exploration_tick: u32,
    /// Curiosity-driven exploration count
    pub curiosity_driven_explorations: u32,
    /// Total curiosity satisfaction gained from discoveries
    pub total_curiosity_satisfaction: f32,
}

impl ExplorationKnowledge {
    pub fn new() -> Self {
        Self {
            explored_tiles: HashSet::new(),
            known_resources: HashMap::new(),
            known_buildings: HashMap::new(),
            known_storage: HashMap::new(),
            encountered_terrains: HashSet::new(),
            discoveries: Vec::new(),
            total_tiles_explored: 0,
            last_exploration_tick: 0,
            curiosity_driven_explorations: 0,
            total_curiosity_satisfaction: 0.0,
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
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(e) = self.known_resources.entry(position) {
            e.insert(resource_type);

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
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(e) = self.known_buildings.entry(position) {
            e.insert(building_type);

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

    /// Discover a storage container at a position
    pub fn discover_storage(
        &mut self,
        position: Position,
        storage_type: String,
        capacity: f32,
        current_tick: u32,
    ) -> bool {
        use std::collections::hash_map::Entry;
        if let Entry::Vacant(e) = self.known_storage.entry(position) {
            e.insert((storage_type.clone(), capacity));

            // Record discovery
            self.discoveries.push(Discovery {
                discovery_type: DiscoveryType::Storage {
                    storage_type,
                    position,
                    capacity,
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

    /// Record a curiosity-driven exploration action and return satisfaction gained
    pub fn record_curiosity_exploration(&mut self, discovery_type: &DiscoveryType) -> f32 {
        self.curiosity_driven_explorations += 1;
        let satisfaction = calculate_exploration_reward(discovery_type);
        self.total_curiosity_satisfaction += satisfaction;
        satisfaction
    }

    /// Get the average curiosity satisfaction per discovery
    pub fn average_curiosity_satisfaction(&self) -> f32 {
        if self.discoveries.is_empty() {
            0.0
        } else {
            self.total_curiosity_satisfaction / self.discoveries.len() as f32
        }
    }

    /// Get exploration efficiency (satisfaction per exploration action)
    pub fn exploration_efficiency(&self) -> f32 {
        if self.curiosity_driven_explorations == 0 {
            0.0
        } else {
            self.total_curiosity_satisfaction / self.curiosity_driven_explorations as f32
        }
    }

    /// Get discoveries by type count
    pub fn discoveries_by_type(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();

        for discovery in &self.discoveries {
            let type_name = match &discovery.discovery_type {
                DiscoveryType::Terrain(_) => "Terrain",
                DiscoveryType::Resource { .. } => "Resource",
                DiscoveryType::Building { .. } => "Building",
                DiscoveryType::AreaExplored { .. } => "Area",
                DiscoveryType::Storage { .. } => "Storage",
            };
            *counts.entry(type_name.to_string()).or_insert(0) += 1;
        }

        counts
    }

    // ===== Fog of War Methods =====

    /// Reveal all tiles within a given visibility radius around a position
    ///
    /// This simulates the agent's line of sight. All tiles within the radius
    /// are marked as explored. Returns the number of newly explored tiles.
    pub fn reveal_in_radius(&mut self, center: Position, radius: u32, current_tick: u32) -> usize {
        let mut newly_explored = 0;

        for dx in -(radius as i32)..=(radius as i32) {
            for dy in -(radius as i32)..=(radius as i32) {
                // Use circular vision (Euclidean distance check)
                let dist_sq = (dx * dx + dy * dy) as u32;
                if dist_sq <= radius * radius {
                    let pos = Position::new(center.x + dx, center.y + dy);
                    if self.explore_tile(pos, current_tick) {
                        newly_explored += 1;
                    }
                }
            }
        }

        newly_explored
    }

    /// Get all tiles currently visible from a position
    ///
    /// Returns positions within the visibility radius. This does NOT mark
    /// them as explored - use `reveal_in_radius` for that.
    pub fn visible_tiles(&self, center: Position, visibility_radius: u32) -> Vec<Position> {
        let mut visible = Vec::new();

        for dx in -(visibility_radius as i32)..=(visibility_radius as i32) {
            for dy in -(visibility_radius as i32)..=(visibility_radius as i32) {
                let dist_sq = (dx * dx + dy * dy) as u32;
                if dist_sq <= visibility_radius * visibility_radius {
                    visible.push(Position::new(center.x + dx, center.y + dy));
                }
            }
        }

        visible
    }

    /// Check if a position is currently visible from the agent's position
    pub fn is_visible(&self, from: Position, target: Position, visibility_radius: u32) -> bool {
        from.distance_to(&target) <= visibility_radius
    }

    /// Get visibility status for a set of positions
    ///
    /// Returns a map of positions to their visibility status:
    /// - `Visible` - Currently in line of sight
    /// - `Explored` - Previously seen but not currently visible
    /// - `Unexplored` - Never seen (fog of war)
    pub fn visibility_status(
        &self,
        viewer_pos: Position,
        visibility_radius: u32,
        positions: &[Position],
    ) -> HashMap<Position, VisibilityStatus> {
        positions
            .iter()
            .map(|pos| {
                let status = if self.is_visible(viewer_pos, *pos, visibility_radius) {
                    VisibilityStatus::Visible
                } else if self.is_explored(pos) {
                    VisibilityStatus::Explored
                } else {
                    VisibilityStatus::Unexplored
                };
                (*pos, status)
            })
            .collect()
    }
}

/// Visibility status for fog of war
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityStatus {
    /// Currently visible (in line of sight)
    Visible,
    /// Previously explored but not currently visible
    Explored,
    /// Never seen (complete fog of war)
    Unexplored,
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
        DiscoveryType::Storage { capacity, .. } => {
            // Storage discovery reward scales with capacity
            // Full storage is more interesting (0.2 base + 0.15 capacity bonus)
            0.2 + (capacity * 0.15)
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

    #[test]
    fn test_discover_storage() {
        let mut knowledge = ExplorationKnowledge::new();
        let pos = Position::new(15, 15);

        // First discovery should be new
        assert!(knowledge.discover_storage(pos, "Chest".to_string(), 0.8, 100));
        assert_eq!(knowledge.known_storage.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);

        // Verify the storage was recorded correctly
        let (storage_type, capacity) = knowledge.known_storage.get(&pos).unwrap();
        assert_eq!(storage_type, "Chest");
        assert_eq!(*capacity, 0.8);

        // Second discovery at same position should not be new
        assert!(!knowledge.discover_storage(pos, "Chest".to_string(), 0.8, 101));
        assert_eq!(knowledge.known_storage.len(), 1);
        assert_eq!(knowledge.discoveries.len(), 1);
    }

    #[test]
    fn test_storage_exploration_reward() {
        // Empty storage
        let empty_storage = DiscoveryType::Storage {
            storage_type: "Box".to_string(),
            position: Position::new(0, 0),
            capacity: 0.0,
        };
        let reward_empty = calculate_exploration_reward(&empty_storage);
        assert_eq!(reward_empty, 0.2); // Base reward only

        // Half-full storage
        let half_storage = DiscoveryType::Storage {
            storage_type: "Barrel".to_string(),
            position: Position::new(0, 0),
            capacity: 0.5,
        };
        let reward_half = calculate_exploration_reward(&half_storage);
        assert!(reward_half > reward_empty);
        assert_eq!(reward_half, 0.275); // 0.2 + 0.5 * 0.15

        // Full storage
        let full_storage = DiscoveryType::Storage {
            storage_type: "Warehouse".to_string(),
            position: Position::new(0, 0),
            capacity: 1.0,
        };
        let reward_full = calculate_exploration_reward(&full_storage);
        assert!(reward_full > reward_half);
        assert!((reward_full - 0.35).abs() < 0.001); // 0.2 + 1.0 * 0.15
    }

    #[test]
    fn test_curiosity_driven_exploration_tracking() {
        let mut knowledge = ExplorationKnowledge::new();

        // Make a resource discovery
        let discovery = DiscoveryType::Resource {
            resource_type: ResourceType::Wood,
            position: Position::new(5, 5),
        };

        let satisfaction = knowledge.record_curiosity_exploration(&discovery);

        assert_eq!(knowledge.curiosity_driven_explorations, 1);
        assert_eq!(satisfaction, 0.3); // Resource reward
        assert_eq!(knowledge.total_curiosity_satisfaction, 0.3);
    }

    #[test]
    fn test_exploration_efficiency() {
        let mut knowledge = ExplorationKnowledge::new();

        // Record multiple explorations with varying rewards
        let discovery1 = DiscoveryType::Terrain(TerrainType::Forest);
        let discovery2 = DiscoveryType::Resource {
            resource_type: ResourceType::Stone,
            position: Position::new(3, 3),
        };

        knowledge.record_curiosity_exploration(&discovery1);
        knowledge.record_curiosity_exploration(&discovery2);

        // Efficiency should be total satisfaction / explorations
        let expected_efficiency = (0.1 + 0.3) / 2.0;
        assert_eq!(knowledge.exploration_efficiency(), expected_efficiency);
    }

    #[test]
    fn test_discoveries_by_type() {
        let mut knowledge = ExplorationKnowledge::new();

        // Add various discoveries
        knowledge.discover_resource(Position::new(1, 1), ResourceType::Wood, 0);
        knowledge.discover_resource(Position::new(2, 2), ResourceType::Stone, 1);
        knowledge.discover_building(Position::new(3, 3), BuildingType::SmallHouse, 2);
        knowledge.discover_storage(Position::new(4, 4), "Chest".to_string(), 0.5, 3);
        knowledge.encounter_terrain(TerrainType::Forest, Position::new(5, 5), 4);

        let counts = knowledge.discoveries_by_type();

        assert_eq!(*counts.get("Resource").unwrap(), 2);
        assert_eq!(*counts.get("Building").unwrap(), 1);
        assert_eq!(*counts.get("Storage").unwrap(), 1);
        assert_eq!(*counts.get("Terrain").unwrap(), 1);
    }

    #[test]
    fn test_average_curiosity_satisfaction() {
        let mut knowledge = ExplorationKnowledge::new();

        // Use record_curiosity_exploration to properly track satisfaction
        let discovery1 = DiscoveryType::Resource {
            resource_type: ResourceType::Wood,
            position: Position::new(1, 1),
        };
        let discovery2 = DiscoveryType::Building {
            building_type: BuildingType::SmallHouse,
            position: Position::new(2, 2),
        };

        knowledge.record_curiosity_exploration(&discovery1); // Adds 0.3 to total_curiosity_satisfaction
        knowledge.record_curiosity_exploration(&discovery2); // Adds 0.2 to total_curiosity_satisfaction

        // Record the actual discoveries (this adds to discoveries vec)
        knowledge.discover_resource(Position::new(1, 1), ResourceType::Wood, 0);
        knowledge.discover_building(Position::new(2, 2), BuildingType::SmallHouse, 1);

        // Average is total_satisfaction (0.5) / discoveries.len() (2) = 0.25
        let avg_satisfaction = knowledge.average_curiosity_satisfaction();
        assert!((avg_satisfaction - 0.25).abs() < 0.001);
    }
}
