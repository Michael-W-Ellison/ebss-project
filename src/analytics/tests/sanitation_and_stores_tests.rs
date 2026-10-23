// src/analytics/tests/sanitation_and_stores_tests.rs
//! Waste, sanitation and storage (#357): a midden found out after filth has
//! laid somebody up, refuse taken to it and leavings into a latrine; water
//! drawn by fouled ground that sickens; a fouled camp moved off its ground;
//! vermin in stores, theft from them, scavengers drawn to what is left
//! lying; a storehouse that is a place; and buildings that make nothing from
//! nothing.

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::belonging::Belongs;
use crate::world::{Building, BuildingType, Pit, Position, TerrainType, World, WorldConfig};

/// One grown person at (100, 100) on dry plains three hundred cells across,
/// living where they stand.
fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig { size: (300, 300), ..WorldConfig::default() });
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
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    for agent in simulation.population.agents.iter_mut() {
        agent.state.now_this_many_years_old(25);
        agent.state.position = (100, 100, 0);
        agent.hearth = Some((100, 100));
        agent.inventory.get_all_items_mut().clear();
        agent.inventory.max_weight = 500.0;
    }
    simulation
}

fn food(simulation: &Simulation, kind: crate::world::ItemType, called: &str, how_many: u32) -> InventoryItem {
    let mut item = InventoryItem::new_with_weight(called.to_string(), how_many, 1.0);
    item.food_data = simulation.food_database.create_food_data(&kind, simulation.current_turn);
    item
}

/// Twice laid up by filth and somebody keeps the refuse in one place; with
/// farming besides, a latrine.
#[test]
fn filth_teaches_a_midden_then_a_latrine() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.taken_ill_with(Agent::OFF_FOUL_GROUND, 0.3, 0);
    agent.what_filth_teaches();
    assert!(!agent.knows_how_to_do("StartMidden"), "once is bad luck");
    agent.state.ailing = None;
    agent.taken_ill_with(Agent::OFF_FOUL_WATER, 0.3, 10);
    agent.what_filth_teaches();
    assert!(agent.knows_how_to_do("StartMidden"));
    assert!(!agent.knows_how_to_do("DigLatrine"), "a forager has no use for one");

    for _ in 0..20 {
        agent.practices.record_outcome(crate::agents::practices::Practice::Farming, true);
    }
    agent.what_filth_teaches();
    assert!(agent.knows_how_to_do("DigLatrine"));
}

/// A midden goes a way from the hearth, away from water; refuse is taken
/// to it; and a latrine by it takes the leavings and fouls nothing.
#[test]
fn refuse_goes_to_the_midden_and_the_latrine() {
    let mut simulation = one_person();
    // Water to the east
    for y in 90..110 {
        if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(112, y)) {
            tile.terrain.terrain_type = TerrainType::Water;
        }
    }
    simulation.population.agents[0].found_out_how_to("do:StartMidden");
    let started = simulation.execute_action(&Action::StartMidden, 0);
    assert!(started.success, "{:?}", started.message);
    let midden = simulation.world.middens[0].at;
    assert!(midden.x < 100, "away from the water: {midden:?}");
    assert!(!simulation.execute_action(&Action::StartMidden, 0).success, "one is enough");

    // What the keeper passes goes to the midden; the other leaves it where
    // they stand
    for agent in simulation.population.agents.iter_mut() {
        agent.state.waste_carried = 1.0;
    }
    simulation.return_what_the_living_and_the_dead_leave();
    let fouling = |simulation: &Simulation, at: Position| simulation.world.grid.get_tile(&at).unwrap().soil.fouling;
    assert!(fouling(&simulation, midden) > 0.0);
    let camp = Position::new(100, 100);
    let camp_fouled = fouling(&simulation, camp);

    simulation.population.agents[0].found_out_how_to("do:DigLatrine");
    let dug = simulation.execute_action(&Action::DigLatrine, 0);
    assert!(dug.success, "{:?}", dug.message);
    simulation.population.agents[0].state.waste_carried = 1.0;
    let at_the_midden = fouling(&simulation, midden);
    simulation.return_what_the_living_and_the_dead_leave();
    assert_eq!(fouling(&simulation, midden), at_the_midden, "into the pit, not on top");
    assert_eq!(fouling(&simulation, camp), camp_fouled);
    assert!(simulation.world.latrines[0].holds > 0.0);
}

/// Water drawn beside fouled ground makes people ill.
#[test]
fn water_by_fouled_ground_sickens() {
    let mut simulation = one_person();
    let spring = Position::new(101, 100);
    simulation
        .world
        .resources
        .push(crate::world::ResourceNode::new(crate::world::ResourceType::Water, spring, 1000));
    assert_eq!(simulation.world.how_foul_about((101, 100), 3), 0.0);
    simulation.world.grid.somebody_voided_on(&Position::new(102, 100), 10.0);
    assert!(simulation.world.how_foul_about((101, 100), 3) > 0.9);

    let mut ill = 0;
    for _ in 0..1000 {
        let agent = &mut simulation.population.agents[0];
        agent.state.ailing = None;
        simulation.execute_action(&Action::Gather { resource_type: "water".to_string() }, 0);
        if simulation.population.agents[0].is_ailing() {
            ill += 1;
        }
    }
    assert!((3..30).contains(&ill), "one drink in a hundred or so: {ill} of 1000");
}

/// Somebody laid up by filth moves the camp off fouled ground to clean
/// ground with food on it.
#[test]
fn a_fouled_camp_is_moved_off() {
    let mut simulation = one_person();
    let elsewhere = Position::new(160, 100);
    simulation
        .world
        .resources
        .push(crate::world::ResourceNode::new(crate::world::ResourceType::Food, elsewhere, 200));
    simulation.population.agents[0]
        .exploration_knowledge
        .known_resources
        .insert(elsewhere, crate::world::ResourceType::Food);
    for dx in -3..=3 {
        simulation.world.grid.somebody_voided_on(&Position::new(100 + dx, 101), 2.0);
    }
    for dx in -3..=3 {
        simulation.world.grid.somebody_voided_on(&Position::new(100 + dx, 99), 2.0);
    }
    let agent = simulation.population.agents[0].clone();
    assert!(!simulation.this_camp_is_fouled(&agent), "nobody has been ill of it");
    simulation.population.agents[0].taken_ill_with(Agent::OFF_FOUL_GROUND, 0.3, 0);
    let agent = simulation.population.agents[0].clone();
    assert!(simulation.this_camp_is_fouled(&agent));
    let moved = simulation.moving_on(&agent, (100, 100, 0));
    assert!(matches!(moved, Some(Action::Move { target }) if (target.0, target.1) == (160, 100)), "got {moved:?}");
}

/// Vermin take from a store by how it is shut, and more by a midden.
#[test]
fn vermin_take_by_how_a_store_is_shut() {
    let mut simulation = one_person();
    let mut store = |at: (i32, i32), covered: bool, lining: Option<&str>| {
        let mut holds = vec![food(&simulation, crate::world::ItemType::Grain, "grain", 1000)];
        if let Some(vessel) = lining {
            holds.push(InventoryItem::new_with_weight(vessel.to_string(), 1, 1.0));
        }
        simulation.world.pits.push(Pit {
            where_it_is: Position::new(at.0, at.1),
            holds,
            covered,
            dug: 0,
            belongs: Belongs::ToNobody,
        });
    };
    store((10, 10), false, None);
    store((40, 10), true, None);
    store((70, 10), true, Some("basket"));
    store((100, 10), true, Some("stoneware"));
    store((130, 10), true, None);
    simulation.world.middens.push(crate::world::Midden { at: Position::new(133, 10), started: 0 });

    for _ in 0..10 {
        simulation.world.what_the_vermin_take();
    }
    let left = |simulation: &Simulation, x: i32| {
        simulation.world.pit_at(Position::new(x, 10)).map(|pit| pit.how_much_is_in_it()).unwrap_or(0)
    };
    let (open, under_earth, lined, potted, by_midden) =
        (left(&simulation, 10), left(&simulation, 40), left(&simulation, 70), left(&simulation, 100), left(&simulation, 130));
    assert!(open < under_earth && under_earth < lined && lined < potted, "{open} {under_earth} {lined} {potted}");
    assert!(by_midden < under_earth, "a midden draws them: {by_midden} against {under_earth}");
    assert!(potted >= 997);
}

/// A spare fired pot goes in as a store's lining, the best keeper there is.
#[test]
fn a_spare_pot_lines_a_store() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(InventoryItem::new_container("stoneware".to_string(), 2, 9.0));
    agent.inventory.add_item(InventoryItem::new_with_weight("basket".to_string(), 2, 1.0));
    let fish = food(&simulation, crate::world::ItemType::Fish, "dried_fish", 30);
    simulation.population.agents[0].inventory.add_item(fish);
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(100, 100),
        holds: Vec::new(),
        covered: false,
        dug: 0,
        belongs: Belongs::To(simulation.population.agents[0].id),
    });
    let covered = simulation.execute_action(&Action::Cover { what: "dried_fish".to_string() }, 0);
    assert!(covered.success, "{:?}", covered.message);
    let pit = simulation.world.pit_at(Position::new(100, 100)).unwrap();
    assert!(pit.holds.iter().any(|item| item.item_id == "stoneware"), "{:?}", pit.holds);
    assert_eq!(pit.what_vermin_get_at(), Pit::VERMIN_PAST_A_POT);
}

/// Taking from the store of somebody living at another camp is theft,
/// found out when its owner next opens it; a campmate's is not.
#[test]
fn taking_from_anothers_store_is_found_out() {
    let mut simulation = one_person();
    let owner = simulation.population.agents[0].id;
    let thief = simulation.population.agents[1].id;
    let stores = vec![food(&simulation, crate::world::ItemType::Grain, "grain", 100)];
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(100, 100),
        holds: stores,
        covered: true,
        dug: 0,
        belongs: Belongs::To(owner),
    });
    let take = Action::PickUp { what: "grain".to_string() };
    assert!(simulation.execute_action(&take, 1).success);
    assert!(simulation.world.stores_robbed.is_empty(), "a campmate's store is shared");

    simulation.population.agents[1].hearth = Some((200, 200));
    assert!(simulation.execute_action(&take, 1).success);
    assert_eq!(simulation.world.stores_robbed.get(&Position::new(100, 100)).map(|(who, _)| *who), Some(thief));

    let before = simulation.population.agents[0]
        .relationships
        .get_relationship(&thief)
        .map(|bond| bond.bond_strength)
        .unwrap_or(0.0);
    assert!(simulation.execute_action(&take, 0).success);
    assert!(simulation.world.stores_robbed.is_empty(), "found out");
    let after = simulation.population.agents[0].relationships.get_relationship(&thief).unwrap().bond_strength;
    assert!(after < before, "and held against them: {before} -> {after}");
}

/// A hungry wolf comes to an open store and takes from it; a covered one
/// it leaves.
#[test]
fn scavengers_come_to_an_open_store() {
    let mut simulation = one_person();
    let grain = vec![food(&simulation, crate::world::ItemType::Meat, "meat", 50)];
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(100, 100),
        holds: grain.clone(),
        covered: false,
        dug: 0,
        belongs: Belongs::ToNobody,
    });
    let wolf = simulation.world.spawn_animal("wolf".to_string(), (130, 100)).expect("a wolf");
    {
        let animal = simulation.world.animals.get_mut(&wolf).unwrap();
        animal.hunger = animal.max_hunger * 0.9;
    }
    for _ in 0..20 {
        simulation.world.what_draws_the_scavengers();
    }
    let left = simulation.world.pit_at(Position::new(100, 100)).unwrap().how_much_is_in_it();
    assert!(left < 50, "the wolf ate: {left} left");
    let at = simulation.world.animals.get(&wolf).unwrap().position;
    assert!((at.0 - 100).abs() <= 1, "and came to it: {at:?}");

    // Covered, nothing comes
    let mut simulation = one_person();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(100, 100),
        holds: grain,
        covered: true,
        dug: 0,
        belongs: Belongs::ToNobody,
    });
    let wolf = simulation.world.spawn_animal("wolf".to_string(), (130, 100)).expect("a wolf");
    simulation.world.animals.get_mut(&wolf).unwrap().hunger = 1.0e6;
    simulation.world.what_draws_the_scavengers();
    assert_eq!(simulation.world.animals.get(&wolf).unwrap().position, (130, 100));
}

/// The storehouse is a place: things go in and come out standing at one,
/// with food's clock still on it, and nowhere else.
#[test]
fn the_storehouse_is_a_place() {
    let mut simulation = one_person();
    let fish = food(&simulation, crate::world::ItemType::Fish, "fish", 10);
    simulation.population.agents[0].inventory.add_item(fish);
    let store = Action::Store { item_type: "fish".to_string(), amount: 10 };
    assert!(!simulation.execute_action(&store, 0).success, "no storehouse here");

    simulation.world.add_building(Building::new(BuildingType::Storehouse, Position::new(101, 100)));
    let put = simulation.execute_action(&store, 0);
    assert!(put.success, "{:?}", put.message);
    let held = simulation.world.pit_at(Position::new(101, 100)).unwrap();
    assert!(held.holds.iter().any(|item| item.item_id == "fish" && item.food_data.is_some()), "with its clock");

    simulation.population.agents[0].state.position = (150, 150, 0);
    let fetch = Action::Retrieve { item_type: "fish".to_string(), amount: 5 };
    assert!(!simulation.execute_action(&fetch, 0).success, "not from over here");
    simulation.population.agents[0].state.position = (100, 100, 0);
    assert!(simulation.execute_action(&fetch, 0).success);
    assert_eq!(simulation.population.agents[0].how_many_i_have("fish"), 5);
}

/// No building turns out anything from nothing.
#[test]
fn nothing_is_made_from_nothing() {
    for kind in [BuildingType::Farm, BuildingType::AnimalPen, BuildingType::Tannery, BuildingType::Forge] {
        assert_eq!(kind.production_interval(), 0, "{kind:?}");
    }
}
