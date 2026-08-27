// src/analytics/tests/carrying_tests.rs
//! Tests for what a person can actually carry home.
//!
//! `Inventory::add_item` enforces the weight limit and returns `false`.
//! Butchering ignored what it returned. So a deer that came to more than a man
//! could carry was **silently deleted** — every time, counted nowhere, and
//! invisible to the waste ledger built one commit ago. A hunter walked away
//! from three quarters of an animal and the world behaved as though the animal
//! had been that size.
//!
//! Which makes carrying capacity the quiet third term in the whole
//! preservation argument. Rot is the wasted half of a hunt; so is a carcass
//! left in a field because it would not fit in the pack, and until now only
//! the first of those existed. Drying takes the water out, so a hunter who
//! dries a kill before walking home carries more of the animal home: preserving
//! buys carrying capacity as well as time, and they are the same thing seen
//! from different ends.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32, each: f32, how: PreparationState) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, each);
    let mut food = database.create_food_data(&of, 0).expect("that is food");
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
// What a thing weighs
// --------------------------------------------------------------------------

/// Drying takes the water out, and water is most of what meat weighs.
#[test]
fn dried_meat_weighs_less_than_the_meat_it_was() {
    let raw = a_meal(ItemType::Meat, "meatstrips", 10, 2.0, PreparationState::Raw);
    let dried = a_meal(ItemType::Meat, "meatstrips", 10, 2.0, PreparationState::Dried);

    assert!(
        dried.total_weight() < raw.total_weight(),
        "{} against {}",
        dried.total_weight(),
        raw.total_weight()
    );
}

/// Smoking takes out less, and cooking less again.
#[test]
fn the_drier_it_is_the_lighter_it_is() {
    let each = |how| a_meal(ItemType::Meat, "meatstrips", 10, 2.0, how).total_weight();

    assert!(each(PreparationState::Dried) < each(PreparationState::Smoked));
    assert!(each(PreparationState::Smoked) < each(PreparationState::Cooked));
    assert!(each(PreparationState::Cooked) < each(PreparationState::Raw));
}

/// Salt puts back about what it draws out, so salting buys keeping and not
/// carrying. The two preserving verbs are not interchangeable.
#[test]
fn salting_buys_keeping_and_not_carrying() {
    let raw = a_meal(ItemType::Meat, "meatstrips", 10, 2.0, PreparationState::Raw);
    let salted = a_meal(ItemType::Meat, "meatstrips", 10, 2.0, PreparationState::Salted);

    assert_eq!(salted.total_weight(), raw.total_weight());
}

/// A stone is a stone however you look at it.
#[test]
fn what_is_not_food_weighs_what_it_weighs() {
    let stone = InventoryItem::new_with_weight("stone".to_string(), 4, 3.0);
    assert_eq!(stone.total_weight(), 12.0);
}

/// And drying a pack of meat makes the pack lighter, which is the point.
#[test]
fn drying_what_is_in_the_pack_lightens_the_pack() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    agent.inventory.add_item(a_meal(
        ItemType::Meat,
        "meatstrips",
        10,
        2.0,
        PreparationState::Raw,
    ));
    agent.inventory.recalculate_weight();
    let laden = agent.inventory.current_weight;

    if let Some(item) = agent.inventory.get_item_mut("meatstrips") {
        if let Some(food) = item.food_data.as_mut() {
            food.preparation = PreparationState::Dried;
        }
    }
    agent.inventory.recalculate_weight();

    assert!(
        agent.inventory.current_weight < laden,
        "{} against {laden}",
        agent.inventory.current_weight
    );
}

// --------------------------------------------------------------------------
// What will not fit
// --------------------------------------------------------------------------

/// What will not go in the pack stays where it fell, rather than ceasing to
/// exist. This is the whole of it.
#[test]
fn what_will_not_fit_stays_where_it_fell() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    // A deer's worth, well past what anybody carries
    let carcass = vec![a_meal(
        ItemType::Meat,
        "meat",
        200,
        2.0,
        PreparationState::Raw,
    )];

    let left = simulation.into_the_pack_or_on_the_ground(0, carcass, here);

    assert!(left > 0, "two hundred joints went into a pack");
    assert!(
        simulation.population.agents[0].how_many_i_have("meat") > 0,
        "he carried what he could"
    );

    let on_the_ground: u32 = simulation
        .world
        .what_is_lying_at(&here)
        .iter()
        .map(|dropped| dropped.item.quantity)
        .sum();

    assert_eq!(
        on_the_ground + simulation.population.agents[0].how_many_i_have("meat"),
        200,
        "every joint is either in the pack or on the grass, and none of it \
         stopped existing"
    );
}

/// And it is counted, because it is exactly as wasted as a joint that rotted.
#[test]
fn what_will_not_fit_is_counted() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    assert_eq!(simulation.what_would_not_fit_in_the_pack, 0);

    simulation.into_the_pack_or_on_the_ground(
        0,
        vec![a_meal(ItemType::Meat, "meat", 200, 2.0, PreparationState::Raw)],
        here,
    );

    assert!(simulation.what_would_not_fit_in_the_pack > 0);
}

/// A small kill goes in whole and nothing is left.
#[test]
fn a_small_kill_goes_in_whole() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    let left = simulation.into_the_pack_or_on_the_ground(
        0,
        vec![a_meal(ItemType::Meat, "meat", 2, 2.0, PreparationState::Raw)],
        here,
    );

    assert_eq!(left, 0);
    assert!(simulation.world.what_is_lying_at(&here).is_empty());
    assert_eq!(simulation.population.agents[0].how_many_i_have("meat"), 2);
}

/// Somebody with a bag carries more of the same animal home.
#[test]
fn somebody_with_a_bag_carries_more_of_it_home() {
    let carried_home = |bag: bool| {
        let mut simulation = one_person();
        let here = Position::new(25, 25);

        if bag {
            simulation.population.agents[0].inventory.add_item(
                InventoryItem::new_with_weight("leatherbag".to_string(), 1, 0.5),
            );
        }

        simulation.into_the_pack_or_on_the_ground(
            0,
            vec![a_meal(ItemType::Meat, "meat", 200, 2.0, PreparationState::Raw)],
            here,
        );

        simulation.population.agents[0].how_many_i_have("meat")
    };

    assert!(
        carried_home(true) > carried_home(false),
        "{} against {}",
        carried_home(true),
        carried_home(false)
    );
}

// --------------------------------------------------------------------------
// What a bag is
// --------------------------------------------------------------------------

/// A leather bag holds more than a flax basket, and costs an animal and the
/// scraping of a hide.
#[test]
fn a_leather_bag_holds_more_than_a_basket() {
    use crate::agents::Inventory;

    assert!(
        Inventory::WHAT_A_LEATHER_BAG_HOLDS > Inventory::WHAT_A_BASKET_HOLDS,
        "otherwise nobody would ever kill anything for one"
    );

    let sewing = crate::environment::making::how_to_work("weave", "leather")
        .expect("leather sews into a bag");
    assert_eq!(sewing.makes, "leatherbag");
    assert_eq!(
        sewing.hands,
        crate::agents::SkillType::Crafting,
        "sewing is making; the leatherworking is the scraping, one step back"
    );

    // And what actually gates it is the material: a hide is not leather until
    // somebody has taken the hair off it.
    let scraping = crate::environment::making::how_to_work("scrape", "hides")
        .expect("a hide scrapes into leather");
    assert_eq!(scraping.makes, "leather");
    assert_eq!(scraping.hands, crate::agents::SkillType::Leatherworking);
}

/// And both of them actually raise what a person can carry.
#[test]
fn a_bag_raises_what_a_person_can_carry() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    let bare = agent.inventory.effective_max_weight();

    agent
        .inventory
        .add_item(InventoryItem::new_with_weight("basket".to_string(), 1, 0.5));
    let with_a_basket = agent.inventory.effective_max_weight();

    agent.inventory.add_item(InventoryItem::new_with_weight(
        "leatherbag".to_string(),
        1,
        0.5,
    ));
    let with_both = agent.inventory.effective_max_weight();

    assert!(with_a_basket > bare);
    assert!(with_both > with_a_basket);
}

// --------------------------------------------------------------------------
// In the running world
// --------------------------------------------------------------------------

/// A hunter who kills something too big to carry leaves the rest of it in the
/// field, and it is there afterwards.
#[test]
fn a_kill_too_big_to_carry_leaves_meat_in_the_field() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    // Fill the pack first, so nothing at all will go in
    {
        let agent = &mut simulation.population.agents[0];
        let full = agent.inventory.effective_max_weight();
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, full));
        agent.inventory.recalculate_weight();
    }

    simulation.into_the_pack_or_on_the_ground(
        0,
        vec![a_meal(ItemType::Meat, "meat", 12, 2.0, PreparationState::Raw)],
        here,
    );

    let on_the_ground: u32 = simulation
        .world
        .what_is_lying_at(&here)
        .iter()
        .map(|dropped| dropped.item.quantity)
        .sum();

    assert_eq!(on_the_ground, 12, "he could not take a single joint of it");
}
