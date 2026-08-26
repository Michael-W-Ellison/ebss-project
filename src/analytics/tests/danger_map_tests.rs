// src/analytics/tests/danger_map_tests.rs
//! Tests for the danger an agent carries about in its head.
//!
//! `ExplorationKnowledge` held explored tiles, resource positions with an age
//! and a source, buildings, storage and terrains — a real picture of the
//! world's *things* — and nothing whatever about danger. An agent could be
//! mauled at a ford and walk back to the same ford the next morning with no
//! more hesitation than the first time, because there was nowhere for "there
//! are wolves in that wood" to live.
//!
//! It fades, and it fades for the same reason a claim about a berry patch
//! fades: a pack works a wood for a season and then moves on, and a man who
//! avoids that wood for the rest of his life is not being careful, he is being
//! wrong.

use crate::agents::exploration::Danger;
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::world::{Position, World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation
}

// --------------------------------------------------------------------------
// Remembering it
// --------------------------------------------------------------------------

/// Nobody starts out afraid of anywhere.
#[test]
fn a_new_map_has_no_bad_places_on_it() {
    let simulation = one_person();
    let map = &simulation.population.agents[0].exploration_knowledge;

    assert!(map.where_it_went_badly.is_empty());
    assert_eq!(map.how_bad_is_it_there(Position::new(25, 25), 0), 0.0);
}

/// Seeing something puts it on the map.
#[test]
fn seeing_something_puts_it_on_the_map() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.saw_danger(Position::new(30, 30), "wolves", 0.8, 100);

    assert!(map.how_bad_is_it_there(Position::new(30, 30), 100) > 0.0);
    assert_eq!(
        map.what_is_wrong_with_that_place(Position::new(30, 30), 100),
        Some("wolves")
    );
}

/// "There are wolves in that wood" is not a fact about one tile.
#[test]
fn a_bad_place_is_wider_than_a_tile() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.saw_danger(Position::new(30, 30), "wolves", 0.8, 100);

    assert!(
        map.how_bad_is_it_there(Position::new(32, 31), 100) > 0.0,
        "the next tile over is the same wood"
    );
    assert_eq!(
        map.how_bad_is_it_there(Position::new(45, 45), 100),
        0.0,
        "and the far side of the map is not"
    );
}

/// It fades. A pack works a wood for a season and moves on.
#[test]
fn a_fright_fades() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.saw_danger(Position::new(30, 30), "wolves", 1.0, 0);

    let fresh = map.how_bad_is_it_there(Position::new(30, 30), 0);
    let later = map.how_bad_is_it_there(Position::new(30, 30), TICKS_PER_DAY * 8);

    assert!(later < fresh, "{fresh} should fade to less than itself");
    assert_eq!(
        map.how_bad_is_it_there(Position::new(30, 30), Danger::HOW_LONG_A_FRIGHT_LASTS + 1),
        0.0,
        "and be gone entirely after a season"
    );
}

/// A quiet afternoon in a bad wood does not talk anybody into going back.
#[test]
fn one_quiet_afternoon_does_not_undo_a_mauling() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.saw_danger(Position::new(30, 30), "bear", 1.0, 100);
    map.saw_danger(Position::new(30, 30), "rabbit", 0.05, 101);

    assert_eq!(
        map.what_is_wrong_with_that_place(Position::new(30, 30), 101),
        Some("bear"),
        "the worse of the two is what somebody remembers"
    );
}

/// Nobody carries an unbounded number of frights about with them.
#[test]
fn a_person_holds_only_so_many_bad_places() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    for i in 0..200 {
        map.saw_danger(Position::new(i % 90, i / 90), "wolves", 0.9, 100 + i as u32);
    }

    assert!(
        map.where_it_went_badly.len() <= 64,
        "a head is not a filing cabinet: {}",
        map.where_it_went_badly.len()
    );
}

// --------------------------------------------------------------------------
// Where it comes from
// --------------------------------------------------------------------------

/// A beast worth more in a fight than the man looking at it goes on the map.
#[test]
fn something_that_could_kill_you_goes_on_the_map() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    for _ in 0..4 {
        simulation
            .world
            .spawn_animal("wolf".to_string(), (here.0 + 3, here.1))
            .expect("there should be wolves to be frightened of");
    }

    // Hold both of them still. A pack that has just been walked up to is
    // nine paces off by the end of the tick, which is a fact about wolves
    // rather than about the map.
    for _ in 0..20 {
        simulation.population.agents[0].state.position = here;
        for animal in simulation.world.animals.get_all_mut() {
            animal.position = (here.0 + 3, here.1);
        }
        simulation.tick();
        if !simulation.population.agents[0].state.is_alive {
            break;
        }
    }

    let alive = simulation.world.animals.get_all().iter().filter(|a| a.is_alive()).count();
    let where_they_are: Vec<_> = simulation.world.animals.get_all().iter()
        .filter(|a| a.is_alive()).map(|a| a.position).collect();
    let map = &simulation.population.agents[0].exploration_knowledge;
    assert!(
        map.where_it_went_badly
            .values()
            .any(|danger| danger.how_bad > 0.0),
        "four wolves three paces away and nothing on the map about it          (alive: {alive}, at {where_they_are:?}, agent at {here:?})"
    );
}

/// And a rabbit does not.
#[test]
fn something_harmless_does_not() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    for _ in 0..6 {
        simulation
            .world
            .spawn_animal("rabbit".to_string(), (here.0 + 2, here.1))
            .expect("rabbits spawn");
    }

    for _ in 0..20 {
        simulation.population.agents[0].state.position = here;
        for animal in simulation.world.animals.get_all_mut() {
            animal.position = (here.0 + 2, here.1);
        }
        simulation.tick();
        if !simulation.population.agents[0].state.is_alive {
            break;
        }
    }

    let map = &simulation.population.agents[0].exploration_knowledge;
    assert!(
        map.where_it_went_badly.is_empty(),
        "nobody has ever been frightened of a rabbit: {:?}",
        map.where_it_went_badly
    );
}

// --------------------------------------------------------------------------
// Acting on it
// --------------------------------------------------------------------------

/// A patch in a bad wood is further away than it measures.
#[test]
fn a_bad_place_is_further_away_than_it_measures() {
    use crate::world::{ResourceNode, ResourceType};

    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    // Two patches, one close and one further off.
    let near = Position::new(here.0 + 3, here.1);
    let far = Position::new(here.0 + 9, here.1);

    simulation.world.resources.clear();
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Food, near, 40));
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Food, far, 40));

    let chosen = simulation.nearest_edible_this_one_would_go_to(
        &simulation.population.agents[0],
        here,
        30,
    );
    assert_eq!(chosen, Some(near), "with nothing known, the near one wins");

    // Now put wolves in the near one's memory.
    simulation.population.agents[0]
        .exploration_knowledge
        .saw_danger(near, "wolves", 1.0, simulation.current_tick);

    let chosen = simulation.nearest_edible_this_one_would_go_to(
        &simulation.population.agents[0],
        here,
        30,
    );
    assert_eq!(
        chosen,
        Some(far),
        "and having seen wolves there, the further one is the shorter walk"
    );
}

/// But hunger outlasts a fright: a settlement that starves rather than walk
/// past a wood is not being careful.
#[test]
fn the_only_patch_there_is_gets_walked_to_anyway() {
    use crate::world::{ResourceNode, ResourceType};

    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;
    let only = Position::new(here.0 + 3, here.1);

    simulation.world.resources.clear();
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Food, only, 40));

    simulation.population.agents[0]
        .exploration_knowledge
        .saw_danger(only, "bear", 1.0, simulation.current_tick);

    assert_eq!(
        simulation.nearest_edible_this_one_would_go_to(
            &simulation.population.agents[0],
            here,
            30,
        ),
        Some(only),
        "there is nowhere else to eat"
    );
}

// --------------------------------------------------------------------------
// People
// --------------------------------------------------------------------------

/// Where somebody was last actually seen, rather than where they are.
#[test]
fn somebody_remembers_where_they_last_saw_you() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;
    let somebody = uuid::Uuid::new_v4();

    assert!(map.where_did_i_last_see(somebody, 0).is_none());

    map.saw_somebody(somebody, Position::new(40, 40), 100);

    assert_eq!(
        map.where_did_i_last_see(somebody, 100),
        Some(Position::new(40, 40))
    );
}

/// And a sighting goes stale, because people move.
#[test]
fn a_sighting_goes_stale() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;
    let somebody = uuid::Uuid::new_v4();

    map.saw_somebody(somebody, Position::new(40, 40), 100);

    assert!(
        map.where_did_i_last_see(somebody, 100 + TICKS_PER_DAY * 3)
            .is_none(),
        "three days on, that is not where they are"
    );
}

/// Two people standing together see each other.
#[test]
fn people_standing_together_see_each_other() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let them = simulation.population.agents[1].id;

    for _ in 0..12 {
        simulation.population.agents[0].state.position = (25, 25, 0);
        simulation.population.agents[1].state.position = (26, 25, 0);
        simulation.tick();
    }

    let now = simulation.current_tick;
    assert!(
        simulation.population.agents[0]
            .exploration_knowledge
            .where_did_i_last_see(them, now)
            .is_some(),
        "they were standing a pace apart for a day"
    );
}
