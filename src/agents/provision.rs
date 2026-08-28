// src/agents/provision.rs
//! Whether there is enough put by, and what not having enough feels like.
//!
//! "Do I have enough supplies to survive the day? No = extreme stress. The
//! week? No = high stress. The month? No = medium-high. The winter? No =
//! medium. Once basic survival needs can be satisfied over the long term,
//! other concerns start coming into play."
//!
//! Four horizons, each further out than the last and each less frightening to
//! fail. That ordering is the whole point: a man with nothing for tonight is
//! not thinking about the winter, and a man with a full larder and a hard
//! winter coming is uneasy rather than desperate.
//!
//! Nothing here is a written-down ladder of priorities. It comes out as one
//! number, and that number is the Preparedness drive, which already knows how
//! to put food by. What it does to the drives further up the chain it does
//! through `DriveState::is_unlocked`, the same as any other unanswered need -
//! which is what "other concerns start coming into play" means in this model.

use serde::{Deserialize, Serialize};

use super::physiology;
use crate::environment::seasons::{Season, DAYS_PER_SEASON, DAYS_PER_YEAR};

/// Days in a week.
pub const DAYS_IN_A_WEEK: u32 = 7;

/// Days in the third rung - "the month".
///
/// This calendar has no months in it. A season is twenty-four days and a year
/// is four of them, so four actual weeks would be *longer* than the winter it
/// is supposed to sit inside, and the ladder would invert: an agent with more
/// than a month put by would already have more than a winter, and the winter
/// rung could never be reached at all.
///
/// What the rung is for is a horizon between a week and a winter, so that is
/// what it is: half a season. One day, seven days, twelve days, a winter.
pub const DAYS_IN_A_MONTH: u32 = DAYS_PER_SEASON / 2;

/// What one item of ordinary food in a pack or a pit is worth to a body.
///
/// Stores are counted in items everywhere else in this model and the body
/// counts in energy, so this is the exchange rate between the two: a handful
/// of something, at what a unit of ordinary forage is worth. Eleven and a half
/// of them is a day.
///
/// It was a whole third of a day per item, which is what a meal used to be
/// worth whatever it was made of. An item is a handful now, and what a
/// particular handful is worth depends on what it is - this is the figure for
/// reckoning a mixed larder, where nobody is counting which berry is which.
pub const UNITS_IN_ONE_STORED_ITEM: f32 =
    physiology::UNITS_IN_ONE_ITEM * physiology::ENERGY_OF_ORDINARY_FOOD;

/// How far ahead an agent can see food, and what failing each horizon costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HowLongTheFoodLasts {
    /// Not enough for tonight
    NotTheDay,
    /// Enough for tonight, not for the week
    NotTheWeek,
    /// Enough for the week, not for the month
    NotTheMonth,
    /// Enough for the month, but not enough to see the winter out
    NotTheWinter,
    /// Enough, and there are other things to think about
    Enough,
}

impl HowLongTheFoodLasts {
    /// What this does to an agent, from nought to one.
    pub fn stress(&self) -> f32 {
        match self {
            HowLongTheFoodLasts::NotTheDay => 1.0,
            HowLongTheFoodLasts::NotTheWeek => 0.8,
            HowLongTheFoodLasts::NotTheMonth => 0.6,
            HowLongTheFoodLasts::NotTheWinter => 0.4,
            HowLongTheFoodLasts::Enough => 0.0,
        }
    }

    /// What to call it.
    pub fn name(&self) -> &'static str {
        match self {
            HowLongTheFoodLasts::NotTheDay => "not the day",
            HowLongTheFoodLasts::NotTheWeek => "not the week",
            HowLongTheFoodLasts::NotTheMonth => "not the month",
            HowLongTheFoodLasts::NotTheWinter => "not the winter",
            HowLongTheFoodLasts::Enough => "enough",
        }
    }

    /// Which rung this agent is on.
    ///
    /// `days_in_hand` is what is put by divided by what a day costs. The
    /// winter rung is different from the other three: it is not about how many
    /// days of food there are but about whether there is enough to see out a
    /// winter of `winter_days`, and it only bites as winter comes on - nobody
    /// in spring is uneasy about a winter three seasons off.
    pub fn reckon(days_in_hand: f32, winter_days: f32, how_near_winter: f32) -> Self {
        if days_in_hand < 1.0 {
            HowLongTheFoodLasts::NotTheDay
        } else if days_in_hand < DAYS_IN_A_WEEK as f32 {
            HowLongTheFoodLasts::NotTheWeek
        } else if days_in_hand < DAYS_IN_A_MONTH as f32 {
            HowLongTheFoodLasts::NotTheMonth
        } else if how_near_winter > 0.0 && days_in_hand < winter_days {
            HowLongTheFoodLasts::NotTheWinter
        } else {
            HowLongTheFoodLasts::Enough
        }
    }
}

/// How close the winter is, from nought a year out to one when it is here.
///
/// Squared, so the pressure is nothing at all through spring and comes on
/// through autumn rather than sitting at a low hum all year. This is the
/// difference between seeing the winter coming and feeling it arrive.
pub fn how_near_winter_is(day_of_year: u32) -> f32 {
    let winter_starts = Season::Winter.first_day();
    let today = day_of_year % DAYS_PER_YEAR;

    // In winter already
    if today >= winter_starts {
        return 1.0;
    }

    let days_off = (winter_starts - today) as f32;
    // Nothing to worry about more than a season and a half out
    let horizon = (DAYS_PER_SEASON as f32) * 1.5;
    let nearness = (1.0 - days_off / horizon).clamp(0.0, 1.0);
    nearness * nearness
}

/// What one agent's own reckoning of its provisions comes to.
///
/// The daily need is what this body has actually been burning rather than a
/// number from a table: a big agent working hard eats more than a small one
/// resting, and it is its own consumption it has to lay in against.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WhatIsPutBy {
    /// Food units this agent can reach - its own pack and the camp's stores
    pub units: f32,
    /// What this agent burns in a day, by its own reckoning
    pub eaten_in_a_day: f32,
    /// How many days that comes to
    pub days_in_hand: f32,
    /// How long a winter this agent expects, in days
    pub winter_days: f32,
    /// How near that winter is
    pub how_near_winter: f32,
    /// Which rung it puts the agent on
    pub rung: HowLongTheFoodLasts,
}

impl WhatIsPutBy {
    /// Work it out.
    pub fn reckon(units: f32, eaten_in_a_day: f32, winter_days: f32, day_of_year: u32) -> Self {
        let eaten_in_a_day = eaten_in_a_day.max(1.0);
        let days_in_hand = units / eaten_in_a_day;
        let how_near_winter = how_near_winter_is(day_of_year);
        Self {
            units,
            eaten_in_a_day,
            days_in_hand,
            winter_days,
            how_near_winter,
            rung: HowLongTheFoodLasts::reckon(days_in_hand, winter_days, how_near_winter),
        }
    }

    /// What this agent is trying to have laid by before the winter.
    pub fn what_a_winter_wants(&self) -> f32 {
        self.eaten_in_a_day * self.winter_days
    }

    /// How much is still wanting.
    pub fn still_short_by(&self) -> f32 {
        (self.what_a_winter_wants() - self.units).max(0.0)
    }

    /// What this does to the agent, as a drive value.
    ///
    /// The three near horizons press at what they are worth. The winter rung
    /// is scaled by how near the winter is, so an empty larder in spring is a
    /// thing to get on with and an empty larder in late autumn is a fright.
    pub fn stress(&self) -> f32 {
        match self.rung {
            HowLongTheFoodLasts::NotTheWinter => self.rung.stress() * self.how_near_winter,
            other => other.stress(),
        }
    }
}

/// What a winter is, for somebody who has never seen one out.
///
/// The calendar's own answer. An agent that has been through winters uses what
/// it actually counted instead - see `Agent::how_long_a_winter_i_expect`.
pub fn how_long_a_winter_is_supposed_to_be() -> f32 {
    DAYS_PER_SEASON as f32
}

/// What an agent has learned about winters by living through them.
///
/// "The agents should calculate the average length of winter." Nobody is born
/// knowing it; it is counted, and the count is kept.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WintersSeen {
    /// Days of winter counted across every winter seen through
    pub days_counted: u32,
    /// How many of them have been seen through
    pub winters: u32,
    /// Whether the count is running now
    pub in_one_now: bool,
    /// The last day of the year counted, so a day is not counted twice
    last_day_counted: Option<u32>,
}

impl WintersSeen {
    /// Another day goes by. Call it once a day with today's season.
    pub fn another_day(&mut self, season: Season, day_of_year: u32) {
        if self.last_day_counted == Some(day_of_year) {
            return;
        }
        self.last_day_counted = Some(day_of_year);

        match (season, self.in_one_now) {
            (Season::Winter, _) => {
                self.in_one_now = true;
                self.days_counted += 1;
            }
            (_, true) => {
                // It broke. That is one winter seen through.
                self.in_one_now = false;
                self.winters += 1;
            }
            _ => {}
        }
    }

    /// How long this agent expects a winter to be.
    ///
    /// Its own average once it has seen one out; the calendar's answer before
    /// that, because a people with no experience of the place still has to lay
    /// something by for its first winter.
    pub fn how_long_a_winter_lasts(&self) -> f32 {
        if self.winters == 0 {
            return how_long_a_winter_is_supposed_to_be();
        }
        self.days_counted as f32 / self.winters as f32
    }
}

/// What one pace of walking costs, there and back again.
pub const WHAT_A_PACE_COSTS: f32 = 0.25;

/// What picking a portion of ordinary food costs, once you are standing over it.
pub const WHAT_PICKING_COSTS: f32 = 2.0;

/// What a forage costs the body, in energy.
///
/// "The energy price of foraging should be based on the distance traveled, the
/// type of food eaten, and the effort it takes to get/prepare the food."
///
/// Three things, then. The walk, counted both ways, because a patch twenty
/// paces off is forty paces of walking. What the food itself takes to win:
/// dense food is animal food and root food - a carcass wants butchering and a
/// root wants digging - where spring greens are picked off the hedge and eaten
/// where they stand, so richness stands in for the work of getting it. And a
/// flat cost for the picking itself, so that even food underfoot is not free.
///
/// It was a flat five whatever the agent did, which is why a patch across the
/// valley cost the same as the bush at the door and nobody had any reason to
/// prefer the near one.
pub fn what_foraging_costs(paces: u32, richness: f32) -> f32 {
    let walk = paces as f32 * WHAT_A_PACE_COSTS * 2.0;
    let winning_it = WHAT_PICKING_COSTS * (0.5 + richness);
    walk + winning_it
}
