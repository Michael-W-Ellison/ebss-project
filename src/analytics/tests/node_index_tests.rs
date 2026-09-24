// src/analytics/tests/node_index_tests.rs
//! Where the nodes are: a question about somewhere reads only what is near
//! it, and gets the answer reading the whole map got.
//!
//! See `world::node_index`. The claim is sameness, not closeness - the same
//! nodes, in the same order - so these compare the filed answer against a
//! plain walk of the whole list.

use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// The plain answer: every node within `reach` either way, by walking the
/// whole list.
fn by_walking_the_list(world: &World, at: Position, reach: u32) -> Vec<usize> {
    let reach = reach as i32;
    world
        .resources
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            (node.position.x - at.x).abs() <= reach && (node.position.y - at.y).abs() <= reach
        })
        .map(|(number, _)| number)
        .collect()
}

/// Somewhere to ask about, spread over the map and off its edges.
fn places_to_ask(world: &World) -> Vec<Position> {
    let (w, h) = (world.grid.width as i32, world.grid.height as i32);
    let mut places = vec![
        Position::new(0, 0),
        Position::new(w - 1, h - 1),
        Position::new(-5, -5),
        Position::new(w + 3, h / 2),
    ];
    for i in 0..40 {
        places.push(Position::new((i * 37) % w, (i * 53) % h));
    }
    places
}

#[test]
fn the_file_gives_the_answer_the_whole_list_gives() {
    crate::core::dice::seed(11);
    let mut world = World::new(WorldConfig::default());
    world.file_the_nodes();

    assert!(world.resources.len() > 100, "a world with something on it");

    for at in places_to_ask(&world) {
        for reach in [0, 1, 3, 7, 8, 12, 25, 60, 1_000] {
            assert_eq!(
                world.node_numbers_near(at, reach),
                by_walking_the_list(&world, at, reach),
                "at {at:?} within {reach}"
            );
        }
    }
}

#[test]
fn a_node_put_down_or_taken_up_is_filed_as_it_goes() {
    crate::core::dice::seed(12);
    let mut world = World::new(WorldConfig::default());
    world.file_the_nodes();

    let there = Position::new(3, 4);
    let number = world.put_a_node_down(ResourceNode::new(ResourceType::Grain, there, 10));
    assert_eq!(number, world.resources.len() - 1);
    assert!(world.node_numbers_on(there).contains(&number));

    // Taking one up moves every node after it; the answers move with them.
    world.take_a_node_up(0);
    for at in places_to_ask(&world) {
        assert_eq!(
            world.node_numbers_near(at, 9),
            by_walking_the_list(&world, at, 9),
            "at {at:?}"
        );
    }
}

#[test]
fn a_list_changed_behind_the_files_back_is_still_answered_right() {
    crate::core::dice::seed(13);
    let mut world = World::new(WorldConfig::default());
    world.file_the_nodes();

    // By hand, as a test would: the file no longer counts what the list holds.
    let there = Position::new(10, 10);
    assert_ne!(world.resources.last().map(|node| node.position), Some(there));
    world.resources.push(ResourceNode::new(ResourceType::Wood, there, 5));
    world.resources.remove(1);

    for at in places_to_ask(&world) {
        assert_eq!(
            world.node_numbers_near(at, 6),
            by_walking_the_list(&world, at, 6),
            "at {at:?}"
        );
    }

    // And refiling puts it right.
    world.file_the_nodes();
    for at in places_to_ask(&world) {
        assert_eq!(world.node_numbers_near(at, 6), by_walking_the_list(&world, at, 6));
    }
}
