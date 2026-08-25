// src/analytics/tests/sprouting_tests.rs
//! Tests for the two accidents that teach a people what seed is for.
//!
//! "Something like grain getting wet should result in the grains sprouting. If
//! sprouted grains are thrown out or dropped, they could grow into adult
//! plants."
//!
//! Neither of these is a decision anybody makes. Grain carried through a wet
//! season stops being grain: it is wet seed and it starts. A pack with wet seed
//! in it loses some, and what falls out on ground that can carry it comes up as
//! a plant where somebody was standing. The whole of farming is downstream of
//! somebody noticing that.

use crate::agents::practices::Practice;
use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{
    Position, ResourceType, Terrain, TerrainType, World, WorldConfig,
};

fn a_person_standing_on(ground: TerrainType, where_it_is: Position) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world
        .resources
        .retain(|resource| resource.position != where_it_is);

    if let Some(tile) = world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(ground);
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (where_it_is.x, where_it_is.y, 0);
    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            0.5,
        ));
}

// --------------------------------------------------------------------------
// Grain in the wet
// --------------------------------------------------------------------------

/// A pack carried on wet ground has grain coming up in it before long.
#[test]
fn grain_carried_in_the_wet_starts_growing() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Wetland, where_it_is);
    give(&mut simulation, "grain", 40);

    for _ in 0..2000 {
        simulation.what_got_wet_sprouts();
    }

    let agent = &simulation.population.agents[0];
    assert!(
        agent.how_many_i_have("sproutedgrain") > 0,
        "grain carried across a marsh should start: {} left as grain",
        agent.how_many_i_have("grain")
    );
    assert!(
        agent.how_many_i_have("grain") < 40,
        "and there should be less grain than there was"
    );
}

/// Dry ground under a clear sky does nothing to it.
#[test]
fn grain_kept_dry_stays_grain() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Desert, where_it_is);
    simulation.world.climate.weather.weather_type =
        crate::environment::weather::WeatherType::Clear;
    give(&mut simulation, "grain", 40);

    for _ in 0..2000 {
        simulation.what_got_wet_sprouts();
    }

    let agent = &simulation.population.agents[0];
    assert_eq!(
        agent.how_many_i_have("grain"),
        40,
        "grain kept dry is still grain"
    );
    assert_eq!(agent.how_many_i_have("sproutedgrain"), 0);
}

/// And rain does it on ground that would not have on its own.
#[test]
fn rain_starts_grain_that_the_ground_would_not() {
    fn sprouted(weather: crate::environment::weather::WeatherType) -> u32 {
        let where_it_is = Position::new(25, 25);
        let mut simulation = a_person_standing_on(TerrainType::Plains, where_it_is);
        simulation.world.climate.weather.weather_type = weather;
        give(&mut simulation, "grain", 40);

        for _ in 0..2000 {
            simulation.what_got_wet_sprouts();
        }

        simulation.population.agents[0].how_many_i_have("sproutedgrain")
    }

    use crate::environment::weather::WeatherType;

    assert_eq!(
        sprouted(WeatherType::Clear),
        0,
        "open plains in the dry are dry"
    );
    assert!(
        sprouted(WeatherType::HeavyRain) > 0,
        "the same ground in a downpour is not"
    );
}

// --------------------------------------------------------------------------
// What falls out of a pack
// --------------------------------------------------------------------------

/// A sprouted grain dropped on ground that can carry it becomes a plant.
#[test]
fn a_dropped_sprout_becomes_a_plant() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Plains, where_it_is);
    give(&mut simulation, "sproutedgrain", 60);

    for _ in 0..1000 {
        simulation.what_was_dropped_takes_root();

        if simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == where_it_is)
        {
            break;
        }
    }

    let grown = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == where_it_is);

    assert!(
        grown.is_some(),
        "seed that falls out of a pack onto open grass grows"
    );
    assert_eq!(
        grown.map(|resource| resource.resource_type),
        Some(ResourceType::Grain),
        "and what grows is what was dropped"
    );
    assert!(
        simulation.population.agents[0].how_many_i_have("sproutedgrain") < 60,
        "the pack should be lighter for it"
    );
}

/// Nothing takes root on a mountainside.
#[test]
fn nothing_takes_root_on_bare_rock() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Mountain, where_it_is);
    give(&mut simulation, "sproutedgrain", 60);

    for _ in 0..1000 {
        simulation.what_was_dropped_takes_root();
    }

    assert!(
        !simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == where_it_is),
        "a mountain does not carry a crop, however much seed is spilt on it"
    );
}

/// And whoever is standing there when it happens learns what seed does.
#[test]
fn seeing_a_dropped_seed_come_up_teaches_farming() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Plains, where_it_is);

    // A second person, well out of sight of it
    simulation
        .population
        .spawn_agent(AgentConfig::default());
    simulation.population.agents[1].state.position = (where_it_is.x + 40, where_it_is.y, 0);

    give(&mut simulation, "sproutedgrain", 60);

    for _ in 0..1000 {
        simulation.what_was_dropped_takes_root();

        if simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == where_it_is)
        {
            break;
        }
    }

    assert!(
        simulation.population.agents[0]
            .practices
            .confidence(Practice::Farming)
            > 0.0,
        "the man whose pack it fell out of is standing over the evidence"
    );
    assert_eq!(
        simulation.population.agents[1]
            .practices
            .confidence(Practice::Farming),
        0.0,
        "the man forty tiles away is not"
    );
}

// --------------------------------------------------------------------------
// What a person does with seed that is already growing
// --------------------------------------------------------------------------

/// Sprouted grain is what an agent sows if it has any, over anything else.
#[test]
fn a_sprouted_seed_is_the_thing_worth_sowing() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Plains, where_it_is);

    give(&mut simulation, "food", 20);
    give(&mut simulation, "sproutedgrain", 20);

    let result = simulation.execute_action(&Action::TillSoil, 0);
    assert!(result.success, "open grass breaks: {:?}", result.message);

    assert_eq!(
        simulation
            .world
            .resources
            .iter()
            .find(|resource| resource.position == where_it_is)
            .map(|resource| resource.resource_type),
        Some(ResourceType::Grain),
        "a man holding seed that is visibly growing puts that in the ground"
    );
}

/// And sowing costs the seed. A field is a meal not eaten.
#[test]
fn sowing_a_field_costs_the_seed_that_went_into_it() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_person_standing_on(TerrainType::Plains, where_it_is);
    give(&mut simulation, "grain", 10);

    let before = simulation.population.agents[0].how_many_i_have("grain");
    simulation.execute_action(&Action::TillSoil, 0);
    let after = simulation.population.agents[0].how_many_i_have("grain");

    assert!(
        after < before,
        "the seed went in the ground: {before} before, {after} after"
    );
}
