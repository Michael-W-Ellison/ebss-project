// src/analytics/tests/crafting_composition_tests.rs
//! Crafting as a three-step composition, and the three things that stopped it.
//!
//! Asked what runs the making drives held, the answer was none: eight worlds,
//! seventy bodies, three quarters of the way through the first year, and not
//! one composition on Utility, Industry or Construction. It was not one
//! defect but three, each of which on its own was enough.
//!
//! 1. Every chain was blocked by a guard that belongs on the reader and not
//!    on the chain - see `the_chain_is_not_stopped_by_the_atom`.
//! 2. The chain walk took the fattest step rather than the one that went
//!    anywhere - see `a_run_that_goes_further_beats_a_fatter_first_step`.
//! 3. The making verbs were written down under ten names, so no making run
//!    ever got worn - see `the_making_verbs_go_in_under_one_name`.
//! 4. And what "worn" meant was a number in units of hunger - see
//!    `a_thin_drive_is_not_held_to_a_fat_ones_yardstick`.

use crate::agents::patterns::{Element, Patterns};
use crate::agents::{Agent, AgentConfig};
use crate::core::DriveType;
use crate::environment::making;
use crate::environment::Action;

/// A run, worn by hand, so a test can say what it wants without a world.
fn wear(patterns: &mut Patterns, need: DriveType, run: &Element, times: u32, each: f32) {
    for round in 0..times {
        patterns.it_worked(need, std::slice::from_ref(run), each, round);
    }
}

/// Knapping, carving and scraping are one thing to the layer that learns.
#[test]
fn the_making_verbs_go_in_under_one_name() {
    for verb in ["craft", "cut", "carve", "scrape", "smash", "mold", "weave"] {
        assert_eq!(
            making::what_making_is_called(verb),
            "craft",
            "{verb} turns material into an object and belongs with the rest"
        );
    }

    // And what is not a making keeps its own name. Drying and salting are
    // preservation, and answer a different need on a different run.
    for verb in ["dry", "salt", "gather", "eat", "build", "burrow"] {
        assert_eq!(making::what_making_is_called(verb), verb);
    }
}

/// And it holds through the action, which is where it has to hold.
#[test]
fn a_turn_spent_carving_is_written_down_as_making() {
    let carving = Action::Work {
        verb: "carve".to_string(),
        to: "wood".to_string(),
    };
    let knapping = Action::Work {
        verb: "smash".to_string(),
        to: "flint".to_string(),
    };
    let drying = Action::Dry {
        what: "meat".to_string(),
    };

    assert_eq!(
        Agent::just_the_verb(&Agent::what_was_tried(&carving)),
        "craft"
    );
    assert_eq!(
        Agent::just_the_verb(&Agent::what_was_tried(&knapping)),
        "craft"
    );
    assert_eq!(Agent::just_the_verb(&Agent::what_was_tried(&drying)), "dry");
}

/// Three days of making three different things is one run learned, not three
/// tenths of three.
///
/// This is the whole of what folding buys. **Measured**: against Utility the
/// store held `gather > craft` at 0.242, `gather > cut` at 0.185,
/// `gather > carve` at 0.305, `gather > smash` at 0.392 and `gather > mold`
/// at 0.214 - five spellings of one habit, none of them worn.
#[test]
fn a_week_of_making_different_things_wears_one_run() {
    let mut agent = Agent::new(AgentConfig::default());

    for (round, verb) in ["carve", "smash", "scrape", "cut", "mold", "carve"]
        .into_iter()
        .enumerate()
    {
        agent.that_is_what_i_just_did(&Action::Gather {
            resource_type: "wood".to_string(),
        });
        let making = Action::Work {
            verb: verb.to_string(),
            to: "wood".to_string(),
        };
        let elements = agent.what_led_up_to_this(&making);
        agent
            .patterns
            .it_worked(DriveType::Utility, &elements, 0.2, round as u32);
        agent.that_is_what_i_just_did(&making);
    }

    let one_run = Element::Then("gather".to_string(), "craft".to_string());
    assert!(
        agent.patterns.trail(DriveType::Utility, &one_run).is_some(),
        "get the stuff, then make the thing - one habit however it is spelled"
    );
    assert!(
        agent
            .patterns
            .trail(
                DriveType::Utility,
                &Element::Then("gather".to_string(), "carve".to_string())
            )
            .is_none(),
        "and nothing is kept under the particular spelling"
    );
}

/// The chain follows worn runs whether or not each step beats itself.
///
/// The reader asks "what should I do instead of the obvious thing", where a
/// run that does not beat its own second half is no reason to depart from the
/// obvious thing. The chain asks "what came next", which is a question about
/// order. **Measured**: with the one guard doing both jobs, no chain of two
/// ever formed, on any drive, in any world - twelve worlds, a hundred and two
/// bodies, not one.
#[test]
fn the_chain_is_not_stopped_by_the_atom() {
    let mut patterns = Patterns::default();
    let run = Element::Then("gather".to_string(), "craft".to_string());
    let alone = Element::Did("craft".to_string());

    wear(&mut patterns, DriveType::Utility, &run, 6, 0.5);
    // And the act on its own is worth far more than the run, which is the
    // ordinary case: it is credited every time, and the run only when the
    // order happened to be that one.
    wear(&mut patterns, DriveType::Utility, &alone, 40, 0.5);

    assert!(
        patterns.what_follows(DriveType::Utility, "gather").is_none(),
        "the reader still refuses to depart from the plain answer"
    );
    assert_eq!(
        patterns.the_chain_that_answers(DriveType::Utility, "gather"),
        vec!["craft".to_string()],
        "but the run is still the run, and a plan is made of runs"
    );
}

/// A chain takes the step that goes somewhere, not the one that weighs most.
///
/// On Utility, `gather > pickup` carries more than half again what
/// `gather > craft` does, and picking a thing up leads nowhere. Taking the
/// fattest step first found no composition at all.
#[test]
fn a_run_that_goes_further_beats_a_fatter_first_step() {
    let mut patterns = Patterns::default();

    let fat = Element::Then("gather".to_string(), "pickup".to_string());
    let thin = Element::Then("gather".to_string(), "craft".to_string());
    let onward = Element::Then("craft".to_string(), "equip".to_string());

    wear(&mut patterns, DriveType::Utility, &fat, 8, 0.5);
    wear(&mut patterns, DriveType::Utility, &thin, 4, 0.5);
    wear(&mut patterns, DriveType::Utility, &onward, 4, 0.5);

    assert_eq!(
        patterns.the_chain_that_answers(DriveType::Utility, "gather"),
        vec!["craft".to_string(), "equip".to_string()],
        "get the stuff, make the thing, put it on: three steps beat two"
    );
}

/// A run is a habit when it has worked often enough, whatever it is worth.
#[test]
fn one_good_afternoon_is_not_a_habit() {
    let mut patterns = Patterns::default();
    let run = Element::Then("gather".to_string(), "craft".to_string());

    wear(
        &mut patterns,
        DriveType::Utility,
        &run,
        Patterns::ENOUGH_TIMES_TO_BE_A_HABIT - 1,
        4.0,
    );
    assert!(
        patterns
            .the_chain_that_answers(DriveType::Utility, "gather")
            .is_empty(),
        "however well it paid, twice is not a habit"
    );

    wear(&mut patterns, DriveType::Utility, &run, 1, 4.0);
    assert_eq!(
        patterns.the_chain_that_answers(DriveType::Utility, "gather"),
        vec!["craft".to_string()],
    );
}

/// And a thin drive is not held to a fat drive's yardstick.
///
/// A trail is fed with demand off the drive per turn spent. A meal takes
/// nine-tenths off Hunger and a making takes two-tenths off Utility, so a
/// fixed bar means four successes on one and eighteen on the other.
/// **Measured**: fifty-two bodies in seventy held `gather > craft`, nearly
/// everybody, and not one was over the fixed bar.
#[test]
fn a_thin_drive_is_not_held_to_a_fat_ones_yardstick() {
    let mut patterns = Patterns::default();
    let making = Element::Then("gather".to_string(), "craft".to_string());

    // Six makings, at what a making is actually worth. Well under the fixed
    // number, and the only thing this body knows about Utility.
    wear(&mut patterns, DriveType::Utility, &making, 6, 0.05);
    assert!(
        patterns
            .trail(DriveType::Utility, &making)
            .map(|trail| trail.worth() < Patterns::WORN_ENOUGH_TO_FOLLOW)
            .unwrap_or(false),
        "the run really is under the fixed bar - otherwise this proves nothing"
    );
    assert_eq!(
        patterns.the_chain_that_answers(DriveType::Utility, "gather"),
        vec!["craft".to_string()],
        "six makings is a habit by the standards of making"
    );
}

/// The looser bar never lets a shallow run past on a drive whose runs are
/// deep, because it is the lower of the two and not the newer of them.
#[test]
fn a_deep_drive_keeps_the_bar_it_had() {
    let mut patterns = Patterns::default();
    let deep = Element::Then("gather".to_string(), "eat".to_string());
    let shallow = Element::Then("gather".to_string(), "examine".to_string());

    wear(&mut patterns, DriveType::Hunger, &deep, 20, 0.9);
    wear(&mut patterns, DriveType::Hunger, &shallow, 4, 0.02);

    // A share of the deepest run would be far over the fixed number here, so
    // the fixed number is what applies, and the shallow run is still out.
    assert_eq!(
        patterns.the_chain_that_answers(DriveType::Hunger, "gather"),
        vec!["eat".to_string()],
        "eating is what follows gathering; looking at it is not"
    );
}
