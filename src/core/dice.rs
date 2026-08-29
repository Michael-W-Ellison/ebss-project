// src/core/dice.rs
//! The one place this model gets its randomness from.
//!
//! Every roll in the simulation used `crate::core::dice::roll()` - eighty call sites
//! across twenty files - which is seeded from the operating system and cannot
//! be reseeded. So no run of this model was ever repeatable, and that had two
//! costs that were paid every day.
//!
//! **The suite could not be trusted.** Measured over three runs, twenty tests
//! failed every time and *fifteen more came and went*. A test that fails two
//! runs in three is worse than a test that fails always: it cannot tell a
//! regression from a coin, so any change made against it is unverifiable, and
//! a refactor made against it is reckless.
//!
//! **And every measurement was noisy.** The survival harness reads a mean over
//! thirty-two worlds with a spread of a hundred and twenty turns, so a change
//! worth fifty turns cannot be seen at all without running it several times and
//! squinting. Several judgements in this project's history were made on
//! differences inside that band, and at least two of them were wrong.
//!
//! What is here is deliberately small. `roll()` hands out an ordinary generator
//! and every call site that said `crate::core::dice::roll()` says `dice::roll()`
//! instead; the difference is that the stream behind it is thread-local and can
//! be set. `seed(n)` sets it, so a test or a harness gets the same world every
//! time, and threads do not interfere with each other - which matters because
//! the test runner puts each test on its own thread.
//!
//! It hands out an *owned* generator rather than a borrow on purpose. Call
//! sites hold their `rng` across other calls that also roll, and a borrow held
//! that way would panic; a fresh generator drawn from the shared stream is
//! deterministic and cannot deadlock against itself.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::cell::RefCell;

/// What a test thread starts from when nobody says otherwise.
///
/// Under test, the stream is set rather than drawn from the operating system,
/// so that every test is the same world every time without two thousand tests
/// each having to remember to say so. The test runner gives each test a thread
/// of its own, and a thread-local starts fresh, so this is per-test rather
/// than per-run.
///
/// A test that wants a *different* world - one that means to sample several -
/// calls `seed` with its own number.
#[cfg(test)]
const WHAT_A_TEST_STARTS_FROM: u64 = 0x_EB55_5EED;

thread_local! {
    /// The stream every roll on this thread is drawn from.
    ///
    /// Seeded from the operating system in a real run, so a run is still a
    /// different world every time; set to a known number under test, so the
    /// suite is a fact rather than a coin.
    static THE_STREAM: RefCell<StdRng> = RefCell::new({
        #[cfg(test)]
        { StdRng::seed_from_u64(WHAT_A_TEST_STARTS_FROM) }
        #[cfg(not(test))]
        { StdRng::from_entropy() }
    });
}

/// Set the stream for this thread, so that what follows is repeatable.
///
/// Call it at the top of a test or a harness. It only affects the calling
/// thread, which is what makes it safe under a test runner that gives every
/// test a thread of its own.
pub fn seed(what: u64) {
    DRAWS.with(|d| d.set(0));
    THE_STREAM.with(|stream| *stream.borrow_mut() = StdRng::seed_from_u64(what));
}

/// A generator to roll with.
///
/// Stands exactly where `crate::core::dice::roll()` stood. Draw one, roll it as many
/// times as the job wants, and drop it.
/// How many draws have been taken on this thread since it was last seeded.
///
/// The instrument that found the last of it. If two runs of the same seed
/// disagree, the point at which this count parts company is the point at which
/// something decided a branch on an input the seed does not fix - which is a
/// different fault, and a different place to look, from a roll that came out
/// the same but was *used* differently.
#[doc(hidden)]
pub fn draws_taken() -> u64 {
    DRAWS.with(|d| d.get())
}

thread_local! {
    static DRAWS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn roll() -> StdRng {
    DRAWS.with(|d| d.set(d.get() + 1));
    THE_STREAM.with(|stream| {
        let mut stream = stream.borrow_mut();
        StdRng::seed_from_u64(stream.next_u64())
    })
}

/// One value of whatever type is asked for, drawn from this stream.
///
/// Stands where `rand::random::<T>()` stood. That call is `thread_rng` under
/// another name, so ten sites in the fauna and flora - every wander an animal
/// takes, and whether it grazes, rests or hunts - were rolling outside the
/// seeded stream. Those ten were enough on their own: **the beasts moved
/// differently in every run of the same seed**, and by the fiftieth tick that
/// had reached the people, through the Safety drive of anyone who could see
/// one.
pub fn any<T>() -> T
where
    rand::distributions::Standard: rand::distributions::Distribution<T>,
{
    use rand::Rng;
    roll().gen::<T>()
}

/// A name for a new thing, drawn from the same stream as everything else.
///
/// `Uuid::new_v4` asks the operating system, not this stream, so every agent,
/// building and relationship in a run got a name no seed could reach. That
/// mattered far more than it looks: an id is not only a label. It is a
/// `BTreeMap` key, a sort key and a tie-break, so **two runs of the same seed
/// disagreed about who was who** before a single decision had been taken, and
/// every ordering downstream of an id disagreed with them.
///
/// Same layout as a v4 - version and variant bits set the same way - so
/// nothing that reads a `Uuid` can tell the difference. Only the source of the
/// sixteen bytes has changed.
pub fn name() -> uuid::Uuid {
    DRAWS.with(|d| d.set(d.get() + 1));
    let mut bytes = [0u8; 16];
    THE_STREAM.with(|stream| stream.borrow_mut().fill_bytes(&mut bytes));
    uuid::Builder::from_random_bytes(bytes).into_uuid()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    /// The same seed is the same world.
    #[test]
    fn a_seeded_stream_repeats_itself() {
        let run = || {
            seed(1234);
            (0..20)
                .map(|_| roll().gen_range(0..1000))
                .collect::<Vec<_>>()
        };

        assert_eq!(run(), run());
    }

    /// And a different seed is a different one, or seeding would be pointless.
    #[test]
    fn a_different_seed_is_a_different_world() {
        seed(1);
        let one: Vec<u32> = (0..20).map(|_| roll().gen_range(0..1_000_000)).collect();
        seed(2);
        let two: Vec<u32> = (0..20).map(|_| roll().gen_range(0..1_000_000)).collect();

        assert_ne!(one, two);
    }

    /// A generator held across another roll does not deadlock.
    ///
    /// This is why `roll` hands out an owned generator rather than a borrow:
    /// call sites do `let mut rng = roll();` and then call something that rolls
    /// again, and a `RefCell` borrow held across that would panic.
    #[test]
    fn one_roll_can_be_held_while_another_is_taken() {
        seed(7);
        let mut mine = roll();
        let theirs = roll().gen_range(0..10);
        let _ = mine.gen_range(0..10) + theirs;
    }

    /// A name is drawn from the stream too, so the same seed names the same
    /// people. This is the half of repeatability that `roll` alone does not
    /// buy: ids are keys and tie-breaks, so if they differ the orderings that
    /// hang off them differ with them.
    #[test]
    fn the_same_seed_names_the_same_people() {
        let run = || {
            seed(99);
            (0..8).map(|_| name()).collect::<Vec<_>>()
        };

        assert_eq!(run(), run());
    }

    /// And it is still a v4 by its bits, so nothing downstream can tell.
    #[test]
    fn a_name_is_shaped_like_the_one_it_replaces() {
        seed(5);
        let it = name();
        assert_eq!(it.get_version_num(), 4);
        assert_ne!(it, uuid::Uuid::nil());
    }
}
