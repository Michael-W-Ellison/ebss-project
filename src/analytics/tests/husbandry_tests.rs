// src/analytics/tests/husbandry_tests.rs
//! Tests for the things an agent does once it has eaten: breaking ground,
//! looking further ahead, keeping its children close, and teaching them by
//! being watched.
//!
//! Wild food regrows about four times slower than a grown settlement eats it,
//! which is why settlements that got past a dozen people starved back down
//! again. A field yields many times what the same ground does wild. None of it
//! happens in an agent with nothing to eat: the drives that look past this
//! afternoon only climb once the afternoon is taken care of.

use crate::agents::{AgentConfig, LifeStage, Population, SkillType};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{TerrainType, World, WorldConfig};

/// Ground broken by an agent grows crops many times faster than the same
/// ground left wild.
#[test]
fn a_field_yields_far_more_than_the_hedgerow() {
    use crate::world::{Position, ResourceNode, ResourceType};

    let mut wild = ResourceNode::new(ResourceType::Grain, Position::new(10, 10), 5000);
    let mut sown = ResourceNode::new(ResourceType::Grain, Position::new(11, 10), 5000);
    wild.amount = 0;
    sown.amount = 0;

    // A season of good growing weather, in the passes the world actually runs
    for _ in 0..60 {
        wild.regenerate_on(20.0, 0.5, 1.0, false);
        sown.regenerate_on(20.0, 0.5, 1.0, true);
    }

    assert!(
        sown.amount > wild.amount * 4,
        "a field should far outyield the wild: {} against {}",
        sown.amount,
        wild.amount
    );
}

/// Only open grass can be broken, and breaking it leaves a field with a crop
/// in it.
#[test]
fn breaking_ground_turns_grass_into_a_field() {
    use crate::world::Position;

    let mut world = World::new(WorldConfig::default());

    // Put a patch of plain grass under the agent, with nothing growing on it
    let where_it_stands = Position::new(25, 25);
    if let Some(tile) = world.grid.get_tile_mut(&where_it_stands) {
        tile.terrain = crate::world::Terrain::new(TerrainType::Plains);
    }
    world
        .resources
        .retain(|resource| resource.position != where_it_stands);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);

    let result = simulation.execute_action(&Action::TillSoil, 0);
    assert!(result.success, "open grass should break: {:?}", result.message);

    assert_eq!(
        simulation
            .world
            .grid
            .get_tile(&where_it_stands)
            .map(|tile| tile.terrain.terrain_type),
        Some(TerrainType::Farmland),
        "the ground should now be a field"
    );

    assert!(
        simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == where_it_stands),
        "and it should have been sown"
    );

    // The same ground cannot be broken twice
    let again = simulation.execute_action(&Action::TillSoil, 0);
    assert!(!again.success, "a field is not grass any more");
}

/// Nobody ploughs a mountainside.
#[test]
fn only_open_grass_can_be_broken() {
    use crate::world::Position;

    let mut world = World::new(WorldConfig::default());
    let where_it_stands = Position::new(25, 25);
    if let Some(tile) = world.grid.get_tile_mut(&where_it_stands) {
        tile.terrain = crate::world::Terrain::new(TerrainType::Mountain);
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);

    let result = simulation.execute_action(&Action::TillSoil, 0);
    assert!(!result.success, "a mountain is not a field");
}

/// An agent that has eaten starts thinking further ahead; one that has not,
/// does not.
#[test]
fn the_long_view_belongs_to_the_well_fed() {
    use crate::core::drives::Drive;

    let mut fed = Drive::new(DriveType::Sustenance);
    let mut hungry = Drive::new(DriveType::Sustenance);

    for _ in 0..200 {
        fed.tick_with_security(true);
        hungry.tick_with_security(false);
    }

    assert!(
        fed.value > hungry.value * 4.0,
        "a fed agent should want a field far more than a starving one: \
         {:.2} against {:.2}",
        fed.value,
        hungry.value
    );

    // And the immediate needs themselves are unaffected either way
    let mut easy = Drive::new(DriveType::Hunger);
    let mut hard = Drive::new(DriveType::Hunger);
    for _ in 0..200 {
        easy.tick_with_security(true);
        hard.tick_with_security(false);
    }
    assert!(
        (easy.value - hard.value).abs() < 0.001,
        "hunger comes on at its own pace whatever else is happening"
    );
}

/// A hungry agent does not stop to break ground.
#[test]
fn nobody_farms_on_an_empty_stomach() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);

    {
        let agent = &mut simulation.population.agents[0];
        if let Some(drive) = agent.drives.get_mut(DriveType::Sustenance) {
            drive.value = 1.0;
        }
        if let Some(drive) = agent.drives.get_mut(DriveType::Hunger) {
            drive.value = 1.0;
        }
    }

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .farming_action(&simulation.population.agents[0], position)
            .is_none(),
        "a starving agent has more pressing business than next year's grain"
    );

    // Fed, it goes to work
    if let Some(drive) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        drive.value = 0.0;
    }

    assert!(
        simulation
            .farming_action(&simulation.population.agents[0], position)
            .is_some(),
        "a fed agent with the drive on it should go and break ground"
    );
}

/// A parent goes after a child that has wandered off.
#[test]
fn a_parent_goes_after_a_straying_child() {
    let mut world = World::new(WorldConfig::default());

    // Nothing prowling: this is about the leash, not about a wolf
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    let parent_id = simulation.population.agents[0].id;
    simulation.population.agents[0].state.position = (10, 10, 0);
    simulation.population.agents[0].state.age = 4000;
    simulation.population.agents[0].update_life_stage();

    {
        let child = &mut simulation.population.agents[1];
        child.parent_ids = vec![parent_id];
        child.state.age = 700;
        child.update_life_stage();
        child.state.position = (10, 11, 0);
    }
    simulation.population.agents[1].update_life_stage();

    let parent_position = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .protective_action(&simulation.population.agents[0], parent_position)
            .is_none(),
        "a child at the parent's elbow needs no fetching"
    );

    // Now let it wander
    simulation.population.agents[1].state.position = (40, 40, 0);

    let going = simulation
        .protective_action(&simulation.population.agents[0], parent_position)
        .expect("a parent should go after a child across the map");

    match going {
        Action::Move { target } => {
            assert_eq!(target, (40, 40, 0), "and go to where the child is");
        }
        other => panic!("expected the parent to set off, got {other:?}"),
    }

    // Somebody else's child is somebody else's business
    simulation.population.agents[1].parent_ids.clear();
    assert!(
        simulation
            .protective_action(&simulation.population.agents[0], parent_position)
            .is_none(),
        "a parent goes after its own"
    );
}

/// Children pick things up by watching, and most from their own parents.
#[test]
fn children_learn_by_watching_their_parents() {
    fn learned(from_a_parent: bool) -> u32 {
        let world = World::new(WorldConfig::default());
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        population.spawn_agent(AgentConfig::default());

        let mut simulation = Simulation::new(world, population);

        let adult_id = simulation.population.agents[0].id;
        simulation.population.agents[0].state.position = (25, 25, 0);
        simulation.population.agents[0].state.age = 4000;
        simulation.population.agents[0].update_life_stage();

        {
            let child = &mut simulation.population.agents[1];
            child.state.position = (26, 25, 0);
            child.state.age = 700;
            child.update_life_stage();
            if from_a_parent {
                child.parent_ids = vec![adult_id];
            }
        }
        simulation.population.agents[1].update_life_stage();

        // The adult works within sight of the child, over and over
        for tick in 0..40 {
            simulation.population.update_who_can_see_whom();
            simulation.population.broadcast_action(
                adult_id,
                (25, 25, 0),
                crate::agents::observational_learning::ActionType::Farming,
                true,
                "TillSoil".to_string(),
                tick as u64,
            );
        }

        // Levels and the experience left over: `gain_experience` rolls every
        // hundred points into a level, so the residue alone says nothing
        simulation.population.agents[1]
            .skills
            .get_skill_if_exists(SkillType::Farming)
            .map(|skill| ((skill.level + 10) as u32) * 100 + skill.experience)
            .unwrap_or(0)
    }

    let from_parent = learned(true);
    let from_a_stranger = learned(false);

    assert!(
        from_a_stranger > 0,
        "a child should pick something up watching any adult work"
    );
    assert!(
        from_parent > from_a_stranger,
        "and more from its own parent: {from_parent} against {from_a_stranger}"
    );
}

/// Agents can see one another at all.
///
/// Nothing populated `vision.visible_agents`, and observation is gated on it,
/// so the whole observational learning system ran every twenty ticks over an
/// empty list and no agent had ever recorded seeing another do anything.
#[test]
fn agents_can_see_each_other() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (4, 4, 0);
    simulation.population.agents[1].state.position = (6, 4, 0);
    simulation.population.agents[2].state.position = (48, 48, 0);

    simulation.population.update_who_can_see_whom();

    let watcher = &simulation.population.agents[0];
    let neighbour = simulation.population.agents[1].id;
    let far_off = simulation.population.agents[2].id;

    assert!(
        watcher.senses.vision.visible_agents.contains(&neighbour),
        "an agent two tiles away should be in sight"
    );
    assert!(
        !watcher.senses.vision.visible_agents.contains(&far_off),
        "one across the map should not be"
    );

    // A blind agent sees nobody
    simulation.population.agents[0]
        .traits
        .add_trait(crate::core::traits::Trait::Blind);
    simulation.population.agents[0].apply_trait_sensory_modifications();
    simulation.population.update_who_can_see_whom();

    assert!(
        simulation.population.agents[0]
            .senses
            .vision
            .visible_agents
            .is_empty(),
        "a blind agent watches nobody, and learns by being told"
    );
}
