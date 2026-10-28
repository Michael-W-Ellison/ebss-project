// src/analytics/tests/hunting_for_meat_tests.rs
//! Hunting for meat (#358): a week without meat and the body wants it; a
//! hunter looks as far as they can see, remembers where game was seen and
//! walks there, and notes the game as they go.

use crate::agents::{AgentConfig, InventoryItem, Population, Quality, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, TerrainType, World, WorldConfig};

/// One grown hunter with spears at (100, 100) on open plains three hundred
/// cells across, no beasts about.
fn a_hunter() -> Simulation {
    let mut world = World::new(WorldConfig { size: (300, 300), ..WorldConfig::default() });
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
    agent.state.position = (100, 100, 0);
    agent.hearth = Some((100, 100));
    agent.inventory.max_weight = 500.0;
    agent.skills.set_skill_level(SkillType::Hunting, 8);
    for _ in 0..4 {
        agent
            .inventory
            .add_item(InventoryItem::new_with_durability("spear".to_string(), 1, 25.0, Quality::Common));
    }
    simulation
}

fn a_deer(simulation: &mut Simulation, at: (i32, i32)) -> uuid::Uuid {
    let id = simulation.world.spawn_animal("deer".to_string(), at).expect("a deer");
    simulation.world.animals.get_mut(&id).unwrap().age = 100_000;
    id
}

/// A week without meat, and the body wants it; a meal of it puts that off
/// another week.
#[test]
fn a_week_without_meat_and_it_is_wanted() {
    let mut simulation = a_hunter();
    let a_day = crate::environment::seasons::TICKS_PER_DAY;
    let agent = &mut simulation.population.agents[0];
    assert!(!agent.craves_meat(6 * a_day));
    assert!(agent.craves_meat(7 * a_day));

    let mut meat = InventoryItem::new_with_weight("cooked_meatportions".to_string(), 1, 1.0);
    meat.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Meat, 7 * a_day);
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(meat);
    let ate = agent.eat_food_item("cooked_meatportions", 7 * a_day);
    assert!(agent.last_ate_meat.is_some(), "ate: {ate:?}, salt {}", agent.salt_in_the_body);
    assert!(!agent.craves_meat(13 * a_day));
    assert!(agent.craves_meat(14 * a_day));
}

/// A hunter looks as far as they can see: a deer twenty cells off, past
/// what was once the limit, is gone after for its meat by somebody who
/// craves it, and struck when it is in reach.
#[test]
fn a_deer_in_sight_is_gone_after_for_its_meat() {
    let mut simulation = a_hunter();
    simulation.current_turn = 8 * crate::environment::seasons::TICKS_PER_DAY;
    let agent = simulation.population.agents[0].clone();
    assert!(Simulation::how_far_a_hunter_looks(&agent) >= 20.0);

    a_deer(&mut simulation, (120, 100));
    let going = simulation.going_after_meat(&agent, (100, 100, 0));
    assert!(matches!(going, Some(Action::Move { target }) if (target.0, target.1) == (120, 100)), "got {going:?}");

    let mut simulation = a_hunter();
    simulation.current_turn = 8 * crate::environment::seasons::TICKS_PER_DAY;
    a_deer(&mut simulation, (101, 100));
    let agent = simulation.population.agents[0].clone();
    assert!(matches!(simulation.hunting_action(&agent, (100, 100, 0)), Some(Action::Hunt { .. })));

    // Fed on meat this week, nobody goes for it unless the skins are wanted
    simulation.population.agents[0].last_ate_meat = Some(simulation.current_turn);
    let agent = simulation.population.agents[0].clone();
    if !Simulation::wants_to_hunt(&agent) {
        assert_eq!(simulation.hunting_action(&agent, (100, 100, 0)), None);
    }
}

/// Nothing in sight, a hunter craving meat walks to where game was lately
/// seen - and not to a sighting gone stale.
#[test]
fn a_hunter_walks_to_where_game_was_seen() {
    let mut simulation = a_hunter();
    let now = 8 * crate::environment::seasons::TICKS_PER_DAY;
    simulation.current_turn = now;
    simulation.population.agents[0].saw_game_at((200, 100), now - 100);
    let agent = simulation.population.agents[0].clone();
    let going = simulation.going_after_meat(&agent, (100, 100, 0));
    assert!(matches!(going, Some(Action::Move { target }) if (target.0, target.1) == (200, 100)), "got {going:?}");

    simulation.current_turn = now + crate::agents::Agent::GAME_STAYS_ABOUT;
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.going_after_meat(&agent, (100, 100, 0)), None, "stale");
}

/// The game in sight is noted where it is; a remembered place found empty is
/// forgotten.
#[test]
fn the_game_is_noted_and_an_empty_place_forgotten() {
    let mut simulation = a_hunter();
    a_deer(&mut simulation, (110, 100));
    simulation.current_turn = 0;
    simulation.the_hunters_note_the_game();
    assert_eq!(simulation.population.agents[0].game_seen.len(), 1);
    let (at, _) = simulation.population.agents[0].game_seen[0];
    assert!((at.0 - 110).abs() <= 2 && (at.1 - 100).abs() <= 2, "{at:?}");

    // Gone, and somebody stood where it was
    simulation.world.animals.get_all_mut().clear();
    simulation.population.agents[0].state.position = (at.0, at.1, 0);
    simulation.the_hunters_note_the_game();
    assert!(simulation.population.agents[0].game_seen.is_empty());
}

/// Preparedness goes hunting (#370): a deer in sight is gone after for the
/// store by somebody who neither craves meat nor is hungry, when the larder
/// is short of the week.
#[test]
fn a_short_larder_sends_a_hunter_after_the_deer_in_sight() {
    let mut simulation = a_hunter();
    a_deer(&mut simulation, (120, 100));
    let day = simulation.world.climate.calendar.day_of_year;
    let agent = &mut simulation.population.agents[0];
    agent.state.what_the_larder_says =
        Some(crate::agents::provision::WhatIsPutBy::reckon(0.0, 1_440.0, 90.0, day));
    let agent = simulation.population.agents[0].clone();
    assert!(!agent.craves_meat(simulation.current_turn), "he does not crave it");
    assert_eq!(simulation.hunting_action(&agent, (100, 100, 0)), None, "so hunger's hunt does not send him");

    let going = simulation.going_out_for_food_to_put_by(&agent, (100, 100, 0));
    assert!(
        matches!(going, Some(Action::Move { target }) if (target.0, target.1) == (120, 100)),
        "the store sends him after the deer: {going:?}"
    );
}

/// But a hunt for the store is a careful one (#370). Hunger takes what is
/// there; putting by can wait for better odds. No boar, whatever the
/// larder says; and a goat, which turns on whoever spears it, is left
/// alone by somebody already hurt.
#[test]
fn the_store_does_not_send_a_hunter_after_what_fights_back() {
    let short = |simulation: &mut Simulation| {
        let day = simulation.world.climate.calendar.day_of_year;
        simulation.population.agents[0].state.what_the_larder_says =
            Some(crate::agents::provision::WhatIsPutBy::reckon(0.0, 1_440.0, 90.0, day));
    };

    let mut simulation = a_hunter();
    short(&mut simulation);
    let id = simulation.world.spawn_animal("boar".to_string(), (120, 100)).expect("a boar");
    simulation.world.animals.get_mut(&id).unwrap().age = 100_000;
    let agent = simulation.population.agents[0].clone();
    let going = simulation.going_out_for_food_to_put_by(&agent, (100, 100, 0));
    assert!(
        !matches!(going, Some(Action::Move { target }) if (target.0, target.1) == (120, 100)),
        "nobody goes after a boar to fill a pit: {going:?}"
    );

    let mut simulation = a_hunter();
    short(&mut simulation);
    let id = simulation.world.spawn_animal("goat".to_string(), (120, 100)).expect("a goat");
    simulation.world.animals.get_mut(&id).unwrap().age = 100_000;
    let agent = simulation.population.agents[0].clone();
    let going = simulation.going_out_for_food_to_put_by(&agent, (100, 100, 0));
    assert!(
        matches!(going, Some(Action::Move { target }) if (target.0, target.1) == (120, 100)),
        "a whole man goes after the goat: {going:?}"
    );
    simulation.population.agents[0].state.health = 60.0;
    let agent = simulation.population.agents[0].clone();
    let going = simulation.going_out_for_food_to_put_by(&agent, (100, 100, 0));
    assert!(
        !matches!(going, Some(Action::Move { target }) if (target.0, target.1) == (120, 100)),
        "and a hurt one does not: {going:?}"
    );
}
