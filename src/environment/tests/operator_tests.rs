// src/environment/tests/operator_tests.rs
//! Verbs as operators: what they want, what they use up, what they change,
//! what they cost and what they risk.
//!
//! "Define each action with: preconditions, inputs, effects, costs, risks,
//! skill requirements."
//!
//! Three of those six were already in the matrix under other names - `targets`
//! and `wants` are the preconditions, `changes` are the effects - and three
//! were nowhere. What is asserted here is mostly that the three new ones are
//! *used* rather than merely declared, and that the one distinction easy to
//! get wrong stays right: a tool is wanted and comes back out of the job, a
//! material is an input and does not.

use super::*;
use crate::environment::tags::Capability;

// --------------------------------------------------------------------------
// The specification's worked operator
// --------------------------------------------------------------------------

/// Filling a container, with every line of the specification's example having
/// somewhere to go.
#[test]
fn filling_a_container_is_declared_the_way_the_specification_writes_it() {
    let fill = what_that_verb_is("fill").expect("there is a verb for it");

    // Preconditions: at a water source, with something to put it in.
    assert_eq!(fill.targets, Targets::Water);
    assert_eq!(
        fill.wants,
        Wants::ACapability(Capability::WaterContainer),
        "the half that could not be said before: not a waterskin by name, but \
         anything at all that holds water"
    );

    // Effects: what is held changes, and the source is drawn down.
    assert!(fill.changes.contains(&Changes::WhatIsHeld));
    assert!(fill.changes.contains(&Changes::TheGround));

    // Costs: a turn, and some effort in it.
    assert!(fill.costs.time > 0.0 && fill.costs.effort > 0.0);

    // Risks: "contamination if source quality poor".
    assert!(fill.risks.contains(&Risk::Contamination));

    // And the honest part: nothing performs it.
    assert!(
        !fill.is_live(),
        "if something now fills a container this assertion is the thing to \
         delete, and the gap is closed"
    );
}

/// Asking for a vessel by name was a want nothing could ever meet.
///
/// There is no waterskin anywhere in this world's recipe chain - the vessels
/// it can make are a bowl, a fired pot, stoneware and a leather bag - so
/// `ThisInHand("waterskin")` was unsatisfiable by construction. Had the verb
/// ever been performed it would have been refused every single time.
#[test]
fn no_verb_wants_a_thing_this_world_cannot_make() {
    use crate::environment::making::EVERY_STEP;

    for verb in EVERY_VERB {
        let Wants::ThisInHand(what) = verb.wants else {
            continue;
        };

        let can_be_made = EVERY_STEP.iter().any(|step| step.makes == what);
        let comes_out_of_the_ground = crate::world::ResourceType::called(what).is_some();

        assert!(
            can_be_made || comes_out_of_the_ground,
            "the verb '{}' wants a {what} in hand, and nothing in this world \
             makes one or digs one up",
            verb.called
        );
    }
}

// --------------------------------------------------------------------------
// Wants against inputs
// --------------------------------------------------------------------------

/// A tool is wanted and survives; a material is an input and does not.
///
/// Worth its own test because conflating the two is how a model ends up eating
/// its own tools: every verb that wants a tool would consume one, and a
/// settlement would burn a knife per hide.
#[test]
fn what_a_verb_wants_is_not_what_it_uses_up() {
    let cut = what_that_verb_is("cut").expect("there is a verb for cutting");

    assert!(
        matches!(cut.wants, Wants::AToolFor(_)),
        "cutting wants something to cut with"
    );
    assert!(
        cut.inputs.is_empty(),
        "and does not eat it: cutting uses up what is being cut, which is the \
         target, not the knife"
    );

    let sew = what_that_verb_is("sew").expect("there is a verb for sewing");
    assert!(
        !sew.inputs.is_empty(),
        "sewing consumes hide and thread, and that is a different kind of fact \
         from what it wants in the hand"
    );
}

// --------------------------------------------------------------------------
// Costs, risks and skill
// --------------------------------------------------------------------------

/// A verb that has been priced says something about all three.
#[test]
fn a_priced_verb_is_priced_in_every_currency() {
    for verb in EVERY_VERB {
        assert!(
            verb.costs.time > 0.0,
            "'{}' takes no time at all, which nothing does",
            verb.called
        );
        assert!(
            verb.costs.effort >= 0.0,
            "'{}' costs negative effort",
            verb.called
        );
    }
}

/// Hard, dangerous, skilled work is declared as such.
#[test]
fn the_dangerous_verbs_say_they_are_dangerous() {
    let hunt = what_that_verb_is("hunt").expect("there is hunting");
    assert!(hunt.risks.contains(&Risk::TheThingFightsBack));
    assert!(hunt.risks.contains(&Risk::Injury));
    assert_eq!(hunt.needs_skill, Some(SkillType::Hunting));
    assert!(hunt.costs.effort > Costs::A_MOMENT.effort);

    // And eating a thing you have never eaten can make you ill, which is the
    // whole of the curiosity gamble.
    let taste = what_that_verb_is("taste").expect("there is tasting");
    assert!(taste.risks.contains(&Risk::Illness));

    // Walking is none of those things.
    let moving = what_that_verb_is("move to").expect("there is moving");
    assert!(moving.risks.is_empty());
    assert_eq!(moving.needs_skill, None);
}

/// What an action risks is every risk of every verb it performs, once each.
#[test]
fn an_action_carries_the_risks_of_the_verbs_it_performs() {
    let hunting = what_this_action_risks("hunt");

    assert!(!hunting.is_empty(), "a hunt risks something");

    let mut once_each = hunting.clone();
    once_each.dedup();
    assert_eq!(once_each, hunting, "a risk is listed twice");

    // A hunt is a hunting and a butchering, and both wear the tool - which is
    // one fact about the action, not two.
    assert_eq!(
        hunting.iter().filter(|risk| **risk == Risk::WearTheTool).count(),
        1
    );
}

/// An action's cost is the cost of the verbs it always performs, and not of
/// the alternatives.
///
/// Charging `Craft` for heating *and* lashing *and* attaching would price one
/// craft at the cost of three. The `always` flag tells them apart here exactly
/// as it does for `wants`.
#[test]
fn alternatives_are_not_added_up() {
    let crafting = what_this_action_costs("craft");
    let one_verb_of_it = Costs::A_TURN_OF_WORK;

    assert!(
        crafting.time <= one_verb_of_it.time * 2.0,
        "a craft is priced at {crafting:?}, which is several crafts"
    );
}

/// How much of the matrix has been thought about as operators, and how much
/// has only been named.
///
/// The count is deliberately not asserted downwards. A verb still on the quiet
/// defaults is not a bug - most of the sixty-eight genuinely are a moment's
/// work with nothing at stake - but the ones that are *not* should be visible,
/// and this is what makes them countable. Compare
/// `everything_still_to_build`.
#[test]
fn what_has_been_priced_and_what_has_only_been_named_are_both_countable() {
    let priced = EVERY_VERB.len() - everything_still_to_price().count();

    assert!(
        priced >= 15,
        "only {priced} of {} verbs have been thought about as operators",
        EVERY_VERB.len()
    );

    // And every verb that carries a risk or a skill has been priced above a
    // moment's work, because nothing that can hurt you is free.
    for verb in EVERY_VERB {
        if !verb.risks.is_empty() && verb.needs_skill.is_some() {
            assert!(
                verb.costs.effort > Costs::A_MOMENT.effort,
                "'{}' is skilled and dangerous and costs a moment",
                verb.called
            );
        }
    }
}

// --------------------------------------------------------------------------
// Wanting by capability
// --------------------------------------------------------------------------

/// A capability want is met by anything that answers it.
#[test]
fn a_capability_want_is_met_by_whatever_answers_it() {
    let wants = Wants::ACapability(Capability::WaterContainer);
    let no_tool = |_: SkillType| false;

    let with_a_pot = |what: &str| if what == "claypot" { 1 } else { 0 };
    assert!(
        wants.satisfied_by(&with_a_pot, &no_tool, true),
        "a fired pot holds water"
    );

    let with_a_bag = |what: &str| if what == "leatherbag" { 1 } else { 0 };
    assert!(
        wants.satisfied_by(&with_a_bag, &no_tool, true),
        "and so does a leather bag - which asking by name would have missed"
    );

    let with_a_spear = |what: &str| if what == "spear" { 1 } else { 0 };
    assert!(!wants.satisfied_by(&with_a_spear, &no_tool, true));
}
