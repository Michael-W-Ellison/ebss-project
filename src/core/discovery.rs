// src/core/discovery.rs
//! How slowly a people comes to new ideas: one setting for every discovery
//! (ISSUES_FOUND #389).
//!
//! "There is also no rush for tech development. I am anticipating it should
//! take hundreds to thousands of generations for technologies to be
//! discovered." Every way of finding something out was calibrated so that it
//! happened inside a two-year run, because a two-year run was what could be
//! measured: selective harvesting by day 130, burning by day 120, gardens by
//! day 400. That is the *development pace*, and it is kept, because it is the
//! only pace at which the machinery can be watched working.
//!
//! The slowness is how many times longer than that an idea takes to come.
//! There are two kinds of discovery and it applies to both the same way:
//!
//! - **Found out by experience**: so many gathers near home, so many hides
//!   scraped, so many days thirsty. At the development pace the idea came at
//!   exactly that count. Now the count is what makes the idea *possible*, and
//!   each experience after it brings it with a chance of one in the count
//!   times the slowness (`does_it_occur_now`), so it takes that many times as
//!   long on average, and comes to one person early and another never.
//! - **Noticed by chance**: a curious hand turning a thing over, a firing
//!   showing something. The odds are divided by the slowness
//!   (`at_this_pace`).
//!
//! What it does not touch is knowledge passing between people - watching it
//! done, being told, being taught - which is how a camp keeps what one person
//! found, and has its own pace.
//!
//! A run of the model starts at a people's pace, or at what `EBSS_PACE` says.
//! A thread under test starts at the development pace, as the dice start
//! seeded, so that the suite tests the machinery; a test or a harness that
//! wants either says so with `set` (the measurement harness reads `PACE`).

use std::cell::Cell;

/// The development pace: every idea comes at the count it was written with.
pub const THE_DEVELOPMENT_PACE: f64 = 1.0;

/// A people's pace: thirty thousand times slower (#389).
///
/// Reckoned from what the development pace measured. A camp of a dozen comes
/// back to the stands near home about twenty-eight times a day between them,
/// and selective harvesting wanted three hundred of those; at thirty thousand
/// times that it is about nine hundred years, some thirty-five generations,
/// for the first person in a camp to see it. Gardens wait on sowing, sowing on
/// seeing seed come up, and the stages after on those, so forager to farming
/// runs to hundreds of generations, and the later stages to thousands. Ideas
/// that come out of rarer experience come slower still: ore watched in a fire
/// is tens of thousands of years.
pub const A_PEOPLES_PACE: f64 = 30_000.0;

thread_local! {
    static SLOWNESS: Cell<f64> = Cell::new(the_pace_a_thread_starts_at());
}

/// A thread under test starts at the development pace. Anything else starts
/// at a people's pace, unless `EBSS_PACE` says otherwise - `EBSS_PACE=1` for
/// the development pace.
fn the_pace_a_thread_starts_at() -> f64 {
    if cfg!(test) {
        return THE_DEVELOPMENT_PACE;
    }
    std::env::var("EBSS_PACE")
        .ok()
        .and_then(|pace| pace.parse::<f64>().ok())
        .map_or(A_PEOPLES_PACE, |pace| pace.max(THE_DEVELOPMENT_PACE))
}

/// Set how many times slower than the development pace ideas come, on this
/// thread.
pub fn set(slowness: f64) {
    SLOWNESS.with(|s| s.set(slowness.max(THE_DEVELOPMENT_PACE)));
}

/// How many times slower than the development pace ideas come, on this
/// thread.
pub fn slowness() -> f64 {
    SLOWNESS.with(|s| s.get())
}

/// Whether an idea found out by experience comes now, to somebody who has had
/// the experience it was written as needing (`experience_needed` of it) and
/// one more.
///
/// Always at the development pace, without a roll, so that the development
/// pace is exactly what the model did before; otherwise with a chance of one
/// in `experience_needed` times the slowness.
pub fn does_it_occur_now(experience_needed: u32) -> bool {
    use rand::Rng;
    let slowness = slowness();
    if slowness <= THE_DEVELOPMENT_PACE {
        return true;
    }
    let chance = 1.0 / (experience_needed.max(1) as f64 * slowness);
    crate::core::dice::roll().gen_bool(chance.clamp(0.0, 1.0))
}

/// What odds of noticing something come to at this pace.
pub fn at_this_pace(odds: f64) -> f64 {
    odds / slowness()
}
