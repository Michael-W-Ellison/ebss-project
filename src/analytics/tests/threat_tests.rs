// src/analytics/tests/threat_tests.rs
//! Tests for what a threat is, and for the whole of what an agent does about
//! one.
//!
//! Two things were missing. A threat was read off the animal's own statistics
//! and nothing else, so it was a question about teeth rather than about what
//! the teeth would end — and the appraisal took the single worst thing in
//! sight and threw the rest away, so a man surrounded by four wolves faced
//! whichever one happened to be nearest.
//!
//! And the response had two branches where the specification asks for five.
//! Fight if you can win, run if you cannot; run if you can win but cannot
//! lift an arm; fight if you cannot win and there is nowhere to run; and when
//! neither is possible, freeze.

use crate::agents::body::{BodyPartStatus, BodyPartType};
use crate::agents::{AgentConfig, EmotionSource, LifeStage, Population, ThreatAssessment};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::{verbs, Action};
use crate::world::{World, WorldConfig};

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
    simulation.population.agents[0].state.energy = 100.0;
    simulation
}

fn afraid_of(simulation: &mut Simulation, what: &str) {
    let kind = simulation
        .world
        .animals
        .get_species(what)
        .expect("the species exists")
        .name
        .clone();
    simulation.population.agents[0]
        .emotions
        .set_fear(EmotionSource::Creature(kind), 0.9);
}

fn angry_at(simulation: &mut Simulation, what: &str) {
    let kind = simulation
        .world
        .animals
        .get_species(what)
        .expect("the species exists")
        .name
        .clone();
    simulation.population.agents[0]
        .emotions
        .set_anger(EmotionSource::Creature(kind), 0.9);
}

fn cripple(simulation: &mut Simulation, parts: &[BodyPartType]) {
    for part in parts {
        if let Some(limb) = simulation.population.agents[0].body.get_part_mut(*part) {
            limb.status = BodyPartStatus::Disabled;
        }
    }
}

// --------------------------------------------------------------------------
// Several of a thing
// --------------------------------------------------------------------------

/// Four wolves are worse than one wolf. The model used to say they were
/// exactly the same as one, because the appraisal took the worst of what was
/// in sight and discarded everything else.
#[test]
fn a_pack_is_worse_than_the_worst_of_it() {
    let one = ThreatAssessment::a_pack_of(&[1.0]);
    let four = ThreatAssessment::a_pack_of(&[1.0, 1.0, 1.0, 1.0]);

    assert!(
        four > one * 2.0,
        "four wolves should be much worse than one: {four} against {one}"
    );
}

/// And they are not four times worse, because a man can only be bitten from
/// so many sides at once.
#[test]
fn a_pack_is_not_the_sum_of_it() {
    let one = ThreatAssessment::a_pack_of(&[1.0]);
    let four = ThreatAssessment::a_pack_of(&[1.0, 1.0, 1.0, 1.0]);

    assert!(
        four < one * 4.0,
        "and not four times worse: {four} against {one}"
    );
}

/// The worst of them counts for the most, whatever order they come in.
#[test]
fn the_worst_of_them_counts_for_the_most() {
    let one_way = ThreatAssessment::a_pack_of(&[0.2, 1.0, 0.5]);
    let other_way = ThreatAssessment::a_pack_of(&[1.0, 0.5, 0.2]);

    assert!(
        (one_way - other_way).abs() < 0.0001,
        "the order they are counted in should not matter: {one_way} and {other_way}"
    );
}

/// The specification's own example: a man who would stand his ground against
/// one wolf runs from four.
///
/// This is the whole point of counting the pack. Before it, the appraisal
/// took the worst single thing in sight and threw the rest away, so being
/// hemmed in by four wolves was indistinguishable from meeting one.
#[test]
fn a_man_who_would_fight_one_wolf_runs_from_four() {
    let ring = [(31, 30), (29, 30), (30, 31), (30, 29)];

    let answer_with = |how_many: usize| {
        let mut world = an_empty_country();
        for at in ring.iter().take(how_many) {
            world
                .spawn_animal("wolf".to_string(), *at)
                .expect("a wolf should spawn");
        }

        let mut simulation = one_person(world);
        // A man in middling condition, which is what most people in a
        // settlement are most of the time
        simulation.population.agents[0].state.health = 70.0;
        simulation.feel_about_what_stands_in_the_way();

        let here = simulation.population.agents[0].state.position;
        simulation.how_this_one_answers_a_threat(&simulation.population.agents[0], here)
    };

    let one = answer_with(1).expect("one wolf at his elbow wants an answer");
    let four = answer_with(4).expect("four of them certainly do");

    assert!(
        matches!(one, Action::Fight { .. }),
        "he can take one wolf: {one:?}"
    );
    assert!(
        matches!(four, Action::FleeFrom { .. }),
        "he cannot take four: {four:?}"
    );
}

/// A deer is not a threat, and neither is a field full of them.
///
/// This is what pack-counting cost before it was fixed. A rabbit has an
/// `attack_damage` of 1.0 and a deer of 5.0, because both defend themselves if
/// you go at them — so reading danger off the number alone made a herd of
/// twenty reindeer about as frightening as a wolf. Over twenty-four worlds it
/// had a settlement running 465 times where it should have run 213.
#[test]
fn a_herd_of_deer_is_not_a_pack_of_wolves() {
    let mut world = an_empty_country();
    for at in [(31, 30), (32, 30), (29, 30), (30, 31), (30, 29), (31, 31)] {
        world
            .spawn_animal("deer".to_string(), at)
            .expect("a deer should spawn");
    }

    let mut simulation = one_person(world);
    simulation.population.agents[0].state.health = 70.0;
    simulation.feel_about_what_stands_in_the_way();

    assert_eq!(
        simulation.population.agents[0].emotions.fear, 0.0,
        "nobody is afraid of deer"
    );
}

/// And the thing that does come after people still is one.
#[test]
fn a_wolf_is_still_a_wolf() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");

    let mut simulation = one_person(world);
    simulation.population.agents[0].state.health = 12.0;
    simulation.feel_about_what_stands_in_the_way();

    assert!(
        simulation.population.agents[0].emotions.fear > 0.0,
        "a wolf at his elbow is a wolf"
    );
}

/// What a thing does to somebody who has done nothing to it is a question
/// about its temper, not about its teeth.
#[test]
fn what_menaces_you_is_a_question_about_temper() {
    use crate::environment::fauna::AnimalBehavior;

    assert_eq!(
        AnimalBehavior::Passive.how_much_it_menaces_you(),
        0.0,
        "a thing that runs away is not a threat"
    );
    assert!(
        AnimalBehavior::Aggressive.how_much_it_menaces_you()
            > AnimalBehavior::Defensive.how_much_it_menaces_you(),
        "a thing that comes after you beats a thing that would rather not"
    );
}

// --------------------------------------------------------------------------
// What a threat threatens
// --------------------------------------------------------------------------

/// Everybody has something to lose, because being alive is what makes
/// tomorrow's dinner possible.
#[test]
fn a_comfortable_man_still_has_something_to_lose() {
    let simulation = one_person(an_empty_country());

    assert!(
        simulation.population.agents[0].what_i_stand_to_lose() > 0.5,
        "nobody shrugs at a wolf"
    );
}

/// And a man with everything pressing at once has more.
#[test]
fn a_man_with_everything_pressing_has_more_to_lose() {
    let mut simulation = one_person(an_empty_country());

    let comfortable = simulation.population.agents[0].what_i_stand_to_lose();

    for drive in [
        DriveType::Hunger,
        DriveType::Thirst,
        DriveType::Rest,
        DriveType::Safety,
    ] {
        if let Some(asking) = simulation.population.agents[0].drives.get_mut(drive) {
            asking.value = 1.0;
        }
    }

    let desperate = simulation.population.agents[0].what_i_stand_to_lose();

    assert!(
        desperate > comfortable,
        "a man with everything to answer for minds losing it more: \
         {desperate} against {comfortable}"
    );
}

/// What is at stake moves how much the thing is feared, and not the odds of
/// beating it. Whether a man can beat a wolf is a question about the man and
/// the wolf.
#[test]
fn what_is_at_stake_moves_the_fear_and_not_the_odds() {
    let little = ThreatAssessment::assess_against_what_is_at_stake(
        1.0,
        0.5,
        0.5,
        EmotionSource::Creature("wolf".to_string()),
    );
    let much = ThreatAssessment::assess_against_what_is_at_stake(
        1.0,
        0.5,
        1.0,
        EmotionSource::Creature("wolf".to_string()),
    );

    assert!(
        much.threat_level > little.threat_level,
        "the man with more to lose feels it more"
    );
    assert_eq!(
        much.can_overcome, little.can_overcome,
        "but he is exactly as able to win the fight"
    );
}

// --------------------------------------------------------------------------
// The five answers
// --------------------------------------------------------------------------

/// Can overcome it: stand and fight.
#[test]
fn a_man_who_can_win_stands_his_ground() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    angry_at(&mut simulation, "wolf");

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf one pace off wants an answer");

    assert!(
        matches!(answer, Action::Fight { .. }),
        "standing your ground is a fight, not {answer:?}"
    );
}

/// Cannot overcome it: run.
#[test]
fn a_man_who_cannot_win_runs() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    afraid_of(&mut simulation, "wolf");

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf one pace off wants an answer");

    assert!(
        matches!(answer, Action::FleeFrom { .. }),
        "he should go, not {answer:?}"
    );
}

/// Cannot overcome it and cannot run: turn and fight anyway. This is the
/// cornered case, and it did not exist — an agent with nowhere to go went
/// back to gathering berries with a wolf at its elbow.
#[test]
fn a_cornered_man_turns_and_fights() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    afraid_of(&mut simulation, "wolf");

    // Nothing left in him to run with
    simulation.population.agents[0].state.energy = 1.0;

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf one pace off wants an answer");

    assert!(
        matches!(answer, Action::Fight { .. }),
        "with nothing left to run on he has to fight: {answer:?}"
    );
}

/// Could overcome it but cannot lift an arm: go.
#[test]
fn a_man_who_cannot_lift_an_arm_goes() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    angry_at(&mut simulation, "wolf");
    cripple(
        &mut simulation,
        &[BodyPartType::LeftArm, BodyPartType::RightArm],
    );

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf one pace off wants an answer");

    assert!(
        matches!(answer, Action::FleeFrom { .. }),
        "he cannot fight it, so he goes: {answer:?}"
    );
}

/// Neither: freeze. The third answer, and the one nobody arrives at on
/// purpose.
#[test]
fn a_man_who_can_do_neither_freezes() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    afraid_of(&mut simulation, "wolf");
    cripple(
        &mut simulation,
        &[BodyPartType::LeftArm, BodyPartType::RightArm],
    );
    simulation.population.agents[0].state.energy = 1.0;

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf one pace off wants an answer");

    assert!(
        matches!(answer, Action::Freeze),
        "he can neither run nor raise a hand: {answer:?}"
    );
}

/// A child does not fight a wolf, whatever else is true of it. This is the
/// commonest way in the world for fighting not to be an option, and leaving
/// it out made freezing unreachable: measured over eight worlds, with the
/// body and the health tests alone, not one agent ever froze.
#[test]
fn a_child_does_not_fight_a_wolf() {
    let mut simulation = one_person(an_empty_country());
    simulation.population.agents[0].state.life_stage = LifeStage::Child;

    assert!(
        !simulation.population.agents[0].could_i_fight_at_all(12.0),
        "a child of five does not stand its ground against a wolf"
    );
}

/// And a child with nothing left to run on freezes, which is what freezing is
/// for.
#[test]
fn a_worn_out_child_freezes() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (31, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    afraid_of(&mut simulation, "wolf");
    simulation.population.agents[0].state.life_stage = LifeStage::Child;
    simulation.population.agents[0].state.energy = 1.0;

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
        .expect("a wolf one pace off wants an answer");

    assert!(
        matches!(answer, Action::Freeze),
        "it can neither run nor fight: {answer:?}"
    );
}

/// A man with no legs cannot run either, and that is the same corner.
#[test]
fn no_legs_is_the_same_corner_as_no_energy() {
    let mut simulation = one_person(an_empty_country());
    cripple(
        &mut simulation,
        &[BodyPartType::LeftLeg, BodyPartType::RightLeg],
    );

    assert!(
        !simulation.population.agents[0]
            .could_i_run_at_all(Simulation::WHAT_RUNNING_COSTS),
        "a man with no legs is not going anywhere"
    );
}

/// Freezing is a real action and does nothing at all, which is the point.
#[test]
fn freezing_leaves_a_man_exactly_where_he_was() {
    let mut simulation = one_person(an_empty_country());
    let here = simulation.population.agents[0].state.position;

    let result = simulation.execute_action(&Action::Freeze, 0);

    assert!(result.success, "{:?}", result.message);
    assert_eq!(
        simulation.population.agents[0].state.position, here,
        "that is the whole of what freezing is"
    );
}

/// Nobody crosses a field to pick a fight with a wolf.
#[test]
fn nobody_goes_looking_for_a_fight_across_a_field() {
    let mut world = an_empty_country();
    world
        .spawn_animal("wolf".to_string(), (39, 30))
        .expect("a wolf should spawn");
    let mut simulation = one_person(world);
    angry_at(&mut simulation, "wolf");

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
            .is_none(),
        "nine paces is too far to go looking for a fight"
    );
}

/// And nobody answers a threat that is not there.
#[test]
fn nobody_answers_an_empty_field() {
    let mut simulation = one_person(an_empty_country());
    simulation.population.agents[0]
        .emotions
        .set_fear(EmotionSource::Creature("Wolf".to_string()), 0.9);

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .how_this_one_answers_a_threat(&simulation.population.agents[0], here)
            .is_none(),
        "there is no wolf in this world"
    );
}

// --------------------------------------------------------------------------
// The matrix
// --------------------------------------------------------------------------

/// Freezing is a verb like any other, and the matrix says so.
#[test]
fn freezing_is_in_the_matrix() {
    let one = verbs::what_that_verb_is("freeze").expect("in the matrix");

    assert!(one.is_live(), "something performs it");
    assert_eq!(
        one.family,
        verbs::Family::Combat,
        "it is one of the three answers to a fight"
    );
}
