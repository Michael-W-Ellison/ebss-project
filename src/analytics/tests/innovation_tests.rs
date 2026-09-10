// src/analytics/tests/innovation_tests.rs
//! **One use of a material teaches its siblings.**
//!
//! A man who knaps ordinary stone into a tip is not told that flint takes half
//! as much and holds a finer edge. He works it out, and what lets him work it
//! out is that flint is *the same sort of thing* as what he already knaps.
//!
//! This is the second way of finding something out, and it is not the first.
//! `somebody_notices_something` is an accident: the right things in your pack
//! and a fire in front of you. This one is reasoning - two things a man
//! already knows, put beside each other - and the second of them is **the name
//! on his map**. Until now nothing in the model asked what a remembered place
//! was; knowing where the flint is is the whole of the prompt.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::memory::SpatialMemoryType;
use crate::environment::making::{
    are_they_of_a_kind, what_else_is_like_it, EVERY_FAMILY, KNAPPED_TIP, KNAPPED_TIP_FROM_FLINT,
    LASHING, LASHING_FROM_COTTON,
};
use crate::world::{World, WorldConfig};

fn one_curious_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    if let Some(curiosity) = simulation.population.agents[0]
        .drives
        .get_mut(crate::core::DriveType::Curiosity)
    {
        curiosity.value = 1.0;
    }
    simulation
}

/// A family is a family both ways, and nothing is its own sibling.
#[test]
fn a_likeness_runs_both_ways_and_not_to_itself() {
    assert!(are_they_of_a_kind("stone", "flint"));
    assert!(are_they_of_a_kind("flint", "stone"));
    assert!(!are_they_of_a_kind("stone", "stone"));
    assert!(!are_they_of_a_kind("stone", "hides"));

    for family in EVERY_FAMILY {
        assert!(
            family.len() >= 2,
            "a family of one is not a family: {family:?}"
        );
        for one in family.iter() {
            assert!(
                !what_else_is_like_it(one).any(|kin| kin == *one),
                "{one} was listed as a sibling of itself"
            );
        }
    }
}

/// **The metals are not a family, and that is on purpose.**
///
/// Iron makes a lump and the lump makes a blade: that is a chain, and each
/// link has to be found out on its own terms over a fire. Calling them
/// siblings would hand a settlement bronze for having once picked up a bright
/// stone.
#[test]
fn a_chain_is_not_a_family() {
    assert!(!are_they_of_a_kind("iron", "shinylump"));
    assert!(!are_they_of_a_kind("shinylump", "metalblade"));
}

/// The sibling techniques are things to find out, not things to be born with.
///
/// If they were obvious there would be nothing for the innovation path to do -
/// every founder would already know that flint knaps and cotton twists, and a
/// craft that arrives complete cannot grow.
#[test]
fn the_sibling_technique_is_not_something_anybody_is_born_with() {
    assert!(KNAPPED_TIP.obvious, "knapping ordinary stone is the start");
    assert!(LASHING.obvious, "so is twisting flax");

    assert!(
        !KNAPPED_TIP_FROM_FLINT.obvious,
        "that flint knaps finer is the thing to work out"
    );
    // And cotton is **not** one of them, on the measurement rather than on
    // the principle. Twisting cotton is as good an innovation as knapping
    // flint, but lashing is what fifteen of the steps in this chain want and
    // taking a fibre away from a founding people costs more than the thought
    // is worth: with both removed, settlements out of their first winter fell
    // from 21 of 64 to 15. See ISSUES_FOUND #195.
    //
    // It is the awkward one, because cotton is also the only sibling that
    // lies in the ground - so it is the only one whose name could ever reach
    // a map, and leaving it obvious is what keeps the map arm of the prompt
    // idle. Flint alone was measured free: 219,208 person-days against
    // 215,149, blocks disagreeing in direction, 46 worlds emptied against 48.
    assert!(
        LASHING_FROM_COTTON.obvious,
        "cotton was made a discovery again, and it was measured as too dear"
    );
}

/// **He knows a stone when he sees one, before he knows how to knap it.**
///
/// Recognising a thing and knowing how to work it are two questions, and this
/// is the line between them. Without it the innovation path eats its own tail:
/// he could not name the flint until he knew flint knapping, and he could not
/// work out flint knapping without first noticing there was flint about.
#[test]
fn a_thing_of_a_kind_is_a_thing_he_can_name() {
    let simulation = one_curious_person();
    let agent = &simulation.population.agents[0];

    assert!(
        !agent.knows_how_to(&KNAPPED_TIP_FROM_FLINT),
        "the fixture starts him not knowing flint"
    );
    assert!(
        agent.do_i_know_what_this_is_for("flint"),
        "he could not tell a flint bank from a mudbank, so he can never learn"
    );
}

/// **The end-to-end statement: knowing where the flint is teaches him flint.**
///
/// He has never held a flint. What he has is the job - he knaps ordinary stone
/// - and a place on his map he can put the name "flint" to. That is the whole
/// of the prompt, and it is the first thing in this model ever to read
/// `SpatialMemory::what_it_is`.
///
/// **In a live world this arm never fires**, and the honest place to say so is
/// beside the test that says it works. Flint is made rather than found, so no
/// map ever carries its name: over 8 worlds of 180 days, 5 worked flint
/// knapping out and every one of them was holding the stuff. See
/// ISSUES_FOUND #195. What is asserted here is that the reader is correct, not
/// that the world currently gives it anything to read.
#[test]
fn knowing_where_the_flint_is_is_what_teaches_him_flint() {
    let mut simulation = one_curious_person();

    simulation.population.agents[0].memory.remember_this_here(
        SpatialMemoryType::Resource,
        (30, 25, 0),
        Some("flint".to_string()),
        9,
    );

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("flint"),
        0,
        "he is not to have any flint about him - the map is the whole prompt"
    );

    // Long enough for a tenth-a-turn chance to be all but certain.
    for _ in 0..400 {
        simulation.somebody_puts_two_and_two_together();
        if simulation.population.agents[0].knows_how_to(&KNAPPED_TIP_FROM_FLINT) {
            return;
        }
    }

    panic!("he walked past a flint bank four hundred times and never had the thought");
}

/// And without the flint on his map, he never has the thought at all.
///
/// The control, and the one that says the name is doing the work rather than
/// the curiosity or the passage of time.
#[test]
fn a_man_who_knows_of_no_flint_never_works_flint_out() {
    let mut simulation = one_curious_person();

    for _ in 0..400 {
        simulation.somebody_puts_two_and_two_together();
    }

    assert!(
        !simulation.population.agents[0].knows_how_to(&KNAPPED_TIP_FROM_FLINT),
        "he worked out flint knapping having never seen or heard of flint"
    );
}

/// A place he can no longer put a name to is no prompt either.
///
/// The two tiers of the map memory doing their work: a valley he remembers as
/// worth something does not tell him there is flint in it.
#[test]
fn a_place_he_cannot_name_prompts_nothing() {
    let mut simulation = one_curious_person();

    simulation.population.agents[0].memory.remember_this_here(
        SpatialMemoryType::Resource,
        (30, 25, 0),
        Some("flint".to_string()),
        9,
    );

    // Faded past the name, still well short of forgetting the place.
    for place in simulation.population.agents[0]
        .memory
        .spatial_memories
        .iter_mut()
    {
        place.confidence = 0.5;
    }

    for _ in 0..400 {
        simulation.somebody_puts_two_and_two_together();
    }

    assert!(
        !simulation.population.agents[0].knows_how_to(&KNAPPED_TIP_FROM_FLINT),
        "a valley he knows only as worth-a-look told him it had flint in it"
    );
}

/// A thing in the pack is as good a prompt as a place on the map.
///
/// Picking a stone up and turning it over is at least as good as remembering
/// where it lies, and refusing it would make the rule about maps rather than
/// about likeness.
#[test]
fn a_thing_in_the_pack_prompts_him_too() {
    let mut simulation = one_curious_person();

    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new("flint".to_string(), 2));

    for _ in 0..400 {
        simulation.somebody_puts_two_and_two_together();
        if simulation.population.agents[0].knows_how_to(&KNAPPED_TIP_FROM_FLINT) {
            return;
        }
    }

    panic!("he carried flint about for four hundred turns and never looked at it");
}

/// A thing with no sibling in a job he knows teaches him nothing.
///
/// The likeness is between *materials standing in the same job*, and where
/// there is no likeness there is no thought to have. A lump of iron in the
/// pack is not a stone that flakes and not a fibre that twists; working out
/// what it is for is the accident over a fire that
/// `somebody_notices_something` is, and this pass must not shortcut it.
#[test]
fn a_thing_with_no_likeness_teaches_nothing() {
    use crate::environment::making::SHINY_LUMP;

    let mut simulation = one_curious_person();

    simulation.population.agents[0]
        .inventory
        .add_item(crate::agents::InventoryItem::new("iron".to_string(), 4));
    simulation.population.agents[0].memory.remember_this_here(
        SpatialMemoryType::Resource,
        (30, 25, 0),
        Some("iron".to_string()),
        9,
    );

    for _ in 0..400 {
        simulation.somebody_puts_two_and_two_together();
    }

    assert!(
        !simulation.population.agents[0].knows_how_to(&SHINY_LUMP),
        "he reasoned his way to smelting from a lump of ore, which is the one \
         thing the family table is drawn narrowly to prevent"
    );
}
