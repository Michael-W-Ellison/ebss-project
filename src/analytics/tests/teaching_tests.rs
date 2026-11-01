// src/analytics/tests/teaching_tests.rs
//! The young are taught what their camp knows (ISSUES_FOUND #388).
//!
//! - a child learns, in time, what a parent knows that they do not
//! - and what a grown person of their camp knows, parent or not
//! - not what somebody at another camp knows
//! - a grown person is not taught this way

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::world::{World, WorldConfig};

/// A grown person who knows how to burn the country off, at `teacher_home`,
/// and a person of `age` at `learner_home`.
fn a_teacher_and(age: u32, teacher_home: (i32, i32), learner_home: (i32, i32), a_parent: bool) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let teacher_id = {
        let teacher = &mut simulation.population.agents[0];
        teacher.state.now_this_many_years_old(35);
        teacher.hearth = Some(teacher_home);
        teacher.state.position = (teacher_home.0, teacher_home.1, 0);
        teacher.found_out_how_to("do:BurnOff");
        teacher.id
    };
    let learner = &mut simulation.population.agents[1];
    learner.state.now_this_many_years_old(age);
    learner.hearth = Some(learner_home);
    learner.state.position = (learner_home.0, learner_home.1, 0);
    if a_parent {
        learner.parent_ids.push(teacher_id);
    }
    simulation
}

/// Days of teaching, one pass a day.
fn days_of_it(simulation: &mut Simulation, days: u32) {
    for day in 1..=days {
        simulation.current_turn = day * TICKS_PER_DAY;
        simulation.the_young_are_taught();
    }
}

fn knows(simulation: &Simulation) -> bool {
    simulation.population.agents[1].what_i_found_out().contains("do:BurnOff")
}

/// A child is taught, in time, what a parent knows.
#[test]
fn a_child_learns_what_a_parent_knows() {
    crate::core::dice::seed(7);
    let mut simulation = a_teacher_and(8, (25, 25), (25, 25), true);
    assert!(!knows(&simulation));
    days_of_it(&mut simulation, 365);
    assert!(knows(&simulation), "a year growing up beside it");
    assert_eq!(simulation.world.lessons_taught, 1);
}

/// And what a grown person of the camp knows, parent or not.
#[test]
fn a_child_learns_what_the_camp_knows() {
    crate::core::dice::seed(7);
    let mut simulation = a_teacher_and(12, (25, 25), (30, 25), false);
    days_of_it(&mut simulation, 365);
    assert!(knows(&simulation));
}

/// Not what somebody at another camp knows.
#[test]
fn nobody_is_taught_from_another_camp() {
    crate::core::dice::seed(7);
    let mut simulation = a_teacher_and(8, (25, 25), (90, 25), false);
    days_of_it(&mut simulation, 365);
    assert!(!knows(&simulation));
}

/// A grown person picks things up by watching and doing, not this way.
#[test]
fn a_grown_person_is_not_taught_this_way() {
    crate::core::dice::seed(7);
    let mut simulation = a_teacher_and(30, (25, 25), (25, 25), false);
    days_of_it(&mut simulation, 365);
    assert!(!knows(&simulation));
}
