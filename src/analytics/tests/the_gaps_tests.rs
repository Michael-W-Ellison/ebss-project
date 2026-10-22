// src/analytics/tests/the_gaps_tests.rs
//! The four gaps left by the survival machinery (#352): wind that blows,
//! salt fetched from where it lies, skins wanted for a tent, and a vessel
//! made before a walk.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::Season;
use crate::environment::{Action, WeatherGenerator};
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// One grown person at (25, 25), nothing on them, on plains with no water,
/// salt or animals anywhere.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.pits.clear();
    world
        .resources
        .retain(|node| !matches!(node.resource_type, ResourceType::Water | ResourceType::Salt));
    for x in 0..200 {
        for y in 0..200 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }
    world.file_the_nodes();
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
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.5));
}

fn a_node_at(simulation: &mut Simulation, what: ResourceType, at: Position) {
    simulation.world.resources.push(ResourceNode::new(what, at, 50));
    simulation.world.file_the_nodes();
}

/// Wind blows by the season: a winter gale passes what a windbreak and
/// windburn want, and a summer spell never passes 8 before the weather
/// itself makes it worse.
#[test]
fn the_wind_blows_by_the_season() {
    let spells = |season: Season| {
        let mut generator = WeatherGenerator::new(season, false, false);
        (0..2000)
            .map(|_| generator.generate_weather())
            .filter(|weather| weather.weather_type != crate::environment::WeatherType::Fog)
            .map(|weather| weather.base_wind_speed)
            .collect::<Vec<f32>>()
    };
    let winter = spells(Season::Winter);
    let summer = spells(Season::Summer);
    let over = |winds: &[f32], above: f32| winds.iter().filter(|wind| **wind > above).count();

    assert!(over(&winter, 8.0) > winter.len() / 10, "a winter wind worth a windbreak, sometimes");
    assert!(over(&winter, 10.0) > 0, "and a gale to burn the skin");
    assert!(over(&summer, 8.0) == 0, "no summer wind passes 8 before the weather has its say");
    let mean = |winds: &[f32]| winds.iter().sum::<f32>() / winds.len() as f32;
    assert!(mean(&winter) > mean(&summer));
    assert!(winter.iter().any(|wind| *wind < 2.0), "and still days in winter too");
}

/// Skins for a tent are asked for the way the builder asks: leather and
/// cord count, a branch is made into cord, and a want of skins is a reason
/// to hunt in itself.
#[test]
fn a_tent_is_asked_for_by_what_it_takes() {
    let mut simulation = one_person();
    some(&mut simulation, "wood", 8);
    some(&mut simulation, "leather", 4);
    some(&mut simulation, "lashing", 2);
    let asked = simulation.raising_a_roof(&simulation.population.agents[0], (25, 25, 0));
    assert!(matches!(asked, Some(Action::Build { ref structure_type, .. }) if structure_type == "tent"), "leather is a covering: {asked:?}");

    // Short of cord, with a branch to spare: cord from its bark
    let mut simulation = one_person();
    some(&mut simulation, "wood", 9);
    some(&mut simulation, "hides", 4);
    let asked = simulation.raising_a_roof(&simulation.population.agents[0], (25, 25, 0));
    assert!(matches!(asked, Some(Action::Craft { ref item_type }) if item_type == "lashing"), "got {asked:?}");
}

/// Somebody with food to keep and no salt goes to the salt flat a forage
/// off, rather than waiting on an autumn, a cloud, the sea and a pot.
#[test]
fn salt_is_fetched_from_where_it_lies() {
    let mut simulation = one_person();
    let mut roots = InventoryItem::new_with_weight("roots".to_string(), 30, 0.1);
    roots.food_data = FoodDatabase::new().create_food_data(&ItemType::Roots, 0);
    simulation.population.agents[0].inventory.add_item(roots);
    if let Some(hunger) = simulation.population.agents[0].drives.get_mut(crate::core::DriveType::Hunger) {
        hunger.value = 0.0;
    }
    assert!(
        simulation.population.agents[0].what_food_i_can_spare().is_some(),
        "the test wants food to spare"
    );
    let before = simulation.putting_food_by(&simulation.population.agents[0], (25, 25, 0));
    assert!(!matches!(before, Some(Action::Gather { ref resource_type }) if resource_type == "salt"), "no salt anywhere: {before:?}");

    a_node_at(&mut simulation, ResourceType::Salt, Position::new(29, 25));
    let asked = simulation.putting_food_by(&simulation.population.agents[0], (25, 25, 0));
    assert!(matches!(asked, Some(Action::Gather { ref resource_type }) if resource_type == "salt"), "got {asked:?}");
}

/// Somebody with no vessel, setting off on a long walk from beside the
/// water with a blade and the wood for a bowl, carves one first.
#[test]
fn a_vessel_is_made_before_a_long_walk() {
    let mut simulation = one_person();
    a_node_at(&mut simulation, ResourceType::Water, Position::new(26, 25));
    let far = Action::Move { target: (80, 25, 0) };
    some(&mut simulation, "wood", 2);

    let asked = simulation.water_for_the_walk(0, far.clone(), false);
    assert!(matches!(asked, Action::Move { .. }), "no blade, no bowl: {asked:?}");

    let blade = crate::environment::making::what_helps_with(crate::agents::SkillType::Crafting)
        .next()
        .expect("something carves");
    some(&mut simulation, blade.called, 1);
    let asked = simulation.water_for_the_walk(0, far, false);
    assert!(matches!(asked, Action::Work { ref verb, ref to } if verb == "carve" && to == "wood"), "got {asked:?}");
}
