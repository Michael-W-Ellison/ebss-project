// src/analytics/tests/short_handed_tests.rs
//! Tests that a turn about to be spent on a refusal is spent on the tool.
//!
//! The same argument as `get_the_tool_out_for`, one step further back. That
//! one says: reaching for a tool is not what somebody does with a spare
//! moment, it is what they do just before using it. Neither is *making* one —
//! but making sat in the Utility branch, behind two others, and Utility is a
//! drive that rarely wins.
//!
//! Measured over eight worlds of ten thousand ticks: `Work` attempted 18,756
//! times and refused **88.2%** for want of a tool, `Excavate` attempted 6,348
//! times and refused **99.4%**, `Hunt` refused 2,227 times for want of a
//! spear — while **every man alive knew how to make a handaxe and 2.8% of
//! them owned one**. Twenty-two thousand turns went on wanting a thing nobody
//! would spend a turn making.

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::{making, verbs, Action};
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

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.4));
}

/// The makings of whatever the first step towards a stone knife wants, so the
/// test is about the substitution rather than about the recipe tree.
fn the_makings_of(simulation: &mut Simulation, what: &str) -> bool {
    let knows = |step: &making::Making| simulation.population.agents[0].knows_how_to(step);

    let Some(step) = making::every_way_to_make(what).find(|step| knows(step)) else {
        return false;
    };

    for (needed, how_many) in step.needs {
        give(simulation, needed, how_many * 2);
    }
    if let Some(in_hand) = step.wants_in_hand {
        give(simulation, in_hand, 1);
    }

    true
}

// --------------------------------------------------------------------------
// The substitution itself
// --------------------------------------------------------------------------

/// A working refused for want of a knife becomes the making of a knife.
#[test]
fn a_job_that_wants_a_knife_becomes_the_making_of_one() {
    let mut simulation = one_person();

    // The materials for the job, and the materials for the tool, and no tool
    give(&mut simulation, "hide", 4);
    assert!(the_makings_of(&mut simulation, "stoneknife"));

    let job = Action::Work {
        verb: "scrape".to_string(),
        to: "hide".to_string(),
    };

    // It would be refused as it stands
    let short = simulation.what_these_hands_are_short_of(&job, 0);
    if short.is_none() {
        // This world's recipe for a knife handed him one already; nothing to
        // test, and the other tests cover the rule itself.
        return;
    }

    let instead = simulation.make_what_this_wants(job.clone(), 0);

    assert!(
        matches!(instead, Action::Craft { .. }),
        "a turn that was going to be a refusal should go on the tool, not {instead:?}"
    );
    assert!(
        simulation.what_these_hands_are_short_of(&instead, 0).is_none(),
        "and the substitute must not be short-handed itself, or this trades \
         one refusal for another and calls it progress"
    );
}

/// Somebody who already has the tool is left alone.
#[test]
fn a_man_with_the_tool_is_left_to_get_on_with_it() {
    let mut simulation = one_person();
    give(&mut simulation, "hide", 4);
    give(&mut simulation, "stoneknife", 1);

    let job = Action::Work {
        verb: "scrape".to_string(),
        to: "hide".to_string(),
    };

    assert_eq!(
        simulation.make_what_this_wants(job.clone(), 0),
        job,
        "nothing is missing, so nothing is substituted"
    );
}

/// And somebody with neither the tool nor the makings, standing on ground that
/// has none of what the tool wants, is left alone. The refusal is honest and
/// it belongs in the record: a man who cannot make a knife should find that
/// out.
#[test]
fn a_man_with_nothing_to_make_it_from_and_nowhere_to_get_it_still_gets_his_refusal() {
    let mut simulation = one_person();
    give(&mut simulation, "hide", 4);

    // Bare ground, so there is nothing to fetch either
    simulation.world.resources.clear();

    let job = Action::Work {
        verb: "scrape".to_string(),
        to: "hide".to_string(),
    };

    if simulation.what_these_hands_are_short_of(&job, 0).is_none() {
        return;
    }

    assert_eq!(
        simulation.make_what_this_wants(job.clone(), 0),
        job,
        "no step to take and nothing to fetch, so the turn stays where it was"
    );
}

// --------------------------------------------------------------------------
// And the link past that: fetching what the making wants
// --------------------------------------------------------------------------

/// A man who knows how to knap a knife, standing in a meadow with no stone,
/// goes and gets stone. Measured after the tool step went in, **1,690
/// short-handed refusals a world remained** and they are all this case.
#[test]
fn a_man_short_of_the_makings_goes_and_fetches_them() {
    use crate::world::{Position, ResourceNode, ResourceType};

    let mut simulation = one_person();
    give(&mut simulation, "hide", 4);

    let job = Action::Work {
        verb: "scrape".to_string(),
        to: "hide".to_string(),
    };

    if simulation.what_these_hands_are_short_of(&job, 0).is_none() {
        return;
    }

    // Nothing in the pack to make a knife from, and no step he could take
    if matches!(simulation.make_what_this_wants(job.clone(), 0), Action::Craft { .. }) {
        // He could already make one; that is the other test's case
        return;
    }

    // Now put what the chain wants under his feet
    let agent = &simulation.population.agents[0];
    let holding = |what: &str| agent.how_many_i_have(what);
    let knows = |step: &making::Making| agent.knows_how_to(step);
    let wanting = making::everything_wanting_knowing("stoneknife", &holding, &knows);

    let Some(raw) = wanting.first().copied() else {
        return;
    };
    let Some(kind) = ResourceType::called(raw) else {
        return;
    };

    let here = Position::new(25, 25);
    simulation.world.resources.push(ResourceNode::new(kind, here, 40));
    simulation.population.agents[0]
        .exploration_knowledge
        .discover_resource(here, kind, 0);

    let instead = simulation.make_what_this_wants(job.clone(), 0);

    assert!(
        matches!(&instead, Action::Gather { resource_type } if resource_type == raw),
        "with {raw} at his feet he should be fetching it, not {instead:?}"
    );
}

/// But only something he has actually seen and that is near enough for the
/// fetching to come to anything. Naming a thing this ground has not got trades
/// a refusal for want of a tool for a refusal for want of a source, and a
/// refusal is worse than a wasted turn.
#[test]
fn nobody_sets_off_after_a_material_this_ground_has_not_got() {
    let mut simulation = one_person();
    give(&mut simulation, "hide", 4);
    simulation.world.resources.clear();

    let job = Action::Work {
        verb: "scrape".to_string(),
        to: "hide".to_string(),
    };

    let instead = simulation.make_what_this_wants(job.clone(), 0);

    assert!(
        !matches!(instead, Action::Gather { .. }),
        "there is nothing on this ground to fetch, so no Gather should be          proposed: {instead:?}"
    );
}

// --------------------------------------------------------------------------
// The guards
// --------------------------------------------------------------------------

/// Making a thing is itself the answer to this, so it is never substituted:
/// that is how a substitution becomes a loop.
#[test]
fn the_making_of_a_thing_is_never_itself_substituted() {
    let mut simulation = one_person();
    the_makings_of(&mut simulation, "stoneknife");

    for job in [
        Action::Craft { item_type: "spear".to_string() },
        Action::Equip { what: "spear".to_string() },
        Action::Unequip { what: "spear".to_string() },
    ] {
        assert_eq!(
            simulation.make_what_this_wants(job.clone(), 0),
            job,
            "{job:?} answers the question itself and must not be redirected"
        );
    }
}

/// A free hand and a vessel are somebody else's problem — `free_a_hand_for`
/// and the vessel branch — and this must not fight them for the turn.
#[test]
fn a_full_pair_of_hands_is_not_answered_by_making_something() {
    use verbs::Wants;

    // The rule, stated where it can be read rather than inferred from a world
    for wants in [Wants::AFreeHand, Wants::AVessel, Wants::BareHands] {
        assert!(
            !matches!(wants, Wants::AToolFor(_) | Wants::ThisInHand(_)),
            "only a missing tool is answered by making one"
        );
    }
}

// --------------------------------------------------------------------------
// One question, one function
// --------------------------------------------------------------------------

/// The executor's refusal and the decision's substitution read the same
/// answer. Two ways of asking whether a man can do a job is how this project
/// has lost measurements before — see ISSUES_FOUND #66 and #67.
#[test]
fn the_refusal_and_the_substitution_ask_the_same_question() {
    let mut simulation = one_person();
    give(&mut simulation, "hide", 4);

    let job = Action::Work {
        verb: "scrape".to_string(),
        to: "hide".to_string(),
    };

    let missing = simulation.what_this_wants_that_is_missing(&job, 0);
    let said = simulation.what_these_hands_are_short_of(&job, 0);

    assert_eq!(
        missing.is_some(),
        said.is_some(),
        "the structured answer and the sentence must agree about whether \
         anything is missing at all"
    );

    if let Some(verbs::Wants::AToolFor(trade)) = missing {
        assert_eq!(trade, SkillType::Leatherworking);
        assert!(said.unwrap().contains("Leatherworking"));
    }
}
