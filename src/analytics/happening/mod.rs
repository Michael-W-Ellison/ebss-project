// src/analytics/happening/mod.rs
//! What happens whether or not anybody decides anything.
//!
//! The third of the three layers, and the one that was hardest to see, because
//! its parts were never next to each other: the ground coming up in berries was
//! two thousand lines from the weather that made it wet, and the beasts deciding
//! what to make of us was a thousand lines from the beasts acting on it.
//!
//! - [`soil`] - what the ground does, and what goes back into it
//! - [`weather`] - the weather on a body, and what a clear day dries
//! - [`beasts`] - what they make of us, and what they do about it
//! - [`kin`] - carrying, bearing, and feeding what cannot feed itself
//! - [`noticing`] - what a person finds out by being somewhere at the time
//! - [`senses`] - what can be smelled, and what stops being worth remembering
//! - [`situation`] - reading the world, so a drive rises on a condition
//! - [`buildings`] - buildings, and what standing in one does to somebody
//!
//! The three layers, together: [`crate::analytics::wanting`] decides,
//! [`crate::analytics::doing`] acts, and this happens. The order they run in is
//! [`crate::analytics::turn`], and the arguments about that order - the beasts
//! look before they move, the world is ticked once and not twice, waste goes
//! back on the ground before anybody smells it - are the reason the order is
//! written down in one place rather than implied by where the code sits.
//!
//! Behaviour-neutral, and proved so: three seeds run six hundred ticks give
//! byte-identical worlds either side of the move.

pub mod beasts;
pub mod buildings;
pub mod kin;
pub mod noticing;
pub mod senses;
pub mod situation;
pub mod soil;
pub mod weather;
