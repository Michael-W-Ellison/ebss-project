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

// ---------------------------------------------------------------------------
// Novelty, and the forgetting that keeps it coming back
// ---------------------------------------------------------------------------

use crate::agents::practices::Lessons;
use crate::environment::seasons::{DAYS_PER_SEASON, TICKS_PER_DAY};

/// Diminishing returns, and no second rule to say so.
///
/// "Trying something new, even if it does not work, helps satisfy the drive...
/// but it should be diminishing returns until they are forced to pick new
/// unknown actions." One over one plus the count is the whole of it.
#[test]
fn each_go_at_a_thing_is_worth_less_than_the_one_before() {
    let mut lessons = Lessons::default();

    let never = lessons.how_new_is_this("stack:stone");
    assert_eq!(never, 1.0, "an untried thing should be wholly new");

    let mut worth = Vec::new();
    for _ in 0..4 {
        lessons.record_particular("stack:stone", true);
        worth.push(lessons.how_new_is_this("stack:stone"));
    }

    assert_eq!(worth, vec![0.5, 1.0 / 3.0, 0.25, 0.2]);
    for pair in worth.windows(2) {
        assert!(
            pair[1] < pair[0],
            "the next go was worth as much as the last: {pair:?}"
        );
    }
}

/// Novelty is not about whether it worked.
///
/// A man is not curious about a thing because it pays. Four failures make a
/// thing exactly as stale as four successes, and `how_likely_to_try_this` is
/// where the difference lives.
#[test]
fn what_is_new_does_not_depend_on_what_came_of_it() {
    let mut paid = Lessons::default();
    let mut did_not = Lessons::default();

    // A fair go apiece: below `A_FAIR_GO` the agent reserves judgement and
    // both read as nearly certain, which would let this pass without
    // measuring anything.
    for _ in 0..12 {
        paid.record_particular("stack:stone", true);
        did_not.record_particular("stack:stone", false);
    }

    assert_eq!(
        paid.how_new_is_this("stack:stone"),
        did_not.how_new_is_this("stack:stone"),
        "novelty read the outcome, which is the other question"
    );
    assert!(
        paid.how_likely_to_try_this("stack:stone")
            > did_not.how_likely_to_try_this("stack:stone"),
        "and the other question stopped telling them apart"
    );
}

/// A thing tried once and never again is new to its own agent by the autumn.
///
/// "Once an action is attempted, if it did nothing, the agent will need to
/// forget that it tried the action to use it to satisfy its curiosity drive
/// demand again."
#[test]
fn a_thing_tried_once_and_left_is_new_again_within_the_year() {
    let mut lessons = Lessons::default();
    lessons.record_particular("stack:stone", false);
    assert_eq!(lessons.how_new_is_this("stack:stone"), 0.5);

    // A season of not doing it. `fade` is charged by the day, so one call
    // carrying a season's worth of ticks is the same as ninety daily ones.
    lessons.fade(DAYS_PER_SEASON * TICKS_PER_DAY);

    assert_eq!(
        lessons.how_new_is_this("stack:stone"),
        1.0,
        "a season went by and he still remembers the one stone he stacked"
    );
}

/// And a thing kept up is not forgotten.
///
/// The other half. A rule that forgot everything on a clock would make an
/// agent new to its own trade every spring; what is wanted is that things go
/// because nobody did them again.
#[test]
fn a_thing_done_often_survives_the_season() {
    let mut lessons = Lessons::default();
    for _ in 0..40 {
        lessons.record_particular("gather:roots", true);
    }
    let before = lessons.tried_this("gather:roots");

    lessons.fade(DAYS_PER_SEASON * TICKS_PER_DAY);
    let after = lessons.tried_this("gather:roots");

    assert!(after < before, "a season took nothing off it at all: {before}");
    assert!(
        after > 0,
        "forty goes at the same thing and a single season wiped it: {before} then {after}"
    );
}

/// Forgetting leaves a lesson where it started, not where it failed.
///
/// Drifting the belief towards nought would make forgetting the same thing as
/// having found it useless, which is backwards: a man who cannot remember
/// trying something is in the position of never having tried it - worth one
/// attempt and no more.
#[test]
fn forgetting_a_failure_leaves_it_worth_one_more_go() {
    let mut lessons = Lessons::default();
    for _ in 0..12 {
        lessons.record_particular("fold:hide", false);
    }
    let soured = lessons.how_likely_to_try_this("fold:hide");

    // Long enough that the count is gone entirely. A dozen goes halve every
    // season, so they fall under `TOO_FAINT_TO_COUNT` at about fourteen
    // months - a thing done a dozen times is remembered for rather more than
    // a year, which is the shape wanted.
    lessons.fade(5 * DAYS_PER_SEASON * TICKS_PER_DAY);

    assert!(
        lessons.how_likely_to_try_this("fold:hide") > soured,
        "he forgot failing at it and is still as sour on it: {soured:.2}"
    );
    assert_eq!(
        lessons.how_new_is_this("fold:hide"),
        1.0,
        "the count outlived the forgetting"
    );
}

/// Fade is charged by the day, so calling it every turn costs nothing.
#[test]
fn fading_twice_in_a_day_takes_no_more_than_fading_once() {
    let mut once = Lessons::default();
    let mut often = Lessons::default();
    for _ in 0..40 {
        once.record_particular("gather:roots", true);
        often.record_particular("gather:roots", true);
    }

    let a_day = DAYS_PER_SEASON * TICKS_PER_DAY;
    once.fade(a_day);
    for tick in 0..=a_day {
        often.fade(tick);
    }

    assert_eq!(
        once.tried_this("gather:roots"),
        often.tried_this("gather:roots"),
        "fading every turn charged more than fading once"
    );
}
