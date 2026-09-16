// src/agents/tests/affordance_tests.rs
//! What a verb does to a kind of thing.
//!
//! "An agent attempts to pick up a tree, nothing happens, the agent fails. It
//! then learns it cannot pick up trees. The agent attempts to pick up a rock
//! and succeeds, the agent learns that it can pick up rocks."
//!
//! It could not. `Agent::what_was_tried` is the key every lesson is filed
//! under, and ten of its arms threw the object away - `PickUp` on a tree and
//! `PickUp` on a stone were the same row, and averaged to *picking things up
//! works about half the time*. Both facts went to one place, so neither
//! survived.

use crate::agents::wondering::Wondering;
use crate::agents::{Agent, AgentConfig};
use crate::environment::Action;

/// Enough attempts that `Lessons` will hold an opinion rather than reserving
/// judgement. Below `A_FAIR_GO` everything reads as nearly certain, which is
/// right for a thing nobody has given a fair go and would make these tests
/// pass without measuring anything.
const A_FAIR_GO: u32 = 12;

fn lift(what: &str) -> Action {
    Action::PickUp {
        what: what.to_string(),
    }
}

/// The lesson names the thing, so two things are two lessons.
#[test]
fn picking_up_two_different_things_is_two_different_lessons() {
    let tree = Agent::what_was_tried(&lift("tree"));
    let stone = Agent::what_was_tried(&lift("stone"));

    assert_ne!(
        tree, stone,
        "a tree and a stone are filed under one key, so neither can be learned about"
    );
    assert!(
        tree.contains("tree") && stone.contains("stone"),
        "the key should name what was tried on: got {tree} and {stone}"
    );
}

/// A man who cannot lift a tree still picks up stones.
///
/// The whole of the defect, put as the thing an agent should end up believing.
/// With one shared key the failures and the successes cancel and he is left
/// half-hearted about both.
#[test]
fn a_man_who_cannot_lift_a_tree_still_picks_up_stones() {
    let mut agent = Agent::new(AgentConfig::default());

    for _ in 0..A_FAIR_GO {
        agent
            .lessons
            .record_particular(&Agent::what_was_tried(&lift("tree")), false);
        agent
            .lessons
            .record_particular(&Agent::what_was_tried(&lift("stone")), true);
    }

    let tree = agent
        .lessons
        .how_likely_to_try_this(&Agent::what_was_tried(&lift("tree")));
    let stone = agent
        .lessons
        .how_likely_to_try_this(&Agent::what_was_tried(&lift("stone")));

    assert!(
        stone > tree,
        "twelve trees that would not move and twelve stones that would, and he \
         is no keener on the stones: {stone:.2} against {tree:.2}"
    );
    assert!(
        tree < 0.5,
        "he has failed at a tree twelve times and still fancies his chances: {tree:.2}"
    );
    assert!(
        stone > 0.5,
        "he has lifted a stone twelve times and does not believe he can: {stone:.2}"
    );
}

/// Salting a joint and finding out what became of it are one lesson.
///
/// `Wondering::called` has always been `salt:meat`; the salting itself wrote a
/// bare `salt`. Two rows, and neither ever saw the other's evidence - the
/// recurring "two spellings of one question" in this model. This asserts they
/// agree rather than asserting either spelling, so whichever moves, the test
/// notices.
#[test]
fn salting_a_joint_and_the_question_about_it_are_one_lesson() {
    let salted = Agent::what_was_tried(&Action::Salt {
        what: "meat".to_string(),
    });

    let asked = Wondering {
        did: Wondering::SALTING_IT.to_string(),
        what: "meat".to_string(),
        where_it_is: crate::world::Position::new(0, 0),
        since: 0,
        as_it_was: crate::agents::wondering::Watched {
            called: "meat".to_string(),
            freshness: None,
            preparation: None,
        },
        in_this: Vec::new(),
    };

    assert_eq!(
        salted,
        asked.called(),
        "the salting and the question about it are filed apart, so a man who \
         salts a joint never learns from what becomes of it"
    );
}

/// A person is not a kind of thing, and the key does not pretend otherwise.
///
/// This guards a deliberate exclusion rather than a fix. `Trade`, `GiveTo` and
/// `TakeFrom` carry a `Uuid`: keying on it would put one row per neighbour
/// into a map meant to hold what an agent knows about *sorts* of thing, which
/// grows without bound and never asks the same question twice. What is known
/// about a particular person lives in `relationships`. Anybody bringing these
/// arms into line with the rest by sweep will fail here and read why.
#[test]
fn a_person_is_not_a_kind_of_thing() {
    let somebody = uuid::Uuid::new_v4();
    let them = somebody.to_string();

    for action in [
        Action::Trade { with: somebody },
        Action::GiveTo { to: somebody },
        Action::TakeFrom { from: somebody },
    ] {
        let key = Agent::what_was_tried(&action);
        assert!(
            !key.contains(&them),
            "the lesson is keyed on who it was, so there is a row per \
             neighbour and none of them is ever asked twice: {key}"
        );
    }
}
