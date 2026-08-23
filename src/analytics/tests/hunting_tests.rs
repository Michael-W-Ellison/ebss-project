// src/analytics/tests/hunting_tests.rs
//! Tests for hunting: what an agent will take on, what it has to get close
//! enough to do, and what a kill is worth.
//!
//! Nothing in a run had ever hunted. `Action::Hunt` and the whole fauna model
//! worked, and the one place the action appeared passed a nil animal id that
//! the executor could not resolve, so meat, hides and wool never reached an
//! inventory and the warm half of the garment table was out of reach. These
//! cover:
//! - a kill is butchered into things the rest of the simulation understands
//! - a hunter has to be next to the animal
//! - an unarmed agent leaves the dangerous animals alone
//! - an agent hunts for skins it needs, and stops once it has them

use crate::agents::storage_integration::butchered_item_id;
use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// Every named cut and skin lands in the inventory as something the nutrition
/// database, the cooking rules and the garment table already understand.
#[test]
fn a_kill_is_butchered_into_things_agents_understand() {
    for cut in [
        "mutton",
        "beef",
        "pork",
        "deer_meat",
        "rabbit_meat",
        "bear_meat",
        "elk_meat",
        "blubber",
    ] {
        assert_eq!(butchered_item_id(cut), "meat", "{cut} should butcher to meat");
    }

    assert_eq!(butchered_item_id("fish_meat"), "fish");

    for skin in ["fur", "thick_hide", "snake_skin"] {
        assert_eq!(butchered_item_id(skin), "hides", "{skin} should be a hide");
    }

    // Things that already have a name keep it
    assert_eq!(butchered_item_id("leather"), "leather");
    assert_eq!(butchered_item_id("wool"), "wool");

    // Trophies pass through: no use yet, and inventing one would be worse
    assert_eq!(butchered_item_id("antler"), "antler");
    assert_eq!(butchered_item_id("feathers"), "feathers");
}

/// A hunter has to be within a spear's throw.
///
/// Without this an agent could kill a deer on the far side of the map without
/// leaving where it stood.
#[test]
fn an_animal_across_the_map_cannot_be_hunted() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let deer = world
        .spawn_animal("deer".to_string(), (45, 45))
        .expect("a deer should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (10, 10, 0);

    let result = simulation.execute_action(
        &Action::Hunt {
            animal_id: deer,
            weapon: None,
        },
        0,
    );

    assert!(!result.success, "a deer across the map is not in reach");
    assert!(
        simulation
            .world
            .animals
            .get(&deer)
            .map(|animal| animal.is_alive())
            .unwrap_or(false),
        "the deer should be untouched"
    );
}

/// Standing next to it, a hunter eventually brings it home.
#[test]
fn a_kill_fills_the_pack_with_meat_and_skins() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let deer = world
        .spawn_animal("deer".to_string(), (30, 30))
        .expect("a deer should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;
    simulation.population.agents[0]
        .skills
        .set_skill_level(SkillType::Hunting, 8);

    // Even a practised hunter misses sometimes
    for _ in 0..40 {
        if simulation
            .world
            .animals
            .get(&deer)
            .map(|animal| !animal.is_alive())
            .unwrap_or(true)
        {
            break;
        }

        simulation.execute_action(
            &Action::Hunt {
                animal_id: deer,
                weapon: None,
            },
            0,
        );
    }

    let agent = &simulation.population.agents[0];
    let carried: Vec<&String> = agent.inventory.get_all_items().keys().collect();

    assert!(
        agent.inventory.has_item("meat", 1),
        "a deer should be worth some meat, carrying {carried:?}"
    );
    assert!(
        agent.inventory.has_item("leather", 1),
        "a deer should be worth some leather, carrying {carried:?}"
    );

    // And the meat is meat: it carries nutrition and can go on a fire
    let meat = agent
        .inventory
        .get_item("meat")
        .expect("the meat should be there");
    assert!(
        meat.food_data.is_some(),
        "meat off an animal should be worth eating"
    );
}

/// Hunting teaches hunting, whether or not the animal gets away.
///
/// The old code read MeleeCombat and had no floor on the odds: an untrained
/// agent has that skill at -10, and 0.5 + (-10 x 0.05) is zero, so the first
/// kill an agent ever made created the skill and left it unable to hunt again.
#[test]
fn hunting_teaches_hunting() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let rabbit = world
        .spawn_animal("rabbit".to_string(), (30, 30))
        .expect("a rabbit should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);

    let before = simulation.population.agents[0]
        .skills
        .get_skill_if_exists(SkillType::Hunting)
        .map(|skill| skill.experience)
        .unwrap_or(0);

    simulation.execute_action(
        &Action::Hunt {
            animal_id: rabbit,
            weapon: None,
        },
        0,
    );

    let after = simulation.population.agents[0]
        .skills
        .get_skill_if_exists(SkillType::Hunting)
        .map(|skill| skill.experience)
        .unwrap_or(0);

    assert!(
        after > before,
        "an attempt should teach something either way: {before} -> {after}"
    );
}

/// An unarmed agent leaves the bears alone.
#[test]
fn nobody_walks_up_to_a_bear_empty_handed() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    // The bear is the only animal in this world, so what the agent does is
    // about the bear and not about a rabbit two fields over
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("bear".to_string(), (30, 31))
        .expect("a bear should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].body_temperature.ideal = 45.0;

    let position = simulation.population.agents[0].state.position;
    let bear = simulation
        .world
        .animals
        .get_all()
        .iter()
        .find(|animal| animal.species_id == "bear")
        .expect("the bear should be there")
        .clone();

    assert!(
        !simulation.worth_hunting(&simulation.population.agents[0], &bear),
        "an unarmed agent should not take on a bear"
    );

    assert!(
        simulation
            .hunting_action(&simulation.population.agents[0], position)
            .is_none(),
        "and should not set out to"
    );
}

/// A cold agent with no skins goes after an animal; one that already has
/// enough stays where it is.
#[test]
fn an_agent_hunts_for_the_skins_it_needs() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    world
        .spawn_animal("deer".to_string(), (30, 33))
        .expect("a deer should spawn");

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;

    // Wants to be warmer than the weather will ever make it
    simulation.population.agents[0].body_temperature.ideal = 45.0;

    let position = simulation.population.agents[0].state.position;

    let going = simulation.hunting_action(&simulation.population.agents[0], position);
    assert!(
        matches!(going, Some(Action::Move { .. }) | Some(Action::Hunt { .. })),
        "a cold agent with no skins should go after the deer, got {going:?}"
    );

    // Give it more hides than any garment asks for
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("hides".to_string(), 30, 2.0));

    assert!(
        simulation
            .hunting_action(&simulation.population.agents[0], position)
            .is_none(),
        "an agent with a pack full of hides has no reason to hunt"
    );
}

/// Skins off an animal become the warm clothing that plants cannot make.
#[test]
fn skins_become_the_warm_clothing() {
    use crate::agents::equipment::garment_recipe;

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("hides".to_string(), 20, 2.0));
        agent.body_temperature.ideal = 45.0;
    }

    let position = simulation.population.agents[0].state.position;
    let making = simulation
        .clothing_action(&simulation.population.agents[0], position, true)
        .expect("a cold agent with hides should make something of them");

    let garment = match &making {
        Action::MakeClothing { garment } => garment.clone(),
        other => panic!("expected to make a garment, got {other:?}"),
    };

    let recipe = garment_recipe(&garment).expect("the garment should have a recipe");
    assert_eq!(
        recipe.material_item, "hides",
        "the hides should be what it reaches for"
    );

    simulation.execute_action(&making, 0);

    let agent = &simulation.population.agents[0];
    assert!(
        agent.body.total_cold_insulation() > 0.4,
        "fur should be warmer than anything woven from flax, got {:.2}",
        agent.body.total_cold_insulation()
    );
}
