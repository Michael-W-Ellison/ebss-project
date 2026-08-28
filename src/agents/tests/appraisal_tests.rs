// src/agents/tests/appraisal_tests.rs
//! Tests that fear and anger are appraisals rather than timers.
//!
//! Does a thing threaten my ability to satisfy my drives, and can I fight it?
//! Did a thing prevent it, and can I fight *that*? Where the answer is yes it
//! comes out as anger and the agent stands its ground; where it is no it comes
//! out as fear and the agent goes.
//!
//! `ThreatAssessment` has always turned coping potential into one or the
//! other. What was missing was anything to consult it except the resolution of
//! a blow that had already landed - a wolf ten paces off and closing produced
//! no feeling at all until it bit somebody. Measured over three worlds, mean
//! fear ran at 0.01 to 0.06 and mean anger at exactly zero, and not one agent
//! in a hundred and seventy ever reached the 0.6 that `should_flee` wants, so
//! the branch of `generate_action` that lets an agent run or fight never once
//! fired in a whole settlement's life.

use crate::agents::practices::Undertaking;
use crate::agents::{Agent, AgentConfig, EmotionSource, LifeStage};
use crate::core::{DriveType, EmotionType};

/// An adult in good order, with no opinions of its own to muddy the reading.
fn somebody() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits = crate::core::traits::TraitSet::new();
    agent.state.life_stage = LifeStage::Adult;
    agent.state.health = 100.0;
    agent.state.energy = 100.0;
    agent
}

/// The same wolf frightens one person and angers another.
#[test]
fn the_same_threat_is_fear_to_one_and_anger_to_another() {
    let wolf = EmotionSource::Creature("Wolf".to_string());

    let mut strong = somebody();
    strong
        .skills
        .get_skill_mut(crate::agents::SkillType::MeleeCombat)
        .level = 9;

    let mut weak = somebody();
    weak.state.health = 20.0;
    weak.skills
        .get_skill_mut(crate::agents::SkillType::MeleeCombat)
        .level = -10;

    assert!(
        strong.own_strength() > weak.own_strength(),
        "the test needs one of them to be the better bet in a fight"
    );

    let felt_by_strong = strong.appraise_what_is_there(0.5, wolf.clone());
    let felt_by_weak = weak.appraise_what_is_there(0.5, wolf);

    assert_eq!(
        felt_by_strong,
        EmotionType::Anger,
        "somebody who can take it should be angry about it"
    );
    assert_eq!(
        felt_by_weak,
        EmotionType::Fear,
        "somebody who cannot should be afraid of it"
    );

    assert!(strong.emotions.anger > 0.0 && strong.emotions.fear == 0.0);
    assert!(weak.emotions.fear > 0.0 && weak.emotions.anger == 0.0);
}

/// A thing that is simply standing there is one thing, however long it stands.
#[test]
fn a_standing_threat_does_not_build_up_for_ever() {
    let wolf = EmotionSource::Creature("Wolf".to_string());
    let mut agent = somebody();

    agent.appraise_what_is_there(0.5, wolf.clone());
    let after_one_look = agent.emotions.anger.max(agent.emotions.fear);

    for _ in 0..500 {
        agent.appraise_what_is_there(0.5, wolf.clone());
    }
    let after_five_hundred = agent.emotions.anger.max(agent.emotions.fear);

    assert!(
        (after_five_hundred - after_one_look).abs() < 0.01,
        "one wolf is one wolf however many ticks it stands there: {after_one_look:.2} \
         became {after_five_hundred:.2}"
    );
}

/// And when it goes, the feeling goes with it.
#[test]
fn what_is_no_longer_there_stops_being_frightening() {
    let wolf = EmotionSource::Creature("Wolf".to_string());
    let mut agent = somebody();
    agent.state.health = 20.0;

    agent.appraise_what_is_there(0.9, wolf);
    assert!(agent.emotions.fear > 0.0, "it should be afraid while it is there");

    agent.emotions.nothing_is_stalking_me();
    assert_eq!(
        agent.emotions.fear, 0.0,
        "an agent that outran a wolf should stop running from it"
    );
}

/// Having won makes fighting look better; having lost makes it look worse.
#[test]
fn what_happened_last_time_changes_what_a_fight_looks_like() {
    let mut untried = somebody();
    let mut winner = somebody();
    let mut loser = somebody();

    for _ in 0..12 {
        winner.lessons.record(Undertaking::Fighting, true);
        loser.lessons.record(Undertaking::Fighting, false);
    }

    assert!(
        winner.what_fighting_has_taught_me() > untried.what_fighting_has_taught_me(),
        "somebody who has fought and won should reckon themselves better for it"
    );
    assert!(
        loser.what_fighting_has_taught_me() < untried.what_fighting_has_taught_me(),
        "and somebody who has fought and lost, worse"
    );

    assert!(
        winner.own_strength() > loser.own_strength(),
        "two agents of identical build should not appraise the same wolf the \
         same way if one of them has beaten a wolf before"
    );
}

/// And that difference is enough to turn a fight into a flight.
#[test]
fn a_beaten_agent_runs_where_a_winner_stands() {
    let wolf = EmotionSource::Creature("Wolf".to_string());

    let mut winner = somebody();
    let mut loser = somebody();
    for _ in 0..12 {
        winner.lessons.record(Undertaking::Fighting, true);
        loser.lessons.record(Undertaking::Fighting, false);
    }

    // A wolf pitched between what the two of them believe they can manage
    let between = (winner.own_strength() + loser.own_strength()) / 2.0;

    assert_eq!(
        winner.appraise_what_is_there(between, wolf.clone()),
        EmotionType::Anger,
        "the one who has won before should stand its ground"
    );
    assert_eq!(
        loser.appraise_what_is_there(between, wolf),
        EmotionType::Fear,
        "the one who has been beaten should run"
    );
}

/// Nobody learns anything from a fight they have not had.
#[test]
fn somebody_who_has_never_fought_takes_themselves_at_face_value() {
    let agent = somebody();

    assert_eq!(agent.lessons.attempts(Undertaking::Fighting), 0);
    assert_eq!(
        agent.what_fighting_has_taught_me(),
        1.0,
        "with no record to go on, an agent is worth what it is worth"
    );
}

/// A need nothing can be done about is frightening, not enraging.
#[test]
fn a_need_with_nothing_to_round_on_produces_fear() {
    let mut agent = somebody();

    // Days of asking for food and getting none. There is no adversary in this:
    // a worked-out field and a hard winter prevent an agent satisfying its
    // drives exactly as a wolf does, and the difference is that there is
    // nothing to round on.
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
        hunger.denied_ticks = 400;
    }
    agent.state.ticks_without_food = 9_600;
    agent.state.physiology.gone_without_food_for(9_600);

    agent.update_emotions_from_drives();

    assert!(
        agent.emotions.fear > 0.5,
        "somebody within a day of starving should be frightened of it; fear \
         stood at {:.2}",
        agent.emotions.fear
    );

    // And it is proportionate: the same hunger with a week still in hand is
    // worrying rather than terrifying
    let mut earlier = somebody();
    if let Some(hunger) = earlier.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
        hunger.denied_ticks = 400;
    }
    earlier.state.ticks_without_food = 6_000;
    earlier.state.physiology.gone_without_food_for(6_000);
    earlier.update_emotions_from_drives();

    assert!(
        earlier.emotions.fear < agent.emotions.fear,
        "eleven days from starving should frighten less than one day from it"
    );
    assert_eq!(
        agent.emotions.anger, 0.0,
        "and there is nothing there to be angry at"
    );
}

/// Wanting a thing that keeps arriving is not being prevented from anything.
#[test]
fn a_need_that_is_being_met_frightens_nobody() {
    let mut agent = somebody();

    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.95;
        hunger.denied_ticks = 0;
    }
    agent.state.ticks_without_food = 0;
    agent.state.physiology.gone_without_food_for(0);

    agent.update_emotions_from_drives();

    assert!(
        agent.emotions.fear < 0.1,
        "somebody about to sit down to dinner is not afraid of anything; fear \
         stood at {:.2}",
        agent.emotions.fear
    );
}

/// The nearer the thing, the worse it is.
#[test]
fn a_wolf_across_the_field_is_not_the_wolf_at_your_elbow() {
    let wolf = EmotionSource::Creature("Wolf".to_string());

    let mut close = somebody();
    close.state.health = 20.0;
    let mut far = somebody();
    far.state.health = 20.0;

    // The falloff is applied by the simulation before it gets here, so this
    // checks the appraisal answers to the strength it is handed
    close.appraise_what_is_there(0.9, wolf.clone());
    far.appraise_what_is_there(0.1, wolf);

    assert!(
        close.emotions.fear > far.emotions.fear,
        "the nearer wolf should frighten more: {:.2} against {:.2}",
        close.emotions.fear,
        far.emotions.fear
    );
}
