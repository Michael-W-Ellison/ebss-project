//! The four horizons, the winter reckoning, and what a forage costs.
//!
//! Every figure asserted here is one from the specification: day, week, month,
//! winter; extreme, high, medium-high, medium; and a forage priced by the
//! walk, the food and the getting of it.

use crate::agents::physiology;
use crate::agents::provision::*;
use crate::environment::seasons::{Season, DAYS_PER_SEASON};

/// The rungs must be in increasing order of horizon, or an agent could reach a
/// later one without having passed an earlier one.
#[test]
fn each_horizon_is_further_out_than_the_last() {
    assert!(1 < DAYS_IN_A_WEEK);
    assert!(DAYS_IN_A_WEEK < DAYS_IN_A_MONTH);
    assert!(
        (DAYS_IN_A_MONTH as f32) < how_long_a_winter_is_supposed_to_be(),
        "the month rung must sit inside the winter it precedes: {} against {}",
        DAYS_IN_A_MONTH,
        how_long_a_winter_is_supposed_to_be()
    );
}

/// The ladder, in the order it was given and with the stresses it was given.
#[test]
fn the_four_horizons_are_the_ones_that_were_asked_for() {
    use HowLongTheFoodLasts::*;
    assert_eq!(NotTheDay.stress(), 1.0, "extreme");
    assert_eq!(NotTheWeek.stress(), 0.8, "high");
    assert_eq!(NotTheMonth.stress(), 0.6, "medium-high");
    assert_eq!(NotTheWinter.stress(), 0.4, "medium");
    assert_eq!(Enough.stress(), 0.0);

    // Each horizon is further out than the last, and each is less frightening
    // to fail. That ordering is the whole point.
    let rungs = [NotTheDay, NotTheWeek, NotTheMonth, NotTheWinter, Enough];
    for pair in rungs.windows(2) {
        assert!(
            pair[0].stress() > pair[1].stress(),
            "{:?} should press harder than {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn each_rung_is_reached_at_the_horizon_it_names() {
    let winter = DAYS_PER_SEASON as f32;
    // Deep in winter, so the winter rung is live
    let near = 1.0;

    assert_eq!(
        HowLongTheFoodLasts::reckon(0.5, winter, near),
        HowLongTheFoodLasts::NotTheDay
    );
    assert_eq!(
        HowLongTheFoodLasts::reckon(3.0, winter, near),
        HowLongTheFoodLasts::NotTheWeek
    );
    assert_eq!(
        HowLongTheFoodLasts::reckon(DAYS_IN_A_MONTH as f32 - 1.0, winter, near),
        HowLongTheFoodLasts::NotTheMonth
    );
    // Past the month but not enough to see a twenty-four day winter out
    assert_eq!(
        HowLongTheFoodLasts::reckon(DAYS_IN_A_MONTH as f32 + 1.0, winter, near),
        HowLongTheFoodLasts::NotTheWinter
    );
    assert_eq!(
        HowLongTheFoodLasts::reckon(winter + 1.0, winter, near),
        HowLongTheFoodLasts::Enough
    );
}

/// Nobody in spring is uneasy about a winter three seasons off.
#[test]
fn the_winter_rung_only_bites_as_the_winter_comes_on() {
    let winter = DAYS_PER_SEASON as f32;
    let plenty_for_a_month = DAYS_IN_A_MONTH as f32 + 1.0;

    let spring = WhatIsPutBy::reckon(
        plenty_for_a_month * 1440.0,
        1440.0,
        winter,
        Season::Spring.first_day() + 2,
    );
    let late_autumn = WhatIsPutBy::reckon(
        plenty_for_a_month * 1440.0,
        1440.0,
        winter,
        Season::Winter.first_day() - 2,
    );

    assert!(
        spring.stress() < late_autumn.stress(),
        "the same larder should be easier in spring ({:.2}) than in late autumn ({:.2})",
        spring.stress(),
        late_autumn.stress()
    );
}

#[test]
fn the_winter_gets_nearer_through_the_year() {
    let spring = how_near_winter_is(Season::Spring.first_day() + 1);
    let summer = how_near_winter_is(Season::Summer.first_day() + 1);
    let autumn = how_near_winter_is(Season::Fall.first_day() + 1);
    let winter = how_near_winter_is(Season::Winter.first_day() + 1);

    assert_eq!(spring, 0.0, "a winter three seasons off is nothing to anybody");
    assert!(summer <= autumn, "summer {summer} autumn {autumn}");
    assert!(autumn < winter, "autumn {autumn} winter {winter}");
    assert_eq!(winter, 1.0, "and in winter it is here");
}

/// An agent lays in against what it actually eats, not a number from a table.
#[test]
fn what_a_winter_wants_follows_what_this_body_eats() {
    let winter = DAYS_PER_SEASON as f32;
    let small = WhatIsPutBy::reckon(0.0, 500.0, winter, 0);
    let large = WhatIsPutBy::reckon(0.0, 1440.0, winter, 0);

    assert!(small.what_a_winter_wants() < large.what_a_winter_wants());
    assert_eq!(large.what_a_winter_wants(), 1440.0 * winter);
    // With nothing put by, the whole of it is still wanting
    assert_eq!(large.still_short_by(), large.what_a_winter_wants());
}

/// Nobody is born knowing how long a winter is; it is counted.
#[test]
fn a_winter_is_counted_rather_than_known() {
    let mut seen = WintersSeen::default();
    assert_eq!(
        seen.how_long_a_winter_lasts(),
        how_long_a_winter_is_supposed_to_be(),
        "with no winter behind it, an agent uses the calendar's answer"
    );

    // Live through one short winter of ten days, then out the other side
    for day in 0..10 {
        seen.another_day(Season::Winter, day);
    }
    seen.another_day(Season::Spring, 10);

    assert_eq!(seen.winters, 1);
    assert_eq!(seen.days_counted, 10);
    assert_eq!(
        seen.how_long_a_winter_lasts(),
        10.0,
        "and after one, its own count"
    );
}

#[test]
fn the_same_day_is_not_counted_twice() {
    let mut seen = WintersSeen::default();
    for _ in 0..12 {
        seen.another_day(Season::Winter, 80);
    }
    assert_eq!(seen.days_counted, 1, "twelve turns is one day");
}

/// The walk, the food, and the getting of it.
#[test]
fn a_forage_is_priced_by_the_walk_and_the_food() {
    let ordinary = physiology::how_rich_this_food_is(25.0);

    // Further costs more
    assert!(
        what_foraging_costs(20, ordinary) > what_foraging_costs(2, ordinary),
        "a patch across the valley should cost more than the bush at the door"
    );

    // Denser food takes more getting: a carcass wants butchering, greens do not
    let greens = physiology::how_rich_this_food_is(6.0);
    let fat = physiology::how_rich_this_food_is(80.0);
    assert!(what_foraging_costs(5, fat) > what_foraging_costs(5, greens));

    // And nothing is free, even underfoot
    assert!(what_foraging_costs(0, greens) > 0.0);

    // The walk is counted both ways
    let near = what_foraging_costs(0, ordinary);
    let ten_off = what_foraging_costs(10, ordinary);
    assert!(
        (ten_off - near - 10.0 * WHAT_A_PACE_COSTS * 2.0).abs() < 1e-4,
        "ten paces off should cost twenty paces of walking"
    );
}

/// A body that has just eaten is not short of supper.
#[test]
fn what_is_in_the_body_counts_towards_the_day() {
    let winter = DAYS_PER_SEASON as f32;
    let a_days_food = physiology::UNITS_BURNED_IN_AN_ORDINARY_DAY;

    let empty = WhatIsPutBy::reckon(0.0, a_days_food, winter, 0);
    assert_eq!(empty.rung, HowLongTheFoodLasts::NotTheDay);
    assert_eq!(empty.stress(), 1.0, "nothing for tonight is extreme");

    let fed = WhatIsPutBy::reckon(a_days_food * 2.0, a_days_food, winter, 0);
    assert_ne!(fed.rung, HowLongTheFoodLasts::NotTheDay);
    assert!(fed.stress() < empty.stress());
}

/// A settlement cannot lay anything by if a trip brings back one berry.
///
/// This is the arithmetic behind the yield, rather than a test of the gather
/// itself. A body eats three portions a day, so a trip that brings back one
/// portion is a third of a day's food for a day's walking - a settlement
/// gathering like that has no surplus, ever, and the Preparedness drive has
/// nothing to bury however much it wants a winter store.
#[test]
fn a_trip_has_to_bring_back_more_than_a_meal() {
    let a_days_food = physiology::UNITS_BURNED_IN_AN_ORDINARY_DAY;

    // One portion a trip, which is what food used to yield
    let one_portion = UNITS_IN_ONE_STORED_ITEM;
    assert!(
        one_portion < a_days_food,
        "one portion is not a day's food, so a trip a turn cannot feed anybody"
    );

    // An armful - three to six - is a day or two, which leaves something over
    let armful = UNITS_IN_ONE_STORED_ITEM * 3.0;
    assert!(
        armful >= a_days_food,
        "an armful should be at least a day's food: {armful} against {a_days_food}"
    );
}
