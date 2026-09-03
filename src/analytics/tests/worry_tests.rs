// src/analytics/tests/worry_tests.rs
//! Tests that an agent prices what its habits may cost it, and that the price
//! comes off when nothing happens.
//!
//! "An agent might avoid stealing to prevent the loss of future socialization
//! drive demand if the theft is discovered. If an agent gets hungry enough to
//! steal food and does not experience a loss to future socialization drive
//! demand satisfaction, then the 'worry' decreases and the pattern of using
//! theft to satisfy their hunger drive is strengthened."

use crate::agents::patterns::{how_long_a_worry_lasts, Element, Patterns};
use crate::agents::{Agent, AgentConfig, Population};
use crate::core::DriveType;
use crate::environment::seasons::TICKS_PER_DAY;

fn a_lone_agent() -> Population {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population
}

/// Taking food answers a need, and being seen doing it costs something.
#[test]
fn being_seen_taking_is_what_costs_the_thief_rather_than_the_taking() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];

    let taking = [
        Element::Did("takefrom".to_string()),
        Element::At((5, 5, 0)),
    ];

    // He takes, and it answers his need
    agent.patterns.it_worked(DriveType::Utility, &taking, 0.5, 10);
    let unbothered = agent.patterns.what_i_dread(DriveType::Utility, &taking);
    assert_eq!(unbothered, 0.0, "nothing has happened to him yet");

    // Nobody saw. Nothing follows. He is no warier than he was
    agent.patterns.fade(10 + TICKS_PER_DAY);
    assert_eq!(
        agent.patterns.what_i_dread(DriveType::Utility, &taking),
        0.0,
        "a theft nobody saw teaches nothing"
    );

    // He takes again, and this time the camp is standing there
    agent.patterns.it_worked(DriveType::Utility, &taking, 0.5, 20);
    agent.this_cost_me(DriveType::Social, 0.3, 20);

    let now_wary = agent.patterns.what_i_dread(DriveType::Utility, &taking);
    assert!(
        now_wary > 0.0,
        "being seen should have taught him something: {now_wary}"
    );
}

/// The user's case, run forward: impunity strengthens the habit.
#[test]
fn a_thief_who_is_never_caught_again_gets_bolder() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];

    let taking = [Element::Did("takefrom".to_string())];

    // Caught once, early
    agent.patterns.it_worked(DriveType::Utility, &taking, 0.5, 0);
    agent.this_cost_me(DriveType::Social, 0.4, 0);

    let worth_when_wary = agent
        .patterns
        .trail(DriveType::Utility, &taking[0])
        .expect("the habit is written down")
        .worth();

    // Then a month of taking, and nobody ever says anything
    let mut now = 0;
    for _ in 0..30 {
        now += TICKS_PER_DAY;
        agent.patterns.it_worked(DriveType::Utility, &taking, 0.5, now);
        agent.patterns.fade(now);
    }

    let trail = agent
        .patterns
        .trail(DriveType::Utility, &taking[0])
        .expect("still written down");

    assert!(
        trail.worth() > worth_when_wary,
        "a month of getting away with it should leave him bolder than the day \
         he was caught: {} against {worth_when_wary}",
        trail.worth()
    );
    assert!(
        trail.threat_to(DriveType::Social) < 0.4,
        "and the worry itself should have come off: {}",
        trail.threat_to(DriveType::Social)
    );
}

/// How long a worry lasts depends on how much the thing it guards matters.
///
/// "This should vary from a day to a month."
#[test]
fn a_worry_about_something_that_kills_you_outlasts_a_worry_about_a_slight() {
    use crate::environment::seasons::DAYS_PER_MONTH;

    let about_starving = how_long_a_worry_lasts(DriveType::Hunger);
    let about_standing = how_long_a_worry_lasts(DriveType::Social);
    let about_a_fine_coat = how_long_a_worry_lasts(DriveType::Utility);

    assert!(
        about_starving > about_standing,
        "hunger kills and standing does not: {about_starving} against {about_standing}"
    );
    assert!(
        about_standing > about_a_fine_coat,
        "{about_standing} against {about_a_fine_coat}"
    );

    // And the whole range sits between a day and a month, as specified
    for lasts in [about_starving, about_standing, about_a_fine_coat] {
        assert!(
            (1.0..=DAYS_PER_MONTH as f32).contains(&lasts),
            "a worry lasting {lasts} days is outside a day to a month"
        );
    }
}

/// A newborn has no history, so its wariness has to come from somewhere else.
#[test]
fn a_child_is_born_wary_of_what_its_parents_were_wary_of() {
    let mut careful = Agent::new(AgentConfig { random_weights: false });
    careful.patterns.taught_to_dread(
        DriveType::Utility,
        Element::Did("takefrom".to_string()),
        DriveType::Social,
        0.6,
    );

    let other = Agent::new(AgentConfig { random_weights: false });
    let mut child = Agent::new(AgentConfig { random_weights: false });

    assert_eq!(
        child
            .patterns
            .what_i_dread(DriveType::Utility, &[Element::Did("takefrom".to_string())]),
        0.0,
        "a child starts with nothing"
    );

    child
        .patterns
        .what_the_child_takes_from(&[&careful.patterns, &other.patterns]);

    let born_wary = child
        .patterns
        .what_i_dread(DriveType::Utility, &[Element::Did("takefrom".to_string())]);

    assert!(born_wary > 0.0, "it should have taken something: {born_wary}");
    assert!(
        born_wary < 0.6,
        "but not all of it - a fear has to be re-earned to stay sharp: {born_wary}"
    );

    // And it takes the worry without taking the map
    assert_eq!(
        child.patterns.where_it_worked(DriveType::Hunger, 0),
        None,
        "a child does not inherit its parents' bushes"
    );
}

/// Worry has to make somebody act, not merely refuse.
#[test]
fn worry_presses_on_the_drive_it_fears_for() {
    let mut agent = Agent::new(AgentConfig { random_weights: false });

    assert_eq!(agent.what_worry_adds_to(DriveType::Social), 0.0);

    agent.patterns.taught_to_dread(
        DriveType::Utility,
        Element::Did("takefrom".to_string()),
        DriveType::Social,
        0.2,
    );

    let pressing = agent.what_worry_adds_to(DriveType::Social);
    assert!(
        pressing > 0.0,
        "a man who expects his standing to suffer should attend to his standing"
    );
    assert!(
        pressing <= Agent::THE_MOST_WORRY_CAN_ADD,
        "but being worried is not the same as being friendless: {pressing}"
    );

    // And it presses on the drive that is at risk, not the one that earned it
    assert_eq!(
        agent.what_worry_adds_to(DriveType::Hunger),
        0.0,
        "nothing has threatened his supper"
    );
}

/// The felt total is a read-out of the pattern layer, not a second copy.
#[test]
fn what_somebody_feels_is_what_the_habits_say_they_should() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];

    assert_eq!(agent.emotions.worry, 0.0);

    agent.patterns.it_worked(
        DriveType::Utility,
        &[Element::Did("takefrom".to_string())],
        0.5,
        10,
    );
    agent.this_cost_me(DriveType::Social, 0.3, 10);

    assert!(agent.emotions.worry > 0.0, "he feels it");
    assert!(
        (agent.emotions.worry - agent.patterns.everything_i_dread().clamp(0.0, 1.0)).abs() < 1e-6,
        "and what he feels is exactly what is written down: {} against {}",
        agent.emotions.worry,
        agent.patterns.everything_i_dread()
    );
}

/// Blame goes to what was done lately, and nothing older than that.
#[test]
fn a_consequence_is_laid_at_a_recent_door_and_not_an_old_one() {
    let mut patterns = Patterns::default();

    let long_ago = [Element::Did("fish".to_string())];
    let lately = [Element::Did("takefrom".to_string())];

    patterns.it_worked(DriveType::Hunger, &long_ago, 0.5, 0);

    let much_later = Patterns::AS_LONG_AS_ANYBODY_CONNECTS + TICKS_PER_DAY;
    patterns.it_worked(DriveType::Utility, &lately, 0.5, much_later);
    patterns.it_cost_me(DriveType::Social, 0.3, much_later);

    assert!(
        patterns.what_i_dread(DriveType::Utility, &lately) > 0.0,
        "what he did this week takes the blame"
    );
    assert_eq!(
        patterns.what_i_dread(DriveType::Hunger, &long_ago),
        0.0,
        "and what he did last month does not"
    );
}
