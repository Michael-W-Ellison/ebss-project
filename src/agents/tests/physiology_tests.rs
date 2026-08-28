//! The physiology, checked against the numbers it was specified in.
//!
//! Every figure asserted here is one from the specification rather than one
//! read back off the implementation: three days to die of thirst, three weeks
//! to starve, six hours to empty a stomach, a day in the gut, 600 units, 1440
//! a day. If a constant drifts, these say so.

use crate::agents::physiology::*;

/// A turn is two hours, and the body's clock does not care that it is.
#[test]
fn a_turn_is_two_hours_of_living() {
    assert_eq!(MINUTES_PER_DAY, 1440);
    assert_eq!(
        MINUTES_PER_TURN,
        MINUTES_PER_DAY / crate::environment::seasons::TICKS_PER_DAY
    );
    // Twelve turns to the day, so two hours to the turn
    assert_eq!(MINUTES_PER_TURN, 120);
}

#[test]
fn the_clocks_are_the_ones_that_were_asked_for() {
    assert_eq!(MINUTES_TO_DIE_OF_THIRST, 4_320, "three days");
    assert_eq!(MINUTES_TO_STARVE, 30_240, "three weeks");
    assert_eq!(MINUTES_TO_DIGEST, 1_440, "a day in the gut");
    assert_eq!(MINUTES_FOR_A_DRINK_TO_TELL, 20);
    assert_eq!(STOMACH_CAPACITY, 600.0);
    assert_eq!(UNITS_BURNED_IN_AN_ORDINARY_DAY, 1440.0);
}

/// Three full meals is more than a day needs, which is why an agent has no
/// reason to eat until full.
#[test]
fn three_full_stomachs_is_more_than_a_day_wants() {
    assert!(STOMACH_CAPACITY * 3.0 > UNITS_BURNED_IN_AN_ORDINARY_DAY);
    assert_eq!(STOMACH_CAPACITY * 3.0, 1800.0);
    // And three ordinary portions is exactly a day
    assert_eq!(UNITS_IN_A_PORTION * 3.0, UNITS_BURNED_IN_AN_ORDINARY_DAY);
}

/// The gastric schedule, at every boundary the specification names.
#[test]
fn the_stomach_empties_on_the_stated_schedule() {
    let full = 600.0;
    // Nothing for the first half hour
    assert_eq!(share_of_a_meal_gone_by(0), 0.0);
    assert_eq!(share_of_a_meal_gone_by(30), 0.0);

    // An eighth every half hour to three hours
    let eighth = 1.0 / 8.0;
    for (minutes, eighths) in [(60, 1), (90, 2), (120, 3), (150, 4), (180, 5)] {
        let gone = share_of_a_meal_gone_by(minutes);
        assert!(
            (gone - eighth * eighths as f32).abs() < 1e-5,
            "at {minutes} minutes expected {eighths}/8 gone, got {gone}"
        );
    }

    // "leaving the stomach with 525 food" after the first eighth
    assert!((full - full * share_of_a_meal_gone_by(60) - 525.0).abs() < 0.01);
    // "2.5 hr to 3 hr another 1/8th leaving 225 food"
    assert!((full - full * share_of_a_meal_gone_by(180) - 225.0).abs() < 0.01);

    // Then an eighth an hour, and empty at six hours
    for (minutes, eighths) in [(240, 6), (300, 7), (360, 8)] {
        let gone = share_of_a_meal_gone_by(minutes);
        assert!(
            (gone - eighth * eighths as f32).abs() < 1e-5,
            "at {minutes} minutes expected {eighths}/8 gone, got {gone}"
        );
    }
    assert_eq!(share_of_a_meal_gone_by(1000), 1.0);
}

/// Advancing two hours at a time must land where advancing a minute at a time
/// lands, or the coarse decision loop would change the physiology.
#[test]
fn the_step_size_does_not_change_the_body() {
    let mut coarse = Physiology::new();
    let mut fine = Physiology::new();
    coarse.eat(600.0, 1.0);
    fine.eat(600.0, 1.0);

    for _ in 0..12 {
        coarse.advance(120, 5.0);
    }
    for _ in 0..1440 {
        fine.advance(1, 5.0);
    }

    assert!(
        (coarse.in_the_stomach() - fine.in_the_stomach()).abs() < 0.5,
        "stomach: coarse {} fine {}",
        coarse.in_the_stomach(),
        fine.in_the_stomach()
    );
    assert!(
        (coarse.hydration - fine.hydration).abs() < 1e-3,
        "hydration: coarse {} fine {}",
        coarse.hydration,
        fine.hydration
    );
    assert!(
        (coarse.reserve - fine.reserve).abs() < 1.0,
        "reserve: coarse {} fine {}",
        coarse.reserve,
        fine.reserve
    );
}

#[test]
fn a_meal_is_gone_from_the_stomach_in_six_hours() {
    let mut body = Physiology::new();
    body.eat(600.0, 1.0);
    assert!((body.in_the_stomach() - 600.0).abs() < 0.01);

    // Three turns is six hours
    for _ in 0..3 {
        body.advance(120, 5.0);
    }
    assert!(
        body.in_the_stomach() < 0.5,
        "stomach still holds {}",
        body.in_the_stomach()
    );
    assert!(body.in_the_gut() > 590.0, "gut holds {}", body.in_the_gut());
}

#[test]
fn what_leaves_the_stomach_is_worth_nothing_for_a_day() {
    let mut body = Physiology::new();
    let started = body.reserve;
    body.eat(600.0, 1.0);

    // Six hours: all of it out of the stomach, none of it counted yet
    for _ in 0..3 {
        body.advance(120, 5.0);
    }
    assert!(body.reserve < started, "the body has only burned so far");

    // A day after the last of it left the stomach, all of it has counted
    for _ in 0..12 {
        body.advance(120, 5.0);
    }
    assert!(body.in_the_gut() < 1.0, "gut still holds {}", body.in_the_gut());
}

/// A drink does not tell for twenty minutes.
#[test]
fn a_drink_takes_twenty_minutes_to_tell() {
    let mut body = Physiology::new();
    body.hydration = 0.5;
    body.drink(0.3);
    // Ten minutes is not enough
    body.advance(10, 0.0);
    assert!(body.hydration < 0.55, "hydration {}", body.hydration);
    // Twenty is
    body.advance(15, 0.0);
    assert!(body.hydration > 0.75, "hydration {}", body.hydration);
}

/// Three days from full to dead, at an ordinary level of activity.
#[test]
fn three_days_without_water_kills_an_adult() {
    let mut body = Physiology::new();
    // An ordinary turn costs five energy, which is the middle of the range
    let mut turns = 0;
    while !body.died_of_thirst() && turns < 10_000 {
        body.advance(MINUTES_PER_TURN, 5.0);
        turns += 1;
    }
    let days = turns as f32 * MINUTES_PER_TURN as f32 / MINUTES_PER_DAY as f32;
    assert!(
        (2.5..3.6).contains(&days),
        "died of thirst after {days} days, wanted about three"
    );
}

/// Three weeks from full to dead.
#[test]
fn three_weeks_without_food_kills_an_adult() {
    let mut body = Physiology::new();
    let mut turns = 0;
    while !body.starved() && turns < 100_000 {
        body.advance(MINUTES_PER_TURN, 5.0);
        turns += 1;
    }
    let days = turns as f32 * MINUTES_PER_TURN as f32 / MINUTES_PER_DAY as f32;
    assert!(
        (18.0..24.0).contains(&days),
        "starved after {days} days, wanted about twenty-one"
    );
}

/// The capability bands, exactly as stated.
#[test]
fn going_short_of_water_costs_a_quarter_at_a_time() {
    let mut body = Physiology::new();
    for (hydration, expected) in [
        (1.00, 1.00),
        (0.76, 1.00),
        (0.75, 0.75),
        (0.51, 0.75),
        (0.50, 0.50),
        (0.26, 0.50),
        (0.25, 0.25),
        (0.01, 0.25),
    ] {
        body.hydration = hydration;
        assert_eq!(
            body.capability(),
            expected,
            "at {hydration} hydration expected {expected}"
        );
    }
}

/// A body cannot put three weeks of hunger right in one sitting.
#[test]
fn a_full_stomach_will_not_take_more() {
    let mut body = Physiology::new();
    body.reserve = 100.0; // starving
    let first = body.eat(600.0, 1.0);
    assert!((first - 600.0).abs() < 0.01);
    let second = body.eat(600.0, 1.0);
    assert_eq!(second, 0.0, "there is no room and it should not go down");
}

/// Working makes a body burn and sweat faster than resting does.
#[test]
fn work_costs_more_than_rest() {
    assert!(what_the_work_costs(0.0) < what_the_work_costs(5.0));
    assert!(what_the_work_costs(5.0) < what_the_work_costs(20.0));

    let mut resting = Physiology::new();
    let mut working = Physiology::new();
    for _ in 0..12 {
        resting.advance(MINUTES_PER_TURN, 0.0);
        working.advance(MINUTES_PER_TURN, 20.0);
    }
    assert!(
        working.hydration < resting.hydration,
        "working {} resting {}",
        working.hydration,
        resting.hydration
    );
    assert!(
        working.reserve < resting.reserve,
        "working {} resting {}",
        working.reserve,
        resting.reserve
    );
}

/// A day's ordinary work costs about a day's ordinary food.
#[test]
fn an_ordinary_day_burns_about_what_an_ordinary_day_holds() {
    let mut body = Physiology::new();
    let started = body.reserve;
    for _ in 0..12 {
        body.advance(MINUTES_PER_TURN, 5.0);
    }
    let burned = started - body.reserve;
    assert!(
        (1000.0..1900.0).contains(&burned),
        "burned {burned} in a day, wanted about {UNITS_BURNED_IN_AN_ORDINARY_DAY}"
    );
}

/// Three ordinary portions a day holds a body steady.
#[test]
fn three_meals_a_day_keeps_a_body_level() {
    let mut body = Physiology::new();
    let started = body.reserve;
    // Ten days, eating at the first, fifth and ninth turn of each
    for _ in 0..10 {
        for turn in 0..12 {
            if turn == 0 || turn == 4 || turn == 8 {
                body.eat(UNITS_IN_A_PORTION, 1.0);
            }
            body.advance(MINUTES_PER_TURN, 5.0);
        }
    }
    let lost = started - body.reserve;
    assert!(
        lost < RESERVE_OF_A_GROWN_BODY * 0.25,
        "lost {lost} of {started} over ten days on three meals a day"
    );
    assert!(!body.starved());
}

/// Hunger is felt about five hours after eating, which is what puts three
/// meals in a day rather than one or ten.
#[test]
fn hunger_comes_on_about_five_hours_after_a_meal() {
    let mut body = Physiology::new();
    body.eat(UNITS_IN_A_PORTION, 1.0);
    body.advance(MINUTES_PER_TURN, 5.0);
    let just_fed = body.hunger();
    assert!(just_fed < 0.7, "hunger right after eating is {just_fed}");

    // Five hours on
    for _ in 0..2 {
        body.advance(MINUTES_PER_TURN, 5.0);
    }
    let later = body.hunger();
    assert!(
        later > just_fed,
        "hunger should build: {just_fed} -> {later}"
    );
}

/// Fat is worth more than greens, unit for unit.
#[test]
fn caloric_density_follows_the_food() {
    // The database runs six (greens) to eighty (fat)
    let greens = how_rich_this_food_is(6.0);
    let ordinary = how_rich_this_food_is(25.0);
    let fat = how_rich_this_food_is(80.0);
    assert!(greens < ordinary && ordinary < fat);
    assert!((ordinary - 1.0).abs() < 1e-5, "twenty-five is ordinary forage");

    // And it tells on the body: a stomach of greens is worth less than a
    // stomach of fat
    let mut thin = Physiology::new();
    let mut rich = Physiology::new();
    thin.eat(600.0, greens);
    rich.eat(600.0, fat);
    for _ in 0..24 {
        thin.advance(MINUTES_PER_TURN, 5.0);
        rich.advance(MINUTES_PER_TURN, 5.0);
    }
    assert!(
        rich.reserve > thin.reserve,
        "fat {} greens {}",
        rich.reserve,
        thin.reserve
    );
}

/// A child carries days where an adult carries weeks.
#[test]
fn a_smaller_body_has_less_to_go_on() {
    let child = Physiology::for_a_body_of(0.45);
    let adult = Physiology::for_a_body_of(1.0);
    assert!(child.reserve_capacity < adult.reserve_capacity);
    assert!(child.stomach_capacity < adult.stomach_capacity);

    // And starves sooner on the same going-without
    let mut child = child;
    let mut adult = adult;
    let mut turns = 0;
    while !child.starved() && turns < 100_000 {
        child.advance(MINUTES_PER_TURN, 5.0);
        adult.advance(MINUTES_PER_TURN, 5.0);
        turns += 1;
    }
    assert!(!adult.starved(), "the adult should still be alive");
}

/// The drive an agent acts on has to rise before the body is in trouble, or
/// the agent finds out too late. This is the mistake ISSUES #73 records.
#[test]
fn thirst_presses_before_the_body_goes_short() {
    let mut body = Physiology::new();
    // Thirst should reach the drive threshold (0.75) while the body is still
    // at full capability
    while body.thirst() < 0.75 {
        body.advance(MINUTES_PER_TURN, 5.0);
        assert!(body.minute < MINUTES_TO_DIE_OF_THIRST, "thirst never pressed");
    }
    assert_eq!(
        body.capability(),
        1.0,
        "thirst pressed only after capability had already dropped, at {} hydration",
        body.hydration
    );
}

#[test]
fn hunger_presses_before_the_reserve_runs_down() {
    let mut body = Physiology::new();
    body.eat(UNITS_IN_A_PORTION, 1.0);
    while body.hunger() < 0.7 {
        body.advance(MINUTES_PER_TURN, 5.0);
        assert!(body.minute < MINUTES_TO_STARVE, "hunger never pressed");
    }
    assert!(
        body.reserve > body.reserve_capacity * 0.9,
        "hunger pressed only after the reserve was down to {}",
        body.reserve
    );
}

/// A body on three meals a day is never once starving, however the meals fall.
///
/// The gut empties about thirty hours after a meal, so a missed meal empties
/// it - and "nothing in the stomach and nothing in the gut" on its own is a
/// long morning rather than starvation. Bodies carrying sixteen and nineteen
/// days of food were reading as starving because their gut happened to be
/// empty, and the breeding gate took that as a reason not to have children.
/// See ISSUES #77.
#[test]
fn a_fed_body_is_never_starving_between_meals() {
    let mut body = Physiology::new();
    for day in 0..14 {
        for turn in 0..12 {
            // Three meals a day, and one day in seven with a meal missed
            let a_meal = if day % 7 == 3 {
                turn == 0 || turn == 6
            } else {
                turn == 0 || turn == 4 || turn == 8
            };
            if a_meal {
                body.eat(UNITS_IN_A_PORTION, 1.0);
            }
            body.advance(MINUTES_PER_TURN, 5.0);
            assert!(
                !body.is_starving(),
                "starving on day {day} turn {turn} with {:.0} of {:.0} reserve \
                 ({:.1} days in), stomach {:.0}, gut {:.0}",
                body.reserve,
                body.reserve_capacity,
                body.days_into_the_reserve(),
                body.in_the_stomach(),
                body.in_the_gut(),
            );
        }
    }
}

/// And a body that has actually gone without is.
#[test]
fn a_body_three_days_without_food_is_starving() {
    let mut body = Physiology::new();
    assert!(!body.is_starving(), "a body that has never eaten is not starving");

    // Two days without: hungry, not starving
    body.gone_without_food_for(2 * MINUTES_PER_DAY);
    assert!(
        !body.is_starving(),
        "two days is going hungry, not starving ({:.1} days in)",
        body.days_into_the_reserve()
    );

    // Four days without, and it is
    let mut body = Physiology::new();
    body.gone_without_food_for(4 * MINUTES_PER_DAY);
    assert!(body.is_starving(), "four days without food is starving");
}

/// How far into the reserve a body is does not depend on how big the body is.
#[test]
fn days_into_the_reserve_is_the_same_question_for_a_child() {
    let mut child = Physiology::for_a_body_of(0.45);
    let mut adult = Physiology::for_a_body_of(1.0);
    child.gone_without_food_for(3 * MINUTES_PER_DAY);
    adult.gone_without_food_for(3 * MINUTES_PER_DAY);

    assert!((child.days_into_the_reserve() - 3.0).abs() < 0.01);
    assert!((adult.days_into_the_reserve() - 3.0).abs() < 0.01);
    // Though the child has eaten through far more of what it had
    assert!(
        child.reserve / child.reserve_capacity < adult.reserve / adult.reserve_capacity,
        "three days costs a small body a larger share of what it carries"
    );
}
