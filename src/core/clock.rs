// src/core/clock.rs
//! Two units of time, and a compiler that will not let you mix them.
//!
//! A **tick** is a minute. It is what the lifecycle specification counts, what
//! every clock in the body runs on, and what `Simulation::current_turn` and
//! `World::turn` hold. Durations, thresholds and timestamps are all ticks.
//!
//! A **turn** is a planning period: thirty ticks, forty-eight to the day. It is
//! when somebody *may* stop and decide. Loop counts, `denied_turns`,
//! `Errand::set_aside`, `turns_before_this_kills_me` and every rate applied
//! once per step are turns.
//!
//! # Why this is a type and not a comment
//!
//! Both were `u32`, and the difference between them was carried in the name of
//! the constant and in a docstring beside it. When `TICKS_PER_DAY` went from 48
//! to 1,440 - the day a tick became a minute rather than a step - every
//! constant derived from it changed meaning without changing its type, and
//! seventeen places went on reading it the old way. The suite could not tell
//! us, because the same change made two of the test helpers run thirty times
//! too long and the run stopped finishing.
//!
//! What that cost, in the world rather than in the arithmetic:
//!
//! - `WHAT_DREAD_LOOKS_AHEAD` was three days of *ticks* divided by a count of
//!   *turns*, so every agent alive read itself as half a day from dying of
//!   something. Thirteen thousand fear events in a run that should have had
//!   none, and errands set out fell from 966 to 22.
//! - A larder gave back one tick of preservation for every thirty it took, so
//!   every settlement that dug a pit had been getting nothing for it.
//! - A snare took a fifth of a chance a *month* instead of a day.
//! - `forget_a_little` takes turns and was handed ticks, so a buried winter
//!   store was forgotten in a fortnight.
//!
//! Each of those reads correctly until you know what unit the other side is
//! in. That is what a type is for.
//!
//! # The shapes that are now impossible
//!
//! `Ticks` deliberately does **not** implement `Step`, so it cannot be a range
//! bound:
//!
//! ```compile_fail
//! # use ebss::core::clock::Ticks;
//! for _ in Ticks::ZERO..Ticks::of(1440) {}   // will not compile
//! ```
//!
//! `Turns` can, because counting decisions is what it is for. And the two do
//! not add, subtract or compare to each other: crossing between them is
//! [`Turns::in_ticks`] or [`Ticks::whole_turns`], both of which name the
//! conversion at the point it happens.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

/// How many ticks there are in one turn.
///
/// The one place the two units meet. Everything else goes through
/// [`Turns::in_ticks`] and [`Ticks::whole_turns`].
pub const TICKS_IN_A_TURN: u32 = 30;

// ---------------------------------------------------------------------------
// Ticks
// ---------------------------------------------------------------------------

/// A span of minutes, or an instant measured in them.
///
/// The world's own clock. Not iterable on purpose - see the module note.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Ticks(pub u32);

impl Ticks {
    pub const ZERO: Ticks = Ticks(0);

    /// This many minutes.
    pub const fn of(minutes: u32) -> Self {
        Ticks(minutes)
    }

    /// The number, for arithmetic that has to leave the type - a ratio, a
    /// cast to `f32`, an index.
    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }

    /// How many whole turns this span covers.
    ///
    /// Rounds down: a span of forty ticks is one turn and ten minutes over.
    pub const fn whole_turns(self) -> Turns {
        Turns(self.0 / TICKS_IN_A_TURN)
    }

    pub fn saturating_sub(self, other: Ticks) -> Ticks {
        Ticks(self.0.saturating_sub(other.0))
    }

    pub fn saturating_add(self, other: Ticks) -> Ticks {
        Ticks(self.0.saturating_add(other.0))
    }

    pub fn max(self, other: Ticks) -> Ticks {
        Ticks(self.0.max(other.0))
    }

    pub fn min(self, other: Ticks) -> Ticks {
        Ticks(self.0.min(other.0))
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// How far apart two instants are, whichever came first.
    pub fn abs_diff(self, other: Ticks) -> Ticks {
        Ticks(self.0.abs_diff(other.0))
    }

    /// The same arithmetic as the operators, in a form `const` accepts.
    ///
    /// `impl Mul` cannot be `const`, and the calendar is nearly all constants
    /// derived from other constants - "four days", "a day and a half". These
    /// are for those.
    pub const fn times(self, by: u32) -> Ticks {
        Ticks(self.0 * by)
    }

    pub const fn plus(self, other: Ticks) -> Ticks {
        Ticks(self.0 + other.0)
    }

    pub const fn minus(self, other: Ticks) -> Ticks {
        Ticks(self.0 - other.0)
    }

    pub const fn divided_by(self, by: u32) -> Ticks {
        Ticks(self.0 / by)
    }
}

impl fmt::Display for Ticks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Ticks {
    type Output = Ticks;
    fn add(self, other: Ticks) -> Ticks {
        Ticks(self.0 + other.0)
    }
}

impl Sub for Ticks {
    type Output = Ticks;
    fn sub(self, other: Ticks) -> Ticks {
        Ticks(self.0 - other.0)
    }
}

impl AddAssign for Ticks {
    fn add_assign(&mut self, other: Ticks) {
        self.0 += other.0;
    }
}

impl SubAssign for Ticks {
    fn sub_assign(&mut self, other: Ticks) {
        self.0 -= other.0;
    }
}

/// A span times a count is a longer span - three days, ten turns' worth.
impl Mul<u32> for Ticks {
    type Output = Ticks;
    fn mul(self, by: u32) -> Ticks {
        Ticks(self.0 * by)
    }
}

impl Mul<Ticks> for u32 {
    type Output = Ticks;
    fn mul(self, span: Ticks) -> Ticks {
        Ticks(self * span.0)
    }
}

/// A span divided by a count is a shorter span.
impl Div<u32> for Ticks {
    type Output = Ticks;
    fn div(self, by: u32) -> Ticks {
        Ticks(self.0 / by)
    }
}

/// A span divided by a span is a bare ratio - how many of those fit in this.
impl Div for Ticks {
    type Output = u32;
    fn div(self, other: Ticks) -> u32 {
        self.0 / other.0
    }
}

/// `now % ONCE_A_DAY`, which is how the world's cadences are kept.
impl Rem for Ticks {
    type Output = Ticks;
    fn rem(self, other: Ticks) -> Ticks {
        Ticks(self.0 % other.0)
    }
}

// ---------------------------------------------------------------------------
// Turns
// ---------------------------------------------------------------------------

/// A count of planning periods - of chances to stop and think.
///
/// Iterable, because counting decisions is the thing it is for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Turns(pub u32);

impl Turns {
    pub const ZERO: Turns = Turns(0);
    pub const ONE: Turns = Turns(1);

    /// This many chances to think.
    pub const fn of(periods: u32) -> Self {
        Turns(periods)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }

    /// How long these take on the world's clock.
    pub const fn in_ticks(self) -> Ticks {
        Ticks(self.0 * TICKS_IN_A_TURN)
    }

    /// A loop over each of them.
    ///
    /// `for _ in how_many.each()` rather than `for _ in 0..how_many`, so that
    /// a span of ticks cannot be handed to a loop that steps the simulation -
    /// which is the mistake that made the suite stop finishing.
    pub fn each(self) -> std::ops::Range<u32> {
        0..self.0
    }

    pub fn saturating_sub(self, other: Turns) -> Turns {
        Turns(self.0.saturating_sub(other.0))
    }

    pub fn saturating_add(self, other: Turns) -> Turns {
        Turns(self.0.saturating_add(other.0))
    }

    pub fn max(self, other: Turns) -> Turns {
        Turns(self.0.max(other.0))
    }

    pub fn min(self, other: Turns) -> Turns {
        Turns(self.0.min(other.0))
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// The same arithmetic as the operators, in a form `const` accepts.
    pub const fn times(self, by: u32) -> Turns {
        Turns(self.0 * by)
    }

    pub const fn plus(self, other: Turns) -> Turns {
        Turns(self.0 + other.0)
    }

    pub const fn divided_by(self, by: u32) -> Turns {
        Turns(self.0 / by)
    }
}

impl fmt::Display for Turns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Turns {
    type Output = Turns;
    fn add(self, other: Turns) -> Turns {
        Turns(self.0 + other.0)
    }
}

impl Sub for Turns {
    type Output = Turns;
    fn sub(self, other: Turns) -> Turns {
        Turns(self.0 - other.0)
    }
}

impl AddAssign for Turns {
    fn add_assign(&mut self, other: Turns) {
        self.0 += other.0;
    }
}

impl SubAssign for Turns {
    fn sub_assign(&mut self, other: Turns) {
        self.0 -= other.0;
    }
}

impl Mul<u32> for Turns {
    type Output = Turns;
    fn mul(self, by: u32) -> Turns {
        Turns(self.0 * by)
    }
}

impl Mul<Turns> for u32 {
    type Output = Turns;
    fn mul(self, count: Turns) -> Turns {
        Turns(self * count.0)
    }
}

impl Div<u32> for Turns {
    type Output = Turns;
    fn div(self, by: u32) -> Turns {
        Turns(self.0 / by)
    }
}

impl Div for Turns {
    type Output = u32;
    fn div(self, other: Turns) -> u32 {
        self.0 / other.0
    }
}

impl Rem for Turns {
    type Output = Turns;
    fn rem(self, other: Turns) -> Turns {
        Turns(self.0 % other.0)
    }
}

impl IntoIterator for Turns {
    type Item = u32;
    type IntoIter = std::ops::Range<u32>;
    fn into_iter(self) -> Self::IntoIter {
        self.each()
    }
}

#[cfg(test)]
#[path = "tests/clock_tests.rs"]
mod clock_tests;
