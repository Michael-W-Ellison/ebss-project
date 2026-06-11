// src/world/tdd_tests/path_planning_tests.rs
//! TDD tests for road/path planning system
//!
//! These tests define the expected behavior for creating efficient paths
//! between buildings to facilitate movement and trade.

use crate::world::{World, WorldConfig, BuildingType};
use crate::world::path_planning::{PathPlanner, Road, RoadNetwork, PathNode};

#[test]
fn test_path_planner_creation() {
    let world = World::new(WorldConfig::default());
    let planner = PathPlanner::new(&world);

    assert!(planner.is_initialized(), "PathPlanner should be initialized");
}

#[test]
fn test_find_path_between_two_positions() {
    let world = World::new(WorldConfig::default());
    let planner = PathPlanner::new(&world);

    let start = (10, 10, 0);
    let end = (20, 20, 0);

    let path = planner.find_path(start, end);

    assert!(path.is_some(), "Should find a path between two positions");
    let path_nodes = path.unwrap();
    assert!(!path_nodes.is_empty(), "Path should have nodes");
    assert_eq!(path_nodes.first().unwrap().position, start, "Path should start at start position");
    assert_eq!(path_nodes.last().unwrap().position, end, "Path should end at end position");
}

#[test]
fn test_straight_path_is_shortest() {
    let world = World::new(WorldConfig::default());
    let planner = PathPlanner::new(&world);

    // Straight horizontal path
    let path = planner.find_path((10, 10, 0), (20, 10, 0)).unwrap();
    let path_length = path.len();

    // Should be roughly 11 nodes (start + 9 intermediate + end) for distance of 10
    assert!(path_length <= 15, "Straight path should be efficient, got {} nodes", path_length);
}

#[test]
fn test_path_avoids_impassable_terrain() {
    let mut world = World::new(WorldConfig::default());

    // Create an obstacle
    world.set_terrain_impassable((15, 10, 0), 3);

    let planner = PathPlanner::new(&world);

    // Try to find path that would go through obstacle
    let path = planner.find_path((10, 10, 0), (20, 10, 0));

    assert!(path.is_some(), "Should find alternate path around obstacle");
    let path_nodes = path.unwrap();

    // Verify no node in path is impassable
    for node in &path_nodes {
        assert!(world.is_terrain_passable(node.position),
                "Path should not go through impassable terrain at {:?}", node.position);
    }
}

#[test]
fn test_road_network_creation() {
    let network = RoadNetwork::new();

    assert_eq!(network.get_roads().len(), 0, "New network should have no roads");
}

#[test]
fn test_add_road_to_network() {
    let mut network = RoadNetwork::new();

    let road = Road::new(vec![
        PathNode::new((10, 10, 0)),
        PathNode::new((11, 10, 0)),
        PathNode::new((12, 10, 0)),
    ]);

    network.add_road(road);

    assert_eq!(network.get_roads().len(), 1, "Network should have one road");
}

#[test]
fn test_connect_buildings_with_road() {
    let mut world = World::new(WorldConfig::default());

    // Place two buildings
    world.add_building_at(BuildingType::SmallHouse, (10, 10, 0));
    world.add_building_at(BuildingType::Workshop, (20, 20, 0));

    // Create road network and connect them
    let planner = PathPlanner::new(&world);
    let path = planner.connect_buildings((10, 10, 0), (20, 20, 0));

    assert!(path.is_some(), "Should create a path between buildings");

    let road_path = path.unwrap();
    world.road_network.add_road(Road::new(road_path.clone()));

    // Verify road exists in network
    assert_eq!(world.road_network.get_roads().len(), 1, "Should have one road");
}

#[test]
fn test_road_network_connectivity() {
    let mut world = World::new(WorldConfig::default());

    // Create a connected settlement
    world.add_building_at(BuildingType::SmallHouse, (10, 10, 0));
    world.add_building_at(BuildingType::Workshop, (20, 10, 0));
    world.add_building_at(BuildingType::Farm, (20, 20, 0));

    let planner = PathPlanner::new(&world);

    // Connect all buildings
    let path1 = planner.connect_buildings((10, 10, 0), (20, 10, 0)).unwrap();
    let path2 = planner.connect_buildings((20, 10, 0), (20, 20, 0)).unwrap();

    world.road_network.add_road(Road::new(path1));
    world.road_network.add_road(Road::new(path2));

    // Check if all buildings are connected
    assert!(world.road_network.are_connected((10, 10, 0), (20, 20, 0)),
            "Buildings should be connected through road network");
}

#[test]
fn test_path_cost_calculation() {
    let world = World::new(WorldConfig::default());
    let planner = PathPlanner::new(&world);

    let path = planner.find_path((10, 10, 0), (20, 20, 0)).unwrap();

    let cost = PathPlanner::calculate_path_cost(&path);

    // Diagonal distance is roughly 14.14, cost should be reasonable
    assert!(cost > 10.0 && cost < 30.0,
            "Path cost should be reasonable for diagonal path, got {}", cost);
}

#[test]
fn test_road_placement_bonus_in_spatial_planning() {
    let mut world = World::new(WorldConfig::default());

    // Add a road
    let road_path = vec![
        PathNode::new((15, 10, 0)),
        PathNode::new((15, 11, 0)),
        PathNode::new((15, 12, 0)),
    ];
    world.road_network.add_road(Road::new(road_path));

    let planner = crate::world::spatial_planning::SpatialPlanner::new(&world);

    // Position near road should get bonus
    let score_near_road = planner.score_location(
        (14, 11, 0),
        BuildingType::SmallHouse,
        crate::world::spatial_planning::PlacementCriteria::NearSettlement
    );

    let score_far_from_road = planner.score_location(
        (50, 50, 0),
        BuildingType::SmallHouse,
        crate::world::spatial_planning::PlacementCriteria::NearSettlement
    );

    // Building near road should have better accessibility
    println!("Score near road: {}, far from road: {}", score_near_road, score_far_from_road);
}

#[test]
fn test_minimum_spanning_tree_for_settlement() {
    let mut world = World::new(WorldConfig::default());

    // Place multiple buildings
    world.add_building_at(BuildingType::SmallHouse, (10, 10, 0));
    world.add_building_at(BuildingType::SmallHouse, (20, 10, 0));
    world.add_building_at(BuildingType::SmallHouse, (15, 20, 0));
    world.add_building_at(BuildingType::Workshop, (25, 15, 0));

    let planner = PathPlanner::new(&world);

    // Generate minimum spanning tree to connect all buildings
    let building_positions = vec![
        (10, 10, 0),
        (20, 10, 0),
        (15, 20, 0),
        (25, 15, 0),
    ];

    let mst_roads = planner.create_minimum_spanning_tree(&building_positions);

    assert!(!mst_roads.is_empty(), "Should create roads for MST");
    assert!(mst_roads.len() >= 3, "MST should have at least n-1 roads for n buildings");
}

#[test]
fn test_road_upgrades_and_types() {
    let mut road = Road::new(vec![
        PathNode::new((10, 10, 0)),
        PathNode::new((11, 10, 0)),
    ]);

    assert_eq!(road.road_type(), crate::world::path_planning::RoadType::DirtPath);

    // Upgrade road
    road.upgrade_to(crate::world::path_planning::RoadType::StoneRoad);

    assert_eq!(road.road_type(), crate::world::path_planning::RoadType::StoneRoad);
}

#[test]
fn test_traffic_flow_capacity() {
    let road = Road::new(vec![
        PathNode::new((10, 10, 0)),
        PathNode::new((20, 20, 0)),
    ]);

    // Different road types have different capacities
    assert!(road.travel_speed_multiplier() > 0.0, "Road should have travel speed");
}

#[test]
fn test_auto_connect_new_building_to_network() {
    let mut world = World::new(WorldConfig::default());

    // Create existing settlement with roads
    world.add_building_at(BuildingType::SmallHouse, (10, 10, 0));
    world.add_building_at(BuildingType::Workshop, (20, 10, 0));

    let planner = PathPlanner::new(&world);
    let path = planner.connect_buildings((10, 10, 0), (20, 10, 0)).unwrap();
    world.road_network.add_road(Road::new(path));

    // Add new building
    let new_building_pos = (15, 20, 0);
    world.add_building_at(BuildingType::Farm, new_building_pos);

    // Auto-connect to nearest existing building
    let planner = PathPlanner::new(&world);
    let nearest_connection = planner.find_nearest_road_connection(new_building_pos);

    assert!(nearest_connection.is_some(), "Should find nearest road connection point");
}

#[test]
fn test_path_smoothing() {
    let world = World::new(WorldConfig::default());
    let planner = PathPlanner::new(&world);

    // Create a zigzag path
    let raw_path = vec![
        PathNode::new((0, 0, 0)),
        PathNode::new((1, 0, 0)),
        PathNode::new((1, 1, 0)),
        PathNode::new((2, 1, 0)),
        PathNode::new((2, 2, 0)),
        PathNode::new((3, 2, 0)),
    ];

    let smoothed = planner.smooth_path(&raw_path);

    // Smoothed path should have fewer nodes
    assert!(smoothed.len() <= raw_path.len(),
            "Smoothed path should have same or fewer nodes");
}

#[test]
fn test_road_intersection_handling() {
    let mut network = RoadNetwork::new();

    // Create two intersecting roads
    let road1 = Road::new(vec![
        PathNode::new((10, 10, 0)),
        PathNode::new((10, 20, 0)),
    ]);

    let road2 = Road::new(vec![
        PathNode::new((5, 15, 0)),
        PathNode::new((15, 15, 0)),
    ]);

    network.add_road(road1);
    network.add_road(road2);

    // Check for intersections
    let intersections = network.find_intersections();

    // Should detect intersection near (10, 15)
    println!("Found {} intersections", intersections.len());
}

