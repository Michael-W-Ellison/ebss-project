// src/analytics/tests/medicine_tests.rs
//! Medicine's first stage (#399-#404): wounds washed, seared and sewn, broken
//! limbs and setting them, hard births and the help at them, and remedies
//! steeped and bound in.

use crate::agents::pregnancy::PregnancyState;
use crate::agents::{Agent, AgentConfig, AgentState, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::traits::Trait;
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
    for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
        agent.state.now_this_many_years_old(25);
        agent.state.position = (25 + index as i32, 25, 0);
        agent.inventory.get_all_items_mut().clear();
        agent.traits = crate::core::traits::TraitSet::new();
    }
    simulation
}

fn give(simulation: &mut Simulation, index: usize, what: &str, how_many: u32) {
    simulation.population.agents[index]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.1));
}

fn a_waterskin(simulation: &mut Simulation, index: usize) {
    let mut waterskin = InventoryItem::new_with_weight("waterskin".to_string(), 1, 0.5);
    waterskin.max_capacity = Some(2.0);
    waterskin.fill_level = Some(2.0);
    simulation.population.agents[index].inventory.add_item(waterskin);
}

fn a_fire_at(simulation: &mut Simulation, at: (i32, i32)) {
    let mut hearth =
        crate::environment::HeatSource::new(crate::environment::HeatSourceType::Campfire, (at.0, at.1, 0), 0);
    hearth.add_fuel("wood".to_string(), 1.0, 100);
    assert!(hearth.light());
    simulation.world.heat_sources.add(hearth);
}

/// A wound as open as they come, bleeding as hard as it is open, and no bone
/// broken by it.
fn wounded(simulation: &mut Simulation, index: usize) {
    let state = &mut simulation.population.agents[index].state;
    state.an_open_wound = 1.0;
    state.bleeding = AgentState::WHAT_AN_OPEN_WOUND_BLEEDS;
    state.wound_cleaned = false;
    state.wound_stitched = false;
    state.wound_poulticed = false;
}

/// Nothing in the first stage is born knowledge, and none of it waits on
/// anything else: each is found out from what happens.
#[test]
fn the_catalogue_knows_medicine_is_found_out() {
    for found_out in
        ["do:CleanWound", "do:Cauterise", "do:StitchWound", "do:SetBone", "do:AttendBirth", "know:poultices_and_teas"]
    {
        assert!(!catalogue::is_born_knowledge(found_out), "{found_out}");
        assert!(catalogue::what_comes_before(found_out).is_empty(), "{found_out} is found out, not reached");
    }
}

/// A wound washed, with water carried, turns half as often; a fresh blow
/// opens a fresh, dirty one.
#[test]
fn a_washed_wound_turns_half_as_often() {
    let mut simulation = people(1);
    simulation.population.agents[0].found_out_how_to("do:CleanWound");
    wounded(&mut simulation, 0);
    assert!(!simulation.execute_action(&Action::CleanWound { who: None }, 0).success, "no water");

    a_waterskin(&mut simulation, 0);
    let dirty = simulation.population.agents[0].how_likely_my_wound_is_to_turn(0);
    let washed = simulation.execute_action(&Action::CleanWound { who: None }, 0);
    assert!(washed.success, "{:?}", washed.message);
    let agent = &simulation.population.agents[0];
    assert!(agent.state.wound_cleaned);
    assert!((agent.how_much_water_i_carry() - 1.5).abs() < 1e-4, "{}", agent.how_much_water_i_carry());
    let clean = agent.how_likely_my_wound_is_to_turn(0);
    assert!((clean - dirty * 0.5).abs() < 1e-12, "{clean} against {dirty}");
    assert_eq!(simulation.world.wounds_cleaned, 1);

    simulation.population.agents[0].state.an_open_wound = 0.2;
    simulation.population.agents[0].state.take_damage(30.0);
    assert!(!simulation.population.agents[0].state.wound_cleaned, "a fresh wound is a dirty one");
}

/// Wounds seen to turn, twice, and somebody near is apt to wash the next;
/// somebody across the world sees nothing of it.
#[test]
fn washing_comes_of_seeing_wounds_turn() {
    let mut simulation = people(3);
    simulation.population.agents[2].state.position = (80, 80, 0);
    for _ in 0..crate::analytics::Simulation::WOUNDS_SEEN_TO_TURN_BEFORE_WASHING {
        assert!(!simulation.population.agents[1].knows_how_to_do("CleanWound"));
        simulation.population.agents[0].state.to_be_seen.push(AgentState::A_WOUND_TURNED.to_string());
        simulation.what_the_camp_saw_of_bodies();
    }
    assert!(simulation.population.agents[1].knows_how_to_do("CleanWound"));
    assert!(!simulation.population.agents[2].knows_how_to_do("CleanWound"), "too far off to see");
    assert_eq!(simulation.world.wounds_turned, 2);
    assert!(simulation.population.agents[0].state.to_be_seen.is_empty());
}

/// A brand from a lit fire stops the bleeding at once and sears the wound
/// half shut and clean, for a burn.
#[test]
fn a_brand_stops_the_bleeding_and_sears_the_wound() {
    let mut simulation = people(1);
    simulation.population.agents[0].found_out_how_to("do:Cauterise");
    wounded(&mut simulation, 0);
    assert!(!simulation.execute_action(&Action::Cauterise { who: None }, 0).success, "no fire");

    a_fire_at(&mut simulation, (25, 26));
    let health = simulation.population.agents[0].state.health;
    let seared = simulation.execute_action(&Action::Cauterise { who: None }, 0);
    assert!(seared.success, "{:?}", seared.message);
    let state = &simulation.population.agents[0].state;
    assert_eq!(state.bleeding, 0.0);
    assert!((state.an_open_wound - 0.5).abs() < 1e-6);
    assert!(state.wound_cleaned);
    assert!(state.health < health, "a burn");
    assert_eq!(simulation.world.wounds_cauterised, 1);
}

/// Seeing to wounds bleeding hard beside a fire, three times, brings the
/// brand to mind.
#[test]
fn the_brand_comes_of_wounds_bleeding_by_a_fire() {
    let mut simulation = people(1);
    a_fire_at(&mut simulation, (25, 26));
    for _ in 0..crate::analytics::Simulation::WOUNDS_BLED_BY_THE_FIRE_BEFORE_SEARING {
        assert!(!simulation.population.agents[0].knows_how_to_do("Cauterise"));
        wounded(&mut simulation, 0);
        assert!(simulation.execute_action(&Action::PressOnWound { who: None }, 0).success);
    }
    assert!(simulation.population.agents[0].knows_how_to_do("Cauterise"));
}

/// Who will lay a brand on a wound is temperament: the cowardly and the
/// anxious will not sear their own, and a kind heart will not sear anybody's.
#[test]
fn the_cowardly_will_not_sear_their_own_nor_the_kind_another() {
    let busy = Action::Gather { resource_type: "wood".to_string() };
    let mut simulation = people(2);
    a_fire_at(&mut simulation, (25, 26));
    for index in 0..2 {
        simulation.population.agents[index].found_out_how_to("do:Cauterise");
    }
    wounded(&mut simulation, 0);
    let asked = simulation.what_a_wound_asks(0, busy.clone(), false);
    assert!(matches!(asked, Action::Cauterise { who: None }), "got {asked:?}");
    simulation.population.agents[0].traits.add_trait(Trait::Coward);
    let asked = simulation.what_a_wound_asks(0, busy.clone(), false);
    assert!(matches!(asked, Action::PressOnWound { who: None }), "a coward presses, got {asked:?}");

    let patient = simulation.population.agents[0].id;
    let asked = simulation.what_a_wound_asks(1, busy.clone(), false);
    assert!(matches!(asked, Action::Cauterise { who: Some(who) } if who == patient), "got {asked:?}");
    simulation.population.agents[1].traits.add_trait(Trait::KindHearted);
    let asked = simulation.what_a_wound_asks(1, busy, false);
    assert!(matches!(asked, Action::PressOnWound { who: Some(who) } if who == patient), "got {asked:?}");
}

/// A wound sewn with a point and a thread bleeds no more and closes twice
/// as fast.
#[test]
fn a_sewn_wound_closes_twice_as_fast() {
    let mut simulation = people(1);
    simulation.population.agents[0].found_out_how_to("do:StitchWound");
    wounded(&mut simulation, 0);
    give(&mut simulation, 0, "lashing", 1);
    assert!(!simulation.execute_action(&Action::StitchWound { who: None }, 0).success, "nothing to pierce with");

    give(&mut simulation, 0, "needle", 1);
    let sewn = simulation.execute_action(&Action::StitchWound { who: None }, 0);
    assert!(sewn.success, "{:?}", sewn.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("lashing"), 0);
    let mut stitched = simulation.population.agents[0].clone();
    assert!(stitched.state.wound_stitched);
    assert_eq!(stitched.state.bleeding, 0.0);
    let mut open = stitched.clone();
    open.state.wound_stitched = false;
    for turn in 0..48 {
        stitched.turn_the_wound(turn);
        open.turn_the_wound(turn);
    }
    let healed_sewn = 1.0 - stitched.state.an_open_wound;
    let healed_open = 1.0 - open.state.an_open_wound;
    assert!((healed_sewn - 2.0 * healed_open).abs() < 1e-3, "{healed_sewn} against {healed_open}");
}

/// Binding wounds with a point in the pack, three times, brings sewing them
/// to mind.
#[test]
fn sewing_comes_of_binding_wounds_with_a_point_to_hand() {
    let mut simulation = people(1);
    simulation.population.agents[0].found_out_how_to("do:Bandage");
    give(&mut simulation, 0, "needle", 1);
    for turn in 0..crate::analytics::Simulation::WOUNDS_BOUND_BEFORE_SEWING_ONE {
        assert!(!simulation.population.agents[0].knows_how_to_do("StitchWound"));
        wounded(&mut simulation, 0);
        simulation.population.agents[0].state.wound_dressed_until = 0;
        give(&mut simulation, 0, "lashing", 1);
        simulation.current_turn = turn;
        assert!(simulation.execute_action(&Action::Bandage { who: None }, 0).success);
    }
    assert!(simulation.population.agents[0].knows_how_to_do("StitchWound"));
}

/// A blow no harder than a deer's kick breaks nothing; a broken limb halves
/// the pace, and a crooked knitting takes a fifth off it for good.
#[test]
fn a_broken_limb_halves_the_pace_and_a_crooked_one_limps() {
    let mut simulation = people(1);
    let agent = &mut simulation.population.agents[0];
    for _ in 0..200 {
        agent.state.health = 100.0;
        agent.state.take_damage(AgentState::WHAT_A_BLOW_HAS_TO_BE_TO_BREAK_A_BONE);
    }
    assert_eq!(agent.state.a_broken_limb, 0.0, "nothing that light breaks a bone");

    agent.state.a_broken_limb = 1.0;
    assert!((agent.what_my_bones_leave() - 0.5).abs() < 1e-6);
    agent.state.a_broken_limb = 0.0;
    agent.state.limps = Agent::WHAT_A_CROOKED_BONE_TAKES;
    assert!((agent.what_my_bones_leave() - 0.8).abs() < 1e-6);
}

/// A break knits in six weeks, and says how it knit.
#[test]
fn a_break_knits_in_six_weeks() {
    let mut simulation = people(1);
    let agent = &mut simulation.population.agents[0];
    agent.state.a_broken_limb = 1.0;
    let turns = (1.0 / Agent::HOW_FAST_A_BONE_KNITS).ceil() as u32;
    let days = turns / crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    assert!((41..=42).contains(&days), "{days} days");
    for _ in 0..turns + 1 {
        agent.turn_the_bone();
    }
    assert_eq!(agent.state.a_broken_limb, 0.0);
    let said = agent.state.to_be_seen.clone();
    assert_eq!(said.len(), 1, "{said:?}");
    let crooked = said[0] == AgentState::A_BONE_KNIT_CROOKED;
    assert!(crooked || said[0] == AgentState::A_BONE_KNIT_STRAIGHT);
    assert_eq!(agent.state.limps > 0.0, crooked);
}

/// Somebody who knows sets another's break with wood and a binding; nobody
/// sets their own.
#[test]
fn a_bone_is_set_with_a_splint_and_a_binding() {
    let mut simulation = people(2);
    simulation.population.agents[0].found_out_how_to("do:SetBone");
    simulation.population.agents[1].state.a_broken_limb = 1.0;
    simulation.population.agents[0].state.a_broken_limb = 1.0;
    let me = simulation.population.agents[0].id;
    let them = simulation.population.agents[1].id;
    give(&mut simulation, 0, "lashing", 2);
    give(&mut simulation, 0, "wood", 2);
    assert!(!simulation.execute_action(&Action::SetBone { who: me }, 0).success, "not one's own");

    simulation.population.agents[0].inventory.remove_item("wood", 2);
    assert!(!simulation.execute_action(&Action::SetBone { who: them }, 0).success, "no wood");
    give(&mut simulation, 0, "wood", 1);
    let busy = Action::Gather { resource_type: "wood".to_string() };
    let asked = simulation.what_a_wound_asks(0, busy, false);
    assert!(matches!(asked, Action::SetBone { who } if who == them), "got {asked:?}");
    let set = simulation.execute_action(&Action::SetBone { who: them }, 0);
    assert!(set.success, "{:?}", set.message);
    assert!(simulation.population.agents[1].state.limb_set);
    assert_eq!(simulation.population.agents[0].how_many_i_have("wood"), 0);
    assert_eq!(simulation.world.bones_set, 1);
}

/// Bones seen to knit crooked, twice, bring setting the next to mind; and a
/// broken bone is counted.
#[test]
fn setting_comes_of_seeing_bones_knit_crooked() {
    let mut simulation = people(2);
    simulation.population.agents[0].state.to_be_seen.push(AgentState::A_BONE_BROKE.to_string());
    for _ in 0..crate::analytics::Simulation::CROOKED_BONES_SEEN_BEFORE_SETTING {
        assert!(!simulation.population.agents[1].knows_how_to_do("SetBone"));
        simulation.population.agents[0].state.to_be_seen.push(AgentState::A_BONE_KNIT_CROOKED.to_string());
        simulation.what_the_camp_saw_of_bodies();
    }
    assert!(simulation.population.agents[1].knows_how_to_do("SetBone"));
    assert_eq!(simulation.world.bones_broken, 1);
    assert_eq!(simulation.world.bones_knit_crooked, 2);
}

/// The cold-hearted see to nobody but their own; the kind go across the camp
/// for anybody.
#[test]
fn how_far_somebody_goes_for_another_is_temperament() {
    let mut simulation = people(2);
    let (one, other) = (simulation.population.agents[0].clone(), simulation.population.agents[1].clone());
    let reach = |agent: &Agent| Simulation::how_far_i_would_go_for(agent, &other);
    let mut agent = one.clone();
    assert_eq!(reach(&agent), Simulation::WITHIN_A_HANDS_REACH);
    agent.traits.add_trait(Trait::ColdHearted);
    assert!(reach(&agent) < 0);
    let mut agent = one;
    agent.traits.add_trait(Trait::KindHearted);
    assert_eq!(reach(&agent), Simulation::HOW_FAR_YOU_WILL_GO_TO_A_WOUND);
    simulation.population.agents.clear();
}

/// One birth in eight goes hard; somebody who knows how, at hand, halves
/// what a hard one costs. Being at births, three times, is how helping one is
/// found out.
#[test]
fn a_birth_can_go_hard_and_help_at_it_is_found_out() {
    crate::core::dice::seed(7);
    let mut simulation = people(2);
    let mut hard = 0;
    let trials = 4000;
    for _ in 0..trials {
        simulation.population.agents[0].state.health = 100.0;
        simulation.population.agents[0].state.is_alive = true;
        let before = simulation.world.hard_births;
        simulation.how_the_birth_went(0);
        hard += (simulation.world.hard_births - before) as usize;
    }
    let share = hard as f64 / trials as f64;
    assert!((share - crate::agents::Population::A_BIRTH_GOES_HARD).abs() < 0.02, "{share}");
    let lost = simulation.world.children_lost_at_birth as f64 / hard as f64;
    // The second person was at all of them, and knew how after the third
    assert!(simulation.population.agents[1].knows_how_to_do("AttendBirth"));
    let helped = crate::agents::Population::A_HARD_BIRTH_LOSES_THE_CHILD * crate::agents::Population::WHAT_A_MIDWIFE_LEAVES;
    assert!((lost - helped).abs() < 0.06, "{lost} of hard births lost, against {helped}");
    assert!(simulation.world.births_with_a_midwife as usize >= hard - 3);
}

/// Somebody who knows how goes to a mother whose child is near and stays
/// with her; a mother with months to go is let be.
#[test]
fn a_midwife_goes_to_a_mother_whose_child_is_near() {
    let busy = Action::Gather { resource_type: "wood".to_string() };
    let mut simulation = people(2);
    simulation.population.agents[0].found_out_how_to("do:AttendBirth");
    simulation.population.agents[1].state.position = (30, 25, 0);
    let father = simulation.population.agents[0].id;
    let mut carrying = PregnancyState::new(0, father);
    carrying.due_turn = simulation.current_turn + 100 * crate::environment::seasons::TICKS_PER_DAY;
    simulation.population.agents[1].pregnancy = Some(carrying.clone());
    simulation.population.agents[0].traits.add_trait(Trait::KindHearted);
    let asked = simulation.what_a_wound_asks(0, busy.clone(), false);
    assert!(matches!(asked, Action::Gather { .. }), "months off, got {asked:?}");

    carrying.due_turn = simulation.current_turn + 10;
    simulation.population.agents[1].pregnancy = Some(carrying);
    let asked = simulation.what_a_wound_asks(0, busy.clone(), false);
    assert!(matches!(asked, Action::Move { target: (30, 25, 0) }), "got {asked:?}");
    simulation.population.agents[0].state.position = (29, 25, 0);
    let mother = simulation.population.agents[1].id;
    let asked = simulation.what_a_wound_asks(0, busy, false);
    assert!(matches!(asked, Action::AttendBirth { who } if who == mother), "got {asked:?}");
    assert!(simulation.execute_action(&asked, 0).success);
}

/// A remedy steeped in hot water draws out half as much again, by somebody
/// who knows to, with water carried and a fire lit.
#[test]
fn a_tea_draws_out_half_as_much_again() {
    let mut simulation = people(3);
    for patient in [1, 2] {
        simulation.population.agents[patient].taken_ill_with(Agent::OFF_RAW_FLESH, 0.8, 0);
    }
    simulation.population.agents[0].found_out_how_to("know:poultices_and_teas");
    give(&mut simulation, 0, "mint_leaves", 2);
    a_fire_at(&mut simulation, (25, 26));
    simulation.population.agents[2].state.position = (25, 24, 0);

    let raw = {
        // Somebody with no water carried chews it
        let them = simulation.population.agents[2].id;
        assert!(simulation.execute_action(&Action::Treat { who: Some(them) }, 0).success);
        0.8 - simulation.population.agents[2].what_ails_me().unwrap().severity
    };
    a_waterskin(&mut simulation, 0);
    let steeped = {
        let them = simulation.population.agents[1].id;
        assert!(simulation.execute_action(&Action::Treat { who: Some(them) }, 0).success);
        0.8 - simulation.population.agents[1].what_ails_me().unwrap().severity
    };
    assert!((steeped - 1.5 * raw).abs() < 1e-4, "{steeped} against {raw}");
    assert_eq!(simulation.world.teas_made, 1);
}

/// A herb bound in under a dressing halves how often the wound turns.
#[test]
fn a_poultice_under_a_dressing_halves_the_turning() {
    let mut simulation = people(1);
    simulation.population.agents[0].found_out_how_to("do:Bandage");
    simulation.population.agents[0].found_out_how_to("know:poultices_and_teas");
    wounded(&mut simulation, 0);
    give(&mut simulation, 0, "lashing", 1);
    give(&mut simulation, 0, "aloe_gel", 1);
    let mut plain = simulation.population.agents[0].clone();
    assert!(simulation.execute_action(&Action::Bandage { who: None }, 0).success);
    let agent = &simulation.population.agents[0];
    assert!(agent.state.wound_poulticed);
    assert_eq!(agent.how_many_i_have("aloe_gel"), 0);
    plain.state.wound_dressed_until = agent.state.wound_dressed_until;
    let now = simulation.current_turn;
    let poulticed = agent.how_likely_my_wound_is_to_turn(now);
    let dressed = plain.how_likely_my_wound_is_to_turn(now);
    assert!((poulticed - 0.5 * dressed).abs() < 1e-12, "{poulticed} against {dressed}");
    assert_eq!(simulation.world.poultices_laid, 1);
}

/// Dosing other people, five times, brings steeping and binding herbs in to
/// mind.
#[test]
fn teas_and_poultices_come_of_dosing_other_people() {
    let mut simulation = people(2);
    let them = simulation.population.agents[1].id;
    for _ in 0..crate::analytics::Simulation::DOSES_GIVEN_BEFORE_A_TEA {
        assert!(!simulation.population.agents[0].knows_the_way("poultices_and_teas"));
        simulation.population.agents[1].state.ailing = None;
        simulation.population.agents[1].taken_ill_with(Agent::OFF_RAW_FLESH, 0.8, 0);
        give(&mut simulation, 0, "mint_leaves", 1);
        simulation.execute_action(&Action::Treat { who: Some(them) }, 0);
    }
    assert!(simulation.population.agents[0].knows_the_way("poultices_and_teas"));
}

/// The birth roll is made where children are actually delivered. The turn's
/// population pass takes every pregnancy that is due before the simulation's
/// own birth pass runs, so a roll made only in the latter is never made at
/// all: the first measurement of #403 counted no hard birth in a hundred and
/// twenty.
#[test]
fn the_birth_roll_is_made_where_children_are_delivered() {
    crate::core::dice::seed(11);
    let mut simulation = people(2);
    let father = simulation.population.agents[1].id;
    let trials = 400;
    for _ in 0..trials {
        let mother = &mut simulation.population.agents[0];
        mother.state.health = 100.0;
        mother.state.is_alive = true;
        let now = simulation.population.current_turn;
        let mut pregnancy = crate::agents::PregnancyState::new(now, father);
        pregnancy.due_turn = now;
        mother.pregnancy = Some(pregnancy);
        simulation.population.process_pregnancies();
        simulation.population.agents.truncate(2);
    }
    simulation.count_the_hard_births();
    let hard = simulation.world.hard_births as f64;
    let expected = crate::agents::Population::A_BIRTH_GOES_HARD * trials as f64;
    assert!((hard - expected).abs() < expected * 0.5, "{hard} hard births against {expected}");
    assert_eq!(simulation.population.stats.births_gone_hard, Default::default());
}
