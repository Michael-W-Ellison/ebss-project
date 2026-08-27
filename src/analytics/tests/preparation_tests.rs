// src/analytics/tests/preparation_tests.rs
//! Tests for making the trip pay.
//!
//! "I am going here or doing this action anyway — is there anything I can do
//! which decreases the time to satisfy a drive without detracting from the
//! current one?" A trip out is the expensive part and the load is nearly free.
//!
//! Two halves. **Something to carry water in**, which nothing in this world
//! had ever wanted: a bowl and a fired pot both declared what they hold and
//! neither was ever made by anybody, because `what_i_would_make` asks only
//! after tools — something to hunt with, to cut wood with, to work a hide
//! with. So no agent could carry water and every drink was a walk to the
//! river; and `Boil` was refused for want of something to hold the sea in
//! **247 times a world**, which put salt out of reach on the same account.
//!
//! And **taking what you can carry while you are standing there**. A salt flat
//! is a long walk and salt keeps for ever, so taking one lot is throwing away
//! the walk.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::{making, Action};
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
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
// Something to carry water in
// --------------------------------------------------------------------------

/// Somebody with the wood and the knowledge, and nothing to carry water in,
/// wants a vessel.
#[test]
fn somebody_with_nothing_to_carry_water_in_wants_a_vessel() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 1.0));
        agent.found_out_how_to("bowl");
    }

    assert_eq!(
        simulation.population.agents[0].what_vessel_i_would_rather_have(),
        Some(("carve", "wood")),
        "a bowl is what a person with wood and an idea makes"
    );
}

/// Somebody who already has one does not want another.
#[test]
fn somebody_who_has_one_does_not_want_another() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 1.0));
        agent.found_out_how_to("bowl");
        agent
            .inventory
            .add_item(InventoryItem::new_container("bowl".to_string(), 1, 4.0));
    }

    assert_eq!(
        simulation.population.agents[0].what_vessel_i_would_rather_have(),
        None
    );
}

/// Nobody carves a bowl out of an idea: it wants wood in the pack.
#[test]
fn a_vessel_wants_wood_and_wants_knowing_about() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent.found_out_how_to("bowl");
    }
    assert_eq!(
        simulation.population.agents[0].what_vessel_i_would_rather_have(),
        None,
        "no wood"
    );

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 1.0));
    }
    assert!(simulation.population.agents[0]
        .what_vessel_i_would_rather_have()
        .is_some());

    let carving = making::how_to_work("carve", "wood").expect("wood carves");
    assert!(carving.holds.is_some_and(|held| held > 0.0));
    assert!(
        carving.obvious,
        "weaving a basket out of flax is obvious in this table and hollowing \
         out a block of wood is no greater a leap - and gating it kept the \
         entire fluid family inert"
    );
}

/// The bigger vessel wins, which is why anybody bothers firing clay.
#[test]
fn the_vessel_that_holds_more_is_the_one_worth_making() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 1.0));
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("claypot".to_string(), 8, 1.0));
        agent.found_out_how_to("bowl");
        agent.found_out_how_to("stoneware");
    }

    let bowl = making::how_to_work("carve", "wood").unwrap().holds.unwrap();
    let pot = making::how_to_work("fire", "claypot").unwrap().holds.unwrap();
    assert!(pot > bowl, "a fired pot is the better vessel");

    assert_eq!(
        simulation.population.agents[0].what_vessel_i_would_rather_have(),
        Some(("fire", "claypot"))
    );
}

/// And what comes off the making actually holds water.
#[test]
fn what_comes_off_the_making_holds_water() {
    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 1.0));
        agent.found_out_how_to("bowl");

        // Carving wants something to carve with, and the matrix is right to
        // insist on it
        let tool = making::what_helps_with(crate::agents::SkillType::Crafting)
            .next()
            .expect("something in this world carves");
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight(tool.called.to_string(), 1, 1.0));
        agent.take_in_hand(tool.called);
    }

    let result = simulation.execute_action(
        &Action::Work {
            verb: "carve".to_string(),
            to: "wood".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    assert!(
        simulation.population.agents[0].what_i_can_hold_water_in() > 0,
        "he has something to carry water in now"
    );
}

/// Which is the point of the whole thing: a person with a vessel walks away
/// from the water with water.
#[test]
fn somebody_with_a_vessel_leaves_the_river_carrying_water() {
    let carried = |vessel: bool| {
        let mut simulation = one_person();
        let here = Position::new(25, 25);

        simulation
            .world
            .resources
            .push(ResourceNode::new(ResourceType::Water, here, 200));

        if vessel {
            simulation.population.agents[0]
                .inventory
                .add_item(InventoryItem::new_container("bowl".to_string(), 1, 4.0));
        }

        let result = simulation.execute_action(
            &Action::Gather {
                resource_type: "water".to_string(),
            },
            0,
        );
        assert!(result.success, "{:?}", result.message);

        simulation.population.agents[0].inventory.available_water()
    };

    assert!(
        carried(true) > carried(false),
        "the trip is the cost and the water is free"
    );
    assert_eq!(carried(false), 0.0, "and without one you drink and walk home");
}

// --------------------------------------------------------------------------
// Taking what you can while you are here
// --------------------------------------------------------------------------

/// Standing on a salt flat with no salt, a person fills up.
#[test]
fn somebody_standing_on_salt_takes_what_they_can_carry() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Salt,
        Position::new(here.0, here.1),
        400,
    ));

    assert_eq!(
        simulation
            .what_i_should_take_while_i_am_here(&simulation.population.agents[0], here)
            .as_deref(),
        Some("salt")
    );
}

/// And stops once they have a working stock of it, rather than standing there
/// for the rest of their life.
#[test]
fn somebody_with_a_stock_of_it_walks_on() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Salt,
        Position::new(here.0, here.1),
        400,
    ));

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 900.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("salt".to_string(), 60, 0.1));
    }

    assert_eq!(
        simulation.what_i_should_take_while_i_am_here(&simulation.population.agents[0], here),
        None
    );
}

/// A thing nine paces off is a trip, not a top-up. The premise is that the
/// walk has already been paid for.
#[test]
fn a_thing_across_the_field_is_a_trip_and_not_a_top_up() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Salt,
        Position::new(here.0 + 9, here.1),
        400,
    ));

    assert_eq!(
        simulation.what_i_should_take_while_i_am_here(&simulation.population.agents[0], here),
        None
    );
}

/// Nobody carries home a fortnight of berries. The question is about keeping,
/// not about eating.
#[test]
fn nobody_stocks_up_on_something_that_will_not_keep() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(here.0, here.1),
        400,
    ));

    assert_eq!(
        simulation.what_i_should_take_while_i_am_here(&simulation.population.agents[0], here),
        None
    );

    assert!(Simulation::does_it_keep("wood"));
    assert!(Simulation::does_it_keep("salt"));
    assert!(!Simulation::does_it_keep("greens"));
    assert!(!Simulation::does_it_keep("roots"));
}

/// Nor from ground they know they have already stripped.
#[test]
fn nobody_tops_up_at_a_patch_they_picked_bare() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;
    let underfoot = Position::new(here.0, here.1);

    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Wood, underfoot, 400));

    assert!(simulation
        .what_i_should_take_while_i_am_here(&simulation.population.agents[0], here)
        .is_some());

    simulation.population.agents[0]
        .exploration_knowledge
        .found_none_at(underfoot, simulation.current_tick);

    assert_eq!(
        simulation.what_i_should_take_while_i_am_here(&simulation.population.agents[0], here),
        None
    );
}

/// Nor with their arms already full.
#[test]
fn nobody_tops_up_with_their_arms_full() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Wood,
        Position::new(here.0, here.1),
        400,
    ));

    {
        let agent = &mut simulation.population.agents[0];
        let full = agent.inventory.effective_max_weight();
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, full));
        agent.inventory.recalculate_weight();
    }

    assert_eq!(
        simulation.what_i_should_take_while_i_am_here(&simulation.population.agents[0], here),
        None
    );
}
