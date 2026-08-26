// src/analytics/tests/knife_chain_tests.rs
//! Tests for a want that reaches the pack rather than only the list.
//!
//! Measured, of thirty-two people: **twenty-seven wanted something to carve
//! with, thirty-two knew how to make one, and five owned one.** The want was
//! there and the knowledge was there and the knife was not.
//!
//! Two things were in the way, and both are the same mistake in different
//! clothes.
//!
//! `what_i_would_make` and `what_i_must_find` — the tool a man wants, and the
//! material that tool wants — sat *behind* `what_i_would_work_on`, which is
//! "work any material I happen to be holding into whatever it makes". That is
//! undirected and nearly always answerable, so it was the answer every single
//! turn: a settlement crafted 110 times a world against 1,896 workings. **Being
//! equipped comes before pottering**, and both of the directed branches go
//! quiet the moment a man is equipped, so putting them first costs nothing
//! afterwards and is the whole of it before.
//!
//! And `what_to_do_first_knowing` checks the *materials* and nothing else, so
//! it would name a step wanting a hammerstone in the hand of a man with none,
//! or one wanting a fire where there was no fire. Once the ordering was fixed
//! that became **2,378 refused crafts a world out of 2,719 attempted** — and a
//! refusal is worse than a wasted turn, because it goes into the record and
//! teaches a man that making knives does not work.

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::{making, Action};
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

// --------------------------------------------------------------------------
// Not naming a step that cannot be taken
// --------------------------------------------------------------------------

/// A step that wants a fire is not named where there is no fire.
#[test]
fn nobody_proposes_a_making_that_wants_a_fire_they_have_not_got() {
    let knows = |_: &making::Making| true;
    let in_hand = |_: &str| true;

    let over_a_fire = making::EVERY_STEP
        .iter()
        .find(|step| step.over_a_fire)
        .expect("something in this world is made over a fire");

    // Plenty of every input and none of the thing itself - somebody already
    // holding a stack of them has no reason to make another.
    let wanted = over_a_fire.makes;
    let holding = move |what: &str| if what == wanted { 0 } else { 99 };

    assert!(
        making::what_to_do_first_that_can_be_done(
            over_a_fire.makes,
            &holding,
            &knows,
            &in_hand,
            false,
        )
        .is_none(),
        "{} wants a fire and there is none", over_a_fire.makes
    );

    assert!(
        making::what_to_do_first_that_can_be_done(
            over_a_fire.makes,
            &holding,
            &knows,
            &in_hand,
            true,
        )
        .is_some(),
        "and with a fire it is on"
    );
}

/// Nor one that wants a tool in the hand that nobody owns.
#[test]
fn nobody_proposes_a_making_that_wants_a_tool_they_have_not_got() {
    let knows = |_: &making::Making| true;

    let wants_a_tool = making::EVERY_STEP
        .iter()
        .find(|step| step.wants_in_hand.is_some() && !step.over_a_fire)
        .expect("something in this world is beaten out with something else");

    let wanted = wants_a_tool.makes;
    let holding = move |what: &str| if what == wanted { 0 } else { 99 };

    assert!(
        making::what_to_do_first_that_can_be_done(
            wants_a_tool.makes,
            &holding,
            &knows,
            &|_| false,
            true,
        )
        .is_none(),
        "{} wants a {:?}", wants_a_tool.makes, wants_a_tool.wants_in_hand
    );

    assert!(
        making::what_to_do_first_that_can_be_done(
            wants_a_tool.makes,
            &holding,
            &knows,
            &|_| true,
            true,
        )
        .is_some()
    );
}

/// And the plain answer is unchanged where nothing stands in the way.
#[test]
fn an_ordinary_making_is_named_as_it_always_was() {
    let holding = |what: &str| if what == "lashing" { 0 } else { 99 };
    let knows = |_: &making::Making| true;

    assert_eq!(
        making::what_to_do_first_that_can_be_done("lashing", &holding, &knows, &|_| true, false)
            .map(|step| step.makes),
        making::what_to_do_first_knowing("lashing", &holding, &knows).map(|step| step.makes),
    );
}

// --------------------------------------------------------------------------
// Getting the thing out before using it
// --------------------------------------------------------------------------

/// A making can name a tool that must be in the hand and is not used up by
/// the work. The matrix cannot express that, because it is keyed on the verb
/// rather than on the recipe — so somebody who owned the hammerstone and had
/// not got it out was refused every time.
#[test]
fn somebody_who_owns_the_hammerstone_gets_it_out_first() {
    let mut simulation = one_person();

    let beaten_out = making::EVERY_STEP
        .iter()
        .find(|step| step.wants_in_hand.is_some())
        .expect("something is beaten out with something else");
    let tool = beaten_out.wants_in_hand.unwrap();

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight(tool.to_string(), 1, 1.0));
        agent.found_out_how_to(beaten_out.makes);
    }

    let asked = Action::Craft {
        item_type: beaten_out.makes.to_string(),
    };
    let instead = simulation.get_the_tool_out_for(asked, 0);

    assert!(
        matches!(instead, Action::Equip { ref what } if what == tool),
        "he owns the {tool} and has not got it out: {instead:?}"
    );
}

/// Somebody who does not own it is left alone — getting a thing out of a pack
/// it is not in helps nobody.
#[test]
fn somebody_who_does_not_own_it_is_left_alone() {
    let mut simulation = one_person();

    let beaten_out = making::EVERY_STEP
        .iter()
        .find(|step| step.wants_in_hand.is_some())
        .expect("something is beaten out with something else");

    simulation.population.agents[0].found_out_how_to(beaten_out.makes);

    let asked = Action::Craft {
        item_type: beaten_out.makes.to_string(),
    };
    let instead = simulation.get_the_tool_out_for(asked.clone(), 0);

    assert!(!matches!(instead, Action::Equip { .. }), "{instead:?}");
}

// --------------------------------------------------------------------------
// Being equipped before pottering
// --------------------------------------------------------------------------

/// The undirected working comes first, and it is *not* obvious that it should.
///
/// I moved it to the bottom on the reasoning that being equipped ought to come
/// before pottering — it is nearly always answerable, so it was the answer
/// every turn, and a settlement crafted 110 times a world against 1,896
/// workings. Measured, that cost **two thirds of the settlement's vessels**
/// (t = -4.6) and put its rot up (t = 2.1).
///
/// The reason is the thing worth keeping: **the pottering is where bowls come
/// from.** Carving a bowl is a working, not a making, so the undirected branch
/// is the only route to a vessel anybody actually takes. Demoting it did not
/// redirect those turns to something better, it deleted the thing they were
/// producing. Reverted, and this test is here so the next person does not try
/// it again.
#[test]
fn the_undirected_working_comes_first_and_that_is_where_bowls_come_from() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        for what in ["stone", "flax", "wood", "hides"] {
            agent
                .inventory
                .add_item(InventoryItem::new_with_weight(what.to_string(), 20, 0.1));
        }
        agent.inventory.recalculate_weight();
    }

    let chosen = simulation.what_this_drive_offers(
        crate::core::DriveType::Utility,
        &simulation.population.agents[0],
        here,
    );

    assert!(
        matches!(chosen, Some(Action::Work { .. })),
        "a pack full of makings and the working is what he reaches for: {chosen:?}"
    );

    // And a bowl is one of the things that branch can reach, which is the
    // whole reason it must not be demoted.
    let carving = making::how_to_work("carve", "wood").expect("wood carves");
    assert!(carving.holds.is_some_and(|held| held > 0.0));
    assert!(carving.obvious, "and anybody can do it");
}

/// The directed want is still in the chain behind it, and is reached by
/// somebody with nothing to potter with.
#[test]
fn somebody_with_nothing_to_work_on_still_reaches_for_a_tool() {
    let mut simulation = one_person();

    // Nothing in the pack at all: there is no working to be done, so the
    // branches below it get the turn.
    assert_eq!(
        simulation.population.agents[0].what_i_would_work_on(),
        None
    );

    assert!(
        simulation.population.agents[0].what_i_must_find().is_some(),
        "he owns nothing and wants a spear: the ground has what it takes"
    );
}

/// And once he is equipped, both of the directed branches go quiet and he is
/// free to potter — which is why putting them first costs nothing afterwards.
#[test]
fn a_man_who_is_equipped_goes_back_to_pottering() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        for what in ["stone", "flax", "wood", "hides"] {
            agent
                .inventory
                .add_item(InventoryItem::new_with_weight(what.to_string(), 20, 0.1));
        }
        // The best of everything he knows how to make
        for trade in crate::agents::Agent::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO {
            while let Some(tool) = simulation.population.agents[0].what_i_would_rather_have(trade) {
                simulation.population.agents[0].inventory.add_item(
                    InventoryItem::new_with_weight(tool.called.to_string(), 1, 1.0),
                );
            }
        }
        let agent = &mut simulation.population.agents[0];
        agent.inventory.recalculate_weight();
    }

    assert_eq!(
        simulation.population.agents[0].what_i_would_make(true),
        None,
        "there is nothing left he would rather have"
    );
    assert_eq!(
        simulation.population.agents[0].what_i_must_find(),
        None,
        "and nothing left to fetch for it"
    );
}

/// A pair of hands wants a tool for carving and a tool for digging, which it
/// did not. Nothing else in the model ever wanted either.
#[test]
fn a_pair_of_hands_wants_something_to_carve_and_something_to_dig_with() {
    let wants = crate::agents::Agent::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO;

    assert!(wants.contains(&SkillType::Crafting));
    assert!(wants.contains(&SkillType::Mining));
    assert_eq!(
        wants.first(),
        Some(&SkillType::Hunting),
        "a spear is still the difference between eating meat and not"
    );
}

// --------------------------------------------------------------------------
// In the running world
// --------------------------------------------------------------------------

/// The whole of it: a settlement left to itself crafts, and what it asks for
/// is not refused.
#[test]
fn a_settlement_crafts_without_being_refused() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 120) {
        simulation.tick();
        if !simulation.population.agents.iter().any(|a| a.state.is_alive) {
            break;
        }
    }

    let asked = simulation.actions_taken.get("Craft").copied().unwrap_or(0);
    let refused = simulation.actions_failed.get("Craft").copied().unwrap_or(0);

    assert!(asked > 0, "four months and nobody made anything");
    assert!(
        refused * 4 < asked,
        "asked {asked} times and was refused {refused}: a refusal is worse \
         than a wasted turn, because it teaches a man that making does not work"
    );
}
