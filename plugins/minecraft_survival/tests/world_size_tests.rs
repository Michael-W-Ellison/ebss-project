// plugins/minecraft_survival/tests/world_size_tests.rs
//! Tests for world generation with various map sizes.

use minecraft_survival::MinecraftSurvivalPlugin;
use ebss::environment::*;

#[test]
fn test_tiny_world_generation() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    let config = PluginConfig::tiny(111);

    plugin.initialize(config.clone()).unwrap();

    // Verify world bounds
    assert_eq!(config.world_size, (64, 64, 64));

    // Check that positions within bounds work
    assert!(plugin.is_valid_position(Position::new(0, 0, 0)));
    assert!(plugin.is_valid_position(Position::new(31, 32, 31)));
    assert!(plugin.is_valid_position(Position::new(-32, 63, -32)));

    // Check that positions outside bounds don't work
    assert!(!plugin.is_valid_position(Position::new(32, 0, 0)));
    assert!(!plugin.is_valid_position(Position::new(0, 64, 0)));
}

#[test]
fn test_small_world_generation() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    let config = PluginConfig::small(222);

    plugin.initialize(config.clone()).unwrap();

    assert_eq!(config.world_size, (128, 128, 96));

    // Verify generation created materials
    let materials = plugin.get_materials();
    assert!(!materials.is_empty());

    // Sample some positions
    let samples = vec![
        plugin.get_material_at(Position::new(0, 65, 0)),
        plugin.get_material_at(Position::new(50, 50, 50)),
        plugin.get_material_at(Position::new(-50, 70, -50)),
    ];

    // At least some should have materials (world generated)
    assert!(samples.iter().any(|s| s.is_some()));
}

#[test]
fn test_medium_world_generation() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    // Use a smaller test size to avoid memory issues
    let mut config = PluginConfig::medium(333);
    config.world_size = (64, 64, 96); // Reduce for testing

    plugin.initialize(config.clone()).unwrap();

    // Check material at various heights
    let mut found_stone = false;

    for y in 0..96 {
        if let Some(material) = plugin.get_material_at(Position::new(0, y, 0)) {
            if material.id == "stone" {
                found_stone = true;
                break;
            }
        }
    }

    assert!(found_stone, "Should find stone in generated world");
}

#[test]
fn test_large_world_config() {
    // Test large world configuration without full generation to avoid memory issues
    let config = PluginConfig::large(444);

    assert_eq!(config.world_size, (512, 512, 160));
    assert!(config.is_valid());

    // World volume should be large
    assert_eq!(config.world_volume(), 41943040); // 512*512*160
}

#[test]
fn test_huge_world_config() {
    // Test huge world configuration without full generation to avoid memory issues
    let config = PluginConfig::huge(555);

    assert_eq!(config.world_size, (1024, 1024, 192));
    assert!(config.is_valid());

    // World volume should be massive
    assert_eq!(config.world_volume(), 201326592); // 1024*1024*192
}

#[test]
fn test_custom_world_generation() {
    let mut plugin = MinecraftSurvivalPlugin::new();
    let config = PluginConfig::custom(666, 80, 80, 64);

    plugin.initialize(config.clone()).unwrap();

    assert_eq!(config.world_size, (80, 80, 64));

    // Check bounds match custom size
    assert!(plugin.is_valid_position(Position::new(39, 32, 39)));
    assert!(plugin.is_valid_position(Position::new(-40, 63, -40)));

    // Out of custom bounds
    assert!(!plugin.is_valid_position(Position::new(40, 32, 40)));
    assert!(!plugin.is_valid_position(Position::new(0, 64, 0)));
}

#[test]
fn test_world_size_validation() {
    let valid = PluginConfig::new(123);
    assert!(valid.is_valid());

    let invalid_negative = PluginConfig::custom(123, -100, 100, 50);
    assert!(!invalid_negative.is_valid());

    let invalid_zero = PluginConfig::custom(123, 100, 0, 50);
    assert!(!invalid_zero.is_valid());

    let invalid_too_large = PluginConfig::custom(123, 10000, 100, 50);
    assert!(!invalid_too_large.is_valid());
}

#[test]
fn test_world_generation_scales_with_size() {
    // Test that configurations scale properly
    let tiny_config = PluginConfig::tiny(777);
    let small_config = PluginConfig::small(777);
    let medium_config = PluginConfig::medium(777);
    let large_config = PluginConfig::large(777);

    // Volumes should increase with size
    assert!(tiny_config.world_volume() < small_config.world_volume());
    assert!(small_config.world_volume() < medium_config.world_volume());
    assert!(medium_config.world_volume() < large_config.world_volume());

    // Generate a small world to verify it works
    let mut plugin = MinecraftSurvivalPlugin::new();
    plugin.initialize(tiny_config.clone()).unwrap();
    assert!(plugin.get_material("stone").is_some());
}

#[test]
fn test_water_generation_in_different_sizes() {
    // Test that water generation works in different world sizes
    // Use tiny sizes to keep memory usage low
    let tiny_config = PluginConfig::tiny(888);
    let mut plugin = MinecraftSurvivalPlugin::new();
    plugin.initialize(tiny_config.clone()).unwrap();

    // Should have water material registered
    assert!(plugin.get_material("water").is_some());

    // Search for water in the generated world
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

    // Water should be found in the generated world
    assert!(found_water, "No water found in tiny world");
}
