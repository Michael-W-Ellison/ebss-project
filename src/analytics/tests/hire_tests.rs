// src/analytics/tests/hire_tests.rs
//! Carrying a kill home for a share (ISSUES_FOUND #374, after #331).
//!
//! - a kill too big to get home in time takes on whoever is in earshot
//! - a kill its owner can carry home alone is never shared out
//! - the carrier cuts it, carries it to the owner's store, puts in all but
//!   their share, and the two think better of each other for it
//! - a hungry man, or one with no room in the pack, is not taken on
//! - a kill being carried for its killer is not food to whoever happens
//!   past it; one nobody is carrying is anybody's, as it always was
//! - a kill being carried for a share is not given away by telling

use crate::agents::hire::Hire;
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::Action;
use crate::world::{Belongs, Pit, Position, World, WorldConfig};

const CAMP: (i32, i32) = (30, 30);
const KILL: Position = Position { x: 36, y: 30 };
const STORE: Position = Position { x: 31, y: 30 };

/// A hunter and a neighbour at camp, a pit of the hunter's beside them, a
/// kill of his lying a short walk off with `joints` on it and `days_left`
/// before it goes, and nothing else on the map to eat.
fn a_kill_and_a_neighbour(joints: u32, days_left: f32) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    world.resources.clear();
    let mut population = Population::new();
    let mut config = AgentConfig::default();
    config.random_weights = false;
    population.spawn_agent(config.clone());
    population.spawn_agent(config);
    let mut simulation = Simulation::new(world, population);
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    for agent in &mut simulation.population.agents {
        agent.traits = crate::core::traits::TraitSet::new();
        agent.state.position = (CAMP.0, CAMP.1, 0);
        agent.inventory.get_all_items_mut().clear();
        agent.inventory.max_weight = 12.0;
    }

    let hunter = simulation.population.agents[0].id;
    simulation.world.pits.push(Pit {
        where_it_is: STORE,
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: Belongs::To(hunter),
    });

    let lies = World::HOW_LONG_A_CARCASS_LIES;
    let now = lies - (days_left * TICKS_PER_DAY as f32) as u32;
    simulation.current_turn = now;
    let carcass = simulation
        .world
        .a_carcass_lies_here(KILL, joints, 0)
        .expect("a carcass with meat on it");
    simulation.world.resources[carcass].killed_by = Some(hunter);
    simulation
}

fn meat_in_the_store(simulation: &Simulation) -> u32 {
    simulation
        .world
        .pit_at(STORE)
        .map(|pit| pit.holds.iter().filter(|item| item.item_id == "meat").map(|item| item.quantity).sum())
        .unwrap_or(0)
}

/// A bear's worth of meat with a day left on it: the hunter brings home one
/// load alone, so a neighbour's load, less a third of it, is more for him.
#[test]
fn a_kill_too_big_to_get_home_in_time_takes_somebody_on() {
    let mut simulation = a_kill_and_a_neighbour(40, 1.0);
    simulation.hands_for_the_carrying();

    let hunter = simulation.population.agents[0].id;
    let hire = simulation.population.agents[1]
        .working_for
        .clone()
        .expect("the neighbour should have been asked, and agreed");
    assert_eq!(hire.by, hunter);
    assert_eq!(hire.from_and_to(), ((KILL.x, KILL.y), (STORE.x, STORE.y)));
    assert!((hire.share - Hire::A_CARRIERS_SHARE).abs() < 1e-6);
    assert!(simulation.population.agents[0].working_for.is_none(), "nobody works for himself");
    assert_eq!(simulation.world.carriers_taken_on, 1);
    assert!(
        simulation.population.agents[1].exploration_knowledge.known_resources.contains_key(&KILL),
        "whoever is taken on is told where the kill is"
    );
}

/// A deer he can carry home himself in the days it has left: a carrier would
/// only take a share of what was coming home anyway.
#[test]
fn a_kill_its_owner_can_carry_home_alone_is_never_shared_out() {
    let mut simulation = a_kill_and_a_neighbour(10, 4.0);
    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_none());
    assert_eq!(simulation.world.carriers_taken_on, 0);
}

/// The carrier walks out, cuts a load, walks it to the hunter's pit, puts in
/// all of it but the share, and goes back while there is meat and time; the
/// job seen through, the two of them think better of each other.
#[test]
fn the_carrier_carries_it_home_and_keeps_a_share() {
    let mut simulation = a_kill_and_a_neighbour(40, 1.0);
    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_some());
    let hunter = simulation.population.agents[0].id;
    let carrier = simulation.population.agents[1].id;

    let mut covered = 0;
    for _ in 0..60 {
        let agent = simulation.population.agents[1].clone();
        let Some(step) = simulation.the_carrying_i_took_on(&agent, agent.state.position) else {
            break;
        };
        match step {
            Action::Move { target } => simulation.population.agents[1].state.position = target,
            other => {
                if matches!(other, Action::Cover { .. }) {
                    covered += 1;
                }
                simulation.population.agents[1].busy_until = 0;
                let result = simulation.execute_action(&other, 1);
                assert!(result.success, "{other:?} was refused: {result:?}");
            }
        }
        if covered > 0 {
            break;
        }
    }

    let hire = simulation.population.agents[1].working_for.clone().expect("still at it");
    let stored = meat_in_the_store(&simulation);
    let kept = simulation.population.agents[1].how_many_i_have("meat");
    assert!(hire.taken >= 4, "a pack's load cut off the carcass: {}", hire.taken);
    assert_eq!(stored, hire.handed_over);
    assert_eq!(stored + kept, hire.taken, "nothing lost between the carcass and the pit");
    assert_eq!(stored, (hire.taken as f32 * (1.0 - hire.share)).round() as u32);
    assert!(kept > 0, "the carrier keeps a share");
    assert_eq!(simulation.world.joints_carried_for_a_share, u64::from(stored));

    // The kill gone, nothing more is owed and the job is let go, paid
    simulation.world.resources.retain(|node| !node.is_a_carcass());
    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_none());
    assert_eq!(simulation.world.joints_kept_as_pay, u64::from(kept));
    let fond = |of: usize, whom| {
        simulation.population.agents[of]
            .relationships
            .get_relationship(&whom)
            .map(|bond| bond.bond_strength)
            .unwrap_or(0.0)
    };
    assert!(fond(0, carrier) > 0.0, "the hunter thinks better of the carrier");
    assert!(fond(1, hunter) > 0.0, "and the carrier of the hunter");
}

/// The same kill, and a hungry neighbour: he goes for his own supper, and is
/// not taken on to carry somebody else's.
#[test]
fn a_hungry_man_is_not_taken_on() {
    let mut simulation = a_kill_and_a_neighbour(40, 1.0);
    if let Some(hunger) = simulation.population.agents[1].drives.get_mut(crate::core::DriveType::Hunger) {
        hunger.value = 0.95;
    }
    assert!(simulation.population.agents[1]
        .drives
        .get(crate::core::DriveType::Hunger)
        .is_some_and(|hunger| hunger.is_active()));
    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_none());
}

/// Nor somebody with no room in the pack for a joint: there is nothing he
/// could carry, so nothing to pay him for.
#[test]
fn nobody_with_a_full_pack_is_taken_on() {
    let mut simulation = a_kill_and_a_neighbour(40, 1.0);
    simulation.population.agents[1].inventory.max_weight = 1.0;
    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_none());
}

/// A kill nobody is carrying for its killer is anybody's, as it always was,
/// so its leftovers feed whoever comes past rather than the crows. Once he
/// has taken people on to carry it, it is his to give: a passer-by may not
/// cut from it, and whoever he told of it may.
#[test]
fn a_kill_being_carried_is_its_killers_to_give() {
    let here = Position::new(CAMP.0, CAMP.1);
    let sees_it = |simulation: &Simulation, who: usize| {
        let them = simulation.population.agents[who].clone();
        simulation
            .nodes_this_one_knows_of(&them, here, 25)
            .iter()
            .any(|&n| simulation.world.resources[n].is_a_carcass())
    };

    let mut simulation = a_kill_and_a_neighbour(40, 1.0);
    let mut config = AgentConfig::default();
    config.random_weights = false;
    simulation.population.spawn_agent(config);
    simulation.population.agents[2].state.position = (CAMP.0 + 40, CAMP.1, 0);
    simulation.population.agents[2].inventory.max_weight = 0.0;
    assert!(sees_it(&simulation, 1), "nobody carrying it, it is anybody's");

    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_some());
    simulation.population.agents[2].state.position = (CAMP.0, CAMP.1, 0);
    assert!(!sees_it(&simulation, 2), "being carried for him, a passer-by may not cut from it");
    assert!(sees_it(&simulation, 1), "the carrier may");

    let hunter = simulation.population.agents[0].id;
    let now = simulation.current_turn;
    let knows = &mut simulation.population.agents[2].exploration_knowledge;
    knows.discover_resource(KILL, crate::world::ResourceType::Meat, now);
    knows.who_told_me.insert(
        KILL,
        crate::agents::exploration::Hearsay {
            who: hunter,
            they_saw_it_on: now,
            told_me_on: now,
            how_much_they_said: Some(40),
        },
    );
    assert!(sees_it(&simulation, 2), "told where it is, it is his to take");
}

/// A kill with people carrying it home for a share is not told about: the
/// meat on it is promised.
#[test]
fn a_kill_being_carried_for_a_share_is_not_given_away() {
    let mut simulation = a_kill_and_a_neighbour(40, 1.0);
    let now = simulation.current_turn;
    {
        let knows = &mut simulation.population.agents[0].exploration_knowledge;
        knows.discover_resource(KILL, crate::world::ResourceType::Meat, now);
        knows.saw_it_again(KILL, 40, now);
    }
    simulation.hands_for_the_carrying();
    assert!(simulation.population.agents[1].working_for.is_some());

    // A third person comes up and the hunter has his say
    let mut config = AgentConfig::default();
    config.random_weights = false;
    simulation.population.spawn_agent(config);
    simulation.population.agents[2].state.position = (CAMP.0, CAMP.1, 0);
    simulation.population.current_turn = now;
    simulation.population.say_it_out_loud();
    assert!(
        !simulation.population.agents[2].exploration_knowledge.known_resources.contains_key(&KILL),
        "the kill should not have been told about"
    );
}
