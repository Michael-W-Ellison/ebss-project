// src/analytics/tests/food_keeping_tests.rs
//! Food keeping, the second of the unlinked survival machinery (#348):
//! brining, caches, racks, stone boiling and pit ovens. Smoking and drying in
//! the wet are in `keeping_tests`.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::catalogue;
use crate::environment::{Action, HeatSourceType, WeatherType};
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, TerrainType, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32) -> InventoryItem {
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 0.5);
    meal.food_data = FoodDatabase::new().create_food_data(&of, 0);
    meal
}

/// One grown person at (25, 25) with nothing on them, on a clear day, on
/// ground of the kind asked for.
fn one_person_on(ground: TerrainType) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.pits.clear();
    world.climate.weather.weather_type = WeatherType::Clear;
    for x in 22..29 {
        for y in 22..29 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = ground;
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
    agent.inventory.get_all_items_mut().clear();
    simulation
}

fn a_fire_here(simulation: &mut Simulation) {
    let fire = simulation
        .world
        .build_heat_source(HeatSourceType::Campfire, (25, 25, 0), None)
        .expect("a hearth goes here");
    let _ = simulation.world.add_fuel_to_heat_source(&fire, "wood".to_string(), 20.0);
    simulation.world.light_heat_source(&fire).expect("and it lights");
}

fn a_bowl_of_water() -> InventoryItem {
    let mut bowl = InventoryItem::new_container("bowl".to_string(), 1, 4.0);
    bowl.fill(4.0);
    bowl
}

/// What the catalogue says about the six: smoking, racks and pit ovens are
/// found out; a cache is born knowledge; brine waits on dry salting, hanging
/// food on a rack on putting one up, and stone boiling on a basket.
#[test]
fn the_catalogue_knows_how_food_keeping_comes() {
    for found_out in ["do:Smoke", "do:PutUpRack", "do:PitOven"] {
        assert!(!catalogue::is_born_knowledge(found_out), "{found_out}");
        assert!(catalogue::what_comes_before(found_out).is_empty(), "{found_out} is found out, not reached");
    }
    assert!(catalogue::is_born_knowledge("do:Cache"));
    assert_eq!(catalogue::what_comes_before("do:Brine"), ["do:Salt"]);
    assert_eq!(catalogue::what_comes_before("do:HangUp"), ["do:PutUpRack"]);
    assert_eq!(catalogue::what_comes_before("do:StoneBoil"), ["make:basket"]);
}

/// Brine keeps better than dry salt, for the water and a vessel to hold it.
#[test]
fn brine_keeps_better_than_dry_salt() {
    use crate::environment::tags::{how_fast_this_goes_off, Tag};
    let salted = how_fast_this_goes_off("meatportions", &[Tag::Salted]);
    let brined = how_fast_this_goes_off("brined_meatportions", &[Tag::Soured]);
    assert!(brined < salted, "brined {brined} against salted {salted}");

    let mut simulation = one_person_on(TerrainType::Plains);
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("do:Salt");
    agent.inventory.add_item(InventoryItem::new_with_weight("salt".to_string(), 1, 0.2));
    agent.inventory.add_item(a_meal(ItemType::Meat, "meatportions", 4));
    assert!(agent.knows_how_to_do("Brine"), "brine comes with knowing salt");

    let refused = simulation.execute_action(&Action::Brine { what: "meatportions".to_string() }, 0);
    assert!(!refused.success, "nothing with water in it to put it down in");

    simulation.population.agents[0].inventory.add_item(a_bowl_of_water());
    let brined = simulation.execute_action(&Action::Brine { what: "meatportions".to_string() }, 0);
    assert!(brined.success, "{:?}", brined.message);
    let agent = &simulation.population.agents[0];
    let how = agent
        .inventory
        .get_item("brined_meatportions")
        .and_then(|item| item.food_data.as_ref())
        .map(|food| food.preparation);
    assert_eq!(how, Some(PreparationState::Pickled));
    assert_eq!(agent.how_many_i_have("salt"), 0, "the salt went into it");
    assert!(agent.how_much_water_i_carry() < 4.0, "and the water");
}

/// A cache goes where a pit will not: stones over a hollow on the forest
/// floor, covered from the first, and the shade of the trees keeps it cool.
#[test]
fn a_cache_goes_where_a_pit_will_not() {
    let mut simulation = one_person_on(TerrainType::Forest);
    let here = Position::new(25, 25);
    assert!(!simulation.is_ground_a_pit_will_go_in(here), "the forest floor will not take a hole");

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), Simulation::STONES_TO_A_CAIRN, 1.0));
    assert!(simulation.a_cairn_would_go_here(&simulation.population.agents[0], here));
    let cached = simulation.execute_action(&Action::Cache, 0);
    assert!(cached.success, "{:?}", cached.message);
    let pit = simulation.world.pit_at(here).expect("a store here now");
    assert!(pit.covered, "covered from the first");
    assert_eq!(simulation.population.agents[0].how_many_i_have("stone"), 0);

    // And the same lining keeps better in the shade than in the open
    let open = {
        let mut world = World::new(WorldConfig::default());
        world.pits.clear();
        let at = Position::new(25, 25);
        if let Some(tile) = world.grid.get_tile_mut(&at) {
            tile.terrain.terrain_type = TerrainType::Plains;
        }
        world.pits.push(crate::world::Pit {
            where_it_is: at,
            holds: vec![a_meal(ItemType::Meat, "meatportions", 10)],
            covered: true,
            dug: 0,
            belongs: Default::default(),
        });
        world
    };
    let mut shaded = open.clone();
    if let Some(tile) = shaded.grid.get_tile_mut(&Position::new(25, 25)) {
        tile.terrain.terrain_type = TerrainType::Forest;
    }
    let mut worlds = [open, shaded];
    for world in worlds.iter_mut() {
        for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY * 4 {
            world.take_a_turn();
        }
    }
    let fresh = |world: &World| {
        world.pits[0].holds.first().and_then(|item| item.food_data.as_ref()).map_or(0.0, |food| food.freshness)
    };
    assert!(
        fresh(&worlds[1]) > fresh(&worlds[0]),
        "four days in the shade of trees against four in the open: {} against {}",
        fresh(&worlds[1]),
        fresh(&worlds[0])
    );
}

/// A rack: put up from poles and cord, food hung on it sheds the rain and
/// stays there until it is taken down; once dry it is taken down.
#[test]
fn food_on_a_rack_dries_and_is_taken_down() {
    let mut simulation = one_person_on(TerrainType::Plains);
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("do:PutUpRack");
    agent.found_out_how_to(Simulation::THAT_LAYING_IT_OUT_KEEPS_IT);
    agent.inventory.add_item(InventoryItem::new_with_weight("wood".to_string(), 3, 1.0));
    agent.inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 2, 0.2));
    assert!(agent.knows_how_to_do("HangUp"), "hanging food up comes with the rack");

    let up = simulation.execute_action(&Action::PutUpRack, 0);
    assert!(up.success, "{:?}", up.message);
    assert_eq!(simulation.world.racks, vec![Position::new(25, 25)]);

    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 6));
    let hung = simulation.execute_action(&Action::HangUp { what: "fishstrips".to_string() }, 0);
    assert!(hung.success, "{:?}", hung.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("fishstrips"), 0);

    // A fortnight of clear weather, and it is dry and still there
    for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY * 14 {
        simulation.world.climate.weather.weather_type = WeatherType::Clear;
        simulation.world.take_a_turn();
    }
    let on_the_rack = simulation.world.what_is_lying_at(&Position::new(25, 25));
    let strips = on_the_rack.first().expect("still on the rack");
    assert_eq!(
        strips.item.food_data.as_ref().map(|food| food.preparation),
        Some(PreparationState::Dried),
        "dried on the rack"
    );
    let at = simulation.population.agents[0].state.position;
    assert_eq!(
        simulation.what_has_dried_on_a_rack(at),
        Some(Action::PickUp { what: "fishstrips".to_string() }),
        "and what has dried is taken down"
    );
}

/// Stone boiling: a vessel of water, stones out of the hearth, more at once
/// than over the flames and nothing burnt; and the water goes.
#[test]
fn stones_in_a_vessel_of_water_cook_without_burning() {
    let mut simulation = one_person_on(TerrainType::Plains);
    a_fire_here(&mut simulation);
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("basket");
    agent.inventory.add_item(a_bowl_of_water());
    agent.inventory.add_item(a_meal(ItemType::Meat, "meatportions", 10));
    assert!(agent.knows_how_to_do("StoneBoil"), "stone boiling comes with the basket");

    let at = simulation.population.agents[0].state.position;
    assert_eq!(
        simulation.a_better_way_to_cook(&simulation.population.agents[0], at),
        Some(Action::StoneBoil),
        "more than a fire takes at once, and the water for it"
    );
    let boiled = simulation.execute_action(&Action::StoneBoil, 0);
    assert!(boiled.success, "{:?}", boiled.message);
    let agent = &simulation.population.agents[0];
    assert_eq!(agent.how_many_i_have("cooked_meatportions"), Simulation::WHAT_A_BOILING_COOKS);
    assert!(agent.how_much_water_i_carry() < 4.0, "and the water went");
}

/// Boiling the sea for salt wants something to boil it in (#348).
#[test]
fn boiling_wants_a_vessel() {
    let simulation = one_person_on(TerrainType::Plains);
    let agent = &simulation.population.agents[0];
    assert_eq!(Simulation::something_to_boil_it_in(agent), None, "nothing to boil it in");

    let mut simulation = one_person_on(TerrainType::Plains);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stoneware".to_string(), 1, 1.0));
    assert_eq!(Simulation::something_to_boil_it_in(&simulation.population.agents[0]), Some(false), "a fired pot");

    let mut simulation = one_person_on(TerrainType::Plains);
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("basket");
    agent.inventory.add_item(InventoryItem::new_with_weight("basket".to_string(), 1, 0.5));
    assert_eq!(
        Simulation::something_to_boil_it_in(&simulation.population.agents[0]),
        Some(true),
        "or a basket, and the hearth's stones"
    );
}

/// A pit oven: found out after enough suppers over a fire, and then most of
/// a pack cooked at once and nothing burnt.
#[test]
fn a_pit_oven_cooks_most_of_a_pack_at_once() {
    let mut simulation = one_person_on(TerrainType::Plains);
    a_fire_here(&mut simulation);
    let agent = &mut simulation.population.agents[0];
    for _ in 0..crate::agents::Agent::BATCHES_BEFORE_A_PIT_OVEN - 1 {
        agent.skills.made_one(crate::agents::Agent::A_BATCH_OFF_THE_FIRE);
    }
    agent.inventory.add_item(a_meal(ItemType::Meat, "meatportions", 30));
    assert!(!agent.knows_how_to_do("PitOven"));
    crate::core::dice::seed(7);
    let cooked = simulation.execute_action(&Action::Cook { food_type: "generic".to_string() }, 0);
    let _ = cooked;
    assert!(
        simulation.population.agents[0].knows_how_to_do("PitOven"),
        "one supper more, whether it burnt or not, and it occurs to somebody"
    );

    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(InventoryItem::new_with_weight("wood".to_string(), 2, 1.0));
    let before = agent.how_many_i_have("meatportions");
    let at = agent.state.position;
    assert_eq!(
        simulation.a_better_way_to_cook(&simulation.population.agents[0], at),
        Some(Action::PitOven)
    );
    let roasted = simulation.execute_action(&Action::PitOven, 0);
    assert!(roasted.success, "{:?}", roasted.message);
    let agent = &simulation.population.agents[0];
    assert_eq!(
        before - agent.how_many_i_have("meatportions"),
        Simulation::WHAT_A_PIT_OVEN_COOKS.min(before)
    );
    assert_eq!(agent.how_many_i_have("wood"), 0, "the wood is burnt");
}

/// Food on a rack is food put by: the drive that sent it there falls for it
/// (#370). It was counted by nobody, so a rack of drying fish left
/// Preparedness reading as though the catch had never come home.
#[test]
fn food_on_a_rack_eases_preparedness() {
    use crate::core::DriveType;

    let preparedness_with = |on_the_rack: u32| {
        let mut simulation = one_person_on(TerrainType::Plains);
        simulation.world.racks.push(Position::new(25, 25));
        if on_the_rack > 0 {
            simulation.world.dropped.push(crate::world::Dropped {
                item: a_meal(ItemType::Fish, "fishstrips", on_the_rack),
                where_it_is: Position::new(25, 25),
                since: 0,
                dried_in_the_sun: 0,
            });
        }
        simulation.reckon_what_is_put_by();
        simulation.population.agents[0]
            .drives
            .get(DriveType::Preparedness)
            .map(|drive| drive.value)
            .unwrap_or(0.0)
    };

    let bare = preparedness_with(0);
    let stocked = preparedness_with(200);
    assert!(stocked < bare, "two hundred strips on the rack and no easier: {bare:.2} against {stocked:.2}");
}
