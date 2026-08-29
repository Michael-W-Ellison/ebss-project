// src/agents/patterns.rs
//! What answered what, and where.
//!
//! "When an agent satisfies drive demand, it links its previous actions taken
//! to the drive satisfaction to form a pattern. (e.g., travel to + specific
//! location = water)."
//!
//! `Lessons` already records whether a thing works: an agent that keeps
//! failing to fish fishes less. What it cannot say is *what need* the thing
//! answered, or *where* it answered it - and the place is most of the value.
//! The largest single failure in the whole simulation is a thirsty agent
//! standing where there is no water asking for water, over and over, because
//! nothing connected the drink it had yesterday to the bank it drank from.
//!
//! A pattern here is the pair (the need, the thing done), and what is written
//! against it is how often it worked, when it last worked, and the ground it
//! was standing on when it did.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::DriveType;

/// One thing that has answered one need.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Habit {
    /// How many times doing this answered this need
    pub times: u32,
    /// The ground it was standing on the last time it worked
    pub where_it_worked: Option<(i32, i32, i32)>,
    /// When it last worked
    pub last_worked: u32,
}

/// Everything an agent has noticed about what answers what.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Patterns {
    /// Keyed by the need, and then by the thing done - see
    /// `Agent::what_was_tried`. Nested rather than keyed by a pair so that it
    /// survives a round trip through a format whose map keys are strings.
    what_answers: BTreeMap<DriveType, BTreeMap<String, Habit>>,
}

impl Patterns {
    /// How much a drive has to fall in one action for the agent to connect
    /// the two.
    ///
    /// A drink is worth half a drive; a berry a fifth. Below this is the drift
    /// that happens anyway, and joining that to whatever the agent happened to
    /// be doing is how a superstition gets made.
    pub const ENOUGH_TO_NOTICE: f32 = 0.1;

    /// How many times a thing has to have worked before an agent will walk
    /// across a map for it.
    ///
    /// Twice is a coincidence.
    pub const A_HABIT_BY_NOW: u32 = 3;

    /// How long a place stays worth walking to.
    ///
    /// A season. A river is still a river next month; a berry patch picked
    /// bare in the spring is not worth the walk in the autumn, and the agent
    /// finding that out is what takes the pattern off the list.
    pub const STILL_WORTH_THE_WALK: u32 = 288;

    /// Note that doing this answered this need, here.
    pub fn it_worked(
        &mut self,
        need: DriveType,
        what: &str,
        where_it_was: (i32, i32, i32),
        now: u32,
    ) {
        let habit = self
            .what_answers
            .entry(need)
            .or_default()
            .entry(what.to_string())
            .or_default();

        habit.times = habit.times.saturating_add(1);
        habit.where_it_worked = Some(where_it_was);
        habit.last_worked = now;
    }

    /// Note that doing this did not answer this need after all.
    ///
    /// The pattern is not forgotten - a river that was dry today is still a
    /// river - but the count goes back down, so a place that stops working
    /// stops being worth the walk.
    pub fn it_did_not(&mut self, need: DriveType, what: &str) {
        if let Some(habit) = self
            .what_answers
            .get_mut(&need)
            .and_then(|against| against.get_mut(what))
        {
            habit.times = habit.times.saturating_sub(1);
        }
    }

    /// What this agent has found answers a need, best established first.
    pub fn what_answers(&self, need: DriveType) -> Option<(&str, &Habit)> {
        self.what_answers
            .get(&need)?
            .iter()
            .max_by_key(|(_, habit)| habit.times)
            .map(|(what, habit)| (what.as_str(), habit))
    }

    /// How often a particular thing has answered a particular need.
    pub fn how_often(&self, need: DriveType, what: &str) -> u32 {
        self.what_answers
            .get(&need)
            .and_then(|against| against.get(what))
            .map(|habit| habit.times)
            .unwrap_or(0)
    }

    /// Ground worth going back to for a need, if there is any.
    ///
    /// Only a place that has worked often enough to be a habit rather than an
    /// accident, and recently enough to still be there.
    pub fn where_it_worked(&self, need: DriveType, now: u32) -> Option<(i32, i32, i32)> {
        let against = self.what_answers.get(&need)?;

        against
            .values()
            .filter(|habit| habit.times >= Self::A_HABIT_BY_NOW)
            .filter(|habit| now.saturating_sub(habit.last_worked) <= Self::STILL_WORTH_THE_WALK)
            .max_by_key(|habit| habit.times)
            .and_then(|habit| habit.where_it_worked)
    }

    /// How many needs this agent has worked out an answer to.
    pub fn how_much_i_have_worked_out(&self) -> usize {
        self.what_answers
            .values()
            .flat_map(|against| against.values())
            .filter(|habit| habit.times >= Self::A_HABIT_BY_NOW)
            .count()
    }

    /// Whether anything at all has been noticed.
    pub fn is_empty(&self) -> bool {
        self.what_answers.is_empty()
    }
}
