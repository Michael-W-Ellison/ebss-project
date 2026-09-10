// src/analytics/tests/goal_tests.rs
//! **Layer 2: what answering a drive would look like, and when it is done.**
//!
//! The layer was not absent. It was distributed across four spellings, none of
//! them called a goal: the thresholds are inside `Preparedness` and
//! `Sustenance` as `ENOUGH_FOOD` and friends; the commitment is `Errand`; the
//! food reckoning is `WhatIsPutBy`; and the type actually named `Goal` is
//! about emotions and property and was measured near-dead in #187.
//!
//! So what is asserted here is mostly that the naming did not fork the model:
//! a goal's threshold is the same number the drive uses, and a goal that
//! nothing can answer says so out loud. The one exception is
//! `ObtainPotableWater`, which nothing asked before and which now spends a
//! turn that would otherwise have been spent standing still.

use crate::agents::{AgentConfig, Population};
use crate::analytics::wanting::goal::{Enough, Goal, WhoAsks};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation
}

/// **The goal table and the drives cannot drift into two sets of books.**
///
/// The guard the whole layer stands on. `Preparedness` is
/// `short_of(food_put_by, ENOUGH_FOOD)`, and `PreserveFood` is the same
/// question in Layer 2's words - so it has to be the same number, taken from
/// the same place. If somebody changes one, this fails rather than the model
/// quietly keeping two opinions about what enough is.
#[test]
fn the_goal_table_and_the_drives_cannot_drift() {
    assert_eq!(
        Goal::PreserveFood.enough(),
        Enough::UnitsPutBy(DriveType::ENOUGH_FOOD),
        "the winter store threshold has been restated instead of read"
    );
    assert_eq!(
        Goal::AcquireCuttingTool.enough(),
        Enough::ToolsToHand(DriveType::ENOUGH_TOOLS),
    );
}

/// Every goal answers a drive that exists, and every drive with goals lists
/// them.
#[test]
fn every_goal_hangs_off_a_drive_that_lists_it() {
    for goal in Goal::every_one() {
        let need = goal.answers();
        assert!(
            Goal::all_for(need).contains(goal),
            "{goal:?} says it answers {need:?} and {need:?} does not list it"
        );
    }
}

/// **A goal nothing can answer says so, rather than being quietly always met.**
///
/// The same honesty device as `Strategy::reach`. A declared and unanswerable
/// goal is a gap somebody can count; one that was never named is a gap nobody
/// knows is there.
#[test]
fn a_goal_nothing_can_tell_you_about_says_so() {
    assert!(!Goal::ImproveLocalShelter.can_be_told());
    assert!(matches!(
        Goal::ImproveLocalShelter.enough(),
        Enough::NothingCanSay(_)
    ));

    // And everything else can be told.
    for goal in Goal::every_one() {
        if *goal == Goal::ImproveLocalShelter {
            continue;
        }
        assert!(goal.can_be_told(), "{goal:?} cannot be answered and does not say so");
    }
}

/// Exactly one goal is asked by nobody and is the thing this layer adds.
#[test]
fn the_one_goal_nobody_asked_is_the_water() {
    let unasked: Vec<_> = Goal::every_one()
        .iter()
        .filter(|goal| goal.who_already_asks_it() == WhoAsks::Nobody)
        .filter(|goal| goal.can_be_told())
        .collect();

    assert_eq!(
        unasked,
        vec![&Goal::ObtainPotableWater],
        "the set of goals nothing asks has changed and nobody said so"
    );
}

/// **A man with no vessel is not short of carried water.**
///
/// Layer 5 gating Layer 2. Without this the goal could never be met and would
/// ask for ever - which is how a layer meant to say "this is done" turns into
/// a drive that never goes quiet.
#[test]
fn a_man_with_nothing_to_carry_water_in_is_not_short_of_it() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    assert!(
        !agent.inventory.has_a_container(),
        "the fixture is meant to start him with no vessel"
    );
    assert_eq!(
        simulation.how_short_of(Goal::ObtainPotableWater, agent, agent.state.position),
        0.0,
        "he was told to fill a skin he does not have"
    );
}

/// And with a vessel and nothing in it, he is as short as he can be.
#[test]
fn an_empty_skin_is_a_goal_wide_open() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new_container(
            "waterskin".to_string(),
            1,
            2.0,
        ));

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.how_short_of(Goal::ObtainPotableWater, agent, agent.state.position),
        1.0
    );
}

/// **The end-to-end statement: a turn nobody wanted goes on tomorrow's water.**
#[test]
fn a_spare_turn_beside_the_water_fills_the_skin() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new_container(
            "waterskin".to_string(),
            1,
            2.0,
        ));

    // Water under his feet.
    simulation.world.resources.push(crate::world::ResourceNode::new(
        crate::world::ResourceType::Water,
        crate::world::Position::new(25, 25),
        50,
    ));

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.top_up_before_you_need_it(agent, agent.state.position),
        Some(Action::Gather {
            resource_type: "water".to_string()
        }),
        "he stood beside the river with an empty skin and did nothing"
    );
}

/// And a full skin is not a reason to stand in the river.
#[test]
fn a_full_skin_is_not_topped_up_again() {
    let mut simulation = one_person();
    let mut skin = crate::agents::InventoryItem::new_container("waterskin".to_string(), 1, 2.0);
    skin.fill(2.0);
    simulation.population.agents[0].inventory.add_item(skin);

    simulation.world.resources.push(crate::world::ResourceNode::new(
        crate::world::ResourceType::Water,
        crate::world::Position::new(25, 25),
        50,
    ));

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.top_up_before_you_need_it(agent, agent.state.position),
        None
    );
}

/// Nor is a dry hillside.
#[test]
fn there_is_no_topping_up_away_from_water() {
    let mut simulation = one_person();
    simulation.world.resources.clear();
    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new_container(
            "waterskin".to_string(),
            1,
            2.0,
        ));

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.top_up_before_you_need_it(agent, agent.state.position),
        None
    );
}
