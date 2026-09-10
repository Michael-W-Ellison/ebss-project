// src/analytics/tests/map_memory_tests.rs
//! **What a place is remembered as is a fact about the rememberer.**
//!
//! A man who knows what clay is for remembers a clay bank; a man who does not
//! remembers that there is something on that bend of the river. Both of them
//! saw the same mud.
//!
//! Two absences behind this, and they are the same one. The sight pass filed
//! food and water and **threw every other resource away** - so
//! `SpatialMemoryType::Resource` has been in the memory since memories were
//! written with nothing to write it, and every making in the model wanted a
//! material that nothing could remember the whereabouts of. And a memory had
//! no name at all: everything was filed under its category, so there was
//! nothing for a craft to ask.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::memory::{SpatialMemory, SpatialMemoryType};
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

/// The name goes before the place does.
///
/// "Flax grows at that field edge" becomes "there is something worth having in
/// that valley" becomes nothing. One clock and two thresholds, so a memory
/// cannot be sure what was there and unsure where.
#[test]
fn what_it_was_is_forgotten_before_where_it_was() {
    let mut remembered = SpatialMemory::new(SpatialMemoryType::Resource, (30, 25, 0), 0);
    remembered.what_it_is = Some("clay".to_string());

    assert_eq!(
        remembered.what_i_could_name_it(),
        Some("clay"),
        "fresh, and he cannot say what he saw"
    );

    // Faded past the name but not past the place.
    remembered.confidence = SpatialMemory::STILL_KNOWS_WHAT_IT_WAS - 0.01;
    assert_eq!(
        remembered.what_i_could_name_it(),
        None,
        "he still names it long after he could tell one bank from another"
    );
    assert!(
        remembered.confidence > 0.3,
        "the place should outlast the name, not go with it"
    );
}

/// And a name he never had is not one he loses.
#[test]
fn a_place_can_be_remembered_without_a_name() {
    let remembered = SpatialMemory::new(SpatialMemoryType::Resource, (30, 25, 0), 0);
    assert_eq!(remembered.what_i_could_name_it(), None);
    assert_eq!(remembered.confidence, 1.0);
}

/// **A man remembers what he knows the use of.**
///
/// The competence rule. `is_a_familiar_thing` is what anybody is born
/// knowing - a stone you can knap, a stick you can sharpen - and
/// `knows_how_to` is what this one has since worked out, so what a man can
/// name grows with his craft.
#[test]
fn knowing_the_use_of_a_thing_is_what_lets_him_name_it() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    assert!(
        agent.do_i_know_what_this_is_for("stone"),
        "a stone is a thing anybody is born knowing the use of"
    );
    assert!(
        agent.do_i_know_what_this_is_for("wood"),
        "so is a stick"
    );
    assert!(
        !agent.do_i_know_what_this_is_for("a thing nobody has ever heard of"),
        "he named something the model has no use for at all"
    );
}

/// **He notices where the stone is.**
///
/// The writer `SpatialMemoryType::Resource` never had. Until now the sight
/// pass filed food and water and returned `None` for everything else, so no
/// making in this model could ever be told where to fetch its material.
#[test]
fn what_is_not_food_is_something_a_man_notices_now() {
    let mut simulation = one_person();

    let before = simulation.population.agents[0]
        .memory
        .recall_locations(SpatialMemoryType::Resource)
        .len();
    assert_eq!(before, 0, "the fixture starts him with nothing remembered");

    let mut world = std::mem::replace(&mut simulation.world, World::new(WorldConfig::default()));
    simulation.population.process_exploration_with_world(&mut world);
    simulation.world = world;

    let noticed = simulation.population.agents[0]
        .memory
        .recall_locations(SpatialMemoryType::Resource);

    assert!(
        !noticed.is_empty(),
        "he stood in a world full of stone and timber and remembered none of it"
    );
}

/// **A place he has no use for is worth less than one he has.**
///
/// The retention half of the specificity rule, and the half that costs
/// nothing. A man who cannot name what was there loses it a band faster and
/// goes over the side first when the shelf is full - and nobody walks
/// anywhere on the strength of it.
#[test]
fn a_place_he_can_name_is_kept_longer_than_one_he_cannot() {
    let mut named = SpatialMemory::new(SpatialMemoryType::Resource, (30, 25, 0), 0);
    named.what_it_is = Some("clay".to_string());

    let nameless = SpatialMemory::new(SpatialMemoryType::Resource, (31, 25, 0), 0);

    assert!(
        named.what_forgetting_this_would_cost().decay_multiplier()
            < nameless.what_forgetting_this_would_cost().decay_multiplier(),
        "a bank of clay he knows the use of fades no slower than a rock he does not"
    );

    let mut named_after = named.clone();
    let mut nameless_after = nameless.clone();
    named_after.forget_a_little(300);
    nameless_after.forget_a_little(300);

    assert!(
        named_after.confidence > nameless_after.confidence,
        "after a week he remembers the nameless place as well as the clay"
    );
}

/// And the name never lifts a place above its own kind.
///
/// Knowing what was in a pit does not make it more than a store, and not
/// knowing does not make a store into a bush. The type sets the band; the
/// name decides whether he keeps the whole of it.
#[test]
fn a_name_does_not_promote_a_place_past_what_it_is() {
    let mut bush = SpatialMemory::new(SpatialMemoryType::Food, (30, 25, 0), 0);
    bush.what_it_is = Some("berries".to_string());

    let pit = SpatialMemory::new(SpatialMemoryType::Storage, (31, 25, 0), 0);

    assert!(
        pit.what_forgetting_this_would_cost().decay_multiplier()
            < bush.what_forgetting_this_would_cost().decay_multiplier(),
        "a nameless winter store fell behind a named berry bush"
    );
}
