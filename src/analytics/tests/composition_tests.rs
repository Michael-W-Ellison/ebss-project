// src/analytics/tests/composition_tests.rs
//! Patterns as compositions of smaller actions.
//!
//! The specification says an agent "links its **previous actions** taken to
//! the drive satisfaction to form a pattern" - plural - and until now the only
//! thing linked was the single act that happened to produce the drive change.
//! So `Did("eat")` took all the credit for hunger coming off and the gathering
//! that filled the pack took none, and the composition that actually feeds a
//! man could not be represented, let alone learned.

use crate::agents::patterns::{Element, Patterns};
use crate::agents::{Agent, AgentConfig};
use crate::core::DriveType;
use crate::environment::Action;

/// A run is two things in order, and it survives being written down.
#[test]
fn a_run_is_an_element_like_any_other() {
    let run = Element::Then("gather".to_string(), "eat".to_string());
    let written: String = run.clone().into();
    assert_eq!(written, "then:gather>eat");
    assert_eq!(Element::try_from(written).unwrap(), run);
}

/// What led up to a thing is the handful of steps before it, and no more.
#[test]
fn a_man_remembers_the_last_few_things_he_did() {
    let mut agent = Agent::new(AgentConfig::default());

    for _ in 0..10 {
        agent.that_is_what_i_just_did(&Action::Gather {
            resource_type: "berries".to_string(),
        });
    }

    assert_eq!(
        agent.lately.len(),
        Agent::A_RUN_WORTH_KEEPING,
        "a longer memory only adds coincidences to be reinforced"
    );
}

/// The runs offered for credit are the ones that end in what was just done.
#[test]
fn the_runs_are_what_led_up_to_this() {
    let mut agent = Agent::new(AgentConfig::default());

    agent.that_is_what_i_just_did(&Action::Move { target: (1, 1, 0) });
    agent.that_is_what_i_just_did(&Action::Gather {
        resource_type: "berries".to_string(),
    });

    let runs = agent.what_led_up_to_this(&Action::Eat {
        food_type: "generic".to_string(),
    });

    assert!(
        runs.contains(&Element::Then("gather".to_string(), "eat".to_string())),
        "gathering then eating is the run that feeds a man: {runs:?}"
    );
    assert!(
        runs.contains(&Element::Then("move".to_string(), "eat".to_string())),
        "and the step before it is offered too, so the arithmetic can \
         choose between them: {runs:?}"
    );
}

/// A thing that follows itself teaches nothing about order.
#[test]
fn doing_the_same_thing_twice_is_not_a_composition() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.that_is_what_i_just_did(&Action::Gather {
        resource_type: "berries".to_string(),
    });

    let runs = agent.what_led_up_to_this(&Action::Gather {
        resource_type: "berries".to_string(),
    });

    assert!(
        !runs
            .iter()
            .any(|run| matches!(run, Element::Then(first, next) if first == next)),
        "a man gathering twice has not learned that gathering follows \
         gathering: {runs:?}"
    );
}

/// The composition and its halves compete on the same terms.
///
/// Where the pair is what matters it is there every time and outruns either
/// half; where only the last act matters the pairs vary and the atom wins.
/// That is the module's own arithmetic, pointed at order.
#[test]
fn the_pair_outruns_its_halves_when_the_pair_is_what_matters() {
    let mut patterns = Patterns::default();
    let gather_then_eat = Element::Then("gather".to_string(), "eat".to_string());
    let eat = Element::Did("eat".to_string());

    // Every success came after gathering, and half of them also came after a
    // walk - so the walk climbs on its own successes only.
    for round in 0..10u32 {
        let mut elements = vec![eat.clone(), gather_then_eat.clone()];
        if round % 2 == 0 {
            elements.push(Element::Then("move".to_string(), "eat".to_string()));
        }
        patterns.it_worked(DriveType::Hunger, &elements, 0.5, round);
    }

    let with_gathering = patterns
        .trail(DriveType::Hunger, &gather_then_eat)
        .map(|trail: &crate::agents::patterns::Trail| trail.worth())
        .unwrap_or(0.0);
    let with_a_walk = patterns
        .trail(
            DriveType::Hunger,
            &Element::Then("move".to_string(), "eat".to_string()),
        )
        .map(|trail: &crate::agents::patterns::Trail| trail.worth())
        .unwrap_or(0.0);

    assert!(
        with_gathering > with_a_walk,
        "the run that was there every time should outrun the one that was \
         there half the time: {with_gathering:.2} against {with_a_walk:.2}"
    );
}

/// A run has to be worn to be followed, and has to beat the bare act.
#[test]
fn a_run_has_to_be_worn_before_it_is_followed() {
    let mut patterns = Patterns::default();
    let run = Element::Then("gather".to_string(), "eat".to_string());

    patterns.it_worked(DriveType::Hunger, &[run.clone()], 0.05, 0);
    assert!(
        patterns.what_follows(DriveType::Hunger, "gather").is_none(),
        "one lucky afternoon is a coincidence, not a habit"
    );

    for round in 1..12u32 {
        patterns.it_worked(DriveType::Hunger, &[run.clone()], 0.5, round);
    }
    assert_eq!(
        patterns.what_follows(DriveType::Hunger, "gather"),
        Some("eat"),
        "a run walked a dozen times is what he does next"
    );
}

/// Where the atom is what matters, the atom is what he does - whatever order
/// it happened to come in.
#[test]
fn where_the_act_is_what_matters_the_order_is_not_followed() {
    let mut patterns = Patterns::default();
    let run = Element::Then("move".to_string(), "drink".to_string());
    let drink = Element::Did("drink".to_string());

    // Drinking answers thirst whether or not a walk came first, and the
    // record says so: the atom is there every time and the run is not.
    for round in 0..20u32 {
        let mut elements = vec![drink.clone()];
        if round % 4 == 0 {
            elements.push(run.clone());
        }
        patterns.it_worked(DriveType::Thirst, &elements, 0.5, round);
    }

    assert!(
        patterns.what_follows(DriveType::Thirst, "move").is_none(),
        "the walk is not what made the drink work, and following it would be \
         a superstition"
    );
}

/// And the whole of it through the agent: what he has just done decides what
/// he reaches for next.
#[test]
fn what_he_just_did_decides_what_he_reaches_for() {
    let mut agent = Agent::new(AgentConfig::default());
    let run = Element::Then("gather".to_string(), "eat".to_string());

    for round in 0..12u32 {
        agent
            .patterns
            .it_worked(DriveType::Hunger, &[run.clone()], 0.5, round);
    }

    assert_eq!(
        agent.what_usually_comes_next(DriveType::Hunger),
        None,
        "with nothing lately done there is no run to be in the middle of"
    );

    agent.that_is_what_i_just_did(&Action::Gather {
        resource_type: "berries".to_string(),
    });

    assert_eq!(
        agent.what_usually_comes_next(DriveType::Hunger),
        Some("eat"),
        "having gathered, the thing that has answered hunger next is eating"
    );
}
