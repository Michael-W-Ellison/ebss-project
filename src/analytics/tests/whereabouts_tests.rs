// src/analytics/tests/whereabouts_tests.rs
//! Tests that an agent ends up knowing the country it lives in and forgetting
//! the country it crossed.
//!
//! "If we say that an agent seeing an area results in a 5% increase in
//! remembered general details, which can be triggered no more than once per
//! day, and that after not observing an area for a month, the amount of
//! general knowledge decreases by 5% per month, then this will allow agents to
//! remember familiar areas while forgetting areas they simply passed through."

use crate::agents::whereabouts::{Area, Whereabouts, HOW_WIDE_AN_AREA_IS};
use crate::core::DriveType;
use crate::environment::seasons::{DAYS_PER_MONTH, DAYS_PER_YEAR};

/// Twenty days of looking is enough to know a place thoroughly.
#[test]
fn living_somewhere_for_three_weeks_is_knowing_it() {
    let mut known = Whereabouts::default();
    let valley = Area { across: 3, down: 7 };

    for day in 0..20 {
        known.looked_at(valley, day);
    }

    let how_well = known.how_well_i_know(&valley);
    assert!(
        (how_well - 1.0).abs() < 1e-5,
        "twenty days at five points a day is the lot: {how_well}"
    );

    // And it does not go past the lot
    known.looked_at(valley, 21);
    assert!(known.how_well_i_know(&valley) <= 1.0);
}

/// Standing in a field all week is one day's worth of learning it.
#[test]
fn looking_twice_in_a_day_is_looking_once() {
    let mut known = Whereabouts::default();
    let field = Area { across: 0, down: 0 };

    for _ in 0..50 {
        known.looked_at(field, 4);
    }

    assert!(
        (known.how_well_i_know(&field) - Whereabouts::WHAT_A_LOOK_IS_WORTH).abs() < 1e-6,
        "fifty looks in one day is one look: {}",
        known.how_well_i_know(&field)
    );
}

/// A month's grace, then five points a month.
#[test]
fn ground_crossed_once_is_gone_by_the_summer() {
    let mut known = Whereabouts::default();
    let crossed = Area { across: 9, down: 9 };

    known.looked_at(crossed, 0);
    let first_impression = known.how_well_i_know(&crossed);
    assert!(
        (first_impression - 0.05).abs() < 1e-6,
        "one look is five points: {first_impression}"
    );

    // A month later it is untouched - the grace has not run out
    known.forget_what_has_not_been_seen(DAYS_PER_MONTH);
    assert!(
        (known.how_well_i_know(&crossed) - 0.05).abs() < 1e-6,
        "a month is the grace, not the loss"
    );

    // Two months and the five points are gone, and so is the area
    known.forget_what_has_not_been_seen(DAYS_PER_MONTH * 2);
    assert_eq!(
        known.how_well_i_know(&crossed),
        0.0,
        "a field crossed once in the spring is nothing by the summer"
    );
    assert_eq!(known.how_many_areas(), 0, "and takes up no room");
}

/// While the ground crossed once goes, the ground lived in stays.
#[test]
fn the_parish_outlasts_the_road() {
    let mut known = Whereabouts::default();
    let home = Area { across: 0, down: 0 };
    let road = Area { across: 5, down: 0 };

    for day in 0..20 {
        known.looked_at(home, day);
    }
    known.looked_at(road, 10);

    // A year of neither being visited
    known.forget_what_has_not_been_seen(DAYS_PER_YEAR);

    assert_eq!(known.how_well_i_know(&road), 0.0, "the road is gone");
    assert!(
        known.how_well_i_know(&home) > 0.4,
        "and home is still most of the way there: {}",
        known.how_well_i_know(&home)
    );

    // Twenty months of it, and even home goes
    known.forget_what_has_not_been_seen(DAYS_PER_MONTH * 21);
    assert_eq!(
        known.how_well_i_know(&home),
        0.0,
        "nothing is kept for ever on the general side"
    );
}

/// The forgetting must not depend on how often anybody asks for it.
#[test]
fn forgetting_is_charged_by_the_calendar_and_not_by_the_asking() {
    let a_year = DAYS_PER_YEAR;

    let patient = {
        let mut known = Whereabouts::default();
        let there = Area { across: 1, down: 1 };
        for day in 0..10 {
            known.looked_at(there, day);
        }
        known.forget_what_has_not_been_seen(a_year);
        known.how_well_i_know(&there)
    };

    let fretful = {
        let mut known = Whereabouts::default();
        let there = Area { across: 1, down: 1 };
        for day in 0..10 {
            known.looked_at(there, day);
        }
        for day in 10..=a_year {
            known.forget_what_has_not_been_seen(day);
        }
        known.how_well_i_know(&there)
    };

    assert!(
        (patient - fretful).abs() < 1e-5,
        "asked once or three hundred times, a year is a year: {patient} against {fretful}"
    );
}

/// An important place is a different kind of thing and keeps for five years.
#[test]
fn a_valley_that_fed_somebody_is_remembered_for_five_years() {
    let mut known = Whereabouts::default();
    let valley = Area { across: 2, down: 2 };

    known.it_answered_here(valley, DriveType::Hunger, "gather:Berries", 0);

    assert_eq!(known.how_many_important_places(), 1);
    assert!(known.is_important(&valley));

    // Four years on, with nobody ever going back, it is still known
    known.forget_what_has_not_been_seen(DAYS_PER_YEAR * 4);
    assert!(
        known.is_important(&valley),
        "there are berries in that valley and he has not forgotten it"
    );
    assert_eq!(
        known.how_well_i_know(&valley),
        0.0,
        "though he could not tell you a thing about the ground itself"
    );

    // Six years on it has finally gone
    known.forget_what_has_not_been_seen(DAYS_PER_YEAR * 6);
    assert!(!known.is_important(&valley), "five years is five years");
}

/// What it answered is what comes back when the need arises.
#[test]
fn asking_where_food_is_answers_with_the_valley_and_not_the_bush() {
    let mut known = Whereabouts::default();
    let valley = Area { across: 2, down: 2 };
    let spring = Area { across: -4, down: 1 };

    known.it_answered_here(valley, DriveType::Hunger, "gather:Berries", 0);
    known.it_answered_here(spring, DriveType::Thirst, "gather:water", 0);

    let for_food: Vec<_> = known.anywhere_that_answers(DriveType::Hunger).collect();
    assert_eq!(for_food.len(), 1);
    assert_eq!(for_food[0].area, valley);
    assert_eq!(for_food[0].what, "gather:Berries");

    // And it is an area, so what comes back is somewhere to head for rather
    // than a tile that may have been picked bare years ago
    let heading = valley.middle();
    assert_eq!(Area::holding(heading), valley);
}

/// Two needs answered in one valley are two landmarks; two berry patches in
/// one valley are one.
#[test]
fn a_valley_is_one_place_however_many_bushes_are_in_it() {
    let mut known = Whereabouts::default();
    let valley = Area { across: 2, down: 2 };

    known.it_answered_here(valley, DriveType::Hunger, "gather:Berries", 0);
    known.it_answered_here(valley, DriveType::Hunger, "gather:Nuts", 3);
    assert_eq!(known.how_many_important_places(), 1, "still one valley");

    known.it_answered_here(valley, DriveType::Thirst, "gather:water", 4);
    assert_eq!(
        known.how_many_important_places(),
        2,
        "but food and water are two different things to know about it"
    );
}

/// Areas either side of the origin are the same size as each other.
#[test]
fn the_country_west_of_nought_is_divided_like_the_country_east_of_it() {
    assert_eq!(Area::holding((0, 0, 0)), Area { across: 0, down: 0 });
    assert_eq!(
        Area::holding((HOW_WIDE_AN_AREA_IS - 1, 0, 0)),
        Area { across: 0, down: 0 }
    );
    assert_eq!(
        Area::holding((HOW_WIDE_AN_AREA_IS, 0, 0)),
        Area { across: 1, down: 0 }
    );
    assert_eq!(
        Area::holding((-1, 0, 0)),
        Area { across: -1, down: 0 },
        "one pace west of the origin is the area west of it, not the same one"
    );
    assert_eq!(
        Area::holding((-HOW_WIDE_AN_AREA_IS, 0, 0)),
        Area { across: -1, down: 0 }
    );
    assert_eq!(
        Area::holding((-HOW_WIDE_AN_AREA_IS - 1, 0, 0)),
        Area { across: -2, down: 0 }
    );
}

/// Areas are map keys, and JSON has only string keys.
#[test]
fn the_country_survives_being_written_down_and_read_back() {
    let mut known = Whereabouts::default();
    let there = Area { across: -3, down: 11 };
    for day in 0..4 {
        known.looked_at(there, day);
    }
    known.it_answered_here(there, DriveType::Hunger, "fish", 4);

    let written = serde_json::to_string(&known).expect("written");
    let read_back: Whereabouts = serde_json::from_str(&written).expect("read");

    assert!((read_back.how_well_i_know(&there) - known.how_well_i_know(&there)).abs() < 1e-6);
    assert!(read_back.is_important(&there));
}
