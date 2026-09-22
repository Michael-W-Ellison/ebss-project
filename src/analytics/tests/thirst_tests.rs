// src/analytics/tests/thirst_tests.rs
//! Regression tests for drinking, dehydration and how survival harm reaches
//! an agent's health.
//!
//! These cover the failure that left agents parched for their whole lives:
//! - thirst is acted on, not just tracked, so agents drink and keep drinking
//! - a carried waterskin can be drunk from away from open water
//! - dehydration and other survival harm actually reduce health, instead of
//!   being wiped by the body-condition sync every turn
//! - health recovers once the agent is fed, watered and unhurt

use crate::agents::{Agent, AgentConfig, AgentState, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::drives::DriveType;
use crate::world::{World, WorldConfig};

/// Agents drink over a long run instead of drinking once and never again.
///
/// Thirst used to be reachable only through the drive-based fallback at the
/// bottom of action selection, which hunger monopolised: agents went thousands
/// of turns without water with a river a dozen tiles away.
#[test]
fn agents_keep_themselves_watered() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..6 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..3000 {
        simulation.take_a_turn();
    }

    let agents = &simulation.population.agents;
    assert!(!agents.is_empty(), "population should not have died out");

    let parched = agents
        .iter()
        .filter(|a| a.state.turns_without_water > 1440)
        .count();

    assert_eq!(
        parched,
        0,
        "no agent should go a day without drinking; longest was {} turns",
        agents
            .iter()
            .map(|a| a.state.turns_without_water)
            .max()
            .unwrap_or(0)
    );

    let dehydrated = agents.iter().filter(|a| a.state.is_dehydrated()).count();
    assert_eq!(dehydrated, 0, "no agent should end the run dehydrated");
}

/// A waterskin is worth carrying: an agent away from open water drinks from it.
#[test]
fn agents_drink_from_a_carried_container() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    // Put the agent somewhere with a full waterskin and a raging thirst
    {
        let agent = &mut simulation.population.agents[0];

        let mut waterskin = InventoryItem::new_with_weight("waterskin".to_string(), 1, 0.5);
        waterskin.max_capacity = Some(2.0);
        waterskin.fill_level = Some(2.0);
        agent.inventory.add_item(waterskin);

        if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
            thirst.value = 1.0;
        }
        agent.state.last_drank_turn = 0;
    }

    // Remove every water source so only the container can help
    simulation
        .world
        .resources
        .retain(|r| r.resource_type != crate::world::ResourceType::Water);

    for _ in 0..40 {
        simulation.take_a_turn();
    }

    let agent = &simulation.population.agents[0];

    assert!(
        agent.state.turns_without_water < 40,
        "an agent with a full waterskin should have drunk from it, {} turns dry",
        agent.state.turns_without_water
    );
}

/// Dehydration has to reach the agent's health, or the drive means nothing.
///
/// Health was overwritten from body condition every turn, so starvation,
/// dehydration and exposure damage were all silently discarded: an agent could
/// go six thousand turns without water and still read as near perfect health.
#[test]
fn dehydration_damages_health() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.last_drank_turn = 0;
    agent.state.last_ate_turn = 0;

    // Well past the point where thirst starts doing harm
    let mut turn = 5000;
    let starting_health = agent.state.health;

    for _ in 0..200 {
        agent.turn_with_percepts(turn);
        agent.process_survival_turn(turn);
        turn += 1;
    }

    assert!(
        agent.state.health < starting_health,
        "prolonged dehydration should cost health, stayed at {}",
        agent.state.health
    );
}

/// Health recovers when nothing is wrong. `regenerate_health` had no callers,
/// so agents could only ever lose condition over a lifetime.
#[test]
fn health_recovers_when_fed_and_watered() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.health = 50.0;

    let mut turn = 100;
    for _ in 0..200 {
        // Keep the agent fed and watered so nothing is harming it.
        //
        // Through the **body**, not the turn counters. `age_turn_with_modifier`
        // says in its own comment that those counters "are kept only for the
        // interface and for older tests to read, and are derived rather than
        // counted so they cannot disagree with the body" - so setting them was
        // writing to a readout. The body dried out regardless, thirst took
        // health off faster than it could come back, and this test had been
        // asking whether health recovers while quietly dehydrating the man.
        agent.state.physiology.hydration = 1.0;
        agent.state.physiology.reserve = agent.state.physiology.reserve_capacity;
        agent.state.last_ate_turn = turn;
        agent.state.last_drank_turn = turn;

        agent.turn_with_percepts(turn);
        agent.process_survival_turn(turn);
        turn += 1;
    }

    assert!(
        agent.state.health > 50.0,
        "a healthy, fed, watered agent should recover, stayed at {}",
        agent.state.health
    );
}

/// Agents leave food alone once it has turned, rather than eating themselves
/// to death one bite a turn.
#[test]
fn agents_refuse_food_that_would_make_them_sick() {
    use crate::world::{FoodDatabase, ItemType};

    let mut agent = Agent::new(AgentConfig::default());

    let database = FoodDatabase::default();
    let mut rotten = InventoryItem::new_with_weight("food".to_string(), 5, 0.5);
    let mut food_data = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    food_data.freshness = 0.05; // well past turning
    rotten.food_data = Some(food_data);
    agent.inventory.add_item(rotten);

    assert!(
        agent.find_best_food_to_eat().is_none(),
        "rotten food should not be chosen as the best thing to eat"
    );
    assert!(
        !agent.has_edible_food(),
        "an agent holding only rotten food is not carrying anything edible"
    );
}

/// Mending is paid for out of what the body has spare, rather than switched
/// off by a cliff.
///
/// The gate was `is_starving() || is_dehydrated() || any active exposure`, and
/// each of those three is wider than it looks. **Any** active exposure means
/// Hypothermia, Frostbite, Hyperthermia, Dehydration or Sunburn at any
/// severity - in winter, everybody, always. And `is_starving()` is itself
/// `physiology.is_starving() || energy < 20.0`, where that second half is the
/// action-energy pool: tiredness, not starvation. So a tired man in mild cold
/// healed at exactly nought, and so did most of a settlement for most of a
/// winter - the months in which 35.8% of the dead are killed by a blow.
///
/// The share of the reserve says the same thing without the cliff and without
/// a number anybody picked. See #216.
#[test]
fn a_body_mends_out_of_what_it_has_spare() {
    let mend_from = |share: f32| {
        let mut agent = Agent::new(AgentConfig::default());
        agent.state.health = 50.0;
        agent.state.physiology.reserve = agent.state.physiology.reserve_capacity * share;
        let before = agent.state.health;
        agent.regenerate_health(false);
        agent.state.health - before
    };

    let whole = mend_from(1.0);
    let half = mend_from(0.5);
    let spent = mend_from(0.0);

    assert!(whole > 0.0, "a well-found body should mend, got {whole}");
    assert!(
        half > 0.0 && half < whole,
        "a body half through its reserve should mend, and slower: {half} against {whole}"
    );
    assert_eq!(
        spent, 0.0,
        "a body that has eaten its whole reserve has nothing to mend with"
    );
}

/// The wound cap is bookkeeping, and bookkeeping does not stop because things
/// are going badly.
///
/// `take_health_down_to` holds health down to what a broken body can carry and
/// books the difference to `A_WOUND`. It shared a gate with the healing, so
/// **suffering exempted a man from his own wound cap**: exactly while
/// starving, freezing or parched, his health was not held down to his body.
/// The whole point of #209 is that a settlement can say what killed its
/// people, and this was the one drain that went unbooked in the months they
/// actually die.
#[test]
fn a_starving_man_is_still_held_down_to_his_broken_body() {
    use crate::agents::body::BodyPartType;

    let mut agent = Agent::new(AgentConfig::default());

    // Wreck the body, and badly: a turn of starvation takes health off by
    // itself, so the gap between what he has and what his body can carry has
    // to be wider than that, or the cap has nothing left to do and the test
    // proves nothing either way.
    for part in [
        BodyPartType::Torso,
        BodyPartType::LeftArm,
        BodyPartType::RightArm,
        BodyPartType::LeftLeg,
        BodyPartType::RightLeg,
    ] {
        agent.body.damage_part(part, 60.0);
    }
    let body_condition = agent.body.overall_health() * 100.0;
    agent.state.health = 100.0;
    assert!(
        body_condition < 80.0,
        "the fixture meant to break him badly, got {body_condition}"
    );

    // And make him "suffering" in the sense the old gate meant, without
    // actually harming him - so that what the cap does is the only thing
    // moving. A body with an empty reserve dies of hunger inside one turn and
    // takes the whole hundred with it, which tells you nothing about the cap.
    //
    // `is_starving()` is `physiology.is_starving() || energy < 20.0`, and that
    // second half is the action-energy pool - tiredness, not starvation. So a
    // well-fed, well-watered, tired man reads as suffering to the old gate and
    // loses no health to anything. That is the over-reach and the instrument
    // for measuring it at once.
    agent.state.physiology.reserve = agent.state.physiology.reserve_capacity;
    agent.state.physiology.hydration = 1.0;
    agent.state.energy = 5.0;
    assert!(
        agent.state.is_starving(),
        "the fixture meant him to read as starving on the tiredness clause"
    );
    assert!(
        !agent.state.physiology.is_starving(),
        "and to not actually be starving"
    );

    agent.process_survival_turn(100);

    // Asked of the **ledger**, not of the health figure. A starving body
    // loses health to hunger every turn anyway, so "his health came down"
    // cannot tell the cap from the starvation - the first draft of this test
    // passed against the old gate for exactly that reason. What only the cap
    // does is book the drop to `A_WOUND`, which is the whole point of #209.
    // Nothing else here books it: `turn_the_wound` returns at once unless
    // there is an open wound, and this man has none.
    let booked_to_wounds: f32 = agent
        .state
        .what_has_taken_health
        .iter()
        .filter(|(what, _)| what == AgentState::A_WOUND)
        .map(|(_, how_much)| *how_much)
        .sum();

    assert!(
        booked_to_wounds > 0.0,
        "a starving man was exempted from his own wound cap: health {} on a \
         body that can carry {body_condition}, and nothing booked to wounds",
        agent.state.health
    );
}
