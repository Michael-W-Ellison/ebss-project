// src/core/tests/drive_leaning_tests.rs
//! Tests that a personality reaches what somebody wants.
//!
//! Traits were defined almost entirely as modifiers on how an agent *feels*
//! about what happened - "Lazy: constant happiness decrease when working",
//! "Builder: happiness from building structures" - and `core/drives.rs` did
//! not mention traits at all. Once everybody had a personality it made no
//! difference to anything: agents holding `Handy` spent 83% of their attempts
//! foraging, `Builder` 81%, `Greedy` 84%. A Builder did not build. Eight
//! worlds a side to fifteen thousand ticks came out 1.10 and 0.66 standard
//! errors apart on population and on the fertility of the ground - which is
//! to say sixty traits arrived and not one settlement noticed.
//!
//! Feeling differently about the same life is not having a different one.

use crate::core::traits::{Trait, TraitSet};
use crate::core::{DriveState, DriveType};

fn somebody_who_is(traits: &[Trait]) -> DriveState {
    let mut set = TraitSet::new();
    for held in traits {
        assert!(
            set.add_trait(*held),
            "the test means this person to be {held:?}"
        );
    }

    let mut drives = DriveState::new();
    drives.lean_towards(&set);
    drives
}

fn leaning_on(drives: &DriveState, drive_type: DriveType) -> f32 {
    drives.get(drive_type).expect("every drive exists").lean
}

fn acts_at(drives: &DriveState, drive_type: DriveType) -> f32 {
    drives
        .get(drive_type)
        .expect("every drive exists")
        .threshold
}

/// A personality with no view leaves the drives as it found them.
#[test]
fn somebody_with_no_opinions_is_an_ordinary_person() {
    let plain = somebody_who_is(&[Trait::Goth, Trait::Melancholic]);

    for drive_type in DriveType::all() {
        assert_eq!(
            leaning_on(&plain, drive_type),
            1.0,
            "{drive_type:?} should be untouched: mood is not motivation"
        );
        assert_eq!(
            acts_at(&plain, drive_type),
            drive_type.default_threshold(),
            "{drive_type:?} threshold should be untouched"
        );
    }
}

/// A lazy person and a diligent one do not want the same things.
#[test]
fn the_lazy_and_the_diligent_pull_opposite_ways() {
    let lazy = somebody_who_is(&[Trait::Lazy]);
    let diligent = somebody_who_is(&[Trait::Diligent]);

    assert!(
        leaning_on(&lazy, DriveType::Industry) < 1.0,
        "work should argue less loudly in somebody lazy"
    );
    assert!(
        leaning_on(&diligent, DriveType::Industry) > 1.0,
        "and more loudly in somebody diligent"
    );

    // And the threshold is the other half of it: how much pushing it takes
    // before they start at all
    assert!(
        acts_at(&lazy, DriveType::Industry) > acts_at(&diligent, DriveType::Industry),
        "the lazy one should need more of a push before starting: {} against {}",
        acts_at(&lazy, DriveType::Industry),
        acts_at(&diligent, DriveType::Industry)
    );
}

/// The two halves do different work, and a trait usually wants both.
#[test]
fn caring_more_and_noticing_sooner_are_different_things() {
    let coward = somebody_who_is(&[Trait::Coward]);
    let brave = somebody_who_is(&[Trait::Brave]);

    assert!(leaning_on(&coward, DriveType::Safety) > leaning_on(&brave, DriveType::Safety));
    assert!(acts_at(&coward, DriveType::Safety) < acts_at(&brave, DriveType::Safety));

    // A coward is not more frightened of a given wolf. They start running at a
    // smaller one - which is the threshold - and once running they are harder
    // to talk out of it, which is the weight.
    let ordinary = DriveType::Safety.default_threshold();
    assert!(acts_at(&coward, DriveType::Safety) < ordinary);
    assert!(acts_at(&brave, DriveType::Safety) > ordinary);
}

/// Two people with the same drive weights want different things if they are
/// different people.
#[test]
fn the_same_need_argues_differently_in_two_people() {
    let mut sociable = DriveState::new();
    let mut solitary = DriveState::new();

    // Identical individual variation: the only difference is who they are
    for drives in [&mut sociable, &mut solitary] {
        drives.get_mut(DriveType::Social).unwrap().weight = 1.0;
        drives.get_mut(DriveType::Social).unwrap().value = 0.6;
    }

    let mut outgoing = TraitSet::new();
    outgoing.add_trait(Trait::Extrovert);
    let mut retiring = TraitSet::new();
    retiring.add_trait(Trait::Introvert);

    sociable.lean_towards(&outgoing);
    solitary.lean_towards(&retiring);

    let loud = sociable.get(DriveType::Social).unwrap();
    let quiet = solitary.get(DriveType::Social).unwrap();

    assert!(
        loud.bare_urgency() > quiet.bare_urgency() * 2.0,
        "the same amount of loneliness should argue far harder in an extrovert: \
         {:.3} against {:.3}",
        loud.bare_urgency(),
        quiet.bare_urgency()
    );

    // At six tenths, the extrovert is already past the point of doing
    // something about it and the introvert is not
    assert!(loud.is_active(), "the extrovert should be looking for company");
    assert!(!quiet.is_active(), "the introvert should be content alone");
}

/// Applying a personality twice is applying it once.
///
/// This is what lets the same call serve a founder, whose personality is drawn
/// after its drives exist, and a child, whose traits are settled after it has
/// inherited its parents' drive weights.
#[test]
fn a_personality_can_be_applied_twice_without_compounding() {
    let mut traits = TraitSet::new();
    traits.add_trait(Trait::Builder);
    traits.add_trait(Trait::Greedy);
    traits.add_trait(Trait::Coward);

    let mut once = DriveState::new();
    once.lean_towards(&traits);

    let mut thrice = DriveState::new();
    thrice.lean_towards(&traits);
    thrice.lean_towards(&traits);
    thrice.lean_towards(&traits);

    for drive_type in DriveType::all() {
        assert_eq!(
            leaning_on(&once, drive_type),
            leaning_on(&thrice, drive_type),
            "{drive_type:?} leaning compounded"
        );
        assert_eq!(
            acts_at(&once, drive_type),
            acts_at(&thrice, drive_type),
            "{drive_type:?} threshold compounded"
        );
    }
}

/// A personality bends what somebody wants; it does not replace who they are.
#[test]
fn a_personality_leaves_the_individual_variation_alone() {
    let mut drives = DriveState::new();
    drives.get_mut(DriveType::Industry).unwrap().weight = 2.2;

    let mut traits = TraitSet::new();
    traits.add_trait(Trait::Lazy);
    drives.lean_towards(&traits);

    assert_eq!(
        drives.get(DriveType::Industry).unwrap().weight,
        2.2,
        "weight is what somebody is born with and hands on; the personality \
         sits on top of it rather than instead of it"
    );

    // Two equally lazy people can still differ in how much work matters to
    // them, and this one cares unusually much for a lazy person
    let mut ordinary_lazy = DriveState::new();
    ordinary_lazy.lean_towards(&traits);
    assert!(
        drives.get(DriveType::Industry).unwrap().bare_urgency()
            > ordinary_lazy.get(DriveType::Industry).unwrap().bare_urgency()
            || drives.get(DriveType::Industry).unwrap().value == 0.0
    );
}

/// No draw of traits can silence a need entirely or drown out every other.
#[test]
fn nobody_is_so_much_one_thing_that_nothing_else_matters() {
    // Everything in the pool that bears on wanting things kept about you
    let hoarder = somebody_who_is(&[Trait::Greedy, Trait::Anxious, Trait::Paranoid]);
    let indifferent = somebody_who_is(&[Trait::Ascetic]);

    for drives in [&hoarder, &indifferent] {
        for drive_type in DriveType::all() {
            let lean = leaning_on(drives, drive_type);
            assert!(
                (DriveState::LEAST_ANYBODY_CARES..=DriveState::MOST_ANYBODY_CARES)
                    .contains(&lean),
                "{drive_type:?} leaning {lean} is outside what a person can be"
            );

            let threshold = acts_at(drives, drive_type);
            assert!(
                threshold > 0.0 && threshold <= DriveState::ALWAYS_EVENTUALLY,
                "{drive_type:?} threshold {threshold} would never fire, or fires always"
            );
        }
    }
}

/// The traits that ought to bear on a drive do, and the ones that ought not,
/// do not.
#[test]
fn the_table_says_what_it_means() {
    let builder = somebody_who_is(&[Trait::Builder]);
    assert!(leaning_on(&builder, DriveType::Construction) > 1.5);
    assert_eq!(
        leaning_on(&builder, DriveType::Hunger),
        1.0,
        "being a builder is not an opinion about dinner"
    );

    let curious = somebody_who_is(&[Trait::Curious]);
    assert!(leaning_on(&curious, DriveType::Curiosity) > 1.5);

    let caretaker = somebody_who_is(&[Trait::Caretaker]);
    assert!(leaning_on(&caretaker, DriveType::Protection) > 1.4);

    let callous = somebody_who_is(&[Trait::Callous]);
    assert!(leaning_on(&callous, DriveType::Protection) < 0.7);

    let ascetic = somebody_who_is(&[Trait::Ascetic]);
    assert!(leaning_on(&ascetic, DriveType::Luxury) < 0.5);
    assert!(acts_at(&ascetic, DriveType::Luxury) > DriveType::Luxury.default_threshold());
}
