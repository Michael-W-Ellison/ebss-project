// src/analytics/tests/cadence_tests.rs
//! Cadence: how often a thing is worth doing, found out rather than written
//! down.
//!
//! See [`crate::agents::rhythm`] for what is being climbed and why.

use crate::agents::practices::Undertaking;
use crate::agents::rhythm::Rhythm;
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation
}

/// Walk a rhythm through a world whose yield is a known function of how long
/// you leave it, and report where it settles.
///
/// `yield_at` stands in for the world: what one doing brings back after this
/// many ticks of waiting.
fn where_it_settles(yield_at: impl Fn(u32) -> f32, doings: u32) -> f32 {
    let mut rhythm = Rhythm::unfound();
    let mut now = 0u32;

    for _ in 0..doings {
        now += rhythm.every();
        let got = yield_at(rhythm.every());
        rhythm.how_it_went(now, got);
    }

    rhythm.every() as f32 / TICKS_PER_DAY as f32
}

/// A rhythm starts somewhere deliberately wrong.
///
/// An agent that started at the answer would not be discovering anything, and
/// nothing measured about the search would mean much.
#[test]
fn a_rhythm_starts_at_a_guess_and_not_at_the_answer() {
    let fresh = Rhythm::unfound();
    assert_eq!(fresh.every(), Rhythm::WHAT_A_BODY_GUESSES_FIRST);
    assert!(fresh.every() > Rhythm::THE_SHORTEST_ANYBODY_LEAVES_IT);
    assert!(fresh.every() < Rhythm::THE_LONGEST_ANYBODY_LEAVES_IT);
    assert!(fresh.is_it_due(0), "a thing never done is always due");
    assert!(fresh.still_finding_it());
}

/// A thing that pays the same however long you leave it settles short.
///
/// This is the prejudice the margin buys and it is the right one: food in hand
/// today beats the same food in hand on Thursday, so where waiting gains
/// nothing the agent stops waiting.
#[test]
fn where_waiting_gains_nothing_he_stops_waiting() {
    let settled = where_it_settles(|_| 1.0, 200);

    assert!(
        settled <= 1.5,
        "a thing that pays the same whenever you go should be done often: \
         settled at {settled:.2} days"
    );
}

/// A thing that keeps paying more the longer you leave it settles long.
#[test]
fn where_waiting_keeps_paying_he_waits() {
    // Yield straight in proportion to the wait: leaving it twice as long
    // brings back twice as much, for ever.
    let settled = where_it_settles(|every| every as f32 / TICKS_PER_DAY as f32, 200);

    assert!(
        settled >= 5.0,
        "a thing that pays in proportion to the wait should be left: settled \
         at {settled:.2} days"
    );
}

/// And a thing that pays more up to a point and no further settles at the
/// point.
///
/// This is the trapline's shape and the whole reason the type exists: a catch
/// left too long has been eaten by something else, so the wait buys nothing
/// past the span the country takes to rob a snare. Nobody writes that span
/// down here - the agent finds it.
#[test]
fn he_settles_at_the_knee_of_the_curve() {
    // Saturating at about two days, which stands in for a robbing rate.
    let two_days = 2.0 * TICKS_PER_DAY as f32;
    let settled = where_it_settles(
        move |every| 1.0 - (-(every as f32) / two_days).exp(),
        300,
    );

    assert!(
        settled > 1.0 && settled < 5.0,
        "he should settle near where the curve stops rising, not at either \
         bound: {settled:.2} days"
    );
}

/// A rhythm found out on one world is found out differently on another.
///
/// The point of climbing the world rather than reading a constant: change what
/// the country does and the settled cadence follows it.
#[test]
fn the_cadence_follows_the_world_and_not_a_constant() {
    let quick = where_it_settles(
        move |every| 1.0 - (-(every as f32) / (1.0 * TICKS_PER_DAY as f32)).exp(),
        300,
    );
    let slow = where_it_settles(
        move |every| 1.0 - (-(every as f32) / (5.0 * TICKS_PER_DAY as f32)).exp(),
        300,
    );

    assert!(
        slow > quick,
        "a country that is slow to take the catch should be walked less often \
         than one that is quick: {slow:.2} days against {quick:.2}"
    );
}

/// It never goes outside what a person would actually do.
#[test]
fn a_rhythm_stays_inside_what_a_person_would_do() {
    for pull in [0.0f32, 1.0, 100.0] {
        let mut rhythm = Rhythm::unfound();
        let mut now = 0;
        for _ in 0..300 {
            now += rhythm.every();
            rhythm.how_it_went(now, pull * rhythm.every() as f32);
            assert!(
                rhythm.every() >= Rhythm::THE_SHORTEST_ANYBODY_LEAVES_IT
                    && rhythm.every() <= Rhythm::THE_LONGEST_ANYBODY_LEAVES_IT,
                "a rhythm ran off the end at {}",
                rhythm.every()
            );
        }
    }
}

/// The search reports where it has got to, and `Lessons` is not told to
/// ignore it.
///
/// Sheltering a searching rhythm from the coarse book is the obvious thing to
/// do and it was measured and refused - see `Rhythm::still_finding_it`, which
/// carries the numbers. It is left readable because it is what the search
/// knows about itself, and unread because taking the brake off cost 4,200
/// person-days.
#[test]
fn a_searching_rhythm_says_so_and_is_still_judged() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];
    let nowhere: Vec<crate::agents::practices::Circumstance> = Vec::new();

    for _ in 0..6 {
        agent.learn_from_this_here(&Action::CheckSnares, false, &nowhere);
    }

    assert!(
        agent.how_often_i(Undertaking::Trapping).still_finding_it(),
        "six rounds is not enough to have settled a rhythm"
    );
    assert_eq!(
        agent.lessons.attempts(Undertaking::Trapping),
        6,
        "and every one of them is still evidence about trapping, because the \
         coarse book is the only thing that stops a settlement walking an \
         empty line all winter"
    );
}

/// A rhythm that has moved enough times says it has settled.
#[test]
fn a_rhythm_that_has_moved_enough_says_it_has_settled() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    let mut now = 0;
    for _ in 0..(Rhythm::ENOUGH_TO_TELL * (Rhythm::SETTLED_AFTER + 2)) {
        now += 1;
        agent.that_is_done(Undertaking::Trapping, now, 1.0);
    }

    let rhythm = agent.how_often_i(Undertaking::Trapping);
    assert!(!rhythm.still_finding_it());
    assert!(rhythm.changes() >= Rhythm::SETTLED_AFTER);
}

/// And the whole of it through the live path: a man with a line walks it, on
/// a rhythm, without being hungry enough to be desperate.
#[test]
fn a_man_with_a_line_walks_it() {
    crate::core::dice::seed(3);
    let mut simulation = one_person();
    let me = simulation.population.agents[0].id;

    for step in 0..6 {
        simulation.world.snares.push(crate::environment::small_life::Snare {
            at: (26 + step, 25),
            set_by: me,
            set_at: 0,
            caught_at: None,
        });
    }

    let agent = &simulation.population.agents[0];
    let doing = simulation.going_round_is_due(agent, agent.state.position);

    assert!(
        doing.is_some(),
        "a man who has never walked his line is due to walk it"
    );

    // And having walked it, he is not due again until the rhythm says so.
    let mut agent = simulation.population.agents[0].clone();
    agent.that_is_done(Undertaking::Trapping, simulation.current_tick, 0.0);
    assert!(
        simulation
            .going_round_is_due(&agent, agent.state.position)
            .is_none(),
        "and he does not walk it twice in the same half hour"
    );
    assert!(
        agent.how_often_i(Undertaking::Trapping).every() >= TICKS_PER_DAY,
        "nobody walks a line more than once a day"
    );
}
