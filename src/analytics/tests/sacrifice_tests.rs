// src/analytics/tests/sacrifice_tests.rs
//! Tests for laying down your life for your own.
//!
//! Two forms, and the specification asks for both: standing between a threat
//! and somebody who cannot deal with it, and going without food you need
//! yourself so that somebody who needs it more gets it.
//!
//! The first is the only place in the model where an agent knowingly takes
//! the worse of two options — the whole fight-or-flight tree is about picking
//! the better one, and a parent with a wolf standing over their child is not
//! picking anything. The second is a gift that costs: `somebody_to_give_to`
//! hands over what is spare, and what is spare is by definition not a
//! sacrifice.

use crate::agents::{AgentConfig, EmotionSource, InventoryItem, LifeStage, Population};
use crate::agents::emotions::{Relationship, RelationshipType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, World, WorldConfig};

/// A meal that the nutrition machinery recognises as one.
fn supper(how_many: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight("food".to_string(), how_many, 0.5);
    meal.food_data = database.create_food_data(&ItemType::Food, 0);
    meal
}

fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

/// A parent and a small child, standing together.
fn a_parent_and_a_child(world: World) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    let parent = simulation.population.agents[0].id;
    let child = simulation.population.agents[1].id;

    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = 60.0;
    simulation.population.agents[0].state.energy = 100.0;

    simulation.population.agents[1].state.position = (31, 30, 0);
    simulation.population.agents[1].state.life_stage = LifeStage::Child;
    simulation.population.agents[1].parent_ids = vec![parent];

    simulation.population.agents[0]
        .relationships
        .add_relationship(Relationship::new(child, RelationshipType::Child));
    simulation.population.agents[1]
        .relationships
        .add_relationship(Relationship::new(parent, RelationshipType::Parent));

    simulation
}

fn empty_the_pack(simulation: &mut Simulation, who: usize) {
    simulation.population.agents[who]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[who]
        .inventory
        .recalculate_weight();
}

// --------------------------------------------------------------------------
// Standing in the way
// --------------------------------------------------------------------------

/// A wolf standing over a child brings the parent, whatever the odds are.
#[test]
fn a_parent_stands_between_the_wolf_and_the_child() {
    let mut world = an_empty_country();
    // Four of them, which the parent could not beat and would otherwise run
    // from — see `threat_tests::a_man_who_would_fight_one_wolf_runs_from_four`
    for at in [(32, 30), (32, 31), (31, 31), (32, 29)] {
        world
            .spawn_animal("wolf".to_string(), at)
            .expect("a wolf should spawn");
    }

    let mut simulation = a_parent_and_a_child(world);
    simulation.feel_about_what_stands_in_the_way();

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("wolves at his elbow want an answer");

    assert!(
        matches!(answer, Action::Fight { .. } | Action::Move { .. }),
        "he goes at them, he does not go: {answer:?}"
    );
}

/// And the same man, with no child there, goes.
#[test]
fn the_same_man_alone_runs() {
    let mut world = an_empty_country();
    for at in [(32, 30), (32, 31), (31, 31), (32, 29)] {
        world
            .spawn_animal("wolf".to_string(), at)
            .expect("a wolf should spawn");
    }

    let mut simulation = a_parent_and_a_child(world);
    // The child is somewhere else entirely
    simulation.population.agents[1].state.position = (10, 10, 0);
    simulation.feel_about_what_stands_in_the_way();

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("wolves at his elbow want an answer");

    assert!(
        matches!(answer, Action::FleeFrom { .. }),
        "with nobody to stand for, he takes the better of the two: {answer:?}"
    );
}

/// Somebody who could fight it themselves is not being protected.
#[test]
fn nobody_lays_down_their_life_for_a_grown_man() {
    let mut world = an_empty_country();
    for at in [(32, 30), (32, 31), (31, 31), (32, 29)] {
        world
            .spawn_animal("wolf".to_string(), at)
            .expect("a wolf should spawn");
    }

    let mut simulation = a_parent_and_a_child(world);
    // The "child" is a grown adult who can look after himself
    simulation.population.agents[1].state.life_stage = LifeStage::Adult;
    simulation.population.agents[1].state.health = 100.0;
    simulation.feel_about_what_stands_in_the_way();

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("wolves at his elbow want an answer");

    assert!(
        matches!(answer, Action::FleeFrom { .. }),
        "that is somebody being joined, not somebody being protected: {answer:?}"
    );
}

/// And a man who cannot lift an arm does not stand in the way either. There
/// is a difference between a sacrifice and a gesture.
#[test]
fn a_man_who_cannot_fight_still_cannot_fight() {
    use crate::agents::body::{BodyPartStatus, BodyPartType};

    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");

    let mut simulation = a_parent_and_a_child(world);
    for part in [BodyPartType::LeftArm, BodyPartType::RightArm] {
        if let Some(arm) = simulation.population.agents[0].body.get_part_mut(part) {
            arm.status = BodyPartStatus::Disabled;
        }
    }
    simulation.feel_about_what_stands_in_the_way();

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf at his elbow wants an answer");

    assert!(
        !matches!(answer, Action::Fight { .. }),
        "he has nothing to fight it with: {answer:?}"
    );
}

// --------------------------------------------------------------------------
// Going without
// --------------------------------------------------------------------------

/// A parent with food and a starving child hands it over.
#[test]
fn a_parent_goes_without_for_a_starving_child() {
    let mut simulation = a_parent_and_a_child(an_empty_country());
    empty_the_pack(&mut simulation, 0);
    empty_the_pack(&mut simulation, 1);

    let _ = simulation.population.agents[0].inventory.add_item(supper(2));

    simulation.population.agents[1].nutrition.energy_reserves = 2.0;

    let here = simulation.population.agents[0].state.position;
    let for_them = simulation
        .somebody_of_mine_who_needs_it_more(&simulation.population.agents[0], here)
        .expect("his child is dying and he has supper in his pack");

    assert_eq!(for_them, simulation.population.agents[1].id);
}

/// And the food actually changes hands.
#[test]
fn what_is_gone_without_ends_up_with_them() {
    let mut simulation = a_parent_and_a_child(an_empty_country());
    empty_the_pack(&mut simulation, 0);
    empty_the_pack(&mut simulation, 1);

    let _ = simulation.population.agents[0].inventory.add_item(supper(2));

    let for_them = simulation.population.agents[1].id;
    let result = simulation.execute_action(&Action::GoWithout { for_them }, 0);

    assert!(result.success, "{:?}", result.message);
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("food"),
        1,
        "he gave one of his two away"
    );
    assert_eq!(
        simulation.population.agents[1].how_many_i_have("food"),
        1,
        "and the child has it"
    );
}

/// Nobody goes without for a stranger. That is what a gift is for.
#[test]
fn nobody_goes_without_for_a_stranger() {
    let mut simulation = a_parent_and_a_child(an_empty_country());
    empty_the_pack(&mut simulation, 0);
    empty_the_pack(&mut simulation, 1);

    let _ = simulation.population.agents[0].inventory.add_item(supper(2));

    simulation.population.agents[1].nutrition.energy_reserves = 2.0;
    // No longer anybody's child
    simulation.population.agents[0].relationships =
        crate::agents::emotions::RelationshipMap::new();

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .somebody_of_mine_who_needs_it_more(&simulation.population.agents[0], here)
            .is_none(),
        "you go hungry for your own, not for anybody who happens to be standing there"
    );
}

/// And a man already past bearing keeps what he has. Two dead people is not
/// better than one.
#[test]
fn a_man_already_dying_keeps_his_supper() {
    let mut simulation = a_parent_and_a_child(an_empty_country());
    empty_the_pack(&mut simulation, 0);
    empty_the_pack(&mut simulation, 1);

    let _ = simulation.population.agents[0].inventory.add_item(supper(2));

    simulation.population.agents[1].nutrition.energy_reserves = 2.0;

    simulation.population.agents[0].state.energy = 1.0;
    simulation.population.agents[0].state.gone_without_food_for(2000);
    simulation.population.agents[0].nutrition.energy_reserves = 1.0;

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .somebody_of_mine_who_needs_it_more(&simulation.population.agents[0], here)
            .is_none(),
        "two dead people is not better than one"
    );
}

/// Somebody who has food of their own is not going without anything.
#[test]
fn nobody_goes_without_for_somebody_with_a_full_pack() {
    let mut simulation = a_parent_and_a_child(an_empty_country());
    empty_the_pack(&mut simulation, 0);
    empty_the_pack(&mut simulation, 1);

    for who in 0..2 {
        let _ = simulation.population.agents[who].inventory.add_item(supper(2));
    }

    simulation.population.agents[1].nutrition.energy_reserves = 2.0;

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .somebody_of_mine_who_needs_it_more(&simulation.population.agents[0], here)
            .is_none(),
        "the child has supper of its own"
    );
}

/// Going without counts for more with the person it was done for than an
/// ordinary gift does.
#[test]
fn going_without_counts_for_more_than_a_gift() {
    assert!(
        Simulation::WHAT_GOING_WITHOUT_IS_WORTH > Simulation::WHAT_A_GIFT_IS_WORTH,
        "a thing somebody could spare is not the same as a thing they could not"
    );
}
