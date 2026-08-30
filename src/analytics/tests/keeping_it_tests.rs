// src/analytics/tests/keeping_it_tests.rs
//! Tests for the three things that were stopping the store from filling.
//!
//! A settlement held **0.56 units** of preserved food through a whole winter.
//! It dried and salted plenty — three hundred and fifteen turns a world went
//! into it — and then ate it the same afternoon, because nothing anywhere
//! preferred the thing that would be lost over the thing that would keep.
//!
//! Hunting sat behind eating what you carry, behind foraging, behind walking
//! to a known patch, behind moving the whole camp, behind walking back to
//! ground that fed you once, and then behind being *desperate* on top of all
//! that. Forty agents in forty-seven believed it paid and none had done any.
//!
//! And a drizzle and a thunderstorm were the same event, because the
//! intensity the weather has always reported was thrown away at the first
//! comparison.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::{Action, WeatherType};
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32, how: PreparationState) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 1.0);
    let mut food = database
        .create_food_data(&of, 0)
        .expect("that is food");
    food.preparation = how;
    meal.food_data = Some(food);
    meal
}

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

// --------------------------------------------------------------------------
// Eating what will be lost, saving what will keep
// --------------------------------------------------------------------------

/// A person eats the thing that is about to go and saves the thing that
/// keeps. This is the whole reason for preserving anything.
#[test]
fn somebody_eats_the_perishable_thing_first() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.inventory.add_item(a_meal(
        ItemType::Meat,
        "meatstrips",
        6,
        PreparationState::Dried,
    ));
    agent.inventory.add_item(a_meal(
        ItemType::Meat,
        "meatportions",
        6,
        PreparationState::Raw,
    ));

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("meatportions"),
        "the raw joint will be gone in a week; the dried strips will not"
    );
}

/// And salted food is saved the same way.
#[test]
fn salted_food_is_saved_too() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.inventory.add_item(a_meal(
        ItemType::Meat,
        "meatstrips",
        6,
        PreparationState::Salted,
    ));
    agent.inventory.add_item(a_meal(ItemType::Food, "berries", 6, PreparationState::Raw));

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("berries")
    );
}

/// In February, when there is nothing else, the store is what there is — and
/// this needs no special case, because there is nothing perishable to prefer.
#[test]
fn in_february_the_store_is_what_there_is() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.inventory.add_item(a_meal(
        ItemType::Meat,
        "meatstrips",
        6,
        PreparationState::Dried,
    ));

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("meatstrips"),
        "there is nothing else to eat"
    );
}

/// "Eat what will be lost first" must not become "eat the rot first". A raw
/// thing on the turn still scores below a raw thing that is fresh.
#[test]
fn nobody_eats_the_rot_first() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    let mut going = a_meal(ItemType::Food, "berries", 6, PreparationState::Raw);
    if let Some(food) = going.food_data.as_mut() {
        food.freshness = 0.35;
    }
    agent.inventory.add_item(going);
    agent.inventory.add_item(a_meal(ItemType::Food, "greens", 6, PreparationState::Raw));

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("greens"),
        "fresh beats nearly-gone, even though nearly-gone will be lost sooner"
    );
}

// --------------------------------------------------------------------------
// The deer at your feet
// --------------------------------------------------------------------------

/// An animal standing right here, with the nearest berry a walk away, is
/// worth turning aside for - by somebody who can bring it down.
///
/// The spear in the pack is not decoration. A deer is bigger than a thrown
/// stone will kill, the executor has said so since hunting was written, and
/// the decision layer now asks the same question before setting out rather
/// than after walking there. See ISSUES_FOUND.md #121.
#[test]
fn a_deer_at_your_feet_beats_a_berry_patch_a_walk_away() {
    use crate::core::DriveType;

    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    // Nothing to pick up anywhere near, and nothing in the pack.
    simulation.world.resources.clear();

    simulation
        .world
        .spawn_animal("deer".to_string(), (here.0 + 2, here.1))
        .expect("a deer should spawn");

    {
        let agent = &mut simulation.population.agents[0];
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.7;
            hunger.weight = 1.0;
            hunger.lean = 1.0;
        }
    }

    // Empty-handed, the deer is not an answer to anything: walking to it buys
    // a refusal at the end of the walk.
    let empty_handed = simulation.food_action(&simulation.population.agents[0], here, false);
    assert!(
        !matches!(empty_handed, Some(Action::Hunt { .. })),
        "nobody runs down a deer by hand: {empty_handed:?}"
    );

    simulation.population.agents[0].inventory.add_item(
        crate::agents::InventoryItem::new_with_weight("spear".to_string(), 1, 2.0),
    );

    let chosen = simulation.food_action(&simulation.population.agents[0], here, false);
    assert!(
        matches!(chosen, Some(Action::Hunt { .. }) | Some(Action::Move { .. })),
        "with a spear, a deer two paces off and nothing else to eat: {chosen:?}"
    );
}

/// And a deer across the valley is not. That is the expedition that does not
/// pay and never did.
#[test]
fn a_deer_across_the_valley_is_not_worth_it() {
    use crate::core::DriveType;

    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.clear();
    simulation
        .world
        .spawn_animal("deer".to_string(), (here.0 + 15, here.1))
        .expect("a deer should spawn");

    {
        let agent = &mut simulation.population.agents[0];
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.7;
            hunger.weight = 1.0;
            hunger.lean = 1.0;
        }
    }

    let chosen = simulation.food_action(&simulation.population.agents[0], here, false);

    assert!(
        !matches!(chosen, Some(Action::Hunt { .. })),
        "fifteen paces is a morning's walk, not a meal: {chosen:?}"
    );
}

/// Somebody who has thrown at animals and hit none stops throwing. The
/// learning gate is the one that already existed.
#[test]
fn a_hunter_who_never_catches_anything_stops_trying() {
    use crate::core::DriveType;

    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.clear();
    simulation
        .world
        .spawn_animal("deer".to_string(), (here.0 + 2, here.1))
        .expect("a deer should spawn");

    {
        let agent = &mut simulation.population.agents[0];
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.7;
            hunger.weight = 1.0;
            hunger.lean = 1.0;
        }
        // A long record of throwing and missing.
        for _ in 0..40 {
            agent.lessons.record_particular("hunt", false);
        }
    }

    let mut hunted = 0;
    for _ in 0..40 {
        if matches!(
            simulation.food_action(&simulation.population.agents[0], here, false),
            Some(Action::Hunt { .. })
        ) {
            hunted += 1;
        }
    }

    assert!(
        hunted < 20,
        "forty misses is enough to put anybody off: hunted {hunted} times in 40"
    );
}

// --------------------------------------------------------------------------
// How hard it is coming down
// --------------------------------------------------------------------------

/// A drizzle and a thunderstorm are not the same event.
#[test]
fn a_downpour_is_worse_than_a_drizzle() {
    let how_much_is_left = |sky: WeatherType| {
        let mut simulation = one_person();
        let where_it_is = Position::new(30, 30);

        simulation.world.somebody_left_this(
            a_meal(ItemType::Fish, "fish", 8, PreparationState::Raw),
            where_it_is,
            0,
        );

        for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 3) {
            simulation.world.climate.weather.weather_type = sky;
            simulation.world.climate.weather.duration_remaining = u32::MAX;
            simulation.world.tick();
        }

        simulation
            .world
            .what_is_lying_at(&where_it_is)
            .first()
            .and_then(|left| left.item.food_data.as_ref())
            .map(|food| food.freshness)
            .unwrap_or(0.0)
    };

    let drizzled = how_much_is_left(WeatherType::LightRain);
    let drowned = how_much_is_left(WeatherType::HeavyRain);

    assert!(
        drizzled > drowned,
        "three days of drizzle should leave more than three days of downpour: \
         {drizzled} against {drowned}"
    );
}

/// And a roof keeps the rain off what is under it.
#[test]
fn a_roof_keeps_the_rain_off() {
    let how_much_is_left = |under_a_roof: bool| {
        let mut simulation = one_person();
        let where_it_is = Position::new(30, 30);

        if under_a_roof {
            simulation.world.add_building_at(
                crate::world::BuildingType::SkinTent,
                (where_it_is.x, where_it_is.y, 0),
            );
        }

        simulation.world.somebody_left_this(
            a_meal(ItemType::Fish, "fish", 8, PreparationState::Raw),
            where_it_is,
            0,
        );

        for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 3) {
            simulation.world.climate.weather.weather_type = WeatherType::HeavyRain;
            simulation.world.climate.weather.duration_remaining = u32::MAX;
            simulation.world.tick();
        }

        simulation
            .world
            .what_is_lying_at(&where_it_is)
            .first()
            .and_then(|left| left.item.food_data.as_ref())
            .map(|food| food.freshness)
            .unwrap_or(0.0)
    };

    assert!(
        how_much_is_left(true) > how_much_is_left(false),
        "that is what a roof is for"
    );
}
