// src/analytics/tests/pottery_tests.rs
//! Pottery (#355): a fired pot to stew in, to keep food in and to carry
//! water; one that cracks over the fire and breaks on a move; one shaped and
//! not fired that goes back to mud in the wet; and a pot made by the evening
//! fire by whoever knows how.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::{Action, HeatSourceType};
use crate::world::nutrition::PreparationState;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// One grown person at (25, 25) on dry plains with nothing on them.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    for x in 0..world.grid.width as i32 {
        for y in 0..world.grid.height as i32 {
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

/// Fired pots, with this much water in them.
fn pots(simulation: &mut Simulation, how_many: u32, water: f32) {
    let mut pot = InventoryItem::new_container(
        "stoneware".to_string(),
        how_many,
        crate::environment::making::WHAT_A_FIRED_POT_HOLDS,
    );
    pot.fill_level = Some(water);
    simulation.population.agents[0].inventory.add_item(pot);
}

/// Raw roots, with their food clock started.
fn raw_roots(simulation: &mut Simulation, how_many: u32) {
    let food = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Roots, simulation.current_turn);
    let mut roots = InventoryItem::new_with_weight("roots".to_string(), how_many, 1.0);
    roots.food_data = food;
    simulation.population.agents[0].inventory.add_item(roots);
}

fn a_fire_here(simulation: &mut Simulation) {
    let fire = simulation
        .world
        .build_heat_source(HeatSourceType::Campfire, (25, 25, 0), None)
        .expect("a hearth goes here");
    let _ = simulation.world.add_fuel_to_heat_source(&fire, "wood".to_string(), 200.0);
    simulation.world.light_heat_source(&fire).expect("and it lights");
}

/// In a fired pot of water the whole batch stews - nothing burns, and more
/// is got out of it than over the flames - and the water goes into it.
#[test]
fn a_pot_of_water_stews_the_lot() {
    let mut simulation = one_person();
    a_fire_here(&mut simulation);
    pots(&mut simulation, 1, 4.0);
    raw_roots(&mut simulation, 10);

    let cooked = simulation.execute_action(&Action::Cook { food_type: "generic".to_string() }, 0);
    assert!(cooked.success, "{:?}", cooked.message);
    let agent = &simulation.population.agents[0];
    assert_eq!(agent.how_many_i_have("stewed_roots"), 10, "the whole batch, none of it burnt");
    let stew = agent.inventory.get_item("stewed_roots").and_then(|item| item.food_data.as_ref()).unwrap();
    assert_eq!(stew.preparation, PreparationState::Stewed);
    assert!(
        PreparationState::Stewed.utilization_multiplier() > PreparationState::Cooked.utilization_multiplier(),
        "more is got out of a stew"
    );
    assert!((agent.how_much_water_i_carry() - 3.0).abs() < 1e-4, "a pot of water went into it");

    // And with no pot it is over the flames, a handful at a time
    let mut simulation = one_person();
    a_fire_here(&mut simulation);
    raw_roots(&mut simulation, 10);
    let _ = simulation.execute_action(&Action::Cook { food_type: "generic".to_string() }, 0);
    assert_eq!(simulation.population.agents[0].how_many_i_have("stewed_roots"), 0);
}

/// A fired pot breaks: one at a time, keeping the others' share of the
/// water, and counted.
#[test]
fn a_pot_breaks_one_at_a_time() {
    let mut simulation = one_person();
    pots(&mut simulation, 2, 6.0);
    let agent = &mut simulation.population.agents[0];
    assert!(agent.a_pot_breaks());
    assert_eq!(agent.how_many_i_have("stoneware"), 1);
    assert!((agent.how_much_water_i_carry() - 3.0).abs() < 1e-4);
    assert!(agent.a_pot_breaks());
    assert!(!agent.a_pot_breaks(), "none left");
    assert_eq!(agent.pots_broken, 2);
}

/// A pot shaped and not fired goes back to mud on somebody soaked through.
#[test]
fn an_unfired_pot_slumps_in_the_wet() {
    let mut simulation = one_person();
    some(&mut simulation, "claypot", 2);
    pots(&mut simulation, 1, 0.0);
    let agent = &mut simulation.population.agents[0];
    agent.turn_with_percepts(0);
    assert_eq!(agent.how_many_i_have("claypot"), 2, "dry, it keeps its shape");

    agent.exposure_status.wetness = 0.9;
    agent.turn_with_percepts(1);
    assert_eq!(agent.how_many_i_have("claypot"), 0);
    assert_eq!(agent.how_many_i_have("clay"), 2);
    assert_eq!(agent.how_many_i_have("stoneware"), 1, "a fired one does not mind");
}

/// Pots carried on a camp move do not all get there.
#[test]
fn pots_break_on_a_move() {
    let mut simulation = one_person();
    pots(&mut simulation, 40, 0.0);
    let who = simulation.population.agents[0].id;
    simulation.the_camp_moves(crate::analytics::wanting::camp::CampMove {
        who,
        to: (40, 40),
        feeds: None,
        why: "a test",
    });
    let left = simulation.population.agents[0].how_many_i_have("stoneware");
    assert!(left < 40 && left > 20, "about one in five broke: {left} of 40 left");
}

/// By the evening fire, whoever knows how makes pots until they have two:
/// fires a shaped one, shapes clay, or fetches it.
#[test]
fn a_pot_is_made_by_the_evening_fire() {
    let mut simulation = one_person();
    let here = (25, 25, 0);
    let ask = |simulation: &Simulation| simulation.a_pot_to_make(&simulation.population.agents[0], here);
    some(&mut simulation, "clay", 2);
    assert_eq!(ask(&simulation), None, "nobody has found out a pot");

    simulation.population.agents[0].found_out_how_to("claypot");
    simulation.population.agents[0].found_out_how_to("stoneware");
    assert!(matches!(ask(&simulation), Some(Action::Work { ref verb, ref to }) if verb == "mold" && to == "clay"));

    simulation.population.agents[0].inventory.remove_item("clay", 2);
    some(&mut simulation, "claypot", 1);
    assert_eq!(ask(&simulation), None, "a shaped pot waits for a fire");
    a_fire_here(&mut simulation);
    assert!(matches!(ask(&simulation), Some(Action::Work { ref verb, ref to }) if verb == "fire" && to == "claypot"));

    simulation.population.agents[0].inventory.remove_item("claypot", 1);
    simulation.world.resources.push(ResourceNode::new(ResourceType::Clay, Position::new(30, 25), 50));
    assert!(matches!(ask(&simulation), Some(Action::Gather { ref resource_type }) if resource_type == "clay"));

    pots(&mut simulation, 2, 0.0);
    assert_eq!(ask(&simulation), None, "two is enough");
}

/// An empty pot boils the sea: it is filled where the sea is.
#[test]
fn an_empty_pot_boils_the_sea() {
    let mut simulation = one_person();
    if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(27, 25)) {
        tile.terrain.terrain_type = TerrainType::Sea;
    }
    a_fire_here(&mut simulation);
    pots(&mut simulation, 1, 0.0);
    let boiled = simulation.execute_action(&Action::Boil, 0);
    assert!(boiled.success, "{:?}", boiled.message);
    assert!(simulation.population.agents[0].how_many_i_have("salt") > 0);
}
