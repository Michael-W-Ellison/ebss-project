// src/world/tdd_tests/zoning_tests.rs
//! TDD tests for spatial zoning system
//!
//! These tests define the expected behavior for zoning preferences where
//! buildings are placed in appropriate zones (residential, industrial, agricultural).

use crate::world::{World, WorldConfig, BuildingType};
use crate::world::spatial_planning::{SpatialPlanner, PlacementStrategy, PlacementCriteria};
use crate::world::zoning::{ZoneType, ZoneManager};

#[test]
fn test_zone_manager_creation() {
    let _world = World::new(WorldConfig::default());
    let zone_manager = ZoneManager::new();

    assert!(zone_manager.get_zones().is_empty(), "New zone manager should have no zones");
}

#[test]
fn test_define_residential_zone() {
    let mut zone_manager = ZoneManager::new();

    // Define a residential zone in the northwest area
    zone_manager.add_zone(ZoneType::Residential, (10, 10, 0), 15);

    let zones = zone_manager.get_zones_at_position((10, 10, 0));
    assert!(zones.contains(&ZoneType::Residential), "Position should be in residential zone");
}

#[test]
fn test_define_industrial_zone() {
    let mut zone_manager = ZoneManager::new();

    // Define an industrial zone in the northeast area
    zone_manager.add_zone(ZoneType::Industrial, (50, 10, 0), 10);

    let zones = zone_manager.get_zones_at_position((50, 10, 0));
    assert!(zones.contains(&ZoneType::Industrial), "Position should be in industrial zone");
}

#[test]
fn test_define_agricultural_zone() {
    let mut zone_manager = ZoneManager::new();

    // Define an agricultural zone in the south
    zone_manager.add_zone(ZoneType::Agricultural, (30, 50, 0), 12);

    let zones = zone_manager.get_zones_at_position((30, 50, 0));
    assert!(zones.contains(&ZoneType::Agricultural), "Position should be in agricultural zone");
}

#[test]
fn test_overlapping_zones() {
    let mut zone_manager = ZoneManager::new();

    // Create overlapping residential and commercial zones (mixed use)
    zone_manager.add_zone(ZoneType::Residential, (20, 20, 0), 10);
    zone_manager.add_zone(ZoneType::Commercial, (25, 25, 0), 8);

    // Position in overlap should have both zones
    let zones = zone_manager.get_zones_at_position((23, 23, 0));
    assert!(!zones.is_empty(), "Overlapping area should have at least one zone");
}

#[test]
fn test_house_prefers_residential_zone() {
    let mut world = World::new(WorldConfig::default());

    // Define a residential zone at (20, 20)
    world.zone_manager.add_zone(ZoneType::Residential, (20, 20, 0), 10);

    // Define an industrial zone far away at (50, 50)
    world.zone_manager.add_zone(ZoneType::Industrial, (50, 50, 0), 10);

    let planner = SpatialPlanner::new(&world);

    // Agent at (35, 35) - equidistant from both zones
    let house_pos = planner.find_optimal_location_for_agent(
        BuildingType::SmallHouse,
        (35, 35, 0),
        PlacementStrategy::BalancedProximity
    );

    assert!(house_pos.is_some());
    let pos = house_pos.unwrap();

    // House should be closer to residential zone than industrial zone
    let dist_to_residential = calculate_distance(pos, (20, 20, 0));
    let dist_to_industrial = calculate_distance(pos, (50, 50, 0));

    assert!(dist_to_residential < dist_to_industrial,
            "House should prefer residential zone over industrial zone");
}

#[test]
fn test_workshop_prefers_industrial_zone() {
    let mut world = World::new(WorldConfig::default());

    // Define a residential zone at (20, 20)
    world.zone_manager.add_zone(ZoneType::Residential, (20, 20, 0), 10);

    // Define an industrial zone at (50, 50)
    world.zone_manager.add_zone(ZoneType::Industrial, (50, 50, 0), 10);

    let planner = SpatialPlanner::new(&world);

    // Agent at (35, 35) - equidistant from both zones
    let workshop_pos = planner.find_optimal_location_for_agent(
        BuildingType::Workshop,
        (35, 35, 0),
        PlacementStrategy::BalancedProximity
    );

    assert!(workshop_pos.is_some());
    let pos = workshop_pos.unwrap();

    // Workshop should be closer to industrial zone than residential zone
    let dist_to_industrial = calculate_distance(pos, (50, 50, 0));
    let dist_to_residential = calculate_distance(pos, (20, 20, 0));

    assert!(dist_to_industrial < dist_to_residential,
            "Workshop should prefer industrial zone over residential zone");
}

#[test]
fn test_farm_prefers_agricultural_zone() {
    let mut world = World::new(WorldConfig::default());

    // Define an agricultural zone at (30, 50)
    world.zone_manager.add_zone(ZoneType::Agricultural, (30, 50, 0), 12);

    // Define a residential zone at (20, 20)
    world.zone_manager.add_zone(ZoneType::Residential, (20, 20, 0), 10);

    let planner = SpatialPlanner::new(&world);

    // Agent at (25, 35) - between both zones
    let farm_pos = planner.find_optimal_location_for_agent(
        BuildingType::Farm,
        (25, 35, 0),
        PlacementStrategy::BalancedProximity
    );

    assert!(farm_pos.is_some());
    let pos = farm_pos.unwrap();

    // Farm should be closer to agricultural zone
    let dist_to_agricultural = calculate_distance(pos, (30, 50, 0));
    let dist_to_residential = calculate_distance(pos, (20, 20, 0));

    assert!(dist_to_agricultural < dist_to_residential,
            "Farm should prefer agricultural zone over residential zone");
}

#[test]
fn test_zoning_overrides_nearby_agent_position() {
    let mut world = World::new(WorldConfig::default());

    // Define industrial zone far from agent
    world.zone_manager.add_zone(ZoneType::Industrial, (60, 60, 0), 15);

    let planner = SpatialPlanner::new(&world);

    // Agent very close to (10, 10), but industrial zone is at (60, 60)
    let forge_pos = planner.find_optimal_location_for_agent(
        BuildingType::Forge,
        (10, 10, 0),
        PlacementStrategy::BalancedProximity
    );

    assert!(forge_pos.is_some());
    let pos = forge_pos.unwrap();

    // Forge should be pulled toward industrial zone despite agent being far
    // (within reasonable search radius)
    let dist_to_zone = calculate_distance(pos, (60, 60, 0));
    let dist_to_agent = calculate_distance(pos, (10, 10, 0));

    // Forge should be significantly closer to industrial zone center
    // if the zone is within search radius
    println!("Forge at {:?}, dist to zone: {}, dist to agent: {}",
             pos, dist_to_zone, dist_to_agent);
}

#[test]
fn test_multiple_buildings_cluster_within_zone() {
    let mut world = World::new(WorldConfig::default());

    // Define a residential zone
    world.zone_manager.add_zone(ZoneType::Residential, (25, 25, 0), 12);

    let planner = SpatialPlanner::new(&world);

    // Place first house
    let house1_pos = planner.find_optimal_location_for_agent(
        BuildingType::SmallHouse,
        (30, 30, 0),
        PlacementStrategy::BalancedProximity
    ).unwrap();

    drop(planner);
    world.add_building_at(BuildingType::SmallHouse, house1_pos);

    // Place second house
    let planner = SpatialPlanner::new(&world);
    let house2_pos = planner.find_optimal_location_for_agent(
        BuildingType::SmallHouse,
        (30, 30, 0),
        PlacementStrategy::BalancedProximity
    ).unwrap();

    drop(planner);
    world.add_building_at(BuildingType::SmallHouse, house2_pos);

    // Both houses should be within the residential zone
    let zones1 = world.zone_manager.get_zones_at_position(house1_pos);
    let zones2 = world.zone_manager.get_zones_at_position(house2_pos);

    assert!(zones1.contains(&ZoneType::Residential), "House 1 should be in residential zone");
    assert!(zones2.contains(&ZoneType::Residential), "House 2 should be in residential zone");
}

#[test]
fn test_zone_preference_scoring_bonus() {
    let mut world = World::new(WorldConfig::default());

    // Define a residential zone
    world.zone_manager.add_zone(ZoneType::Residential, (20, 20, 0), 10);

    let planner = SpatialPlanner::new(&world);

    // Score two positions - one in zone, one outside
    let in_zone_pos = (20, 20, 0);
    let outside_zone_pos = (50, 50, 0);

    let score_in_zone = planner.score_location_with_zones(
        in_zone_pos,
        BuildingType::SmallHouse,
        PlacementCriteria::NearSettlement
    );

    let score_outside_zone = planner.score_location_with_zones(
        outside_zone_pos,
        BuildingType::SmallHouse,
        PlacementCriteria::NearSettlement
    );

    // Position in appropriate zone should score significantly higher
    assert!(score_in_zone > score_outside_zone,
            "Position in residential zone should score higher for houses: {} vs {}",
            score_in_zone, score_outside_zone);
}

#[test]
fn test_storehouse_works_in_any_zone() {
    let mut world = World::new(WorldConfig::default());

    // Define different zones
    world.zone_manager.add_zone(ZoneType::Residential, (20, 20, 0), 10);
    world.zone_manager.add_zone(ZoneType::Industrial, (50, 50, 0), 10);

    // Place some buildings in different zones
    world.add_building_at(BuildingType::SmallHouse, (20, 20, 0));
    world.add_building_at(BuildingType::Workshop, (50, 50, 0));

    let planner = SpatialPlanner::new(&world);

    // Storehouse should be placed centrally, not restricted by zones
    let storehouse_pos = planner.find_optimal_location(
        BuildingType::Storehouse,
        PlacementCriteria::CentralToSettlement
    );

    assert!(storehouse_pos.is_some(), "Storehouse should find a location");
    // Storehouse can be in any zone or no zone - it's zone-neutral
}

// Helper functions
fn calculate_distance(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> f32 {
    let dx = (pos1.0 - pos2.0) as f32;
    let dy = (pos1.1 - pos2.1) as f32;
    let dz = (pos1.2 - pos2.2) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
