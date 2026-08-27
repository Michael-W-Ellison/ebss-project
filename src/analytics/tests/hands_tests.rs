// src/analytics/tests/hands_tests.rs
//! Tests for having two hands, and for what a load costs to carry.
//!
//! `verbs::A_PAIR_OF_HANDS` has been in the matrix since the matrix existed
//! and nothing had ever made it true. A tool in the pack was a tool in the
//! hand — an axe helped you the moment you owned one, whether or not you had
//! got it out — and "a free hand" could only be guessed at from how loaded
//! the pack was, which was a fudge and was written down as one.
//!
//! And carrying was free. A man walked as easily under sixty pounds of stone
//! as under nothing, which made a full pack pure gain and a basket a thing
//! with no cost at all.

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::verbs;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (25, 25, 0);

    // Founders come with a stone-age kit in the pack — see task 102. These
    // tests are about what is in the hands, so the pack starts empty and
    // gets exactly what each one means to put there.
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0]
        .inventory
        .recalculate_weight();

    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    let mut item = InventoryItem::new(what.to_string(), how_many);
    item.weight_per_unit = 1.0;
    item.current_durability = Some(40.0);
    item.max_durability = Some(40.0);
    let _ = simulation.population.agents[0].inventory.add_item(item);
}

// --------------------------------------------------------------------------
// Taking a thing up and putting it away
// --------------------------------------------------------------------------

/// A tool goes from the pack into a hand, and stays in the pack while it does.
#[test]
fn a_tool_taken_up_is_in_the_hand_and_still_in_the_pack() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);

    let result = simulation.execute_action(
        &Action::Equip {
            what: "handaxe".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert!(
        simulation.population.agents[0].is_in_my_hand("handaxe"),
        "the axe should be out"
    );
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("handaxe"),
        1,
        "a hand is a claim on a thing, not a second place to keep it"
    );
}

/// Nobody takes up what they have not got.
#[test]
fn nobody_takes_up_what_they_do_not_have() {
    let mut simulation = one_person();

    let result = simulation.execute_action(
        &Action::Equip {
            what: "handaxe".to_string(),
        },
        0,
    );

    assert!(!result.success, "there is no axe in that pack");
}

/// And nobody takes up the same thing twice.
#[test]
fn nobody_takes_the_same_thing_up_twice() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);

    simulation.execute_action(
        &Action::Equip {
            what: "handaxe".to_string(),
        },
        0,
    );
    let again = simulation.execute_action(
        &Action::Equip {
            what: "handaxe".to_string(),
        },
        0,
    );

    assert!(!again.success, "it is already in his hand");
}

/// Two hands hold two things and no more.
#[test]
fn two_hands_hold_two_things() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);
    give(&mut simulation, "spear", 1);
    give(&mut simulation, "fishingspear", 1);

    for what in ["handaxe", "spear"] {
        let result = simulation.execute_action(
            &Action::Equip {
                what: what.to_string(),
            },
            0,
        );
        assert!(result.success, "{what}: {:?}", result.message);
    }

    let third = simulation.execute_action(
        &Action::Equip {
            what: "fishingspear".to_string(),
        },
        0,
    );

    assert!(!third.success, "there is no third hand");
    assert_eq!(
        simulation.population.agents[0].what_is_in_my_hands().count() as u32,
        verbs::A_PAIR_OF_HANDS,
        "and the matrix has always said how many there are"
    );
}

/// Putting a thing away frees the hand again.
#[test]
fn putting_a_thing_away_frees_the_hand() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);
    give(&mut simulation, "spear", 1);

    for what in ["handaxe", "spear"] {
        simulation.execute_action(
            &Action::Equip {
                what: what.to_string(),
            },
            0,
        );
    }

    assert!(
        !simulation.population.agents[0].a_hand_to_spare(),
        "both hands are full"
    );

    let result = simulation.execute_action(
        &Action::Unequip {
            what: "spear".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert!(
        simulation.population.agents[0].a_hand_to_spare(),
        "and now one is free"
    );
}

/// Nobody puts away what they are not holding.
#[test]
fn nobody_puts_away_what_they_are_not_holding() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);

    let result = simulation.execute_action(
        &Action::Unequip {
            what: "handaxe".to_string(),
        },
        0,
    );

    assert!(!result.success, "it was never out");
}

// --------------------------------------------------------------------------
// Why anybody would bother
// --------------------------------------------------------------------------

/// The whole point: the same axe does more work when it is out.
#[test]
fn an_axe_in_the_hand_is_worth_more_than_the_same_axe_in_the_bag() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);

    let in_the_bag =
        simulation.population.agents[0].how_much_my_tools_help(SkillType::Woodcutting);

    simulation.execute_action(
        &Action::Equip {
            what: "handaxe".to_string(),
        },
        0,
    );

    let in_the_hand =
        simulation.population.agents[0].how_much_my_tools_help(SkillType::Woodcutting);

    assert!(
        in_the_hand > in_the_bag,
        "an axe you have got out should beat one you have to dig for: \
         {in_the_hand} against {in_the_bag}"
    );
    assert!(
        in_the_bag > 1.0,
        "and one in the bag should still beat no axe at all"
    );
}

/// And the moment that matters is just before the work, not any idle turn.
///
/// The first cut put reaching for a tool at the bottom of the Utility chain,
/// where it fired half a time in a world of ten thousand ticks: there is
/// always some material wanting fetching, so nothing ever reached it.
#[test]
fn a_job_whose_tool_is_in_the_bag_becomes_getting_the_tool_out() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);

    let wanted = Action::Work {
        verb: "smash".to_string(),
        to: "flintcore".to_string(),
    };
    let instead = simulation.get_the_tool_out_for(wanted, 0);

    assert!(
        matches!(&instead, Action::Equip { what } if what == "handaxe"),
        "the axe should come out first: {instead:?}"
    );
}

/// And having come out, the work goes ahead.
#[test]
fn once_the_tool_is_out_the_work_goes_ahead() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);

    simulation.execute_action(
        &Action::Equip {
            what: "handaxe".to_string(),
        },
        0,
    );

    let wanted = Action::Work {
        verb: "smash".to_string(),
        to: "flintcore".to_string(),
    };
    let instead = simulation.get_the_tool_out_for(wanted.clone(), 0);

    assert_eq!(
        instead, wanted,
        "nobody reaches twice for what is already in their hand"
    );
}

/// A hand does not go on holding a spear that has been given away.
#[test]
fn a_hand_lets_go_of_what_the_owner_no_longer_has() {
    let mut simulation = one_person();
    give(&mut simulation, "spear", 1);

    simulation.execute_action(
        &Action::Equip {
            what: "spear".to_string(),
        },
        0,
    );
    assert!(simulation.population.agents[0].is_in_my_hand("spear"));

    simulation.population.agents[0]
        .inventory
        .remove_item("spear", 1);
    simulation.population.agents[0].let_go_of_what_i_no_longer_have();

    assert!(
        !simulation.population.agents[0].is_in_my_hand("spear"),
        "you cannot hold a spear you have not got"
    );
}

// --------------------------------------------------------------------------
// Both hands full, and a job that wants one free
// --------------------------------------------------------------------------

/// A person with both hands full who needs one puts the lesser thing away
/// rather than standing there defeated.
#[test]
fn a_full_pair_of_hands_puts_something_down_to_get_on() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);
    give(&mut simulation, "spear", 1);

    for what in ["handaxe", "spear"] {
        simulation.execute_action(
            &Action::Equip {
                what: what.to_string(),
            },
            0,
        );
    }

    let wanted = Action::MakeClothing {
        garment: "hide_tunic".to_string(),
    };
    let instead = simulation.free_a_hand_for(wanted.clone(), 0);

    assert!(
        matches!(instead, Action::Unequip { .. }),
        "stitching wants a hand free, so something goes away first: {instead:?}"
    );
}

/// But a job that wants nothing in particular is left alone.
#[test]
fn a_job_that_wants_no_hand_is_left_alone() {
    let mut simulation = one_person();
    give(&mut simulation, "handaxe", 1);
    give(&mut simulation, "spear", 1);

    for what in ["handaxe", "spear"] {
        simulation.execute_action(
            &Action::Equip {
                what: what.to_string(),
            },
            0,
        );
    }

    let wanted = Action::Sleep { duration: 4 };
    let instead = simulation.free_a_hand_for(wanted.clone(), 0);

    assert!(
        matches!(instead, Action::Sleep { .. }),
        "sleeping does not want a hand: {instead:?}"
    );
}

// --------------------------------------------------------------------------
// What a load costs
// --------------------------------------------------------------------------

/// An empty-handed person pays nothing extra for a step.
#[test]
fn walking_empty_handed_costs_what_it_always_did() {
    let simulation = one_person();

    assert_eq!(
        Simulation::what_this_load_costs(&simulation.population.agents[0]),
        1.0,
        "an empty pack is not a burden"
    );
}

/// And nor does a light one.
#[test]
fn a_light_pack_is_not_felt() {
    let mut simulation = one_person();
    let capacity = simulation.population.agents[0]
        .inventory
        .effective_max_weight();
    give(&mut simulation, "food", (capacity * 0.2) as u32);

    assert_eq!(
        Simulation::what_this_load_costs(&simulation.population.agents[0]),
        1.0,
        "a day's food and a spear is not a load"
    );
}

/// A heavy one is.
#[test]
fn a_heavy_pack_costs_more_every_step() {
    let mut simulation = one_person();
    let capacity = simulation.population.agents[0]
        .inventory
        .effective_max_weight();
    give(&mut simulation, "stone", (capacity * 0.95) as u32);

    let loaded = Simulation::what_this_load_costs(&simulation.population.agents[0]);

    assert!(
        loaded > 1.4,
        "sixty pounds of stone should tell on a walk, and cost {loaded}"
    );
}

/// And what a load costs rises with the load rather than jumping at a line.
#[test]
fn what_a_load_costs_rises_with_the_load() {
    let mut simulation = one_person();
    let capacity = simulation.population.agents[0]
        .inventory
        .effective_max_weight();

    give(&mut simulation, "stone", (capacity * 0.6) as u32);
    let middling = Simulation::what_this_load_costs(&simulation.population.agents[0]);

    give(&mut simulation, "wood", (capacity * 0.3) as u32);
    let heavy = Simulation::what_this_load_costs(&simulation.population.agents[0]);

    assert!(
        heavy > middling && middling > 1.0,
        "it should climb rather than jump: {middling} then {heavy}"
    );
}

// --------------------------------------------------------------------------
// The matrix
// --------------------------------------------------------------------------

/// The equipment family is no longer a declaration.
#[test]
fn the_equipment_family_does_something_now() {
    for called in ["equip", "unequip", "use", "hold", "carry"] {
        let one = verbs::what_that_verb_is(called).expect("in the matrix");
        assert!(one.is_live(), "{called} should be doing something now");
    }
}

/// Taking a thing up is a decision; holding it and wearing it out are not.
#[test]
fn only_two_of_them_are_decisions() {
    for called in ["equip", "unequip"] {
        let one = verbs::what_that_verb_is(called).expect("in the matrix");
        assert!(one.is_chosen(), "{called} is something somebody decides");
    }

    for called in ["use", "hold", "carry"] {
        let one = verbs::what_that_verb_is(called).expect("in the matrix");
        assert!(
            !one.is_chosen(),
            "{called} is what happens, not what anybody chooses"
        );
    }
}
