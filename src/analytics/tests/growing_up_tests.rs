// src/analytics/tests/growing_up_tests.rs
//! What being young costs, and what it is owed.
//!
//! Three things the lifecycle described and nothing read. The age capability
//! curve was written, hung on nothing, and deleted as dead code in the sweep
//! of #93 — which left a six-year-old carrying what a grown man carried,
//! walking as fast, working as hard and hitting as heavily, on a fifth of his
//! food. The supervision bands have been in `LifeStage`'s own doc comment
//! since the lifecycle was written and nothing ever consulted them. And a
//! parent had no way to hand a hungry child anything short of the sacrifice
//! branch, which waits until somebody is starving.

use crate::agents::{Agent, AgentConfig, InventoryItem, LifeStage, Population};
use crate::agents::agent::{what_a_body_this_age_can_do, what_a_body_this_age_eats};
use crate::agents::emotions::{Relationship, RelationshipType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn a_world() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

fn somebody_of(years: u32, at: (i32, i32, i32)) -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.now_this_many_years_old(years);
    agent.state.position = at;
    agent.state.health = 100.0;
    agent.state.energy = 100.0;
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.recalculate_weight();
    agent.take_up_the_cart();
    agent
}

// --------------------------------------------------------------------------
// The capability curve
// --------------------------------------------------------------------------

/// The specification's table: nothing at one, a fifth at four, all of it from
/// sixteen, and falling away after forty.
#[test]
fn the_curve_is_the_table_the_specification_gives() {
    assert_eq!(what_a_body_this_age_can_do(1), 0.0);
    assert_eq!(what_a_body_this_age_can_do(4), 0.2);
    assert_eq!(what_a_body_this_age_can_do(8), 0.4);
    assert_eq!(what_a_body_this_age_can_do(16), 1.0);
    assert_eq!(what_a_body_this_age_can_do(39), 1.0);

    // And it falls away rather than stopping
    let mut last = 1.0;
    for years in 40..=70 {
        let now = what_a_body_this_age_can_do(years);
        assert!(now <= last, "it should only fall after forty: {years}");
        assert!(now > 0.0, "an old man is not nothing at {years}");
        last = now;
    }
}

/// A child carries less than a grown man, which is what it was not doing.
#[test]
fn a_child_carries_less_than_a_grown_man() {
    let child = somebody_of(6, (0, 0, 0));
    let grown = somebody_of(30, (0, 0, 0));

    assert!(
        child.total_carrying_capacity() < grown.total_carrying_capacity(),
        "a six-year-old's two hands are not a man's: {:.1} against {:.1}",
        child.total_carrying_capacity(),
        grown.total_carrying_capacity()
    );
}

/// And walks slower, and hits softer.
#[test]
fn a_child_walks_slower_and_hits_softer() {
    let child = somebody_of(6, (0, 0, 0));
    let grown = somebody_of(30, (0, 0, 0));

    assert!(
        child.movement_speed() < grown.movement_speed(),
        "short legs cover less ground: {:.2} against {:.2}",
        child.movement_speed(),
        grown.movement_speed()
    );
    assert!(
        child.own_strength() < grown.own_strength(),
        "and a child is not his father in a fight: {:.2} against {:.2}",
        child.own_strength(),
        grown.own_strength()
    );
}

/// An old man is worth less than he was, and the table takes him down to
/// exactly half and no further.
///
/// Which puts a man of sixty-five on the same rung as a ten-year-old - and
/// that is the specification's table rather than an accident of it. It is
/// worth writing down rather than asserting away: the curve's floor is where
/// a boy is on his way up, and the two meet there.
#[test]
fn strength_falls_away_after_forty() {
    let young = somebody_of(30, (0, 0, 0));
    let older = somebody_of(60, (0, 0, 0));
    let old = somebody_of(65, (0, 0, 0));
    let boy = somebody_of(10, (0, 0, 0));

    assert!(older.own_strength() < young.own_strength(), "sixty is not thirty");
    assert!(old.own_strength() < older.own_strength(), "and sixty-five is not sixty");
    assert!(older.own_strength() > boy.own_strength(), "sixty still beats ten");

    assert_eq!(what_a_body_this_age_can_do(65), what_a_body_this_age_can_do(10));
    assert_eq!(what_a_body_this_age_can_do(69), 0.5, "and it stops at half");
}

/// The two age tables answer different questions and must not be confused: one
/// is what a body needs, the other is what it can give.
///
/// A six-year-old eats about a third of a grown share and can do about three
/// tenths of a grown day's work, which is roughly a wash. A two-year-old eats
/// a fifth and can do a tenth, which is not — and that is the shape of a
/// child: it costs more than it brings in until it is grown.
#[test]
fn a_child_costs_more_than_it_brings_in() {
    for years in [2u32, 4, 6, 8, 10] {
        let eats = what_a_body_this_age_eats(years);
        let does = what_a_body_this_age_can_do(years);
        assert!(
            does <= eats,
            "at {years} a body does {does:.2} of a grown day's work and eats \
             {eats:.2} of a grown share, so it feeds itself and more"
        );
    }

    // And by sixteen it is the other way about
    assert_eq!(what_a_body_this_age_can_do(16), 1.0);
    assert_eq!(what_a_body_this_age_eats(16), 1.0);
}

// --------------------------------------------------------------------------
// The supervision bands
// --------------------------------------------------------------------------

/// The three bands, and no leash on a grown person.
#[test]
fn the_bands_are_the_ones_the_lifecycle_describes() {
    let leash = Simulation::how_far_from_a_grown_person_this_one_may_be;

    let arms = leash(LifeStage::Infant).expect("under six is kept close");
    let sight = leash(LifeStage::Child).expect("under eleven is kept in sight");
    let errand = leash(LifeStage::Adolescent).expect("under sixteen is kept within a walk");

    assert!(arms < sight, "with a parent is closer than within sight");
    assert!(sight < errand, "within sight is closer than an hour's walk");

    assert!(leash(LifeStage::Adult).is_none(), "a grown person goes where they like");
    assert!(leash(LifeStage::Elderly).is_none());
}

/// A child too far from anybody grown heads back.
#[test]
fn a_child_on_its_own_goes_back_to_somebody_grown() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (10, 10, 0)));
    population.agents.push(somebody_of(7, (40, 40, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[1];

    let answer = simulation
        .keeping_close_to_somebody_grown(child, child.state.position)
        .expect("thirty paces off is not within sight of anybody");

    match answer {
        Action::Move { target } => assert_eq!((target.0, target.1), (10, 10)),
        other => panic!("he should be walking back: {other:?}"),
    }
}

/// And one already beside an adult carries on with its day.
#[test]
fn a_child_beside_an_adult_is_left_alone() {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (10, 10, 0)));
    population.agents.push(somebody_of(7, (11, 10, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[1];

    assert!(
        simulation
            .keeping_close_to_somebody_grown(child, child.state.position)
            .is_none(),
        "he is standing next to his father"
    );
}

/// A child with no adult left alive is not marched across the map to a corpse.
#[test]
fn a_child_with_nobody_left_is_on_its_own() {
    let mut population = Population::new();
    population.agents.push(somebody_of(7, (40, 40, 0)));
    population.agents.push(somebody_of(9, (10, 10, 0)));

    let simulation = Simulation::new(a_world(), population);
    let child = &simulation.population.agents[0];

    assert!(
        simulation
            .keeping_close_to_somebody_grown(child, child.state.position)
            .is_none(),
        "there is nobody grown to go to"
    );
}

// --------------------------------------------------------------------------
// Feeding a child from a parent's stores
// --------------------------------------------------------------------------

fn a_parent_and_a_hungry_child() -> Simulation {
    let mut population = Population::new();
    population.agents.push(somebody_of(30, (10, 10, 0)));
    population.agents.push(somebody_of(7, (11, 10, 0)));

    let parent = population.agents[0].id;
    let child = population.agents[1].id;

    // A pack with food to spare in it
    let database = crate::world::nutrition::FoodDatabase::new();
    let mut supper = InventoryItem::new_with_weight("food".to_string(), 20, 0.5);
    supper.food_data = database.create_food_data(&crate::world::ItemType::Food, 0);
    let _ = population.agents[0].inventory.add_item(supper);

    // Who they are to one another
    population.agents[0]
        .relationships
        .add_relationship(Relationship::new(child, RelationshipType::Child));
    population.agents[1]
        .relationships
        .add_relationship(Relationship::new(parent, RelationshipType::Parent));

    // And the child is hungry
    population.agents[1]
        .drives
        .get_mut(crate::core::DriveType::Hunger)
        .unwrap()
        .value = 0.9;

    Simulation::new(a_world(), population)
}

/// A parent with food to spare feeds a hungry child of their own.
#[test]
fn a_parent_feeds_a_hungry_child() {
    let simulation = a_parent_and_a_hungry_child();
    let parent = &simulation.population.agents[0];

    let fed = simulation
        .a_child_of_mine_to_feed(parent, parent.state.position)
        .expect("his own child is hungry and he has twenty in his pack");

    assert_eq!(fed, simulation.population.agents[1].id);
}

/// And it reaches the turn: the decision layer hands over rather than
/// carrying on with the day.
#[test]
fn feeding_a_child_reaches_the_turn() {
    let simulation = a_parent_and_a_hungry_child();
    let parent = &simulation.population.agents[0];

    let (action, _) = simulation.generate_non_emotional_action(parent, parent.state.position);

    assert!(
        matches!(action, Action::GiveTo { to } if to == simulation.population.agents[1].id),
        "he should be handing his child something: {action:?}"
    );
}

/// A hungry child that is nothing to this agent gets nothing. A gift is one
/// thing and feeding your own is another.
#[test]
fn somebody_elses_hungry_child_is_not_this_ones_business() {
    let mut simulation = a_parent_and_a_hungry_child();
    simulation.population.agents[0].relationships = Default::default();

    let parent = &simulation.population.agents[0];
    assert!(
        simulation
            .a_child_of_mine_to_feed(parent, parent.state.position)
            .is_none(),
        "this branch is about a parent and a child, not about charity"
    );
}

/// And a parent with nothing to spare does not hand over what they have not
/// got. The sacrifice branch is where that decision belongs.
#[test]
fn a_parent_with_an_empty_pack_gives_nothing() {
    let mut simulation = a_parent_and_a_hungry_child();
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();

    let parent = &simulation.population.agents[0];
    assert!(
        simulation
            .a_child_of_mine_to_feed(parent, parent.state.position)
            .is_none()
    );
}
