// src/analytics/tests/verb_tests.rs
//! Tests for the action verb matrix.
//!
//! "Every action must be defined by what it targets, what it requires
//! (tool/free-hand), and what state-change it triggers."
//!
//! Three things have to be true of a matrix for it to be worth having. It has
//! to be complete — every verb in the twelve families declared, and every
//! action in the simulation named by one. It has to be honest — a verb nothing
//! performs has to say so, because a table that quietly implied sixty-eight
//! working verbs would be worse than no table at all. And it has to be
//! load-bearing: what a verb says it wants in the hand has to be what the
//! executor refuses to proceed without, or the declaration is decoration.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::verbs::{
    self, Changes, Family, Targets, Wants, EVERY_VERB,
};
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn a_person() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(World::new(WorldConfig::default()), population)
}

fn empty_the_pack(simulation: &mut Simulation) {
    let everything: Vec<(String, u32)> = simulation.population.agents[0]
        .inventory
        .get_all_items()
        .values()
        .map(|item| (item.item_id.clone(), item.quantity))
        .collect();

    for (what, how_many) in everything {
        for _ in 0..how_many {
            simulation.population.agents[0]
                .inventory
                .remove_item(&what, 1);
        }
    }
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            1.0,
        ));
}

// --------------------------------------------------------------------------
// The matrix is complete and well formed
// --------------------------------------------------------------------------

/// All twelve families are represented.
#[test]
fn every_family_has_verbs_in_it() {
    let families = [
        Family::Locomotion,
        Family::Manipulation,
        Family::Disruption,
        Family::Thermal,
        Family::Fluid,
        Family::Assembly,
        Family::Subterranean,
        Family::Survival,
        Family::Combat,
        Family::Exchange,
        Family::Equipment,
        Family::Sensory,
    ];

    for family in families {
        let how_many = EVERY_VERB
            .iter()
            .filter(|verb| verb.family == family)
            .count();

        assert!(
            how_many >= 3,
            "{family:?} has only {how_many} verbs in it"
        );
    }
}

/// Every verb is named once, and named something.
#[test]
fn no_verb_is_declared_twice_or_nameless() {
    let mut seen = std::collections::HashSet::new();

    for one in EVERY_VERB {
        assert!(!one.called.is_empty(), "a verb with no name");
        assert!(
            seen.insert(one.called),
            "{} is in the matrix twice",
            one.called
        );
    }

    assert!(
        EVERY_VERB.len() >= 60,
        "the matrix should carry the whole list, not {}",
        EVERY_VERB.len()
    );
}

/// Every verb declares all three of the things a verb has to declare: what it
/// targets, what it wants, and what it changes.
#[test]
fn every_verb_says_what_it_targets_wants_and_changes() {
    for one in EVERY_VERB {
        assert!(
            !one.changes.is_empty(),
            "{} does not say what it changes",
            one.called
        );

        // Changes::Nothing is a real answer, and it has to be the only one
        if one.changes.contains(&Changes::Nothing) {
            assert_eq!(
                one.changes.len(),
                1,
                "{} both changes nothing and changes something",
                one.called
            );
        }

        // A verb that acts on nothing outside the actor cannot want a
        // particular thing that is somewhere else
        if one.targets == Targets::Nobody {
            assert!(
                !matches!(one.wants, Wants::AFreeHand),
                "{} targets nobody and wants a hand free for it",
                one.called
            );
        }
    }
}

/// Anything named as performing a verb is an action that exists.
#[test]
fn every_verb_is_performed_by_something_real() {
    // What the actions are actually called, as `what_was_tried` names them
    let real = [
        "move", "gather", "eat", "sleep", "build", "craft", "store", "retrieve",
        "explore", "socialize", "attack", "hunt", "fight", "tame", "mate",
        "mount", "dismount", "seekshelter", "lightfire", "cook", "makeclothing",
        "wearclothing", "tillsoil", "tendfield", "taste", "takecutting",
        "plantcutting", "spreadmuck", "fish", "wait", "shareinformation",
        "collectanimalproduct", "harvestplant",
    ];

    for one in EVERY_VERB {
        if let Some(named) = one.done_by {
            assert!(
                real.contains(&named),
                "{} claims to be done by {named}, which is not an action",
                one.called
            );
        }
    }
}

/// And the matrix is honest about what it has not built.
#[test]
fn the_matrix_admits_what_nothing_does_yet() {
    let live = verbs::everything_anybody_can_do().count();
    let waiting = verbs::everything_still_to_build().count();

    assert_eq!(live + waiting, EVERY_VERB.len());

    assert!(
        live >= 15,
        "a good part of the matrix should be doing something: {live}"
    );
    assert!(
        waiting > 0,
        "and the matrix should say so where nothing does yet"
    );

    // The families that are all declaration and no mechanism yet are worth
    // being able to name
    let untouched: Vec<&str> = verbs::everything_still_to_build()
        .filter(|verb| verb.family == Family::Fluid)
        .map(|verb| verb.called)
        .collect();

    assert!(
        !untouched.is_empty(),
        "the fluid family is not built; the matrix should not pretend it is"
    );
}

// --------------------------------------------------------------------------
// What a verb wants in the hand
// --------------------------------------------------------------------------

/// Bare hands are always enough for a verb that wants nothing.
#[test]
fn bare_hands_satisfy_a_verb_that_wants_nothing() {
    let nothing_held = |_: &str| 0;
    let no_tools = |_| false;

    assert!(Wants::BareHands.satisfied_by(&nothing_held, &no_tools, false));
    assert!(!Wants::AFreeHand.satisfied_by(&nothing_held, &no_tools, false));
    assert!(Wants::AFreeHand.satisfied_by(&nothing_held, &no_tools, true));
}

/// A verb that wants a named thing wants that thing.
#[test]
fn a_named_requirement_wants_that_thing() {
    let holding_a_spear = |what: &str| u32::from(what == "spear");
    let no_tools = |_| false;

    assert!(Wants::ThisInHand("spear").satisfied_by(&holding_a_spear, &no_tools, true));
    assert!(!Wants::ThisInHand("handaxe").satisfied_by(&holding_a_spear, &no_tools, true));
}

/// A verb that wants a tool for a trade takes any tool that helps with it.
#[test]
fn a_trade_requirement_takes_any_tool_for_that_trade() {
    use crate::agents::SkillType;

    let nothing_held = |_: &str| 0;
    let a_woodsman = |trade| trade == SkillType::Woodcutting;

    assert!(Wants::AToolFor(SkillType::Woodcutting).satisfied_by(
        &nothing_held,
        &a_woodsman,
        true
    ));
    assert!(!Wants::AToolFor(SkillType::Mining).satisfied_by(
        &nothing_held,
        &a_woodsman,
        true
    ));
}

/// A hand is free until the arms are full.
///
/// Owning tools does not take your hands away — a pack is not a pair of hands,
/// and that mistake cost a settlement its coats once already. Being loaded to
/// the limit of what you can carry does.
#[test]
fn a_hand_is_free_until_the_arms_are_full() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    assert!(
        simulation.population.agents[0].a_hand_to_spare(),
        "empty-handed, both hands are free"
    );

    give(&mut simulation, "handaxe", 1);
    give(&mut simulation, "spear", 1);
    assert!(
        simulation.population.agents[0].a_hand_to_spare(),
        "a man who owns an axe and a spear still has hands"
    );

    // Loaded to the limit, and there is nowhere to put anything
    let room = simulation.population.agents[0]
        .inventory
        .weight_capacity_remaining();
    give(&mut simulation, "stone", (room / 1.0).ceil() as u32);

    assert!(
        !simulation.population.agents[0].a_hand_to_spare(),
        "arms full of rock is no hands: {:.1} spare",
        simulation.population.agents[0].inventory.weight_capacity_remaining()
    );
}

// --------------------------------------------------------------------------
// The matrix is load-bearing
// --------------------------------------------------------------------------

/// The requirements the matrix declares are the ones the executor enforces.
#[test]
fn what_the_matrix_demands_is_what_the_executor_refuses_without() {
    // Lighting a fire wants wood: that is declared in the matrix and nowhere
    // else, and this is the same statement read back out of it
    assert_eq!(
        verbs::what_this_action_cannot_do_without("lightfire"),
        vec![Wants::ThisInHand("wood")],
    );

    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    let empty_handed = simulation.execute_action(&Action::LightFire, 0);
    assert!(
        !empty_handed.success,
        "no wood, no fire: {:?}",
        empty_handed.message
    );
    assert!(
        empty_handed
            .message
            .as_deref()
            .is_some_and(|said| said.contains("wood")),
        "and it should say what is missing: {:?}",
        empty_handed.message
    );
}

/// A hunt wants a spear, and says so.
#[test]
fn a_hunt_wants_a_spear() {
    assert_eq!(
        verbs::what_this_action_cannot_do_without("hunt"),
        vec![Wants::ThisInHand("spear")],
        "the spec's 'hunting = spear + animal', declared where it can be enforced"
    );

    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    let barehanded = simulation.execute_action(
        &Action::Hunt {
            animal_id: uuid::Uuid::nil(),
            weapon: None,
        },
        0,
    );
    assert!(!barehanded.success, "nobody runs down a deer by hand");

    // With a spear it gets past the matrix and fails on its own terms - there
    // is no such animal - rather than for want of a weapon
    give(&mut simulation, "spear", 1);
    let armed = simulation.execute_action(
        &Action::Hunt {
            animal_id: uuid::Uuid::nil(),
            weapon: None,
        },
        0,
    );
    assert!(
        !armed
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("spear in hand"),
        "a man with a spear should be refused for some other reason: {:?}",
        armed.message
    );
}

/// An action the matrix asks nothing of goes through.
#[test]
fn an_action_the_matrix_asks_nothing_of_is_not_stopped_by_it() {
    assert!(verbs::what_this_action_cannot_do_without("eat").is_empty());
    assert!(verbs::what_this_action_cannot_do_without("move").is_empty());

    // Craft performs several verbs, any one of which the step in hand might
    // call for, so the matrix holds it to none of them and the step decides
    assert!(
        verbs::what_this_action_cannot_do_without("craft").is_empty(),
        "which of heating, lashing and attaching applies is the step's business"
    );
}

/// Looking a verb up by name gets that verb.
#[test]
fn a_verb_can_be_looked_up_by_name() {
    let cut = verbs::what_that_verb_is("cut").expect("cutting is in the matrix");
    assert_eq!(cut.family, Family::Disruption);
    assert!(
        cut.wants_something_in_hand(),
        "a man with no edge cannot cut"
    );

    assert!(verbs::what_that_verb_is("teleport").is_none());
}

/// And an action can be asked what verbs it carries out.
#[test]
fn an_action_can_be_asked_what_it_does() {
    let hunting = verbs::what_that_action_does("hunt");

    assert!(
        hunting.iter().any(|verb| verb.called == "hunt"),
        "a hunt is a hunting"
    );
    assert!(
        hunting.iter().any(|verb| verb.called == "butcher"),
        "and, when it lands, a butchering"
    );
    assert!(
        hunting
            .iter()
            .find(|verb| verb.called == "butcher")
            .is_some_and(|verb| !verb.always),
        "though only when it lands, which is why it is not demanded up front"
    );
}
