// src/analytics/tests/sight_tests.rs
//! Tests for sight: what agents discover by looking, and what blindness takes
//! away.
//!
//! Until this was wired up, `process_exploration_with_world` had no callers and
//! agents perceived the world by smell alone. These cover:
//! - sighted agents discover terrain and resources around them
//! - the Blind trait removes that entirely, leaving smell and word of mouth
//! - what an agent sees reaches its memory, so sight feeds foraging

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::memory::SpatialMemoryType;
use crate::core::traits::Trait;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// Blindness zeroes sight; ordinary agents can see.
#[test]
fn blind_agents_have_no_sight_range() {
    let mut agent = crate::agents::Agent::new(AgentConfig::default());
    assert!(agent.can_see(), "an ordinary agent should be able to see");
    assert!(agent.sight_range() > 0);

    agent.traits.add_trait(Trait::Blind);
    agent.apply_trait_sensory_modifications();

    assert!(!agent.can_see(), "a blind agent should not be able to see");
    assert_eq!(agent.sight_range(), 0);
    assert_eq!(agent.senses.vision.acuity, 0.0);
}

/// Traits have to reach the senses when an agent is created.
///
/// `apply_trait_sensory_modifications` had no callers at all, so a Deaf or
/// Blind agent was born with perfect senses.
#[test]
fn sensory_traits_are_applied_at_creation() {
    let mut config = AgentConfig::default();
    config.random_weights = false;

    // Whatever traits an agent is created with must already be in effect
    let agent = crate::agents::Agent::new(config);

    if agent.traits.has(Trait::Blind) {
        assert_eq!(agent.senses.vision.acuity, 0.0);
    }
    if agent.traits.has(Trait::Deaf) {
        assert_eq!(agent.senses.hearing.sensitivity, 0.0);
    }
}

/// A sighted agent discovers the ground around it; a blind one does not.
#[test]
fn sight_discovers_the_world_and_blindness_does_not() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    {
        let blind = &mut simulation.population.agents[1];
        blind.traits.add_trait(Trait::Blind);
        blind.apply_trait_sensory_modifications();
    }

    for _ in 0..200 {
        simulation.tick();
    }

    let seeing = simulation.population.agents[0]
        .exploration_knowledge
        .explored_tiles
        .len();
    let blind = simulation.population.agents[1]
        .exploration_knowledge
        .explored_tiles
        .len();

    assert!(
        seeing > 0,
        "a sighted agent should have discovered ground around it"
    );
    assert_eq!(
        blind, 0,
        "a blind agent should discover nothing by sight, saw {blind} tiles"
    );
}

/// What an agent sees has to reach the memory foraging reads.
///
/// Discovery writes into `exploration_knowledge`, which the food seeking code
/// never consults, so without the bridge an agent would catalogue a berry patch
/// it had seen and still walk past it hungry. Smell is disabled here so only
/// sight can account for the memory.
#[test]
fn spotted_food_is_remembered() {
    let mut world = World::new(WorldConfig::default());

    // Clear the ground, then put a single patch of food within sight.
    // Note `place_resource_node` fills a different store (`resource_nodes`,
    // used by spatial planning) than the one sight and foraging read.
    world.resources.clear();
    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(5, 0),
        50,
    ));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    {
        let agent = &mut simulation.population.agents[0];
        agent.state.position = (0, 0, 0);
        // No sense of smell, so anything remembered was seen
        agent.senses.smell.sensitivity = 0.0;
    }

    for _ in 0..20 {
        simulation.tick();
    }

    let remembered = simulation.population.agents[0]
        .memory
        .recall_locations(SpatialMemoryType::Food);

    assert!(
        !remembered.is_empty(),
        "an agent that saw food should remember where it is"
    );
}

/// A blind agent with no sense of smell finds nothing by itself.
#[test]
fn blind_agents_do_not_remember_unseen_food() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(5, 0),
        50,
    ));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    {
        let agent = &mut simulation.population.agents[0];
        agent.state.position = (0, 0, 0);
        agent.senses.smell.sensitivity = 0.0;
        agent.traits.add_trait(Trait::Blind);
        agent.apply_trait_sensory_modifications();
    }

    for _ in 0..20 {
        simulation.tick();
    }

    let remembered = simulation.population.agents[0]
        .memory
        .recall_locations(SpatialMemoryType::Food);

    assert!(
        remembered.is_empty(),
        "an agent that can neither see nor smell should not know where food is"
    );
}

/// Edibility has one definition, used by sight, smell and foraging alike.
#[test]
fn edible_resources_are_agreed_on() {
    assert!(ResourceType::Food.is_edible());
    assert!(ResourceType::Grain.is_edible());
    assert!(ResourceType::Fish.is_edible());
    assert!(ResourceType::Meat.is_edible());

    assert!(!ResourceType::Wood.is_edible());
    assert!(!ResourceType::Stone.is_edible());
    assert!(!ResourceType::Water.is_edible(), "water is drunk, not eaten");
}

/// Sight reaches the resources it can see, not just the tiles.
#[test]
fn sight_finds_resources_not_only_ground() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.resources.push(ResourceNode::new(
        ResourceType::Stone,
        Position::new(4, 0),
        50,
    ));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (0, 0, 0);

    for _ in 0..20 {
        simulation.tick();
    }

    let known = &simulation.population.agents[0]
        .exploration_knowledge
        .known_resources;

    assert!(
        known.contains_key(&Position::new(4, 0)),
        "an agent should notice a stone deposit four tiles away"
    );
}
