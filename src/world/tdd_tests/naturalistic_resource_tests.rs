// src/world/tdd_tests/naturalistic_resource_tests.rs
//! Tests for naturalistic resource distribution system

use crate::world::{World, WorldConfig, ResourceConfig, ResourceType, TerrainType};
use std::collections::HashMap;

#[test]
fn test_world_generates_all_resource_types() {
    let config = WorldConfig {
        size: (100, 100),
        initial_resources: ResourceConfig::default(),
    };

    let world = World::new(config);

    // Count resource types
    let mut resource_counts: HashMap<ResourceType, usize> = HashMap::new();
    for resource in &world.resources {
        *resource_counts.entry(resource.resource_type).or_insert(0) += 1;
    }

    // Check that basic resources exist
    assert!(resource_counts.get(&ResourceType::Wood).unwrap_or(&0) > &0, "No wood resources");
    assert!(resource_counts.get(&ResourceType::Stone).unwrap_or(&0) > &0, "No stone resources");
    assert!(resource_counts.get(&ResourceType::Iron).unwrap_or(&0) > &0, "No iron resources");
    assert!(resource_counts.get(&ResourceType::Food).unwrap_or(&0) > &0, "No food resources");

    // Check that new naturalistic resources exist
    assert!(resource_counts.get(&ResourceType::Clay).unwrap_or(&0) > &0, "No clay resources");
    assert!(resource_counts.get(&ResourceType::Coal).unwrap_or(&0) > &0, "No coal resources");
    assert!(resource_counts.get(&ResourceType::Grain).unwrap_or(&0) > &0, "No grain resources");
    assert!(resource_counts.get(&ResourceType::Flax).unwrap_or(&0) > &0, "No flax resources");
    assert!(resource_counts.get(&ResourceType::Herbs).unwrap_or(&0) > &0, "No herbs resources");
}

#[test]
fn test_resources_in_appropriate_terrain() {
    let config = WorldConfig {
        size: (100, 100),
        initial_resources: ResourceConfig::default(),
    };

    let world = World::new(config);

    // Check wood is in forests
    for resource in world.resources.iter().filter(|r| r.resource_type == ResourceType::Wood) {
        if let Some(tile) = world.grid.get_tile(&resource.position) {
            assert_eq!(
                tile.terrain.terrain_type,
                TerrainType::Forest,
                "Wood found outside forest at {:?}",
                resource.position
            );
        }
    }

    // Check stone is in mountains or hills
    for resource in world.resources.iter().filter(|r| r.resource_type == ResourceType::Stone) {
        if let Some(tile) = world.grid.get_tile(&resource.position) {
            assert!(
                matches!(tile.terrain.terrain_type, TerrainType::Mountain | TerrainType::Hills),
                "Stone found in invalid terrain {:?} at {:?}",
                tile.terrain.terrain_type,
                resource.position
            );
        }
    }

    // Check iron is in mountains
    for resource in world.resources.iter().filter(|r| r.resource_type == ResourceType::Iron) {
        if let Some(tile) = world.grid.get_tile(&resource.position) {
            assert_eq!(
                tile.terrain.terrain_type,
                TerrainType::Mountain,
                "Iron found outside mountains at {:?}",
                resource.position
            );
        }
    }
}

#[test]
fn test_terrain_diversity() {
    let config = WorldConfig {
        size: (100, 100),
        initial_resources: ResourceConfig::default(),
    };

    let world = World::new(config);

    // Count terrain types
    let mut terrain_counts: HashMap<TerrainType, usize> = HashMap::new();
    for row in &world.grid.tiles {
        for tile in row {
            *terrain_counts.entry(tile.terrain.terrain_type).or_insert(0) += 1;
        }
    }

    // Verify we have a diverse landscape
    assert!(terrain_counts.len() >= 4, "Not enough terrain variety: {:?}", terrain_counts);

    // Check that basic terrain types exist
    assert!(terrain_counts.get(&TerrainType::Plains).unwrap_or(&0) > &0, "No plains terrain");
    assert!(terrain_counts.get(&TerrainType::Forest).unwrap_or(&0) > &0, "No forest terrain");
    assert!(terrain_counts.get(&TerrainType::Mountain).unwrap_or(&0) > &0, "No mountain terrain");
    assert!(terrain_counts.get(&TerrainType::Water).unwrap_or(&0) > &0, "No water terrain");
}

#[test]
fn test_resource_clustering() {
    let config = WorldConfig {
        size: (100, 100),
        initial_resources: ResourceConfig {
            clay_clusters: 5,
            ..Default::default()
        },
    };

    let world = World::new(config);

    // Get all clay positions
    let clay_positions: Vec<_> = world.resources
        .iter()
        .filter(|r| r.resource_type == ResourceType::Clay)
        .map(|r| (r.position.x, r.position.y))
        .collect();

    if clay_positions.len() > 1 {
        // Check that some clay nodes are close to each other (clustered)
        let mut found_cluster = false;
        for i in 0..clay_positions.len() {
            for j in (i + 1)..clay_positions.len() {
                let dx = (clay_positions[i].0 - clay_positions[j].0).abs();
                let dy = (clay_positions[i].1 - clay_positions[j].1).abs();
                let distance = dx + dy;

                if distance <= 20 {
                    found_cluster = true;
                    break;
                }
            }
            if found_cluster {
                break;
            }
        }

        assert!(found_cluster, "Clay resources don't appear to be clustered");
    }
}

#[test]
fn test_naturalistic_spawning_disabled() {
    let config = WorldConfig {
        size: (50, 50),
        initial_resources: ResourceConfig {
            use_naturalistic_spawning: false,
            ..Default::default()
        },
    };

    let world = World::new(config);

    // Count resource types
    let mut resource_counts: HashMap<ResourceType, usize> = HashMap::new();
    for resource in &world.resources {
        *resource_counts.entry(resource.resource_type).or_insert(0) += 1;
    }

    // When naturalistic spawning is disabled, only basic resources should exist
    assert!(resource_counts.get(&ResourceType::Wood).unwrap_or(&0) > &0);
    assert!(resource_counts.get(&ResourceType::Stone).unwrap_or(&0) > &0);
    assert!(resource_counts.get(&ResourceType::Iron).unwrap_or(&0) > &0);
    assert!(resource_counts.get(&ResourceType::Food).unwrap_or(&0) > &0);

    // Naturalistic resources should NOT exist
    assert_eq!(resource_counts.get(&ResourceType::Clay).unwrap_or(&0), &0, "Clay should not exist with naturalistic spawning disabled");
    assert_eq!(resource_counts.get(&ResourceType::Coal).unwrap_or(&0), &0, "Coal should not exist with naturalistic spawning disabled");
    assert_eq!(resource_counts.get(&ResourceType::Grain).unwrap_or(&0), &0, "Grain should not exist with naturalistic spawning disabled");
}

#[test]
fn test_technology_progression_resources_available() {
    let config = WorldConfig {
        size: (100, 100),
        initial_resources: ResourceConfig::default(),
    };

    let world = World::new(config);

    // Count resource types
    let mut resource_counts: HashMap<ResourceType, usize> = HashMap::new();
    for resource in &world.resources {
        *resource_counts.entry(resource.resource_type).or_insert(0) += 1;
    }

    // Check resources required for technology progression are available

    // Stone Age - should have wood, stone
    assert!(resource_counts.get(&ResourceType::Wood).unwrap_or(&0) > &0, "Need wood for Stone Age");
    assert!(resource_counts.get(&ResourceType::Stone).unwrap_or(&0) > &0, "Need stone for Stone Age");

    // Copper/Bronze Age - should have clay (for pottery/smelting)
    assert!(resource_counts.get(&ResourceType::Clay).unwrap_or(&0) > &0, "Need clay for Copper Age pottery");

    // Iron Age - should have coal and iron
    assert!(resource_counts.get(&ResourceType::Coal).unwrap_or(&0) > &0, "Need coal for Iron Age");
    assert!(resource_counts.get(&ResourceType::Iron).unwrap_or(&0) > &0, "Need iron for Iron Age");

    // Medieval - should have flax, grain, herbs
    assert!(resource_counts.get(&ResourceType::Flax).unwrap_or(&0) > &0, "Need flax for Medieval textiles");
    assert!(resource_counts.get(&ResourceType::Grain).unwrap_or(&0) > &0, "Need grain for Medieval brewing");
    assert!(resource_counts.get(&ResourceType::Herbs).unwrap_or(&0) > &0, "Need herbs for Medieval crafts");
}

#[test]
fn test_resource_amounts_reasonable() {
    let config = WorldConfig {
        size: (50, 50),
        initial_resources: ResourceConfig::default(),
    };

    let world = World::new(config);

    // Check that resource amounts are within reasonable ranges
    for resource in &world.resources {
        assert!(resource.amount > 0, "Resource has zero amount: {:?}", resource);
        assert!(resource.amount <= 500, "Resource has unreasonably high amount: {:?}", resource);
        assert!(resource.max_amount >= resource.amount, "Max amount less than current: {:?}", resource);
    }
}
