// src/analytics/between_us/mod.rs
//! How one agent stands towards another.
//!
//! Afraid of, angry at, willing to trade with, willing to give to, worth
//! asking. A beast counts as another here: what these have in common is not
//! that the other party is a person but that there *is* another party, and that
//! what this one does next depends on what it makes of them.
//!
//! - [`threat`] - fear, anger, and the four answers to a thing in the way
//! - [`seeing`] - what everybody saw, and what they made of it
//! - [`exchange`] - trading, taking, and giving
//! - [`asking`] - putting a question to somebody who might know
//!
//! This layer sits across the other three rather than beside them: it is
//! consulted by [`crate::analytics::wanting`] when a drive needs somebody else
//! to answer it, and run as a phase by [`crate::analytics::turn`] when what
//! somebody feels has to be worked out before they can act on it. That is why
//! it is its own directory and not folded into either.
//!
//! Behaviour-neutral, and proved so: three seeds run six hundred ticks give
//! byte-identical worlds either side of the move.

pub mod asking;
pub mod exchange;
pub mod seeing;
pub mod threat;

use super::Simulation;

impl Simulation {
    /// How far a voice carries.
    ///
    /// The same reach as seeing somebody do something - if you can see a man
    /// pick a thing up, you can call across to him. It had no value at all
    /// before: `find_nearest_social_target` returned the nearest person *on the
    /// map*, and neither `socialising` nor `sharing_information` looked at where
    /// that person was, so a settlement's whole social life was conducted at
    /// arbitrary range. Two men twelve tiles apart, each alone in a different
    /// wood, greeted one another, exchanged news and gave one another presents.
    ///
    /// See ISSUES_FOUND.md #102.
    pub(in crate::analytics) const WITHIN_TALKING_DISTANCE: i32 =
        Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP;

    /// Whether these two are near enough to say anything to each other.
    pub(in crate::analytics) fn near_enough_to_talk(
        here: (i32, i32, i32),
        there: (i32, i32, i32),
    ) -> bool {
        (here.0 - there.0).abs().max((here.1 - there.1).abs()) <= Self::WITHIN_TALKING_DISTANCE
    }
}
