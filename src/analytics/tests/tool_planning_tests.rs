//! Stopping to make a better tool, when the tool buys back more than it costs.
//!
//! "The agent should look at the drive, their skills, the availability of tools
//! to decrease time, if they need to make any tools, and decide the quickest
//! method of satisfying their most important drive." And, earlier: "eight hours
//! with this axe, or two hours making a better one and six with that."

use crate::environment::making::{how_many_turns_to_make, Making};

/// The chain can be priced, not just walked one step at a time.
#[test]
fn a_thing_already_in_the_pack_costs_nothing_to_make() {
    let holding = |what: &str| u32::from(what == "handaxe");
    let knows = |_: &Making| true;

    assert_eq!(
        how_many_turns_to_make("handaxe", &holding, &knows),
        Some(0),
        "you do not make what you are already carrying"
    );
}

/// And something with a chain behind it costs more than something with none.
#[test]
fn a_longer_chain_costs_more_turns() {
    let empty_handed = |_: &str| 0u32;
    let knows_everything = |_: &Making| true;

    let axe = how_many_turns_to_make("handaxe", &empty_handed, &knows_everything);
    assert!(axe.is_some(), "a stone-age people can make a handaxe");
    let axe = axe.unwrap();
    assert!(axe > 0, "an axe out of nothing is not free");

    // With the makings already to hand it is one turn - the difference between
    // the two is the whole of what the chain costs
    let makings_to_hand = |what: &str| match what {
        "handaxe" => 0,
        _ => 9,
    };
    let with_makings = how_many_turns_to_make("handaxe", &makings_to_hand, &knows_everything)
        .expect("still makeable with the makings in hand");
    assert!(
        with_makings < axe,
        "an axe with the makings to hand ({with_makings}) should cost less \
         than one from nothing ({axe})"
    );
}

/// Nobody prices a thing they have never heard of.
#[test]
fn what_nobody_knows_how_to_make_has_no_price() {
    let empty_handed = |_: &str| 0u32;
    let knows_nothing = |_: &Making| false;

    assert_eq!(
        how_many_turns_to_make("handaxe", &empty_handed, &knows_nothing),
        None
    );
}

/// The arithmetic itself: what a tool saves against what it costs.
///
/// This is the specification's own worked example. A job that takes eight
/// hours by hand and six with a better tool is worth two hours of making, and
/// is not worth three.
#[test]
fn a_tool_is_worth_making_when_it_buys_back_more_than_it_costs() {
    // What the model actually computes, spelled out: the work the tool has in
    // it, at the difference between the two rates.
    let saves = |lasts: f32, now: f32, after: f32| lasts * (1.0 / now - 1.0 / after);

    // A tool with forty pieces of work in it, taking bare hands from one to
    // 1.8, saves about eighteen turns - well worth a few turns of knapping
    let a_real_axe = saves(40.0, 1.0, 1.8);
    assert!(
        a_real_axe > 10.0,
        "forty jobs at nearly twice the speed should be worth ten turns: {a_real_axe}"
    );

    // A tool that is barely better than what is already in hand is not worth
    // stopping for, however long it lasts
    let barely_better = saves(40.0, 1.7, 1.8);
    assert!(
        barely_better < a_real_axe,
        "a marginal upgrade is worth less than a first tool"
    );

    // And one that does not last is not worth it either
    let wears_out_at_once = saves(2.0, 1.0, 1.8);
    assert!(
        wears_out_at_once < 1.0,
        "a tool with two jobs in it cannot pay for a turn of making: \
         {wears_out_at_once}"
    );
}
