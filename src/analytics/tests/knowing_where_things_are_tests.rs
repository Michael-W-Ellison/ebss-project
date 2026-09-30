// src/analytics/tests/knowing_where_things_are_tests.rs
//! Nobody sets out for something they have no way of knowing is there.
//!
//! The searches for somewhere to go read the whole map, so a man knew where
//! every berry within sixty paces stood whether or not he had ever been that
//! way - and the search for the best food anywhere read all of it, however
//! far. Now what an agent goes after is what it can see, what it remembers
//! (seen or told), what it can smell and what is under its hand. See
//! `Simulation::nodes_this_one_knows_of`.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// Where the man stands.
const HERE: (i32, i32, i32) = (5, 10, 0);

/// A country with one bush in it, this far east of a man standing at `HERE`.
fn one_bush(paces_east: i32) -> Simulation {
    crate::core::dice::seed(21);
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(HERE.0 + paces_east, HERE.1),
        500,
    ));
    world.file_the_nodes();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = HERE;
    population.agents[0].exploration_knowledge.known_resources.clear();

    Simulation::new(world, population)
}

fn the_bush(paces_east: i32) -> Position {
    Position::new(HERE.0 + paces_east, HERE.1)
}

#[test]
fn a_bush_in_sight_is_somewhere_to_go() {
    let simulation = one_bush(12);
    let agent = simulation.population.agents[0].clone();
    assert!(agent.sight_range() >= 12, "the fixture wants a man who can see");

    assert_eq!(simulation.the_best_food_anywhere(&agent, HERE), Some(the_bush(12)));
}

#[test]
fn a_bush_out_of_sight_and_never_seen_is_not() {
    let simulation = one_bush(32);
    let agent = simulation.population.agents[0].clone();
    assert!(agent.sight_range() < 32, "the fixture wants the bush out of sight");

    assert_eq!(
        simulation.the_best_food_anywhere(&agent, HERE),
        None,
        "he went after a bush he has never seen or heard of"
    );
}

#[test]
fn a_bush_out_of_sight_he_remembers_is() {
    let mut simulation = one_bush(32);
    simulation.population.agents[0]
        .exploration_knowledge
        .discover_resource(the_bush(32), ResourceType::Food, 0);
    let agent = simulation.population.agents[0].clone();

    assert_eq!(simulation.the_best_food_anywhere(&agent, HERE), Some(the_bush(32)));
}

#[test]
fn a_bush_out_of_sight_he_was_told_about_is() {
    let mut simulation = one_bush(32);
    simulation.population.agents[0]
        .exploration_knowledge
        .take_their_word_for_it(
            the_bush(32),
            ResourceType::Food,
            uuid::Uuid::from_u128(7),
            0,
            Some(500),
            0,
        );
    let agent = simulation.population.agents[0].clone();

    assert_eq!(simulation.the_best_food_anywhere(&agent, HERE), Some(the_bush(32)));
}

#[test]
fn a_blind_man_finds_what_is_to_hand_and_not_what_is_across_the_clearing() {
    let mut simulation = one_bush(6);
    simulation.population.agents[0].senses.vision.impaired = true;
    let agent = simulation.population.agents[0].clone();
    assert_eq!(agent.sight_range(), 0);

    assert_eq!(
        simulation.the_best_food_anywhere(&agent, HERE),
        None,
        "a blind man walked six paces straight to a bush nobody told him of"
    );

    let mut simulation = one_bush(1);
    simulation.population.agents[0].senses.vision.impaired = true;
    let agent = simulation.population.agents[0].clone();

    assert_eq!(simulation.the_best_food_anywhere(&agent, HERE), Some(the_bush(1)));
}

#[test]
fn what_he_knows_of_is_read_off_the_ground_near_him_in_list_order() {
    let mut simulation = one_bush(3);
    for east in [40, 2, 9] {
        simulation.world.resources.push(ResourceNode::new(
            ResourceType::Wood,
            the_bush(east),
            10,
        ));
    }
    simulation.world.file_the_nodes();
    simulation.population.agents[0]
        .exploration_knowledge
        .discover_resource(the_bush(40), ResourceType::Wood, 0);
    let agent = simulation.population.agents[0].clone();

    let from = Position::new(HERE.0, HERE.1);
    assert_eq!(simulation.nodes_this_one_knows_of(&agent, from, 60), vec![0, 1, 2, 3]);
    assert_eq!(
        simulation.nodes_this_one_knows_of(&agent, from, 5),
        vec![0, 2],
        "reach is still reach"
    );
}
