// src/analytics/tests/relationship_graph_tests.rs
//! Tests that what an agent feels about somebody reaches what it thinks of
//! them.
//!
//! `EmotionState` and `Relationship` kept separate books. A grudge lived in
//! `anger_sources`, was read by action selection and by nothing else, and
//! never touched the bond; a blow dealt damage, wrote anger and broke a bone
//! and left the relationship exactly where it found it. So a man who had just
//! been hit went on counting the man who hit him a close friend.
//!
//! And nothing could have shown through if it had. The proximity bonus added
//! up to 0.10 a tick with no ceiling, so a bond saturated within a day of
//! standing beside somebody. Measured at fifteen thousand ticks before any of
//! this: 82 to 105 relationships apiece, nine in ten of them at 0.6 or
//! better, mean bond 0.901, and `RelationshipType::Rival` and `Enemy`
//! constructed nowhere outside a test file in the whole project's history.

use crate::agents::{
    AgentConfig, EmotionSource, LifeStage, Population, Relationship, RelationshipType,
};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn two_neighbours() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    for (index, position) in [(30, 30, 0), (31, 30, 0)].into_iter().enumerate() {
        let agent = &mut simulation.population.agents[index];
        agent.state.position = position;
        agent.state.life_stage = LifeStage::Adult;
        agent.state.health = 100.0;
        agent.traits = crate::core::traits::TraitSet::new();
    }
    simulation
}

/// Being near somebody makes them a familiar face and no more.
#[test]
fn standing_beside_a_man_for_a_year_does_not_make_him_a_friend() {
    let mut bond = Relationship::new_neutral(crate::core::dice::name(), 0);

    // A whole year of never once leaving his side
    for _ in 0..1152 {
        bond.keep_company(1.0);
    }

    assert!(
        (bond.bond_strength - Relationship::A_FAMILIAR_FACE).abs() < 0.001,
        "proximity alone should stop at a familiar face, not run to {:.2}",
        bond.bond_strength
    );
    assert!(
        !bond.is_loved_one(),
        "and certainly should not make somebody you would grieve for"
    );
}

/// But it does get you there.
#[test]
fn and_it_does_make_him_a_familiar_face() {
    let mut bond = Relationship::new_neutral(crate::core::dice::name(), 0);
    let stranger = bond.bond_strength;

    // A season of it
    for _ in 0..288 {
        bond.keep_company(1.0);
    }

    assert!(
        bond.bond_strength > stranger,
        "a season of somebody's company should count for something"
    );
}

/// A grudge pulls the bond down, and beats standing next to them.
#[test]
fn a_grudge_outweighs_being_near_somebody() {
    let them = crate::core::dice::name();

    let mut resented = Relationship::new_neutral(them, 0);
    let mut ignored = Relationship::new_neutral(them, 0);

    // A month of the two of them being thrown together, one of them nursing
    // something the whole time
    for _ in 0..288 {
        resented.keep_company(1.0);
        resented.let_it_tell(0.8);
        ignored.keep_company(1.0);
    }

    assert!(
        resented.bond_strength < 0.0,
        "a man you cannot stand should not become a friend because you keep \
         finding yourself beside him; the bond stood at {:.2}",
        resented.bond_strength
    );
    assert!(
        ignored.bond_strength > resented.bond_strength,
        "and the same company without the grudge should go the other way"
    );
}

/// Nursing nothing changes nothing.
#[test]
fn holding_nothing_against_somebody_costs_them_nothing() {
    let mut bond = Relationship::new_neutral(crate::core::dice::name(), 0);
    bond.strengthen(0.4);
    let before = bond.bond_strength;

    for _ in 0..100 {
        bond.let_it_tell(0.0);
    }

    assert_eq!(bond.bond_strength, before);
}

/// What two people are follows what they think of each other.
#[test]
fn what_two_people_are_follows_what_they_think_of_each_other() {
    let mut bond = Relationship::new(crate::core::dice::name(), RelationshipType::Acquaintance);

    // Acquaintances start at 0.2
    bond.strengthen(0.5);
    bond.settle_what_we_are();
    assert_eq!(bond.relationship_type, RelationshipType::Friend);

    // 0.7 down to -0.3
    bond.weaken(1.0);
    bond.settle_what_we_are();
    assert_eq!(
        bond.relationship_type,
        RelationshipType::Rival,
        "a bond gone sour makes a rival; it stood at {:.2}",
        bond.bond_strength
    );

    // and on down to -0.7
    bond.weaken(0.4);
    bond.settle_what_we_are();
    assert_eq!(
        bond.relationship_type,
        RelationshipType::Enemy,
        "and one gone right through the floor makes an enemy"
    );
}

/// Blood is not renamed.
#[test]
fn a_brother_you_cannot_stand_is_still_a_brother() {
    let mut bond = Relationship::new(crate::core::dice::name(), RelationshipType::Sibling);

    bond.weaken(1.5);
    bond.settle_what_we_are();

    assert_eq!(
        bond.relationship_type,
        RelationshipType::Sibling,
        "however the two of them get on"
    );
    assert!(bond.bond_strength < -0.6, "which is not the same as getting on");
}

/// Being struck decides what two people are to each other.
#[test]
fn a_blow_lands_on_the_relationship_as_well_as_the_body() {
    let mut simulation = two_neighbours();
    let attacker = simulation.population.agents[0].id;
    let struck = simulation.population.agents[1].id;

    // They were friends before it
    for index in 0..2 {
        let other = if index == 0 { struck } else { attacker };
        let bond = simulation.population.agents[index]
            .relationships
            .get_or_create_relationship(other, 0);
        bond.strengthen(0.8);
        bond.settle_what_we_are();
    }
    assert!(simulation.population.agents[1]
        .relationships
        .get_relationship(&attacker)
        .expect("they know each other")
        .is_loved_one());

    simulation.execute_action(
        &Action::Attack {
            target_agent_id: struck,
            weapon: None,
        },
        0,
    );

    let after = simulation.population.agents[1]
        .relationships
        .get_relationship(&attacker)
        .expect("they still know each other");
    assert!(
        after.bond_strength < 0.8 - 0.2,
        "being hit should cost the man who did it dearly; the bond stood at \
         {:.2}",
        after.bond_strength
    );

    // And the one who threw it thinks less of them too
    let thrower = simulation.population.agents[0]
        .relationships
        .get_relationship(&struck)
        .expect("he knows who he hit");
    assert!(
        thrower.bond_strength < 0.8,
        "you do not warm to somebody you have just hit"
    );
}

/// Enough blows and they are enemies by name, not just by number.
#[test]
fn enough_of_them_and_they_are_enemies_by_name() {
    let mut simulation = two_neighbours();
    let attacker = simulation.population.agents[0].id;
    let struck = simulation.population.agents[1].id;

    for _ in 0..8 {
        simulation.population.agents[1].state.health = 100.0;
        simulation.execute_action(
            &Action::Attack {
                target_agent_id: struck,
                weapon: None,
            },
            0,
        );
    }

    let after = simulation.population.agents[1]
        .relationships
        .get_relationship(&attacker)
        .expect("he will not have forgotten");
    assert_eq!(
        after.relationship_type,
        RelationshipType::Enemy,
        "eight beatings should make an enemy, not an acquaintance; the bond \
         stood at {:.2}",
        after.bond_strength
    );
}

/// A grudge reaches the bond through the whole tick, wherever the two of them
/// are standing.
#[test]
fn a_grudge_reaches_the_bond_from_across_the_map() {
    let mut simulation = two_neighbours();
    let them = simulation.population.agents[1].id;

    // He has gone right over the hill, well out of anybody's sight
    simulation.population.agents[1].state.position = (90, 90, 0);

    let bond = simulation.population.agents[0]
        .relationships
        .get_or_create_relationship(them, 0);
    bond.strengthen(0.5);
    let before = bond.bond_strength;

    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.9);

    for _ in 0..40 {
        simulation.population.let_grudges_tell_on_the_bond();
    }

    let after = simulation.population.agents[0]
        .relationships
        .get_relationship(&them)
        .expect("out of sight is not out of mind");
    assert!(
        after.bond_strength < before,
        "resenting a man does not require him to be standing there: {before:.2} \
         became {:.2}",
        after.bond_strength
    );
}

/// A settlement ends up with people in it who dislike each other.
#[test]
fn a_settlement_ends_up_with_enemies_in_it() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..25 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    for _ in 0..4000 {
        simulation.tick();
    }

    let (mut named, mut soured) = (0usize, 0usize);
    for agent in simulation
        .population
        .agents
        .iter()
        .filter(|a| a.state.is_alive)
    {
        for bond in agent.relationships.get_all().values() {
            if matches!(
                bond.relationship_type,
                RelationshipType::Rival | RelationshipType::Enemy
            ) {
                named += 1;
            }
            if bond.bond_strength < 0.0 {
                soured += 1;
            }
        }
    }

    assert!(
        soured > 0,
        "in four thousand ticks somebody should have fallen out with somebody"
    );
    assert!(
        named > 0,
        "and it should show in what they are to each other, not only in a \
         number nothing reads: {soured} soured bonds and {named} named as \
         rival or enemy"
    );
}
