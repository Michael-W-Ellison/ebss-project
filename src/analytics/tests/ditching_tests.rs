// src/analytics/tests/ditching_tests.rs
//! Water diversion on a very small scale (ISSUES_FOUND #383).
//!
//! - a ditch brings dry ground up to the middle of what a plant does best on,
//!   does nothing for ground wet enough already, and silts up in half a year
//! - somebody who knows how walks to a patch near home on dry ground with
//!   water close by, in the growing season, and leads the water to it
//! - not on ground wet enough already, not with no water near, not in the
//!   winter, not by somebody who never saw it done
//! - gathering often enough from stands on dry ground by water is how it is
//!   found out

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::Season;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

const HOME: (i32, i32) = (25, 25);
const PATCH: (i32, i32) = (40, 25);

/// One grown person at home on open plains in the spring, with a stretch of
/// hills about `PATCH` and nothing else about.
fn at_home_in_spring() -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(80, 50));
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    for x in 0..world.grid.width as i32 {
        for y in 0..world.grid.height as i32 {
            let hills = (x - PATCH.0).abs() <= 5 && (y - PATCH.1).abs() <= 5;
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = if hills { TerrainType::Hills } else { TerrainType::Plains };
            }
        }
    }
    world.climate.calendar.day_of_year = Season::Spring.first_day() + 20;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some(HOME);
    simulation
}

/// Three berry stands about `at`.
fn a_patch_of_berries(simulation: &mut Simulation, at: (i32, i32)) {
    for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
        let mut stand = ResourceNode::new(ResourceType::Food, Position::new(at.0 + dx, at.1 + dy), 100);
        stand.amount = 5;
        simulation.world.put_a_node_down(stand);
    }
}

fn a_spring_at(simulation: &mut Simulation, at: (i32, i32)) {
    simulation
        .world
        .put_a_node_down(ResourceNode::new(ResourceType::Water, Position::new(at.0, at.1), 100));
}

fn knows_how(simulation: &mut Simulation) {
    simulation.population.agents[0].found_out_how_to("do:DigDitch");
}

/// A ditch brings dry ground up to the middle of what a plant does best on,
/// leaves wet ground as it is, and silts up in half a year.
#[test]
fn a_ditch_waters_dry_ground_and_silts_up_in_half_a_year() {
    let mut stand = ResourceNode::new(ResourceType::Grain, Position::new(0, 0), 100);
    assert_eq!(stand.what_it_has_to_drink(0.25), 0.25, "no ditch, no water");
    stand.watered = 1.0;
    assert_eq!(stand.what_it_has_to_drink(0.25), ResourceNode::WHAT_A_DITCH_BRINGS);
    assert_eq!(stand.what_it_has_to_drink(0.85), 0.85, "a riverbank is wet enough already");
    stand.watered = 0.5;
    let half = stand.what_it_has_to_drink(0.2);
    assert!(half > 0.2 && half < ResourceNode::WHAT_A_DITCH_BRINGS, "{half}");
    stand.watered = 1.0;
    for _ in 0..180 {
        stand.a_day_since_it_was_seen_to();
    }
    assert!(stand.watered < 0.01, "silted up: {}", stand.watered);
}

/// Somebody who knows how walks to a patch near home on dry ground with
/// water close by and leads the water to it; every stand there is watered.
#[test]
fn a_dry_patch_near_water_is_walked_to_and_watered() {
    let mut simulation = at_home_in_spring();
    a_patch_of_berries(&mut simulation, PATCH);
    a_spring_at(&mut simulation, (PATCH.0 + 3, PATCH.1));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_ditch_to_dig(&agent, agent.state.position), None, "not by somebody who never saw it done");

    knows_how(&mut simulation);
    let agent = simulation.population.agents[0].clone();
    let Some(Action::Move { target }) = simulation.a_ditch_to_dig(&agent, agent.state.position) else {
        panic!("walks to the patch");
    };
    assert!(Simulation::within((target.0, target.1), PATCH, 1), "{target:?}");

    simulation.population.agents[0].state.position = (PATCH.0, PATCH.1, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_ditch_to_dig(&agent, agent.state.position), Some(Action::DigDitch));
    let result = simulation.execute_action(&Action::DigDitch, 0);
    assert!(result.success, "{result:?}");
    let berries = simulation.world.resources.iter().filter(|node| node.resource_type == ResourceType::Food);
    assert!(berries.clone().count() == 3 && berries.into_iter().all(|stand| stand.watered == 1.0));
    assert_eq!(simulation.world.ditches_dug, 1);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_ditch_to_dig(&agent, agent.state.position), None, "and not dug again straight off");
}

/// Ground wet enough already, no water near, or the winter: no ditch.
#[test]
fn no_ditch_on_wet_ground_away_from_water_or_in_the_winter() {
    let mut wet = at_home_in_spring();
    a_patch_of_berries(&mut wet, (PATCH.0 - 12, PATCH.1));
    a_spring_at(&mut wet, (PATCH.0 - 9, PATCH.1));
    knows_how(&mut wet);
    let agent = wet.population.agents[0].clone();
    assert_eq!(wet.a_ditch_to_dig(&agent, agent.state.position), None, "plains hold water enough");

    let mut dry = at_home_in_spring();
    a_patch_of_berries(&mut dry, PATCH);
    knows_how(&mut dry);
    let agent = dry.population.agents[0].clone();
    assert_eq!(dry.a_ditch_to_dig(&agent, agent.state.position), None, "no water to lead");
    dry.population.agents[0].state.position = (PATCH.0, PATCH.1, 0);
    assert!(!dry.execute_action(&Action::DigDitch, 0).success);

    let mut winter = at_home_in_spring();
    winter.world.climate.calendar.day_of_year = Season::Winter.first_day() + 20;
    a_patch_of_berries(&mut winter, PATCH);
    a_spring_at(&mut winter, (PATCH.0 + 3, PATCH.1));
    knows_how(&mut winter);
    let agent = winter.population.agents[0].clone();
    assert_eq!(winter.a_ditch_to_dig(&agent, agent.state.position), None, "in the winter");
}

/// Gathering often enough from a stand near home on dry ground, with water
/// close by, is how somebody thinks to lead the water to it.
#[test]
fn gathering_dry_by_water_is_how_it_is_found_out() {
    let mut simulation = at_home_in_spring();
    simulation.population.agents[0].hearth = Some(PATCH);
    simulation.population.agents[0].state.position = (PATCH.0, PATCH.1, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;
    simulation
        .world
        .put_a_node_down(ResourceNode::new(ResourceType::Roots, Position::new(PATCH.0, PATCH.1), 10_000));
    a_spring_at(&mut simulation, (PATCH.0 + 2, PATCH.1));
    for _ in 0..Simulation::GATHERED_DRY_BY_WATER_BEFORE_DITCHING {
        assert!(!simulation.population.agents[0].knows_how_to_do("DigDitch"));
        simulation.population.agents[0].inventory.get_all_items_mut().clear();
        simulation.population.agents[0].busy_until = 0;
        let result = simulation.execute_action(&Action::Gather { resource_type: "roots".to_string() }, 0);
        assert!(result.success, "{result:?}");
    }
    assert!(simulation.population.agents[0].knows_how_to_do("DigDitch"));
}
