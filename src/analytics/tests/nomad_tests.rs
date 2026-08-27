// src/analytics/tests/nomad_tests.rs
//! Tests for a people that has no way of making food happen staying on the
//! move.
//!
//! "Until there is a method of producing food through farming, the agents
//! should likely stick to a nomadic way of life."
//!
//! Wild food regrows about four times slower than a camp of any size eats it,
//! so ground that fed twelve people last season does not feed them this one.
//! Without a field there is nothing to be done about that where you stand: you
//! go where the ground already carries something. A field is the thing that
//! makes staying worth it, and an agent that has worked farming out stops
//! moving.
//!
//! This is not the same mechanism as `migration_action`, which fires on an
//! agent that has already been hungry for a hundred and twenty ticks. That is
//! fleeing. This fires while there is still something here to eat, on the
//! strength of there not being much of it.

use crate::agents::practices::Practice;
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{
    Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig,
};

/// A world with nothing edible in it but what the test puts there
fn bare_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.resources.retain(|resource| {
        !matches!(
            resource.resource_type,
            ResourceType::Food
                | ResourceType::Grain
                | ResourceType::Greens
                | ResourceType::Roots
                | ResourceType::Fish
                | ResourceType::Herbs
        )
    });
    world
}

fn a_camp_at(world: World, where_it_is: (i32, i32), how_many: usize) -> Simulation {
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);
    for agent in &mut simulation.population.agents {
        agent.state.position = (where_it_is.0, where_it_is.1, 0);
    }
    simulation
}

fn put_food(simulation: &mut Simulation, where_it_is: Position, how_much: u32) {
    let mut patch = ResourceNode::new(ResourceType::Food, where_it_is, how_much.max(1));
    patch.amount = how_much;
    simulation.world.resources.push(patch);
}

/// Picked-over ground and somewhere better a fortnight off: the camp moves.
#[test]
fn a_people_with_no_field_moves_off_ground_that_will_not_feed_it() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 12);

    // A handful of berries here, and a valley of them forty tiles away
    put_food(&mut simulation, Position::new(21, 20), 8);
    put_food(&mut simulation, Position::new(60, 20), 400);

    let position = simulation.population.agents[0].state.position;
    let action = simulation.moving_on(&simulation.population.agents[0], position);

    match action {
        Some(Action::Move { target }) => {
            assert!(
                (target.0 - 60).abs() <= 1 && (target.1 - 20).abs() <= 1,
                "the camp should head for the valley, not {target:?}"
            );
        }
        other => panic!("a camp on stripped ground should move: {other:?}"),
    }
}

/// Ground that is still carrying enough is ground worth staying on.
#[test]
fn nobody_moves_off_ground_that_is_still_feeding_them() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 12);

    // Well over what twelve people want standing
    put_food(&mut simulation, Position::new(21, 20), 2000);
    put_food(&mut simulation, Position::new(60, 20), 4000);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_none(),
        "there is no reason to walk forty tiles away from a full larder"
    );
}

/// The same stripped ground, with more mouths on it, is stripped sooner.
#[test]
fn more_mouths_strip_the_ground_sooner() {
    fn would_move(mouths: usize, standing: u32) -> bool {
        let mut simulation = a_camp_at(bare_country(), (20, 20), mouths);
        put_food(&mut simulation, Position::new(21, 20), standing);
        put_food(&mut simulation, Position::new(60, 20), 4000);

        let position = simulation.population.agents[0].state.position;
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_some()
    }

    // Ground carrying a hundred units feeds four people and does not feed
    // forty
    assert!(
        !would_move(4, 100),
        "four people can live off a hundred units of berries"
    );
    assert!(
        would_move(40, 100),
        "forty cannot, and should be looking for somewhere else"
    );
}

/// A field is a reason to stay. This is the whole of what settling down is.
#[test]
fn a_field_is_a_reason_to_stay() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 12);

    put_food(&mut simulation, Position::new(21, 20), 8);
    put_food(&mut simulation, Position::new(60, 20), 400);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_some(),
        "stripped ground and no field: the camp moves"
    );

    // Break ground beside the camp and put a crop in it
    let field = Position::new(22, 20);
    if let Some(tile) = simulation.world.grid.get_tile_mut(&field) {
        tile.terrain = Terrain::new(TerrainType::Farmland);
    }
    put_food(&mut simulation, field, 30);

    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_none(),
        "a standing crop on broken ground is worth more than a fortnight's walk"
    );
}

/// And a farmer stays whether or not there is anything in the ground today:
/// somebody who knows how to make this ground carry a crop has an answer to
/// stripped country that does not involve walking.
#[test]
fn a_farmer_stops_wandering() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 12);

    put_food(&mut simulation, Position::new(21, 20), 8);
    put_food(&mut simulation, Position::new(60, 20), 400);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_some(),
        "before: a forager on stripped ground moves"
    );

    {
        let agent = &mut simulation.population.agents[0];
        agent.practices.saw_it_work(Practice::Farming);
        agent.practices.saw_it_work(Practice::Farming);
        assert!(agent.practices.is_established(Practice::Farming));
    }

    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_none(),
        "after: a farmer breaks ground instead of walking away from it"
    );
}

/// Nowhere better to go is not a reason to set off.
#[test]
fn nobody_wanders_off_towards_nothing() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 12);

    // Stripped ground here, and nothing anywhere else either
    put_food(&mut simulation, Position::new(21, 20), 8);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_none(),
        "there is no point setting out for somewhere that is not there"
    );
}

/// And a patch just over the hill is not a move: this is about picking up the
/// camp, not about walking to the next hedge.
#[test]
fn a_stroll_is_not_a_move() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 12);

    put_food(&mut simulation, Position::new(21, 20), 8);
    // Well inside the ordinary foraging radius, which the food drive already
    // covers
    put_food(&mut simulation, Position::new(26, 20), 4000);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .moving_on(&simulation.population.agents[0], position)
            .is_none(),
        "a patch six tiles off is somewhere to forage, not somewhere to move to"
    );
}

// --------------------------------------------------------------------------
// Leaving for want of water
//
// `migration_action` read the Hunger drive and nothing else, and `moving_on`
// counts what is edible standing within reach. So a settlement whose springs
// had gone dry and whose hedgerows were full had no reason anywhere in this
// model to pick up and leave — and did not. Thirst kills a man three times
// faster than hunger does. See ISSUES_FOUND #53.
// --------------------------------------------------------------------------

use crate::core::memory::{SpatialMemory, SpatialMemoryType};
use crate::core::DriveType;

/// Somebody who has been going without water long enough sets out.
#[test]
fn thirst_is_a_reason_to_leave_a_country() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 1);

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .migration_action(&simulation.population.agents[0], position)
            .is_none(),
        "a man with nothing wrong with him stays where he is"
    );

    if let Some(thirst) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Thirst)
    {
        thirst.denied_ticks = Simulation::HUNGRY_ENOUGH_TO_LEAVE + 1;
    }

    assert!(
        matches!(
            simulation.migration_action(&simulation.population.agents[0], position),
            Some(Action::Move { .. })
        ),
        "ten days without a drink is a reason to be somewhere else"
    );
}

/// And a man leaving for want of water walks towards water, not towards a
/// berry bush.
#[test]
fn a_man_leaving_for_water_walks_towards_water() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 1);
    let position = simulation.population.agents[0].state.position;

    let river = (20, 60, 0);
    let hedgerow = (60, 20, 0);

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .memory
            .spatial_memories
            .push(SpatialMemory::new(SpatialMemoryType::Water, river, 0));
        agent
            .memory
            .spatial_memories
            .push(SpatialMemory::new(SpatialMemoryType::Food, hedgerow, 0));

        if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
            thirst.denied_ticks = Simulation::HUNGRY_ENOUGH_TO_LEAVE + 1;
        }
    }

    let answer = simulation
        .migration_action(&simulation.population.agents[0], position)
        .expect("he is leaving");

    assert!(
        matches!(answer, Action::Move { target } if target == river),
        "he sets out for the water he remembers, not the berries: {answer:?}"
    );
}

/// Hunger still works the same way, and still sends a man to food.
#[test]
fn hunger_still_sends_a_man_to_food() {
    let mut simulation = a_camp_at(bare_country(), (20, 20), 1);
    let position = simulation.population.agents[0].state.position;

    let river = (20, 60, 0);
    let hedgerow = (60, 20, 0);

    {
        let agent = &mut simulation.population.agents[0];
        agent
            .memory
            .spatial_memories
            .push(SpatialMemory::new(SpatialMemoryType::Water, river, 0));
        agent
            .memory
            .spatial_memories
            .push(SpatialMemory::new(SpatialMemoryType::Food, hedgerow, 0));

        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.denied_ticks = Simulation::HUNGRY_ENOUGH_TO_LEAVE + 1;
        }
    }

    let answer = simulation
        .migration_action(&simulation.population.agents[0], position)
        .expect("he is leaving");

    assert!(
        matches!(answer, Action::Move { target } if target == hedgerow),
        "a hungry man walks to the berries: {answer:?}"
    );
}
