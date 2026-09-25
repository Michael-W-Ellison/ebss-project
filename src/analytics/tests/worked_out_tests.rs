// src/analytics/tests/worked_out_tests.rs
//! A spent seam is looked for when something is spent, not every turn.
//!
//! `World::remove_depleted_resources` walked every node on the map every turn
//! for what only a hand can do. Now whatever takes from a node asks
//! `World::did_that_empty_it`, and the world sweeps only when told. See
//! ISSUES_FOUND #251.

use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// A world with a stone seam and a berry bush on it and nothing else, looked
/// over once already.
fn a_seam_and_a_bush() -> World {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.resources.push(ResourceNode::new(ResourceType::Stone, Position::new(5, 5), 10));
    world.resources.push(ResourceNode::new(ResourceType::Food, Position::new(6, 5), 10));
    world.file_the_nodes();
    world.take_a_turn();
    world
}

#[test]
fn a_seam_a_hand_empties_is_gone_by_the_end_of_the_turn() {
    let mut world = a_seam_and_a_bush();

    world.resources[0].harvest(10);
    assert!(world.did_that_empty_it(0), "the last of the stone is the last of the seam");
    world.take_a_turn();

    assert_eq!(world.resources.len(), 1);
    assert_eq!(world.resources[0].resource_type, ResourceType::Food);
    assert!(world.where_it_was_worked_out.contains(&Position::new(5, 5)));
}

#[test]
fn a_bush_picked_bare_stays_to_bear_again() {
    let mut world = a_seam_and_a_bush();

    world.resources[1].harvest(10);
    assert!(!world.did_that_empty_it(1), "a bush is not used up by being picked");
    world.take_a_turn();

    assert_eq!(world.resources.len(), 2);
}

#[test]
fn a_world_just_made_looks_on_its_first_turn() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    let mut spent = ResourceNode::new(ResourceType::Iron, Position::new(3, 3), 10);
    spent.amount = 0;
    world.resources.push(spent);

    world.take_a_turn();

    assert!(world.resources.is_empty(), "a spent seam from before anybody looked is swept");
}

/// And a new way of emptying a seam that forgets to say so is caught in a
/// debug build, rather than leaving a spent seam on the map for ever.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "ran out that nobody told the world about")]
fn a_seam_emptied_without_saying_so_is_caught() {
    let mut world = a_seam_and_a_bush();

    world.resources[0].harvest(10);
    world.take_a_turn();
}
