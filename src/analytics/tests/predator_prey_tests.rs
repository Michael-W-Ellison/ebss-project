// src/analytics/tests/predator_prey_tests.rs
//! Tests for the predator-prey balance, and for what a hungry predator does
//! about the people living next to it.
//!
//! The fauna model had all the parts and none of the connections. Predation
//! sat behind a single roll for the whole world each tick, predators were
//! spawned that could not eat anything living in that world, and nothing let
//! an animal touch an agent. Herds grew until they hit the hard population
//! cap. These cover:
//! - a world only gets predators that can live off what is in it
//! - a hungry predator hunts on its own account, and thins a herd
//! - a starving one takes prey outside its usual list
//! - grass is finite, so a herd has a size the land will carry
//! - a hungry predator beside a settlement turns on the people

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// A world is stocked with predators that can eat what lives in it.
///
/// Drawing the two lists independently put foxes, which eat rabbits and
/// squirrels, into worlds of sheep and cattle. Their hunger climbed in a
/// straight line from birth to death and the herds ran to the population cap
/// unopposed.
#[test]
fn predators_can_live_off_what_the_world_holds() {
    let mut worlds_with_predators = 0;
    let mut fed_predators = 0;

    for _ in 0..8 {
        let world = World::new(WorldConfig::default());

        let present: std::collections::HashSet<String> = world
            .animals
            .get_all()
            .iter()
            .map(|animal| animal.species_id.clone())
            .collect();

        let predators: Vec<_> = world
            .animals
            .get_all()
            .iter()
            .filter_map(|animal| world.animals.get_species(&animal.species_id))
            .filter(|species| !species.prey_species.is_empty())
            .collect();

        if predators.is_empty() {
            continue;
        }
        worlds_with_predators += 1;

        if predators.iter().all(|species| {
            species
                .prey_species
                .iter()
                .any(|prey| present.contains(prey))
        }) {
            fed_predators += 1;
        }
    }

    assert!(
        worlds_with_predators > 0,
        "a default world should be stocked with predators"
    );
    assert_eq!(
        fed_predators, worlds_with_predators,
        "every predator spawned should have something in the world to eat"
    );
}

/// Predators thin a herd. Left alone, the herd does not stop growing.
#[test]
fn predators_hold_a_herd_down() {
    fn herd_after(with_predators: bool, ticks: u32) -> usize {
        let mut world = World::new(WorldConfig::default());
        world.animals.get_all_mut().clear();

        for i in 0..12 {
            let _ = world.spawn_animal("sheep".to_string(), (20 + i % 4, 20 + i / 4));
        }
        if with_predators {
            for i in 0..6 {
                let _ = world.spawn_animal("wolf".to_string(), (21 + i % 3, 21 + i / 3));
            }
        }

        let mut simulation = Simulation::new(world, Population::new());
        for _ in 0..ticks {
            simulation.tick();
        }

        simulation
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && animal.species_id == "sheep")
            .count()
    }

    let unmolested = herd_after(false, 6000);
    let hunted = herd_after(true, 6000);

    assert!(
        unmolested > 12,
        "a herd nothing eats should grow, ended at {unmolested}"
    );
    assert!(
        hunted < unmolested,
        "wolves should hold the herd below what it reaches alone: {hunted} against {unmolested}"
    );
}

/// The land has a size of herd it will carry.
///
/// Grazing feeds an animal nearly a hundred times what it burns, so hunger
/// never became the limit on a herd: it grew until it hit the hard population
/// cap, however little ground it was standing on. Animals breed when there is
/// room around them to.
#[test]
fn the_land_will_only_carry_so_many() {
    fn herd_after(penned: bool) -> usize {
        let mut world = World::new(WorldConfig::default());
        world.animals.get_all_mut().clear();

        for i in 0..10 {
            let _ = world.spawn_animal("sheep".to_string(), (20 + i % 5, 20 + i / 5));
        }

        let mut simulation = Simulation::new(world, Population::new());
        for _ in 0..8000 {
            if penned {
                // All on one patch of ground, with nowhere to spread to
                for animal in simulation.world.animals.get_all_mut() {
                    animal.position = (25, 25);
                }
            }
            simulation.tick();
        }

        simulation
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive())
            .count()
    }

    let penned = herd_after(true);
    let roaming = herd_after(false);

    assert!(
        roaming > penned,
        "a herd with the run of the map should outgrow one on a single patch: {roaming} against {penned}"
    );
    assert!(
        penned <= 20,
        "a herd penned on one patch should stop growing, ended at {penned}"
    );
}

/// A starving predator takes whatever it can catch, not only what is on its
/// usual menu.
#[test]
fn a_starving_predator_takes_what_it_can_get() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    // Squirrels are not on a wolf's list of prey, and are well inside the
    // size of the deer that are
    for i in 0..8 {
        let _ = world.spawn_animal("squirrel".to_string(), (25 + i % 3, 25 + i / 3));
    }
    let wolf = world
        .spawn_animal("wolf".to_string(), (25, 25))
        .expect("a wolf should spawn");

    // Send it out already close to starving
    if let Some(animal) = world.animals.get_mut(&wolf) {
        animal.hunger = animal.max_hunger * 0.95;
    }

    let mut simulation = Simulation::new(world, Population::new());

    let before = simulation
        .world
        .animals
        .get_all()
        .iter()
        .filter(|animal| animal.is_alive() && animal.species_id == "squirrel")
        .map(|animal| animal.current_health)
        .sum::<f32>();

    for _ in 0..400 {
        // Keep them in sight of each other: this measures whether a starving
        // wolf will try a goat, not whether the two happened to drift apart
        for animal in simulation.world.animals.get_all_mut() {
            animal.position = (25, 25);
            if animal.species_id == "wolf" {
                animal.hunger = animal.max_hunger * 0.95;
                animal.current_health = animal.max_health;
            }
        }
        simulation.tick();
    }

    let after = simulation
        .world
        .animals
        .get_all()
        .iter()
        .filter(|animal| animal.is_alive() && animal.species_id == "squirrel")
        .map(|animal| animal.current_health)
        .sum::<f32>();

    assert!(
        after < before,
        "a starving wolf should have gone for the squirrels: {before} -> {after}"
    );
}

/// A hungry predator beside a settlement turns on the people.
///
/// This is where thinning the herds comes back on the settlement that did it:
/// nothing else in the model let an animal touch an agent, so a wolf could
/// starve in the middle of a village.
#[test]
fn a_hungry_predator_turns_on_the_settlement() {
    fn maulings(hungry: bool) -> usize {
        let mut world = World::new(WorldConfig::default());
        world.animals.get_all_mut().clear();

        let mut wolves = Vec::new();
        for i in 0..8 {
            if let Ok(wolf) = world.spawn_animal("wolf".to_string(), (25, 25)) {
                wolves.push(wolf);
            }
            let _ = i;
        }

        for wolf in &wolves {
            if let Some(animal) = world.animals.get_mut(wolf) {
                animal.hunger = if hungry {
                    animal.max_hunger * 0.98
                } else {
                    0.0
                };
            }
        }

        let mut population = Population::new();
        for _ in 0..6 {
            population.spawn_agent(AgentConfig::default());
        }

        let mut simulation = Simulation::new(world, population);
        for agent in &mut simulation.population.agents {
            agent.state.position = (25, 25, 0);
        }

        let mut attacked = 0;
        for _ in 0..300 {
            // Keep everyone in the same place so this measures the decision,
            // not who happened to wander off
            for agent in &mut simulation.population.agents {
                agent.state.position = (25, 25, 0);
            }
            for wolf in &wolves {
                if let Some(animal) = simulation.world.animals.get_mut(wolf) {
                    animal.position = (25, 25);
                    animal.hunger = if hungry {
                        animal.max_hunger * 0.98
                    } else {
                        0.0
                    };
                }
            }

            simulation.tick();

            attacked += simulation
                .population
                .agents
                .iter()
                .filter(|agent| {
                    agent
                        .emotions
                        .recent_attacker(simulation.current_tick)
                        .map(|id| wolves.contains(&id))
                        .unwrap_or(false)
                })
                .count();
        }

        attacked
    }

    let starving = maulings(true);
    let well_fed = maulings(false);

    assert!(
        starving > 0,
        "starving wolves standing among people should attack them"
    );
    assert!(
        starving > well_fed,
        "hunger is what changes a wolf's mind: {starving} against {well_fed}"
    );
}

/// The world is ticked once per tick.
///
/// Simulation::tick used to advance climate, fauna and flora itself and then
/// call World::tick, which does all three again, so the living world ran at
/// double speed against the agents living in it.
#[test]
fn the_world_advances_once_per_tick() {
    let world = World::new(WorldConfig::default());
    let mut simulation = Simulation::new(world, Population::new());

    let before = simulation.world.tick;
    for _ in 0..50 {
        simulation.tick();
    }

    assert_eq!(
        simulation.world.tick - before,
        50,
        "the world should advance one tick per simulation tick"
    );
}
