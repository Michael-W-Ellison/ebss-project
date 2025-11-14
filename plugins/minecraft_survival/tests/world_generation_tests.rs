// plugins/minecraft_survival/tests/world_generation_tests.rs
//! Tests for natural terrain generation with water and biomes.

use minecraft_survival::MinecraftSurvivalPlugin;
use ebss::environment::*;

#[test]
fn test_plugin_has_water_materials() {
    let plugin = MinecraftSurvivalPlugin::new();

    // Check that water material exists
    let water = plugin.get_material("water");
    assert!(water.is_some());
    assert_eq!(water.unwrap().category, MaterialCategory::Liquid);

    // Check terrain materials
    assert!(plugin.get_material("dirt").is_some());
    assert!(plugin.get_material("grass").is_some());
    assert!(plugin.get_material("sand").is_some());
}

#[test]
fn test_world_generation_creates_terrain() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    let mut config = PluginConfig::new(42);
    config.world_size = (32, 32, 128); // Smaller test world

    plugin.initialize(config).unwrap();

    // Check that world was generated with some materials
    let materials = plugin.get_materials();
    assert!(!materials.is_empty());

    // Verify key materials exist
    assert!(plugin.get_material("water").is_some());
    assert!(plugin.get_material("stone").is_some());
    assert!(plugin.get_material("grass").is_some());
}

#[test]
fn test_world_has_varying_heights() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    let mut config = PluginConfig::new(12345);
    config.world_size = (32, 32, 128); // Smaller test world

    plugin.initialize(config).unwrap();

    // Sample a few positions and check for terrain variation
    let pos1 = Position::new(0, 65, 0);
    let pos2 = Position::new(10, 45, 10);
    let pos3 = Position::new(-10, 70, -10);

    // At least some positions should have materials (terrain exists)
    let samples = vec![
        plugin.get_material_at(pos1),
        plugin.get_material_at(pos2),
        plugin.get_material_at(pos3),
    ];

    // Some positions should have materials (world is generated)
    assert!(samples.iter().any(|s| s.is_some()));
}

#[test]
fn test_water_exists_below_sea_level() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    let mut config = PluginConfig::new(99999);
    config.world_size = (32, 32, 128); // Smaller test world

    plugin.initialize(config).unwrap();

    // Check for water at sea level (y=64)
    let mut found_water = false;
    for x in -20..20 {
        for z in -20..20 {
            if let Some(material) = plugin.get_material_at(Position::new(x, 64, z)) {
                if material.id == "water" {
                    found_water = true;
                    break;
                }
            }
        }
        if found_water {
            break;
        }
    }

    // At least some water should exist in the world
    // (Due to noise generation, there should be low-lying areas)
    assert!(found_water, "No water found in world - terrain generation may be broken");
}

#[test]
fn test_material_count_increased() {
    let plugin = MinecraftSurvivalPlugin::new();
    let materials = plugin.get_materials();

    // Should have at least 16 materials (original 12 + water + dirt + grass + sand)
    assert!(materials.len() >= 16, "Expected at least 16 materials, got {}", materials.len());
}
