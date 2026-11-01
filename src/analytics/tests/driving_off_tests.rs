// src/analytics/tests/driving_off_tests.rs
//! Driving a beast off the camp's ground (ISSUES_FOUND #377).
//!
//! - a boar lying by the hearth is driven off by those about, together
//! - one man alone does not try it, and it stands if he does
//! - a brand from a burning fire is what tips a small band over
//! - a boar off away from the camp is left alone
//! - a camp a beast has troubled is ringed with thorn, over several trips
//! - a closed ring turns a boar and not a hare, and shuts nothing in
//! - a troubled camp of three keeps a watch through the night, in turns;
//!   the watch drives off what it can and wakes whoever sleeps near what it
//!   cannot

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

const HEARTH: (i32, i32) = (30, 30);

/// `people` grown and well at camp, four cells from a boar lying on the
/// hearth, which is everybody's home.
fn a_boar_on_the_hearth(people: usize) -> (Simulation, uuid::Uuid) {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let boar = world.spawn_animal("boar".to_string(), HEARTH).expect("a boar should spawn");
    let mut population = Population::new();
    for _ in 0..people {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for (i, agent) in simulation.population.agents.iter_mut().enumerate() {
        agent.state.now_this_many_years_old(30);
        agent.state.health = 100.0;
        agent.state.position = (HEARTH.0 - 4, HEARTH.1 + i as i32 % 2, 0);
        agent.hearth = Some(HEARTH);
        agent.inventory.get_all_items_mut().clear();
    }
    (simulation, boar)
}

fn how_far_it_is_from_the_hearth(simulation: &Simulation, boar: uuid::Uuid) -> i32 {
    let at = simulation.world.animals.get(&boar).expect("the boar").position;
    (at.0 - HEARTH.0).abs().max((at.1 - HEARTH.1).abs())
}

/// Four grown people outweigh a boar together, and it bolts from the camp
/// as far as a frightened beast gets, or the edge of the country allows.
#[test]
fn a_boar_on_the_hearth_is_driven_off_by_those_about() {
    let (mut simulation, boar) = a_boar_on_the_hearth(4);
    let agent = simulation.population.agents[0].clone();
    let offered = simulation.a_beast_on_our_ground(&agent, agent.state.position);
    assert_eq!(offered, Some(Action::DriveOff { animal_id: boar }));

    let result = simulation.execute_action(&Action::DriveOff { animal_id: boar }, 0);
    assert!(result.success, "{result:?}");
    assert!(
        how_far_it_is_from_the_hearth(&simulation, boar) > Simulation::ON_THE_CAMPS_GROUND + Simulation::A_BEASTS_GROUND,
        "the boar should be far off the camp's ground, not {} cells",
        how_far_it_is_from_the_hearth(&simulation, boar)
    );
    assert_eq!(simulation.world.beasts_driven_off, 1);
}

/// One man alone is no match for a boar, so he does not try; and if he does,
/// it stands its ground.
#[test]
fn one_man_alone_does_not_try_and_it_stands_if_he_does() {
    let (mut simulation, boar) = a_boar_on_the_hearth(1);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_beast_on_our_ground(&agent, agent.state.position), None);

    let result = simulation.execute_action(&Action::DriveOff { animal_id: boar }, 0);
    assert!(!result.success);
    assert!(how_far_it_is_from_the_hearth(&simulation, boar) <= 1, "the boar should not have moved");
}

/// Two are not enough with bare hands, and are with brands from a fire
/// burning beside them.
#[test]
fn brands_from_a_fire_tip_a_small_band_over() {
    let (mut simulation, boar) = a_boar_on_the_hearth(2);
    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;
    let animal = simulation.world.animals.get(&boar).unwrap().clone();
    let bare = simulation.would_it_be_driven_off(&animal, (here.0, here.1));

    let fire = simulation
        .world
        .build_heat_source(crate::environment::HeatSourceType::Campfire, (here.0, here.1 + 1, 0), None)
        .expect("a fire");
    let _ = simulation.world.add_fuel_to_heat_source(&fire, "wood".to_string(), 200.0);
    let _ = simulation.world.light_heat_source(&fire);
    let with_brands = simulation.would_it_be_driven_off(&animal, (here.0, here.1));

    assert!(
        simulation.what_those_about_are_worth((here.0, here.1)) > 0.0,
        "two grown people are worth something to a boar"
    );
    assert!(!bare || with_brands, "brands never make it harder");
    assert!(with_brands, "two people with brands should drive a boar off");
}

/// A boar off in the wood, away from anybody's hearth, is nobody's business.
#[test]
fn a_boar_away_from_the_camp_is_left_alone() {
    let (mut simulation, _) = a_boar_on_the_hearth(4);
    for agent in simulation.population.agents.iter_mut() {
        agent.hearth = Some((HEARTH.0 - 40, HEARTH.1));
    }
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_beast_on_our_ground(&agent, agent.state.position), None);
}

/// Every tile `keep` does not keep made water.
fn water_beyond(simulation: &mut Simulation, keep: impl Fn(i32, i32) -> bool) {
    let (width, height) = (simulation.world.grid.width as i32, simulation.world.grid.height as i32);
    for x in 0..width {
        for y in 0..height {
            if !keep(x, y) {
                if let Some(tile) = simulation.world.grid.get_tile_mut(&crate::world::Position::new(x, y)) {
                    tile.terrain = crate::world::Terrain::new(crate::world::TerrainType::Water);
                }
            }
        }
    }
}

/// With a lake straight behind it, a boar driven off goes the other way
/// round it rather than standing where it was. Straight away and nowhere
/// else, a mammoth with water behind it was driven off every turn for two
/// years (#377).
#[test]
fn a_beast_with_water_behind_it_goes_round() {
    let (mut simulation, boar) = a_boar_on_the_hearth(4);
    water_beyond(&mut simulation, |x, _| x <= HEARTH.0 + 3);

    let result = simulation.execute_action(&Action::DriveOff { animal_id: boar }, 0);
    assert!(result.success, "{result:?}");
    let at = simulation.world.animals.get(&boar).unwrap().position;
    assert!(
        how_far_it_is_from_the_hearth(&simulation, boar) > Simulation::ON_THE_CAMPS_GROUND + Simulation::A_BEASTS_GROUND,
        "it should be off the camp's ground, at {at:?}"
    );
    let tile = simulation.world.grid.get_tile(&crate::world::Position::new(at.0, at.1)).unwrap();
    assert!(tile.terrain.is_walkable(), "and on ground it can stand on");
}

/// And with nowhere at all to go, nobody tries it: it is not driven off by
/// being shouted at where it stands.
#[test]
fn a_beast_with_nowhere_to_go_is_not_driven() {
    let (mut simulation, boar) = a_boar_on_the_hearth(4);
    water_beyond(&mut simulation, |x, y| (x - HEARTH.0).abs().max((y - HEARTH.1).abs()) <= 6);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_beast_on_our_ground(&agent, agent.state.position), None);
    let result = simulation.execute_action(&Action::DriveOff { animal_id: boar }, 0);
    assert!(!result.success);
    assert_eq!(simulation.world.beasts_driven_off, 0);
}

// --------------------------------------------------------------------------
// A ring of thorn round the camp (#378)
// --------------------------------------------------------------------------

fn give_wood(simulation: &mut Simulation, who: usize, how_many: u32) {
    simulation.population.agents[who]
        .inventory
        .add_item(crate::agents::InventoryItem::new_with_weight("wood".to_string(), how_many, 0.1));
}

/// Nobody hedges a camp no beast has troubled; one that remembers a beast on
/// its ground lays brush, and enough of it closes the ring.
#[test]
fn a_troubled_camp_is_ringed_with_thorn() {
    let (mut simulation, _) = a_boar_on_the_hearth(4);
    simulation.world.animals.get_all_mut().clear();
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (HEARTH.0, HEARTH.1, 0);
    }
    give_wood(&mut simulation, 0, 30);
    let here = simulation.population.agents[0].state.position;
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_thorn_hedge_to_lay(&agent, here), None, "an untroubled camp is not hedged");

    let now = simulation.current_turn;
    simulation.population.agents[0]
        .exploration_knowledge
        .saw_danger(crate::world::Position::new(HEARTH.0 + 1, HEARTH.1), "Wild Boar", 1.0, now);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_thorn_hedge_to_lay(&agent, here), Some(Action::PutUpThornHedge));

    let result = simulation.execute_action(&Action::PutUpThornHedge, 0);
    assert!(result.success, "{result:?}");
    let hedge = &simulation.world.thorn_hedges[0];
    assert_eq!(hedge.laid, crate::world::ThornHedge::WHAT_IT_TAKES);
    assert!(hedge.is_closed(simulation.world.turn), "a full ring of brush is closed");
    assert_eq!(simulation.world.thorn_hedges_closed, 1);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_thorn_hedge_to_lay(&agent, here), None, "and is not laid again");
}

/// Several people bring brush to the same ring until it is closed.
#[test]
fn a_ring_is_laid_by_several_trips() {
    let (mut simulation, _) = a_boar_on_the_hearth(2);
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (HEARTH.0, HEARTH.1 + 1, 0);
    }
    let half = crate::world::ThornHedge::WHAT_IT_TAKES / 2;
    give_wood(&mut simulation, 0, half);
    give_wood(&mut simulation, 1, half);
    assert!(simulation.execute_action(&Action::PutUpThornHedge, 0).success);
    assert!(simulation.world.thorn_hedges[0].closed_on.is_none(), "half a ring keeps nothing out");
    assert!(simulation.execute_action(&Action::PutUpThornHedge, 1).success);
    assert_eq!(simulation.world.thorn_hedges.len(), 1, "one ring, not two");
    assert!(simulation.world.thorn_hedges[0].closed_on.is_some());
}

/// A boar that wanders up to a closed ring goes no further; a hare goes
/// under it; and a boar already inside when it was closed is not shut in.
#[test]
fn a_closed_ring_turns_a_boar_and_not_a_hare() {
    let (mut simulation, boar) = a_boar_on_the_hearth(1);
    let hare = simulation.world.spawn_animal("rabbit".to_string(), (HEARTH.0 + 10, HEARTH.1 + 3)).expect("a rabbit");
    let outside = (HEARTH.0 + 10, HEARTH.1 + 2);
    if let Some(animal) = simulation.world.animals.get_mut(&boar) {
        animal.position = outside;
    }
    let now = simulation.world.turn;
    simulation.world.thorn_hedges.push(crate::world::ThornHedge {
        around: crate::world::Position::new(HEARTH.0, HEARTH.1),
        started: now,
        laid: crate::world::ThornHedge::WHAT_IT_TAKES,
        closed_on: Some(now),
    });

    let stood = simulation.world.where_the_thorns_would_turn_them();
    for animal in [boar, hare] {
        if let Some(animal) = simulation.world.animals.get_mut(&animal) {
            animal.position = (HEARTH.0 + 1, HEARTH.1);
        }
    }
    simulation.world.the_thorns_turn_them(stood);

    assert_eq!(simulation.world.animals.get(&boar).unwrap().position, outside, "the boar is turned");
    assert_eq!(simulation.world.animals.get(&hare).unwrap().position, (HEARTH.0 + 1, HEARTH.1), "the hare goes under");
    assert_eq!(simulation.world.beasts_the_thorns_turned, 1);

    // Inside when the ring closed, and moving about inside it: let be
    if let Some(animal) = simulation.world.animals.get_mut(&boar) {
        animal.position = (HEARTH.0 + 2, HEARTH.1);
    }
    let stood_inside = simulation.world.where_the_thorns_would_turn_them();
    if let Some(animal) = simulation.world.animals.get_mut(&boar) {
        animal.position = (HEARTH.0 + 3, HEARTH.1);
    }
    simulation.world.the_thorns_turn_them(stood_inside);
    assert_eq!(simulation.world.animals.get(&boar).unwrap().position, (HEARTH.0 + 3, HEARTH.1), "one inside is not stopped moving");
}

/// Driving a beast off the camp leaves those who did it remembering a beast
/// on the camp's ground, which is what sets them hedging it.
#[test]
fn a_beast_driven_off_sets_the_camp_hedging() {
    let (mut simulation, boar) = a_boar_on_the_hearth(4);
    assert!(simulation.execute_action(&Action::DriveOff { animal_id: boar }, 0).success);
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (HEARTH.0, HEARTH.1, 0);
    }
    give_wood(&mut simulation, 1, 5);
    let agent = simulation.population.agents[1].clone();
    assert_eq!(simulation.a_thorn_hedge_to_lay(&agent, agent.state.position), Some(Action::PutUpThornHedge));
}

// --------------------------------------------------------------------------
// A watch through the night (#379)
// --------------------------------------------------------------------------

/// The middle of the night, and a camp of `people` that remembers a beast on
/// its ground, all asleep by the hearth, with nothing about.
fn a_troubled_camp_at_night(people: usize) -> Simulation {
    let (mut simulation, _) = a_boar_on_the_hearth(people);
    simulation.world.animals.get_all_mut().clear();
    simulation.world.climate.calendar.time_of_day = 1.0;
    let now = simulation.current_turn;
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (HEARTH.0, HEARTH.1 + 1, 0);
        agent
            .exploration_knowledge
            .saw_danger(crate::world::Position::new(HEARTH.0 + 1, HEARTH.1), "Wild Boar", 1.0, now);
    }
    assert!(simulation.where_in_the_night_it_is().0, "one in the morning is night");
    simulation
}

fn who_is_on_watch(simulation: &Simulation) -> Vec<usize> {
    (0..simulation.population.agents.len()).filter(|&i| simulation.on_watch_now(i)).collect()
}

/// One of a troubled camp of three is on watch at any hour of the night,
/// and the watch goes round them.
#[test]
fn one_of_a_troubled_camp_keeps_watch() {
    let mut simulation = a_troubled_camp_at_night(3);
    let first = who_is_on_watch(&simulation);
    assert_eq!(first.len(), 1, "one on watch, not {first:?}");
    assert_eq!(simulation.what_the_watch_does(first[0]), Action::KeepWatch);

    // Later in the night, and the next night, somebody else
    let mut watchers = std::collections::BTreeSet::new();
    for night in 0..3u32 {
        for hour in [23.0f32, 1.0, 3.0] {
            simulation.current_turn = night * crate::environment::seasons::TICKS_PER_DAY;
            simulation.world.climate.calendar.time_of_day = hour;
            watchers.extend(who_is_on_watch(&simulation));
        }
    }
    assert_eq!(watchers.len(), 3, "the watch goes round all three: {watchers:?}");
}

/// No watch at a camp of two, at an untroubled camp, by day, or inside a
/// closed thorn ring.
#[test]
fn no_watch_where_none_is_wanted() {
    let simulation = a_troubled_camp_at_night(2);
    assert!(who_is_on_watch(&simulation).is_empty(), "two are too few to keep a watch");

    let (mut untroubled, _) = a_boar_on_the_hearth(3);
    untroubled.world.climate.calendar.time_of_day = 1.0;
    assert!(who_is_on_watch(&untroubled).is_empty(), "nobody watches an untroubled camp");

    let mut by_day = a_troubled_camp_at_night(3);
    by_day.world.climate.calendar.time_of_day = 12.0;
    assert!(who_is_on_watch(&by_day).is_empty(), "nobody keeps a night watch at noon");

    let mut ringed = a_troubled_camp_at_night(3);
    let now = ringed.world.turn;
    ringed.world.thorn_hedges.push(crate::world::ThornHedge {
        around: crate::world::Position::new(HEARTH.0, HEARTH.1),
        started: now,
        laid: crate::world::ThornHedge::WHAT_IT_TAKES,
        closed_on: Some(now),
    });
    assert!(who_is_on_watch(&ringed).is_empty(), "a closed ring keeps the beasts out without a watch");
}

/// A boar coming onto the camp in the night is driven off by the watch with
/// the camp behind it.
#[test]
fn the_watch_drives_off_a_boar_with_the_camp_behind_it() {
    let mut simulation = a_troubled_camp_at_night(4);
    let boar = simulation.world.spawn_animal("boar".to_string(), (HEARTH.0 + 4, HEARTH.1)).expect("a boar");
    let watcher = who_is_on_watch(&simulation)[0];
    assert_eq!(simulation.what_the_watch_does(watcher), Action::DriveOff { animal_id: boar });
}

/// A mammoth the camp could not drive off: the watch wakes whoever is asleep
/// near it, frightened of it, so they get out of its way.
#[test]
fn the_watch_wakes_whoever_sleeps_near_a_beast_it_cannot_drive() {
    let mut simulation = a_troubled_camp_at_night(3);
    simulation.world.spawn_animal("mammoth".to_string(), (HEARTH.0 + 3, HEARTH.1 + 1)).expect("a mammoth");
    let watcher = who_is_on_watch(&simulation)[0];
    let sleeper = (watcher + 1) % 3;
    simulation.population.agents[sleeper].busy_until = simulation.current_turn + 100;

    let result = simulation.execute_action(&Action::KeepWatch, watcher);
    assert!(result.success, "{result:?}");
    assert_eq!(simulation.population.agents[sleeper].busy_until, 0, "the sleeper is woken");
    assert!(
        simulation.population.agents[sleeper].emotions.what_frightens_me_most().is_some(),
        "and frightened of it"
    );
    assert!(simulation.world.woken_by_the_watch >= 1);
}

/// Nobody goes to bed with a boar lying by the hearth that the camp could
/// drive off: the night sends them to drive it off first (#379).
#[test]
fn nobody_goes_to_bed_with_a_boar_by_the_hearth() {
    let mut simulation = a_troubled_camp_at_night(4);
    let boar = simulation.world.spawn_animal("boar".to_string(), (HEARTH.0 + 4, HEARTH.1)).expect("a boar");
    let asked = simulation.what_the_night_asks(0, Action::Wait, false);
    assert_eq!(asked, Action::DriveOff { animal_id: boar });
}

/// And at night the camp's ground is wherever its people sleep: a boar
/// fifteen cells from the hearth, beside a sleeper, is the camp's business.
#[test]
fn by_night_the_camps_ground_is_where_its_people_sleep() {
    let mut simulation = a_troubled_camp_at_night(4);
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (HEARTH.0 - 15, HEARTH.1, 0);
    }
    let boar = simulation.world.spawn_animal("boar".to_string(), (HEARTH.0 - 11, HEARTH.1)).expect("a boar");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.a_beast_on_our_ground(&agent, agent.state.position),
        Some(Action::DriveOff { animal_id: boar })
    );
    simulation.world.climate.calendar.time_of_day = 12.0;
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_beast_on_our_ground(&agent, agent.state.position), None, "by day it is out of the camp");
}

/// A closed ring saves the camp its watch, unless a beast is inside it: one
/// inside when the ring closed is not shut out.
#[test]
fn a_ring_with_a_beast_inside_it_is_still_watched() {
    let mut simulation = a_troubled_camp_at_night(3);
    let now = simulation.world.turn;
    simulation.world.thorn_hedges.push(crate::world::ThornHedge {
        around: crate::world::Position::new(HEARTH.0, HEARTH.1),
        started: now,
        laid: crate::world::ThornHedge::WHAT_IT_TAKES,
        closed_on: Some(now),
    });
    assert!(who_is_on_watch(&simulation).is_empty());
    simulation.world.spawn_animal("boar".to_string(), (HEARTH.0 + 1, HEARTH.1)).expect("a boar");
    assert_eq!(who_is_on_watch(&simulation).len(), 1, "a boar inside the ring is watched for");
}

/// A hungry boar drawn to meat lying inside the thorn ring does not walk
/// through it to get at it, as scavengers drawn to a camp's refuse did
/// (#379).
#[test]
fn the_ring_turns_a_scavenger_drawn_to_meat_inside_it() {
    let (mut simulation, boar) = a_boar_on_the_hearth(1);
    let outside = (HEARTH.0 + 10, HEARTH.1);
    let meat = crate::world::Position::new(HEARTH.0 + 3, HEARTH.1);
    if let Some(animal) = simulation.world.animals.get_mut(&boar) {
        animal.position = outside;
        animal.hunger = animal.max_hunger;
    }
    let now = simulation.world.turn;
    simulation.world.a_carcass_lies_here(meat, 20, now);
    simulation.world.thorn_hedges.push(crate::world::ThornHedge {
        around: crate::world::Position::new(HEARTH.0, HEARTH.1),
        started: now,
        laid: crate::world::ThornHedge::WHAT_IT_TAKES,
        closed_on: Some(now),
    });
    for _ in 0..10 {
        simulation.take_a_turn();
    }
    let at = simulation.world.animals.get(&boar).map(|animal| animal.position).unwrap_or(outside);
    assert!(
        !simulation.world.thorn_hedges[0].takes_in(at),
        "a boar drawn to the meat is inside the ring at {at:?}"
    );
}
