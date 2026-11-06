// src/analytics/tests/trait_technology_tests.rs
//! Technology moving through personalities (ISSUES_FOUND #390).
//!
//! - ideas come more readily to the curious and the imaginative, less to the
//!   stubborn and the traditional, at a people's pace; at the development
//!   pace to everybody alike
//! - the curious and a copycat learn more readily, the stubborn less; an
//!   extrovert teaches more readily, an introvert and the mute less
//! - an eager child with a willing teacher learns more in a year than a
//!   reluctant child with an unwilling one
//! - a lesson makes teacher and taught warmer to each other
//! - a proud finder is pleased, and the envious of the camp resent them

use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::discovery;
use crate::core::traits::{Trait, TraitSet};
use crate::environment::seasons::TICKS_PER_DAY;
use crate::world::{World, WorldConfig};

fn somebody(traits: &[Trait]) -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits = TraitSet::new();
    for &t in traits {
        agent.traits.add_trait(t);
    }
    agent
}

/// Twice as many ideas to the curious, half to the stubborn, at a people's
/// pace; at the development pace, each at its count whoever has it.
#[test]
fn ideas_come_more_readily_to_the_curious() {
    assert_eq!(somebody(&[]).how_inventive_i_am(), 1.0);
    assert_eq!(somebody(&[Trait::Curious]).how_inventive_i_am(), 2.0);
    assert_eq!(somebody(&[Trait::Stubborn]).how_inventive_i_am(), 0.5);

    discovery::set(1_000.0);
    let count = |inventive: f64| {
        crate::core::dice::seed(5);
        (0..200_000).filter(|_| discovery::does_it_occur_now_for(10, inventive)).count()
    };
    let (plain, curious, stubborn) = (count(1.0), count(2.0), count(0.5));
    discovery::set(discovery::THE_DEVELOPMENT_PACE);
    assert!(curious > plain * 3 / 2 && plain > stubborn * 3 / 2, "{curious} {plain} {stubborn}");

    assert!(discovery::does_it_occur_now_for(10, 0.5), "at the development pace, to the stubborn too");
}

/// Who learns and who teaches readily.
#[test]
fn who_learns_and_who_teaches_readily() {
    assert!(somebody(&[Trait::Copycat]).how_readily_i_learn() > 1.0);
    assert!(somebody(&[Trait::Traditionalist]).how_readily_i_learn() < 1.0);
    assert!(somebody(&[Trait::Extrovert]).how_readily_i_teach() > 1.0);
    assert!(somebody(&[Trait::Introvert]).how_readily_i_teach() < 1.0);
    assert!(somebody(&[Trait::Mute]).how_readily_i_teach() < 1.0);
}

/// A grown teacher knowing forty things and a child at the same hearth, each
/// with the traits given, after a year of days.
fn a_year_of_teaching(teacher: &[Trait], child: &[Trait], seed: u64) -> (Simulation, usize) {
    crate::core::dice::seed(seed);
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let parent = {
        let t = &mut simulation.population.agents[0];
        t.traits = TraitSet::new();
        for &x in teacher {
            t.traits.add_trait(x);
        }
        t.state.now_this_many_years_old(35);
        t.hearth = Some((25, 25));
        for n in 0..40 {
            t.found_out_how_to(&format!("a thing worth knowing {n}"));
        }
        t.id
    };
    let c = &mut simulation.population.agents[1];
    c.traits = TraitSet::new();
    for &x in child {
        c.traits.add_trait(x);
    }
    c.state.now_this_many_years_old(9);
    c.hearth = Some((25, 25));
    c.parent_ids.push(parent);
    for day in 1..=365 {
        simulation.current_turn = day * TICKS_PER_DAY;
        simulation.the_young_are_taught();
    }
    let learned = simulation.world.lessons_taught as usize;
    (simulation, learned)
}

/// An eager child with a willing teacher learns more in a year than a
/// reluctant child with an unwilling one.
#[test]
fn an_eager_child_with_a_willing_teacher_learns_more() {
    let eager: usize = (0..6).map(|seed| a_year_of_teaching(&[Trait::Extrovert], &[Trait::Curious], seed).1).sum();
    let reluctant: usize =
        (0..6).map(|seed| a_year_of_teaching(&[Trait::Introvert], &[Trait::Stubborn], seed).1).sum();
    assert!(eager > reluctant * 2, "{eager} against {reluctant}");
}

/// A lesson makes teacher and taught warmer to each other.
#[test]
fn a_lesson_warms_teacher_and_taught() {
    let (simulation, learned) = a_year_of_teaching(&[], &[], 3);
    assert!(learned > 0);
    let (teacher, child) = (&simulation.population.agents[0], &simulation.population.agents[1]);
    let warmth = |of: &Agent, to: &Agent| {
        of.relationships.get_relationship(&to.id).map(|r| r.bond_strength).unwrap_or(0.0)
    };
    assert!(warmth(child, teacher) > 0.0, "{}", warmth(child, teacher));
    assert!(warmth(teacher, child) > 0.0, "{}", warmth(teacher, child));
}

/// A proud finder is pleased with what they came to; the envious of their camp
/// resent them for it; somebody at another camp does not hear of it.
#[test]
fn the_proud_are_pleased_and_the_envious_resent_it() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    for _ in 0..3 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for (index, (traits, home)) in
        [(vec![Trait::Proud], (25, 25)), (vec![Trait::Envious], (27, 25)), (vec![Trait::Envious], (90, 25))]
            .into_iter()
            .enumerate()
    {
        let agent = &mut simulation.population.agents[index];
        agent.traits = TraitSet::new();
        for t in traits {
            agent.traits.add_trait(t);
        }
        agent.state.now_this_many_years_old(30);
        agent.hearth = Some(home);
        agent.emotions.happiness = 0.0;
        agent.emotions.sadness = 0.0;
    }
    let finder = simulation.population.agents[0].id;
    assert!(simulation.population.agents[0].came_to_it_myself("know:coppice"));
    simulation.what_the_camp_makes_of_it();

    assert!(simulation.population.agents[0].emotions.happiness > 0.0, "pleased with it");
    let neighbour = &simulation.population.agents[1];
    assert!(neighbour.emotions.sadness > 0.0);
    assert!(neighbour.relationships.get_relationship(&finder).is_some_and(|r| r.bond_strength < 0.0));
    let far = &simulation.population.agents[2];
    assert_eq!(far.emotions.sadness, 0.0, "another camp's find is nothing to them");
    assert!(simulation.population.agents[0].lately_came_to.is_empty());
}
