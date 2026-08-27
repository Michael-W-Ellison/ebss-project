// src/analytics/tests/portion_tests.rs
//! Tests for a carcass having to come apart before it is supper.
//!
//! "How are agents eating meat? Are they cooking it first? Can they just
//! absorb an entire side of beef? Should they not have to cut it into smaller
//! pieces so they can cook and eat it?"
//!
//! They could, they were not, and they should. A kill dropped two-kilo lumps
//! of `meat` and one `Eat` swallowed one of them raw. There was nothing in
//! this model between the animal and the mouth.
//!
//! A carcass is now whole until somebody takes a knife to it: it cannot be
//! eaten and it cannot be put over a fire. Everybody is born knowing it comes
//! apart - there is nothing to discover about a joint of meat - but knowing it
//! is not the same as having an edge to do it with.

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::making;
use crate::world::nutrition::{FoodDatabase, Piece, PreparationState};
use crate::world::{ItemType, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 2.0);
    meal.food_data = database.create_food_data(&of, 0);
    meal
}

/// One agent, nothing in the pack, standing still.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
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

/// Put an edge in somebody's pack, so `cut` is a thing they can actually do.
fn give_them_a_knife(agent: &mut Agent) {
    let knife = making::what_helps_with(crate::agents::SkillType::Leatherworking)
        .next()
        .expect("something in this world cuts");
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight(knife.called.to_string(), 1, 0.3));
}

// --------------------------------------------------------------------------
// How big a piece is
// --------------------------------------------------------------------------

/// Flesh off a kill is whole until somebody cuts it.
#[test]
fn a_carcass_is_whole_and_a_joint_is_not() {
    assert_eq!(Piece::of("meat"), Piece::Whole);
    assert_eq!(Piece::of("fish"), Piece::Whole);
    assert_eq!(Piece::of("meatportions"), Piece::Portion);
    assert_eq!(Piece::of("fishportions"), Piece::Portion);
    assert_eq!(Piece::of("meatstrips"), Piece::Strip);
    assert_eq!(Piece::of("fishstrips"), Piece::Strip);
}

/// A berry is already the size of a mouthful. Nobody butchers a berry, and
/// nothing has to be cut down before it will dry: a berry has the bulk of a
/// mouthful and the thickness of nothing.
#[test]
fn what_comes_off_a_bush_needs_no_butchering() {
    for already_small in ["berries", "food", "greens", "roots", "grain"] {
        assert_eq!(
            Piece::of(already_small),
            Piece::Small,
            "{already_small} arrives at about the size of a mouthful"
        );
        assert!(Piece::of(already_small).can_it_be_eaten());
        assert_eq!(
            Piece::of(already_small).how_long_it_takes_to_dry(),
            Piece::Strip.how_long_it_takes_to_dry(),
            "and dries as fast as anything cut thin"
        );
    }
}

/// Cooking a thing does not put it back together.
#[test]
fn a_cooked_joint_is_still_a_joint() {
    assert_eq!(Piece::of("cooked_meatportions"), Piece::Portion);
    assert_eq!(Piece::of("cooked_meat"), Piece::Whole);
}

/// A whole beast goes neither in the mouth nor on the fire.
#[test]
fn a_whole_beast_can_be_neither_eaten_nor_cooked() {
    assert!(!Piece::Whole.can_it_be_eaten());
    assert!(!Piece::Whole.can_it_be_cooked());
    assert_eq!(Piece::Whole.how_many_fit_over_a_fire(), 0);

    assert!(Piece::Portion.can_it_be_eaten());
    assert!(Piece::Portion.can_it_be_cooked());
    assert!(Piece::Strip.can_it_be_eaten());
    assert!(Piece::Strip.can_it_be_cooked());
}

/// Cut small and more of it is ready at the end of the same turn.
#[test]
fn smaller_pieces_cook_more_at_once() {
    assert!(
        Piece::Strip.how_many_fit_over_a_fire() > Piece::Portion.how_many_fit_over_a_fire(),
        "a handful of strips goes over a fire where one joint does"
    );
}

/// And dry faster. This is the whole reason anybody cuts a thing into strips
/// rather than just quartering it.
#[test]
fn smaller_pieces_dry_faster() {
    assert!(
        Piece::Strip.how_long_it_takes_to_dry() < Piece::Portion.how_long_it_takes_to_dry(),
        "a strip is dry in days and a joint takes most of a week"
    );
    assert_eq!(
        Piece::Whole.how_long_it_takes_to_dry(),
        u32::MAX,
        "a whole beast in the sun does not dry, it turns"
    );
}

/// What the weather does to it follows from that: cut flesh dries, whole
/// flesh rots.
#[test]
fn the_sun_dries_what_has_been_cut_and_not_what_has_not() {
    assert!(!World::will_this_dry("meat"), "a whole beast turns");
    assert!(!World::will_this_dry("fish"), "and so does a whole fish");
    assert!(World::will_this_dry("meatportions"));
    assert!(World::will_this_dry("fishstrips"));
}

// --------------------------------------------------------------------------
// Cutting it up
// --------------------------------------------------------------------------

/// A carcass comes apart, and everybody knows it does.
#[test]
fn taking_a_carcass_apart_is_not_a_discovery() {
    for (verb, to) in [("cut", "meat"), ("cut", "fish")] {
        let working = making::how_to_work(verb, to)
            .unwrap_or_else(|| panic!("{to} should come apart"));
        assert!(
            working.obvious,
            "there is nothing to find out about a carcass coming apart"
        );
        assert!(working.makes.ends_with("portions"));
    }
}

/// And a joint comes apart further, which is a separate step off a separate
/// thing: you quarter the deer first and cut the joint down after.
#[test]
fn strips_come_off_a_joint_rather_than_off_the_animal() {
    let working = making::how_to_work("cut", "meatportions").expect("a joint cuts down");
    assert_eq!(working.makes, "meatstrips");

    let fish = making::how_to_work("cut", "fishportions").expect("and so does a fillet");
    assert_eq!(fish.makes, "fishstrips");
}

/// Whatever came off the knife is still the animal it came off, so the rest
/// of the model can price it, cook it and store it.
#[test]
fn a_cut_piece_is_still_the_thing_it_was_cut_off() {
    use crate::agents::storage_integration::id_to_item_type;

    assert_eq!(id_to_item_type("meatportions"), Some(ItemType::Meat));
    assert_eq!(id_to_item_type("meatstrips"), Some(ItemType::Meat));
    assert_eq!(id_to_item_type("fishportions"), Some(ItemType::Fish));
    assert_eq!(id_to_item_type("fishstrips"), Some(ItemType::Fish));
}

// --------------------------------------------------------------------------
// What an agent does about it
// --------------------------------------------------------------------------

/// Nobody eats a deer.
#[test]
fn a_whole_carcass_is_not_a_meal() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));

    assert!(
        agent.find_best_food_to_eat().is_none(),
        "four whole carcasses and nothing to eat"
    );
    assert!(!agent.has_edible_food());
}

/// A joint is.
#[test]
fn a_joint_is_a_meal() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 4));

    assert_eq!(
        agent.find_best_food_to_eat().as_deref(),
        Some("meatportions")
    );
}

/// The executor holds the line too, whatever route the carcass arrived by.
#[test]
fn the_eating_itself_refuses_a_carcass() {
    use crate::world::nutrition::EatResult;

    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));

    assert!(matches!(
        agent.eat_food_item("meat", 0),
        EatResult::NoFood
    ));
    assert_eq!(
        agent.how_many_i_have("meat"),
        4,
        "and it is all still there, because nothing was eaten"
    );
}

/// A man with a knife and a deer cuts the deer up.
#[test]
fn somebody_with_an_edge_and_a_carcass_cuts_it_up() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    give_them_a_knife(agent);
    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));

    let (verb, to) = agent
        .what_flesh_i_should_cut_up()
        .expect("a deer and a knife is a job of work");

    assert_eq!(verb, "cut");
    assert_eq!(to, "meat");
}

/// A man with no knife does not choose to cut, because choosing it would
/// spend the turn and come straight back refused.
#[test]
fn somebody_with_no_edge_does_not_try() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));

    assert!(
        agent.what_flesh_i_should_cut_up().is_none(),
        "the matrix wants an edge for `cut`, so asking without one is a wasted turn"
    );
}

/// And a man with a joint already in his pack eats it rather than stopping to
/// quarter the rest of the deer.
#[test]
fn nobody_butchers_the_rest_of_it_while_supper_is_in_hand() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    give_them_a_knife(agent);
    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));
    agent
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 2));

    assert!(agent.what_flesh_i_should_cut_up().is_none());
}

/// Carrion is not worth an edge or a turn.
#[test]
fn nobody_butchers_something_that_has_already_turned() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    give_them_a_knife(agent);

    let mut gone_off = a_meal(ItemType::Meat, "meat", 4);
    if let Some(food) = gone_off.food_data.as_mut() {
        food.freshness = 0.0;
    }
    agent.inventory.add_item(gone_off);

    assert!(agent.what_flesh_i_should_cut_up().is_none());
}

/// End to end: a hungry agent with a carcass and a knife takes the carcass
/// apart and ends up with something it can eat.
#[test]
fn a_hungry_agent_with_a_deer_ends_up_with_joints() {
    use crate::core::DriveType;

    let mut simulation = one_person();

    {
        let agent = &mut simulation.population.agents[0];
        give_them_a_knife(agent);
        agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 6));

        // Hungry, and hungry in the way the drive system reads: pinning a
        // value without pinning weight and lean pins nothing at all.
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.9;
            hunger.weight = 1.0;
            hunger.lean = 1.0;
        }
    }

    for _ in 0..40 {
        simulation.tick();
        let agent = &simulation.population.agents[0];
        if agent.how_many_i_have("meatportions") > 0 {
            return;
        }
    }

    panic!(
        "forty ticks with a deer, a knife and an empty stomach and not one joint cut: {:?}",
        simulation.population.agents[0].inventory.get_all_items().keys()
    );
}

// --------------------------------------------------------------------------
// The fire
// --------------------------------------------------------------------------

/// Nothing puts a whole beast over a fire.
#[test]
fn a_carcass_is_not_something_you_put_on_a_fire() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(a_meal(ItemType::Meat, "meat", 4));

    assert!(
        Simulation::cookable_item(&simulation.population.agents[0], "generic").is_none(),
        "the outside chars and the inside stays raw, which is not cooking"
    );
}

/// A joint is.
#[test]
fn a_joint_is_something_you_put_on_a_fire() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 4));

    assert_eq!(
        Simulation::cookable_item(&simulation.population.agents[0], "generic").as_deref(),
        Some("meatportions")
    );
}

/// Cooking does not un-dry a thing that was already dried, and drying is
/// worth twenty times what cooking is - so what is already kept stays kept.
#[test]
fn what_is_already_dried_is_not_put_back_on_the_fire() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    let mut kept = a_meal(ItemType::Meat, "meatstrips", 4);
    if let Some(food) = kept.food_data.as_mut() {
        food.preparation = PreparationState::Dried;
    }
    agent.inventory.add_item(kept);

    assert!(
        Simulation::cookable_item(&simulation.population.agents[0], "generic").is_none(),
        "only raw food goes on a fire"
    );
}
