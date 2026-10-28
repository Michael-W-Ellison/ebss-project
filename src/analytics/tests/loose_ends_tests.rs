// src/analytics/tests/loose_ends_tests.rs
//! Defects left open in ISSUES_FOUND, each fixed and held here: a dig offered
//! without a tool to dig with (#126), food eaten where it grew going
//! uncounted (#113), honey nobody could eat (#120), and burnt food carried
//! for ever (#16); and from the third batch, clocks counted in the wrong
//! units (#73), a threshold nobody could reach (#173), tiredness read as
//! hunger (#70), a spear that never counted as a weapon (#100), and beliefs
//! that all ended on a floor (#33).

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

/// Wasting begins after a day of going short, in the half-hour turns the
/// body is reckoned in, not after a month of them.
#[test]
fn wasting_begins_after_a_day_short() {
    use crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    use crate::world::nutrition::NutritionalState;
    let mut body = NutritionalState { protein_stores: 10.0, ..NutritionalState::full() };
    for _ in 0..PLANNING_PERIODS_PER_DAY {
        body.turn_metabolism(0.5);
    }
    assert!(!body.has_protein_deficiency(), "not inside the day");
    for _ in 0..PLANNING_PERIODS_PER_DAY {
        body.turn_metabolism(0.5);
    }
    assert!(body.has_protein_deficiency(), "but within two");
}

/// A pack can hold enough materials to quiet Industry.
#[test]
fn enough_materials_fits_in_a_pack() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    // What a grown pair of hands holds, with nothing to carry it in.
    agent.inventory.max_weight = crate::agents::Agent::WHAT_TWO_HANDS_HOLD;
    let wanted = crate::core::DriveType::ENOUGH_MATERIALS as u32;
    assert!(agent.inventory.add_item(InventoryItem::new_with_weight("wood".to_string(), wanted, 1.0)));
    assert_eq!(agent.what_the_situation_asks().materials_put_by, wanted);
}

/// Worn out is not starving.
#[test]
fn tired_is_not_starving() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.state.energy = 5.0;
    assert!(!agent.state.is_starving());
}

/// A spear carried in the pack is a weapon to have with you; it is out in
/// the hand, not in the pack, that makes a man feel armed; and it adds
/// nothing to his own strength, because it is counted where it does its work.
#[test]
fn a_spear_in_the_pack_is_a_weapon() {
    let mut simulation = one_person();
    let bare = simulation.population.agents[0].own_strength();
    assert!(!simulation.population.agents[0].have_i_a_weapon());
    let spear = crate::environment::making::what_helps_with(crate::agents::SkillType::Hunting)
        .find(|tool| tool.called == "spear")
        .expect("a spear hunts");
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_durability(spear.called.to_string(), 1, 25.0, crate::agents::Quality::Common));
    let agent = &simulation.population.agents[0];
    assert!(agent.have_i_a_weapon());
    assert!(!agent.what_the_situation_asks().armed, "in the pack");
    assert_eq!(agent.own_strength(), bare, "counted in the throw and the fight, not twice");

    let agent = &mut simulation.population.agents[0];
    assert!(agent.take_in_hand("spear"));
    assert!(agent.what_the_situation_asks().armed, "and in the hand");
}

/// A belief settles somewhere that says how often the thing works, rather
/// than at the floor for everything that works less than five times in
/// eight.
#[test]
fn a_belief_follows_how_often_it_works() {
    use crate::agents::practices::Lessons;
    let settle = |every: u32| {
        let mut lessons = Lessons::new();
        for turn in 0..400u32 {
            lessons.record_particular("try", turn % every == 0);
        }
        lessons.how_likely_to_try_this("try")
    };
    let half = settle(2);
    let tenth = settle(10);
    let mostly = {
        let mut lessons = Lessons::new();
        for turn in 0..400u32 {
            lessons.record_particular("try", turn % 5 != 0);
        }
        lessons.how_likely_to_try_this("try")
    };
    assert!(half > tenth, "half the time {half} against a tenth {tenth}");
    assert!(half > Lessons::NEVER_QUITE_GIVES_UP, "half the time is off the floor: {half}");
    assert!(mostly > half && mostly < Lessons::NEVER_QUITE_CERTAIN, "four in five {mostly}");
}
