// src/agents/practices.rs
//! Ways of working that an agent has come to believe in.
//!
//! Not everything an agent does should be something it was born knowing. Some
//! things are worked out: somebody tips a basket of spoiled food onto a field
//! because they were curious and it was in the way, notices the following
//! season that the ground there is darker and the crop heavier, does it again,
//! and the people who watched them do it start doing it too.
//!
//! This is the record of that. A practice starts unproven; an agent will try an
//! unproven practice occasionally, out of curiosity, and what happens next
//! decides whether it does it again. Watching somebody else do it counts for
//! something too, though less than doing it yourself - which is the difference
//! between being told a thing works and finding out.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A way of working that has to be discovered rather than known
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Practice {
    /// Carrying spoiled food, bones and refuse onto a field, on the theory
    /// that it does the ground good
    SpreadingMuck,
}

/// What an agent believes about the practices it knows of
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Practices {
    /// How sure the agent is that a practice is worth the trouble, 0.0 to 1.0
    confidence: HashMap<Practice, f32>,
    /// How many times it has tried it
    attempts: HashMap<Practice, u32>,
}

impl Practices {
    /// Confidence above which an agent stops experimenting and simply does the
    /// thing as a matter of course
    pub const ESTABLISHED: f32 = 0.5;

    /// How often an agent tries something it has no opinion about, per
    /// opportunity, before curiosity is taken into account
    const BASE_CURIOSITY: f32 = 0.05;

    pub fn new() -> Self {
        Self::default()
    }

    /// How sure the agent is about this practice
    pub fn confidence(&self, practice: Practice) -> f32 {
        self.confidence.get(&practice).copied().unwrap_or(0.0)
    }

    /// How many times it has tried it
    pub fn attempts(&self, practice: Practice) -> u32 {
        self.attempts.get(&practice).copied().unwrap_or(0)
    }

    /// Whether this is settled practice for this agent
    pub fn is_established(&self, practice: Practice) -> bool {
        self.confidence(practice) >= Self::ESTABLISHED
    }

    /// Whether the agent gives this a go on an opportunity it has now.
    ///
    /// A settled practice is simply done. An unproven one is tried now and
    /// again, more often by a curious agent and by one that has already had it
    /// half work. Something tried repeatedly and found useless is dropped.
    pub fn would_try(&self, practice: Practice, curiosity: f32, roll: f32) -> bool {
        if self.is_established(practice) {
            return true;
        }

        let belief = self.confidence(practice);

        // Given up on: tried a good few times and it never came to anything
        if self.attempts(practice) >= 6 && belief <= 0.05 {
            return false;
        }

        let appetite = Self::BASE_CURIOSITY * (0.5 + curiosity.clamp(0.0, 1.0)) + belief * 0.4;

        roll < appetite
    }

    /// Record how a try turned out.
    ///
    /// Trial and error, and error counts: something that does nothing loses
    /// ground faster than something that works gains it, which is why a
    /// practice has to earn its place several times over.
    pub fn record_outcome(&mut self, practice: Practice, worked: bool) {
        *self.attempts.entry(practice).or_insert(0) += 1;

        let belief = self.confidence.entry(practice).or_insert(0.0);

        if worked {
            *belief = (*belief + 0.2).min(1.0);
        } else {
            *belief = (*belief - 0.1).max(0.0);
        }
    }

    /// Watching somebody else do it.
    ///
    /// Worth something, and less than doing it: seeing a thing done tells you
    /// it is done, not that it works.
    pub fn learn_from_watching(&mut self, practice: Practice) {
        let belief = self.confidence.entry(practice).or_insert(0.0);
        *belief = (*belief + 0.06).min(1.0);
    }

    /// Every practice this agent has an opinion about
    pub fn known(&self) -> impl Iterator<Item = (&Practice, &f32)> {
        self.confidence.iter()
    }
}
