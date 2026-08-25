// src/analytics/tests/fight_or_flight_tests.rs
//! Tests that fear and anger reach an agent's hands.
//!
//! The appraisal decided what an agent felt about what was in front of it and
//! stopped there. Both branches of action selection that read those feelings
//! were keyed on `recent_attacker`, which is only ever another agent who has
//! just landed a blow, so an agent terrified of a wolf ten paces off fell
//! straight through the flight branch and carried on foraging, and an agent
//! furious at a neighbour fell through the attack branch and did the same.
//!
//! Measured before this: of 22,802 samples that read as ready to fight, anger
//! at people ran to 0.806 and anger at creatures to 0.025. Nearly all the
//! anger in the model was a grudge against somebody, held for life, with
//! nothing whatever downstream of it.

use crate::agents::{AgentConfig, EmotionSource, LifeStage, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// A world with nothing wandering about in it, so what an agent does is about
/// what the test put there.
fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

fn one_person(world: World) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation.population.agents[0].state.health = 100.0;
    simulation
}

/// Somebody frightened of a wolf puts ground between themselves and it.
#[test]
fn a_frightened_agent_runs_away_from_the_thing_itself() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (34, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);

    let kind = simulation
        .world
        .animals
        .get_species("wolf")
        .expect("wolves exist")
        .name
        .clone();

    let here = simulation.population.agents[0].state.position;
    simulation.population.agents[0]
        .emotions
        .set_fear(EmotionSource::Creature(kind), 0.9);

    let away = simulation
        .run_from_what_frightens_me(&simulation.population.agents[0], here)
        .expect("somebody terrified of a wolf four paces off should go");

    let Action::Move { target } = away else {
        panic!("running away is a move, not {away:?}");
    };

    // The wolf is to the east, so the agent goes west
    assert!(
        target.0 < here.0,
        "it should be heading away from the wolf at x=34, not to {target:?}"
    );
    assert!(
        (target.0 - 34).abs() > (here.0 - 34).abs(),
        "and should end further from it than it started"
    );
}

/// And does not run from something that is not there.
#[test]
fn nobody_runs_from_an_empty_field() {
    let mut simulation = one_person(an_empty_country());
    let here = simulation.population.agents[0].state.position;

    simulation.population.agents[0]
        .emotions
        .set_fear(EmotionSource::Creature("Wolf".to_string()), 0.9);

    assert!(
        simulation
            .run_from_what_frightens_me(&simulation.population.agents[0], here)
            .is_none(),
        "there is no wolf in this world to run from"
    );
}

/// Somebody angry at a wolf within reach strikes at it.
#[test]
fn an_angry_agent_strikes_at_what_is_within_reach() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);

    let kind = simulation
        .world
        .animals
        .get_species("wolf")
        .expect("wolves exist")
        .name
        .clone();
    let here = simulation.population.agents[0].state.position;
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Creature(kind), 0.9);

    let strike = simulation
        .round_on_what_angers_me(&simulation.population.agents[0], here)
        .expect("a wolf one pace off is within reach");

    assert!(
        matches!(strike, Action::Fight { .. }),
        "standing your ground is a fight, not {strike:?}"
    );
}

/// A wolf across the field is walked at, not thrown at.
#[test]
fn an_angry_agent_closes_the_last_pace_but_does_not_cross_the_map() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (34, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);

    let kind = simulation
        .world
        .animals
        .get_species("wolf")
        .expect("wolves exist")
        .name
        .clone();
    let here = simulation.population.agents[0].state.position;
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Creature(kind.clone()), 0.9);

    let closing = simulation
        .round_on_what_angers_me(&simulation.population.agents[0], here)
        .expect("four paces is close enough to bother");
    assert!(
        matches!(closing, Action::Move { .. }),
        "it should walk at it first, not {closing:?}"
    );

    // The same wolf right across the field is somebody else's problem
    if let Some(wolf) = simulation.world.animals.get_all_mut().first_mut() {
        wolf.position = (39, 30);
    }
    assert!(
        simulation
            .round_on_what_angers_me(&simulation.population.agents[0], here)
            .is_none(),
        "nine paces is too far to go looking for a fight"
    );
}

/// Two people, one of whom cannot stand the other.
fn two_people() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(an_empty_country(), population);

    for (index, position) in [(30, 30, 0), (31, 30, 0)].into_iter().enumerate() {
        let agent = &mut simulation.population.agents[index];
        agent.state.position = position;
        agent.state.life_stage = LifeStage::Adult;
        agent.state.health = 100.0;
        agent.traits = crate::core::traits::TraitSet::new();
    }

    simulation
}

/// A grudge against somebody standing next to you comes to blows.
#[test]
fn a_standing_grudge_against_somebody_in_reach_comes_to_blows() {
    let mut simulation = two_people();
    let them = simulation.population.agents[1].id;
    let here = simulation.population.agents[0].state.position;

    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.8);

    let strike = simulation
        .round_on_whoever_angers_me(&simulation.population.agents[0], here)
        .expect("somebody you cannot stand, one pace away");

    match strike {
        Action::Attack { target_agent_id, .. } => assert_eq!(target_agent_id, them),
        other => panic!("it should come to blows, not {other:?}"),
    }
}

/// A mild dislike does not.
#[test]
fn a_mild_dislike_is_not_worth_hitting_anybody_over() {
    let mut simulation = two_people();
    let them = simulation.population.agents[1].id;
    let here = simulation.population.agents[0].state.position;

    // Three small grudges: `should_attack` reads the total and calls this a
    // man ready to fight, and there is nobody he is ready to fight
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.2);
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Creature("Wolf".to_string()), 0.2);
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Event("a bad winter".to_string()), 0.2);

    assert!(
        simulation.population.agents[0].emotions.anger > 0.5,
        "the test needs the total to read as ready to fight"
    );
    assert!(
        simulation
            .round_on_whoever_angers_me(&simulation.population.agents[0], here)
            .is_none(),
        "and nobody in particular to fight"
    );
}

/// Nobody raises a hand to a child.
#[test]
fn nobody_raises_a_hand_to_a_child() {
    let mut simulation = two_people();
    let them = simulation.population.agents[1].id;
    simulation.population.agents[1].state.life_stage = LifeStage::Child;
    let here = simulation.population.agents[0].state.position;

    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.9);

    assert!(
        simulation
            .round_on_whoever_angers_me(&simulation.population.agents[0], here)
            .is_none(),
        "however the day has gone"
    );
}

/// A grudge against somebody who would flatten you turns into keeping away
/// from them.
#[test]
fn a_grudge_against_somebody_stronger_comes_out_as_keeping_clear() {
    let mut simulation = two_people();
    let them = simulation.population.agents[1].id;

    simulation.population.agents[1]
        .skills
        .get_skill_mut(SkillType::MeleeCombat)
        .level = 10;
    simulation.population.agents[0]
        .skills
        .get_skill_mut(SkillType::MeleeCombat)
        .level = -10;
    simulation.population.agents[0].state.health = 25.0;

    assert!(
        simulation.population.agents[1].own_strength()
            > simulation.population.agents[0].own_strength(),
        "the test needs one of them to be much the better bet"
    );

    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.9);

    simulation.square_up_to_the_people_i_resent();

    assert!(
        simulation.population.agents[0].emotions.fear > 0.0,
        "somebody who cannot take the man he resents should be afraid of him"
    );
    assert_eq!(
        simulation.population.agents[0]
            .emotions
            .who_angers_me_most()
            .map(|(_, held)| held),
        Some(0.9),
        "and should go on resenting him exactly as much"
    );

    let here = simulation.population.agents[0].state.position;
    let away = simulation
        .run_from_whoever_frightens_me(&simulation.population.agents[0], here)
        .expect("and should want to be somewhere else");
    assert!(matches!(away, Action::Move { .. }));
}

/// And one against somebody you can take does not.
#[test]
fn a_grudge_against_somebody_weaker_stays_a_grudge() {
    let mut simulation = two_people();
    let them = simulation.population.agents[1].id;

    simulation.population.agents[0]
        .skills
        .get_skill_mut(SkillType::MeleeCombat)
        .level = 10;
    simulation.population.agents[1].state.health = 20.0;

    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.9);

    simulation.square_up_to_the_people_i_resent();

    assert_eq!(
        simulation.population.agents[0].emotions.fear, 0.0,
        "a man who can take the neighbour he resents is not afraid of him"
    );
}

/// Once they are out of sight there is nothing to shrink from, though the
/// grudge stands.
#[test]
fn keeping_clear_of_somebody_stops_when_they_are_not_there() {
    let mut simulation = two_people();
    let them = simulation.population.agents[1].id;

    simulation.population.agents[1]
        .skills
        .get_skill_mut(SkillType::MeleeCombat)
        .level = 10;
    simulation.population.agents[0].state.health = 25.0;
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Agent(them), 0.9);

    simulation.square_up_to_the_people_i_resent();
    assert!(simulation.population.agents[0].emotions.fear > 0.0);

    // He walks off over the hill
    simulation.population.agents[1].state.position = (80, 80, 0);
    simulation.square_up_to_the_people_i_resent();

    assert_eq!(
        simulation.population.agents[0].emotions.fear, 0.0,
        "there is nobody there to be afraid of now"
    );
    assert_eq!(
        simulation.population.agents[0]
            .emotions
            .who_angers_me_most()
            .map(|(_, held)| held),
        Some(0.9),
        "but out of sight is not out of mind"
    );
}

/// Winning a fight against a creature teaches an agent about fighting, not
/// about hunting.
#[test]
fn a_fight_teaches_an_agent_about_fighting() {
    use crate::agents::practices::Undertaking;

    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);

    let which = simulation.world.animals.get_all()[0].id;
    let before_fighting = simulation.population.agents[0]
        .lessons
        .attempts(Undertaking::Fighting);
    let before_hunting = simulation.population.agents[0]
        .lessons
        .attempts(Undertaking::Hunting);

    simulation.execute_action(
        &Action::Fight {
            animal_id: which,
            weapon: None,
        },
        0,
    );

    assert_eq!(
        simulation.population.agents[0]
            .lessons
            .attempts(Undertaking::Fighting),
        before_fighting + 1,
        "standing your ground is something an agent learns about fighting"
    );
    assert_eq!(
        simulation.population.agents[0]
            .lessons
            .attempts(Undertaking::Hunting),
        before_hunting,
        "and nothing whatever about hunting"
    );
}

/// You cannot fight what has walked off.
#[test]
fn a_fight_needs_something_within_arms_reach() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (39, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);

    let which = simulation.world.animals.get_all()[0].id;
    let outcome = simulation.execute_action(
        &Action::Fight {
            animal_id: which,
            weapon: None,
        },
        0,
    );

    assert!(
        !outcome.success,
        "a wolf nine paces off cannot be hit: {:?}",
        outcome.message
    );
}
