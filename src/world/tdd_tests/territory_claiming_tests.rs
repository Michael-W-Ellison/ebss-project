// src/world/tdd_tests/territory_claiming_tests.rs
//! TDD tests for territory claiming and land ownership system
//!
//! These tests define the expected behavior for agents claiming and managing
//! territories to organize settlement development and prevent building conflicts.

use crate::world::{World, WorldConfig, BuildingType};
use crate::world::territory::{Territory, TerritoryManager, TerritoryClaimResult};

#[test]
fn test_territory_manager_creation() {
    let world = World::new(WorldConfig::default());

    assert_eq!(world.territory_manager.get_all_territories().len(), 0,
               "New world should have no territories");
}

#[test]
fn test_agent_claims_territory() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let center = (50, 50, 0);
    let radius = 10;

    let result = world.territory_manager.claim_territory(agent_id, center, radius);

    assert!(matches!(result, TerritoryClaimResult::Success(_)),
            "Agent should be able to claim unclaimed territory");

    let territories = world.territory_manager.get_territories_for_agent(agent_id);
    assert_eq!(territories.len(), 1, "Agent should have one territory");
}

#[test]
fn test_territory_ownership_tracking() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let center = (50, 50, 0);

    world.territory_manager.claim_territory(agent_id, center, 10);

    // Check ownership at center
    assert_eq!(world.territory_manager.get_owner_at(center), Some(agent_id),
               "Center should be owned by claiming agent");

    // Check ownership at edge (within radius)
    assert_eq!(world.territory_manager.get_owner_at((55, 50, 0)), Some(agent_id),
               "Position within radius should be owned");

    // Check ownership outside territory
    assert_eq!(world.territory_manager.get_owner_at((100, 100, 0)), None,
               "Position outside territory should be unowned");
}

#[test]
fn test_territory_claim_conflicts() {
    let mut world = World::new(WorldConfig::default());

    let agent1 = 1;
    let agent2 = 2;
    let center = (50, 50, 0);

    // Agent 1 claims territory
    let result1 = world.territory_manager.claim_territory(agent1, center, 10);
    assert!(matches!(result1, TerritoryClaimResult::Success(_)));

    // Agent 2 tries to claim overlapping territory
    let overlapping_center = (55, 50, 0); // Within agent1's territory
    let result2 = world.territory_manager.claim_territory(agent2, overlapping_center, 10);

    assert!(matches!(result2, TerritoryClaimResult::Conflict(_)),
            "Should not allow overlapping territory claims");
}

#[test]
fn test_territory_size_limits() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let center = (50, 50, 0);

    // Try to claim extremely large territory
    let result = world.territory_manager.claim_territory(agent_id, center, 1000);

    assert!(matches!(result, TerritoryClaimResult::TooLarge),
            "Should reject territories that are too large");

    // Reasonable size should work
    let result2 = world.territory_manager.claim_territory(agent_id, center, 20);
    assert!(matches!(result2, TerritoryClaimResult::Success(_)),
            "Reasonable territory size should succeed");
}

#[test]
fn test_agent_maximum_territories() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;

    // Claim multiple territories
    world.territory_manager.claim_territory(agent_id, (10, 10, 0), 5);
    world.territory_manager.claim_territory(agent_id, (30, 10, 0), 5);
    world.territory_manager.claim_territory(agent_id, (50, 10, 0), 5);
    world.territory_manager.claim_territory(agent_id, (70, 10, 0), 5);
    world.territory_manager.claim_territory(agent_id, (90, 10, 0), 5);

    // Try to claim one more
    let result = world.territory_manager.claim_territory(agent_id, (110, 10, 0), 5);

    assert!(matches!(result, TerritoryClaimResult::TooManyTerritories),
            "Should limit number of territories per agent");
}

#[test]
fn test_territory_expansion() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let center = (50, 50, 0);

    let result = world.territory_manager.claim_territory(agent_id, center, 10);
    let territory_id = match result {
        TerritoryClaimResult::Success(id) => id,
        _ => panic!("Initial claim should succeed"),
    };

    // Expand territory
    let expand_result = world.territory_manager.expand_territory(territory_id, 5);

    assert!(expand_result.is_ok(), "Should be able to expand territory");

    // Verify new radius
    let territory = world.territory_manager.get_territory(territory_id).unwrap();
    assert_eq!(territory.radius(), 15, "Territory should have expanded");
}

#[test]
fn test_building_placement_prefers_owned_territory() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let territory_center = (50, 50, 0);

    // Agent claims territory
    world.territory_manager.claim_territory(agent_id, territory_center, 15);

    // Add resource to territory
    world.resource_nodes.insert("wood".to_string(), vec![territory_center]);

    let planner = crate::world::spatial_planning::SpatialPlanner::new(&world);

    // Try to place a building - should prefer owned territory
    let agent_pos = (45, 45, 0);
    let optimal_pos = planner.find_optimal_location_with_territory(
        BuildingType::Workshop,
        agent_pos,
        crate::world::spatial_planning::PlacementStrategy::NearResources,
        agent_id,
    );

    assert!(optimal_pos.is_some(), "Should find a location");

    // The optimal position should be in the agent's territory
    let pos = optimal_pos.unwrap();
    let owner = world.territory_manager.get_owner_at(pos);

    // Territory bonus is now integrated - building placement should prefer owned territory
    assert_eq!(owner, Some(agent_id), "Should prefer building in owned territory");
}

#[test]
fn test_territory_boundaries() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let center = (50, 50, 0);
    let radius = 10;

    world.territory_manager.claim_territory(agent_id, center, radius);

    // Test exact boundary points (Manhattan distance)
    assert_eq!(world.territory_manager.get_owner_at((60, 50, 0)), Some(agent_id),
               "Point at exact radius distance should be owned");

    assert_eq!(world.territory_manager.get_owner_at((50, 60, 0)), Some(agent_id),
               "Point at exact radius distance should be owned");

    assert_eq!(world.territory_manager.get_owner_at((61, 50, 0)), None,
               "Point beyond radius should not be owned");
}

#[test]
fn test_territory_abandonment() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    let center = (50, 50, 0);

    let result = world.territory_manager.claim_territory(agent_id, center, 10);
    let territory_id = match result {
        TerritoryClaimResult::Success(id) => id,
        _ => panic!("Claim should succeed"),
    };

    // Abandon territory
    let abandon_result = world.territory_manager.abandon_territory(territory_id);

    assert!(abandon_result.is_ok(), "Should be able to abandon territory");

    // Verify territory is no longer owned
    assert_eq!(world.territory_manager.get_owner_at(center), None,
               "Abandoned territory should have no owner");
}

#[test]
fn test_territory_transfer() {
    let mut world = World::new(WorldConfig::default());

    let agent1 = 1;
    let agent2 = 2;
    let center = (50, 50, 0);

    let result = world.territory_manager.claim_territory(agent1, center, 10);
    let territory_id = match result {
        TerritoryClaimResult::Success(id) => id,
        _ => panic!("Claim should succeed"),
    };

    // Transfer to another agent
    let transfer_result = world.territory_manager.transfer_territory(territory_id, agent2);

    assert!(transfer_result.is_ok(), "Should be able to transfer territory");

    // Verify new ownership
    assert_eq!(world.territory_manager.get_owner_at(center), Some(agent2),
               "Territory should be owned by new agent");
}

#[test]
fn test_multiple_non_overlapping_territories() {
    let mut world = World::new(WorldConfig::default());

    let agent1 = 1;
    let agent2 = 2;

    // Two agents claim separate territories
    let result1 = world.territory_manager.claim_territory(agent1, (20, 20, 0), 8);
    let result2 = world.territory_manager.claim_territory(agent2, (50, 50, 0), 8);

    assert!(matches!(result1, TerritoryClaimResult::Success(_)));
    assert!(matches!(result2, TerritoryClaimResult::Success(_)));

    // Verify boundaries
    assert_eq!(world.territory_manager.get_owner_at((20, 20, 0)), Some(agent1));
    assert_eq!(world.territory_manager.get_owner_at((50, 50, 0)), Some(agent2));
    assert_eq!(world.territory_manager.get_owner_at((35, 35, 0)), None,
               "Area between territories should be unowned");
}

#[test]
fn test_territory_visualization_data() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    world.territory_manager.claim_territory(agent_id, (50, 50, 0), 10);

    // Get visualization data (boundary points)
    let territories = world.territory_manager.get_all_territories();
    assert_eq!(territories.len(), 1, "Should have one territory");

    let territory = &territories[0];
    assert_eq!(territory.center(), (50, 50, 0));
    assert_eq!(territory.radius(), 10);
    assert_eq!(territory.owner(), agent_id);
}

#[test]
fn test_building_conflict_in_others_territory() {
    let mut world = World::new(WorldConfig::default());

    let agent1 = 1;
    let agent2 = 2;

    // Agent 1 claims territory
    world.territory_manager.claim_territory(agent1, (50, 50, 0), 10);

    // Check if agent 2 can build in agent 1's territory
    let can_build = world.territory_manager.can_build_at(agent2, (50, 50, 0));

    assert!(!can_build, "Agent should not be able to build in another agent's territory");

    // Agent 1 should be able to build in their own territory
    let can_build_own = world.territory_manager.can_build_at(agent1, (50, 50, 0));
    assert!(can_build_own, "Agent should be able to build in their own territory");
}

#[test]
fn test_territory_claim_scoring_bonus() {
    let mut world = World::new(WorldConfig::default());

    let agent_id = 1;
    world.territory_manager.claim_territory(agent_id, (50, 50, 0), 15);

    let planner = crate::world::spatial_planning::SpatialPlanner::new(&world);

    // Score a position in owned territory
    let score_owned = planner.score_location_with_territory(
        (50, 50, 0),
        BuildingType::SmallHouse,
        crate::world::spatial_planning::PlacementCriteria::NearSettlement,
        Some(agent_id),
    );

    // Score a position in unowned territory
    let score_unowned = planner.score_location_with_territory(
        (100, 100, 0),
        BuildingType::SmallHouse,
        crate::world::spatial_planning::PlacementCriteria::NearSettlement,
        Some(agent_id),
    );

    assert!(score_owned > score_unowned,
            "Building in owned territory should score higher than unowned");
}
