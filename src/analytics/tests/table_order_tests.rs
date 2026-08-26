// src/analytics/tests/table_order_tests.rs
//! Tests for the order of a hand-written list not deciding what a people makes,
//! and for leatherworking being the scraping rather than the sewing.
//!
//! `what_i_would_work_on` took the **first** thing in the working table it
//! could do and stopped. So whatever sits early in that table and has materials
//! to hand wins every turn, for every agent, for ever. Carving a bowl sits
//! late — and measured, a settlement made essentially no vessels at all however
//! badly it wanted one, which cost it carried water, boiling, and salt.
//!
//! This exact trap was found and fixed once before, in
//! `what_working_i_would_try_out`: retting flax sits above fermenting fruit, so
//! over eight worlds nobody ever fermented anything, because somebody always
//! had flax. The fix never got carried across to the function next to it.
//!
//! And leatherworking is what you do to a *hide*: taking a flint to it removes
//! the hair and turns skin into leather. Sewing a bag out of the leather
//! afterwards is making, like any other making — putting the skill on both
//! steps paid a man twice for one trade.

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::{making, Action};
use crate::world::{World, WorldConfig};

fn a_settlement(how_many: usize) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for who in 0..how_many {
        simulation.population.agents[who].state.position = (25, 25, 0);
        simulation.population.agents[who]
            .inventory
            .get_all_items_mut()
            .clear();
        simulation.population.agents[who].inventory.recalculate_weight();
    }
    simulation
}

// --------------------------------------------------------------------------
// Where a man starts in the list is his own business
// --------------------------------------------------------------------------

/// Forty people who could all do the same several things do not all do the
/// same one thing.
#[test]
fn a_people_with_the_same_materials_does_not_all_make_the_same_thing() {
    let mut simulation = a_settlement(40);

    // Everybody holding plenty of everything the obvious workings want, so
    // that the only thing deciding what each does is where they start.
    for who in 0..40 {
        let agent = &mut simulation.population.agents[who];
        agent.inventory.max_weight = 2000.0;
        for what in ["wood", "stone", "hides", "flax", "leather", "clay"] {
            agent
                .inventory
                .add_item(InventoryItem::new_with_weight(what.to_string(), 40, 0.1));
        }
        agent.inventory.recalculate_weight();
    }

    let chosen: std::collections::HashSet<(String, String)> = simulation
        .population
        .agents
        .iter()
        .filter_map(|agent| agent.what_i_would_work_on())
        .collect();

    assert!(
        chosen.len() > 1,
        "forty people with the same pack all reached for the same thing: {chosen:?}"
    );
}

/// And the same man reaches for the same thing every time, because this is
/// about where he starts rather than about a coin toss.
#[test]
fn the_same_man_reaches_for_the_same_thing() {
    let mut simulation = a_settlement(1);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 2000.0;
        for what in ["wood", "stone", "hides", "flax"] {
            agent
                .inventory
                .add_item(InventoryItem::new_with_weight(what.to_string(), 40, 0.1));
        }
        agent.inventory.recalculate_weight();
    }

    // Sampled rather than asserted once: whether he can be bothered today is a
    // coin toss by design - see `Lessons::will_try_this_again` - and it is only
    // *where he starts* that is fixed.
    let mut reached: std::collections::HashMap<(String, String), u32> =
        std::collections::HashMap::new();
    for _ in 0..200 {
        if let Some(what) = simulation.population.agents[0].what_i_would_work_on() {
            *reached.entry(what).or_insert(0) += 1;
        }
    }

    let favourite = reached
        .values()
        .max()
        .copied()
        .expect("he can do several things");

    assert!(
        favourite as f32 / 200.0 > 0.8,
        "a man's trade is his own and it does not change by the hour: {reached:?}"
    );
}

/// Somebody with the makings of exactly one thing makes that thing, wherever
/// it sits in the table.
#[test]
fn somebody_who_can_do_one_thing_does_it() {
    let mut simulation = a_settlement(1);

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("hides".to_string(), 6, 1.0));
    }

    // Sampled, because the willingness is a coin toss and refuses about one
    // time in twenty by design.
    let reached = (0..40)
        .filter_map(|_| simulation.population.agents[0].what_i_would_work_on())
        .filter(|(verb, to)| verb == "scrape" && to == "hides")
        .count();

    assert!(reached > 20, "he did it {reached} times in 40");
}

/// And somebody with nothing makes nothing.
#[test]
fn somebody_with_nothing_makes_nothing() {
    let simulation = a_settlement(1);
    assert_eq!(
        simulation.population.agents[0].what_i_would_work_on(),
        None
    );
}

/// A carved bowl is reachable now, which is the whole point: it sits late in
/// the table and was never once reached.
#[test]
fn a_bowl_is_reachable_by_somebody() {
    let mut simulation = a_settlement(40);

    for who in 0..40 {
        let agent = &mut simulation.population.agents[who];
        agent.inventory.max_weight = 2000.0;
        for what in ["wood", "stone", "hides", "flax", "leather", "clay"] {
            agent
                .inventory
                .add_item(InventoryItem::new_with_weight(what.to_string(), 40, 0.1));
        }
        agent.inventory.recalculate_weight();
    }

    let anybody_carving = simulation
        .population
        .agents
        .iter()
        .filter_map(|agent| agent.what_i_would_work_on())
        .any(|(verb, to)| verb == "carve" && to == "wood");

    assert!(
        anybody_carving,
        "a bowl sits late in the table, and somebody in forty ought to get to it"
    );
}

// --------------------------------------------------------------------------
// Leatherworking is what you do to a hide
// --------------------------------------------------------------------------

/// Taking a flint to a hide removes the hair and turns skin into leather.
/// Cutting a hide gets you two smaller hides.
#[test]
fn leather_is_scraped_off_a_hide_rather_than_cut_out_of_one() {
    let scraping = making::how_to_work("scrape", "hides").expect("a hide scrapes");

    assert_eq!(scraping.makes, "leather");
    assert_eq!(
        scraping.hands,
        SkillType::Leatherworking,
        "this is what leatherworking is"
    );

    assert!(
        making::how_to_work("cut", "hides").is_none(),
        "cutting a hide gets you two smaller hides"
    );
}

/// And sewing a bag out of the leather afterwards is making, like any other
/// making. Putting the skill on both steps pays a man twice for one trade.
#[test]
fn sewing_a_bag_is_making_and_not_leatherworking() {
    let sewing = making::how_to_work("weave", "leather").expect("leather sews");

    assert_eq!(sewing.makes, "leatherbag");
    assert_eq!(
        sewing.hands,
        SkillType::Crafting,
        "the skill sits one step earlier, on the scraping"
    );
}

/// What gates the bag is the material rather than the hand: a hide is not
/// leather until somebody has scraped the hair off it.
#[test]
fn what_gates_the_bag_is_the_leather() {
    let mut simulation = a_settlement(1);

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("hides".to_string(), 20, 1.0));
        let knife = making::what_helps_with(SkillType::Leatherworking)
            .next()
            .expect("something in this world scrapes");
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight(knife.called.to_string(), 1, 1.0));
        agent.take_in_hand(knife.called);
    }

    // Hides in the pack and no leather: the bag is not on the table yet.
    let result = simulation.execute_action(
        &Action::Work {
            verb: "weave".to_string(),
            to: "leather".to_string(),
        },
        0,
    );
    assert!(!result.success, "no leather to sew");

    // Scrape one, and it is.
    let scraped = simulation.execute_action(
        &Action::Work {
            verb: "scrape".to_string(),
            to: "hides".to_string(),
        },
        0,
    );
    assert!(scraped.success, "{:?}", scraped.message);
    assert!(simulation.population.agents[0].how_many_i_have("leather") > 0);
}

/// Scraping still wants something to scrape with.
#[test]
fn scraping_a_hide_wants_a_flint() {
    let mut simulation = a_settlement(1);

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("hides".to_string(), 6, 1.0));
    }

    let result = simulation.execute_action(
        &Action::Work {
            verb: "scrape".to_string(),
            to: "hides".to_string(),
        },
        0,
    );

    assert!(!result.success, "bare hands do not take the hair off a hide");
}
