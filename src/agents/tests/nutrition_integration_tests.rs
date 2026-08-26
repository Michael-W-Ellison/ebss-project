// src/agents/tests/nutrition_integration_tests.rs
//! Integration tests for the nutrition system

use crate::agents::{Agent, AgentConfig, InventoryItem};
use crate::world::nutrition::{
    FoodData, NutritionalContent, PreparationState, NutritionalState, EatResult,
};
use crate::core::DriveType;

#[test]
fn test_agent_starts_with_nutrition() {
    let agent = Agent::new(AgentConfig::default());

    // Agent should have nutritional state initialized
    assert!(agent.nutrition.energy_reserves > 0.0);
    assert!(agent.nutrition.protein_stores > 0.0);
    assert!(agent.nutrition.micronutrient_level > 0.0);
}

#[test]
fn test_eating_raw_food_less_effective() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.nutrition = NutritionalState {
        energy_reserves: 30.0,
        protein_stores: 30.0,
        micronutrient_level: 30.0,
        ticks_protein_deficit: 0,
        ticks_micronutrient_deficit: 0,
    };

    // Create raw meat (high protein, low utilization when raw)
    let raw_meat = FoodData::new(
        NutritionalContent::new(30.0, 50.0, 10.0, 0.6),
        PreparationState::Raw,
        1000,
        0,
    );

    let raw_item = InventoryItem::new_food(
        "raw_meat".to_string(),
        5,
        2.0,
        raw_meat,
    );

    agent.inventory.add_item(raw_item);
    let initial_protein = agent.nutrition.protein_stores;

    // Eat raw meat
    let result = agent.eat_food_item("raw_meat", 100);

    match result {
        EatResult::Success(nutrition) => {
            // Raw utilization is 35%, so 50 * 0.35 = 17.5 protein
            assert!(nutrition.protein < 20.0);
            assert!(nutrition.protein > 15.0);
        }
        _ => panic!("Expected success eating raw meat"),
    }

    assert!(agent.nutrition.protein_stores > initial_protein);
}

#[test]
fn test_eating_cooked_food_more_effective() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.nutrition = NutritionalState {
        energy_reserves: 30.0,
        protein_stores: 30.0,
        micronutrient_level: 30.0,
        ticks_protein_deficit: 0,
        ticks_micronutrient_deficit: 0,
    };

    // Create cooked meat (high utilization)
    let cooked_meat = FoodData::new(
        NutritionalContent::new(30.0, 50.0, 10.0, 0.6),
        PreparationState::Cooked,
        1000,
        0,
    );

    let cooked_item = InventoryItem::new_food(
        "cooked_meatportions".to_string(),
        5,
        2.0,
        cooked_meat,
    );

    agent.inventory.add_item(cooked_item);
    let initial_protein = agent.nutrition.protein_stores;

    // Eat cooked meat
    let result = agent.eat_food_item("cooked_meatportions", 100);

    match result {
        EatResult::Success(nutrition) => {
            // Cooked utilization is 95%, so 50 * 0.95 = 47.5 protein
            assert!(nutrition.protein > 45.0);
            assert!(nutrition.protein < 50.0);
        }
        _ => panic!("Expected success eating cooked meat"),
    }

    // Should gain more protein from cooked than raw
    let protein_gained = agent.nutrition.protein_stores - initial_protein;
    assert!(protein_gained > 40.0);
}

#[test]
fn test_spoiled_food_harmful() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.health = 100.0;

    // Create spoiled food
    let mut spoiled_food = FoodData::new(
        NutritionalContent::new(30.0, 50.0, 10.0, 0.6),
        PreparationState::Raw,
        1000,
        0,
    );
    spoiled_food.freshness = 0.0; // Completely spoiled

    let spoiled_item = InventoryItem::new_food(
        "spoiled_meat".to_string(),
        5,
        2.0,
        spoiled_food,
    );

    agent.inventory.add_item(spoiled_item);

    // Eating spoiled food should make sick
    let result = agent.eat_food_item("spoiled_meat", 100);

    match result {
        EatResult::MadeSick(damage) => {
            assert!(damage > 0.0);
            assert!(agent.state.health < 100.0);
        }
        _ => panic!("Expected to get sick from spoiled food"),
    }
}

#[test]
fn test_protein_deficiency_causes_health_loss() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.health = 100.0;

    // Set up severe protein deficiency
    agent.nutrition = NutritionalState {
        energy_reserves: 80.0,
        protein_stores: 5.0, // Very low
        micronutrient_level: 80.0,
        ticks_protein_deficit: 3000, // Well past threshold
        ticks_micronutrient_deficit: 0,
    };

    assert!(agent.nutrition.has_protein_deficiency());

    let penalty = agent.nutrition.deficiency_health_penalty();
    assert!(penalty > 0.0);
}

#[test]
fn test_micronutrient_deficiency_scurvy() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set up micronutrient deficiency
    agent.nutrition = NutritionalState {
        energy_reserves: 80.0,
        protein_stores: 80.0,
        micronutrient_level: 5.0, // Very low
        ticks_protein_deficit: 0,
        ticks_micronutrient_deficit: 6000, // Well past threshold
    };

    assert!(agent.nutrition.has_micronutrient_deficiency());

    let penalty = agent.nutrition.deficiency_health_penalty();
    assert!(penalty > 0.0);
}

#[test]
fn test_balanced_diet_maintains_health() {
    let mut agent = Agent::new(AgentConfig::default());

    // Start with moderate levels
    agent.nutrition = NutritionalState {
        energy_reserves: 60.0,
        protein_stores: 60.0,
        micronutrient_level: 60.0,
        ticks_protein_deficit: 0,
        ticks_micronutrient_deficit: 0,
    };

    // No deficiencies
    assert!(!agent.nutrition.has_protein_deficiency());
    assert!(!agent.nutrition.has_micronutrient_deficiency());
    assert_eq!(agent.nutrition.deficiency_health_penalty(), 0.0);
}

#[test]
fn test_food_spoilage_in_inventory() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add fresh food
    let fresh_food = FoodData::new(
        NutritionalContent::new(30.0, 10.0, 20.0, 0.8),
        PreparationState::Raw,
        100, // Spoils quickly
        0,
    );

    let food_item = InventoryItem::new_food(
        "berries".to_string(),
        10,
        0.5,
        fresh_food,
    );

    agent.inventory.add_item(food_item);

    // Check food is fresh
    let item = agent.inventory.get_item("berries").unwrap();
    assert!(item.food_data.as_ref().unwrap().freshness > 0.9);

    // Simulate time passing
    agent.tick_food_spoilage(50);

    // Food should have degraded
    let item = agent.inventory.get_item("berries").unwrap();
    assert!(item.food_data.as_ref().unwrap().freshness < 0.6);

    // More time - should be spoiled and removed
    agent.tick_food_spoilage(150);

    // Food should be removed from inventory
    assert!(agent.inventory.get_item("berries").is_none());
}

#[test]
fn test_dried_food_lasts_longer_in_inventory() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add dried food (20x slower spoilage)
    let dried_food = FoodData::new(
        NutritionalContent::new(30.0, 50.0, 10.0, 0.1),
        PreparationState::Dried,
        100, // Base 100 ticks, but dried = 2000 effective
        0,
    );

    let food_item = InventoryItem::new_food(
        "dried_meat".to_string(),
        5,
        1.5,
        dried_food,
    );

    agent.inventory.add_item(food_item);

    // Simulate significant time passing
    agent.tick_food_spoilage(100);

    // Dried food should still be mostly fresh
    let item = agent.inventory.get_item("dried_meat").unwrap();
    assert!(item.food_data.as_ref().unwrap().freshness > 0.9);
}

#[test]
fn test_find_best_food_prioritizes_needs() {
    let mut agent = Agent::new(AgentConfig::default());

    // Agent needs energy most
    agent.nutrition = NutritionalState {
        energy_reserves: 10.0, // Very low
        protein_stores: 80.0,
        micronutrient_level: 80.0,
        ticks_protein_deficit: 0,
        ticks_micronutrient_deficit: 0,
    };

    // Add high-protein food
    let meat = FoodData::new(
        NutritionalContent::new(30.0, 50.0, 10.0, 0.6),
        PreparationState::Cooked,
        1000,
        0,
    );
    agent.inventory.add_item(InventoryItem::new_food(
        "meatportions".to_string(), 5, 2.0, meat,
    ));

    // Add high-energy food (grain/bread)
    let bread = FoodData::new(
        NutritionalContent::new(55.0, 12.0, 10.0, 0.3),
        PreparationState::Cooked,
        1000,
        0,
    );
    agent.inventory.add_item(InventoryItem::new_food(
        "bread".to_string(), 5, 0.5, bread,
    ));

    // Should prefer bread (high energy) over meat (high protein)
    let best = agent.find_best_food_to_eat();
    assert_eq!(best, Some("bread".to_string()));
}

#[test]
fn test_nutrition_metabolism_depletes_over_time() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.nutrition = NutritionalState::full();

    let initial_energy = agent.nutrition.energy_reserves;
    let initial_protein = agent.nutrition.protein_stores;

    // Simulate 100 ticks of metabolism
    for _ in 0..100 {
        agent.tick_nutrition(0);
    }

    // Energy should have depleted
    assert!(agent.nutrition.energy_reserves < initial_energy);

    // Protein depletes slower
    assert!(agent.nutrition.protein_stores < initial_protein);
    assert!(agent.nutrition.protein_stores > agent.nutrition.energy_reserves);
}

#[test]
fn test_eating_satisfies_hunger_drive() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set high hunger
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.8;
    }

    // Add food
    let food = FoodData::new(
        NutritionalContent::new(50.0, 20.0, 30.0, 0.7),
        PreparationState::Cooked,
        1000,
        0,
    );
    agent.inventory.add_item(InventoryItem::new_food(
        "meal".to_string(), 3, 1.0, food,
    ));

    let initial_hunger = agent.drives.get(DriveType::Hunger).unwrap().value;

    // Eat
    agent.eat_food_item("meal", 100);

    // Hunger should decrease
    let final_hunger = agent.drives.get(DriveType::Hunger).unwrap().value;
    assert!(final_hunger < initial_hunger);
}

#[test]
fn test_food_with_water_satisfies_thirst() {
    let mut agent = Agent::new(AgentConfig::default());

    // Set moderate thirst
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.6;
    }

    // Add juicy food (high water content)
    let fruit = FoodData::new(
        NutritionalContent::new(20.0, 2.0, 35.0, 0.85), // High water
        PreparationState::Raw, // Raw fruit is fine
        500,
        0,
    );
    agent.inventory.add_item(InventoryItem::new_food(
        "fruit".to_string(), 5, 0.3, fruit,
    ));

    let initial_thirst = agent.drives.get(DriveType::Thirst).unwrap().value;

    // Eat fruit
    agent.eat_food_item("fruit", 100);

    // Thirst should decrease (water content > 0.3)
    let final_thirst = agent.drives.get(DriveType::Thirst).unwrap().value;
    assert!(final_thirst < initial_thirst);
}
