// src/world/spatial_planning.rs
//! Spatial planning and intelligent building placement system
//!
//! This module implements algorithms for finding optimal building locations
//! that minimize travel time and maximize production efficiency.

use super::{World, BuildingType};
use std::collections::HashMap;

/// Position as (x, y, z) tuple for spatial planning
pub type Position = (i32, i32, i32);

/// Placement strategy for building location selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementStrategy {
    /// Prioritize proximity to agent's current position
    NearAgent,
    /// Prioritize proximity to required resources
    NearResources,
    /// Balance between agent proximity and resource/building proximity
    BalancedProximity,
    /// Find nearest available unoccupied spot
    NearestAvailable,
}

/// Criteria for evaluating building placement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementCriteria {
    /// Near a specific resource type
    NearResource(String),
    /// Near buildings in the production chain
    NearRelatedBuilding,
    /// Near existing settlement buildings
    NearSettlement,
    /// Centrally located relative to all buildings
    CentralToSettlement,
}

/// Spatial planner for intelligent building placement
pub struct SpatialPlanner<'a> {
    world: &'a World,
    resource_locations: HashMap<String, Vec<Position>>,
    building_locations: HashMap<BuildingType, Vec<Position>>,
}

impl<'a> SpatialPlanner<'a> {
    /// Create a new spatial planner for the given world
    pub fn new(world: &'a World) -> Self {
        let mut planner = Self {
            world,
            resource_locations: HashMap::new(),
            building_locations: HashMap::new(),
        };

        planner.index_world();
        planner
    }

    /// Check if planner is properly initialized
    pub fn is_initialized(&self) -> bool {
        true // Always initialized after new()
    }

    /// Index the world's resources and buildings for fast lookup
    fn index_world(&mut self) {
        // Index resource nodes (already stored as tuples)
        for (resource_type, positions) in &self.world.resource_nodes {
            self.resource_locations.insert(
                resource_type.clone(),
                positions.clone()
            );
        }

        // Index buildings by type, converting Position to tuple
        for building in &self.world.buildings {
            let pos = (building.position.x, building.position.y, 0);
            self.building_locations
                .entry(building.building_type)
                .or_insert_with(Vec::new)
                .push(pos);
        }
    }

    /// Find optimal location for a building based on criteria
    pub fn find_optimal_location(
        &self,
        building_type: BuildingType,
        criteria: PlacementCriteria,
    ) -> Option<Position> {
        self.find_optimal_location_with_spacing(building_type, criteria, 0)
    }

    /// Find optimal location with minimum spacing requirement
    pub fn find_optimal_location_with_spacing(
        &self,
        building_type: BuildingType,
        criteria: PlacementCriteria,
        min_spacing: i32,
    ) -> Option<Position> {
        let mut best_pos: Option<Position> = None;
        let mut best_score = f32::MIN;

        // Search in a reasonable radius
        let search_radius = 50;
        let center = self.get_search_center(&criteria);

        // Determine Z-level search range based on building type
        let z_range = self.get_z_level_range(building_type, center.2);

        for x in (center.0 - search_radius)..=(center.0 + search_radius) {
            for y in (center.1 - search_radius)..=(center.1 + search_radius) {
                for z in z_range.0..=z_range.1 {
                    let pos = (x, y, z);

                    // Skip if occupied or impassable
                    let grid_pos = crate::world::grid::Position::new(x, y);
                    if self.world.is_position_occupied(&grid_pos) {
                        continue;
                    }
                    if !self.world.is_terrain_passable(pos) {
                        continue;
                    }

                    // Check minimum spacing
                    if min_spacing > 0 && !self.check_spacing(pos, min_spacing) {
                        continue;
                    }

                    // Score this location (includes elevation preference)
                    let mut score = self.score_location(pos, building_type, criteria.clone());

                    // Apply elevation scoring
                    score += self.score_elevation(pos, building_type, center.2);

                    if score > best_score {
                        best_score = score;
                        best_pos = Some(pos);
                    }
                }
            }
        }

        best_pos
    }

    /// Get the Z-level search range for a building type
    fn get_z_level_range(&self, building_type: BuildingType, center_z: i32) -> (i32, i32) {
        match building_type {
            // Farms prefer flat, low-lying areas
            BuildingType::Farm | BuildingType::AnimalPen => {
                (center_z.saturating_sub(5), center_z + 2)
            }
            // Defensive structures prefer high ground
            BuildingType::GuardPost | BuildingType::TownCenter => {
                (center_z, center_z + 10)
            }
            // Religious buildings often on elevated positions
            BuildingType::Temple | BuildingType::Shrine => {
                (center_z, center_z + 8)
            }
            // Mills need consistent water flow (lower elevation)
            BuildingType::Mill => {
                (center_z.saturating_sub(3), center_z + 1)
            }
            // Most buildings are flexible within a reasonable range
            _ => (center_z.saturating_sub(3), center_z + 3)
        }
    }

    /// Score elevation preference for building placement
    fn score_elevation(&self, pos: Position, building_type: BuildingType, reference_z: i32) -> f32 {
        let elevation_diff = pos.2 - reference_z;

        match building_type {
            // Guard posts and defensive structures benefit from high ground
            BuildingType::GuardPost | BuildingType::TownCenter => {
                (elevation_diff as f32 * 2.0).max(0.0) // Bonus for higher elevation
            }
            // Temples prefer elevated positions for visibility
            BuildingType::Temple | BuildingType::Shrine => {
                (elevation_diff as f32 * 1.5).max(0.0)
            }
            // Farms and mills prefer lower, flatter ground
            BuildingType::Farm | BuildingType::AnimalPen | BuildingType::Mill => {
                if elevation_diff.abs() <= 1 {
                    5.0 // Bonus for flat terrain
                } else {
                    -(elevation_diff.abs() as f32 * 2.0) // Penalty for elevation changes
                }
            }
            // Storage buildings prefer accessible (moderate) elevations
            BuildingType::Storehouse | BuildingType::TownStorage => {
                if elevation_diff.abs() <= 2 {
                    3.0
                } else {
                    -(elevation_diff.abs() as f32)
                }
            }
            // Most buildings prefer staying near reference elevation
            _ => {
                -(elevation_diff.abs() as f32 * 0.5) // Small penalty for elevation changes
            }
        }
    }

    /// Find optimal location considering agent's position.
    ///
    /// This is a convenience method that infers placement criteria from the building type.
    /// For more control, use [`find_optimal_location_with_criteria`] to specify explicit criteria.
    ///
    /// # Arguments
    /// * `building_type` - The type of building to place (used to infer criteria)
    /// * `agent_pos` - The agent's current position (used for distance calculations)
    /// * `strategy` - The placement strategy to use
    ///
    /// # Returns
    /// The optimal position for the building, or `None` if no valid location exists.
    pub fn find_optimal_location_for_agent(
        &self,
        building_type: BuildingType,
        agent_pos: Position,
        strategy: PlacementStrategy,
    ) -> Option<Position> {
        let criteria = self.infer_criteria_from_building(building_type);
        self.find_optimal_location_with_criteria(building_type, agent_pos, strategy, criteria)
    }

    /// Find optimal location for an agent with territory bonus consideration.
    ///
    /// Similar to `find_optimal_location_for_agent` but includes a territory ownership bonus.
    /// Agents prefer to build within their own territory.
    ///
    /// # Arguments
    /// * `building_type` - The type of building to place
    /// * `agent_pos` - The agent's current position
    /// * `strategy` - The placement strategy to use
    /// * `agent_id` - The agent's ID for territory ownership checking
    ///
    /// # Returns
    /// The optimal position for the building, preferring owned territory.
    pub fn find_optimal_location_with_territory(
        &self,
        building_type: BuildingType,
        agent_pos: Position,
        strategy: PlacementStrategy,
        agent_id: u32,
    ) -> Option<Position> {
        let criteria = self.infer_criteria_from_building(building_type);
        let mut best_pos: Option<Position> = None;
        let mut best_score = f32::MIN;

        let search_radius = match strategy {
            PlacementStrategy::NearAgent => 15,
            PlacementStrategy::NearestAvailable => 10,
            _ => 30,
        };

        for x in (agent_pos.0 - search_radius)..=(agent_pos.0 + search_radius) {
            for y in (agent_pos.1 - search_radius)..=(agent_pos.1 + search_radius) {
                for z in [agent_pos.2] {
                    let pos = (x, y, z);

                    let grid_pos = crate::world::grid::Position::new(x, y);
                    if self.world.is_position_occupied(&grid_pos) {
                        continue;
                    }
                    if !self.world.is_terrain_passable(pos) {
                        continue;
                    }

                    // Get base score from strategy
                    let mut score = self.score_location_for_agent_with_criteria(
                        pos,
                        agent_pos,
                        building_type,
                        strategy,
                        &criteria,
                    );

                    // Add territory bonus - strongly prefer building in owned territory
                    let territory_bonus = self.world.territory_manager.get_territory_bonus(pos, agent_id);
                    score += territory_bonus;

                    if score > best_score {
                        best_score = score;
                        best_pos = Some(pos);
                    }
                }
            }
        }

        best_pos
    }

    /// Find optimal location with explicit criteria
    pub fn find_optimal_location_with_criteria(
        &self,
        building_type: BuildingType,
        agent_pos: Position,
        strategy: PlacementStrategy,
        criteria: PlacementCriteria,
    ) -> Option<Position> {
        let mut best_pos: Option<Position> = None;
        let mut best_score = f32::MIN;

        let search_radius = match strategy {
            PlacementStrategy::NearAgent => 15,
            PlacementStrategy::NearestAvailable => 10,
            _ => 30,
        };

        for x in (agent_pos.0 - search_radius)..=(agent_pos.0 + search_radius) {
            for y in (agent_pos.1 - search_radius)..=(agent_pos.1 + search_radius) {
                for z in [agent_pos.2] {
                    let pos = (x, y, z);

                    let grid_pos = crate::world::grid::Position::new(x, y);
                    if self.world.is_position_occupied(&grid_pos) {
                        continue;
                    }
                    if !self.world.is_terrain_passable(pos) {
                        continue;
                    }

                    let score = self.score_location_for_agent_with_criteria(
                        pos,
                        agent_pos,
                        building_type,
                        strategy,
                        &criteria,
                    );

                    if score > best_score {
                        best_score = score;
                        best_pos = Some(pos);
                    }
                }
            }
        }

        best_pos
    }

    /// Infer placement criteria from building type
    fn infer_criteria_from_building(&self, building_type: BuildingType) -> PlacementCriteria {
        use BuildingType::*;
        match building_type {
            Forge | Smithy => PlacementCriteria::NearResource("iron".to_string()),
            Workshop => PlacementCriteria::NearResource("wood".to_string()),
            Mill | Bakery => PlacementCriteria::NearRelatedBuilding,
            SmallHouse | MediumHouse | LargeHouse => PlacementCriteria::NearSettlement,
            Storehouse => PlacementCriteria::CentralToSettlement,
            _ => PlacementCriteria::NearSettlement,
        }
    }

    /// Score a specific location for a building type and criteria (including zones)
    pub fn score_location_with_zones(
        &self,
        pos: Position,
        building_type: BuildingType,
        criteria: PlacementCriteria,
    ) -> f32 {
        // Get base score from placement criteria
        let mut score = self.score_location(pos, building_type, criteria.clone());

        // Add zone bonus
        let zone_bonus = self.world.zone_manager.get_zone_bonus(pos, building_type);
        score += zone_bonus;

        score
    }

    /// Score a specific location considering territory ownership
    pub fn score_location_with_territory(
        &self,
        pos: Position,
        building_type: BuildingType,
        criteria: PlacementCriteria,
        agent_id: Option<u32>,
    ) -> f32 {
        // Get base score from placement criteria
        let mut score = self.score_location(pos, building_type, criteria.clone());

        // Add zone bonus
        let zone_bonus = self.world.zone_manager.get_zone_bonus(pos, building_type);
        score += zone_bonus;

        // Add territory bonus if agent specified
        if let Some(agent) = agent_id {
            let territory_bonus = self.world.territory_manager.get_territory_bonus(pos, agent);
            score += territory_bonus;
        }

        score
    }

    /// Score a specific location for a building type and criteria (without zones)
    pub fn score_location(
        &self,
        pos: Position,
        building_type: BuildingType,
        criteria: PlacementCriteria,
    ) -> f32 {
        let mut score = 0.0;

        match criteria {
            PlacementCriteria::NearResource(resource_type) => {
                if let Some(resource_positions) = self.resource_locations.get(&resource_type) {
                    // Find closest resource
                    let min_distance = resource_positions.iter()
                        .map(|&res_pos| Self::distance(pos, res_pos))
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap_or(f32::MAX);

                    // Closer is better (inverse score)
                    score += 100.0 / (1.0 + min_distance);
                }
            }

            PlacementCriteria::NearRelatedBuilding => {
                // Find prerequisite buildings
                let prerequisites = building_type.prerequisites();

                for prereq in prerequisites {
                    if let Some(prereq_positions) = self.building_locations.get(&prereq) {
                        let min_distance = prereq_positions.iter()
                            .map(|&prereq_pos| Self::distance(pos, prereq_pos))
                            .min_by(|a, b| a.partial_cmp(b).unwrap())
                            .unwrap_or(f32::MAX);

                        // Strongly prefer being near prerequisites
                        score += 200.0 / (1.0 + min_distance);
                    }
                }

                // Also consider buildings that use this one's output
                let consumers = self.get_consumer_buildings(building_type);
                for consumer in consumers {
                    if let Some(consumer_positions) = self.building_locations.get(&consumer) {
                        let min_distance = consumer_positions.iter()
                            .map(|&consumer_pos| Self::distance(pos, consumer_pos))
                            .min_by(|a, b| a.partial_cmp(b).unwrap())
                            .unwrap_or(f32::MAX);

                        score += 150.0 / (1.0 + min_distance);
                    }
                }
            }

            PlacementCriteria::NearSettlement => {
                // Find any existing houses
                let house_types = vec![
                    BuildingType::SmallHouse,
                    BuildingType::MediumHouse,
                    BuildingType::LargeHouse,
                    BuildingType::Longhouse,
                ];

                for house_type in house_types {
                    if let Some(house_positions) = self.building_locations.get(&house_type) {
                        let avg_distance: f32 = house_positions.iter()
                            .map(|&house_pos| Self::distance(pos, house_pos))
                            .sum::<f32>() / house_positions.len() as f32;

                        score += 50.0 / (1.0 + avg_distance);
                    }
                }
            }

            PlacementCriteria::CentralToSettlement => {
                // Calculate distance to all buildings
                let all_positions: Vec<Position> = self.building_locations.values()
                    .flat_map(|positions| positions.iter().copied())
                    .collect();

                if !all_positions.is_empty() {
                    let avg_distance: f32 = all_positions.iter()
                        .map(|&building_pos| Self::distance(pos, building_pos))
                        .sum::<f32>() / all_positions.len() as f32;

                    // Central location scores higher (inverse of average distance)
                    score += 100.0 / (1.0 + avg_distance);
                }
            }
        }

        score
    }

    /// Score location considering agent position and strategy.
    ///
    /// This is a convenience method that infers placement criteria from the
    /// building type. For more control over placement criteria, use
    /// `score_location_for_agent_with_criteria` directly.
    pub fn score_location_for_agent(
        &self,
        pos: Position,
        agent_pos: Position,
        building_type: BuildingType,
        strategy: PlacementStrategy,
    ) -> f32 {
        let criteria = self.infer_criteria_from_building(building_type);
        self.score_location_for_agent_with_criteria(pos, agent_pos, building_type, strategy, &criteria)
    }

    fn score_location_for_agent_with_criteria(
        &self,
        pos: Position,
        agent_pos: Position,
        building_type: BuildingType,
        strategy: PlacementStrategy,
        criteria: &PlacementCriteria,
    ) -> f32 {
        let distance_to_agent = Self::distance(pos, agent_pos);

        // Get zone bonus (applies to all strategies)
        let zone_bonus = self.world.zone_manager.get_zone_bonus(pos, building_type);

        // Get road accessibility bonus
        let road_bonus = self.calculate_road_accessibility_bonus(pos);

        match strategy {
            PlacementStrategy::NearAgent => {
                // Strongly prioritize being near agent
                let base_score = 100.0 / (1.0 + distance_to_agent);
                base_score + zone_bonus + road_bonus
            }

            PlacementStrategy::NearestAvailable => {
                // Just find the nearest spot
                let base_score = 100.0 / (1.0 + distance_to_agent);
                base_score + zone_bonus + road_bonus
            }

            PlacementStrategy::NearResources => {
                // Prioritize resource proximity using the actual criteria
                let resource_score = self.score_location(
                    pos,
                    building_type,
                    criteria.clone(),
                );
                let agent_penalty = distance_to_agent * 2.0;
                resource_score - agent_penalty + zone_bonus + road_bonus
            }

            PlacementStrategy::BalancedProximity => {
                // Balance both factors using the actual criteria
                let resource_score = self.score_location(
                    pos,
                    building_type,
                    criteria.clone(),
                );
                let agent_score = 50.0 / (1.0 + distance_to_agent);
                (resource_score * 0.6 + agent_score * 0.4) + zone_bonus + road_bonus
            }
        }
    }

    /// Calculate bonus for being near roads (good accessibility)
    fn calculate_road_accessibility_bonus(&self, pos: Position) -> f32 {
        // Direct road access
        if self.world.road_network.has_road_at(pos) {
            return 30.0; // Strong bonus for being directly on a road
        }

        // Find nearest road
        let mut min_distance = f32::MAX;
        for road in self.world.road_network.get_roads() {
            for node in road.nodes() {
                let dist = Self::distance(pos, node.position);
                if dist < min_distance {
                    min_distance = dist;
                }
            }
        }

        // Bonus decreases with distance from nearest road
        if min_distance < 5.0 {
            20.0 / (1.0 + min_distance) // Nearby road gives good bonus
        } else if min_distance < 10.0 {
            10.0 / (1.0 + min_distance) // Moderate bonus
        } else {
            0.0 // Too far from roads
        }
    }

    /// Get the center point for searching based on criteria
    fn get_search_center(&self, criteria: &PlacementCriteria) -> Position {
        match criteria {
            PlacementCriteria::NearResource(resource_type) => {
                if let Some(positions) = self.resource_locations.get(resource_type) {
                    if let Some(&first) = positions.first() {
                        return first;
                    }
                }
            }

            PlacementCriteria::NearRelatedBuilding | PlacementCriteria::NearSettlement => {
                // Use first building as center
                if let Some(positions) = self.building_locations.values().next() {
                    if let Some(&first) = positions.first() {
                        return first;
                    }
                }
            }

            PlacementCriteria::CentralToSettlement => {
                // Calculate centroid of all buildings
                let all_positions: Vec<Position> = self.building_locations.values()
                    .flat_map(|positions| positions.iter().copied())
                    .collect();

                if !all_positions.is_empty() {
                    return Self::calculate_centroid(&all_positions);
                }
            }
        }

        // Default to world center
        let (width, height) = self.world.config.size;
        ((width / 2) as i32, (height / 2) as i32, 0)
    }

    /// Check if position maintains minimum spacing from other buildings
    fn check_spacing(&self, pos: Position, min_spacing: i32) -> bool {
        let min_spacing_f = min_spacing as f32;

        for positions in self.building_locations.values() {
            for &building_pos in positions {
                if Self::distance(pos, building_pos) < min_spacing_f {
                    return false;
                }
            }
        }

        true
    }

    /// Get buildings that consume output from this building type
    fn get_consumer_buildings(&self, building_type: BuildingType) -> Vec<BuildingType> {
        let mut consumers = Vec::new();

        // Check all building types to see if they require this one
        let all_types = vec![
            BuildingType::Mill,
            BuildingType::Bakery,
            BuildingType::Forge,
            BuildingType::Smithy,
            BuildingType::TailorShop,
            BuildingType::CobblerShop,
        ];

        for consumer_type in all_types {
            let prerequisites = consumer_type.prerequisites();
            if prerequisites.contains(&building_type) {
                consumers.push(consumer_type);
            }
        }

        consumers
    }

    /// Calculate Euclidean distance between two positions
    fn distance(pos1: Position, pos2: Position) -> f32 {
        let dx = (pos1.0 - pos2.0) as f32;
        let dy = (pos1.1 - pos2.1) as f32;
        let dz = (pos1.2 - pos2.2) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate centroid of a set of positions
    fn calculate_centroid(positions: &[Position]) -> Position {
        let sum_x: i32 = positions.iter().map(|p| p.0).sum();
        let sum_y: i32 = positions.iter().map(|p| p.1).sum();
        let sum_z: i32 = positions.iter().map(|p| p.2).sum();
        let count = positions.len() as i32;
        (sum_x / count, sum_y / count, sum_z / count)
    }
}
