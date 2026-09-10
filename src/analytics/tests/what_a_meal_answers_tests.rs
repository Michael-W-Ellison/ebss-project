// src/analytics/tests/what_a_meal_answers_tests.rs
//! A mouthful does not buy off a starving man's hunger.
//!
//! Hunger is an accumulator - `drive.value += rate` - not something re-read
//! off the body each turn, and every eating path discharged it by a flat
//! three tenths: three tenths for a full sitting, three tenths for one berry.
//! So a man eleven days into a three-week reserve who found a single handful
//! stopped being hungry enough to act, another drive won the turn, and he
//! walked away from the food still emptying.
//!
//! Measured over eight seeded world-years, sampling every living body once a
//! day, the fold shows in the middle of the table: bodies at a quarter to a
//! half of their reserve carry a hunger of 0.96 on an empty stomach, and
//! bodies under a *tenth* of their reserve carry 0.71 with a hundred and
//! fifty energy in the belly. The ones nearest death were the less hungry.

use crate::agents::physiology::{what_this_meal_answers, WHAT_A_SITTING_AIMS_AT};

/// A full sitting answers the whole of what a full sitting used to answer.
#[test]
fn a_full_sitting_answers_what_it_always_did() {
    assert_eq!(what_this_meal_answers(WHAT_A_SITTING_AIMS_AT), 1.0);
    assert_eq!(
        crate::analytics::WHAT_A_FULL_SITTING_ANSWERS
            * what_this_meal_answers(WHAT_A_SITTING_AIMS_AT),
        0.3,
        "a whole meal must be worth exactly what it was before, or every \
         number measured against the old behaviour moves for nothing"
    );
}

/// And a mouthful answers a mouthful's worth.
#[test]
fn a_mouthful_answers_a_mouthful() {
    let a_fifth = WHAT_A_SITTING_AIMS_AT / 5.0;
    assert!((what_this_meal_answers(a_fifth) - 0.2).abs() < 1e-6);

    // A single handful of ordinary forage - five units at twenty apiece - is
    // a hundred energy against a sitting of four hundred and eighty.
    let one_handful = 5.0 * 20.0;
    let answered = what_this_meal_answers(one_handful);
    assert!(
        answered < 0.25,
        "one handful answered {answered} of a hunger, which is most of a meal"
    );
}

/// Nothing eaten answers nothing, and a feast answers one meal's worth.
#[test]
fn nothing_answers_nothing_and_a_feast_answers_one_meal() {
    assert_eq!(what_this_meal_answers(0.0), 0.0);
    assert_eq!(what_this_meal_answers(-5.0), 0.0);
    assert_eq!(what_this_meal_answers(WHAT_A_SITTING_AIMS_AT * 10.0), 1.0);
}
