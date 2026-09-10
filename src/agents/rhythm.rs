// src/agents/rhythm.rs
//! How often a thing is worth doing, and how an agent finds that out.
//!
//! Everything an agent has learned until now has been about *what* to do. A
//! `Lesson` says whether a thing is worth trying and `how_far_down_the_list_to_look`
//! says whether to try the next thing instead - but neither of them can say
//! **how often**, and for a whole class of work that is the only question that
//! matters.
//!
//! A trapline is the plain case. Setting snares works, walking the line works,
//! and a settlement that does both and goes round every four days starves
//! beside a wood full of rabbits, because a catch left that long has been
//! eaten by something else. Nothing was wrong with what they were doing. The
//! rhythm was wrong, and there was nothing anywhere in the model that could
//! hold a rhythm, let alone change one.
//!
//! # What is being climbed
//!
//! **What one doing of it brings back.** Not what it brings in a day - that
//! measure is maximised by going round constantly and would drive every
//! rhythm to its floor - but what the *turn* buys, which is the question the
//! decision layer actually has to answer against every other use of the turn.
//!
//! That measure has a real knee in it, and the knee is the answer. Leaving a
//! line longer brings more back per round, but only up to the point where
//! what was caught first is gone again; past that the extra wait buys nothing
//! and costs the same turn. So an agent climbing it settles at about the span
//! the world takes to rob a snare - a number nobody wrote down here, which
//! lives in `SmallLife::WHAT_A_QUIET_COUNTRY_TAKES` and would move if that
//! did. That is the property worth having: the rhythm tracks the world rather
//! than a constant in this file.
//!
//! # And why it prefers the shorter of two equals
//!
//! The curve saturates, so near the top a longer wait and a shorter one bring
//! back much the same, and a plain hill climb would wander along the plateau
//! for ever. `WORTH_THE_WAIT` is the margin: a longer rhythm has to be
//! *meaningfully* better to be kept, and otherwise the agent goes back to the
//! shorter one. That puts it at the knee rather than out on the flat, and it
//! is also the right prejudice to have - food in hand today beats the same
//! food in hand on Thursday.

use serde::{Deserialize, Serialize};

use crate::environment::seasons::TICKS_PER_DAY;

/// How often one agent does one thing, and the evidence it has for that.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rhythm {
    /// Ticks between doings, as this agent currently has it.
    every: u32,
    /// When it was last done. `None` until it has been done once.
    last: Option<u32>,
    /// What has come back since this rhythm was last changed, and over how
    /// many doings.
    since_the_change: f32,
    doings: u32,
    /// What one doing was worth under the rhythm before this one.
    was_worth: Option<f32>,
    /// Which way the last change went, so a change that did not pay can be
    /// put back.
    went_shorter: bool,
    /// How many times the rhythm has moved, which is how far along the search
    /// is. See `still_finding_it`.
    changes: u32,
}

impl Rhythm {
    /// The longest anybody leaves anything: a week.
    ///
    /// Past this a man has not got a rhythm, he has forgotten about it, and
    /// whether the thing is worth doing at all is a question `Lessons`
    /// answers rather than this.
    pub const THE_LONGEST_ANYBODY_LEAVES_IT: u32 = 7 * TICKS_PER_DAY;

    /// And the shortest: a round in the morning.
    ///
    /// A trapper walks his line once a day. Going twice does not give the
    /// snares time to catch anything and spends a second turn finding that
    /// out, and the floor says so rather than making the agent discover it
    /// the expensive way.
    pub const THE_SHORTEST_ANYBODY_LEAVES_IT: u32 = TICKS_PER_DAY;

    /// Where a rhythm starts before anything is known about it.
    ///
    /// Deliberately in the middle and deliberately wrong: an agent that
    /// started at the answer would not be discovering anything, and the
    /// measurement below would say nothing about whether the search works.
    pub const WHAT_A_BODY_GUESSES_FIRST: u32 = 3 * TICKS_PER_DAY;

    /// How many doings it takes before a rhythm may be judged.
    ///
    /// Few enough that a change is tested inside a season, and more than one
    /// so that a single lucky round does not settle it.
    pub const ENOUGH_TO_TELL: u32 = 4;

    /// How much better a longer rhythm has to be before it is kept.
    ///
    /// A tenth. See the note above on the plateau: without a margin the climb
    /// wanders, and with one it sits at the knee.
    pub const WORTH_THE_WAIT: f32 = 1.1;

    /// And how much a rhythm moves when it moves. A quarter either way, so a
    /// span of a week is crossed in a handful of changes.
    pub const HOW_MUCH_A_RHYTHM_SHIFTS: f32 = 0.25;

    /// How many moves it takes before the rhythm is somebody's settled habit
    /// rather than a thing still being found out.
    ///
    /// Eight, which at a quarter a move is enough to cross the whole span from
    /// the first guess to either bound and back.
    pub const SETTLED_AFTER: u32 = 8;

    /// A rhythm nobody has any evidence about yet.
    pub fn unfound() -> Self {
        Self {
            every: Self::WHAT_A_BODY_GUESSES_FIRST,
            last: None,
            since_the_change: 0.0,
            doings: 0,
            was_worth: None,
            went_shorter: true,
            changes: 0,
        }
    }

    /// Whether this is still being found out rather than done.
    ///
    /// **Reported, not acted on, and that is a measurement rather than an
    /// oversight.** The obvious use is to shelter a searching rhythm from
    /// `Lessons`: while the cadence is still moving, an empty round says he
    /// went at the wrong time, not that trapping does not feed him, so the
    /// coarse book should not hear about it. That is a good argument and it
    /// is wrong, because the coarse book is also the brake.
    ///
    /// Measured over 32 seeded worlds. Sheltered, rounds went from 10.5 a
    /// lifetime to **43.6** at a 15% success rate, the cadence drifted out to
    /// three and a half days instead of in, and person-days fell **105,933 to
    /// 101,733** with worlds emptied 24 of 32 to 29. Taking the brake off a
    /// search that is not converging does not buy convergence, it buys a
    /// settlement that spends its winter walking an empty line. Shipped
    /// unsheltered: an agent may give up on trapping while its rhythm is
    /// still moving, and on this ecology's numbers it is right to.
    pub fn still_finding_it(&self) -> bool {
        self.changes < Self::SETTLED_AFTER
    }

    /// How many times it has moved, which is how far along the search is.
    pub fn changes(&self) -> u32 {
        self.changes
    }

    /// How long this one leaves it, as it currently has it.
    pub fn every(&self) -> u32 {
        self.every
    }

    /// Whether it is due.
    ///
    /// A thing never done is always due: that is the first go, and without it
    /// nothing would ever start.
    pub fn is_it_due(&self, now: u32) -> bool {
        match self.last {
            None => true,
            Some(then) => now.saturating_sub(then) >= self.every,
        }
    }

    /// How long since it was last done, for anybody wanting to weigh it.
    pub fn how_long_since(&self, now: u32) -> Option<u32> {
        self.last.map(|then| now.saturating_sub(then))
    }

    /// It was done, and this is what it brought back.
    ///
    /// The rhythm is judged here and nowhere else, so there is one place that
    /// decides what an agent believes about how often to do a thing.
    pub fn how_it_went(&mut self, now: u32, brought_back: f32) {
        self.last = Some(now);
        self.since_the_change += brought_back.max(0.0);
        self.doings += 1;

        if self.doings < Self::ENOUGH_TO_TELL {
            return;
        }

        let worth = self.since_the_change / self.doings as f32;
        self.since_the_change = 0.0;
        self.doings = 0;

        let Some(before) = self.was_worth else {
            // The first stretch is the thing every later stretch is measured
            // against; there is nothing yet to compare it with, so take a
            // step and find out.
            self.was_worth = Some(worth);
            self.shift(self.went_shorter);
            return;
        };

        // Was the last change worth making? A shorter rhythm has only to hold
        // its own; a longer one has to earn the wait.
        let paid = if self.went_shorter {
            worth * Self::WORTH_THE_WAIT >= before
        } else {
            worth >= before * Self::WORTH_THE_WAIT
        };

        self.was_worth = Some(worth);
        if paid {
            self.shift(self.went_shorter);
        } else {
            self.shift(!self.went_shorter);
        }
    }

    /// Move the rhythm, and remember which way it went.
    fn shift(&mut self, shorter: bool) {
        self.went_shorter = shorter;
        self.changes = self.changes.saturating_add(1);
        let by = self.every as f32 * Self::HOW_MUCH_A_RHYTHM_SHIFTS;
        let moved = if shorter {
            self.every as f32 - by
        } else {
            self.every as f32 + by
        };

        self.every = (moved.round() as u32).clamp(
            Self::THE_SHORTEST_ANYBODY_LEAVES_IT,
            Self::THE_LONGEST_ANYBODY_LEAVES_IT,
        );
    }
}

impl Default for Rhythm {
    fn default() -> Self {
        Self::unfound()
    }
}
