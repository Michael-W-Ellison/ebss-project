// src/agents/whereabouts.rs
//! What an agent knows about the country, which is two different things.
//!
//! "Map data should be divided between important places and general
//! knowledge."
//!
//! **General knowledge** is the impression a place leaves by being walked
//! through. It builds by looking - five points a day, and no more than once a
//! day however long somebody stands there - and it goes again by not looking:
//! a month's grace, then five points a month. So twenty days of living in a
//! place is enough to know it thoroughly, and twenty months of never going
//! back is enough to lose it. A place walked through once is a five per cent
//! impression that is gone by the summer. That is the whole point of the
//! split: an agent ends up holding the country it lives in rather than every
//! field it has ever crossed.
//!
//! An area is thirty-two tiles across, which is not an arbitrary figure. An
//! agent sees three tiles in each direction, so one look takes in forty-nine
//! tiles and about twenty looks covers an area. Five points a look reaching a
//! hundred in twenty looks is the same number arrived at from the other end.
//!
//! **Important places** are the ones that answered a need. A man who found a
//! berry patch and ate remembers that there are berries over there for five
//! years, and he remembers it as *over there* - the area, not the tile. That
//! is deliberate and it is what makes it cheap to keep: an exact spot goes
//! stale when the bush is picked, an area does not, and "there is food that
//! way" is what somebody actually carries around for years.
//!
//! What this replaces: a flat ceiling of ninety-six remembered places, kept or
//! dropped on a score made of how much the agent wants the thing, how fresh
//! the news is and how much was standing there - none of which is about
//! whether the agent has ever been near the place. See
//! `Agent::forget_what_does_not_matter`.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::core::DriveType;
use crate::environment::seasons::{DAYS_PER_MONTH, DAYS_PER_YEAR};

/// How many tiles across an area is.
///
/// Thirty-two, so about twenty looks covers one - see the module note.
pub const HOW_WIDE_AN_AREA_IS: i32 = 32;

/// A square of country, coarse enough that "the general area" means something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Area {
    pub across: i32,
    pub down: i32,
}

impl Area {
    /// The area a place falls in.
    ///
    /// Floored rather than truncated, so the areas west of the origin are the
    /// same size as the ones east of it. Truncation puts a double-width area
    /// across zero, which is the sort of thing that shows up as one square of
    /// country nobody can ever quite learn.
    pub fn holding(place: (i32, i32, i32)) -> Self {
        Self {
            across: place.0.div_euclid(HOW_WIDE_AN_AREA_IS),
            down: place.1.div_euclid(HOW_WIDE_AN_AREA_IS),
        }
    }

    /// The middle of it, which is where somebody heads for when all they know
    /// is the area.
    pub fn middle(&self) -> (i32, i32, i32) {
        (
            self.across * HOW_WIDE_AN_AREA_IS + HOW_WIDE_AN_AREA_IS / 2,
            self.down * HOW_WIDE_AN_AREA_IS + HOW_WIDE_AN_AREA_IS / 2,
            0,
        )
    }

    /// How many areas apart two are.
    pub fn apart(&self, other: &Area) -> i32 {
        (self.across - other.across).abs() + (self.down - other.down).abs()
    }
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.across, self.down)
    }
}

impl From<Area> for String {
    fn from(area: Area) -> Self {
        area.to_string()
    }
}

impl TryFrom<String> for Area {
    type Error = String;

    fn try_from(written: String) -> Result<Self, Self::Error> {
        let (across, down) = written
            .split_once(',')
            .ok_or_else(|| format!("not an area: {written}"))?;
        match (across.parse(), down.parse()) {
            (Ok(across), Ok(down)) => Ok(Area { across, down }),
            _ => Err(format!("not an area: {written}")),
        }
    }
}

/// What an agent carries away from having been somewhere.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Impression {
    /// How much of what is there this one could tell you about, 0.0 to 1.0.
    pub detail: f32,
    /// The day it was last looked at.
    pub seen_on: u32,
    /// And the day the impression was last added to, which is not the same:
    /// standing in a field all week is one day's worth of learning it.
    pub topped_up_on: u32,
}

/// A place that answered a need, remembered as an area and kept for years.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    /// Roughly where. Not exactly where - that is the point.
    pub area: Area,
    /// What it answered.
    pub answered: DriveType,
    /// And what was there. "Berries", "water".
    pub what: String,
    /// The day it was found out.
    pub learned_on: u32,
}

/// Everything an agent knows about where things are.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Whereabouts {
    general: BTreeMap<Area, Impression>,
    /// Keyed by the area and the need, so a berry patch and a spring in the
    /// same valley are two landmarks and two berry patches are one.
    important: BTreeMap<String, Landmark>,
    /// The day the forgetting last ran.
    #[serde(default)]
    tidied_on: u32,
}

impl Whereabouts {
    /// What a day's looking adds to an impression.
    pub const WHAT_A_LOOK_IS_WORTH: f32 = 0.05;

    /// How long an unvisited area keeps what it has before it starts going.
    pub const HOW_LONG_BEFORE_IT_STARTS_GOING: u32 = DAYS_PER_MONTH;

    /// And what a month of not going back then costs.
    pub const WHAT_A_MONTH_AWAY_COSTS: f32 = 0.05;

    /// Below this an impression is nothing at all and is dropped.
    pub const TOO_FAINT_TO_BE_AN_IMPRESSION: f32 = 0.001;

    /// How long somebody remembers that a need was answered over there.
    ///
    /// Five years. A great deal longer than anything else in this model keeps
    /// anything, and reasonably so: "there is food in that valley" is the kind
    /// of thing people carry for life, and it is cheap to carry because it is
    /// an area and a name rather than a map.
    pub const HOW_LONG_AN_IMPORTANT_PLACE_KEEPS: u32 = 5 * DAYS_PER_YEAR;

    /// This one looked at an area today.
    ///
    /// At most once a day counts, however long they stand there.
    pub fn looked_at(&mut self, area: Area, today: u32) {
        let impression = self.general.entry(area).or_default();
        impression.seen_on = today;

        // `or_default` gives `topped_up_on = 0`, which on day zero would read
        // as "already topped up today". A brand new impression is always
        // topped up, which is what makes the first look worth something.
        let brand_new = impression.detail == 0.0;
        if brand_new || impression.topped_up_on < today {
            impression.detail = (impression.detail + Self::WHAT_A_LOOK_IS_WORTH).min(1.0);
            impression.topped_up_on = today;
        }
    }

    /// How well this one knows an area, 0.0 to 1.0.
    ///
    /// This is the remembered impression. What an agent knows about the ground
    /// it is *standing on* is not this and does not come from here - while you
    /// are looking at a place you can see all of it.
    pub fn how_well_i_know(&self, area: &Area) -> f32 {
        self.general
            .get(area)
            .map(|impression| impression.detail)
            .unwrap_or(0.0)
    }

    /// Note that a need was answered in this area.
    pub fn it_answered_here(
        &mut self,
        area: Area,
        answered: DriveType,
        what: &str,
        today: u32,
    ) {
        let key = format!("{area}/{answered:?}");
        self.important.insert(
            key,
            Landmark {
                area,
                answered,
                what: what.to_string(),
                learned_on: today,
            },
        );
    }

    /// Whether this one knows anywhere that answers a need.
    pub fn anywhere_that_answers(&self, need: DriveType) -> impl Iterator<Item = &Landmark> {
        self.important
            .values()
            .filter(move |landmark| landmark.answered == need)
    }

    /// Whether this area is one of the important ones, for any need.
    pub fn is_important(&self, area: &Area) -> bool {
        self.important
            .values()
            .any(|landmark| landmark.area == *area)
    }

    /// Let go of what has not been looked at, and of landmarks older than a
    /// life's worth of remembering.
    ///
    /// Charged by the day: safe to call every turn.
    pub fn forget_what_has_not_been_seen(&mut self, today: u32) {
        if today <= self.tidied_on {
            return;
        }
        self.tidied_on = today;

        for impression in self.general.values_mut() {
            let away = today.saturating_sub(impression.seen_on);
            if away <= Self::HOW_LONG_BEFORE_IT_STARTS_GOING {
                continue;
            }

            // A month's grace, then five points a month. Counted from the
            // impression's own clock rather than from how long ago it was
            // last tidied, so calling this once or a hundred times between
            // two days makes no difference to what is left.
            let months = (away - Self::HOW_LONG_BEFORE_IT_STARTS_GOING) / DAYS_PER_MONTH;
            let lost = months as f32 * Self::WHAT_A_MONTH_AWAY_COSTS;
            impression.detail = (impression.detail - lost).max(0.0);
        }

        self.general
            .retain(|_, impression| impression.detail > Self::TOO_FAINT_TO_BE_AN_IMPRESSION);

        self.important.retain(|_, landmark| {
            today.saturating_sub(landmark.learned_on) <= Self::HOW_LONG_AN_IMPORTANT_PLACE_KEEPS
        });
    }

    /// How many areas are held at all. For the instruments.
    pub fn how_many_areas(&self) -> usize {
        self.general.len()
    }

    /// And how many important places.
    pub fn how_many_important_places(&self) -> usize {
        self.important.len()
    }

    /// Every area held, best known first. For the instruments and for whoever
    /// is deciding what to keep.
    pub fn areas(&self) -> impl Iterator<Item = (&Area, &Impression)> {
        self.general.iter()
    }
}
