// src/analytics/tests/weather_tests.rs
//! Tests for what the sky does to food left lying in it, and for the one
//! thing anybody here can learn from watching.
//!
//! Rain rots anything. Sun dries what is thin enough to dry — strips cut off
//! a carcass, berries — and rots what is not: a whole fish left out in the
//! sun is carrion by evening, and the same fish cut down and laid out keeps
//! for a season.
//!
//! That difference is the whole of what a people at this stage can find out
//! about preserving anything, and the world teaches it rather than anything
//! written down. Nobody is born knowing it. Somebody has to put something
//! down and come back to it.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::making;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::{Action, WeatherType};
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32, made_at: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 0.5);
    meal.food_data = database.create_food_data(&of, made_at);
    meal
}

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    // And no roof over anything. A default world puts one building at the
    // middle of the map, which is exactly where these tests stand somebody,
    // and food under a roof deliberately does not dry - see
    // `what_is_lying_about_weathers`. Tests about the sky should not be
    // quietly testing the eaves.
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

fn set_the_sky(simulation: &mut Simulation, to: WeatherType) {
    simulation.world.climate.weather.weather_type = to;

    // And hold it there. `World::tick` runs the climate before it runs the
    // weathering pass, so a sky set here and not pinned gets rolled again
    // before anything lying on the ground sees it - which had these tests
    // failing about one run in three on a sky that was never actually the one
    // they asked for.
    simulation.world.climate.weather.duration_remaining = u32::MAX;
}

/// Leave a thing on the ground under a given sky for a given number of days,
/// holding the weather steady throughout.
fn leave_it_out(
    simulation: &mut Simulation,
    item: InventoryItem,
    where_it_is: Position,
    sky: WeatherType,
    days: u32,
) {
    simulation
        .world
        .somebody_left_this(item, where_it_is, simulation.world.tick);

    for _ in 0..(TICKS_PER_DAY * days) {
        set_the_sky(simulation, sky);
        simulation.world.tick();
    }
}

fn what_is_left(simulation: &Simulation, where_it_is: Position) -> Option<(PreparationState, f32)> {
    simulation
        .world
        .what_is_lying_at(&where_it_is)
        .first()
        .and_then(|left| left.item.food_data.as_ref())
        .map(|food| (food.preparation, food.freshness))
}

// --------------------------------------------------------------------------
// Cutting it up
// --------------------------------------------------------------------------

/// A fish comes apart into joints, and a joint comes apart into strips.
/// Anybody with an edge works both out.
#[test]
fn a_fish_can_be_cut_into_strips() {
    let quartering = making::how_to_work("cut", "fish").expect("a fish comes apart");
    assert_eq!(quartering.makes, "fishportions");
    assert!(
        quartering.obvious,
        "there is nothing to discover about a fish coming apart"
    );

    let cutting = making::how_to_work("cut", "fishportions").expect("and a joint cuts down");
    assert_eq!(cutting.makes, "fishstrips");
}

/// And so does a carcass.
#[test]
fn meat_can_be_cut_into_strips() {
    let quartering = making::how_to_work("cut", "meat").expect("a carcass comes apart");
    assert_eq!(quartering.makes, "meatportions");

    let cutting = making::how_to_work("cut", "meatportions").expect("and a joint cuts down");
    assert_eq!(cutting.makes, "meatstrips");
}

/// Strips are thin enough to dry through. A whole fish is not.
#[test]
fn what_is_thin_enough_to_dry() {
    assert!(World::will_this_dry("fishstrips"));
    assert!(World::will_this_dry("meatstrips"));
    assert!(World::will_this_dry("food"), "a berry is mostly skin");

    assert!(
        !World::will_this_dry("fish"),
        "the outside dries and the inside goes on being a fish"
    );
    assert!(!World::will_this_dry("meat"));
}

// --------------------------------------------------------------------------
// What the sun does
// --------------------------------------------------------------------------

/// The specification's own example: strips of fish left in the sun dry out
/// and fail to rot.
#[test]
fn cut_fish_left_in_the_sun_dries() {
    let mut simulation = one_person();
    let where_it_is = Position::new(30, 30);

    leave_it_out(
        &mut simulation,
        a_meal(ItemType::Fish, "fishstrips", 6, 0),
        where_it_is,
        WeatherType::Clear,
        4,
    );

    let (how, freshness) = what_is_left(&simulation, where_it_is).expect("still there");

    assert_eq!(how, PreparationState::Dried);
    assert!(freshness > 0.5, "and it kept: {freshness:.2}");
}

/// And the same fish left whole in the sun rots.
#[test]
fn a_whole_fish_left_in_the_sun_rots() {
    let mut simulation = one_person();
    let where_it_is = Position::new(30, 30);

    leave_it_out(
        &mut simulation,
        a_meal(ItemType::Fish, "fish", 6, 0),
        where_it_is,
        WeatherType::Clear,
        4,
    );

    match what_is_left(&simulation, where_it_is) {
        None => {} // gone entirely, which is the point
        Some((how, freshness)) => {
            assert_ne!(how, PreparationState::Dried, "a whole fish does not dry");
            assert!(
                freshness < 0.5,
                "four days of sun on a whole fish: {freshness:.2}"
            );
        }
    }
}

/// Berries left in the sun dry too.
#[test]
fn berries_left_in_the_sun_dry() {
    let mut simulation = one_person();
    let where_it_is = Position::new(30, 30);

    leave_it_out(
        &mut simulation,
        a_meal(ItemType::Food, "food", 8, 0),
        where_it_is,
        WeatherType::Clear,
        4,
    );

    let (how, _) = what_is_left(&simulation, where_it_is).expect("still there");

    assert_eq!(how, PreparationState::Dried);
}

/// Anything being rained on starts to rot, cut up or not.
#[test]
fn rain_rots_even_what_would_have_dried() {
    let mut simulation = one_person();
    let where_it_is = Position::new(30, 30);

    leave_it_out(
        &mut simulation,
        a_meal(ItemType::Fish, "fishstrips", 6, 0),
        where_it_is,
        WeatherType::Rain,
        4,
    );

    match what_is_left(&simulation, where_it_is) {
        None => {}
        Some((how, _)) => assert_ne!(
            how,
            PreparationState::Dried,
            "nothing dries in the rain"
        ),
    }
}

/// A wet fortnight is a fortnight nothing gets preserved, and the drying that
/// was done before it is not undone.
#[test]
fn rain_stops_the_drying_without_undoing_it() {
    let mut simulation = one_person();
    let where_it_is = Position::new(30, 30);

    simulation.world.somebody_left_this(
        a_meal(ItemType::Fish, "fishstrips", 6, 0),
        where_it_is,
        simulation.world.tick,
    );

    // A day of sun, then a day of rain, then sun again
    for (sky, days) in [
        (WeatherType::Clear, 1),
        (WeatherType::Rain, 1),
        (WeatherType::Clear, 2),
    ] {
        for _ in 0..(TICKS_PER_DAY * days) {
            set_the_sky(&mut simulation, sky);
            simulation.world.tick();
        }
    }

    let (how, _) = what_is_left(&simulation, where_it_is).expect("still there");

    assert_eq!(
        how,
        PreparationState::Dried,
        "the sun before and after the shower still adds up"
    );
}

// --------------------------------------------------------------------------
// Somebody sees it
// --------------------------------------------------------------------------

/// The whole point of doing it this way round: an agent standing near enough
/// to watch a thing dry out learns what dried it.
#[test]
fn whoever_is_standing_near_learns_what_the_sun_did() {
    let mut simulation = one_person();
    let where_it_is = Position::new(25, 25);

    // This used to be where somebody found out that laying food out keeps it.
    // Everybody is born knowing that now - see
    // `Agent::what_anybody_is_born_knowing` - so what is still taken from
    // watching it happen is what it was worth, which is a lesson rather than
    // a discovery.
    let before = simulation.population.agents[0].lessons.tried_this("dry");

    simulation.world.somebody_left_this(
        a_meal(ItemType::Fish, "fishstrips", 6, 0),
        where_it_is,
        simulation.world.tick,
    );

    for _ in 0..(TICKS_PER_DAY * 4) {
        set_the_sky(&mut simulation, WeatherType::Clear);
        simulation.world.tick();
        simulation.who_saw_that_dry();
    }

    let after = simulation.population.agents[0].lessons.tried_this("dry");

    assert!(
        after > before,
        "he was standing over it while it happened: {before} then {after}"
    );
}

/// And somebody across the map does not.
#[test]
fn nobody_across_the_map_learns_anything() {
    let mut simulation = one_person();
    simulation.population.agents[0].state.position = (70, 70, 0);

    simulation.world.somebody_left_this(
        a_meal(ItemType::Fish, "fishstrips", 6, 0),
        Position::new(10, 10),
        simulation.world.tick,
    );

    for _ in 0..(TICKS_PER_DAY * 4) {
        set_the_sky(&mut simulation, WeatherType::Clear);
        simulation.world.tick();
        simulation.who_saw_that_dry();
    }

    assert_eq!(
        simulation.population.agents[0].lessons.tried_this("dry"),
        0,
        "he was forty tiles away and took nothing from it"
    );
}

/// Laying food out to keep it is not a discovery: everybody has always known
/// that a thing left in the sun goes hard rather than green. See
/// `Agent::what_anybody_is_born_knowing` and ISSUES_FOUND.md #125.
#[test]
fn anybody_can_dry_a_thing_without_being_shown_first() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 6, 0));

    let straight_off = simulation.execute_action(
        &Action::Dry {
            what: "meatportions".to_string(),
        },
        0,
    );

    assert!(
        straight_off.success,
        "a person knows what the sun does to a thing left out in it: {:?}",
        straight_off.message
    );

    // And it is not a one-off: the second thing laid out works the same way,
    // because there was never a thing to be found out in the first place.
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 6, 0));

    let again = simulation.execute_action(
        &Action::Dry {
            what: "fishstrips".to_string(),
        },
        0,
    );

    assert!(
        again.success,
        "nothing had to be learned in between: {:?}",
        again.message
    );
}

// --------------------------------------------------------------------------
// Minding it
// --------------------------------------------------------------------------

/// Food going off in your own hands is a threat to the drive that kills you
/// soonest, and it should feel like one.
#[test]
fn watching_your_supper_turn_is_worrying() {
    let mut simulation = one_person();

    let before = simulation.population.agents[0].emotions.fear;
    simulation.population.agents[0].watched_food_go_off("fish", 6);
    let after = simulation.population.agents[0].emotions.fear;

    assert!(
        after > before,
        "it is the next meal that has been lost, not this one: {after} against {before}"
    );
}

/// And losing a basketful is worse than losing a mouthful, up to a point.
#[test]
fn losing_more_is_worse_but_not_endlessly() {
    let mut simulation = one_person();

    let worry_over = |how_much: u32| {
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        let mut alone = Simulation::new(World::new(WorldConfig::default()), population);
        alone.population.agents[0].traits.traits.clear();
        alone.population.agents[0].watched_food_go_off("fish", how_much);
        alone.population.agents[0].emotions.fear
    };

    assert!(worry_over(8) > worry_over(1), "a basketful is worse");
    assert!(
        worry_over(400) <= worry_over(8) * 4.0,
        "and it stops short of paralysing"
    );

    let _ = &mut simulation;
}

/// It also goes on the record: whatever this agent was doing with that food,
/// it did not work.
#[test]
fn food_going_off_is_something_to_learn_from() {
    let mut simulation = one_person();

    simulation.population.agents[0].watched_food_go_off("fish", 4);

    assert!(
        simulation.population.agents[0].lessons.tried_this("keeping food") > 0,
        "it is a thing that happened"
    );
}
