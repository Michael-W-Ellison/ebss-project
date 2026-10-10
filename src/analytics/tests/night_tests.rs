// src/analytics/tests/night_tests.rs
//! Tests for the night: people sleep in it, come home before it, and cook
//! supper on an evening fire (#299).

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, World, WorldConfig};

/// A world with nobody in it but `how_many` grown people, all where they
/// were put down.
fn people(how_many: usize) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for agent in simulation.population.agents.iter_mut() {
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.0;
        }
    }
    simulation
}

/// First light today, in hours.
fn first_light(simulation: &Simulation) -> f32 {
    12.0 - simulation.world.climate.calendar.current_season().day_length() / 2.0
}

/// Stand somebody among trees, so that bed is where they are.
fn under_the_trees(simulation: &mut Simulation, index: usize) {
    let at = simulation.population.agents[index].state.position;
    if let Some(tile) = simulation
        .world
        .grid
        .get_tile_mut(&crate::world::Position::new(at.0, at.1))
    {
        tile.terrain.terrain_type = crate::world::TerrainType::Forest;
    }
}

fn set_the_clock(simulation: &mut Simulation, hour: f32) {
    simulation.world.climate.calendar.time_of_day = hour.rem_euclid(24.0);
}

/// Asleep in the hour before first light, awake the hour after, and on the
/// way home in the evening.
#[test]
fn the_night_runs_from_bedtime_to_first_light() {
    let mut simulation = people(1);
    let dawn = first_light(&simulation);

    set_the_clock(&mut simulation, dawn - 1.0);
    let (asleep, _, turns_left) = simulation.where_in_the_night_it_is();
    assert!(asleep, "an hour before first light should be night");
    assert_eq!(turns_left, 2, "an hour before first light is two turns of sleep left");

    set_the_clock(&mut simulation, dawn + 1.0);
    let (asleep, heading_home, _) = simulation.where_in_the_night_it_is();
    assert!(!asleep && !heading_home, "an hour after first light is the working day");

    // A night's sleep is between 7 and 9 hours, so 10 hours before first
    // light is always the evening, and inside the three hours of it spent
    // getting home.
    set_the_clock(&mut simulation, dawn - 10.0);
    let (asleep, heading_home, _) = simulation.where_in_the_night_it_is();
    assert!(!asleep && heading_home, "the evening is for getting home");
}

/// Somebody at home at night is put to bed until first light, a stretch at a
/// time.
#[test]
fn a_person_at_night_sleeps_until_first_light() {
    let mut simulation = people(1);
    under_the_trees(&mut simulation, 0);
    let dawn = first_light(&simulation);
    set_the_clock(&mut simulation, dawn - 3.0);

    let action = simulation.what_the_night_asks(
        0,
        Action::Gather { resource_type: "wood".to_string() },
        false,
    );
    assert!(
        matches!(action, Action::Sleep { duration: 4 }),
        "three hours before first light should be a two-hour stretch asleep, got {action:?}"
    );

    // And the last stretch ends at first light.
    set_the_clock(&mut simulation, dawn - 1.0);
    let action = simulation.what_the_night_asks(
        0,
        Action::Gather { resource_type: "wood".to_string() },
        false,
    );
    assert!(matches!(action, Action::Sleep { duration: 2 }), "got {action:?}");

    // Running for one's life is not put off for bed.
    let action = simulation.what_the_night_asks(
        0,
        Action::Gather { resource_type: "wood".to_string() },
        true,
    );
    assert!(matches!(action, Action::Gather { .. }), "a frightened person is let be, got {action:?}");
}

/// Somebody far from the others in the evening walks back to them.
#[test]
fn somebody_far_off_heads_home_in_the_evening() {
    let mut simulation = people(4);
    for (index, at) in [(10, 10), (10, 10), (10, 10), (90, 10)].into_iter().enumerate() {
        simulation.population.agents[index].state.position.0 = at.0;
        simulation.population.agents[index].state.position.1 = at.1;
        simulation.population.agents[index].hearth = Some((10, 10));
    }
    simulation.world.buildings.clear();
    let home = simulation.where_this_one_sleeps(3).expect("somebody who lives somewhere has a home");
    assert_eq!(home, (30, 10), "home is the middle of the camp's people, the one far off among them");
    let dawn = first_light(&simulation);
    set_the_clock(&mut simulation, dawn - 10.0);

    let action = simulation.what_the_night_asks(
        3,
        Action::Gather { resource_type: "wood".to_string() },
        false,
    );
    match action {
        Action::Move { target } => assert_eq!((target.0, target.1), home, "the walk is to where the others sleep"),
        other => panic!("somebody 60 cells out in the evening should head home, got {other:?}"),
    }
}

/// With tinder under it a fire takes half the wood, and the evening at home
/// knows it: eight sticks and tinder light a fire rather than send the person
/// off for more than they can carry.
#[test]
fn the_evening_fire_takes_half_the_wood_with_tinder() {
    let mut simulation = people(1);
    let half = (Simulation::FIRE_BUILD_WOOD + Simulation::FIRE_FUEL_WOOD).div_ceil(2);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), half, 2.0));
        let mut fish = InventoryItem::new_with_weight("fishportions".to_string(), 5, 0.5);
        fish.food_data = FoodDatabase::new().create_food_data(&ItemType::Fish, 0);
        agent.inventory.add_item(fish);
    }
    assert!(Simulation::has_food_worth_cooking(&simulation.population.agents[0]));

    let without = simulation.the_evening_at_home(0);
    assert!(
        !matches!(without, Some(Action::LightFire)),
        "{half} sticks and no tinder are not enough for a new fire"
    );

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("tinder".to_string(), 1));
    assert_eq!(simulation.wood_a_fire_here_takes(0), half);
    assert!(
        matches!(simulation.the_evening_at_home(0), Some(Action::LightFire)),
        "{half} sticks and tinder light the evening fire"
    );
}

/// A night's sleep pays back its whole length in debt, not half an hour of it.
#[test]
fn a_nights_sleep_pays_back_the_night() {
    let mut simulation = people(1);
    simulation.population.agents[0].fatigue.sleep_debt = 8.0;
    let _ = simulation.sleeping(&16, 0);
    let debt = simulation.population.agents[0].fatigue.sleep_debt;
    assert!(
        debt < 6.0,
        "eight hours asleep should repay more than two hours of eight owed, left {debt}"
    );
}

/// Sent to bed standing on a midden, somebody steps off it first, as the Rest
/// drive's own way to bed always did.
#[test]
fn nobody_sleeps_on_foul_ground() {
    let mut simulation = people(1);
    let at = simulation.population.agents[0].state.position;
    let here = crate::world::Position::new(at.0, at.1);
    simulation
        .world
        .grid
        .somebody_voided_on(&here, crate::world::Soil::AS_FOUL_AS_IT_GETS);
    let dawn = first_light(&simulation);
    set_the_clock(&mut simulation, dawn - 3.0);

    let action = simulation.what_the_night_asks(
        0,
        Action::Gather { resource_type: "wood".to_string() },
        false,
    );
    match action {
        Action::Move { target } => assert!(
            (target.0 - at.0).abs() + (target.1 - at.1).abs() > 0,
            "the step is off the midden"
        ),
        other => panic!("somebody on foul ground at bedtime should step off it, got {other:?}"),
    }
}

/// Sent to bed out in the open with trees a short walk off, somebody goes in
/// among them first; standing among them already, they sleep where they are.
#[test]
fn a_sleeper_goes_in_out_of_the_weather() {
    let mut simulation = people(1);
    let at = simulation.population.agents[0].state.position;
    let wood = crate::world::Position::new(at.0 + 3, at.1);
    for x in [at.0, wood.x] {
        let tile = simulation
            .world
            .grid
            .get_tile_mut(&crate::world::Position::new(x, at.1))
            .expect("on the map");
        tile.terrain.terrain_type = if x == at.0 {
            crate::world::TerrainType::Plains
        } else {
            crate::world::TerrainType::Forest
        };
    }
    // Nothing nearer than the wood: clear any trees closer in.
    for dy in -3..=3 {
        for dx in -3..=3 {
            let p = crate::world::Position::new(at.0 + dx, at.1 + dy);
            if p == wood {
                continue;
            }
            if let Some(tile) = simulation.world.grid.get_tile_mut(&p) {
                if matches!(tile.terrain.terrain_type, crate::world::TerrainType::Forest) {
                    tile.terrain.terrain_type = crate::world::TerrainType::Plains;
                }
            }
        }
    }
    let dawn = first_light(&simulation);
    set_the_clock(&mut simulation, dawn - 3.0);

    let action = simulation.what_the_night_asks(
        0,
        Action::Gather { resource_type: "wood".to_string() },
        false,
    );
    match action {
        Action::Move { target } => assert_eq!((target.0, target.1), (wood.x, wood.y)),
        other => panic!("somebody in the open at bedtime should go in among the trees, got {other:?}"),
    }

    simulation.population.agents[0].state.position.0 = wood.x;
    let action = simulation.what_the_night_asks(
        0,
        Action::Gather { resource_type: "wood".to_string() },
        false,
    );
    assert!(matches!(action, Action::Sleep { .. }), "under the trees, they sleep: got {action:?}");
}

/// Somebody put to bed is asleep for the night, and mends as a sleeper does.
///
/// The night's sleep was reckoned all at once and the sleeper woken on the
/// spot, so through every turn of the stretch that followed the body was
/// awake to everything that asked: on a big-map seed, not one grown person's
/// turn in 71,852 had anybody asleep in it, and the wounded mended at a
/// waking man's rate, a fifth of a sleeper's. See ISSUES_FOUND #310.
#[test]
fn a_sleeper_sleeps_through_the_stretch_and_mends() {
    let mut simulation = people(1);
    under_the_trees(&mut simulation, 0);
    simulation.population.agents[0].state.health = 50.0;

    simulation.execute_action(&Action::Sleep { duration: 4 }, 0);
    assert!(
        simulation.population.agents[0].fatigue.is_sleeping,
        "woken the moment the sleep was reckoned"
    );

    let now = simulation.current_turn;
    let before = simulation.population.agents[0].state.health;
    for step in 1..=4 {
        simulation.population.agents[0].process_survival_turn(now + step * 30);
    }
    let asleep = simulation.population.agents[0].state.health - before;

    let mut awake = people(1);
    awake.population.agents[0].state.health = 50.0;
    let before = awake.population.agents[0].state.health;
    for step in 1..=4 {
        awake.population.agents[0].process_survival_turn(now + step * 30);
    }
    let up_and_about = awake.population.agents[0].state.health - before;

    assert!(
        asleep > up_and_about * 2.0,
        "two hours asleep mended {asleep:.2}, against {up_and_about:.2} up and about"
    );
}

/// Two camps out of each other's reach are two homes: somebody standing by
/// the other camp in the evening walks back to their own.
///
/// Home was the middle of every grown person in the world, so there could be
/// only one camp, and anybody who went off was walked back to the others that
/// evening however far it was (#314).
#[test]
fn two_camps_are_two_homes() {
    let mut simulation = people(6);
    simulation.world.buildings.clear();
    for (index, lives) in [(5, 5), (5, 5), (5, 5), (48, 48), (48, 48), (48, 48)].into_iter().enumerate() {
        let agent = &mut simulation.population.agents[index];
        agent.hearth = Some(lives);
        agent.state.position = (lives.0, lives.1, 0);
    }
    // One of the first camp is over at the second at nightfall
    simulation.population.agents[0].state.position = (46, 46, 0);

    assert_eq!(simulation.where_this_one_sleeps(0), Some((18, 18)), "the middle of the first camp's people");
    assert_eq!(simulation.where_this_one_sleeps(3), Some((48, 48)));

    let dawn = first_light(&simulation);
    set_the_clock(&mut simulation, dawn - 10.0);
    match simulation.what_the_night_asks(0, Action::Gather { resource_type: "wood".to_string() }, false) {
        Action::Move { target } => assert_eq!((target.0, target.1), (18, 18), "home is their own camp"),
        other => panic!("somebody at the wrong camp at nightfall should head home, got {other:?}"),
    }
}

/// And two camps that drift within reach of each other become one.
#[test]
fn camps_within_reach_of_each_other_are_one() {
    let mut simulation = people(4);
    simulation.world.buildings.clear();
    for (index, lives) in [(10, 10), (10, 10), (30, 10), (30, 10)].into_iter().enumerate() {
        simulation.population.agents[index].hearth = Some(lives);
        simulation.population.agents[index].state.position = (lives.0, lives.1, 0);
    }
    assert_eq!(simulation.where_this_one_sleeps(0), simulation.where_this_one_sleeps(3));
    assert_eq!(simulation.where_this_one_sleeps(0), Some((20, 10)));
}

/// Lying down at home makes the camp one's hearth.
#[test]
fn whoever_sleeps_at_home_lives_there() {
    let mut simulation = people(3);
    simulation.world.buildings.clear();
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (12, 12, 0);
        agent.hearth = None;
    }
    under_the_trees(&mut simulation, 0);
    let dawn = first_light(&simulation);
    set_the_clock(&mut simulation, dawn - 3.0);
    let _ = simulation.what_the_night_asks(0, Action::Gather { resource_type: "wood".to_string() }, false);
    assert_eq!(simulation.population.agents[0].hearth, Some((12, 12)));
}

/// A finished roof near the middle of a camp is where it sleeps.
#[test]
fn a_roof_among_them_is_home() {
    let mut simulation = people(3);
    for agent in simulation.population.agents.iter_mut() {
        agent.hearth = Some((20, 20));
    }
    let longhouse = simulation.world.buildings.iter().find(|b| b.is_completed()).map(|b| (b.position.x, b.position.y));
    assert_eq!(simulation.where_this_one_sleeps(0), longhouse, "the longhouse in the middle of the map is within reach");
}

/// Somebody on their own, with nothing keeping them there, sleeps at the
/// nearest camp; one with a fresh move of their own to wait out does not.
#[test]
fn nobody_lives_alone_by_accident() {
    let mut simulation = people(4);
    simulation.world.buildings.clear();
    for (index, lives) in [(5, 5), (5, 5), (5, 5), (48, 48)].into_iter().enumerate() {
        simulation.population.agents[index].hearth = Some(lives);
        simulation.population.agents[index].state.position = (lives.0, lives.1, 0);
    }
    assert_eq!(simulation.where_this_one_sleeps(3), Some((5, 5)), "the straggler goes to the camp");

    simulation.current_turn = 100_000;
    simulation.population.agents[3].hearth_moved_at = simulation.current_turn - 30;
    assert_eq!(simulation.where_this_one_sleeps(3), Some((48, 48)), "a pioneer waits out their move");
}

/// A camp drifts towards where its people are: tonight it sleeps in the
/// middle of where they actually are, not where they slept last night.
///
/// Taken from the hearths alone, home was a fixed point, and a camp that
/// started by the longhouse slept there for ever, walking back every evening
/// to ground it had stripped (#314).
#[test]
fn a_camp_sleeps_where_its_people_are() {
    let mut simulation = people(4);
    simulation.world.buildings.clear();
    for agent in simulation.population.agents.iter_mut() {
        agent.hearth = Some((20, 20));
        agent.state.position = (30, 20, 0);
    }
    assert_eq!(simulation.where_this_one_sleeps(0), Some((30, 20)));
}
