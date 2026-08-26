// src/agents/wondering.rs
//! What happens if.
//!
//! Curiosity in this model has always been the same shape: pick a working
//! nobody here has tried, do it, and get an answer back in the same turn. That
//! is the right shape for "what does this lump of clay do if I press it", and
//! it is the wrong shape for most of what a stone-age people has to find out,
//! because most of it does not answer for three days and does not answer where
//! you are standing.
//!
//! "What happens if I leave meat in the rain?" is not a turn. It is a thing
//! put down, a state remembered, and somebody walking back later to look. What
//! is learned is learned from the *change*, or from the absence of one — a
//! man who has left flax in a stream four times and found it exactly as he
//! left it has learned something too, and what he has learned is to stop.
//!
//! This is the record of a question somebody has open. It is deliberately the
//! same mechanism for all of them: leaving meat out, burying food, putting
//! clay in the embers and salting a joint differ in what happens, not in how
//! anybody finds out.

use serde::{Deserialize, Serialize};

use crate::environment::seasons::TICKS_PER_DAY;
use crate::world::nutrition::PreparationState;
use crate::world::Position;

use super::practices::Circumstance;

/// What a thing was like when somebody left it, coarse enough that a change
/// is a change and fine enough that a change is noticed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Watched {
    /// What it was called. A lump of clay that comes out of a fire is not
    /// called clay any more, and that is the whole of what firing teaches.
    pub called: String,
    /// How far off turning it was, where that means anything
    pub freshness: Option<f32>,
    /// And what had been done to it
    pub preparation: Option<PreparationState>,
}

impl Watched {
    /// What this thing is like now.
    pub fn of(item: &super::InventoryItem) -> Self {
        Self {
            called: item.item_id.clone(),
            freshness: item.food_data.as_ref().map(|food| food.freshness),
            preparation: item.food_data.as_ref().map(|food| food.preparation),
        }
    }

    /// Whether anything has happened to it since, and what.
    ///
    /// `None` is a real answer and not a failure to get one: a thing that is
    /// exactly as it was left is the result of the experiment.
    pub fn what_became_of_it(&self, now: &Watched) -> Option<Became> {
        if now.called != self.called {
            // It is not the same thing any more, which is the strongest
            // answer this can give
            return Some(Became {
                says: "it is not what it was",
                for_the_better: true,
            });
        }

        if now.preparation != self.preparation {
            // Something was done to it by being left where it was: dried in
            // the sun, smoked over a fire, salted through
            let better = now
                .preparation
                .map(|how| how.spoilage_multiplier() < 1.0)
                .unwrap_or(false);
            return Some(Became {
                says: if better { "it keeps" } else { "it changed" },
                for_the_better: better,
            });
        }

        match (self.freshness, now.freshness) {
            (Some(was), Some(is)) if was - is >= Self::ENOUGH_OF_A_CHANGE_TO_NOTICE => {
                Some(Became {
                    says: "it has gone off",
                    for_the_better: false,
                })
            }
            _ => None,
        }
    }

    /// How far a thing has to have gone before somebody coming back to look
    /// would say it had changed at all.
    ///
    /// A fifth. Below that is the ordinary passing of time, which happens to
    /// everything everywhere and teaches nobody anything about the place they
    /// left it.
    pub const ENOUGH_OF_A_CHANGE_TO_NOTICE: f32 = 0.2;
}

/// What became of a thing somebody was watching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Became {
    /// How somebody would put it
    pub says: &'static str,
    /// And whether it is the sort of thing you would do again
    pub for_the_better: bool,
}

/// Where the thing being watched actually is.
///
/// A question is not always about something on the grass. Burying food puts it
/// in a hole; salting it leaves it in the pack. The mechanism is the same in
/// all three - a state remembered and a look taken later - and only the place
/// to look differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kept {
    OnTheGround,
    InThePit,
    InMyPack,
}

/// A question somebody has put to the world and is waiting on the answer to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wondering {
    /// What was done — `leave`, and in time the others
    pub did: String,
    /// What it was done to
    pub what: String,
    /// Where it was left, because that is where the answer is
    pub where_it_is: Position,
    /// When, so a question can be given up on
    pub since: u32,
    /// And what it was like then
    pub as_it_was: Watched,
    /// What the world was doing at the time.
    ///
    /// Carried rather than looked up on the way back, and that is the point of
    /// carrying it: what is being found out is what became of meat left out
    /// **in the rain**, and by the time anybody walks back to look the rain
    /// has usually stopped.
    pub in_this: Vec<Circumstance>,
}

impl Wondering {
    /// What this question is called, for the record an agent keeps of what
    /// pays and what does not.
    pub fn called(&self) -> String {
        format!("{}:{}", self.did, self.what)
    }

    /// Where to go and look for the answer.
    pub fn where_to_look(&self) -> Kept {
        match self.did.as_str() {
            Self::BURYING_IT => Kept::InThePit,
            Self::SALTING_IT => Kept::InMyPack,
            _ => Kept::OnTheGround,
        }
    }

    /// What the state of the thing means, for the question that was asked.
    ///
    /// **The verb decides what counts as a good answer, and it has to.** A
    /// thing left on the grass that is exactly as it was left a week later
    /// teaches nothing worth having: nothing came of leaving it there. A thing
    /// *buried* that is exactly as it was left a week later is the entire
    /// point of burying it.
    ///
    /// This is the whole of why preserving is worth anything. Rot is the
    /// wasted half of whatever was spent getting the food - if half the meat
    /// goes off before it is eaten then half the hunt was wasted, and the
    /// hours are gone either way. So for the keeping verbs, *no change* is the
    /// win, and it is the only one of the three answers that means the thing
    /// worked.
    pub fn what_it_means(&self, as_it_is: &Watched, waited_long_enough: bool) -> Option<Became> {
        let changed = self.as_it_was.what_became_of_it(as_it_is);

        match self.did.as_str() {
            Self::BURYING_IT | Self::SALTING_IT => match changed {
                // It went off in the hole, or in the pack with the salt still
                // on it. That is the answer, and it is no.
                Some(became) if !became.for_the_better => Some(became),
                // It came out better than it went in, which happens: salted
                // food goes on keeping and buried food can dry.
                Some(became) => Some(became),
                // Untouched after a week, which is exactly what was wanted.
                None if waited_long_enough => Some(Became {
                    says: "it kept",
                    for_the_better: true,
                }),
                None => None,
            },
            // Left lying about. A change is the answer; no change after long
            // enough is also an answer, and the answer is that nothing comes
            // of it.
            _ => match changed {
                Some(became) => Some(became),
                None if waited_long_enough => Some(Became {
                    says: "nothing came of it",
                    for_the_better: false,
                }),
                None => None,
            },
        }
    }

    /// What an experiment is called when it is somebody putting a thing in a
    /// hole and covering it over.
    pub const BURYING_IT: &'static str = "bury";

    /// And when it is somebody rubbing salt into it.
    pub const SALTING_IT: &'static str = "salt";

    /// Whether it is time to stop waiting.
    ///
    /// A week. Long enough for the weather to have done something to anything
    /// it was going to do something to, and short enough that a man is not
    /// still waiting on a lump of flint at midwinter.
    ///
    /// Four days and two paces was the first cut, and measured it threw away
    /// two thirds of the questions anybody put: an agent walks a long way in
    /// four days, and a question nobody happens to walk back past is a portion
    /// of food spent on nothing.
    pub fn given_up_on(&self, now: u32) -> bool {
        now.saturating_sub(self.since) > Self::HOW_LONG_ANYBODY_WONDERS
    }

    pub const HOW_LONG_ANYBODY_WONDERS: u32 = TICKS_PER_DAY * 7;

    /// How near you have to be to see what became of it. You would notice a
    /// thing you left five paces off.
    pub const CLOSE_ENOUGH_TO_GO_AND_LOOK: i32 = 5;
}
