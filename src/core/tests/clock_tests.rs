// src/core/tests/clock_tests.rs
//! What the two units are, and what the compiler will not let you say.
//!
//! The arithmetic here is not interesting on its own - it is a wrapper round
//! `u32`. What these hold is the *conversions*, because every defect this type
//! exists to prevent was a conversion somebody did not know they were making.

use crate::core::clock::{Ticks, Turns, TICKS_IN_A_TURN};

/// The two units and the one place they meet.
#[test]
fn a_tick_is_a_minute_and_a_turn_is_thirty_of_them() {
    assert_eq!(TICKS_IN_A_TURN, 30);
    assert_eq!(Turns::of(1).in_ticks(), Ticks::of(30));
    assert_eq!(Turns::of(48).in_ticks(), Ticks::of(1_440), "a day");
}

/// Crossing between them is a named thing, and it rounds the way a clock does.
#[test]
fn a_span_holds_whole_turns_and_says_so() {
    assert_eq!(Ticks::of(30).whole_turns(), Turns::of(1));
    assert_eq!(Ticks::of(1_440).whole_turns(), Turns::of(48));

    // The spec's worked example: an undertaking of 105 ticks covers three
    // whole gates and leaves fifteen minutes over.
    assert_eq!(Ticks::of(105).whole_turns(), Turns::of(3));
}

/// The round trip loses the remainder, and does not pretend otherwise.
#[test]
fn going_back_the_other_way_keeps_only_the_whole_ones() {
    assert_eq!(Turns::of(3).in_ticks(), Ticks::of(90));
    assert_eq!(Ticks::of(105).whole_turns().in_ticks(), Ticks::of(90));
}

/// A count of decisions is a loop; a span of minutes is not.
///
/// `Turns` iterates and `Ticks` does not, which is the whole of the defect
/// that made `cargo test --lib` stop finishing: a test asked for five days and
/// looped `5 * TICKS_PER_DAY` times over something that steps a turn, so it
/// ran a hundred and fifty days. The compile_fail case is in the module
/// docstring, because that is where it can be written down.
#[test]
fn a_count_of_turns_is_what_a_loop_counts() {
    let mut stepped = 0;
    for _ in Turns::of(48).each() {
        stepped += 1;
    }
    assert_eq!(stepped, 48, "a day of thinking is forty-eight decisions");

    let mut again = 0;
    for _ in Turns::of(3) {
        again += 1;
    }
    assert_eq!(again, 3, "and it takes a for-loop directly");
}

/// The arithmetic that the world's cadences are kept with.
#[test]
fn a_span_divides_into_a_span_and_leaves_a_remainder() {
    let a_day = Ticks::of(1_440);

    // `now % ONCE_A_DAY == 0` is how the once-a-day business is attended to,
    // and the step is thirty, so the two have to stay commensurate.
    assert_eq!((a_day * 3) % a_day, Ticks::ZERO);
    assert_eq!(a_day % Turns::of(1).in_ticks(), Ticks::ZERO);

    // A span over a span is a bare count: how many of those fit in this.
    assert_eq!(Ticks::of(1_440) / Ticks::of(30), 48);
}

/// Neither goes below nothing by accident.
#[test]
fn neither_of_them_wraps_round_past_zero() {
    assert_eq!(Ticks::ZERO.saturating_sub(Ticks::of(10)), Ticks::ZERO);
    assert_eq!(Turns::ZERO.saturating_sub(Turns::of(10)), Turns::ZERO);
}
