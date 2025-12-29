// src/world/tdd_tests/spatial_planning_tests.rs
//! TDD tests for spatial planning and intelligent building placement
//!
//! These tests define the expected behavior for optimizing building locations
//! to minimize travel time and maximize production efficiency.

use crate::world::{World, WorldConfig, ResourceConfig, BuildingType, Position};
use crate::world::spatial_planning::{SpatialPlanner, PlacementStrategy, PlacementCriteria};

#[test]
fn test_spatial_planner_creation() {
    let world = World::new(WorldConfig::default());
    let planner = SpatialPlanner::new(&world);

    assert!(planner.is_initialized());
}

#[test]
fn test_find_optimal_location_near_resources() {
    let mut world = World::new(WorldConfig::default());

    // Place a wood resource at (10, 10, 0)
    world.place_resource_node("wood", (10, 10, 0));

    let planner = SpatialPlanner::new(&world);

    // Find optimal location for a Workshop (needs wood)
    let optimal_pos = planner.find_optimal_location(
        BuildingType::Workshop,
        PlacementCriteria::NearResource("wood".to_string())
    );

    assert!(optimal_pos.is_some());
    let pos = optimal_pos.unwrap();

    // Should be within reasonable distance of the wood resource
    let distance = calculate_distance(pos, (10, 10, 0));
    assert!(distance < 10.0, "Workshop should be near wood resource, distance: {}", distance);
}

#[test]
fn test_cluster_related_production_buildings() {
    let mut world = World::new(WorldConfig::default());

    // Create a Farm at (20, 20, 0)
    world.add_building_at(BuildingType::Farm, (20, 20, 0));

    let planner = SpatialPlanner::new(&world);

    // Mill should be placed near Farm (production chain: Farm → Mill)
    let mill_pos = planner.find_optimal_location(
        BuildingType::Mill,
        PlacementCriteria::NearRelatedBuilding
    );

    assert!(mill_pos.is_some());
    let pos = mill_pos.unwrap();

    // Mill should be within 5 tiles of Farm
    let distance = calculate_distance(pos, (20, 20, 0));
    assert!(distance < 5.0, "Mill should be near Farm for production chain, distance: {}", distance);
}

#[test]
fn test_minimize_travel_time_from_agent_position() {
    let world = World::new(WorldConfig::default());
    let planner = SpatialPlanner::new(&world);

    let agent_pos = (15, 15, 0);

    // Find location that balances being near agent but also suitable
    let pos = planner.find_optimal_location_for_agent(
        BuildingType::SmallHouse,
        agent_pos,
        PlacementStrategy::BalancedProximity
    );

    assert!(pos.is_some());
    let optimal_pos = pos.unwrap();

    // Should be reasonably close to agent (within 10 tiles)
    let distance = calculate_distance(optimal_pos, agent_pos);
    assert!(distance < 10.0, "House should be near agent, distance: {}", distance);
}

#[test]
fn test_avoid_occupied_positions() {
    let mut world = World::new(WorldConfig::default());

    // Fill area around (10, 10) with buildings
    for x in 9..=11 {
        for y in 9..=11 {
            world.add_building_at(BuildingType::SmallHouse, (x, y, 0));
        }
    }

    let planner = SpatialPlanner::new(&world);

    // Should find unoccupied location nearby
    let pos = planner.find_optimal_location_for_agent(
        BuildingType::Workshop,
        (10, 10, 0),
        PlacementStrategy::NearestAvailable
    );

    assert!(pos.is_some());
    let optimal_pos = pos.unwrap();

    // Should not be at any occupied position
    let grid_pos = Position::new(optimal_pos.0, optimal_pos.1);
    assert!(!world.is_position_occupied(&grid_pos));

    // Should still be reasonably close
    let distance = calculate_distance(optimal_pos, (10, 10, 0));
    assert!(distance < 5.0, "Should find nearby unoccupied spot, distance: {}", distance);
}

#[test]
fn test_production_chain_clustering() {
    let mut world = World::new(WorldConfig::default());

    // Place Farm at (30, 30)
    world.add_building_at(BuildingType::Farm, (30, 30, 0));

    let planner = SpatialPlanner::new(&world);

    // Production chain: Farm → Mill → Bakery
    let mill_pos = planner.find_optimal_location(
        BuildingType::Mill,
        PlacementCriteria::NearRelatedBuilding
    ).unwrap();

    drop(planner); // Drop planner to release immutable borrow
    world.add_building_at(BuildingType::Mill, mill_pos);

    let planner = SpatialPlanner::new(&world); // Recreate planner with updated world
    let bakery_pos = planner.find_optimal_location(
        BuildingType::Bakery,
        PlacementCriteria::NearRelatedBuilding
    ).unwrap();

    // All three should form a cluster
    let farm_to_mill = calculate_distance((30, 30, 0), mill_pos);
    let mill_to_bakery = calculate_distance(mill_pos, bakery_pos);

    assert!(farm_to_mill < 5.0, "Mill should be near Farm");
    assert!(mill_to_bakery < 5.0, "Bakery should be near Mill");

    // Total chain distance should be efficient
    let total_chain_distance = farm_to_mill + mill_to_bakery;
    assert!(total_chain_distance < 10.0, "Production chain should be compact");
}

#[test]
fn test_metalworking_chain_clustering() {
    let mut world = World::new(WorldConfig::default());

    // Place iron resource
    world.place_resource_node("iron", (40, 40, 0));

    let planner = SpatialPlanner::new(&world);

    // Metalworking chain: Workshop → Forge → Smithy
    let workshop_pos = planner.find_optimal_location(
        BuildingType::Workshop,
        PlacementCriteria::NearResource("iron".to_string())
    ).unwrap();

    drop(planner);
    world.add_building_at(BuildingType::Workshop, workshop_pos);

    let planner = SpatialPlanner::new(&world);
    let forge_pos = planner.find_optimal_location(
        BuildingType::Forge,
        PlacementCriteria::NearRelatedBuilding
    ).unwrap();

    drop(planner);
    world.add_building_at(BuildingType::Forge, forge_pos);

    let planner = SpatialPlanner::new(&world);
    let smithy_pos = planner.find_optimal_location(
        BuildingType::Smithy,
        PlacementCriteria::NearRelatedBuilding
    ).unwrap();

    // Verify clustering around iron resource
    let workshop_to_iron = calculate_distance(workshop_pos, (40, 40, 0));
    let forge_to_workshop = calculate_distance(forge_pos, workshop_pos);
    let smithy_to_forge = calculate_distance(smithy_pos, forge_pos);

    assert!(workshop_to_iron < 10.0, "Workshop near iron");
    assert!(forge_to_workshop < 5.0, "Forge near Workshop");
    assert!(smithy_to_forge < 5.0, "Smithy near Forge");
}

#[test]
fn test_settlement_core_formation() {
    let mut world = World::new(WorldConfig::default());
    let planner = SpatialPlanner::new(&world);

    // First house establishes settlement center
    let first_house = planner.find_optimal_location_for_agent(
        BuildingType::SmallHouse,
        (25, 25, 0),
        PlacementStrategy::BalancedProximity
    ).unwrap();

    drop(planner);
    world.add_building_at(BuildingType::SmallHouse, first_house);

    // Subsequent houses should cluster around first one
    let mut houses = vec![first_house];
    for _ in 0..5 {
        let planner = SpatialPlanner::new(&world);
        let next_house = planner.find_optimal_location(
            BuildingType::SmallHouse,
            PlacementCriteria::NearSettlement
        ).unwrap();

        drop(planner);
        world.add_building_at(BuildingType::SmallHouse, next_house);
        houses.push(next_house);
    }

    // Calculate settlement compactness
    let center = calculate_centroid(&houses);
    let avg_distance: f32 = houses.iter()
        .map(|&pos| calculate_distance(pos, center))
        .sum::<f32>() / houses.len() as f32;

    // Settlement should be compact (avg distance < 8 tiles from center)
    assert!(avg_distance < 8.0, "Settlement should be compact, avg distance: {}", avg_distance);
}

#[test]
fn test_avoid_building_in_impassable_terrain() {
    let mut world = World::new(WorldConfig::default());

    // Mark area as impassable (water, mountains, etc.)
    world.set_terrain_impassable((10, 10, 0), 3); // 3x3 impassable area

    let planner = SpatialPlanner::new(&world);

    let pos = planner.find_optimal_location_for_agent(
        BuildingType::Workshop,
        (10, 10, 0),
        PlacementStrategy::NearestAvailable
    );

    assert!(pos.is_some());
    let optimal_pos = pos.unwrap();

    // Should be outside the impassable zone
    assert!(world.is_terrain_passable(optimal_pos));
}

#[test]
fn test_storehouse_central_placement() {
    let mut world = World::new(WorldConfig::default());

    // Place several production buildings
    world.add_building_at(BuildingType::Farm, (20, 20, 0));
    world.add_building_at(BuildingType::Workshop, (25, 25, 0));
    world.add_building_at(BuildingType::SmallHouse, (22, 18, 0));
    world.add_building_at(BuildingType::SmallHouse, (18, 22, 0));

    let planner = SpatialPlanner::new(&world);

    // Storehouse should be centrally located to all buildings
    let storehouse_pos = planner.find_optimal_location(
        BuildingType::Storehouse,
        PlacementCriteria::CentralToSettlement
    ).unwrap();

    // Calculate average distance to all buildings
    let buildings = vec![(20, 20, 0), (25, 25, 0), (22, 18, 0), (18, 22, 0)];
    let avg_distance: f32 = buildings.iter()
        .map(|&pos| calculate_distance(pos, storehouse_pos))
        .sum::<f32>() / buildings.len() as f32;

    // Should be well-centered (avg distance < 5)
    assert!(avg_distance < 5.0, "Storehouse should be central, avg distance: {}", avg_distance);
}

#[test]
fn test_placement_strategies_differ() {
    let mut world = World::new(WorldConfig::default());
    world.place_resource_node("wood", (10, 10, 0));

    let planner = SpatialPlanner::new(&world);
    let agent_pos = (30, 30, 0);

    // Strategy 1: Prioritize being near agent
    let near_agent = planner.find_optimal_location_for_agent(
        BuildingType::Workshop,
        agent_pos,
        PlacementStrategy::NearAgent
    ).unwrap();

    // Strategy 2: Prioritize being near resources
    let near_resource = planner.find_optimal_location_for_agent(
        BuildingType::Workshop,
        agent_pos,
        PlacementStrategy::NearResources
    ).unwrap();

    // These should be different locations
    let agent_distance_1 = calculate_distance(near_agent, agent_pos);
    let agent_distance_2 = calculate_distance(near_resource, agent_pos);

    let resource_distance_1 = calculate_distance(near_agent, (10, 10, 0));
    let resource_distance_2 = calculate_distance(near_resource, (10, 10, 0));

    // NearAgent strategy should be closer to agent
    assert!(agent_distance_1 < agent_distance_2, "NearAgent should prioritize agent proximity");

    // NearResources strategy should be closer to resource
    assert!(resource_distance_2 < resource_distance_1, "NearResources should prioritize resource proximity");
}

#[test]
fn test_respect_minimum_spacing() {
    let mut world = World::new(WorldConfig::default());
    world.add_building_at(BuildingType::Farm, (15, 15, 0));

    let planner = SpatialPlanner::new(&world);

    // Try to find location with minimum spacing of 2 tiles
    let pos = planner.find_optimal_location_with_spacing(
        BuildingType::SmallHouse,
        PlacementCriteria::NearSettlement,
        2 // min spacing
    ).unwrap();

    let distance = calculate_distance(pos, (15, 15, 0));
    assert!(distance >= 2.0, "Should respect minimum spacing");
}

#[test]
fn test_scoring_system_for_locations() {
    let mut world = World::new(WorldConfig::default());
    world.place_resource_node("wood", (10, 10, 0));
    world.add_building_at(BuildingType::Workshop, (12, 12, 0));

    let planner = SpatialPlanner::new(&world);

    // Score a specific location for Forge (needs Workshop nearby)
    let score_near_workshop = planner.score_location(
        (13, 13, 0),
        BuildingType::Forge,
        PlacementCriteria::NearRelatedBuilding
    );

    let score_far_from_workshop = planner.score_location(
        (50, 50, 0),
        BuildingType::Forge,
        PlacementCriteria::NearRelatedBuilding
    );

    // Location near Workshop should score higher
    assert!(score_near_workshop > score_far_from_workshop,
            "Location near prerequisite should score higher: {} vs {}",
            score_near_workshop, score_far_from_workshop);
}

// Helper functions
fn calculate_distance(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> f32 {
    let dx = (pos1.0 - pos2.0) as f32;
    let dy = (pos1.1 - pos2.1) as f32;
    let dz = (pos1.2 - pos2.2) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn calculate_centroid(positions: &[(i32, i32, i32)]) -> (i32, i32, i32) {
    let sum_x: i32 = positions.iter().map(|p| p.0).sum();
    let sum_y: i32 = positions.iter().map(|p| p.1).sum();
    let sum_z: i32 = positions.iter().map(|p| p.2).sum();
    let count = positions.len() as i32;
    (sum_x / count, sum_y / count, sum_z / count)
}
