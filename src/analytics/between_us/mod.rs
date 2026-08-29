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
