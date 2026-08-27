// src/core/tests/drive_hierarchy_tests.rs
//! Tests that needs are ranked by what they cost to ignore.
//!
//! The drives used to be compared against one another as though they were all
//! the same kind of thing, by `value * weight`. They are not. A need that
//! kills in a day and a wish for a better axe do not belong on one scale, and
//! reading them off one was why an agent would go on hunting while it died of
//! thirst, and why three drives nothing in the world could answer - Luxury,
//! Preparedness, Utility - stood above their thresholds 98, 98 and 85 per cent
//! of the time and took every turn the drives were ever given.
//!
//! Two things order them now. The tier says what a need of that kind may
//! interrupt. Within that, a need that kills presses in proportion to how soon
//! it would, worked out from the clocks the body actually runs on - so nobody
//! wrote down that thirst beats hunger, it falls out of dehydration taking
//! health at 2,160 ticks where starvation takes it at 4,320 times whatever the
//! body has put by.

use crate::agents::{Agent, AgentConfig, LifeStage};
use crate::core::{DriveState, DriveRank, DriveType};

/// A fed, watered, rested agent with nothing wrong with it.
fn somebody_comfortable() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    // Grown, and so carrying a grown body's reserves. An agent starts life as
    // an infant, which has a quarter of them - and that is enough to reorder
    // its needs, which is the point of the test below rather than an accident
    // to be tripped over in the others.
    agent.state.life_stage = LifeStage::Adult;
    agent.state.health = 100.0;
    agent.state.energy = 100.0;
    agent.state.ticks_without_food = 0;
    agent.state.ticks_without_water = 0;
    agent
}

/// The bands are what the specification says they are.
#[test]
fn the_needs_that_kill_are_the_ones_that_interrupt() {
    for drive_type in [
        DriveType::Hunger,
        DriveType::Thirst,
        DriveType::Rest,
        DriveType::Safety,
    ] {
        assert_eq!(
            drive_type.rank(),
            DriveRank::Primary,
            "{drive_type:?} kills you"
        );
    }

    for drive_type in [
        DriveType::Sustenance,
        DriveType::Preparedness,
        DriveType::Shelter,
        DriveType::Social,
        DriveType::Reproduction,
        DriveType::Curiosity,
    ] {
        assert_eq!(drive_type.rank(), DriveRank::Secondary);
    }

    for drive_type in [
        DriveType::Luxury,
        DriveType::Utility,
        DriveType::Construction,
        DriveType::Industry,
        DriveType::Protection,
    ] {
        assert_eq!(drive_type.rank(), DriveRank::Tertiary);
    }

    assert!(DriveRank::Primary.precedence() > DriveRank::Secondary.precedence());
    assert!(DriveRank::Secondary.precedence() > DriveRank::Tertiary.precedence());
}

/// An agent will not go on hunting if it will die of thirst first, even though
/// hunting would answer its hunger.
#[test]
fn thirst_takes_the_turn_from_hunger_when_the_water_runs_out_first() {
    let mut agent = somebody_comfortable();

    // Two days without either. Hunger is further along as a *drive*, but
    // dehydration is much further along as a way of dying.
    agent.state.ticks_without_water = 3_000;
    agent.state.ticks_without_food = 3_000;
    agent.drives.get_mut(DriveType::Hunger).unwrap().value = 0.95;
    agent.drives.get_mut(DriveType::Thirst).unwrap().value = 0.80;

    assert_eq!(
        agent.what_presses_hardest(),
        Some(DriveType::Thirst),
        "the water runs out first, so the water is the problem - hunger \
         pressing {:.1} against thirst {:.1}",
        agent.how_hard_it_presses(DriveType::Hunger),
        agent.how_hard_it_presses(DriveType::Thirst)
    );
}

/// And nothing anybody merely wants outranks either.
#[test]
fn no_amount_of_wanting_a_fine_thing_outranks_being_thirsty() {
    let mut agent = somebody_comfortable();
    agent.state.ticks_without_water = 4_000;
    agent.drives.get_mut(DriveType::Thirst).unwrap().value = 0.9;

    for wish in [
        DriveType::Luxury,
        DriveType::Utility,
        DriveType::Construction,
        DriveType::Industry,
    ] {
        let drive = agent.drives.get_mut(wish).unwrap();
        drive.value = 1.0;
        drive.weight = 3.0;
    }

    assert_eq!(agent.what_presses_hardest(), Some(DriveType::Thirst));
}

/// A body with less put by orders its own needs differently, and nobody wrote
/// that down anywhere.
#[test]
fn a_child_and_an_adult_do_not_rank_the_same_needs_the_same_way() {
    fn how_long_hunger_leaves(stage: LifeStage, empty_for: u32) -> f32 {
        let mut agent = somebody_comfortable();
        agent.state.life_stage = stage;
        agent.state.ticks_without_food = empty_for;
        agent
            .state
            .ticks_before_this_kills_me(DriveType::Hunger)
            .expect("hunger kills")
    }

    let child = how_long_hunger_leaves(LifeStage::Child, 2_000);
    let adult = how_long_hunger_leaves(LifeStage::Adult, 2_000);

    assert!(
        child < adult,
        "a child has less to live on than an adult: {child:.0} against {adult:.0}"
    );

    // And that difference is enough to reorder them. The same two days without
    // food and water: the adult's water is the nearer problem, the child's
    // food has already caught up.
    let mut small = somebody_comfortable();
    small.state.life_stage = LifeStage::Child;
    small.state.ticks_without_food = 3_000;
    small.state.ticks_without_water = 1_000;
    small.drives.get_mut(DriveType::Hunger).unwrap().value = 0.9;

    let mut grown = somebody_comfortable();
    grown.state.life_stage = LifeStage::Adult;
    grown.state.ticks_without_food = 3_000;
    grown.state.ticks_without_water = 1_000;
    grown.drives.get_mut(DriveType::Hunger).unwrap().value = 0.9;

    assert!(
        small.how_hard_it_presses(DriveType::Hunger)
            > grown.how_hard_it_presses(DriveType::Hunger),
        "the same empty stomach should press harder on the smaller body"
    );
}

/// A need that is answered and in no danger is a preference, not an emergency.
#[test]
fn a_satisfied_need_does_not_shout_over_a_real_one() {
    let mut agent = somebody_comfortable();

    // Nothing wrong with it at all, and a settlement that wants a harvest
    agent.drives.get_mut(DriveType::Hunger).unwrap().value = 0.3;
    agent.drives.get_mut(DriveType::Thirst).unwrap().value = 0.3;
    agent.drives.get_mut(DriveType::Sustenance).unwrap().value = 0.9;

    assert_eq!(
        agent.what_presses_hardest(),
        Some(DriveType::Sustenance),
        "a body in no trouble should be thinking about next year's food, not \
         about being a third of the way to hungry - hunger pressing {:.2}, \
         sustenance {:.2}",
        agent.how_hard_it_presses(DriveType::Hunger),
        agent.how_hard_it_presses(DriveType::Sustenance)
    );
}

/// A hungry agent is not thinking about saving food for later.
#[test]
fn nobody_lays_in_stores_on_an_empty_stomach() {
    let mut drives = DriveState::new();

    // Hungry, and has been going short
    {
        let hunger = drives.get_mut(DriveType::Hunger).unwrap();
        hunger.value = 0.9;
        hunger.denied_ticks = 200;
    }

    assert!(
        !drives.is_unlocked(DriveType::Sustenance),
        "next year's grain waits on tonight's dinner"
    );
    assert!(
        !drives.is_unlocked(DriveType::Preparedness),
        "and so does the store cupboard"
    );
    assert!(
        !drives.is_unlocked(DriveType::Luxury),
        "and so, further down the same chain, does anything fine"
    );

    // Fed, and reliably so
    {
        let hunger = drives.get_mut(DriveType::Hunger).unwrap();
        hunger.value = 0.1;
        hunger.denied_ticks = 0;
    }

    assert!(
        drives.is_unlocked(DriveType::Sustenance),
        "a fed agent can think about the harvest"
    );
}

/// One good dinner is not a food supply.
#[test]
fn a_need_has_to_be_answered_reliably_to_count() {
    let mut drives = DriveState::new();
    let hunger = drives.get_mut(DriveType::Hunger).unwrap();

    // Full this moment, but has been going short for days
    hunger.value = 0.0;
    hunger.denied_ticks = DriveState::RELIABLY * 4;

    assert!(
        !drives.is_unlocked(DriveType::Sustenance),
        "a settlement should not start laying in stores on the strength of one \
         good dinner after a bad week"
    );
}

/// A chain does not unlock itself from the far end.
#[test]
fn a_drive_that_is_only_quiet_because_it_is_shut_out_unlocks_nothing() {
    let mut drives = DriveState::new();

    // Hungry, so everything down that chain is shut
    {
        let hunger = drives.get_mut(DriveType::Hunger).unwrap();
        hunger.value = 0.95;
        hunger.denied_ticks = 300;
    }

    // Preparedness reads as low - it is shut out, so it has fallen quiet -
    // and Luxury stands after it. Luxury must not take that quiet for
    // satisfaction.
    drives.get_mut(DriveType::Preparedness).unwrap().value = 0.0;

    assert!(
        !drives.is_unlocked(DriveType::Luxury),
        "Luxury stands behind Preparedness, which stands behind Sustenance and \
         Hunger; a hungry agent wants nothing fine"
    );

    assert!(
        drives
            .what_is_still_wanted_before(DriveType::Luxury)
            .contains(&DriveType::Preparedness),
        "and it should be able to say what it is waiting on"
    );
}

/// A need out of reach stops being felt rather than banking up.
#[test]
fn a_need_that_is_shut_out_fades_rather_than_waiting() {
    let mut drives = DriveState::new();
    drives.get_mut(DriveType::Luxury).unwrap().value = 0.9;

    // Starving, so the whole chain below hunger is shut
    {
        let hunger = drives.get_mut(DriveType::Hunger).unwrap();
        hunger.value = 1.0;
        hunger.denied_ticks = 500;
    }

    // Long enough for a drive of this pace to have gone. A shut-out need fades
    // at the rate it would have built, so how long that takes is the drive's
    // own business: Luxury builds at a thousandth a tick, so nine hundred
    // ticks is the whole of it.
    let ctx = crate::core::DriveContext::default();
    let span = (0.9 / DriveType::Luxury.base_accumulation_rate()).ceil() as usize;

    for _ in 0..span {
        drives.tick_in(&ctx, false);
    }

    assert!(
        drives.get(DriveType::Luxury).unwrap().value < 0.05,
        "somebody who has gone hungry that long is not sitting on a banked-up \
         wish for a finer coat, ready to spend it the moment they eat; it \
         stood at {:.2}",
        drives.get(DriveType::Luxury).unwrap().value
    );
}

/// But it fades at its own pace, not at one rate for everybody.
///
/// A flat rate is a different thing to each drive. At the four thousandths a
/// tick this used to use, Reproduction, Luxury and Protection - which build at
/// a thousandth - fell four times faster than they rose, so a drive shut out
/// even a tenth of the time climbed at half its proper rate. Conception needs
/// the Reproduction drive over its threshold in both parents, so that halved
/// the birth rate, and with it the population of every settlement.
#[test]
fn a_slow_need_does_not_fade_faster_than_it_grows() {
    for drive_type in DriveType::all() {
        let mut drives = DriveState::new();
        drives.get_mut(drive_type).unwrap().value = 0.5;
        drives.get_mut(drive_type).unwrap().fall_quiet();

        let lost = 0.5 - drives.get(drive_type).unwrap().value;
        let builds_at = drive_type.base_accumulation_rate();

        assert!(
            lost <= builds_at + f32::EPSILON,
            "{drive_type:?} loses {lost:.4} a tick when shut out and builds at \
             only {builds_at:.4}, so any time at all shut out leaves it going \
             backwards"
        );
    }
}

/// Nobody has children while something is still trying to kill them.
#[test]
fn children_wait_on_every_primary_need() {
    let mut drives = DriveState::new();

    for pressing in [
        DriveType::Hunger,
        DriveType::Thirst,
        DriveType::Rest,
        DriveType::Safety,
    ] {
        let mut drives = drives.clone();
        let drive = drives.get_mut(pressing).unwrap();
        drive.value = 1.0;
        drive.denied_ticks = 100;

        assert!(
            !drives.is_unlocked(DriveType::Reproduction),
            "{pressing:?} unanswered should be enough on its own to put \
             children out of mind"
        );
    }

    // All four answered
    for answered in [
        DriveType::Hunger,
        DriveType::Thirst,
        DriveType::Rest,
        DriveType::Safety,
    ] {
        let drive = drives.get_mut(answered).unwrap();
        drive.value = 0.0;
        drive.denied_ticks = 0;
    }

    assert!(drives.is_unlocked(DriveType::Reproduction));
}

/// The chains are the ones the specification gives.
#[test]
fn every_chain_is_the_one_that_was_asked_for() {
    use DriveType::*;

    assert_eq!(Sustenance.unlocked_by(), &[Hunger]);
    // Putting something by waits on being neither hungry nor parched today,
    // and on nothing else. It used to stand behind Sustenance, which meant a
    // forager could never store anything - Preparedness sat below its
    // threshold in eight agents out of eight for a whole settlement's life,
    // because food production is never answered in a people that does not
    // farm.
    assert_eq!(Preparedness.unlocked_by(), &[Hunger, Thirst]);
    assert_eq!(Luxury.unlocked_by(), &[Preparedness]);
    assert_eq!(Shelter.unlocked_by(), &[Rest, Safety]);
    assert_eq!(Protection.unlocked_by(), &[Safety, Reproduction]);
    assert_eq!(Construction.unlocked_by(), &[Social]);
    assert_eq!(Industry.unlocked_by(), &[Social]);
    assert_eq!(Utility.unlocked_by(), &[Construction, Industry]);

    // Nothing stands before the things that kill you, nor before wanting to
    // know or wanting company
    for free in [Hunger, Thirst, Rest, Safety, Curiosity, Social] {
        assert!(
            free.unlocked_by().is_empty(),
            "{free:?} should be free to build"
        );
    }
}

/// No chain eats its own tail.
#[test]
fn the_chains_do_not_go_round_in_circles() {
    fn reaches(from: DriveType, looking_for: DriveType, depth: usize) -> bool {
        if depth == 0 {
            panic!("a chain went round in a circle at {from:?}");
        }
        from.unlocked_by()
            .iter()
            .any(|before| *before == looking_for || reaches(*before, looking_for, depth - 1))
    }

    for drive_type in DriveType::all() {
        assert!(
            !reaches(drive_type, drive_type, 20),
            "{drive_type:?} stands behind itself"
        );
    }
}
