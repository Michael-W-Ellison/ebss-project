// src/analytics/tests/dead_subsystems_tests.rs
//! Subsystems that were built and wired to nothing, now fed: hearing (#14),
//! zoning and territory (#15).

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::{DriveContext, DriveType, Surroundings};
use crate::world::buildings::Building;
use crate::world::{BuildingType, Belongs, Position, TerrainType, World, WorldConfig};

/// Three grown people on open plains: one at (25, 25), one a few paces off,
/// and one far out of earshot.
fn three_people() -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(120, 50));
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
    let far = 25 + 90;
    let mut population = Population::new();
    for _ in 0..3 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for (agent, at) in simulation
        .population
        .agents
        .iter_mut()
        .zip([(25, 25, 0), (30, 25, 0), (far, 25, 0)])
    {
        agent.state.now_this_many_years_old(25);
        agent.state.position = at;
    }
    simulation.current_turn = 1_000;
    simulation
}

/// Somebody struck cries out, and whoever is in earshot hears it; nobody
/// further off does, and nor does the one crying.
#[test]
fn a_cry_carries_to_whoever_is_in_earshot() {
    let mut simulation = three_people();
    let now = simulation.current_turn;
    simulation.population.agents[0].emotions.last_attack_turn = now;

    simulation.let_the_cries_carry();

    let heard = |n: usize| !simulation.population.agents[n].senses.hearing.heard_sounds.is_empty();
    assert!(!heard(0), "not your own cry");
    assert!(heard(1), "a few paces off");
    assert!(!heard(2), "out of earshot");

    simulation.read_the_situation();
    assert!(simulation.population.agents[1].surroundings.heard_a_cry);
    assert!(!simulation.population.agents[2].surroundings.heard_a_cry);
}

/// And nobody cries out who was not struck this turn.
#[test]
fn an_old_blow_is_not_cried_over() {
    let mut simulation = three_people();
    let now = simulation.current_turn;
    simulation.population.agents[0].emotions.last_attack_turn =
        now - crate::environment::seasons::TICKS_BETWEEN_PLANS;
    simulation.let_the_cries_carry();
    assert!(simulation.population.agents[1].senses.hearing.heard_sounds.is_empty());
}

/// A cry heard is something about, and frightening as the dark is.
#[test]
fn a_cry_heard_is_frightening() {
    let quiet = DriveContext::default();
    let cried = DriveContext {
        around: Surroundings { heard_a_cry: true, ..Surroundings::default() },
        ..DriveContext::default()
    };
    let safety = |ctx: &DriveContext| DriveType::Safety.demand(ctx).unwrap_or(0.0);
    assert!(safety(&cried) > safety(&quiet));
}

/// The first house lays out ground for more houses round it; a lean-to,
/// which is a night's cover, lays out nothing.
#[test]
fn the_first_house_lays_out_ground_for_more() {
    let mut world = World::new(WorldConfig::default());
    world.buildings.clear();
    world.add_building(Building::new(BuildingType::LeanTo, Position::new(60, 60)));
    assert!(world.zone_manager.get_zones().is_empty(), "brush lays out nothing");

    world.add_building(Building::new(BuildingType::SmallHouse, Position::new(20, 20)));
    assert!(world
        .zone_manager
        .get_zones_at_position((22, 20, 0))
        .contains(&crate::world::zoning::ZoneType::Residential));
    assert!(
        world.zone_manager.get_zone_bonus((22, 20, 0), BuildingType::SmallHouse)
            > world.zone_manager.get_zone_bonus((22, 20, 0), BuildingType::Forge),
        "a house is wanted there and a forge is not"
    );

    // A second house inside the ground lays out no second zone.
    world.add_building(Building::new(BuildingType::SmallHouse, Position::new(23, 20)));
    assert_eq!(world.zone_manager.get_zones().len(), 1);
}

/// A man who puts up a house claims the plot round it, and a neighbour
/// cannot claim the same ground.
#[test]
fn a_house_claims_its_plot() {
    let mut world = World::new(WorldConfig::default());
    world.buildings.clear();
    let mine = uuid::Uuid::from_u128(7);
    let theirs = uuid::Uuid::from_u128(8);

    let mut house = Building::new(BuildingType::SmallHouse, Position::new(30, 30));
    house.now_belongs_to(Belongs::To(mine));
    world.add_building(house);
    assert_eq!(world.territory_manager.get_owner_at((31, 30, 0)), Some(mine));

    let mut next_door = Building::new(BuildingType::SmallHouse, Position::new(32, 30));
    next_door.now_belongs_to(Belongs::To(theirs));
    world.add_building(next_door);
    assert_eq!(world.territory_manager.get_owner_at((31, 30, 0)), Some(mine), "still mine");
    assert!(world.territory_manager.get_territory_bonus((31, 30, 0), theirs) < 0.0);

    // A storehouse is everybody's and claims nothing.
    let mut store = Building::new(BuildingType::Storehouse, Position::new(50, 50));
    store.now_belongs_to(Belongs::ToUsAll);
    world.add_building(store);
    assert_eq!(world.territory_manager.get_owner_at((50, 50, 0)), None);
}
