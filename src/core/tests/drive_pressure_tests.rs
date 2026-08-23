// src/core/tests/drive_pressure_tests.rs
//! Tests for a need that will not be put off for ever.
//!
//! A drive used to build at a flat rate and argue for the agent's attention
//! with a flat `value * weight`, so a need that had gone unanswered for three
//! days made exactly the same case as one that had gone unanswered for an
//! hour. An agent could be starving beside a stripped field and go on tilling
//! it, because nothing in the model knew the difference between a need and a
//! long-standing need.
//!
//! A drive now remembers how long it has been asking. That memory multiplies
//! both how fast it builds and how loudly it argues, so being ignored is
//! self-correcting: the longer a settlement fails to feed somebody, the more
//! completely that person's behaviour is taken over by getting fed.

use crate::core::{Drive, DriveType};

/// A drive that is being answered presses no harder than its face value.
#[test]
fn an_answered_drive_presses_no_harder_than_it_looks() {
    let mut drive = Drive::new(DriveType::Hunger);

    for _ in 0..30 {
        drive.tick();
        // Answered before it ever gets over the threshold
        drive.partial_satisfy(0.05);
    }

    assert_eq!(drive.denied_ticks(), 0, "it was never left asking");
    assert_eq!(drive.pressure(), 1.0);
    assert!((drive.urgency() - drive.bare_urgency()).abs() < 1e-6);
}

/// A drive left asking builds faster the longer it waits.
#[test]
fn a_denied_drive_builds_faster_the_longer_it_waits() {
    fn value_after(ticks: u32, answered: bool) -> f32 {
        let mut drive = Drive::new(DriveType::Rest);
        for _ in 0..ticks {
            drive.tick();
            if answered {
                drive.satisfy();
            }
        }
        drive.value
    }

    let ignored = value_after(120, false);
    let flat = 120.0 * DriveType::Rest.base_accumulation_rate();

    assert!(
        ignored > flat,
        "a drive ignored for 120 ticks should have outrun a flat rate: {ignored:.3} against {flat:.3}"
    );
    assert!(value_after(120, true) < 0.05, "an answered drive stays down");
}

/// The longer it is denied, the more it dominates what the agent does.
#[test]
fn a_long_denied_need_takes_the_agent_over() {
    let mut nagging = Drive::new(DriveType::Hunger);
    let mut fresh = Drive::new(DriveType::Hunger);

    nagging.value = 0.75;
    fresh.value = 0.75;

    // One has been asking for three days of world time; the other just started
    for _ in 0..40 {
        nagging.tick();
        nagging.value = 0.75; // hold it steady so only the waiting differs
    }

    assert!(
        nagging.urgency() > fresh.urgency() * 2.0,
        "three days of being ignored should more than double the case a need makes: \
         {:.2} against {:.2}",
        nagging.urgency(),
        fresh.urgency()
    );
}

/// It does not grow without limit: one old grievance must not outrank an
/// immediate threat for ever.
#[test]
fn the_pressure_is_bounded() {
    let mut drive = Drive::new(DriveType::Hunger);
    drive.value = 1.0;

    for _ in 0..10_000 {
        drive.tick();
    }

    assert!(
        drive.pressure() <= 4.0,
        "pressure ran away to {}",
        drive.pressure()
    );
}

/// Being fed takes the weight off, but not all at once.
#[test]
fn a_meal_takes_the_edge_off_without_erasing_the_memory() {
    let mut drive = Drive::new(DriveType::Hunger);
    drive.value = 0.9;

    for _ in 0..60 {
        drive.tick();
        drive.value = 0.9;
    }

    let starving = drive.denied_ticks();
    assert!(starving >= 60);

    // A meal that takes it below the threshold
    drive.partial_satisfy(0.5);

    assert!(
        drive.denied_ticks() < starving,
        "a meal should relieve the pressure"
    );
    assert!(
        drive.denied_ticks() > 0,
        "somebody who has been starving stays wary for a while"
    );

    // A full meal clears it
    drive.satisfy();
    assert_eq!(drive.denied_ticks(), 0);
}
