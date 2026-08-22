// src/core/tests/drive_context_tests.rs
//! Tests for drives that read the world instead of a clock.
//!
//! The design document specifies each drive by the conditions that raise it -
//! Safety by "hostile entity proximity, recent injury, darkness", Construction
//! by "buildable templates seen, others building, drive synergy", Sustenance by
//! "low food stockpile, crop depletion". None of that existed:
//! `base_accumulation_rate` returned one flat number per drive per tick and
//! that was the whole of it, including for `Safety => 0.02, // Spikes with
//! threats`, whose comment described the specification and whose code was a
//! constant.
//!
//! Because those drives' satisfying actions are chosen rarely, they climbed to
//! their ceiling and stayed: nine of fifteen drives measured at 1.00 and active
//! every tick after eight thousand ticks, which left the per-agent weight as
//! the only thing telling them apart.

use crate::core::{Drive, DriveContext, DriveType, Surroundings};

/// Run a drive for a while against a fixed situation and see where it settles.
fn settles_at(drive_type: DriveType, ctx: &DriveContext) -> f32 {
    let mut drive = Drive::new(drive_type);
    for _ in 0..4000 {
        drive.tick_in(ctx, true);
    }
    drive.value
}

/// An agent with everything and nothing happening wants for nothing.
#[test]
fn a_drive_with_nothing_asking_for_it_falls_quiet() {
    let contented = DriveContext {
        around: Surroundings {
            under_shelter: true,
            crop_near: 1.0,
            ..Surroundings::default()
        },
        food_put_by: 40,
        materials_put_by: 60,
        tools_to_hand: 6,
        fine_things: 4,
        armed: true,
        at_leisure: true,
        ..DriveContext::default()
    };

    for drive_type in [
        DriveType::Shelter,
        DriveType::Safety,
        DriveType::Preparedness,
        DriveType::Industry,
        DriveType::Sustenance,
        DriveType::Luxury,
        DriveType::Utility,
        DriveType::Construction,
        DriveType::Protection,
    ] {
        let settled = settles_at(drive_type, &contented);
        assert!(
            settled < 0.2,
            "{drive_type:?} settled at {settled:.2} with nothing asking for it"
        );
    }
}

/// And one with every reason climbs.
#[test]
fn a_drive_with_every_reason_climbs() {
    let wretched = DriveContext {
        around: Surroundings {
            predator_near: true,
            night: true,
            recently_hurt: true,
            crop_near: 0.0,
            somewhere_to_build: true,
            neighbours_building: true,
            children_to_mind: 2,
            child_astray: true,
            ..Surroundings::default()
        },
        food_put_by: 0,
        materials_put_by: 4,
        tools_to_hand: 0,
        broken_tools: 3,
        fine_things: 0,
        armed: false,
        exposed: true,
        chilly: true,
        shelter_pressing: 1.0,
        at_leisure: true,
    };

    for drive_type in [
        DriveType::Shelter,
        DriveType::Safety,
        DriveType::Preparedness,
        DriveType::Sustenance,
        DriveType::Luxury,
        DriveType::Utility,
        DriveType::Construction,
        DriveType::Protection,
    ] {
        let settled = settles_at(drive_type, &wretched);
        assert!(
            settled > 0.5,
            "{drive_type:?} settled at {settled:.2} with every reason to be high"
        );
    }

    // Industry is the exception, and deliberately: the specification gives it
    // "high tool durability available" as an increase condition, so wanting to
    // go and win materials is partly a question of having something to win
    // them with. The fixture above has nothing but broken tools.
    let empty_handed = settles_at(DriveType::Industry, &wretched);
    let equipped = settles_at(
        DriveType::Industry,
        &DriveContext {
            tools_to_hand: 3,
            ..wretched.clone()
        },
    );

    assert!(
        equipped > 0.5,
        "with a tool in hand and no materials, industry should be high: {equipped:.2}"
    );
    assert!(
        empty_handed < equipped,
        "and lower without one: {empty_handed:.2} against {equipped:.2}"
    );
}

/// Safety spikes with threats, which is what its comment always claimed.
#[test]
fn safety_answers_a_threat_and_not_a_clock() {
    let quiet = DriveContext::default();
    let hunted = DriveContext {
        around: Surroundings {
            predator_near: true,
            ..Surroundings::default()
        },
        ..DriveContext::default()
    };

    let mut drive = Drive::new(DriveType::Safety);

    // A long peaceful stretch leaves it flat
    for _ in 0..2000 {
        drive.tick_in(&quiet, true);
    }
    let peaceful = drive.value;
    assert!(peaceful < 0.1, "nothing happened, yet safety reached {peaceful:.2}");

    // Something with teeth turns up, and within a day it is the agent's problem
    for _ in 0..12 {
        drive.tick_in(&hunted, true);
    }
    assert!(
        drive.value > peaceful * 3.0 + 0.05,
        "a predator should move the needle inside a day: {peaceful:.2} -> {:.2}",
        drive.value
    );
}

/// Being armed and under a roof takes the edge off a threat.
#[test]
fn cover_and_a_weapon_answer_the_same_threat() {
    let exposed = DriveContext {
        around: Surroundings {
            predator_near: true,
            ..Surroundings::default()
        },
        ..DriveContext::default()
    };
    let covered = DriveContext {
        around: Surroundings {
            predator_near: true,
            under_shelter: true,
            ..Surroundings::default()
        },
        armed: true,
        ..DriveContext::default()
    };

    assert!(
        settles_at(DriveType::Safety, &covered) < settles_at(DriveType::Safety, &exposed),
        "a weapon and a roof should make the same predator less of a worry"
    );
}

/// Wanting to be out of the weather is a reason to build something. The
/// specification calls this "drive synergy" and nothing in the code had ever
/// read one drive from another.
#[test]
fn wanting_shelter_is_a_reason_to_build() {
    let with_materials = Surroundings {
        somewhere_to_build: true,
        ..Surroundings::default()
    };

    let comfortable = DriveContext {
        around: with_materials.clone(),
        materials_put_by: 20,
        shelter_pressing: 0.0,
        ..DriveContext::default()
    };
    let out_in_the_rain = DriveContext {
        around: with_materials,
        materials_put_by: 20,
        shelter_pressing: 1.0,
        ..DriveContext::default()
    };

    assert!(
        settles_at(DriveType::Construction, &out_in_the_rain)
            > settles_at(DriveType::Construction, &comfortable),
        "an agent that wants a roof should want to build one"
    );
}

/// Ground that has stopped bearing is a reason to worry about next year's
/// food even when the pack is full.
#[test]
fn failing_ground_raises_the_long_view_on_food() {
    let bearing = DriveContext {
        around: Surroundings {
            crop_near: 1.0,
            ..Surroundings::default()
        },
        food_put_by: 40,
        ..DriveContext::default()
    };
    let failing = DriveContext {
        around: Surroundings {
            crop_near: 0.0,
            ..Surroundings::default()
        },
        food_put_by: 40,
        ..DriveContext::default()
    };

    assert!(
        settles_at(DriveType::Sustenance, &failing) > settles_at(DriveType::Sustenance, &bearing),
        "crop depletion should tell even on a full stomach"
    );
}

/// A person with work to do is not thinking about ornaments.
#[test]
fn finery_waits_for_an_idle_hour() {
    let busy = DriveContext {
        fine_things: 0,
        at_leisure: false,
        ..DriveContext::default()
    };
    let idle = DriveContext {
        fine_things: 0,
        at_leisure: true,
        ..DriveContext::default()
    };

    assert!(
        settles_at(DriveType::Luxury, &busy) < settles_at(DriveType::Luxury, &idle),
        "luxury is specified to rise on idle time as well as on lack"
    );
}

/// A parent with no children has nothing to be anxious about.
#[test]
fn protection_asks_for_nothing_when_there_are_no_children() {
    let childless = DriveContext::default();
    let watchful = DriveContext {
        around: Surroundings {
            children_to_mind: 1,
            ..Surroundings::default()
        },
        ..DriveContext::default()
    };
    let alarmed = DriveContext {
        around: Surroundings {
            children_to_mind: 1,
            child_astray: true,
            ..Surroundings::default()
        },
        ..DriveContext::default()
    };

    let none = settles_at(DriveType::Protection, &childless);
    let some = settles_at(DriveType::Protection, &watchful);
    let strayed = settles_at(DriveType::Protection, &alarmed);

    assert!(none < 0.05, "no children, no anxiety: {none:.2}");
    assert!(some > none && strayed > some, "{none:.2} {some:.2} {strayed:.2}");
}

/// Hunger, thirst and tiredness still build with time whatever is going on.
#[test]
fn the_needs_of_the_body_still_run_on_the_clock() {
    let ctx = DriveContext::default();

    for drive_type in [DriveType::Hunger, DriveType::Thirst, DriveType::Rest] {
        assert!(
            drive_type.demand(&ctx).is_none(),
            "{drive_type:?} should not be reading the world"
        );

        let mut drive = Drive::new(drive_type);
        for _ in 0..200 {
            drive.tick_in(&ctx, true);
        }
        assert!(
            drive.value > 0.5,
            "{drive_type:?} should build with time: {:.2}",
            drive.value
        );
    }
}
