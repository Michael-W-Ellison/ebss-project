// src/analytics/tests/combat_verb_tests.rs
//! Tests for the two combat verbs that now do something.
//!
//! A throw is not a use of a spear, it is a parting with one. Half the throws
//! that miss put the shaft on the ground somewhere out past where the hunter
//! was standing, and it is a spear again as soon as somebody walks over and
//! picks it up. That is what makes a missed throw cost more than the walking.
//!
//! Defending is the other kind of verb: nobody decides to do it. It is what
//! happens when something comes at you and there is a shaft in your hand, and
//! it is why carrying a spear is worth something to a man who never hunts.
//! The matrix carries both kinds now — `done_by` for what somebody chooses and
//! `happens_when` for what happens to them.

use crate::agents::{AgentConfig, InventoryItem, Population, Quality, SkillType};
use crate::analytics::Simulation;
use crate::environment::verbs::{self, Wants};
use crate::environment::{making, Action};
use crate::world::{World, WorldConfig};

fn a_hunter_at(where_it_is: (i32, i32)) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (where_it_is.0, where_it_is.1, 0);
    simulation
}

/// A founder arrives carrying an axe and a knife, which is a thing to get
/// between yourself and a wolf. These tests are about what that is worth, so
/// they start from empty hands.
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

fn arm(simulation: &mut Simulation, with: &str, how_many: u32) {
    for _ in 0..how_many {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_durability(
                with.to_string(),
                1,
                40.0,
                Quality::Basic,
            ));
    }
}

// --------------------------------------------------------------------------
// The matrix carries both kinds of verb
// --------------------------------------------------------------------------

/// Some verbs are chosen and some happen to you, and the table says which.
#[test]
fn the_matrix_tells_a_choice_from_a_thing_that_happens() {
    let throwing = verbs::what_that_verb_is("throw").expect("throwing is in the matrix");
    let defending = verbs::what_that_verb_is("defend with").expect("so is defending");

    assert!(throwing.is_chosen(), "a man decides to throw a spear");
    assert!(throwing.is_live());

    assert!(
        !defending.is_chosen(),
        "nobody decides to get a spear between himself and a wolf"
    );
    assert!(
        defending.is_live(),
        "but it happens, so the matrix should not call it idle"
    );
    assert!(
        defending.happens_when.is_some(),
        "and should say what occasions it"
    );
}

/// Defending wants something in the hand, so a man with nothing does not do it.
#[test]
fn defending_wants_something_to_defend_with() {
    let defending = verbs::what_that_verb_is("defend with").expect("in the matrix");
    assert_eq!(defending.wants, Wants::AToolFor(SkillType::MeleeCombat));

    // And there is something a stone-age people has that answers to it
    let to_hand: Vec<&str> = making::what_helps_with(SkillType::MeleeCombat)
        .map(|tool| tool.called)
        .collect();

    assert!(
        to_hand.contains(&"spear"),
        "a braced spear is a fence: {to_hand:?}"
    );
    assert!(
        to_hand.contains(&"handaxe"),
        "and so, at a pinch, is an axe: {to_hand:?}"
    );
}

/// A spear is worth more for keeping something off than an axe is.
#[test]
fn a_spear_keeps_more_off_than_an_axe() {
    let by = |called: &str| {
        making::what_helps_with(SkillType::MeleeCombat)
            .find(|tool| tool.called == called)
            .map(|tool| tool.how_much_better)
            .unwrap_or(1.0)
    };

    assert!(by("spear") > by("handaxe"), "reach is the whole of it");
    assert!(
        by("metalspear") > by("spear"),
        "and a metal head is worth more again"
    );
}

// --------------------------------------------------------------------------
// What a shaft in the hand is worth when something comes at you
// --------------------------------------------------------------------------

/// A man with a spear takes less of a blow than a man with empty hands.
#[test]
fn something_in_the_hand_turns_a_blow() {
    let mut simulation = a_hunter_at((30, 30));
    empty_the_pack(&mut simulation);

    let coming = 40.0;

    let bare = simulation.population.agents[0].what_a_blow_costs_me(coming);
    assert_eq!(
        bare, coming,
        "empty-handed, the whole of it gets through"
    );

    arm(&mut simulation, "handaxe", 1);
    let with_an_axe = simulation.population.agents[0].what_a_blow_costs_me(coming);

    let mut with_a_spear = a_hunter_at((30, 30));
    empty_the_pack(&mut with_a_spear);
    arm(&mut with_a_spear, "spear", 1);
    let with_a_spear = with_a_spear.population.agents[0].what_a_blow_costs_me(coming);

    assert!(
        with_an_axe < bare,
        "an axe turns some of it: {with_an_axe:.0} of {coming:.0}"
    );
    assert!(
        with_a_spear < with_an_axe,
        "and a spear turns more, because reach is the whole of it: \
         {with_a_spear:.0} against {with_an_axe:.0}"
    );
    assert!(
        with_a_spear > 0.0,
        "though nothing turns all of it"
    );
}

/// And a worn shaft turns less than a fresh one.
#[test]
fn a_worn_shaft_turns_less_than_a_fresh_one() {
    let coming = 40.0;

    let mut fresh = a_hunter_at((30, 30));
    empty_the_pack(&mut fresh);
    arm(&mut fresh, "spear", 1);
    let fresh_takes = fresh.population.agents[0].what_a_blow_costs_me(coming);

    let mut worn = a_hunter_at((30, 30));
    empty_the_pack(&mut worn);
    arm(&mut worn, "spear", 1);
    if let Some(spear) = worn.population.agents[0].inventory.get_item_mut("spear") {
        spear.current_durability = Some(1.0);
    }
    let worn_takes = worn.population.agents[0].what_a_blow_costs_me(coming);

    assert!(
        worn_takes > fresh_takes,
        "a shaft that is nearly firewood is nearly no help: \
         {worn_takes:.0} against {fresh_takes:.0}"
    );
}

// --------------------------------------------------------------------------
// A throw parts you from the spear
// --------------------------------------------------------------------------

/// A throw that misses puts the shaft on the ground.
#[test]
fn a_missed_throw_leaves_the_spear_on_the_ground() {
    let mut lost = 0;

    for _ in 0..40 {
        let mut simulation = a_hunter_at((30, 30));
        arm(&mut simulation, "spear", 1);

        // A hopeless hunter, so most of these are misses
        simulation.population.agents[0]
            .skills
            .set_skill_level(SkillType::Hunting, -10);

        let rabbit = simulation
            .world
            .spawn_animal("rabbit".to_string(), (30, 30))
            .expect("a rabbit should spawn");

        simulation.execute_action(
            &Action::Hunt {
                animal_id: rabbit,
                weapon: None,
            },
            0,
        );

        if simulation.population.agents[0].how_many_i_have("spear") == 0 {
            lost += 1;
            assert!(
                !simulation.world.dropped.is_empty(),
                "if it is not in his hand it is on the ground somewhere"
            );
        }
    }

    assert!(
        lost > 0,
        "a stone-age hunter should be losing spears in the bracken"
    );
    assert!(
        lost < 40,
        "and not every single throw: {lost} of 40"
    );
}

/// And the spear that landed is the spear that was thrown.
#[test]
fn the_spear_on_the_ground_is_the_spear_that_was_thrown() {
    for _ in 0..60 {
        let mut simulation = a_hunter_at((30, 30));
        arm(&mut simulation, "spear", 1);

        // Worn most of the way through, so it is recognisable
        if let Some(spear) = simulation.population.agents[0]
            .inventory
            .get_item_mut("spear")
        {
            spear.current_durability = Some(11.0);
        }

        simulation.population.agents[0]
            .skills
            .set_skill_level(SkillType::Hunting, -10);

        let rabbit = simulation
            .world
            .spawn_animal("rabbit".to_string(), (30, 30))
            .expect("a rabbit should spawn");

        simulation.execute_action(
            &Action::Hunt {
                animal_id: rabbit,
                weapon: None,
            },
            0,
        );

        if let Some(left) = simulation
            .world
            .dropped
            .iter()
            .find(|left| left.item.item_id == "spear")
        {
            assert_eq!(
                left.item.current_durability,
                Some(11.0),
                "the same worn shaft, lying where it fell"
            );
            return;
        }
    }

    panic!("sixty throws by a hopeless hunter and not one spear in the bracken");
}
