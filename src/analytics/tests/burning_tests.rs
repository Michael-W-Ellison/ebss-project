// src/analytics/tests/burning_tests.rs
//! Controlled burning (ISSUES_FOUND #381).
//!
//! - ground a fire went over carries up to twice, and the flush is gone in a
//!   year
//! - a fire burns off the stands it brings on within its reach, and passes
//!   under the nut trees
//! - the country burns only on a dry day in summer or autumn
//! - somebody who knows how walks to a picked-over patch near home and sets
//!   it alight, with a fire at the camp to take a brand from
//! - not a patch hung with fruit, not near the camp, not without a fire at
//!   the camp, not on a wet day or in winter
//! - gathering often enough on burnt ground is how it is found out

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::Season;
use crate::environment::{Action, WeatherType};
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

const HOME: (i32, i32) = (25, 25);

/// One grown person at home on open plains, on a dry summer's day, with a
/// fire burning at the camp and nothing else about.
fn at_home_in_summer() -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(80, 50));
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
    world.climate.calendar.day_of_year = Season::Summer.first_day() + 20;
    world.climate.weather.weather_type = WeatherType::Clear;
    let mut hearth = crate::environment::HeatSource::new(
        crate::environment::HeatSourceType::Campfire,
        (HOME.0, HOME.1, 0),
        0,
    );
    hearth.add_fuel("wood".to_string(), 1.0, 100);
    assert!(hearth.light());
    world.heat_sources.add(hearth);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some(HOME);
    simulation
}

/// Three berry stands about `at`, each carrying `standing`.
fn a_patch_of_berries(simulation: &mut Simulation, at: (i32, i32), standing: u32) {
    for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
        let mut stand = ResourceNode::new(ResourceType::Food, Position::new(at.0 + dx, at.1 + dy), 100);
        stand.amount = standing;
        simulation.world.put_a_node_down(stand);
    }
}

fn knows_how(simulation: &mut Simulation) {
    simulation.population.agents[0].found_out_how_to("do:BurnOff");
}

/// The flush after a fire carries twice the old growth, and is gone in a
/// year.
#[test]
fn burnt_ground_carries_twice_and_the_flush_is_gone_in_a_year() {
    let mut stand = ResourceNode::new(ResourceType::Grain, Position::new(0, 0), 100);
    let old_growth = stand.how_heavy_a_crop_it_carries(1.0);
    stand.goes_up_in_flames();
    assert_eq!(stand.amount, 0, "what stood is burnt off");
    assert_eq!(stand.how_heavy_a_crop_it_carries(1.0), old_growth * 2);
    for _ in 0..365 {
        stand.a_day_since_it_was_seen_to();
    }
    assert_eq!(stand.burnt, 0.0);
    assert_eq!(stand.how_heavy_a_crop_it_carries(1.0), old_growth);
}

/// A fire burns off what it brings on within its reach, and passes under the
/// nut trees and the timber.
#[test]
fn a_fire_burns_what_it_brings_on_within_its_reach() {
    let mut simulation = at_home_in_summer();
    let world = &mut simulation.world;
    let berries = world.put_a_node_down(ResourceNode::new(ResourceType::Food, Position::new(50, 25), 10));
    let nuts = world.put_a_node_down(ResourceNode::new(ResourceType::Nuts, Position::new(51, 25), 10));
    let wood = world.put_a_node_down(ResourceNode::new(ResourceType::Wood, Position::new(50, 26), 10));
    let beyond = world.put_a_node_down(ResourceNode::new(ResourceType::Greens, Position::new(50, 30), 10));

    assert_eq!(world.burn_over(Position::new(50, 25), 3), 1);
    assert_eq!(world.resources[berries].amount, 0);
    assert_eq!(world.resources[berries].burnt, 1.0);
    for untouched in [nuts, wood, beyond] {
        assert_eq!(world.resources[untouched].amount, 10);
        assert_eq!(world.resources[untouched].burnt, 0.0);
    }
    assert_eq!(world.stands_burnt, 1);
}

/// The country burns on a dry day in the summer or the autumn, not in the
/// rain and not in winter or spring.
#[test]
fn the_country_burns_only_on_a_dry_day_in_summer_or_autumn() {
    assert!(World::is_it_a_day_for_burning(Season::Summer, 0.0));
    assert!(World::is_it_a_day_for_burning(Season::Fall, 0.0));
    assert!(!World::is_it_a_day_for_burning(Season::Summer, 0.5));
    assert!(!World::is_it_a_day_for_burning(Season::Winter, 0.0));
    assert!(!World::is_it_a_day_for_burning(Season::Spring, 0.0));
}

/// Somebody who knows how walks to a picked-over patch near home, sets it
/// alight standing in it, and every stand there is burnt off for the flush.
#[test]
fn a_picked_over_patch_near_home_is_walked_to_and_set_alight() {
    let mut simulation = at_home_in_summer();
    a_patch_of_berries(&mut simulation, (40, 25), 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_patch_to_burn(&agent, agent.state.position), None, "not by somebody who never saw it done");

    knows_how(&mut simulation);
    let agent = simulation.population.agents[0].clone();
    let Some(Action::Move { target }) = simulation.a_patch_to_burn(&agent, agent.state.position) else {
        panic!("walks to the patch");
    };
    assert!(Simulation::within((target.0, target.1), (40, 25), 1), "{target:?}");

    simulation.population.agents[0].state.position = (40, 25, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_patch_to_burn(&agent, agent.state.position), Some(Action::BurnOff));
    let result = simulation.execute_action(&Action::BurnOff, 0);
    assert!(result.success, "{result:?}");
    assert!(simulation.world.resources.iter().all(|stand| stand.burnt == 1.0));
    assert_eq!(simulation.world.burns_set, 1);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_patch_to_burn(&agent, agent.state.position), None, "and not burnt again straight off");
}

/// A patch hung with fruit is worth more standing than the flush would add.
#[test]
fn a_patch_hung_with_fruit_is_not_burnt() {
    let mut simulation = at_home_in_summer();
    a_patch_of_berries(&mut simulation, (40, 25), 100);
    knows_how(&mut simulation);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_patch_to_burn(&agent, agent.state.position), None);
}

/// Not where the fire would reach the camp's own ground, whether chosen or
/// stood in.
#[test]
fn nobody_sets_a_fire_near_the_camp() {
    let mut simulation = at_home_in_summer();
    a_patch_of_berries(&mut simulation, (HOME.0 + 5, HOME.1), 0);
    knows_how(&mut simulation);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_patch_to_burn(&agent, agent.state.position), None);

    simulation.population.agents[0].state.position = (HOME.0 + 5, HOME.1, 0);
    let result = simulation.execute_action(&Action::BurnOff, 0);
    assert!(!result.success);
    assert!(simulation.world.resources.iter().all(|stand| stand.burnt == 0.0));
}

/// No fire at the camp to take a brand from, a wet day, or the winter: no
/// burning.
#[test]
fn no_burning_without_a_fire_at_camp_or_in_the_wet_or_the_winter() {
    let mut cold_camp = at_home_in_summer();
    cold_camp.world.heat_sources = crate::environment::HeatSourceRegistry::new();
    a_patch_of_berries(&mut cold_camp, (40, 25), 0);
    knows_how(&mut cold_camp);
    let agent = cold_camp.population.agents[0].clone();
    assert_eq!(cold_camp.a_patch_to_burn(&agent, agent.state.position), None, "no fire at the camp");

    let mut wet = at_home_in_summer();
    wet.world.climate.weather.weather_type = WeatherType::Rain;
    a_patch_of_berries(&mut wet, (40, 25), 0);
    knows_how(&mut wet);
    let agent = wet.population.agents[0].clone();
    assert_eq!(wet.a_patch_to_burn(&agent, agent.state.position), None, "in the rain");

    let mut winter = at_home_in_summer();
    winter.world.climate.calendar.day_of_year = Season::Winter.first_day() + 20;
    a_patch_of_berries(&mut winter, (40, 25), 0);
    knows_how(&mut winter);
    let agent = winter.population.agents[0].clone();
    assert_eq!(winter.a_patch_to_burn(&agent, agent.state.position), None, "in the winter");
}

/// Gathering often enough in the flush on ground a fire went over is how
/// somebody thinks to set one.
#[test]
fn gathering_on_burnt_ground_is_how_it_is_found_out() {
    let mut simulation = at_home_in_summer();
    let mut stand = ResourceNode::new(ResourceType::Food, Position::new(HOME.0, HOME.1), 10_000);
    stand.burnt = 0.9;
    simulation.world.put_a_node_down(stand);
    simulation.population.agents[0].inventory.max_weight = 500.0;
    for _ in 0..Simulation::BURNT_GROUND_GATHERED_BEFORE_BURNING {
        assert!(!simulation.population.agents[0].knows_how_to_do("BurnOff"));
        simulation.population.agents[0].inventory.get_all_items_mut().clear();
        simulation.population.agents[0].busy_until = 0;
        let result = simulation.execute_action(&Action::Gather { resource_type: "food".to_string() }, 0);
        assert!(result.success, "{result:?}");
    }
    assert!(simulation.population.agents[0].knows_how_to_do("BurnOff"));
}
