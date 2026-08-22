// src/analytics/tests/survival_pressure_tests.rs
//! Tests for a settlement that has to reckon with what it is doing to itself.
//!
//! Thirty thousand ticks of tracing showed a settlement that overshoots does
//! not correct - it slides. Four things were missing, and all four are here:
//! ground that carries less as it is worked out, a need that presses harder
//! the longer it is denied, breeding that waits for a surplus rather than for
//! a full stomach, and somewhere else to go when the ground has stopped
//! giving. Children, who have no reserves to speak of, now feel a famine
//! before the adults around them do.

use crate::agents::practices::{Lessons, Undertaking};
use crate::agents::{Agent, AgentConfig, InventoryItem, LifeStage, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, Position, ResourceNode, ResourceType, World, WorldConfig};

fn fed_adult() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.age = 4000;
    agent.update_life_stage();
    agent
}

fn give_food(agent: &mut Agent, quantity: u32) {
    let database = FoodDatabase::new();
    let mut item = InventoryItem::new_with_weight("food".to_string(), quantity, 0.1);
    item.food_data = database.create_food_data(&ItemType::Food, 0);
    agent.inventory.add_item(item);
}

/// Ground worked out carries a smaller crop, not merely a slower one.
#[test]
fn the_crop_falls_with_the_ground() {
    let field = ResourceNode::new(ResourceType::Grain, Position::new(5, 5), 80);

    let fresh = field.standing_capacity(0.55);
    let tired = field.standing_capacity(0.25);
    let spent = field.standing_capacity(0.03);

    assert!(fresh > tired && tired > spent, "{fresh} {tired} {spent}");
    assert!(
        spent < fresh / 4,
        "ground worked from 0.55 to 0.03 should lose most of its yield: {fresh} to {spent}"
    );
}

/// A person who has been hungry for days acts on it more single-mindedly than
/// one who missed a meal.
#[test]
fn hunger_that_is_ignored_takes_an_agent_over() {
    let mut patient = fed_adult();
    let mut desperate = fed_adult();

    for agent in [&mut patient, &mut desperate] {
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.8;
        }
    }

    // One of them is left hungry for ten days of world time
    for _ in 0..120 {
        if let Some(hunger) = desperate.drives.get_mut(DriveType::Hunger) {
            hunger.tick();
            hunger.value = 0.8;
        }
    }

    let patient_hunger = patient.drives.get(DriveType::Hunger).unwrap();
    let desperate_hunger = desperate.drives.get(DriveType::Hunger).unwrap();

    assert!(
        desperate_hunger.urgency() > patient_hunger.urgency() * 2.0,
        "ten days of going without should make a far louder case: {:.2} against {:.2}",
        desperate_hunger.urgency(),
        patient_hunger.urgency()
    );
}

/// Nobody has a child on the strength of one good meal.
#[test]
fn a_child_waits_on_a_surplus_and_not_on_a_full_stomach() {
    let mut just_eaten = fed_adult();
    let mut provided_for = fed_adult();
    give_food(&mut provided_for, 12);

    assert!(
        !just_eaten.should_attempt_reproduction(),
        "a full stomach and an empty pack is not a plan"
    );
    assert!(
        provided_for.should_attempt_reproduction(),
        "food in hand for two is"
    );

    // And somebody who has been going short recently does not, however much
    // they happen to be carrying now
    give_food(&mut just_eaten, 12);
    if let Some(hunger) = just_eaten.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.9;
        for _ in 0..40 {
            hunger.tick();
            hunger.value = 0.9;
        }
        hunger.value = 0.1;
    }

    assert!(
        !just_eaten.should_attempt_reproduction(),
        "a stretch of going short should still be telling"
    );
}

/// A famine takes the young before it takes the grown.
#[test]
fn a_hungry_year_takes_the_children_first() {
    fn health_after_famine(age: u32) -> f32 {
        let mut agent = Agent::new(AgentConfig::default());
        agent.state.age = age;
        agent.update_life_stage();
        agent.state.health = 100.0;
        agent.state.energy = 100.0;
        agent.state.last_ate_tick = 0;

        // Nobody eats for two thousand ticks
        for tick in 1..=2000u32 {
            agent.state.age_tick_with_modifier(tick, 1.0);
        }

        agent.state.health
    }

    let child = health_after_famine(900);
    let adult = health_after_famine(4000);

    assert!(
        child < adult,
        "a child should suffer a famine sooner than an adult: {child:.1} against {adult:.1}"
    );
    assert_eq!(
        LifeStage::from_age(900),
        LifeStage::Child,
        "the fixture should be testing a child"
    );
    assert!(
        LifeStage::Child.hunger_reserve() < LifeStage::Adult.hunger_reserve(),
        "a small body has less put by than a grown one"
    );
}

/// Left hungry long enough, an agent gives up on the country it is in.
#[test]
fn a_starving_agent_walks_out_of_country_that_will_not_feed_it() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.age = 4000;
    simulation.population.agents[0].update_life_stage();
    simulation.population.agents[0].state.position = (25, 25, 0);

    let here = simulation.population.agents[0].state.position;

    // Hungry, but only just: nobody abandons a settlement over one missed meal
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 0.9;
    }

    let agent = &simulation.population.agents[0];
    assert!(
        simulation.migration_action(agent, here).is_none(),
        "one hungry afternoon is not a reason to leave"
    );

    // Ten days of being hungry and not being fed
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        for _ in 0..130 {
            hunger.tick();
            hunger.value = 0.9;
        }
    }

    let agent = &simulation.population.agents[0];
    let leaving = simulation
        .migration_action(agent, here)
        .expect("ten days of going hungry should send somebody looking elsewhere");

    match leaving {
        crate::environment::Action::Move { target } => {
            let distance = (target.0 - here.0).abs().max((target.1 - here.1).abs());
            assert!(
                distance >= 15,
                "leaving means going somewhere else, not next door: {here:?} to {target:?}"
            );
        }
        other => panic!("expected to be walking somewhere, got {other:?}"),
    }
}

/// Somebody who is being fed does not wander off.
#[test]
fn a_fed_agent_stays_where_it_is() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    let here = simulation.population.agents[0].state.position;

    let agent = &simulation.population.agents[0];
    assert!(
        simulation.migration_action(agent, here).is_none(),
        "a fed agent has no reason to go anywhere"
    );
}

/// An agent stops doing what has never once worked, and keeps at what does.
#[test]
fn an_agent_gives_up_on_what_never_works() {
    let mut unlucky = Lessons::new();
    let mut capable = Lessons::new();

    for _ in 0..12 {
        unlucky.record(Undertaking::Hunting, false);
        capable.record(Undertaking::Hunting, true);
    }

    assert!(
        !unlucky.worth_trying(Undertaking::Hunting),
        "twelve empty-handed hunts should be enough to stop"
    );
    assert!(
        capable.worth_trying(Undertaking::Hunting),
        "twelve kills should not be"
    );
    assert!(capable.belief(Undertaking::Hunting) > unlucky.belief(Undertaking::Hunting));
}

/// Nothing is written off before it has been tried.
#[test]
fn nothing_is_given_up_on_before_it_is_tried() {
    let lessons = Lessons::new();

    for undertaking in [
        Undertaking::Hunting,
        Undertaking::Cooking,
        Undertaking::Farming,
        Undertaking::Clothing,
    ] {
        assert!(
            lessons.worth_trying(undertaking),
            "{undertaking:?} should get the benefit of the doubt"
        );
    }

    // One bad result is not a pattern either
    let mut once_burnt = Lessons::new();
    once_burnt.record(Undertaking::Cooking, false);
    assert!(once_burnt.worth_trying(Undertaking::Cooking));
}

/// The agent can say what has served it best, without anybody having told it.
#[test]
fn an_agent_can_say_what_has_served_it_best() {
    let mut lessons = Lessons::new();

    for _ in 0..8 {
        lessons.record(Undertaking::Farming, true);
        lessons.record(Undertaking::Hunting, false);
        lessons.record(Undertaking::Cooking, true);
        lessons.record(Undertaking::Cooking, false);
    }

    assert_eq!(
        lessons.what_works_best(),
        Some(Undertaking::Farming),
        "the best record should be the one it names"
    );
}

/// Doing things in a running simulation actually writes to that record.
#[test]
fn what_agents_do_in_a_run_becomes_something_they_know() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..8 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..2000 {
        simulation.tick();
    }

    let anybody_learned_anything = simulation.population.agents.iter().any(|agent| {
        [
            Undertaking::Hunting,
            Undertaking::Cooking,
            Undertaking::Farming,
            Undertaking::Clothing,
            Undertaking::Foraging,
            Undertaking::Building,
            Undertaking::Crafting,
            Undertaking::Dealing,
        ]
        .iter()
        .any(|undertaking| agent.lessons.attempts(*undertaking) > 0)
    });

    assert!(
        anybody_learned_anything,
        "two thousand ticks of doing things should leave a record of having done them"
    );
}
