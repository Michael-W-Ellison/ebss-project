// src/analytics/tests/loose_ends_tests.rs
//! Defects left open in ISSUES_FOUND, each fixed and held here: a dig offered
//! without a tool to dig with (#126), food eaten where it grew going
//! uncounted (#113), honey nobody could eat (#120), and burnt food carried
//! for ever (#16).

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::nutrition::PreparationState;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// One grown person at (25, 25) on open plains, nothing on them.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    world.pits.clear();
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
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(25);
    agent.state.position = (25, 25, 0);
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    simulation
}

/// Somebody with nothing to dig with is not offered a dig the verb matrix
/// would refuse: they are sent towards a digging tool, or offered nothing.
#[test]
fn a_dig_is_offered_only_with_something_to_dig_with() {
    let mut simulation = one_person();
    let agent = simulation.population.agents[0].clone();
    let offered = simulation.a_dig_or_what_it_wants_first(&agent);
    assert!(!matches!(offered, Some(Action::Excavate)), "nothing to dig with, got {offered:?}");
    if let Some(action) = &offered {
        assert!(
            simulation.what_this_one_is_short_of(action, &agent).is_none(),
            "a step that would itself be refused: {action:?}"
        );
    }

    let stick = crate::environment::making::what_helps_with(crate::agents::SkillType::Mining)
        .next()
        .expect("something digs");
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_durability(stick.called.to_string(), 1, 25.0, crate::agents::Quality::Common));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_dig_or_what_it_wants_first(&agent), Some(Action::Excavate));
    assert!(simulation.what_this_one_is_short_of(&Action::Excavate, &agent).is_none());
}

/// Food eaten where it grew is counted as eaten, and carries its salt into
/// the body, as a meal out of the pack does.
#[test]
fn food_eaten_where_it_grew_is_counted() {
    let mut simulation = one_person();
    simulation.world.resources.push(ResourceNode::new(ResourceType::Greens, Position::new(25, 25), 40));
    let agent = &mut simulation.population.agents[0];
    agent.salt_in_the_body = 0.5;
    let before = agent.food_i_ate;
    let ate = simulation.execute_action(&Action::Eat { food_type: "generic".to_string() }, 0);
    assert!(ate.success, "{:?}", ate.message);
    let agent = &simulation.population.agents[0];
    assert!(agent.food_i_ate > before, "counted");
    assert!(agent.salt_in_the_body > 0.5, "and its salt went in");
}

/// Wild honey is food, gathered and eaten in its season.
#[test]
fn wild_honey_is_food() {
    assert!(ResourceType::Honey.is_it_food());
    assert_eq!(
        Simulation::edible_item_for(ResourceType::Honey),
        Some(crate::world::ItemType::Honey)
    );
}

/// Food burnt past eating is thrown away, not carried for ever.
#[test]
fn burnt_food_is_thrown_away() {
    let mut simulation = one_person();
    let mut burnt = InventoryItem::new_with_weight("burnt_fish".to_string(), 3, 1.0);
    let mut food = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Fish, 0)
        .expect("fish is food");
    food.preparation = PreparationState::Ruined;
    burnt.food_data = Some(food);
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(burnt);
    assert_eq!(agent.how_many_i_have("burnt_fish"), 3);
    agent.turn_food_spoilage(30);
    assert_eq!(agent.how_many_i_have("burnt_fish"), 0);
    assert!(agent.state.waste_carried > 0.0, "and it is waste, which goes back to the ground");
}
