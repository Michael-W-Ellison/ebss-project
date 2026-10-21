// src/analytics/tests/shelter_on_the_move_tests.rs
//! Shelter on the move, the third of the unlinked survival machinery (#349):
//! windbreaks, lean-tos, brush huts and bedding. Cover is graded - a
//! windbreak takes off the wind and is not a roof, and a tent holds more
//! warmth than a hut, a hut more than a lean-to - and brush falls down.

use crate::agents::{AgentConfig, Climate, InventoryItem, Population};
use crate::analytics::wanting::strategy::{Reach, Strategy};
use crate::analytics::Simulation;
use crate::environment::catalogue;
use crate::environment::{Action, WeatherType};
use crate::world::{Bedding, Building, BuildingType, Cover, Position, TerrainType, World, WorldConfig};

/// One grown person at (25, 25) with nothing on them, on a clear day, on
/// open plains with no roof or trees anywhere.
fn one_person_in_the_open() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.climate.weather.weather_type = WeatherType::Clear;
    world.climate.weather.base_wind_speed = 0.0;
    for x in 0..200 {
        for y in 0..200 {
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
    agent.inventory.get_all_items_mut().clear();
    simulation
}

fn poles_and_cord(simulation: &mut Simulation, poles: u32, cord: u32) {
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(InventoryItem::new_with_weight("wood".to_string(), poles, 1.0));
    if cord > 0 {
        agent.inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), cord, 0.1));
    }
}

fn a_finished(what: BuildingType, at: Position) -> Building {
    Building::new(what, at)
}

/// The catalogue links the five stage-0 listings, all born knowledge.
#[test]
fn the_catalogue_knows_brush_and_bedding() {
    for link in ["build:Windbreak", "build:LeanTo", "build:BrushHut", "do:LayBedding"] {
        assert!(catalogue::is_born_knowledge(link), "{link} is stage 0");
        assert!(catalogue::what_comes_before(link).is_empty(), "{link} waits on nothing");
    }
}

/// A tent is warmer than a brush hut, a hut than a lean-to, and a lean-to
/// than a windbreak; a windbreak is the open air with the wind taken off.
#[test]
fn cover_is_graded() {
    let cold_and_windy = Climate { temperature: -5.0, humidity: 0.5, wind_speed: 30.0 };
    let under = |what: BuildingType| cold_and_windy.effective_temperature_under(Some(what.cover()));

    let tent = under(BuildingType::SkinTent);
    let hut = under(BuildingType::BrushHut);
    let lean_to = under(BuildingType::LeanTo);
    let windbreak = under(BuildingType::Windbreak);
    let open = cold_and_windy.effective_temperature_under(None);
    assert!(tent > hut && hut > lean_to && lean_to > windbreak && windbreak > open,
        "tent {tent}, hut {hut}, lean-to {lean_to}, windbreak {windbreak}, open {open}");

    let still = Climate { wind_speed: 0.0, ..cold_and_windy.clone() };
    assert_eq!(windbreak, still.effective_temperature(), "a windbreak takes off the wind and nothing else");
    assert_eq!(tent, cold_and_windy.sheltered_effective_temperature(), "a tent is shelter as it always was");
    assert_eq!(BuildingType::SkinTent.cover(), Cover::FULL);
}

/// A windbreak is cover and not a roof: nobody is out of the weather behind
/// one, nor sheltered, and a half-built lean-to is no roof either.
#[test]
fn a_windbreak_is_not_a_roof() {
    let mut simulation = one_person_in_the_open();
    let here = Position::new(25, 25);
    simulation.world.add_building(a_finished(BuildingType::Windbreak, here));

    let cover = simulation.world.cover_at(&here).expect("a windbreak is cover");
    assert!(cover.keeps_off_wind && !cover.is_a_roof());
    assert!(!simulation.world.is_out_of_the_weather_at(&here));
    assert!(!simulation.agent_has_shelter(0), "behind a windbreak is not under a roof");

    let there = Position::new(27, 25);
    simulation.world.add_building(Building::new_under_construction(BuildingType::LeanTo, there));
    assert!(simulation.world.cover_at(&there).is_none(), "half a lean-to keeps nothing off");
    simulation.world.add_building(a_finished(BuildingType::LeanTo, Position::new(29, 25)));
    assert!(simulation.world.is_out_of_the_weather_at(&Position::new(29, 25)));
}

/// Behind a windbreak in a cold wind, the body loses less heat than in the
/// open, and under a hut less again.
#[test]
fn a_windbreak_takes_the_wind_off_a_body() {
    let simulation = one_person_in_the_open();
    let climate = Climate { temperature: -5.0, humidity: 0.5, wind_speed: 30.0 };

    let after = |cover: Option<Cover>| {
        let mut body = simulation.population.agents[0].clone();
        for _ in 0..20 {
            body.update_temperature_under(&climate, cover, false);
        }
        body.body_temperature.current
    };
    let open = after(None);
    let screened = after(Some(BuildingType::Windbreak.cover()));
    let hut = after(Some(BuildingType::BrushHut.cover()));
    assert!(screened > open, "screened {screened} against open {open}");
    assert!(hut > screened, "hut {hut} against screened {screened}");
}

/// Brush falls down - built or half-built - and when it is down it is gone.
/// Nothing else in the list falls down.
#[test]
fn brush_falls_down_and_is_gone() {
    let mut world = World::new(WorldConfig::default());
    world.buildings.clear();
    let (a, b, c) = (Position::new(20, 20), Position::new(22, 20), Position::new(24, 20));
    world.add_building(a_finished(BuildingType::LeanTo, a));
    world.add_building(Building::new_under_construction(BuildingType::BrushHut, b));
    world.add_building(a_finished(BuildingType::SkinTent, c));
    for building in world.buildings.iter_mut() {
        building.condition = 0.0001;
    }
    world.take_a_turn();
    assert!(world.get_building_at(&a).is_none(), "a lean-to falls down");
    assert!(world.get_building_at(&b).is_none(), "and a hut nobody finished");
    assert!(world.get_building_at(&c).is_some(), "a tent does not");

    // A windbreak lasts a fortnight, a lean-to a month, a hut a season
    let lasts = |what: BuildingType| 1.0 / what.decay_rate() / 48.0;
    assert!((lasts(BuildingType::Windbreak) - 14.0).abs() < 0.5);
    assert!((lasts(BuildingType::LeanTo) - 30.0).abs() < 0.5);
    assert!((lasts(BuildingType::BrushHut) - 90.0).abs() < 0.5);
}

/// A brush hut goes up where its builder stands, takes the poles and cord on
/// the first turn, and is finished on what went into it then.
#[test]
fn a_brush_hut_goes_up_where_its_builder_stands() {
    let mut simulation = one_person_in_the_open();
    poles_and_cord(&mut simulation, 6, 1);
    let build = Action::Build { structure_type: "brushhut".to_string(), position: (25, 25, 0) };
    let here = Position::new(25, 25);

    let started = simulation.execute_action(&build, 0);
    assert!(started.success, "{:?}", started.message);
    assert_eq!(simulation.world.get_building_at(&here).map(|b| b.building_type), Some(BuildingType::BrushHut));
    assert_eq!(simulation.population.agents[0].how_many_i_have("wood"), 0, "the poles went into it");
    assert_eq!(simulation.population.agents[0].how_many_i_have("lashing"), 0, "and the cord");

    let mut turns = 0;
    while !simulation.world.get_building_at(&here).unwrap().is_completed() {
        let carried_on = simulation.execute_action(&build, 0);
        assert!(carried_on.success, "carried on with nothing more in hand: {:?}", carried_on.message);
        turns += 1;
        assert!(turns <= 3, "a hut is three turns of work");
    }
    assert!(simulation.world.is_out_of_the_weather_at(&here));
    assert!(simulation.world.pit_at(here).is_none(), "no store is dug under brush");

    // And another goes nowhere a roof stands already
    poles_and_cord(&mut simulation, 4, 1);
    let lean_to = Action::Build { structure_type: "leanto".to_string(), position: (25, 25, 0) };
    assert!(!simulation.execute_action(&lean_to, 0).success);
}

/// What brush goes up depends on the weather and the pack: a hut on a dry
/// day, a lean-to when it is coming down, a windbreak when there is only the
/// wind to be had against, and nothing under a roof.
#[test]
fn the_brush_put_up_suits_the_weather_and_the_pack() {
    let mut simulation = one_person_in_the_open();
    let here = (25, 25, 0);
    let called = |simulation: &Simulation| match simulation.cover_i_could_put_up(&simulation.population.agents[0], here) {
        Some(Action::Build { structure_type, .. }) => Some(structure_type),
        _ => None,
    };

    assert_eq!(called(&simulation), None, "nothing in the pack");
    poles_and_cord(&mut simulation, 6, 1);
    assert_eq!(called(&simulation).as_deref(), Some("brushhut"), "a dry day and a hut's worth");

    simulation.world.climate.weather.weather_type = WeatherType::Rain;
    assert_eq!(called(&simulation).as_deref(), Some("leanto"), "in the rain, the one that is up in a turn");

    let mut simulation = one_person_in_the_open();
    poles_and_cord(&mut simulation, 3, 0);
    assert_eq!(called(&simulation), None, "no wind, nothing worth screening");
    simulation.world.climate.weather.base_wind_speed = 30.0;
    assert_eq!(called(&simulation).as_deref(), Some("windbreak"), "three poles against a wind");

    simulation
        .world
        .add_building(a_finished(BuildingType::LeanTo, Position::new(25, 25)));
    assert_eq!(called(&simulation), None, "a roof here already");
}

/// Bedding is two wood of brush laid where one stands: a bed to sleep on and
/// warmth to lie on, trodden flat in a month.
#[test]
fn bedding_is_a_bed_and_warmth_for_a_month() {
    let mut simulation = one_person_in_the_open();
    let here = Position::new(25, 25);
    assert!(!simulation.execute_action(&Action::LayBedding, 0).success, "no brush");
    poles_and_cord(&mut simulation, 2, 0);
    let laid = simulation.execute_action(&Action::LayBedding, 0);
    assert!(laid.success, "{:?}", laid.message);
    assert!(simulation.world.bedding_at(&here));
    assert_eq!(simulation.population.agents[0].how_many_i_have("wood"), 0);

    // Sleep on it is better than sleep on the ground
    let mut on_the_ground = simulation.population.agents[0].clone();
    on_the_ground.state.position = (30, 30, 0);
    simulation.population.agents.push(on_the_ground);
    for index in 0..2 {
        simulation.population.agents[index].fatigue.level = 0.9;
    }
    simulation.execute_action(&Action::Sleep { duration: 4 }, 0);
    simulation.execute_action(&Action::Sleep { duration: 4 }, 1);
    let (bedded, bare) = (simulation.population.agents[0].fatigue.level, simulation.population.agents[1].fatigue.level);
    assert!(bedded < bare, "bedded {bedded} against bare {bare}");

    // And warmer to lie on
    let climate = Climate { temperature: 0.0, humidity: 0.5, wind_speed: 0.0 };
    let after = |on_bedding: bool| {
        let mut body = simulation.population.agents[0].clone();
        for _ in 0..20 {
            body.update_temperature_under(&climate, None, on_bedding);
        }
        body.body_temperature.current
    };
    assert!(after(true) > after(false));

    // Trodden flat in a month
    simulation.world.turn = Bedding::HOW_LONG_IT_LASTS + 1_000;
    simulation.world.bedding[0].laid = 1_000;
    simulation.world.take_a_turn();
    assert!(!simulation.world.bedding_at(&here));
}

/// Putting brush up is a way the Shelter drive can take now, and does, for
/// somebody cold in the open with a lean-to in the pack.
#[test]
fn putting_brush_up_is_a_way_to_shelter() {
    assert_eq!(Strategy::BuildTemporaryShelter.reach(), Reach::Now);
    let mut simulation = one_person_in_the_open();
    simulation.world.climate.weather.weather_type = WeatherType::Rain;
    simulation.population.agents[0].surroundings.foul_weather = true;
    simulation.population.agents[0].surroundings.under_shelter = false;
    assert!(simulation
        .can_this_way_be_taken(Strategy::BuildTemporaryShelter, &simulation.population.agents[0], (25, 25, 0))
        .is_none(), "nothing to put up with");
    poles_and_cord(&mut simulation, 4, 1);
    let way = simulation.can_this_way_be_taken(Strategy::BuildTemporaryShelter, &simulation.population.agents[0], (25, 25, 0));
    assert!(matches!(way, Some(Action::Build { ref structure_type, .. }) if structure_type == "leanto"), "got {way:?}");
}

/// Sent to bed in the open with brush in the pack, somebody puts it up over
/// where they lie, and with boughs to spare lays them under it; with a roof
/// in reach, they walk to it instead.
#[test]
fn nobody_with_brush_sleeps_in_the_open() {
    let mut simulation = one_person_in_the_open();
    let dawn = 12.0 - simulation.world.climate.calendar.current_season().day_length() / 2.0;
    simulation.world.climate.calendar.time_of_day = (dawn - 3.0f32).rem_euclid(24.0);
    if let Some(hunger) = simulation.population.agents[0].drives.get_mut(crate::core::DriveType::Hunger) {
        hunger.value = 0.0;
    }
    poles_and_cord(&mut simulation, 8, 1);

    let ask = |simulation: &mut Simulation, chosen: Action| simulation.what_the_night_asks(0, chosen, false);
    let first = ask(&mut simulation, Action::Sleep { duration: 4 });
    assert!(matches!(first, Action::Build { ref structure_type, .. } if structure_type == "brushhut"), "got {first:?}");
    let first = ask(&mut simulation, Action::Gather { resource_type: "wood".to_string() });
    assert!(matches!(first, Action::Build { .. }), "got {first:?}");

    // Once it is up, the two wood left over are bedding, and then sleep
    simulation
        .world
        .add_building(a_finished(BuildingType::BrushHut, Position::new(25, 25)));
    simulation.population.agents[0].inventory.remove_item("wood", 6);
    let then = ask(&mut simulation, Action::Sleep { duration: 4 });
    assert!(matches!(then, Action::LayBedding), "got {then:?}");
    simulation.execute_action(&Action::LayBedding, 0);
    let last = ask(&mut simulation, Action::Sleep { duration: 4 });
    assert!(matches!(last, Action::Sleep { .. }), "got {last:?}");
}

/// A tent is finished too, now: what its builder put into it on the first
/// turn counts as brought to the site, and the turns after want nothing more
/// in hand. Until #349 no tent in any world was ever finished.
#[test]
fn a_tent_is_finished_on_what_went_into_it() {
    let mut simulation = one_person_in_the_open();
    poles_and_cord(&mut simulation, 8, 2);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("hides".to_string(), 4, 1.0));
    let build = Action::Build { structure_type: "tent".to_string(), position: (25, 25, 0) };
    let started = simulation.execute_action(&build, 0);
    assert!(started.success, "{:?}", started.message);
    let site = simulation
        .world
        .buildings
        .iter()
        .find(|b| b.building_type == BuildingType::SkinTent)
        .map(|b| b.position)
        .expect("a tent going up");
    for taken in ["wood", "hides", "lashing"] {
        assert_eq!(simulation.population.agents[0].how_many_i_have(taken), 0, "the {taken} went into it");
    }

    simulation.population.agents[0].state.position = (site.x, site.y, 0);
    let carry_on = Action::Build { structure_type: "tent".to_string(), position: (site.x, site.y, 0) };
    // Forty of work at twelve a turn for an untried hand
    for _ in 0..4 {
        let worked = simulation.execute_action(&carry_on, 0);
        assert!(worked.success, "{:?}", worked.message);
    }
    assert!(simulation.world.get_building_at(&site).is_some_and(|tent| tent.is_completed()));
}
