// src/analytics/tests/swap_tests.rs
//! Tests for putting the wrong thing where a part goes.
//!
//! "Knowing that a stone tool requires the use of specific sub-components, an
//! agent might substitute known sub-components for new/random things."
//!
//! A man who can haft a flake to a stick knows the shape of the job: a shaft,
//! a head, something to bind them. That shape is a thing he can reason with. He
//! has a lump of something in his pack, and where the head goes is a place a
//! lump could go. Almost always he ends up with a lump tied to a stick and has
//! wasted the stick. Occasionally he ends up with an axe nobody had.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::making;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn a_person() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(World::new(WorldConfig::default()), population)
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

fn empty_the_pack(simulation: &mut Simulation) {
    let everything: Vec<(String, u32)> = simulation.population.agents[0]
        .inventory
        .get_all_items()
        .values()
        .map(|item| (item.item_id.clone(), item.quantity))
        .collect();

    for (what, how_many) in everything {
        for _ in 0..how_many {
            simulation.population.agents[0].inventory.remove_item(&what, 1);
        }
    }
}

fn swap(instead_of_making: &str, instead_of: &str, put_in: &str) -> Action {
    Action::TrySwapping {
        instead_of_making: instead_of_making.to_string(),
        instead_of: instead_of.to_string(),
        put_in: put_in.to_string(),
    }
}

// --------------------------------------------------------------------------
// The table itself
// --------------------------------------------------------------------------

/// Every substitution that comes to something names a step that exists and a
/// part that step actually wants.
#[test]
fn every_swap_names_a_real_step_and_a_real_part() {
    for one in making::EVERY_SWAP {
        let step = making::how_to_make(one.instead_of_making)
            .unwrap_or_else(|| panic!("{} is not a step", one.instead_of_making));

        assert!(
            step.needs.iter().any(|(what, _)| *what == one.instead_of),
            "{} does not want a {}",
            one.instead_of_making,
            one.instead_of
        );
        assert!(
            !step.needs.iter().any(|(what, _)| *what == one.put_in),
            "{} already wants a {}, so putting one in is not a substitution",
            one.instead_of_making,
            one.put_in
        );
        assert!(one.how_many >= 1);
    }
}

/// Anything not in the table comes to nothing.
#[test]
fn most_substitutions_come_to_nothing() {
    assert!(making::what_comes_of_swapping("spear", "knappedtip", "stone").is_none());
    assert!(making::what_comes_of_swapping("spear", "wood", "iron").is_none());
    assert!(making::what_comes_of_swapping("handaxe", "lashing", "wool").is_none());

    // And the ones that do are the ones that do
    assert_eq!(
        making::what_comes_of_swapping("lashing", "flax", "hides").map(|swap| swap.makes),
        Some("lashing")
    );
    assert_eq!(
        making::what_comes_of_swapping("handaxe", "knappedtip", "metalblade")
            .map(|swap| swap.makes),
        Some("metalaxe")
    );
}

/// The new tools are worth having, and last longer than stone.
#[test]
fn the_things_a_swap_finds_are_better_than_what_they_replace() {
    let stone_axe = making::what_helps_with(crate::agents::SkillType::Woodcutting)
        .find(|tool| tool.called == "handaxe")
        .expect("a hand axe helps with wood");
    let metal_axe = making::what_helps_with(crate::agents::SkillType::Woodcutting)
        .find(|tool| tool.called == "metalaxe")
        .expect("and so does a metal one");

    assert!(
        metal_axe.how_much_better > stone_axe.how_much_better,
        "a metal axe cuts better"
    );
    assert!(
        metal_axe.how_long_it_lasts > stone_axe.how_long_it_lasts,
        "and lasts longer"
    );
}

// --------------------------------------------------------------------------
// Doing it
// --------------------------------------------------------------------------

/// A substitution that works produces the new thing, and the man knows how
/// afterwards.
#[test]
fn a_substitution_that_works_makes_something_new() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    // Everything a hand axe wants except the flake, and a blade instead
    give(&mut simulation, "wood", 2);
    give(&mut simulation, "lashing", 2);
    give(&mut simulation, "metalblade", 1);

    let result = simulation.execute_action(&swap("handaxe", "knappedtip", "metalblade"), 0);

    assert!(result.success, "that works: {:?}", result.message);
    assert!(
        simulation.population.agents[0].how_many_i_have("metalaxe") > 0,
        "and there is a metal axe in the pack"
    );
    assert!(
        simulation.population.agents[0]
            .what_i_found_out()
            .contains("metalaxe"),
        "and he can do it again on purpose now"
    );
}

/// One that does not still costs the materials.
#[test]
fn a_substitution_that_fails_costs_the_materials_anyway() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    give(&mut simulation, "wood", 2);
    give(&mut simulation, "lashing", 2);
    give(&mut simulation, "stone", 4);

    let wood_before = simulation.population.agents[0].how_many_i_have("wood");
    let cord_before = simulation.population.agents[0].how_many_i_have("lashing");

    let result = simulation.execute_action(&swap("spear", "knappedtip", "stone"), 0);

    assert!(!result.success, "a rock lashed to a stick is not a spear");
    assert!(
        simulation.population.agents[0].how_many_i_have("wood") < wood_before,
        "the stick is gone"
    );
    assert!(
        simulation.population.agents[0].how_many_i_have("lashing") < cord_before,
        "and so is the cord"
    );
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("spear"),
        0,
        "and there is no spear"
    );
}

/// Nothing happens without the rest of the makings.
#[test]
fn a_substitution_needs_the_rest_of_the_parts() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    // A blade and nothing else
    give(&mut simulation, "metalblade", 1);

    let result = simulation.execute_action(&swap("handaxe", "knappedtip", "metalblade"), 0);
    assert!(
        !result.success,
        "you cannot haft anything without a haft: {:?}",
        result.message
    );
}

/// A man who has tried the same wrong thing enough times stops trying it.
#[test]
fn nobody_keeps_putting_the_same_wrong_thing_in() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    let called = making::what_that_swap_is_called("spear", "knappedtip", "stone");

    // A pack holds only so much, so the makings are handed over a few at a
    // time rather than all at once
    for _ in 0..40 {
        give(&mut simulation, "wood", 1);
        give(&mut simulation, "lashing", 1);
        give(&mut simulation, "stone", 1);
        simulation.execute_action(&swap("spear", "knappedtip", "stone"), 0);
    }

    let agent = &simulation.population.agents[0];
    assert!(
        agent.lessons.tried_this(&called) >= 12,
        "he gave it a fair go: {}",
        agent.lessons.tried_this(&called)
    );
    assert!(
        agent.lessons.how_likely_to_try_this(&called) < 0.5,
        "and has largely given up on it: {:.2}",
        agent.lessons.how_likely_to_try_this(&called)
    );
}

// --------------------------------------------------------------------------
// Choosing to
// --------------------------------------------------------------------------

/// An agent one part short, with something else to hand, offers a
/// substitution.
#[test]
fn being_one_part_short_is_what_prompts_it() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    // An empty pack is not a thought: there is nothing to leave out and
    // nothing to put in its place
    assert!(
        simulation.population.agents[0].what_i_would_swap().is_none(),
        "a man with nothing in his hands is not experimenting with anything"
    );

    give(&mut simulation, "wood", 2);
    give(&mut simulation, "lashing", 2);
    give(&mut simulation, "metalblade", 1);

    let proposed = simulation.population.agents[0].what_i_would_swap();
    assert!(
        proposed.is_some(),
        "a blade and a stick and some cord is a thought"
    );

    // Whatever it proposes has the shape of a substitution: a step this agent
    // can do, one of its parts left out, and something in the pack that the
    // step never wanted put in that place
    let (instead_of_making, instead_of, put_in) = proposed.unwrap();
    let step = making::how_to_make(&instead_of_making).expect("a real step");

    assert!(step.obvious, "and a job he knows how to do");
    assert!(
        step.needs.iter().any(|(what, _)| *what == instead_of),
        "the part left out is a part the step wanted"
    );
    assert!(
        !step.needs.iter().any(|(what, _)| *what == put_in),
        "and what goes in its place is not something it wanted anyway"
    );
    assert!(
        simulation.population.agents[0].how_many_i_have(&put_in) > 0,
        "he is actually holding it"
    );
}

/// It never proposes a substitution for a step the agent has not worked out.
#[test]
fn nobody_substitutes_into_a_job_they_cannot_do() {
    let mut simulation = a_person();
    empty_the_pack(&mut simulation);

    // The makings of a metal knife, for somebody who has never seen one made
    give(&mut simulation, "metalblade", 2);
    give(&mut simulation, "lashing", 2);
    give(&mut simulation, "wool", 2);

    if let Some((instead_of_making, _, _)) =
        simulation.population.agents[0].what_i_would_swap()
    {
        let step = making::how_to_make(&instead_of_making).expect("a real step");
        assert!(
            step.obvious,
            "he proposed to vary {instead_of_making}, which he has never made"
        );
    }
}

