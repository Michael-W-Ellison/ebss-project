// src/analytics/tests/working_tests.rs
//! Tests for working a thing down into another thing.
//!
//! The other half of what a tool is for. A making puts several things
//! together; a working takes one thing and reduces it — a core smashed into
//! flakes, a hide cut into leather, a stick scraped into shavings. The verb is
//! different and so is what it wants in the hand: you assemble with your
//! fingers and you reduce with an edge.
//!
//! What each wants is not declared here or in the working. It is declared in
//! the verb matrix and enforced there, once, before the action runs — which is
//! the whole return on having a matrix.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::making;
use crate::environment::verbs::{self, Wants};
#[allow(unused_imports)]
use crate::world::ItemType;
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

fn give_a_tool(simulation: &mut Simulation, called: &str) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_durability(
            called.to_string(),
            1,
            40.0,
            crate::agents::Quality::Basic,
        ));
}

fn work(verb: &str, to: &str) -> Action {
    Action::Work {
        verb: verb.to_string(),
        to: to.to_string(),
    }
}

// --------------------------------------------------------------------------
// The table
// --------------------------------------------------------------------------

/// Every working names a verb the matrix knows about.
#[test]
fn every_working_is_done_with_a_verb_in_the_matrix() {
    for one in making::EVERY_WORKING {
        let verb = verbs::what_that_verb_is(one.verb)
            .unwrap_or_else(|| panic!("{} is not in the matrix", one.verb));

        // Breaking a thing down, putting it together, or working it wet —
        // and, since clay, holding it in a fire until it stops being what it
        // was. That fourth case is Thermal and belongs there: nothing in this
        // list was done over a fire until firing clay existed, which is why
        // the first three were once the whole of it.
        let mut families: Vec<verbs::Family> = vec![
            verbs::Family::Disruption,
            verbs::Family::Assembly,
            verbs::Family::Fluid,
        ];
        if one.over_a_fire {
            families.push(verbs::Family::Thermal);
        }

        assert!(
            families.contains(&verb.family),
            "{} does something to a thing, so it belongs to one of {families:?} \
             and not {:?}",
            one.verb,
            verb.family
        );

        // Most of them want an edge or a hammer, and the ones done with water
        // want a vessel. Weaving is fingers and so is pressing a lump of clay
        // into a shape; a thing held in a fire wants the fire and nothing in
        // the hand at all. The matrix is where those differences are written
        // down.
        if !matches!(one.verb, "weave" | "mold" | "fire") {
            assert!(
                verb.wants_something_in_hand(),
                "{} takes something in the hand, and the matrix should say so",
                one.verb
            );
        }

        // What wants water says so in both places, and they agree
        if one.wants_water > 0.0 {
            assert_eq!(
                verb.wants,
                verbs::Wants::AVessel,
                "{} is done with water, so the matrix should want a vessel",
                one.verb
            );
        }
        assert_eq!(
            verb.done_by,
            Some(one.verb),
            "the matrix should say what performs {}",
            one.verb
        );
        assert!(one.how_much >= 1 && one.how_many >= 1);
    }
}

/// And what comes out of a working is something the world has a use for.
#[test]
fn what_a_working_makes_is_wanted_by_something() {
    // A flake makes a tip for half the stone a core does
    let from_stone = making::how_to_make("knappedtip").expect("a tip is made");
    let ways: Vec<_> = making::every_way_to_make("knappedtip").collect();
    assert!(
        ways.len() >= 2,
        "there should be more than one road to a knapped tip"
    );

    let from_flint = ways
        .iter()
        .find(|step| step.needs.iter().any(|(what, _)| *what == "flint"))
        .expect("one of them off a struck flake");

    let stone_wanted: u32 = from_stone
        .needs
        .iter()
        .filter(|(what, _)| *what == "stone")
        .map(|(_, how_many)| *how_many)
        .sum();
    let flint_wanted: u32 = from_flint
        .needs
        .iter()
        .filter(|(what, _)| *what == "flint")
        .map(|(_, how_many)| *how_many)
        .sum();

    assert!(
        flint_wanted < stone_wanted,
        "a struck flake should be nearer a tip than a raw core is"
    );
}

// --------------------------------------------------------------------------
// Doing it
// --------------------------------------------------------------------------

/// A core with a hammer in hand comes apart into flakes.
#[test]
fn a_core_with_a_hammerstone_becomes_flakes() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    give(&mut simulation, "stone", 6);
    give_a_tool(&mut simulation, "handaxe");

    let result = simulation.execute_action(&work("smash", "stone"), 0);
    assert!(result.success, "that is what a hammerstone is: {:?}", result.message);

    let agent = &simulation.population.agents[0];
    assert!(
        agent.how_many_i_have("flint") > 0,
        "there should be flakes in the pack"
    );
    assert!(
        agent.how_many_i_have("stone") < 6,
        "and less stone than there was"
    );
}

/// And with nothing in hand it does not, because the matrix says so.
#[test]
fn a_core_with_nothing_in_hand_stays_a_core() {
    assert_eq!(
        verbs::what_this_action_cannot_do_without("smash"),
        vec![Wants::AToolFor(crate::agents::SkillType::Mining)],
        "smashing wants something to smash with, declared in the matrix"
    );

    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "stone", 6);

    let result = simulation.execute_action(&work("smash", "stone"), 0);
    assert!(!result.success, "bare hands do not break a core");
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("stone"),
        6,
        "and nothing is spent trying"
    );
}

/// Cutting a hide wants an edge, and gives leather.
#[test]
fn a_hide_with_a_knife_becomes_leather() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "hides", 3);

    let barehanded = simulation.execute_action(&work("scrape", "hides"), 0);
    assert!(!barehanded.success, "nobody cuts a hide with their fingers");

    give_a_tool(&mut simulation, "stoneknife");
    let result = simulation.execute_action(&work("scrape", "hides"), 0);

    assert!(result.success, "with a knife it comes apart: {:?}", result.message);
    assert!(
        simulation.population.agents[0].how_many_i_have("leather") > 0,
        "and there is leather to make things out of"
    );
}

/// Working a thing wears out what worked it.
#[test]
fn the_edge_that_did_it_is_the_worse_for_it() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "hides", 40);
    give_a_tool(&mut simulation, "stoneknife");

    let before = simulation.population.agents[0]
        .inventory
        .get_item("stoneknife")
        .and_then(|item| item.current_durability)
        .unwrap_or(0.0);

    for _ in 0..5 {
        simulation.execute_action(&work("scrape", "hides"), 0);
    }

    let after = simulation.population.agents[0]
        .inventory
        .get_item("stoneknife")
        .and_then(|item| item.current_durability)
        .unwrap_or(0.0);

    assert!(
        after < before,
        "five hides should tell on a stone knife: {after:.1} against {before:.1}"
    );
}

/// A practised hand gets more off the same material.
#[test]
fn a_practised_hand_wastes_less_of_the_core() {
    fn came_off(level: i32) -> u32 {
        let mut got = 0;

        for _ in 0..30 {
            let mut simulation = a_person();
            empty_the_pack(&mut simulation);
            give(&mut simulation, "stone", 4);
            give_a_tool(&mut simulation, "handaxe");
            simulation.population.agents[0]
                .skills
                .get_skill_mut(crate::agents::SkillType::Mining)
                .level = level;

            simulation.execute_action(&work("smash", "stone"), 0);
            got += simulation.population.agents[0].how_many_i_have("flint");
        }

        got
    }

    let beginner = came_off(-5);
    let old_hand = came_off(30);

    assert!(
        old_hand > beginner,
        "years at it should show: {old_hand} against {beginner}"
    );
}

// --------------------------------------------------------------------------
// Knowing how
// --------------------------------------------------------------------------

/// Some workings a people brings with it, and some it does not.
#[test]
fn scraping_a_stick_is_a_thing_to_find_out() {
    let obvious: Vec<&str> = making::EVERY_WORKING
        .iter()
        .filter(|working| working.obvious)
        .map(|working| working.verb)
        .collect();
    let found_out: Vec<&str> = making::every_working_to_find_out()
        .map(|working| working.verb)
        .collect();

    assert!(!obvious.is_empty(), "a people knows how to break a core");
    assert!(
        found_out.contains(&"scrape"),
        "and does not know what shavings are for: {found_out:?}"
    );
}

/// A working nobody knows is not something an agent sets out to do, and a
/// working it has done once is.
#[test]
fn doing_it_once_is_what_makes_it_a_thing_you_do() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "wood", 6);
    give_a_tool(&mut simulation, "stoneknife");

    assert!(
        simulation.population.agents[0]
            .what_i_would_work_on()
            .is_none_or(|(verb, _)| verb != "scrape"),
        "nobody sets out to make shavings before they know what shavings are"
    );

    // Curiosity offers something to do with the stick. Which of the things
    // that can be done to a stick is this particular man's own business - see
    // `what_working_i_would_try_out`, where taking the first of the list meant
    // the order of the table decided what a whole people ever found out.
    let curious = simulation.population.agents[0].what_working_i_would_try_out(true);
    assert!(
        matches!(curious, Some((_, ref to)) if to == "wood"),
        "a man with a stick and a scraper has an idle afternoon's question: {curious:?}"
    );

    simulation.execute_action(&work("scrape", "wood"), 0);

    assert!(
        simulation.population.agents[0]
            .what_i_found_out()
            .contains("tinder"),
        "having made some, he knows what they are"
    );
    // And will make more on purpose, once the shavings he made are used up -
    // nobody scrapes another stick with a pile of them already in the pack
    let made = simulation.population.agents[0].how_many_i_have("tinder");
    for _ in 0..made {
        simulation.population.agents[0]
            .inventory
            .remove_item("tinder", 1);
    }

    // And will make more on purpose. Whether he gets round to it on any
    // particular turn is a roll - see `Lessons::NEVER_QUITE_GIVES_UP`, which
    // is what stops anybody doing the same thing for ever - so this asks
    // whether it is a thing he does at all rather than a thing he does now.
    let would = (0..60)
        .filter_map(|_| simulation.population.agents[0].what_i_would_work_on())
        .any(|(verb, to)| verb == "scrape" && to == "wood");

    assert!(would, "and will make more on purpose");
}

/// Nobody spends a life smashing cores they have no use for.
#[test]
fn nobody_works_more_than_they_have_a_use_for() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    // A few cores rather than an armful: a pack holds only so much, and a
    // full one silently refuses the flakes this test goes on to hand over
    give(&mut simulation, "stone", 6);
    give_a_tool(&mut simulation, "handaxe");

    assert_eq!(
        simulation.population.agents[0].what_i_would_work_on(),
        Some(("smash".to_string(), "stone".to_string())),
        "with a core and a hammer and no flakes, he breaks a core"
    );

    give(&mut simulation, "flint", making::A_FEW_SPARE);
    assert!(
        simulation.population.agents[0].how_many_i_have("flint") >= making::A_FEW_SPARE,
        "the flakes have to actually be in the pack for this to mean anything"
    );

    assert!(
        simulation.population.agents[0]
            .what_i_would_work_on()
            .is_none_or(|(_, to)| to != "stone"),
        "with a pile of flakes he has not used, he does not break another"
    );
}

// --------------------------------------------------------------------------
// What it buys
// --------------------------------------------------------------------------

/// Shavings under a hearth halve the timber it takes to get going.
#[test]
fn tinder_halves_the_wood_a_fire_wants() {
    fn lit_with(tinder: u32, wood: u32) -> bool {
        let mut simulation = a_person();
        empty_the_pack(&mut simulation);
        give(&mut simulation, "wood", wood);
        if tinder > 0 {
            give(&mut simulation, "tinder", tinder);
        }
        simulation.population.agents[0].state.position = (25, 25, 0);

        simulation.execute_action(&Action::LightFire, 0).success
    }

    // A fresh hearth wants ten sticks, or five with shavings under it
    assert!(
        !lit_with(0, 6),
        "six sticks and no shavings will not start a fire"
    );
    assert!(
        lit_with(2, 6),
        "six sticks and a handful of shavings will"
    );
}

// --------------------------------------------------------------------------
// What the later workings are for
// --------------------------------------------------------------------------

/// Grain between two stones is worth more than grain.
#[test]
fn ground_grain_feeds_better_than_whole_grain() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "grain", 6);
    give_a_tool(&mut simulation, "handaxe");

    // A discovery, so it has to be found before it can be done on purpose
    simulation.population.agents[0].found_out_how_to("flour");

    let result = simulation.execute_action(&work("crush", "grain"), 0);
    assert!(result.success, "grain grinds: {:?}", result.message);

    let flour = simulation.population.agents[0]
        .inventory
        .get_item("flour")
        .expect("there is flour in the pack");

    let grain_worth = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Grain, 0)
        .expect("grain is food")
        .base_nutrition
        .energy;

    let flour_worth = flour
        .food_data
        .as_ref()
        .expect("and flour is food")
        .base_nutrition
        .energy;

    assert!(
        flour_worth > grain_worth,
        "opening the seed gets more out of it: {flour_worth} against {grain_worth}"
    );
}

/// And keeps rather less well, which is why you grind it when you mean to eat
/// it.
#[test]
fn ground_grain_keeps_less_well_than_whole_grain() {
    let simulation = a_person();

    let keeps = |what: crate::world::ItemType| {
        simulation
            .food_database
            .create_food_data(&what, 0)
            .expect("food")
            .base_spoilage_ticks
    };

    assert!(
        keeps(crate::world::ItemType::Flour) < keeps(crate::world::ItemType::Grain),
        "a sack of flour does not keep like a sack of seed"
    );
}

/// A basket is how a person carries more than their arms hold.
#[test]
fn a_basket_carries_what_the_arms_cannot() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    // Emptying the pack takes the founder's basket out of it, but a carrier
    // is put down on a turn like it is taken up - so ask for the turn, or
    // "bare arms" is a pair of arms still wearing a backpack.
    simulation.population.agents[0].take_up_the_cart();
    let bare_arms = simulation.population.agents[0]
        .inventory
        .effective_max_weight();

    give(&mut simulation, "flax", 6);
    let result = simulation.execute_action(&work("weave", "flax"), 0);
    assert!(result.success, "flax weaves: {:?}", result.message);

    assert!(
        simulation.population.agents[0].how_many_i_have("basket") > 0,
        "there is a basket in the pack"
    );

    // And it goes on the back, which is a thing that happens rather than a
    // property of having one. `tick_with_percepts` does this every turn, so in
    // a running world the basket is carrying by the next one; here the turn
    // has to be asked for. Before ISSUES #116 the capacity rose the moment the
    // basket entered the pack *and* again when it was taken up, which is
    // where the double count lived.
    simulation.population.agents[0].take_up_the_cart();

    assert!(
        simulation.population.agents[0]
            .inventory
            .effective_max_weight()
            > bare_arms,
        "and the pack holds more for it"
    );
}

/// Weaving is fingers. It is the one reducing verb that wants nothing in the
/// hand.
#[test]
fn weaving_wants_nothing_in_the_hand() {
    assert!(
        verbs::what_this_action_cannot_do_without("weave").is_empty(),
        "you weave with your fingers"
    );

    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "flax", 6);

    let result = simulation.execute_action(&work("weave", "flax"), 0);
    assert!(result.success, "empty-handed and still weaving");
}

/// A carved bowl is a thing you can put water in.
#[test]
fn a_carved_bowl_holds_water() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);
    give(&mut simulation, "wood", 6);
    give_a_tool(&mut simulation, "stoneknife");
    simulation.population.agents[0].found_out_how_to("bowl");

    let result = simulation.execute_action(&work("carve", "wood"), 0);
    assert!(result.success, "wood carves: {:?}", result.message);

    let bowl = simulation.population.agents[0]
        .inventory
        .get_item("bowl")
        .expect("there is a bowl in the pack");

    assert!(bowl.is_container(), "and it is a thing that holds something");

    // And it fills at the water like any other vessel
    let filled = simulation.population.agents[0]
        .inventory
        .fill_containers(10.0);

    assert!(
        filled > 0.0,
        "a bowl held under a river comes up with water in it"
    );
}

/// Nothing in the world made a container before this.
#[test]
fn something_in_the_world_now_makes_a_vessel() {
    let vessels: Vec<&str> = making::EVERY_WORKING
        .iter()
        .filter(|working| working.holds.is_some())
        .map(|working| working.makes)
        .collect();

    assert!(
        !vessels.is_empty(),
        "the container machinery was written long ago and nothing ever made one"
    );
}
