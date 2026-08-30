// src/analytics/tests/scent_tests.rs
//! Tests for what agents can and cannot smell.
//!
//! Smell used to be the sense that found everything: every resource on the map
//! gave off the same scent, and a full-strength one, so an agent could smell a
//! berry from twenty-five tiles away and sight was decoration. A human nose
//! does not work like that. These cover the model that replaced it:
//! - a berry on the bush is close to odourless; you have to be on top of it
//! - flesh carries further than fruit, water further than nothing
//! - food that has turned announces itself from a long way off
//! - cooking is the loudest smell there is
//! - sight outranges every food smell but a cooking fire

use crate::agents::senses::{ScentType, Smell};
use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, ResourceNode, ResourceType, World, WorldConfig};

/// How many tiles a scent of this strength reaches an ordinary nose.
fn reach(strength: f32) -> f32 {
    let nose = Smell::default();
    nose.smell_range * nose.sensitivity * strength
}

/// The ordering the model is built on: cooking loudest, then rot, then flesh,
/// then everything raw and whole.
#[test]
fn cooking_smells_strongest_and_a_berry_smells_least() {
    let database = FoodDatabase::new();

    let mut cooked = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    cooked.preparation = PreparationState::Cooked;

    let mut rotten = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    rotten.freshness = 0.05;

    let cooking = cooked.scent_strength();
    let rot = rotten.scent_strength();
    let flesh = ResourceType::Meat.raw_scent_strength();
    let berry = ResourceType::Food.raw_scent_strength();

    assert!(
        cooking > rot && rot > flesh && flesh > berry && berry > 0.0,
        "expected cooking > rot > flesh > berry, got {cooking} {rot} {flesh} {berry}"
    );

    // And in tiles, so the numbers mean something: a berry is a couple of
    // paces, a fire is the whole range of the nose.
    assert!(reach(berry) <= 3.0, "a berry should be all but odourless");
    assert!(
        reach(flesh) >= 5.0 && reach(flesh) <= 10.0,
        "flesh should carry a few tiles, not a field"
    );
    assert!(reach(cooking) >= 25.0, "a cooking fire should carry as far as a nose reaches");
}

/// Rot is the one food smell that grows as the food gets worse.
#[test]
fn the_further_gone_the_food_the_further_it_carries() {
    let database = FoodDatabase::new();
    let make = |freshness: f32| {
        let mut food = database
            .create_food_data(&ItemType::Food, 0)
            .expect("generic food should be in the database");
        food.freshness = freshness;
        food
    };

    let fresh = make(1.0);
    let turning = make(0.35);
    let foul = make(0.0);

    assert!(!fresh.is_rotting(), "fresh food should not smell of rot");
    assert!(turning.is_rotting());
    assert!(foul.is_rotting());

    assert!(
        foul.scent_strength() > turning.scent_strength(),
        "the further gone it is, the further it should carry"
    );
    assert!(
        turning.scent_strength() > fresh.scent_strength(),
        "food that has turned should be louder than food that has not"
    );
}

/// An agent standing a few tiles from a berry patch does not smell it, and one
/// standing in it does.
///
/// This is the behaviour that made sight matter: under the old model every
/// resource smelled at full strength, so an agent could find its dinner with
/// its eyes shut from the other side of a field.
#[test]
fn a_berry_patch_is_smelled_only_from_a_pace_or_two() {
    let mut world = World::new(WorldConfig::default());

    // Clear the map so only the patch we place can be smelled
    world.resources.clear();
    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(30, 30),
        50,
    ));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 31, 0);
    simulation.population.agents[1].state.position = (30, 40, 0);

    simulation.emit_scents();

    let smelled_by = |agent: &Agent| {
        agent
            .senses
            .smell
            .detected_scents
            .iter()
            .any(|scent| scent.scent_type == ScentType::Food)
    };

    assert!(
        smelled_by(&simulation.population.agents[0]),
        "an agent standing next to the patch should smell it"
    );
    assert!(
        !smelled_by(&simulation.population.agents[1]),
        "an agent ten tiles off should not smell a berry patch"
    );
}

/// Food that has turned gives its carrier away, and as decay rather than food:
/// a nose says "something is rotten here", it does not send anyone over to eat.
#[test]
fn rotting_food_announces_itself_as_decay() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[1].state.position = (30, 35, 0);

    let database = FoodDatabase::new();
    let mut rotten = InventoryItem::new_with_weight("food".to_string(), 3, 0.5);
    let mut food_data = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    food_data.freshness = 0.05;
    rotten.food_data = Some(food_data);
    simulation.population.agents[0].inventory.add_item(rotten);

    simulation.emit_scents();

    let neighbour = &simulation.population.agents[1];
    assert!(
        neighbour
            .senses
            .smell
            .detected_scents
            .iter()
            .any(|scent| scent.scent_type == ScentType::Decay),
        "a neighbour five tiles away should smell the rot"
    );
    assert!(
        !neighbour
            .senses
            .smell
            .detected_scents
            .iter()
            .any(|scent| scent.scent_type == ScentType::Food),
        "rot should not read as something to eat"
    );
}

/// A lit fire with something in it is smelled across the whole range of a nose,
/// blind or not.
#[test]
fn a_cooking_fire_carries_across_the_map() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let fire = world
        .build_heat_source(
            crate::environment::HeatSourceType::Campfire,
            (30, 30, 0),
            None,
        )
        .expect("should be able to build a campfire");
    world
        .add_fuel_to_heat_source(&fire, "wood".to_string(), 5.0)
        .expect("should be able to fuel it");
    world
        .light_heat_source(&fire)
        .expect("a fuelled fire should light");
    world
        .add_to_heat_source(&fire, "food".to_string(), 2)
        .expect("should be able to put food on it");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 50, 0);
    simulation.population.agents[1].state.position = (30, 30, 0);
    simulation.population.agents[1]
        .traits
        .add_trait(crate::core::traits::Trait::Blind);
    simulation.population.agents[1].apply_trait_sensory_modifications();

    simulation.emit_scents();

    for (index, description) in [(0usize, "twenty tiles away"), (1, "a blind agent beside it")] {
        assert!(
            simulation.population.agents[index]
                .senses
                .smell
                .detected_scents
                .iter()
                .any(|scent| scent.scent_type == ScentType::Food),
            "{description} should smell the cooking"
        );
    }
}

/// Sight is the primary way food is found: it outranges every food smell except
/// a cooking fire, which is the one thing a nose beats an eye at.
#[test]
fn sight_outranges_every_food_smell_but_a_fire() {
    let agent = Agent::new(AgentConfig::default());
    let sight = agent.sight_range() as f32;

    for resource in ResourceType::all() {
        assert!(
            sight > reach(resource.raw_scent_strength()),
            "{resource:?} should be seen before it is smelled: sight {sight}, smell {}",
            reach(resource.raw_scent_strength())
        );
    }
}

/// Everything the world gives off that is not water is given off as
/// `ScentType::Food`, so anything with a smell had better be something to eat.
///
/// The scent table and the answer to "is this food" were two hand-written
/// lists and they had drifted apart: **herbs** smelled of dinner and nobody in
/// this model can eat them, while **greens and roots** - most of what anybody
/// eats in three seasons out of four - smelled of nothing at all. A starving
/// agent smells the herbs, walks to them, gathers nothing, and does it again
/// next tick. See ISSUES #229.
#[test]
fn only_food_smells_of_food() {
    for resource in ResourceType::all() {
        // Water is the other thing a nose is for, and is not food.
        if resource == ResourceType::Water {
            assert!(resource.raw_scent_strength() > 0.0, "water should smell");
            continue;
        }

        assert_eq!(
            resource.raw_scent_strength() > 0.0,
            resource.is_it_food(),
            "{resource:?} smells of food ({}) but is food ({})",
            resource.raw_scent_strength() > 0.0,
            resource.is_it_food()
        );
    }
}

/// Everything a person can eat grows back, and everything that grows back
/// stays on the map when it is emptied.
///
/// Three hand-written lists asked the same question and two of them had the
/// same hole. `how_fast_it_comes_back` did not name **Greens** or **Roots**,
/// so they grew at nothing; `is_renewable` did not name them either, so
/// `World::remove_depleted_resources` **deleted the node off the map** the
/// moment somebody finished a patch. Between them that made 63.6% of the food
/// on a map single-use - eaten once and gone for good - which is the whole of
/// why a settlement of twelve was down to three by the end of its first
/// spring. See ISSUES_FOUND.md #123.
///
/// The two questions are one function now, and this holds it to the food list.
#[test]
fn everything_a_person_eats_grows_back() {
    // Anything that stands in the ground as a crop. Meat is food and does not
    // grow: it is what is left of an animal, and an eaten carcass is rightly
    // deleted. Fish come up the river rather than growing back out of what is
    // left of them - see `ResourceNode::fish_run`.
    let crops = ResourceType::all()
        .into_iter()
        .filter(|kind| kind.is_it_food() && kind.is_it_grown());

    for resource in crops {
        assert!(
            resource.how_fast_it_comes_back() > 0.0,
            "{resource:?} grows in the ground and can be eaten, and it does not \
             grow back - so a patch of it is eaten once and gone, and \
             `remove_depleted_resources` will delete the node as well"
        );
    }
}

/// And every food carries at least as far as the thinnest of them, so nothing
/// a settlement lives on is invisible to a nose.
#[test]
fn nothing_anybody_eats_is_odourless() {
    for resource in ResourceType::all().into_iter().filter(|r| r.is_it_food()) {
        assert!(
            resource.raw_scent_strength() > 0.0,
            "{resource:?} is food and gives off nothing"
        );
    }
}

/// Scents do not pile up: they are re-derived from the world every tick.
///
/// Appending instead left thousands of duplicates behind, and stale ones kept
/// rebuilding memories of patches that had long since been eaten.
#[test]
fn scents_do_not_accumulate_across_ticks() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    for _ in 0..50 {
        simulation.tick();
    }

    let scents = simulation.population.agents[0]
        .senses
        .smell
        .detected_scents
        .len();

    assert!(
        scents < 200,
        "scent list should be rebuilt each tick, not appended to; found {scents}"
    );
}
