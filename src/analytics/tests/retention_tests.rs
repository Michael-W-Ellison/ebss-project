// src/analytics/tests/retention_tests.rs
//! **How long a place is kept depends on how it was come by.**
//!
//! Every remembered place used to fade at one rate, bent only by what *kind*
//! of place it was - so the clay bank a potter had dug out every week for a
//! year was forgotten at exactly the rate of one he had glimpsed over
//! somebody's shoulder. `STILL_KNOWS_WHAT_IT_WAS` carried a note saying as
//! much: "making it depend on *how* the place was learned is the next piece,
//! not this one."
//!
//! Three footings, and one sentence each. Watching somebody work is "so-and-so
//! will find this useful, even if I do not" - a week. Knowing what the stuff
//! is for is "I find this useful, though I have not worked it" - a year.
//! Having had it out of the ground is "I find this useful and I have
//! exploited it" - three years, and another for every trip back.
//!
//! And one brake on all of it, which is the half that keeps the rest honest:
//! a bramble is a fact about last autumn. A man walking half a day in March
//! on a patch he stripped in October has been misled by his own good memory.

use crate::core::memory::{
    HowIKnow, HowSteady, Memory, SpatialMemory, SpatialMemoryType,
};
use crate::environment::seasons::{DAYS_PER_YEAR, TICKS_PER_DAY};

/// A named place of the given standing, so the importance band is Normal and
/// the footing is the only thing under test.
fn a_place_known(how_i_know: HowIKnow, how_steady: HowSteady) -> SpatialMemory {
    let mut place = SpatialMemory::new(SpatialMemoryType::Resource, (30, 25, 0), 0);
    place.what_it_is = Some("clay".to_string());
    place.how_i_know = how_i_know;
    place.how_steady = how_steady;
    place
}

/// How many days until he could no longer bring the place to mind.
///
/// `recall_locations` wants confidence above 0.3, so that is what "remembered"
/// means everywhere it matters.
fn days_until_forgotten(mut place: SpatialMemory) -> u32 {
    let mut days = 0;
    while place.confidence > 0.3 {
        place.forget_a_little(TICKS_PER_DAY);
        days += 1;
        if days > DAYS_PER_YEAR * 12 {
            panic!("a place that is never forgotten at all");
        }
    }
    days
}

/// Near enough, on a scale where the units are days and the targets are years.
fn about(got: u32, wanted: u32) {
    let slack = (wanted / 20).max(2);
    assert!(
        got.abs_diff(wanted) <= slack,
        "wanted about {wanted} days, got {got}"
    );
}

/// **A week, a year, three years.** The three numbers the specification asks
/// for, read off the decay rather than asserted about the table.
#[test]
fn the_three_footings_are_a_week_a_year_and_three_years() {
    about(
        days_until_forgotten(a_place_known(HowIKnow::SawItUsed, HowSteady::Steady)),
        7,
    );
    // And the fourth, which is the absence of the other three: a place he
    // noticed and nothing more, held for the fortnight every remembered place
    // in this model was held for before any of this.
    about(
        days_until_forgotten(a_place_known(HowIKnow::JustNoticedIt, HowSteady::Steady)),
        14,
    );
    about(
        days_until_forgotten(a_place_known(HowIKnow::UsedThisKind, HowSteady::Steady)),
        DAYS_PER_YEAR,
    );
    about(
        days_until_forgotten(a_place_known(
            HowIKnow::WorkedThisPlace(1),
            HowSteady::Steady,
        )),
        DAYS_PER_YEAR * 3,
    );
}

/// And every trip back buys another year, until it stops being worth counting.
#[test]
fn a_trip_back_buys_another_year() {
    let after = |trips| {
        days_until_forgotten(a_place_known(
            HowIKnow::WorkedThisPlace(trips),
            HowSteady::Steady,
        ))
    };

    about(after(1), DAYS_PER_YEAR * 3);
    about(after(2), DAYS_PER_YEAR * 4);
    about(after(3), DAYS_PER_YEAR * 5);

    // And past the cap it stops moving, because a settlement lasts about a
    // year and a memory already good for six is good for ever.
    let capped = after(HowIKnow::HOW_MANY_TRIPS_STILL_TELL);
    assert_eq!(
        capped,
        after(HowIKnow::HOW_MANY_TRIPS_STILL_TELL + 40),
        "counting past the cap changed something nothing could tell apart"
    );
}

/// **A bramble is a fact about last autumn.**
///
/// The brake, and the half that keeps the rest honest. However well a man
/// knows a berry patch, it is not a berry patch in March, and a memory good
/// for three years would send him half a day's walk to bare ground.
#[test]
fn a_turning_place_is_never_kept_longer_than_an_ordinary_one() {
    let seam = days_until_forgotten(a_place_known(
        HowIKnow::WorkedThisPlace(3),
        HowSteady::Steady,
    ));
    let patch = days_until_forgotten(a_place_known(
        HowIKnow::WorkedThisPlace(3),
        HowSteady::Turns,
    ));

    assert!(
        patch < seam,
        "a bramble he stripped three times was held as long as a stone seam"
    );

    // Held at the ordinary rate instead: a fortnight, which is what every
    // remembered place in this model was worth before any of this.
    let mut confidence = 1.0f32;
    let mut ordinary = 0;
    while confidence > 0.3 {
        confidence -=
            TICKS_PER_DAY as f32 * SpatialMemory::HOW_FAST_AN_ORDINARY_PLACE_IS_FORGOTTEN;
        ordinary += 1;
    }
    about(patch, ordinary);
}

/// And nothing is *shortened* by turning.
///
/// Two systems shortening the same memory is how a winter store came to be
/// forgotten a fortnight after it was buried. A patch he has only watched
/// somebody else strip is already a week; being a bramble does not make it
/// three days.
#[test]
fn turning_never_makes_a_place_go_faster_than_its_footing() {
    let watched_seam = days_until_forgotten(a_place_known(HowIKnow::SawItUsed, HowSteady::Steady));
    let watched_patch = days_until_forgotten(a_place_known(HowIKnow::SawItUsed, HowSteady::Turns));

    assert_eq!(
        watched_seam, watched_patch,
        "a week became less than a week for want of the ground being solid"
    );
}

/// **A pit a man dug is not a source, and its band was measured where it is.**
///
/// `Storage`, `Water`, `Danger` and `Shelter` are not places you take things
/// out of - there is no trip back to count and no season to turn. Letting the
/// footing near them would have the whole of this multiplying a band that was
/// set by measuring a settlement starving thirty paces from its own larder.
#[test]
fn a_larder_is_not_on_the_footings_at_all() {
    let mut pit = SpatialMemory::new(SpatialMemoryType::Storage, (30, 25, 0), 0);
    let plain = days_until_forgotten(pit.clone());

    pit.how_i_know = HowIKnow::WorkedThisPlace(4);
    pit.how_steady = HowSteady::Turns;

    assert_eq!(
        days_until_forgotten(pit),
        plain,
        "the footing reached a larder, whose band was measured where it stands"
    );
}

/// A footing is never demoted by a weaker sighting.
///
/// Watching somebody else dig a bank you have dug yourself does not turn it
/// back into hearsay.
#[test]
fn a_firmer_footing_is_not_given_up_for_a_weaker_one() {
    let mut place = a_place_known(HowIKnow::WorkedThisPlace(2), HowSteady::Steady);

    place.i_know_this_at_least_this_well(HowIKnow::SawItUsed);
    assert_eq!(place.how_i_know, HowIKnow::WorkedThisPlace(2));

    place.i_know_this_at_least_this_well(HowIKnow::UsedThisKind);
    assert_eq!(place.how_i_know, HowIKnow::WorkedThisPlace(2));

    // And a firmer one is taken.
    let mut heard_of = a_place_known(HowIKnow::SawItUsed, HowSteady::Steady);
    heard_of.i_know_this_at_least_this_well(HowIKnow::UsedThisKind);
    assert_eq!(heard_of.how_i_know, HowIKnow::UsedThisKind);
}

/// Having it out of the ground is the one event that moves a place up.
#[test]
fn working_a_place_is_what_earns_the_firmest_footing() {
    let mut memory = Memory::new();
    memory.remember_what_kind_of_place_this_is(
        SpatialMemoryType::Resource,
        (30, 25, 0),
        Some("clay".to_string()),
        9,
        HowIKnow::UsedThisKind,
        HowSteady::Steady,
    );

    assert!(memory.i_have_worked_this_place(SpatialMemoryType::Resource, (30, 25, 0)));
    assert_eq!(
        memory.spatial_memories[0].how_i_know,
        HowIKnow::WorkedThisPlace(1)
    );

    // And again, which is another year.
    assert!(memory.i_have_worked_this_place(SpatialMemoryType::Resource, (30, 25, 0)));
    assert_eq!(
        memory.spatial_memories[0].how_i_know,
        HowIKnow::WorkedThisPlace(2)
    );

    // A patch he strips without ever having filed it has nothing to promote,
    // which is the ordinary case and not a failure.
    assert!(!memory.i_have_worked_this_place(SpatialMemoryType::Resource, (99, 99, 0)));
}

/// Every footing is accounted for, so adding one fails to compile here.
#[test]
fn every_footing_says_how_long_it_holds() {
    for footing in HowIKnow::all() {
        let named = match footing {
            HowIKnow::SawItUsed => "watched",
            HowIKnow::JustNoticedIt => "noticed",
            HowIKnow::UsedThisKind => "known",
            HowIKnow::WorkedThisPlace(_) => "worked",
        };
        assert!(!named.is_empty());
        assert!(
            footing.how_fast_this_fades() > 0.0,
            "{footing:?} is never forgotten at all"
        );
    }

    // Firmer footings fade slower, which is the whole ordering.
    assert!(
        HowIKnow::WorkedThisPlace(1).how_fast_this_fades()
            < HowIKnow::UsedThisKind.how_fast_this_fades()
    );
    assert!(
        HowIKnow::UsedThisKind.how_fast_this_fades() < HowIKnow::JustNoticedIt.how_fast_this_fades()
    );
    assert!(
        HowIKnow::JustNoticedIt.how_fast_this_fades() < HowIKnow::SawItUsed.how_fast_this_fades()
    );
}

/// **A writer that says nothing changes nothing.**
///
/// The guard on the default. Every `remember_location` caller in this model -
/// a pit a man dug, a roof he worked on, a place that hurt him - files a place
/// without a word about how it was learned, and none of them should have their
/// memories shortened for it. Defaulting to the weakest of the specification's
/// footings did exactly that, and halved the life of every berry patch
/// anybody walked past.
#[test]
fn a_place_filed_with_nothing_said_fades_at_the_rate_it_always_did() {
    let plain = SpatialMemory::new(SpatialMemoryType::Food, (30, 25, 0), 0);
    assert_eq!(plain.how_i_know, HowIKnow::JustNoticedIt);
    assert_eq!(HowIKnow::JustNoticedIt.how_fast_this_fades(), 1.0);
}

// **The category tier: the name goes, the sort outlasts it, the place
// outlasts that.**
//
// "Flax at that field edge" becomes "fibre in that valley" becomes "somewhere
// over there was worth a look" becomes nothing - the specification's own
// worked example, on one clock so the three tiers cannot disagree about how
// long ago this was.
//
// It is read by nothing in the decision layer. The reader it was built for
// was telling somebody where things are, and that channel cost 6.3% of all
// person-days and two thirds of the settlements that got out of their first
// winter, both blocks agreeing. See ISSUES_FOUND #196.

use crate::environment::making::{what_sort_of_thing_is_it, EVERY_FAMILY, WHAT_SORT_OF_THING};

/// **The two tables cannot come apart.**
///
/// A family is what will stand in for what in a making; a sort is what a man
/// calls the stuff with the name gone. They answer different questions, and a
/// family whose members fell into two sorts would mean a man could no longer
/// say that flint and stone were the same sort of thing while still knowing
/// he could knap either.
#[test]
fn every_family_sits_inside_one_sort() {
    for family in EVERY_FAMILY {
        let sorts: Vec<_> = family
            .iter()
            .map(|member| {
                what_sort_of_thing_is_it(member)
                    .unwrap_or_else(|| panic!("{member} is in a family and in no sort"))
            })
            .collect();

        assert!(
            sorts.windows(2).all(|pair| pair[0] == pair[1]),
            "the family {family:?} is spread across sorts {sorts:?}"
        );
    }
}

/// And nothing is in two sorts, or a name would mean two things at once.
#[test]
fn nothing_belongs_to_two_sorts() {
    for (sort, members) in WHAT_SORT_OF_THING {
        for member in members.iter() {
            assert_eq!(
                what_sort_of_thing_is_it(member),
                Some(*sort),
                "{member} is listed under {sort} and answers to something else"
            );
        }
    }
}

/// The name goes, then the sort, then the place.
#[test]
fn a_place_fades_from_a_name_to_a_sort_to_nothing() {
    let mut remembered = SpatialMemory::new(SpatialMemoryType::Resource, (30, 25, 0), 0);
    remembered.what_it_is = Some("flax".to_string());

    assert_eq!(remembered.what_i_could_name_it(), Some("flax"));
    assert_eq!(remembered.what_sort_i_could_say_it_was(), Some("fibre"));

    // Past the name, still inside the sort.
    remembered.confidence = SpatialMemory::STILL_KNOWS_WHAT_IT_WAS - 0.01;
    assert_eq!(remembered.what_i_could_name_it(), None);
    assert_eq!(
        remembered.what_sort_i_could_say_it_was(),
        Some("fibre"),
        "he lost the sort at the same moment he lost the name"
    );

    // Past the sort, still knows the valley was worth something.
    remembered.confidence = SpatialMemory::STILL_KNOWS_THE_SORT_OF_THING - 0.01;
    assert_eq!(remembered.what_sort_i_could_say_it_was(), None);
    assert!(
        remembered.confidence > 0.3,
        "the place should outlast the sort, not go with it"
    );
}

/// A thing of no sort has no middle tier, and that is not a fault.
#[test]
fn a_thing_of_no_sort_has_no_middle_tier() {
    let mut odd = SpatialMemory::new(SpatialMemoryType::Resource, (30, 25, 0), 0);
    odd.what_it_is = Some("a thing nobody has a word for".to_string());

    assert_eq!(
        odd.what_i_could_name_it(),
        Some("a thing nobody has a word for")
    );
    assert_eq!(odd.what_sort_i_could_say_it_was(), None);
}
