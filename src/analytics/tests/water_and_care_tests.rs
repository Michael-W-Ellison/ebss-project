// src/analytics/tests/water_and_care_tests.rs
//! Water and care, the last of the unlinked survival machinery (#351):
//! bleeding and pressure on wounds, bandaging, rest, the water skin, and
//! water taken along on a walk.

use crate::agents::body::{BodyPartType, InjuryType};
use crate::agents::{Agent, AgentConfig, AgentState, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::catalogue;
use crate::environment::Action;
use crate::world::{Position, TerrainType, World, WorldConfig};

/// `how_many` grown people side by side from (25, 25), nothing on them, on
/// plains with no water anywhere.
fn people(how_many: usize) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.retain(|node| node.resource_type != crate::world::ResourceType::Water);
    for x in 0..200 {
        for y in 0..200 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain.terrain_type = TerrainType::Plains;
            }
        }
    }
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
        agent.state.now_this_many_years_old(25);
        agent.state.position = (25 + index as i32, 25, 0);
        agent.inventory.get_all_items_mut().clear();
    }
    simulation
}

fn a_strip(simulation: &mut Simulation, index: usize) {
    simulation.population.agents[index]
        .inventory
        .add_item(InventoryItem::new_with_weight("lashing".to_string(), 1, 0.1));
}

/// Pressure is born knowledge; a dressing and a water skin are found out.
#[test]
fn the_catalogue_knows_how_care_and_water_come() {
    assert!(catalogue::is_born_knowledge("do:PressOnWound"));
    for found_out in ["do:Bandage", "make:waterskin"] {
        assert!(!catalogue::is_born_knowledge(found_out), "{found_out}");
        assert!(catalogue::what_comes_before(found_out).is_empty(), "{found_out} is found out, not reached");
    }
}

/// A blow that opens a wound bleeds as hard as it is open, the bleeding
/// costs health every half hour, and it clots by itself.
#[test]
fn a_blow_bleeds_and_clots() {
    let mut simulation = people(1);
    let agent = &mut simulation.population.agents[0];
    agent.state.take_damage(AgentState::WHAT_A_BLOW_HAS_TO_BE_TO_LEAVE_A_WOUND);
    assert_eq!(agent.state.bleeding, AgentState::WHAT_AN_OPEN_WOUND_BLEEDS);

    let before = agent.state.health;
    agent.turn_the_wound(0);
    assert!(agent.state.health < before, "it costs health");
    assert!(agent.state.bleeding < AgentState::WHAT_AN_OPEN_WOUND_BLEEDS, "and slows");
    let mut turns = 1;
    while agent.state.bleeding > 0.0 {
        agent.turn_the_wound(turns);
        turns += 1;
        assert!(turns < 48, "it stops within a day");
    }
    let lost = before - agent.state.health;
    assert!((20.0..35.0).contains(&lost), "left alone, the worst of wounds costs {lost}");

    // A scratch barely bleeds
    let mut simulation = people(1);
    let agent = &mut simulation.population.agents[0];
    agent.state.take_damage(2.0);
    assert!(agent.state.bleeding < Agent::BLEEDING_WORTH_PRESSING);
}

/// One's own hand slows it to a quarter; somebody else's stops it; and
/// pressing on wounds is how a dressing is found out.
#[test]
fn pressure_slows_a_wound_and_teaches_a_dressing() {
    let mut simulation = people(2);
    simulation.population.agents[0].state.take_damage(30.0);
    let pressed = simulation.execute_action(&Action::PressOnWound { who: None }, 0);
    assert!(pressed.success, "{:?}", pressed.message);
    let bleeding = simulation.population.agents[0].state.bleeding;
    assert!((bleeding - AgentState::WHAT_AN_OPEN_WOUND_BLEEDS * 0.25).abs() < 1e-4, "{bleeding}");

    let patient = simulation.population.agents[0].id;
    let helped = simulation.execute_action(&Action::PressOnWound { who: Some(patient) }, 1);
    assert!(helped.success, "{:?}", helped.message);
    assert_eq!(simulation.population.agents[0].state.bleeding, 0.0);
    assert!(!simulation.execute_action(&Action::PressOnWound { who: Some(patient) }, 1).success, "nothing to press on now");

    // Out of reach is out of reach
    simulation.population.agents[0].state.take_damage(30.0);
    simulation.population.agents[1].state.position = (40, 25, 0);
    assert!(!simulation.execute_action(&Action::PressOnWound { who: Some(patient) }, 1).success);

    // Four wounds pressed and the carer ties a strip over the next
    simulation.population.agents[1].state.position = (26, 25, 0);
    assert!(!simulation.population.agents[1].knows_how_to_do("Bandage"));
    for _ in 0..3 {
        simulation.population.agents[0].state.health = 100.0;
        simulation.population.agents[0].state.take_damage(30.0);
        assert!(simulation.execute_action(&Action::PressOnWound { who: Some(patient) }, 1).success);
    }
    assert!(simulation.population.agents[1].knows_how_to_do("Bandage"));
}

/// A dressing wants a strip, stops the bleeding, and for a week the wound
/// closes twice as fast.
#[test]
fn a_dressing_stops_the_bleeding_and_closes_the_wound() {
    let mut simulation = people(1);
    simulation.population.agents[0].found_out_how_to("do:Bandage");
    simulation.population.agents[0].state.take_damage(30.0);
    assert!(!simulation.execute_action(&Action::Bandage { who: None }, 0).success, "nothing to bind it with");

    a_strip(&mut simulation, 0);
    let bound = simulation.execute_action(&Action::Bandage { who: None }, 0);
    assert!(bound.success, "{:?}", bound.message);
    let state = &simulation.population.agents[0].state;
    assert_eq!(state.bleeding, 0.0);
    assert!(state.wound_dressed_until > simulation.current_turn);
    assert_eq!(simulation.population.agents[0].how_many_i_have("lashing"), 0);

    // Closes twice as fast as one left open
    let mut dressed = simulation.population.agents[0].clone();
    let mut open = simulation.population.agents[0].clone();
    open.state.wound_dressed_until = 0;
    for turn in 0..48 {
        dressed.turn_the_wound(turn);
        open.turn_the_wound(turn);
    }
    let healed_dressed = 1.0 - dressed.state.an_open_wound;
    let healed_open = 1.0 - open.state.an_open_wound;
    assert!((healed_dressed - 2.0 * healed_open).abs() < 1e-3, "{healed_dressed} against {healed_open}");
}

/// Lying still, a body mends as much again: injuries heal twice as fast
/// asleep, and so does an open wound.
#[test]
fn rest_mends_twice_as_fast() {
    let mut simulation = people(1);
    let agent = &mut simulation.population.agents[0];
    agent
        .body
        .get_part_mut(BodyPartType::LeftLeg)
        .expect("a leg")
        .apply_injury(InjuryType::Major, 20.0, 0);
    let mut awake = agent.body.clone();
    let mut asleep = agent.body.clone();
    awake.take_a_turn_resting(false);
    asleep.take_a_turn_resting(true);
    let leg = |body: &crate::agents::body::Body| body.parts[&BodyPartType::LeftLeg].health;
    let start = agent.body.parts[&BodyPartType::LeftLeg].health;
    assert!((leg(&asleep) - start) > 1.9 * (leg(&awake) - start), "asleep {} awake {}", leg(&asleep), leg(&awake));
}

/// A wound bleeding is seen to before anything but running: one's own, by
/// pressure or a dressing; somebody's within reach; one's own kin walked to.
#[test]
fn a_bleeding_wound_comes_first() {
    let mut simulation = people(2);
    let busy = Action::Gather { resource_type: "wood".to_string() };
    simulation.population.agents[0].state.take_damage(30.0);

    let asked = simulation.what_a_wound_asks(0, busy.clone(), false);
    assert!(matches!(asked, Action::PressOnWound { who: None }), "got {asked:?}");
    let fled = simulation.what_a_wound_asks(0, busy.clone(), true);
    assert!(matches!(fled, Action::Gather { .. }), "running for it comes first, got {fled:?}");

    // The one beside them presses on it for them
    let patient = simulation.population.agents[0].id;
    let asked = simulation.what_a_wound_asks(1, busy.clone(), false);
    assert!(matches!(asked, Action::PressOnWound { who: Some(who) } if who == patient), "got {asked:?}");

    // One who knows how, with a strip, binds it
    simulation.population.agents[0].found_out_how_to("do:Bandage");
    a_strip(&mut simulation, 0);
    let asked = simulation.what_a_wound_asks(0, busy.clone(), false);
    assert!(matches!(asked, Action::Bandage { who: None }), "got {asked:?}");

    // A stranger across the camp is not walked to; one's own child is
    simulation.population.agents[1].state.position = (33, 25, 0);
    let asked = simulation.what_a_wound_asks(1, busy.clone(), false);
    assert!(matches!(asked, Action::Gather { .. }), "a stranger out of reach, got {asked:?}");
    let parent = simulation.population.agents[1].id;
    simulation.population.agents[0].parent_ids.push(parent);
    let asked = simulation.what_a_wound_asks(1, busy, false);
    assert!(matches!(asked, Action::Move { target: (25, 25, 0) }), "got {asked:?}");
}

/// Three days thirsty with no water in reach and somebody thinks of taking
/// it along in a skin; then they sew one shut from a hide, and it holds 8.
#[test]
fn going_thirsty_far_from_water_is_how_a_water_skin_is_found_out() {
    let mut simulation = people(1);
    for day in 1..=3 {
        if let Some(thirst) = simulation.population.agents[0].drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.9;
        }
        simulation.current_turn = day * crate::environment::seasons::TICKS_PER_DAY;
        simulation.who_went_thirsty_far_from_water();
    }
    assert!(simulation.population.agents[0].what_i_found_out().contains("waterskin"));

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("hides".to_string(), 1, 1.0));
    assert_eq!(simulation.population.agents[0].what_vessel_i_would_rather_have(), Some(("weave", "hides")));
    let sewn = simulation.execute_action(&Action::Work { verb: "weave".to_string(), to: "hides".to_string() }, 0);
    assert!(sewn.success, "{:?}", sewn.message);
    let skin = simulation.population.agents[0].inventory.get_item("waterskin").expect("a water skin");
    assert_eq!(skin.max_capacity, Some(8.0));
}

/// Somebody setting off on a long walk from beside the water, with room in
/// a vessel, fills it first; a short walk, or no vessel, and they just go.
#[test]
fn a_vessel_is_filled_before_a_long_walk() {
    let mut simulation = people(1);
    let spring = crate::world::ResourceNode::new(crate::world::ResourceType::Water, Position::new(26, 25), 100);
    simulation.world.resources.push(spring);
    simulation.world.file_the_nodes();
    let far = Action::Move { target: (80, 25, 0) };

    let asked = simulation.water_for_the_walk(0, far.clone(), false);
    assert!(matches!(asked, Action::Move { .. }), "no vessel, got {asked:?}");

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_container("bowl".to_string(), 1, 4.0));
    let asked = simulation.water_for_the_walk(0, far.clone(), false);
    assert!(matches!(asked, Action::Gather { ref resource_type } if resource_type == "water"), "got {asked:?}");

    let near = simulation.water_for_the_walk(0, Action::Move { target: (28, 25, 0) }, false);
    assert!(matches!(near, Action::Move { .. }), "a short walk, got {near:?}");
}
