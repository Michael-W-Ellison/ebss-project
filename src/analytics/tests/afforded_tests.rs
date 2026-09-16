// src/analytics/tests/afforded_tests.rs
//! What could I do here, holding this?
//!
//! "An action like holding something should unlock additional actions."
//!
//! The matrix has said so all along - twenty-seven of its seventy-four verbs
//! target `AThingHeld` and are closed to empty hands - and nothing asked it.
//! `EVERY_VERB` was read by the test suite and by nothing else; the one
//! production caller of the module asks what a *named* verb wants so it can go
//! and fetch it. These are the first tests of the question asked the other way
//! round.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::environment::verbs::{Targets, EVERY_VERB};
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// One person on bare ground, with nothing in the pack and nothing underfoot.
///
/// Bare on purpose: what is being measured is which verbs *open* when
/// something is added, so the starting state has to be one where they are
/// shut.
fn one_person_on_bare_ground() -> crate::analytics::Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.dropped.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = crate::analytics::Simulation::new(world, population);

    // `Simulation::new` places people, so the tile to clear is the one they
    // ended up on rather than the one they spawned on.
    let at = simulation.population.agents[0].state.position;
    simulation
        .world
        .resources
        .retain(|node| node.position != Position::new(at.0, at.1));
    simulation.world.dropped.clear();
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();

    simulation
}

fn open_verbs(simulation: &crate::analytics::Simulation) -> Vec<&'static str> {
    let agent = &simulation.population.agents[0];
    let mut names: Vec<&'static str> = simulation
        .what_i_could_do_here(agent)
        .into_iter()
        .map(|verb| verb.called)
        .collect();
    names.sort_unstable();
    names
}

/// Holding something opens verbs that empty hands do not have.
///
/// The whole claim, and it needs no rule of its own: `Targets::AThingHeld` was
/// declared against twenty-seven verbs long before anybody asked which of them
/// a given pair of hands could reach.
#[test]
fn holding_something_opens_verbs_that_empty_hands_do_not() {
    let mut simulation = one_person_on_bare_ground();
    let empty_handed = open_verbs(&simulation);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, 1.0));
    let carrying = open_verbs(&simulation);

    let opened: Vec<_> = carrying
        .iter()
        .filter(|verb| !empty_handed.contains(verb))
        .collect();

    assert!(
        !opened.is_empty(),
        "a stone in the pack opened nothing: {} verbs either way",
        empty_handed.len()
    );

    // And specifically the family that wants a thing to act on. Named rather
    // than counted, so that a matrix edit that moves one of them shows up here
    // as a changed list rather than as a changed number.
    for wants_a_thing in ["drop", "dry", "salt"] {
        assert!(
            carrying.contains(&wants_a_thing) && !empty_handed.contains(&wants_a_thing),
            "{wants_a_thing} targets a thing held and did not open when one was held"
        );
    }
}

/// And nothing underfoot shuts the verbs that want something underfoot.
///
/// The other half of the target question, and the half the matrix had no way
/// of answering: `Wants` came with `satisfied_by_hands` and could always be
/// asked, `Targets` was declared and never once put to a world.
#[test]
fn something_underfoot_opens_the_verbs_that_want_one() {
    let mut simulation = one_person_on_bare_ground();
    let bare = open_verbs(&simulation);

    let at = simulation.population.agents[0].state.position;
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(at.0, at.1),
        50,
    ));
    let standing_on_a_bush = open_verbs(&simulation);

    assert!(
        standing_on_a_bush.len() > bare.len(),
        "a bush underfoot opened nothing: {} verbs either way",
        bare.len()
    );

    let underfoot: Vec<&'static str> = EVERY_VERB
        .iter()
        .filter(|verb| verb.targets == Targets::AThingUnderfoot)
        .map(|verb| verb.called)
        .collect();
    assert!(
        underfoot
            .iter()
            .any(|verb| standing_on_a_bush.contains(verb) && !bare.contains(verb)),
        "not one of the verbs that wants a thing underfoot opened when one arrived"
    );
}

/// A verb the matrix declares and nothing performs is reported, and marked.
///
/// Eighteen of the fifty-four are in that state. Dropping them silently would
/// report a world of possibilities smaller than the matrix describes and give
/// no sign of the difference - which is the objection `verbs.rs` makes to
/// itself in its own header: "a matrix that quietly implied sixty-eight
/// working verbs would be worse than no matrix".
#[test]
fn a_declared_verb_nobody_performs_is_offered_but_not_as_a_thing_to_do() {
    let simulation = one_person_on_bare_ground();
    let agent = &simulation.population.agents[0];

    let could = simulation.what_i_could_do_here(agent);
    let could_now = simulation.what_i_could_do_here_now(agent);

    let declared_only: Vec<&'static str> = could
        .iter()
        .filter(|verb| verb.done_by.is_none())
        .map(|verb| verb.called)
        .collect();

    assert!(
        !declared_only.is_empty(),
        "no unperformed verb came back at all, so this test is not watching anything"
    );
    assert!(
        could_now.len() < could.len(),
        "the filtered form dropped nothing, so it is not filtering"
    );
    for verb in could_now {
        assert!(
            verb.done_by.is_some(),
            "{} has no performer and came back as a thing to do now",
            verb.called
        );
    }
}

/// The generator agrees with the question the executor asks.
///
/// `what_this_one_is_short_of` is what refuses an action for want of a tool or
/// a hand, and it and this now share `do_these_hands_do`. If they ever drift,
/// the decision layer will offer verbs the executor refuses - which is the
/// exact shape of #215 and #243, and this project has paid for it twice.
#[test]
fn what_is_offered_is_what_the_hands_can_actually_do() {
    let mut simulation = one_person_on_bare_ground();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, 1.0));

    let agent = simulation.population.agents[0].clone();
    for verb in simulation.what_i_could_do_here(&agent) {
        assert!(
            crate::analytics::Simulation::do_these_hands_do(&agent, &verb.wants),
            "{} was offered and these hands cannot do it",
            verb.called
        );
    }
}
