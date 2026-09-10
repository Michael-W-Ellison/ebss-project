// src/analytics/tests/herbal_tests.rs
//! Tests for the vocabulary `Gather` answers to, and for the herbal it
//! unlocked.
//!
//! `Simulation::gathered_as` says in its own docstring that it is "the same
//! vocabulary `Gather` answers to, kept here so that the decision and the
//! executor cannot drift apart". It was one of two hand-written lists saying
//! that, and they drifted apart three times — twice recorded in the comments
//! of the other one (grain, clay) and once found here (herbs), which cost the
//! whole of the treatment machinery: measured across twelve worlds and 5,327
//! person-samples, **not one person ever held a remedy**, because every trip
//! an ill agent made for herbs came back "Unknown resource type: herbs".
//!
//! There is one list now. These tests are what holds it to one.
//!
//! The six plants the specification names and this project has no plant for -
//! ginger, calendula, lemon balm, garlic, echinacea, turmeric - are **not**
//! here. They were written, they measured well on their own, and together
//! with the fix below they cost 40% of every settlement. See ISSUES_FOUND.md
//! #166 for the table and for why that is a finding about the disease model
//! rather than about the herbal.

use crate::analytics::Simulation;
use crate::environment::remedies::{is_a_remedy, EVERY_REMEDY, THE_MOST_A_HERBAL_CAN_DO};
use crate::world::ResourceType;

/// Every name a resource can be called round-trips to the resource.
///
/// This is the invariant the two lists were supposed to keep between them and
/// could not, because keeping an invariant between two lists is something
/// nobody does for very long.
#[test]
fn every_name_a_resource_answers_to_finds_it_again() {
    for what in ResourceType::all() {
        let Some(called) = Simulation::gathered_as(what) else {
            continue;
        };

        assert_eq!(
            Simulation::what_a_gather_asks_for(called),
            Some(what),
            "{what:?} is called {called:?} and asking for {called:?} does not \
             find it"
        );
    }
}

/// And every name the decision layer actually emits resolves to something.
///
/// The round trip above cannot catch a name the drive ladder invents that no
/// resource answers to — which is exactly what "herbs" was. These are the
/// literals the arms of `what_this_drive_offers` and
/// `generate_action_for_drive` hand to `Action::Gather`.
#[test]
fn every_errand_the_drives_send_anybody_on_is_one_that_can_be_run() {
    for errand in [
        // Rest, when somebody is ill and carrying nothing for it. This is the
        // one that was broken.
        "herbs",
        // Shelter
        "hides",
        // Construction, and Preparedness
        "wood",
        // Sustenance and Hunger
        "food",
        // Industry, which cannot name what it wants
        "generic",
        // And the rest of what the ladder and the executor ask for by name
        "water", "stone", "clay", "salt", "flax", "cotton",
        "grain", "greens", "roots", "nuts", "legumes",
    ] {
        assert!(
            Simulation::what_a_gather_asks_for(errand).is_some(),
            "a drive sends somebody out for {errand:?} and the executor \
             answers 'Unknown resource type'"
        );
    }
}

/// A patch of herbs is what an ill man is sent for, so it had better be
/// gatherable and had better be a remedy when it lands in the pack.
#[test]
fn a_handful_off_a_hedgerow_is_a_remedy() {
    assert_eq!(
        Simulation::what_a_gather_asks_for("herbs"),
        Some(ResourceType::Herbs),
        "the errand an ill man is sent on has to resolve"
    );
    assert!(
        is_a_remedy("herbs"),
        "and what he brings back has to be worth something when he gets there"
    );
}

// --- the herbal itself ------------------------------------------------------

/// Nothing in the herbal cures anything, however much of it is added.
#[test]
fn nothing_added_to_this_table_ever_cures_anybody() {
    for remedy in EVERY_REMEDY {
        assert!(
            remedy.takes_off < THE_MOST_A_HERBAL_CAN_DO,
            "{} takes off {} and the cap is {THE_MOST_A_HERBAL_CAN_DO}",
            remedy.id,
            remedy.takes_off
        );
    }

    // And the best of them, at a practised hand, against the thing it is for.
    let best = EVERY_REMEDY
        .iter()
        .map(|remedy| remedy.takes_off)
        .fold(0.0f32, f32::max);
    assert!(
        best < THE_MOST_A_HERBAL_CAN_DO,
        "the best thing in the herbal still leaves most of the week: {best}"
    );
}
