// src/analytics/tests/bearing_tests.rs
//! Tests for the turning year in the hedgerows.
//!
//! Growth was seasonal from the beginning and what was *standing* was not, so
//! a berry bush that had grown all summer still had its berries on it in
//! February. A settlement that can pick fruit in the snow has no reason to
//! put anything by, no lean season to be lean in, and no use for a store — so
//! every one of the last three batches was building machinery against a
//! scarcity the world never produced.
//!
//! The year, as this world now keeps it: spring gives leaf and shoot and
//! almost no energy in any of it; summer gives the first roots and pods, which
//! is not a harvest; autumn is when everything else comes on at once; and
//! winter gives nothing at all.

use crate::analytics::Simulation;
use crate::agents::{AgentConfig, Population};
use crate::environment::seasons::{Season, TICKS_PER_DAY};
use crate::world::{ItemType, Position, ResourceNode, ResourceType, World, WorldConfig};

fn a_world() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(world, population)
}

fn turn_the_year_to(simulation: &mut Simulation, wanted: Season) {
    for _ in 0..(TICKS_PER_DAY * 400) {
        if simulation.world.climate.current_season() == wanted {
            return;
        }
        simulation.world.tick();
    }
    panic!("the year never reached {wanted:?}");
}

// --------------------------------------------------------------------------
// What bears when
// --------------------------------------------------------------------------

/// Berries are an autumn thing and nothing else.
#[test]
fn berries_bear_in_autumn_only() {
    assert!(ResourceType::Food.is_it_bearing(Season::Fall));

    for barren in [Season::Winter, Season::Spring, Season::Summer] {
        assert!(
            !ResourceType::Food.is_it_bearing(barren),
            "there are no berries in {barren:?}"
        );
    }
}

/// Spring gives leaf and shoot.
#[test]
fn spring_gives_greens() {
    assert!(ResourceType::Greens.is_it_bearing(Season::Spring));
    // And keeps giving them while the ground is growing: spring does not stop
    // giving greens the day summer starts
    assert!(ResourceType::Greens.is_it_bearing(Season::Summer));
    assert!(!ResourceType::Greens.is_it_bearing(Season::Fall));
}

/// Roots are a spring food as much as a summer one.
///
/// Cattail and dandelion are dug when the top growth is young and the root
/// still holds last year's store, which is exactly what makes them worth
/// digging before anything has ripened. What they ask for is legs: a root patch
/// is dug out and does not come back this year.
#[test]
fn roots_come_up_through_the_growing_half() {
    assert!(ResourceType::Roots.is_it_bearing(Season::Spring));
    assert!(ResourceType::Roots.is_it_bearing(Season::Summer));
    assert!(!ResourceType::Roots.is_it_bearing(Season::Fall));
    assert!(!ResourceType::Roots.is_it_bearing(Season::Winter));
}

/// And winter gives nothing whatever.
#[test]
fn nothing_edible_bears_in_winter() {
    for what in [
        ResourceType::Food,
        ResourceType::Grain,
        ResourceType::Greens,
        ResourceType::Roots,
        ResourceType::Honey,
    ] {
        assert!(
            !what.is_it_bearing(Season::Winter),
            "{what:?} should give nothing in winter — that is the whole point of a store"
        );
    }
}

/// A river does not stop being a river in February, and nor does a rock stop
/// being a rock. Only growing things have a season.
#[test]
fn what_does_not_grow_does_not_stop() {
    for what in [
        ResourceType::Water,
        ResourceType::Stone,
        ResourceType::Wood,
        ResourceType::Clay,
    ] {
        for whenever in [Season::Spring, Season::Summer, Season::Fall, Season::Winter] {
            assert!(
                what.is_it_bearing(whenever),
                "{what:?} is not bearing, so it cannot stop"
            );
        }
    }
}

// --------------------------------------------------------------------------
// And it actually tells on the world
// --------------------------------------------------------------------------

/// A bush carrying fruit sheds it once its season goes by.
#[test]
fn what_a_bush_carries_falls_off_out_of_season() {
    let mut simulation = a_world();
    simulation.world.resources.clear();

    let where_it_is = Position::new(25, 25);
    let mut bush = ResourceNode::new(ResourceType::Food, where_it_is, 60);
    bush.amount = 60;
    simulation.world.resources.push(bush);

    turn_the_year_to(&mut simulation, Season::Winter);

    // A good few weeks of winter
    for _ in 0..(TICKS_PER_DAY * 20) {
        simulation.world.tick();
    }

    let still_on_it = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == where_it_is)
        .map(|resource| resource.amount)
        .unwrap_or(0);

    assert_eq!(
        still_on_it, 0,
        "there should be nothing on that bush by midwinter"
    );
}

/// And it does not shed in the season it bears in.
#[test]
fn a_bush_keeps_what_it_carries_in_its_own_season() {
    let mut simulation = a_world();
    simulation.world.resources.clear();

    let where_it_is = Position::new(25, 25);
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        where_it_is,
        60,
    ));

    // Reach autumn first and *then* put the fruit on it - winding the year
    // forward runs it through three seasons in which it bears nothing
    turn_the_year_to(&mut simulation, Season::Fall);
    if let Some(bush) = simulation
        .world
        .resources
        .iter_mut()
        .find(|resource| resource.position == where_it_is)
    {
        bush.amount = 40;
    }

    for _ in 0..(TICKS_PER_DAY * 5) {
        simulation.world.tick();
    }

    let still_on_it = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == where_it_is)
        .map(|resource| resource.amount)
        .unwrap_or(0);

    assert!(
        still_on_it >= 40,
        "autumn is when it bears, and this one lost {} of it",
        40u32.saturating_sub(still_on_it)
    );
}

/// Shedding always takes at least one, so a patch actually empties rather
/// than creeping down by fractions for ever.
#[test]
fn a_nearly_bare_patch_finishes_emptying() {
    let mut bush = ResourceNode::new(ResourceType::Food, Position::new(1, 1), 60);
    bush.amount = 1;

    bush.what_it_carries_falls_off(0.0001);

    assert_eq!(bush.amount, 0);
}

// --------------------------------------------------------------------------
// What a person can do about it
// --------------------------------------------------------------------------

/// Greens and roots are things a person can eat, and the foraging pass knows
/// it. Without this a settlement would starve in spring standing in a meadow.
#[test]
fn greens_and_roots_are_food() {
    assert!(ResourceType::Greens.is_edible());
    assert!(ResourceType::Roots.is_edible());

    let known: Vec<ItemType> = Simulation::edible_resources()
        .into_iter()
        .map(|(_, item)| item)
        .collect();

    assert!(known.contains(&ItemType::Greens));
    assert!(known.contains(&ItemType::Roots));
}

/// Greens are thin stuff: a great deal of what a body needs a little of, and
/// almost no energy at all. That is what makes spring hungry even when the
/// meadows are full.
#[test]
fn greens_are_thin_stuff() {
    use crate::world::nutrition::FoodDatabase;

    let database = FoodDatabase::new();
    let greens = database.get(&ItemType::Greens).expect("in the database");
    let berries = database.get(&ItemType::Food).expect("in the database");

    assert!(
        greens.base_nutrition.energy < berries.base_nutrition.energy,
        "there is nothing to live on in a leaf"
    );
    assert!(
        greens.base_nutrition.micronutrients > berries.base_nutrition.micronutrients,
        "and everything else in it"
    );
}

/// A world has greens and roots standing in it, or spring is a death
/// sentence.
#[test]
fn a_world_has_something_for_spring_and_summer() {
    let simulation = a_world();

    for what in [ResourceType::Greens, ResourceType::Roots] {
        assert!(
            simulation
                .world
                .resources
                .iter()
                .any(|resource| resource.resource_type == what),
            "{what:?} should be growing somewhere"
        );
    }
}
