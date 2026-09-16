// src/analytics/tests/workings_tests.rs
//! Doing a thing to a thing, and getting a different thing.
//!
//! A verb in the matrix is only half of a process. The other half is a
//! `Working` - `how_to_work(verb, to)` - and without one the executor answers
//! "Nothing comes of {verb} a {to}" and the turn is spent on nothing. Audited
//! before these were written: of the verbs in the working families, **`split`
//! and `drill` were declared, had no working, and were marked `done_by: None`
//! into the bargain**, so they were unreachable twice over. `grind` was not in
//! the matrix at all.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::environment::verbs::{Family, EVERY_VERB};
use crate::environment::{making, Action};
use crate::world::{World, WorldConfig};

/// Every verb in a working family either comes to something or is admitted
/// to be declared and nothing more.
///
/// Three states, and only the third is a hole: it has a `Working`, so the
/// executor turns one thing into another; or the matrix names some other
/// action that performs it, and that action answers for it; or neither, in
/// which case choosing it spends a turn on "Nothing comes of it" and the
/// curiosity layer will keep choosing it, because novelty does not care
/// whether a thing works.
///
/// The first two are read off the data rather than listed here. What is
/// listed is the third, so that the list is the audit and not an excuse.
#[test]
fn every_working_verb_comes_to_something_or_is_owned_up_to() {
    // Declared, performed by nothing, and worked into nothing. Each is a
    // process this world names and cannot do.
    let declared_and_nothing_more = [
        "cool", "quench", "mix", "pour", "coat", "leach", "fold", "stack", "pierce",
        "fill",
    ];

    let mut holes = Vec::new();
    for verb in EVERY_VERB.iter().filter(|verb| {
        matches!(
            verb.family,
            Family::Disruption | Family::Assembly | Family::Thermal | Family::Fluid
        )
    }) {
        let has_a_working = making::EVERY_WORKING
            .iter()
            .any(|working| working.verb == verb.called);

        if has_a_working || verb.done_by.is_some() {
            continue;
        }
        if !declared_and_nothing_more.contains(&verb.called) {
            holes.push(verb.called);
        }
    }

    assert!(
        holes.is_empty(),
        "these are in a working family, nothing performs them and no working \
         turns anything into anything with them - so they are holes rather \
         than declarations: {holes:?}"
    );

    // And the admitted list is not allowed to quietly grow: every name on it
    // has to still be in that state, or it should come off.
    for called in declared_and_nothing_more {
        let verb = EVERY_VERB
            .iter()
            .find(|verb| verb.called == called)
            .unwrap_or_else(|| panic!("{called} is on the list and not in the matrix"));
        let has_a_working = making::EVERY_WORKING
            .iter()
            .any(|working| working.verb == verb.called);
        assert!(
            !has_a_working && verb.done_by.is_none(),
            "{called} is owned up to as doing nothing and now does something - \
             take it off the list"
        );
    }
}

/// A verb with a working is a verb the matrix says somebody performs.
///
/// The other half of the same coin, and the one that actually bit: `split` and
/// `drill` were `done_by: None`, so a working added under either would have
/// been unreachable through the matrix however good it was.
#[test]
fn a_verb_with_a_working_has_a_performer() {
    for working in making::EVERY_WORKING {
        let Some(verb) = EVERY_VERB.iter().find(|verb| verb.called == working.verb) else {
            continue;
        };
        assert!(
            verb.done_by.is_some(),
            "{} works {} into {}, and the matrix says nobody performs {}",
            working.verb,
            working.to,
            working.makes,
            verb.called
        );
    }
}

/// One person with something to work on.
fn somebody_holding(what: &str, how_many: u32) -> crate::analytics::Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = crate::analytics::Simulation::new(world, population);

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 1.0));
    simulation
}

/// The three new processes each turn one thing into another.
///
/// Not "the action succeeds" - the product has to arrive in the pack, which
/// is the difference between a process and a turn spent.
#[test]
fn the_new_workings_turn_one_thing_into_another() {
    for (verb, from, into) in [
        ("grind", "nuts", "nutmeal"),
        ("split", "wood", "staves"),
        ("drill", "antler", "needle"),
    ] {
        let mut simulation = somebody_holding(from, 6);

        let result = simulation.execute_action(
            &Action::Work {
                verb: verb.to_string(),
                to: from.to_string(),
            },
            0,
        );

        assert!(
            result.success,
            "{verb} a {from} was refused: {:?}",
            result.message
        );
        assert!(
            simulation.population.agents[0].how_many_i_have(into) > 0,
            "{verb} a {from} succeeded and no {into} arrived"
        );
        assert!(
            simulation.population.agents[0].how_many_i_have(from) < 6,
            "{verb} a {from} made {into} out of nothing"
        );
    }
}

/// And a new working is discoverable without being wired to anything.
///
/// Both routes to it read the tables directly - the curiosity ladder's own
/// rung through `every_working_to_find_out`, and the novelty terminal through
/// the verb matrix - so adding a row is the whole of adding a process. This
/// asserts the first of those, which is the one that needs the working to be
/// marked as something nobody is born knowing.
#[test]
fn a_new_working_is_something_to_be_found_out() {
    for made in ["nutmeal", "staves", "needle"] {
        assert!(
            making::every_working_to_find_out().any(|working| working.makes == made),
            "{made} is made by a working nobody has to discover, so no \
             curious agent will ever stumble on it"
        );
    }
}
