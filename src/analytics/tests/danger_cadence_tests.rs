// src/analytics/tests/danger_cadence_tests.rs
//! Tests that somebody with something on them decides minute by minute.
//!
//! "Every 30 turns/minutes agents should have the option of making a decision.
//! This does not apply if an agent encounters a dangerous situation, as they
//! must then make decisions minute by minute to enhance their survival odds."
//!
//! Note on the fixtures: it is not enough to write a fright into somebody's
//! emotions and turn. Fear and anger are re-appraised from what is actually
//! there every turn - see ISSUES #260 - so a wolf that does not exist is
//! forgotten before anybody acts on it, which is right. A test about being in
//! danger has to put something in the world to be in danger of.

use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::MINUTES_PER_TURN;
use crate::world::{World, WorldConfig};

fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

fn one_person_alone() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(an_empty_country(), population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation
}

/// A man already hurt, with a wolf at his elbow.
///
/// The hurt is not decoration. A sound adult sizes up one wolf and comes out
/// angry rather than frightened - measured at 0.21 against a gate of 0.43 -
/// so he is in no danger by the model's reckoning, and he is right: he can
/// face it. It takes four wolves to anger him past the gate, and that sits
/// within a hundredth of it, which is no place to rest a test. A man at forty
/// health reads the same wolf as more than he can cope with and comes out at
/// the fear ceiling of 0.700, in every run, at every count of wolves. That is
/// the appraisal doing exactly what it is for, and it is also the case the
/// minute clock exists to serve: the one who cannot simply turn and deal
/// with the thing.
fn one_person_and_a_wolf() -> Simulation {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (30, 31))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = HURT_ENOUGH_TO_BE_AFRAID;
    simulation
}

/// What a man has left when a wolf is more than he can manage.
const HURT_ENOUGH_TO_BE_AFRAID: f32 = 40.0;

/// An ordinary turn is one decision. Nobody gets the fast clock for nothing.
#[test]
fn an_untroubled_turn_is_one_decision() {
    let mut simulation = one_person_alone();

    for _ in 0..6 {
        simulation.take_a_turn();
    }

    assert_eq!(
        simulation.minutes_spent_in_danger, 0,
        "nothing was on anybody, so nobody needed the minutes"
    );
}

/// Something on somebody puts them on the minute clock.
#[test]
fn a_man_with_a_wolf_on_him_is_asked_more_than_once() {
    let mut simulation = one_person_and_a_wolf();

    for _ in 0..8 {
        simulation.take_a_turn();
    }

    assert!(
        simulation.minutes_spent_in_danger > 0,
        "eight turns beside a wolf and nobody ever had to think twice"
    );
}

/// And the danger has to be the reason, not the man merely being hurt.
///
/// The same wolf, the same hurt man, the same eight turns - but the wolf put
/// across the valley instead of at his elbow. Nothing is on him, so nothing
/// changes about his turn: the fast clock is about what is there, not about
/// the state he is in.
#[test]
fn a_wolf_across_the_country_is_not_a_danger() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (5, 5))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = HURT_ENOUGH_TO_BE_AFRAID;

    for _ in 0..8 {
        simulation.take_a_turn();
    }

    assert_eq!(
        simulation.minutes_spent_in_danger, 0,
        "the wolf was a day's walk away and the man spent the half hour on it"
    );
}

/// The half hour is a ceiling: nobody gets more minutes than the turn holds.
///
/// The minutes taken one at a time come out of the half hour, not on top of
/// it: at most every minute of it.
#[test]
fn nobody_gets_more_minutes_than_the_turn_holds() {
    let mut simulation = one_person_and_a_wolf();

    let turns = 8;
    for _ in 0..turns {
        simulation.take_a_turn();
    }

    let ceiling = turns * MINUTES_PER_TURN as u64;
    assert!(
        simulation.minutes_spent_in_danger <= ceiling,
        "{turns} turns cannot hold more than {ceiling} extra minutes, got {}",
        simulation.minutes_spent_in_danger
    );
}

/// And a settlement with nothing after it never pays for the mechanism.
#[test]
fn the_fast_clock_costs_nothing_where_there_is_no_danger() {
    let mut quiet = one_person_alone();
    for _ in 0..12 {
        quiet.take_a_turn();
    }

    assert_eq!(
        quiet.minutes_spent_in_danger, 0,
        "the minute clock is for danger and there was none"
    );
}

/// A small child takes no minutes of its own.
///
/// It takes no turn either: it is carried, and goes where its people go. The
/// minute clock asked it every minute what it would do about the beast, it
/// did nothing, and the beast had a strike at it every one of those minutes.
/// An infant left on the spot where its people had stood was struck four
/// times in a half hour and died of it (#325).
#[test]
fn a_small_child_is_not_given_the_minutes() {
    let mut simulation = one_person_and_a_wolf();
    let child = &mut simulation.population.agents[0];
    child.state.age = 0;
    child.state.life_stage = LifeStage::Infant;

    let mut ever_afraid = false;
    for _ in 0..8 {
        simulation.take_a_turn();
        ever_afraid |= simulation.population.agents[0].emotions.in_danger();
    }

    assert!(ever_afraid, "the child is meant to be frightened of the wolf");
    assert_eq!(
        simulation.minutes_spent_in_danger, 0,
        "a child who cannot act was given minutes to act in"
    );
}

/// Everybody decides once a half hour, and not all at the same minute.
///
/// "Agents should have their 30 minute decision ticks separated out to
/// prevent lag spikes if every agent was forced to make decisions at the
/// exact same time."
#[test]
fn everybody_keeps_their_own_half_hour() {
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(an_empty_country(), population);
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (30, 30, 0);
        agent.state.life_stage = LifeStage::Adult;
    }

    simulation.take_a_turn();

    let began = simulation.current_turn;
    let mut minutes = std::collections::BTreeSet::new();
    for agent in &simulation.population.agents {
        let mine = Simulation::my_minute_of_the_half_hour(agent.id);
        minutes.insert(mine);
        assert_eq!(
            agent.state.next_decision_at,
            began + mine + MINUTES_PER_TURN,
            "decided once, at their own minute, and next in half an hour"
        );
    }
    assert!(minutes.len() > 1, "twelve people all deciding at the same minute");
}

/// Hunger and thirst are answered on the half hour, with every other need.
///
/// The fear of going without used to count as danger, so somebody parched
/// took the half hour a minute at a time, and a beast beside them had a
/// strike at them every minute (#325).
#[test]
fn going_without_is_answered_on_the_half_hour() {
    let mut simulation = one_person_alone();
    let agent = &mut simulation.population.agents[0];
    for need in [crate::core::DriveType::Thirst, crate::core::DriveType::Hunger] {
        if let Some(drive) = agent.drives.get_mut(need) {
            drive.value = 1.0;
        }
    }

    for _ in 0..6 {
        simulation.take_a_turn();
    }

    assert_eq!(
        simulation.minutes_spent_in_danger, 0,
        "nothing was after anybody, however thirsty they were"
    );
}

fn a_starving_wolf(simulation: &mut Simulation) {
    for animal in simulation.world.animals.get_all_mut() {
        animal.hunger = animal.max_hunger;
    }
}

/// Struck in their sleep, a sleeper wakes and answers it.
///
/// "Agents should not be peacefully sleeping while being attacked."
#[test]
fn a_sleeper_who_is_struck_wakes_and_decides_now() {
    let mut simulation = one_person_and_a_wolf();
    a_starving_wolf(&mut simulation);
    simulation.current_turn = 600;
    let agent = &mut simulation.population.agents[0];
    agent.state.health = 100.0;
    agent.fatigue.is_sleeping = true;
    agent.busy_until = 10_000;
    agent.state.next_decision_at = 10_000;

    let mut struck = false;
    for _ in 0..500 {
        simulation.predators_try_their_luck_at(0);
        if simulation.population.agents[0].state.health < 100.0 {
            struck = true;
            break;
        }
    }

    assert!(struck, "the wolf is meant to strike");
    let agent = &simulation.population.agents[0];
    assert_eq!(agent.busy_until, 0, "still asleep under the wolf");
    assert!(agent.state.next_decision_at <= simulation.current_turn, "and left to decide later");
}

/// A beast that is about and does no harm comes to frighten less, and a blow
/// from one makes it a stranger again.
///
/// "An unknown animal or situation might cause fear at first only to
/// eventually be ignored as an agent adjusts to it."
#[test]
fn a_beast_that_does_no_harm_comes_to_frighten_less() {
    let mut simulation = one_person_and_a_wolf();

    simulation.feel_about_what_stands_in_the_way();
    let at_first = simulation.population.agents[0].emotions.fear;
    for _ in 0..200 {
        simulation.feel_about_what_stands_in_the_way();
    }
    let after_a_while = simulation.population.agents[0].emotions.fear;
    assert!(at_first > 0.6, "a stranger at the elbow is meant to frighten: {at_first}");
    assert!(
        after_a_while < at_first / 2.0,
        "four days beside it and no harm done, and still {after_a_while} against {at_first}"
    );

    a_starving_wolf(&mut simulation);
    let before = simulation.population.agents[0].state.health;
    for _ in 0..500 {
        simulation.predators_try_their_luck_at(0);
        if simulation.population.agents[0].state.health < before {
            break;
        }
    }
    assert!(
        simulation.population.agents[0].familiar_with.is_empty(),
        "bitten, and still used to it: {:?}",
        simulation.population.agents[0].familiar_with
    );
}

/// A beast that would kill sooner than thirst is answered before thirst.
///
/// "Primary Drive demand should be determined by whichever drive results in
/// the largest threat to the continued survival of the agent."
#[test]
fn what_kills_first_is_answered_first() {
    let mut simulation = one_person_and_a_wolf();
    simulation.feel_about_what_stands_in_the_way();
    if let Some(thirst) = simulation.population.agents[0].drives.get_mut(crate::core::DriveType::Thirst) {
        thirst.value = 1.0;
    }

    let the_wolf = simulation
        .minutes_before_what_is_after_me_kills_me(0)
        .expect("the wolf is after him");
    let the_thirst = Simulation::minutes_before_want_kills_me(&simulation.population.agents[0]);
    assert!(the_wolf < the_thirst, "the wolf in {the_wolf} minutes, thirst in {the_thirst}");
}
