// src/analytics/tests/full_pack_tests.rs
//! A mouth is not a pack.
//!
//! `Gather: Inventory full - cannot carry more` was the single largest refusal
//! in this model: **139,126 of 199,981, seven in ten of everything anybody was
//! refused**. A good share of it was somebody hungry, standing on a bush,
//! being told they could not carry it - when they were not trying to carry it.
//! Measured at the last look anybody got before they died, 61.5% had no room
//! for another armful, and they died eleven days into a three-week reserve.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// Small enough that what is left over cannot hold a handful of food.
const HALF_A_HANDFUL: f32 = crate::agents::provision::WHAT_A_HANDFUL_OF_FOOD_WEIGHS / 2.0;

/// Fill the pack with stone, right up to the limit, so that not one more
/// handful of anything will go in.
///
/// A stack at a time, because `add_item` is all or nothing and an oversized
/// stack is simply refused; and then pebbles, because `take_what_fits` takes
/// as much as fits, and a pack with room for one berry left is not a pack
/// with no room at all.
fn fill_the_pack(simulation: &mut crate::analytics::Simulation, with: &str) {
    for each in [2.0_f32, HALF_A_HANDFUL] {
        while simulation.population.agents[0]
            .inventory
            .weight_capacity_remaining()
            >= each
        {
            let went_in = simulation.population.agents[0]
                .inventory
                .add_item(InventoryItem::new_with_weight(with.to_string(), 1, each));
            if !went_in {
                break;
            }
        }
    }

    let room = simulation.population.agents[0]
        .inventory
        .weight_capacity_remaining();

    assert!(
        room < crate::agents::provision::WHAT_A_HANDFUL_OF_FOOD_WEIGHS,
        "the fixture is meant to leave no room for so much as one berry, got {room}"
    );
    assert!(
        room < crate::analytics::Simulation::what_one_of_these_weighs(ResourceType::Food),
        "the fixture is meant to leave no room for an armful, got {room}"
    );
}

/// One hungry person with a pack full of rocks, standing on a berry patch.
fn a_full_pack_on_a_berry_patch() -> crate::analytics::Simulation {
    a_pack_full_of("stone")
}

/// The same, with the pack full of something the agent is holding rather than
/// something it would put down: nothing here can be set down for a meal.
fn a_pack_of_nothing_but_food_on_a_berry_patch() -> crate::analytics::Simulation {
    a_pack_full_of("meat")
}

/// One hungry person on a berry patch, with a pack full of whatever is named.
fn a_pack_full_of(filler: &str) -> crate::analytics::Simulation {
    let mut world = World::new(WorldConfig::default());

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let here = population.agents[0].state.position;

    // The one bush in the country, so that `resources[0]` is the bush and
    // nothing the generator scattered can be the thing that got picked.
    world.resources.clear();
    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(here.0, here.1),
        500,
    ));

    let mut simulation = crate::analytics::Simulation::new(world, population);

    fill_the_pack(&mut simulation, filler);

    // And make them hungry enough that a meal is the point.
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 0.95;
    }

    simulation
}

/// The gather is not refused: what will not go in the pack goes in the mouth.
///
/// The pack here is full of meat, which is to say full of things nobody would
/// put on the grass for a handful of berries. A pack full of stone is a
/// different case and has its own test below: the stone goes down.
#[test]
fn a_hungry_man_with_a_full_pack_eats_where_he_stands() {
    let mut simulation = a_pack_of_nothing_but_food_on_a_berry_patch();

    // One tick first, so the body has a reserve and a stomach to put
    // anything in: `now_a_body_of` runs on the turn, and a fixture that has
    // never ticked has a body that has never been sized. The turn is a whole
    // turn, though - he may set something down or eat something in it - so
    // the pack is filled again afterwards, and it is the second filling that
    // the gather is put to.
    simulation.tick();
    fill_the_pack(&mut simulation, "meat");

    let in_the_belly_before = simulation.population.agents[0]
        .state
        .physiology
        .in_the_stomach();
    let carried_before = simulation.population.agents[0].food_put_by();

    let result = simulation.execute_action(
        &Action::Gather {
            resource_type: "food".to_string(),
        },
        0,
    );

    assert!(
        result.success,
        "a hungry man on a berry patch was refused for want of room: {:?}",
        result.message
    );

    let in_the_belly_after = simulation.population.agents[0]
        .state
        .physiology
        .in_the_stomach();
    let carried_after = simulation.population.agents[0].food_put_by();

    assert!(
        in_the_belly_after > in_the_belly_before,
        "the gather succeeded and nothing went down:          {in_the_belly_before} then {in_the_belly_after}"
    );

    // And it went down rather than into the pack, because there was no room
    // in the pack - which is the whole point.
    assert_eq!(
        carried_after, carried_before,
        "there was no room, so nothing should have been carried off"
    );
}

/// And the decision layer lets him go, rather than refusing before the
/// executor can offer the meal.
#[test]
fn the_gate_lets_a_hungry_man_at_a_bush_he_cannot_carry() {
    let simulation = a_full_pack_on_a_berry_patch();
    let agent = simulation.population.agents[0].clone();

    assert!(
        simulation.could_this_gather_come_to_anything(&agent, agent.state.position, "food"),
        "a full pack stopped the decision before the meal could be offered"
    );
}

/// A full pack still cannot carry stone, hungry or not: only the mouth is
/// exempt from the pack, and only for food.
#[test]
fn a_full_pack_still_cannot_carry_rocks() {
    let mut simulation = a_full_pack_on_a_berry_patch();

    let here = simulation.population.agents[0].state.position;
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Stone,
        Position::new(here.0, here.1),
        500,
    ));

    let agent = simulation.population.agents[0].clone();
    assert!(
        !simulation.could_this_gather_come_to_anything(&agent, agent.state.position, "stone"),
        "a man with no room took up more stone"
    );
}

/// A pack full of stone makes room for the berries, and the stone stays on
/// the ground where it was set down.
///
/// This is the other half of the same refusal. `Gather: Inventory full` was
/// 139,126 of 199,981 - seven in ten of everything anybody was ever refused -
/// and it was not that these packs were overloaded. They were exactly full,
/// year round, and the carrying invariant has nothing to say about a pack
/// that is exactly full. Fifty-five per cent of that weight was raw material
/// and about one per cent was food.
#[test]
fn a_full_pack_of_stone_makes_room_for_the_berries() {
    let mut simulation = a_full_pack_on_a_berry_patch();

    simulation.tick();
    fill_the_pack(&mut simulation, "stone");

    let stone_before = simulation.population.agents[0]
        .inventory
        .get_item("stone")
        .map(|item| item.quantity)
        .unwrap_or(0);
    let on_the_ground_before = simulation.world.dropped.len();

    let result = simulation.execute_action(
        &Action::Gather {
            resource_type: "food".to_string(),
        },
        0,
    );

    assert!(
        result.success,
        "a man with a pack of stone was refused the berries: {:?}",
        result.message
    );

    assert!(
        simulation.population.agents[0].food_put_by() > 0,
        "the berries went nowhere: the pack is still all stone"
    );

    let stone_after = simulation.population.agents[0]
        .inventory
        .get_item("stone")
        .map(|item| item.quantity)
        .unwrap_or(0);
    assert!(
        stone_after < stone_before,
        "nothing was set down: {stone_before} stone then {stone_after}"
    );

    assert!(
        simulation.world.dropped.len() > on_the_ground_before,
        "the stone was destroyed rather than set down"
    );
}

/// The gate and the executor weigh a stone the same.
///
/// They did not. The executor charged five for a stone, eight for iron and
/// two for wood, and the gate asked only whether there was one unit of room -
/// so a pack with a unit and a half left passed the gate for stone and was
/// refused by `take_what_fits` the instant the turn was spent, and passed it
/// again the next turn, and the next. Measured over eight seeded world-years,
/// `Gather: Inventory full` came to **241,191 refusals, 79.7% of every
/// refusal in the model**, against 23,293 person-days: ten of the forty-eight
/// turns in everybody's day.
#[test]
fn the_gate_weighs_a_stone_the_same_as_the_pack_does() {
    let mut simulation = a_full_pack_on_a_berry_patch();

    let here = simulation.population.agents[0].state.position;
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Stone,
        Position::new(here.0, here.1),
        500,
    ));

    // Room for an item and a half, which is a unit and a half - enough for
    // the old gate and never enough for a stone.
    let room_for_no_stone = crate::analytics::Simulation::what_one_of_these_weighs(
        ResourceType::Stone,
    ) - 0.5;
    let load = simulation.population.agents[0].inventory.max_weight - room_for_no_stone;
    simulation.population.agents[0].inventory.current_weight = load;

    let agent = simulation.population.agents[0].clone();
    assert!(
        !simulation.could_this_gather_come_to_anything(&agent, agent.state.position, "stone"),
        "the decision let him set off for a stone he had no room for"
    );

    // And the executor agrees, which is the point: one table, two readers.
    let result = simulation.execute_action(
        &Action::Gather {
            resource_type: "stone".to_string(),
        },
        0,
    );
    assert!(
        !result.success,
        "the executor took a stone the decision said would not fit"
    );
}
