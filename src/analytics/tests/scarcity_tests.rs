// src/analytics/tests/scarcity_tests.rs
//! Tests for a world where food has to be worked for, and for a people that
//! does not take more of it than it will eat.
//!
//! Two halves of one problem, and they were measured separately because doing
//! them at once would have confounded each other.
//!
//! The supply half: a settlement with the crudest tools in the model buried
//! **four years' eating** (ISSUES_FOUND #43), which means food was too easy to
//! come by. A wild bush carried up to sixty units whatever ground it stood on,
//! and nothing in the fauna module knew that agents existed except the predator
//! pass — so a deer stood where it stood while a settlement walked up to it.
//!
//! The demand half: correcting resource clustering (#49) put the world's
//! resources back to what the config had always asked for and **cost eight
//! points of efficiency**, because doubling what there is to gather does not
//! double what anybody eats — it doubles what rots in a pack. #43 stopped a
//! people burying more than the camp would eat before winter; nothing asked
//! the same question of a pack.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, ResourceType, World, WorldConfig};

// --------------------------------------------------------------------------
// The supply half
// --------------------------------------------------------------------------

/// A wild hedge is not an orchard.
#[test]
fn a_wild_hedge_is_not_an_orchard() {
    use crate::world::resource_spawning::TerrainResourceMapper;

    let (_, berries) = TerrainResourceMapper::amount_range(ResourceType::Food);
    let (_, timber) = TerrainResourceMapper::amount_range(ResourceType::Wood);

    assert!(
        berries < timber / 2,
        "a bush should carry a great deal less than a wood does: {berries} against {timber}"
    );
    assert!(
        berries <= 24,
        "and a settlement should not be able to live off one: {berries}"
    );
}

/// And what a particular bush carries is what the ground under it carries.
///
/// `regenerate_in_ground` has always capped regrowth on soil fertility. The
/// crop a world *started* with ignored the soil entirely, so a hedge on
/// exhausted ground came up as heavy as one on a river meadow and then shrank
/// towards its real capacity over the following season.
#[test]
fn a_bush_carries_what_the_ground_under_it_carries() {
    let world = World::new(WorldConfig::default());

    let growing: Vec<_> = world
        .resources
        .iter()
        .filter(|resource| resource.resource_type.is_it_grown())
        .collect();

    assert!(!growing.is_empty(), "a world should have something growing in it");

    for patch in &growing {
        let at_its_best = patch.standing_capacity(1.0);
        assert!(
            patch.amount <= at_its_best,
            "a {:?} patch carries {} where the richest ground would carry {at_its_best}",
            patch.resource_type,
            patch.amount
        );
    }

    // And the ground is not all the same, so neither is the crop
    let heaviest = growing.iter().map(|patch| patch.amount).max().unwrap_or(0);
    let lightest = growing.iter().map(|patch| patch.amount).min().unwrap_or(0);

    assert!(
        heaviest > lightest,
        "some ground should carry more than other ground: {lightest} to {heaviest}"
    );
}

/// A seam of clay does not care how rich the topsoil over it is.
#[test]
fn what_is_dug_out_of_the_ground_does_not_grow_in_it() {
    assert!(!ResourceType::Clay.is_it_grown());
    assert!(!ResourceType::Stone.is_it_grown());
    assert!(!ResourceType::Water.is_it_grown());
    assert!(ResourceType::Food.is_it_grown());
    assert!(ResourceType::Grain.is_it_grown());
}

// --------------------------------------------------------------------------
// The demand half
// --------------------------------------------------------------------------

fn a_man_with(how_much_food: u32) -> Simulation {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();

    if how_much_food > 0 {
        let database = FoodDatabase::new();
        let mut fish = InventoryItem::new_with_weight("fish".to_string(), how_much_food, 0.1);
        fish.food_data = database.create_food_data(&ItemType::Fish, 0);
        let _ = simulation.population.agents[0].inventory.add_item(fish);
    }

    simulation
}

/// A pack already fuller of food than anybody will get through is a reason to
/// stop taking food.
#[test]
fn a_full_pack_is_a_reason_to_stop_taking_food() {
    let empty_handed = a_man_with(0);
    assert!(
        !Simulation::more_food_than_he_will_get_through(&empty_handed.population.agents[0]),
        "a man with nothing about him has room for supper"
    );

    let loaded = a_man_with(Simulation::what_a_person_gets_through());
    assert!(
        Simulation::more_food_than_he_will_get_through(&loaded.population.agents[0]),
        "and a man with a fortnight of fish going off on him does not"
    );
}

/// It counts food rather than meals on purpose. A pack of whole fish nobody
/// has taken a knife to is the single largest thing that rots on anybody in
/// this model — 1,398 units in a world against 2,250 of everything foraged put
/// together — and it is food by weight, by bulk and by the smell it gives off.
/// What it is not is supper, and going back to the river for more of it is the
/// mistake this stops.
#[test]
fn a_pack_of_uncut_fish_still_counts_as_a_pack_full_of_food() {
    let simulation = a_man_with(Simulation::what_a_person_gets_through());
    let man = &simulation.population.agents[0];

    assert_eq!(
        man.how_many_meals_i_have(),
        0,
        "not one of them is supper until somebody cuts them up"
    );
    assert!(
        Simulation::more_food_than_he_will_get_through(man),
        "and he should still not go back to the river for more"
    );
}

/// Nobody stands in a river for more fish than he will eat.
#[test]
fn nobody_fishes_with_a_pack_of_fish_going_off() {
    use crate::core::DriveType;

    let mut simulation = a_man_with(Simulation::what_a_person_gets_through());
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 1.0;
    }

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .fishing_action(&simulation.population.agents[0], here)
            .is_none(),
        "he has more fish than he will get through already"
    );
}

/// And a man with an empty pack still goes and gets food, which is the thing
/// this must not break.
#[test]
fn a_hungry_man_with_an_empty_pack_is_not_stopped() {
    let simulation = a_man_with(0);

    assert!(
        !Simulation::more_food_than_he_will_get_through(&simulation.population.agents[0]),
        "nothing here should stand between a hungry man and his supper"
    );
}

/// The cap has to sit above a day's eating and below what a pack holds, or it
/// is either useless or it starves somebody.
///
/// The second assertion used to be `< WHAT_A_HARVEST_TRIP_IS + 4`, which was a
/// numeric slack between two picked numbers and only made sense while both
/// were on the scale of the body this model had before the starvation clock
/// was corrected. What it stood for is that the cap must be reachable and must
/// not exceed what a person can carry, so that is what it says now.
#[test]
fn the_cap_is_a_load_rather_than_a_meal_or_a_cartload() {
    let cap = Simulation::what_a_person_gets_through();
    let a_day = crate::agents::provision::WHAT_A_BODY_EATS_IN_A_DAY;

    assert!(
        cap > Simulation::enough_not_to_open_the_store(),
        "somebody who would not open the store should not be stopped from foraging: \
         {cap} against {}",
        Simulation::enough_not_to_open_the_store()
    );
    assert!(
        cap as f32 > a_day,
        "and it has to be more than a day's eating, or it fires on a man with supper \
         in his bag: {cap} against {a_day:.1}"
    );
    assert!(
        cap < WHAT_A_PACK_HOLDS,
        "and no more than a pack holds, or nothing ever reaches it: {cap} against {WHAT_A_PACK_HOLDS}"
    );
}

/// What a pack holds, for the assertion above.
///
/// `Inventory::default` is twenty slots and a nominal allowance; what an agent
/// can actually carry is worked out from its body and its baskets. Fifty is
/// the figure `Pit::WHAT_A_PIT_TAKES` is written against - "a person carries
/// fifty and a hole in the ground takes six times that" - and it is the one
/// this bound wants.
const WHAT_A_PACK_HOLDS: u32 = 50;
