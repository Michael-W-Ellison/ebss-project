// src/analytics/tests/leatherworking_tests.rs
//! Leatherworking (#353): raw hides that rot unless worked, rawhide that
//! keeps, leather tanned soft over a fire, the garments a skin makes short of
//! a coat, rawhide that rots worn wet, and a hide laid down to sleep on.

use crate::agents::equipment::{ClothingMaterial, ClothingTemplate, EquipmentSlot};
use crate::agents::{Agent, AgentConfig, InventoryItem, Population, Quality};
use crate::analytics::Simulation;
use crate::environment::catalogue;
use crate::environment::{Action, HeatSourceType};
use crate::world::{Position, TerrainType, World, WorldConfig};

/// One grown person at (25, 25) on plains, nothing on them.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    for x in 15..36 {
        for y in 15..36 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(25);
    agent.state.position = (25, 25, 0);
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    simulation
}

fn some(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 1.0));
}

fn an_edge(simulation: &mut Simulation) {
    let edge = crate::environment::making::what_helps_with(crate::agents::SkillType::Leatherworking)
        .next()
        .expect("something scrapes a hide");
    some(simulation, edge.called, 1);
}

fn a_fire_here(simulation: &mut Simulation) {
    let fire = simulation
        .world
        .build_heat_source(HeatSourceType::Campfire, (25, 25, 0), None)
        .expect("a hearth goes here");
    let _ = simulation.world.add_fuel_to_heat_source(&fire, "wood".to_string(), 20.0);
    simulation.world.light_heat_source(&fire).expect("and it lights");
}

/// Rawhide and the skins' garments are born knowledge; tanning is found
/// out, and moccasins wait on it.
#[test]
fn the_catalogue_knows_how_leather_comes() {
    for born in ["make:rawhide", "wear:hide_cloak", "wear:hide_wraps", "wear:fur_wraps"] {
        assert!(catalogue::is_born_knowledge(born), "{born}");
    }
    assert!(!catalogue::is_born_knowledge("make:leather"));
    assert!(catalogue::what_comes_before("make:leather").is_empty(), "tanning is found out, not reached");
    assert_eq!(catalogue::what_comes_before("wear:moccasins"), ["make:leather"]);
}

/// A raw hide rots in about a week in the pack; rawhide keeps; and fresh
/// hides put with old ones are as fresh as the mix.
#[test]
fn a_raw_hide_rots_and_rawhide_keeps() {
    let mut simulation = one_person();
    some(&mut simulation, "hides", 2);
    some(&mut simulation, "rawhide", 2);
    let agent = &mut simulation.population.agents[0];
    let a_day = crate::environment::seasons::PLANNING_PERIODS_PER_DAY;

    for turn in 0..(6 * a_day) {
        agent.turn_food_spoilage(turn * 30);
    }
    assert_eq!(agent.how_many_i_have("hides"), 2, "six days and it is still a hide");
    let old = agent.inventory.get_item("hides").and_then(|item| item.current_durability).unwrap();

    // Two fresh off a kill, blended in
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight("hides".to_string(), 2, 1.0));
    let mixed = agent.inventory.get_item("hides").and_then(|item| item.current_durability).unwrap();
    assert!((mixed - (old + 1.0) / 2.0).abs() < 1e-4, "old {old}, mixed {mixed}");

    for turn in 0..(8 * a_day) {
        agent.turn_food_spoilage(turn * 30);
    }
    assert_eq!(agent.how_many_i_have("hides"), 0, "rotted");
    assert_eq!(agent.hides_that_rotted, 4);
    assert_eq!(agent.how_many_i_have("rawhide"), 2, "and rawhide keeps");
}

/// Scraping makes rawhide; six scraped and somebody finds out tanning, which
/// wants a fire and makes leather.
#[test]
fn scraping_teaches_tanning_and_tanning_wants_a_fire() {
    let mut simulation = one_person();
    an_edge(&mut simulation);
    some(&mut simulation, "hides", 6);
    let scrape = Action::Work { verb: "scrape".to_string(), to: "hides".to_string() };
    for done in 1..=6 {
        assert!(!simulation.population.agents[0].what_i_found_out().contains("leather"));
        let scraped = simulation.execute_action(&scrape, 0);
        assert!(scraped.success, "{done}: {:?}", scraped.message);
    }
    assert!(simulation.population.agents[0].how_many_i_have("rawhide") >= 6);
    assert!(simulation.population.agents[0].what_i_found_out().contains("leather"), "six and it is noticed");

    let tan = Action::Work { verb: "fire".to_string(), to: "rawhide".to_string() };
    assert!(!simulation.execute_action(&tan, 0).success, "no fire");
    a_fire_here(&mut simulation);
    let tanned = simulation.execute_action(&tan, 0);
    assert!(tanned.success, "{:?}", tanned.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("leather"), 1);
}

/// What is worn of rawhide rots three times as fast soaked; tanned leather
/// does not mind the wet.
#[test]
fn rawhide_rots_worn_wet() {
    let mut simulation = one_person();
    let body = &mut simulation.population.agents[0].body;
    body.equip(ClothingTemplate::from_id("hide_cloak", Quality::Common).unwrap());
    body.equip(ClothingTemplate::from_id("moccasins", Quality::Common).unwrap());
    let start = |slot: EquipmentSlot, body: &crate::agents::body::Body| body.equipment[&slot].durability;
    let (cloak, shoes) = (start(EquipmentSlot::Back, body), start(EquipmentSlot::Feet, body));
    assert_eq!(body.equipment[&EquipmentSlot::Back].material, ClothingMaterial::Rawhide);

    body.turn_equipment_wear_in(true);
    let worn = |slot: EquipmentSlot, was: f32, body: &crate::agents::body::Body| was - body.equipment[&slot].durability;
    assert!((worn(EquipmentSlot::Back, cloak, body) - 3.0 * worn(EquipmentSlot::Feet, shoes, body)).abs() < 1e-4);

    // Tanned wears longest of the skins
    assert!(ClothingMaterial::Leather.base_durability() > ClothingMaterial::Rawhide.base_durability());
}

/// A worked hide to spare goes down as bedding: twice as warm as boughs, a
/// season before it wears through, and the one a garment waits on is kept.
#[test]
fn a_hide_to_spare_is_slept_on() {
    let mut simulation = one_person();
    some(&mut simulation, "rawhide", 2);
    some(&mut simulation, "wood", 2);
    assert!(simulation.execute_action(&Action::LayBedding, 0).success);
    assert_eq!(simulation.population.agents[0].how_many_i_have("rawhide"), 2, "two is not one to spare");
    assert!(!simulation.world.bedding[0].of_hide);

    let mut simulation = one_person();
    some(&mut simulation, "rawhide", 3);
    assert!(simulation.execute_action(&Action::LayBedding, 0).success);
    assert_eq!(simulation.population.agents[0].how_many_i_have("rawhide"), 2);
    let laid = &simulation.world.bedding[0];
    assert!(laid.of_hide);
    assert_eq!(laid.what_it_is_worth(), 2.0 * Agent::WHAT_BEDDING_IS_WORTH);
    assert_eq!(laid.how_long_it_lasts(), crate::world::Bedding::HOW_LONG_A_HIDE_LASTS);
    assert_eq!(simulation.world.bedding_worth_at(&Position::new(25, 25)), 2.0 * Agent::WHAT_BEDDING_IS_WORTH);
}

/// The evening's skins: a raw hide scraped by whoever has an edge, and
/// rawhide tanned at a burning fire by whoever knows how.
#[test]
fn the_skins_are_seen_to_in_the_evening() {
    let mut simulation = one_person();
    some(&mut simulation, "hides", 1);
    let here = (25, 25, 0);
    let ask = |simulation: &Simulation| simulation.a_hide_to_work(&simulation.population.agents[0], here);
    assert_eq!(ask(&simulation), None, "no edge to scrape with");

    an_edge(&mut simulation);
    assert!(matches!(ask(&simulation), Some(Action::Work { ref verb, .. }) if verb == "scrape"), "got {:?}", ask(&simulation));

    let mut simulation = one_person();
    some(&mut simulation, "rawhide", 1);
    simulation.population.agents[0].found_out_how_to("leather");
    assert_eq!(ask(&simulation), None, "no fire");
    a_fire_here(&mut simulation);
    assert!(matches!(ask(&simulation), Some(Action::Work { ref verb, ref to }) if verb == "fire" && to == "rawhide"));
}
