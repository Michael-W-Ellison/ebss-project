// src/analytics/tests/hoe_tests.rs
//! Digging sticks and hoes (ISSUES_FOUND #385).
//!
//! - breaking a season's ground for a garden is how the hoe is found out
//! - somebody who keeps a garden wants a tool for it, somebody who does not
//!   does not, and a gardener who knows the hoe would rather have one than a
//!   stick
//! - a hoe weeds more of a field in a turn than a stick, and a stick more
//!   than bare hands

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::making::{GROUND_BROKEN_FOR_A_GARDEN, TIMES_GROUND_IS_BROKEN_BEFORE_A_HOE};
use crate::environment::Action;
use crate::world::soil::Soil;
use crate::world::{Position, Terrain, TerrainType, World, WorldConfig};

fn a_farmer_on_a_field(where_it_is: Position) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.retain(|resource| resource.position != where_it_is);
    if let Some(tile) = world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Farmland);
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.position = (where_it_is.x, where_it_is.y, 0);
    agent.inventory.get_all_items_mut().clear();
    simulation
}

fn give(simulation: &mut Simulation, called: &str) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new(called.to_string(), 1));
}

/// Breaking ground for a garden often enough is how the hoe is found out.
#[test]
fn breaking_ground_often_enough_is_how_the_hoe_is_found_out() {
    let mut simulation = a_farmer_on_a_field(Position::new(25, 25));
    simulation.population.agents[0].found_out_how_to("do:ScatterSeed");
    simulation.population.agents[0].found_out_how_to("do:TillSoil");
    for x in 0..TIMES_GROUND_IS_BROKEN_BEFORE_A_HOE as i32 {
        assert!(!simulation.population.agents[0].knows_how_to_make("hoe"));
        let at = Position::new(30 + x, 30);
        simulation.world.resources.retain(|resource| resource.position != at);
        if let Some(tile) = simulation.world.grid.get_tile_mut(&at) {
            tile.terrain = Terrain::new(TerrainType::Plains);
        }
        simulation.population.agents[0].state.position = (at.x, at.y, 0);
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight("grain".to_string(), 3, 1.0));
        let result = simulation.execute_action(&Action::TillSoil, 0);
        assert!(result.success, "{:?}", result.message);
    }
    assert_eq!(
        simulation.population.agents[0].skills.times_made(GROUND_BROKEN_FOR_A_GARDEN),
        TIMES_GROUND_IS_BROKEN_BEFORE_A_HOE
    );
    assert!(simulation.population.agents[0].knows_how_to_make("hoe"));
}

/// Nobody who has not broken a season's ground can come to the hoe any
/// other way, as a likeness of the shovel it shares its makings with.
#[test]
fn the_hoe_cannot_occur_to_anybody_who_has_not_gardened() {
    let mut simulation = a_farmer_on_a_field(Position::new(25, 25));
    let could = |simulation: &Simulation| {
        simulation.population.agents[0].what_i_could_find_out().any(|step| step.makes == "hoe")
    };
    assert!(!could(&simulation));
    for _ in 0..TIMES_GROUND_IS_BROKEN_BEFORE_A_HOE {
        simulation.population.agents[0].skills.made_one(GROUND_BROKEN_FOR_A_GARDEN);
    }
    assert!(could(&simulation));
}

/// A gardener wants a tool for the garden; somebody who keeps none does not;
/// and a gardener who knows the hoe would rather have it than a stick.
#[test]
fn a_gardener_wants_a_tool_for_it_and_would_rather_a_hoe() {
    let mut simulation = a_farmer_on_a_field(Position::new(25, 25));
    let wants_for_farming =
        |simulation: &Simulation| simulation.population.agents[0].trades_i_want_tools_for().any(|t| t == SkillType::Farming);
    assert!(!wants_for_farming(&simulation), "nobody who keeps no garden");

    simulation.population.agents[0].found_out_how_to("do:ScatterSeed");
    simulation.population.agents[0].found_out_how_to("do:TillSoil");
    assert!(wants_for_farming(&simulation));
    assert_eq!(
        simulation.population.agents[0].what_i_would_rather_have(SkillType::Farming).map(|tool| tool.called),
        Some("diggingstick"),
        "bare-handed, a stick"
    );

    give(&mut simulation, "diggingstick");
    simulation.population.agents[0].found_out_how_to("hoe");
    assert_eq!(
        simulation.population.agents[0].what_i_would_rather_have(SkillType::Farming).map(|tool| tool.called),
        Some("hoe"),
        "with a stick, and knowing the hoe, a hoe"
    );
}

/// A hoe gets round more of an overgrown field in a turn than a stick, and a
/// stick more than bare hands.
#[test]
fn a_hoe_weeds_more_than_a_stick_and_a_stick_more_than_hands() {
    fn cleared(with: Option<&str>) -> f32 {
        let where_it_is = Position::new(25, 25);
        let mut simulation = a_farmer_on_a_field(where_it_is);
        simulation.population.agents[0].skills.practise(SkillType::Farming, 2_000, 0);
        if let Some(called) = with {
            give(&mut simulation, called);
        }
        if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
            tile.soil.weeds = Soil::OVERRUN;
            tile.soil.pests = Soil::OVERRUN;
        }
        simulation.execute_action(&Action::TendField, 0);
        2.0 * Soil::OVERRUN
            - simulation
                .world
                .grid
                .get_tile(&where_it_is)
                .map(|tile| tile.soil.weeds + tile.soil.pests)
                .unwrap_or(0.0)
    }
    let hands = cleared(None);
    let stick = cleared(Some("diggingstick"));
    let hoe = cleared(Some("hoe"));
    assert!(stick >= hands, "{stick:.2} against {hands:.2}");
    assert!(hoe > hands, "{hoe:.2} against {hands:.2}");
    assert!(hoe >= stick, "{hoe:.2} against {stick:.2}");
}
