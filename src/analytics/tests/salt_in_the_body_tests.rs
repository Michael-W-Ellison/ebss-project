// src/analytics/tests/salt_in_the_body_tests.rs
//! Salt in the body, and journeys for it (#356): sweated out, put back by
//! flesh and fish and salt, craved when short and weakening when gone; and a
//! body craving it goes for salt it knows of, on a journey if it lies beyond
//! a day's forage.

use crate::agents::{AgentConfig, InventoryItem, LifeStage, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// One grown person at (25, 25) on open plains three hundred cells across,
/// fed and watered, living where they stand.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig { size: (300, 300), ..WorldConfig::default() });
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    for x in 0..world.grid.width as i32 {
        for y in 0..world.grid.height as i32 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(25);
    agent.state.position = (25, 25, 0);
    agent.hearth = Some((25, 25));
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    for drive in [DriveType::Hunger, DriveType::Thirst] {
        if let Some(it) = agent.drives.get_mut(drive) {
            it.decrease(1.0);
        }
    }
    simulation
}

fn some(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 1.0));
}

/// Salt lying at a place, and known of.
fn salt_known_at(simulation: &mut Simulation, at: (i32, i32)) {
    let where_it_is = Position::new(at.0, at.1);
    simulation.world.resources.push(ResourceNode::new(ResourceType::Salt, where_it_is, 60));
    simulation.population.agents[0]
        .exploration_knowledge
        .known_resources
        .insert(where_it_is, ResourceType::Salt);
}

/// With none going in, a body is short of salt in under a month and has
/// none left in forty days; an infant at the breast never goes short.
#[test]
fn the_salt_runs_out_of_a_body() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    let a_day = crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    for _ in 0..(27 * a_day) {
        agent.turn_the_salt_in_the_body();
    }
    assert!(agent.craves_salt(), "short in under a month: {}", agent.salt_in_the_body);
    for _ in 0..(14 * a_day) {
        agent.turn_the_salt_in_the_body();
    }
    assert_eq!(agent.salt_in_the_body, 0.0);
    let health = agent.state.health;
    for _ in 0..a_day {
        agent.turn_the_salt_in_the_body();
    }
    assert!(agent.state.health < health, "and it weakens");
    // As a want in the diet, and not as a blow that opens a wound (#372)
    assert!(
        agent
            .state
            .what_has_taken_health
            .iter()
            .any(|(what, taken)| what == crate::agents::AgentState::A_POOR_DIET && *taken > 0.0),
        "booked as a poor diet: {:?}",
        agent.state.what_has_taken_health
    );
    assert!(
        !agent.state.what_has_taken_health.iter().any(|(what, taken)| what == crate::agents::AgentState::A_BLOW && *taken > 0.0),
        "and not as a blow"
    );
    assert_eq!(agent.state.an_open_wound, 0.0, "nor a wound");

    agent.state.life_stage = LifeStage::Infant;
    agent.turn_the_salt_in_the_body();
    assert_eq!(agent.salt_in_the_body, 1.0, "at the breast");
}

/// Flesh, fish and milk put salt back, salted food more, wild plants a little
/// and grain least;
/// and a pinch of what is carried is licked when the body craves it.
#[test]
fn salt_is_put_back_by_what_is_eaten_and_licked() {
    use crate::agents::Agent;
    assert!(Agent::salt_in_a_mouthful_of("fish") > Agent::salt_in_a_mouthful_of("meat"));
    assert!(Agent::salt_in_a_mouthful_of("cooked_meat") > 0.0);
    assert!(Agent::salt_in_a_mouthful_of("milk") > 0.0);
    let leaves = Agent::salt_in_a_mouthful_of("greens");
    let grain = Agent::salt_in_a_mouthful_of("grain");
    assert!(Agent::salt_in_a_mouthful_of("meat") > leaves && leaves > grain && grain > 0.0);
    assert!(Agent::salt_in_a_mouthful_of("brined_fish") > Agent::salt_in_a_mouthful_of("fish"));

    let mut simulation = one_person();
    some(&mut simulation, "salt", 2);
    let agent = &mut simulation.population.agents[0];
    agent.salt_in_the_body = 0.5;
    agent.turn_the_salt_in_the_body();
    assert_eq!(agent.how_many_i_have("salt"), 2, "not craving it");
    agent.salt_in_the_body = 0.2;
    agent.turn_the_salt_in_the_body();
    assert_eq!(agent.how_many_i_have("salt"), 1);
    assert!(agent.salt_in_the_body > 0.55);
}

/// Craving salt with salt known within a forage, it is gathered; not
/// craving, nothing changes.
#[test]
fn salt_near_is_gathered_when_craved() {
    let mut simulation = one_person();
    salt_known_at(&mut simulation, (35, 25));
    let wait = Action::Wait;
    assert_eq!(simulation.what_the_salt_asks(0, wait.clone(), false), wait, "not craving");

    simulation.population.agents[0].salt_in_the_body = 0.1;
    assert!(matches!(
        simulation.what_the_salt_asks(0, wait.clone(), false),
        Action::Gather { ref resource_type } if resource_type == "salt"
    ));
    assert_eq!(simulation.what_the_salt_asks(0, wait.clone(), true), wait, "running away comes first");
}

/// Salt known beyond a day's forage is a journey: there, a load gathered,
/// and home, and slept out on the way.
#[test]
fn salt_far_off_is_a_journey() {
    let mut simulation = one_person();
    salt_known_at(&mut simulation, (225, 25));
    simulation.population.agents[0].salt_in_the_body = 0.1;
    // Food enough for the way
    let food = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Meat, simulation.current_turn);
    let mut dried = InventoryItem::new_with_weight("dried_meat".to_string(), 20, 0.5);
    dried.food_data = food;
    simulation.population.agents[0].inventory.add_item(dried);

    let first = simulation.what_the_salt_asks(0, Action::Wait, false);
    let journey = simulation.population.agents[0].journey.clone().expect("set out");
    assert_eq!(journey.to, (225, 25));
    assert!(!journey.homeward);
    assert!(matches!(first, Action::Move { target } if (target.0, target.1) == (225, 25)), "got {first:?}");

    // Kept to, the next turn, whatever else was decided
    let kept = simulation.what_the_salt_asks(0, Action::Wait, false);
    assert!(matches!(kept, Action::Move { target } if (target.0, target.1) == (225, 25)));

    // There, it gathers; loaded, it turns for home
    simulation.population.agents[0].state.position = (224, 25, 0);
    assert!(matches!(
        simulation.what_the_salt_asks(0, Action::Wait, false),
        Action::Gather { ref resource_type } if resource_type == "salt"
    ));
    some(&mut simulation, "salt", Simulation::A_LOAD_OF_SALT);
    let home = simulation.what_the_salt_asks(0, Action::Wait, false);
    assert!(matches!(home, Action::Move { target } if (target.0, target.1) == (25, 25)), "got {home:?}");
    assert!(simulation.population.agents[0].journey.as_ref().unwrap().homeward);

    simulation.population.agents[0].state.position = (26, 25, 0);
    simulation.what_the_salt_asks(0, Action::Wait, false);
    assert!(simulation.population.agents[0].journey.is_none(), "home again");
}

/// Where the salt is stays in mind among a head full of other places.
#[test]
fn where_the_salt_is_is_not_forgotten() {
    let mut simulation = one_person();
    salt_known_at(&mut simulation, (225, 25));
    let agent = &mut simulation.population.agents[0];
    for i in 0..400 {
        agent
            .exploration_knowledge
            .known_resources
            .insert(Position::new(30 + i % 20, 30 + i / 20), ResourceType::Food);
    }
    agent.forget_what_does_not_matter(100);
    assert!(agent.exploration_knowledge.known_resources.contains_key(&Position::new(225, 25)));
}
