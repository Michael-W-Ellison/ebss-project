// src/analytics/tests/rotation_tests.rs
//! Tests for the crop that pays the ground rent.
//!
//! "Legumes: Beans, peas, lentils, chickpeas. Cover crops / green manure:
//! clover, vetch, alfalfa, rye."
//!
//! Every growing thing in this model was a withdrawal. `regenerate_in_ground`
//! ends by taking `NUTRIENT_PER_UNIT_GROWN` out of the soil for each unit that
//! came up, and the only deposits anywhere were muck, litter and what people
//! dropped - so a field was an account one crop drew on and almost nothing
//! paid into. Measured over a settlement's first summer, the ground under its
//! fields fell from 0.60 to 0.27.
//!
//! A legume is the exception, and it is not a fudge. It fixes nitrogen out of
//! the air through the bacteria in its roots, so what it builds itself out of
//! never came from the bank, and what is left when it is done is more than was
//! there before. That one fact is what makes a rotation worth knowing.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::soil::Soil;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// A patch of ground with a known amount in it, and a crop standing on it.
fn a_field_of(crop: ResourceType, nutrients: f32) -> (Simulation, Position) {
    let mut world = World::new(WorldConfig::default().with_size(40, 40));
    world.animals.get_all_mut().clear();
    world.resources.clear();

    let here = Position::new(20, 20);
    if let Some(tile) = world.grid.get_tile_mut(&here) {
        tile.terrain = crate::world::Terrain::new(TerrainType::Farmland);
        tile.soil.nutrients = nutrients;
        tile.soil.weeds = 0.0;
        tile.soil.pests = 0.0;
    }

    let mut node = ResourceNode::new(crop, here.clone(), 60);
    node.amount = 0;
    world.resources.push(node);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (20, 20, 0);
    (simulation, here)
}

fn what_the_ground_holds(simulation: &Simulation, where_it_is: &Position) -> f32 {
    simulation
        .world
        .grid
        .get_tile(where_it_is)
        .map(|tile| tile.soil.nutrients)
        .unwrap_or(0.0)
}

// --- the crop that gives back -----------------------------------------------

/// Growing wheat costs the ground; growing beans does not.
///
/// The same tile, the same starting nutrient, the same number of ticks, and
/// the only difference is which plant is standing on it.
#[test]
fn a_pod_row_leaves_the_ground_better_than_it_found_it() {
    const STARTED_AT: f32 = 0.6;

    let (mut wheat, field) = a_field_of(ResourceType::Grain, STARTED_AT);
    let (mut beans, bean_field) = a_field_of(ResourceType::Legumes, STARTED_AT);

    // Both crops bear in the late summer, which is when this has to be asked:
    // out of season nothing grows and nothing is taken.
    let midsummer = crate::environment::seasons::Season::Summer.first_day()
        + crate::environment::seasons::DAYS_PER_SEASON - 10;
    for simulation in [&mut wheat, &mut beans] {
        simulation.world.climate.calendar.day_of_year = midsummer;
    }

    for _ in 0..600 {
        wheat.world.tick();
        beans.world.tick();
    }

    let after_wheat = what_the_ground_holds(&wheat, &field);
    let after_beans = what_the_ground_holds(&beans, &bean_field);

    assert!(
        after_wheat < STARTED_AT,
        "a hungry crop takes something out: {STARTED_AT} -> {after_wheat}"
    );
    assert!(
        after_beans > after_wheat,
        "and a pod row does not: beans left {after_beans}, wheat left {after_wheat}"
    );
    assert!(
        after_beans >= STARTED_AT,
        "a bean row is not a rest, it is a crop that pays rent: \
         {STARTED_AT} -> {after_beans}"
    );
}

/// And the ledger balances by construction: what a pod crop fixes per unit
/// grown is exactly what an ordinary crop takes, so a year of beans and a year
/// of wheat cancel. That is the whole of a two-course rotation, written as one
/// number.
#[test]
fn a_year_of_beans_answers_a_year_of_wheat() {
    assert_eq!(
        Soil::WHAT_A_LEGUME_FIXES_PER_UNIT_GROWN,
        Soil::NUTRIENT_PER_UNIT_GROWN,
        "the two sides of a rotation are meant to be equal and opposite"
    );
}

/// Ground already full takes nothing more, so nobody makes an infinite larder
/// out of one tile by leaving beans on it.
#[test]
fn ground_that_is_full_takes_no_more() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);
    soil.nutrients = Soil::MAX_NUTRIENTS;

    assert_eq!(soil.feed(0.5), 0.0, "full ground takes nothing");
    assert_eq!(soil.nutrients, Soil::MAX_NUTRIENTS);

    soil.nutrients = Soil::MAX_NUTRIENTS - 0.1;
    let taken = soil.feed(0.5);
    assert!((taken - 0.1).abs() < 1e-5, "it takes what there is room for: {taken}");
    assert_eq!(soil.nutrients, Soil::MAX_NUTRIENTS);
}

/// One answer to which crops feed the ground, and nothing else claims to.
#[test]
fn only_a_pod_crop_feeds_the_ground() {
    assert!(ResourceType::Legumes.feeds_the_ground());

    for other in ResourceType::all() {
        if other == ResourceType::Legumes {
            continue;
        }
        assert!(
            !other.feeds_the_ground(),
            "{other:?} claims to feed the ground and nothing has been written \
             to make it do so"
        );
    }

    // And it is a crop in every other respect - food, grown, and it comes
    // back. A ground-feeder that fell through those lists would be deleted
    // out of season the way the mast was; see ISSUES_FOUND.md #164.
    assert!(ResourceType::Legumes.is_it_food());
    assert!(ResourceType::Legumes.is_it_grown());
    assert!(ResourceType::Legumes.how_fast_it_comes_back() > 0.0);
}

// --- green manure -----------------------------------------------------------

/// Turning a standing crop under gives the ground the part that would
/// otherwise have been carried away in a basket.
#[test]
fn ploughing_a_crop_in_feeds_the_ground_it_stood_on() {
    let (mut simulation, field) = a_field_of(ResourceType::Legumes, 0.2);

    // Something worth turning under
    if let Some(node) = simulation.world.resources.first_mut() {
        node.amount = 40;
    }

    let before = what_the_ground_holds(&simulation, &field);
    let litter_before = simulation
        .world
        .grid
        .get_tile(&field)
        .map(|tile| tile.soil.leaf_litter)
        .unwrap_or(0.0);

    let result = simulation.execute_action(&crate::environment::Action::TillSoil, 0);
    assert!(result.success, "{}", result.message.clone().unwrap_or_default());

    assert!(
        what_the_ground_holds(&simulation, &field) > before,
        "the crop went into the ground: {before} -> {}",
        what_the_ground_holds(&simulation, &field)
    );

    let litter_after = simulation
        .world
        .grid
        .get_tile(&field)
        .map(|tile| tile.soil.leaf_litter)
        .unwrap_or(0.0);
    assert!(litter_after > litter_before, "and so did the haulm");

    // And the crop is gone, because that is what it cost.
    assert!(
        !simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == field),
        "what was turned under is not still standing there to be eaten"
    );

    // The ground is broken now, which is the other half of the day's work.
    assert!(
        simulation
            .world
            .grid
            .get_tile(&field)
            .map(|tile| tile.terrain.is_cultivated())
            .unwrap_or(false),
        "and the ground is broken"
    );
}

/// A berry bush is not a green manure. Breaking ground that carries something
/// which does not feed the ground is still refused.
#[test]
fn a_hungry_crop_is_not_turned_under() {
    let (mut simulation, _field) = a_field_of(ResourceType::Food, 0.2);
    if let Some(node) = simulation.world.resources.first_mut() {
        node.amount = 40;
    }

    let result = simulation.execute_action(&crate::environment::Action::TillSoil, 0);
    assert!(
        !result.success,
        "a bush somebody could be eating off is not ploughed in"
    );
    assert_eq!(simulation.world.resources.len(), 1, "and it is still standing");
}

// --- the reading ------------------------------------------------------------

/// A man with both in his pack puts pods in tired ground and wheat in good.
#[test]
fn tired_ground_gets_the_pod_row() {
    use crate::agents::{Agent, InventoryItem};

    let mut agent = Agent::new(AgentConfig::default());
    agent.inventory.add_item(InventoryItem::new_with_weight("grain".to_string(), 10, 0.1));
    agent.inventory.add_item(InventoryItem::new_with_weight("legumes".to_string(), 10, 0.1));

    let on_good_ground = Simulation::what_this_one_would_sow(&agent, 0.9);
    let on_tired_ground = Simulation::what_this_one_would_sow(&agent, 0.1);

    assert_eq!(
        on_good_ground,
        ResourceType::Grain,
        "good ground carries a harvest and a hungry people wants one"
    );
    assert_eq!(
        on_tired_ground,
        ResourceType::Legumes,
        "and worked-out ground is where a pod row is worth more than a thin crop"
    );
}

/// It is a reading and not a rule: a man who has sown wheat here three years
/// running and carried it home each time goes on sowing wheat.
#[test]
fn a_settled_opinion_still_beats_the_reading() {
    use crate::agents::{Agent, InventoryItem};

    let mut agent = Agent::new(AgentConfig::default());
    agent.inventory.add_item(InventoryItem::new_with_weight("grain".to_string(), 10, 0.1));
    agent.inventory.add_item(InventoryItem::new_with_weight("legumes".to_string(), 10, 0.1));

    for _ in 0..12 {
        agent.lessons.record_particular("sow:grain", true);
        agent.lessons.record_particular("sow:legumes", false);
    }

    assert_eq!(
        Simulation::what_this_one_would_sow(&agent, 0.1),
        ResourceType::Grain,
        "what a man has actually seen work outweighs the look of the ground"
    );
}
