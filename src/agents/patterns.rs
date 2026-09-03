// src/agents/patterns.rs
//! What answered what, and what the answers had in common.
//!
//! "When an agent satisfies drive demand, it links its previous actions taken
//! to the drive satisfaction to form a pattern. (e.g., travel to + specific
//! location = water)."
//!
//! The first version of this wrote one line per (need, thing done) and hung a
//! single place off it. That records an episode but it cannot generalise from
//! two, and generalising from two is the whole of the thing. A man who hunts
//! out east and eats, and hunts out west and eats, has learned that *hunting*
//! answers hunger. He has not learned that *east* does. Both episodes say so,
//! and the only way to read it off them is to notice which parts they share.
//!
//! So an episode here is not a key. It is a handful of elements - what was
//! done, what it was done to, the ground it was done on, which way that ground
//! lies from home, what time of year it was - and every one of them is
//! reinforced when the need is answered. The element that is there every time
//! climbs on every success; the element that varies climbs on its own
//! successes only and is overtaken. Nobody has to decide which part mattered.
//! Arithmetic decides, out of the agent's own history.
//!
//! What that buys, past the generalising:
//!
//! - **Trails, not records.** Strength goes up on a success and down with
//!   time, so what an agent knows is a landscape of worn paths rather than a
//!   filing cabinet. A path nobody walks grows over. This is why an agent need
//!   not hold the whole map: it holds the parts of it that have paid.
//! - **Similarity for free.** Two situations are alike to the degree that they
//!   share elements, so when a trail goes cold there is already an answer to
//!   "what is the nearest thing to this that has worked" - see
//!   `something_like_it`.
//! - **A place to hang worry.** An element can carry what it has *cost* as
//!   well as what it has paid, against the drive it cost it to - see
//!   `Trail::threatens` and `Agent`'s worry. Stealing answers hunger and
//!   endangers standing, and both facts are written against the same elements.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::core::DriveType;
use crate::environment::seasons::{Season, DAYS_PER_SEASON, TICKS_PER_DAY};

/// Which way a place lies from home.
///
/// The coarse form of a position, and the one that can be right about a place
/// the agent has not stood on. "Out east" is a thing several trips can share;
/// a tile is not, and an agent that only ever learns tiles learns nothing it
/// can carry to new ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Bearing {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Bearing {
    /// Which way `there` lies from `home`.
    ///
    /// Returns nothing for a place close enough to home that a direction would
    /// be noise rather than a bearing.
    pub fn from_home(home: (i32, i32, i32), there: (i32, i32, i32)) -> Option<Self> {
        let east = there.0 - home.0;
        let north = there.1 - home.1;

        if east.abs() + north.abs() < Self::NEAR_ENOUGH_TO_HAVE_NO_DIRECTION {
            return None;
        }

        // The eight-point rose: a leg counts as diagonal when the shorter of
        // the two is at least this much of the longer.
        let long = east.abs().max(north.abs()) as f32;
        let short = east.abs().min(north.abs()) as f32;
        let diagonal = short >= long * Self::WHAT_MAKES_IT_A_CORNER;

        if diagonal {
            // Both legs are non-zero: a leg of zero cannot be at least half
            // of a positive longer leg.
            return Some(match (east.signum(), north.signum()) {
                (1, 1) => Bearing::NorthEast,
                (1, -1) => Bearing::SouthEast,
                (-1, 1) => Bearing::NorthWest,
                _ => Bearing::SouthWest,
            });
        }

        // Otherwise the longer leg names it, and the shorter one is a wobble
        Some(if north.abs() > east.abs() {
            if north > 0 {
                Bearing::North
            } else {
                Bearing::South
            }
        } else if east > 0 {
            Bearing::East
        } else {
            Bearing::West
        })
    }

    /// Inside this many tiles of home, a direction says nothing.
    const NEAR_ENOUGH_TO_HAVE_NO_DIRECTION: i32 = 3;

    /// How square a corner has to be before it is a corner rather than a side.
    const WHAT_MAKES_IT_A_CORNER: f32 = 0.5;

    pub fn all() -> [Bearing; 8] {
        [
            Bearing::North,
            Bearing::NorthEast,
            Bearing::East,
            Bearing::SouthEast,
            Bearing::South,
            Bearing::SouthWest,
            Bearing::West,
            Bearing::NorthWest,
        ]
    }

    fn as_str(&self) -> &'static str {
        match self {
            Bearing::North => "N",
            Bearing::NorthEast => "NE",
            Bearing::East => "E",
            Bearing::SouthEast => "SE",
            Bearing::South => "S",
            Bearing::SouthWest => "SW",
            Bearing::West => "W",
            Bearing::NorthWest => "NW",
        }
    }

    fn from_str(what: &str) -> Option<Self> {
        Bearing::all().into_iter().find(|b| b.as_str() == what)
    }
}

/// One thing that was true when something was done.
///
/// Serialised as a string, because these are map keys and JSON has no others.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Element {
    /// What was done. "hunt", "gather", "drink".
    Did(String),
    /// What it was done to, or with. "Deer", "Berries", "spear".
    On(String),
    /// The ground it was done on.
    At((i32, i32, i32)),
    /// Which way that ground lies from home.
    Toward(Bearing),
    /// The time of year it was done in.
    When(Season),
}

impl Element {
    /// Whether this element names a place an agent could walk to.
    pub fn is_a_place(&self) -> bool {
        matches!(self, Element::At(_))
    }

    /// The ground this element names, if it names any.
    pub fn place(&self) -> Option<(i32, i32, i32)> {
        match self {
            Element::At(where_it_was) => Some(*where_it_was),
            _ => None,
        }
    }

    /// The thing done, if this element is one.
    pub fn what_was_done(&self) -> Option<&str> {
        match self {
            Element::Did(what) => Some(what.as_str()),
            _ => None,
        }
    }
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Element::Did(what) => write!(f, "did:{}", what),
            Element::On(what) => write!(f, "on:{}", what),
            Element::At((x, y, z)) => write!(f, "at:{},{},{}", x, y, z),
            Element::Toward(bearing) => write!(f, "toward:{}", bearing.as_str()),
            Element::When(season) => write!(f, "when:{:?}", season),
        }
    }
}

impl From<Element> for String {
    fn from(element: Element) -> Self {
        element.to_string()
    }
}

impl TryFrom<String> for Element {
    type Error = String;

    fn try_from(written: String) -> Result<Self, Self::Error> {
        let (kind, rest) = written
            .split_once(':')
            .ok_or_else(|| format!("not an element: {}", written))?;

        match kind {
            "did" => Ok(Element::Did(rest.to_string())),
            "on" => Ok(Element::On(rest.to_string())),
            "at" => {
                let mut legs = rest.split(',').map(|leg| leg.parse::<i32>());
                match (legs.next(), legs.next(), legs.next()) {
                    (Some(Ok(x)), Some(Ok(y)), Some(Ok(z))) => Ok(Element::At((x, y, z))),
                    _ => Err(format!("not a place: {}", rest)),
                }
            }
            "toward" => Bearing::from_str(rest)
                .map(Element::Toward)
                .ok_or_else(|| format!("not a bearing: {}", rest)),
            "when" => match rest {
                "Spring" => Ok(Element::When(Season::Spring)),
                "Summer" => Ok(Element::When(Season::Summer)),
                "Fall" => Ok(Element::When(Season::Fall)),
                "Winter" => Ok(Element::When(Season::Winter)),
                _ => Err(format!("not a season: {}", rest)),
            },
            _ => Err(format!("not an element: {}", written)),
        }
    }
}

/// How worn one element's path is, for one need.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Trail {
    /// How worn the path is. Success adds, time takes away. This is the
    /// pheromone: nothing about it says what the element *means*, only how
    /// much of the agent's history has run over it.
    pub strength: f32,
    /// How many times this element was there when the need was answered. Kept
    /// beside the strength because "three times is a habit" is a count and not
    /// a weight - a trail can be strong off one enormous success, and one
    /// enormous success is not yet a thing you walk across a map for.
    pub times: u32,
    /// When it last was.
    pub last_worked: u32,
    /// What this element has cost, and which drive it cost it to.
    ///
    /// Worry. Kept against the drive that took the loss, because how long it
    /// takes to stop worrying depends on how much that drive matters - see
    /// `Patterns::fade`.
    #[serde(default)]
    pub threatens: BTreeMap<DriveType, f32>,
}

impl Trail {
    /// What this trail is worth to an agent weighing it up: what it has paid,
    /// less what it is expected to cost.
    pub fn worth(&self) -> f32 {
        self.strength - self.threatens.values().sum::<f32>()
    }

    /// How much this element is expected to cost a particular drive.
    pub fn threat_to(&self, drive: DriveType) -> f32 {
        self.threatens.get(&drive).copied().unwrap_or(0.0)
    }
}

/// Everything an agent has noticed about what answers what.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Patterns {
    /// Keyed by the need answered, then by the element that was there when it
    /// was. Nested rather than keyed by a pair so that it survives a round
    /// trip through a format whose map keys are strings.
    against: BTreeMap<DriveType, BTreeMap<Element, Trail>>,
    /// The tick the trails were last faded on, so fading can be charged by
    /// the day however often it is asked for.
    #[serde(default)]
    faded_at: u32,
    /// What was done lately, and what need it was doing it for.
    ///
    /// A consequence does not always arrive in the turn that earned it. A man
    /// who takes food from somebody finds out what it cost him when the camp
    /// next turns its back on him, which may be days later, and by then the
    /// taking is over and there is nothing to attach the lesson to unless
    /// somebody kept a note of it. This is the note.
    ///
    /// Deliberately short. An agent that could lay a grievance from last
    /// spring at the door of something it did last spring would be a better
    /// reasoner than anybody in this model is meant to be, and a worse one at
    /// the thing that matters, which is noticing that *this* keeps costing me.
    #[serde(default)]
    lately: Vec<(u32, DriveType, Vec<Element>)>,
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
    /// A season, and now actually a season. This read 288 - twenty-four days
    /// at twelve turns to the day - against a comment saying "a season", which
    /// was right on some earlier calendar and has been wrong since a season
    /// became ninety days. It is derived here so it cannot drift again.
    pub const STILL_WORTH_THE_WALK: u32 = DAYS_PER_SEASON * TICKS_PER_DAY;

    /// What a day takes off an unreinforced trail.
    ///
    /// Two per cent. Over a season that leaves about a sixth of what was
    /// there, which is the shape wanted: a path walked once in the spring is
    /// gone by the autumn, a path walked weekly is still there years later.
    /// Memory is not required to stay accurate, only to stay useful.
    pub const WHAT_A_DAY_TAKES: f32 = 0.02;

    /// Below this a trail is not worth the room it takes, and is forgotten.
    ///
    /// This is what keeps an agent from holding the whole map. Nothing prunes
    /// by age or by count; things go because nobody walked them.
    pub const TOO_FAINT_TO_FOLLOW: f32 = 0.05;

    /// The most any one success can add.
    ///
    /// A single spectacular result should not settle the question for good;
    /// what makes a path is being walked, not being walked once.
    pub const WHAT_ONE_SUCCESS_IS_WORTH: f32 = 1.0;

    /// How many trails an agent holds against any one need before the faintest
    /// start going.
    ///
    /// A hard ceiling behind the fading, so that an agent which is answering
    /// one need in fifty different places cannot grow without bound between
    /// two fadings.
    pub const AS_MANY_TRAILS_AS_ANYBODY_HOLDS: usize = 64;

    /// Note that these elements were all present when this need was answered,
    /// and how efficiently it was answered.
    ///
    /// `efficiency` is how much demand came off per turn spent - see
    /// `Agent::how_efficiently_that_went`. It is what is added to every
    /// element, which is the whole mechanism: the element that is there for
    /// all of them gets all of the additions.
    pub fn it_worked(
        &mut self,
        need: DriveType,
        elements: &[Element],
        efficiency: f32,
        now: u32,
    ) {
        let earned = efficiency.clamp(0.0, Self::WHAT_ONE_SUCCESS_IS_WORTH);
        if earned <= 0.0 {
            return;
        }

        let against = self.against.entry(need).or_default();

        for element in elements {
            let trail = against.entry(element.clone()).or_default();
            trail.strength += earned;
            trail.times = trail.times.saturating_add(1);
            trail.last_worked = now;
        }

        Self::shed_the_faintest(against);

        self.lately.push((now, need, elements.to_vec()));
        self.forget_what_is_too_old_to_blame(now);
    }

    /// How long anybody connects a consequence back to what caused it.
    ///
    /// Three days. Long enough that a grudge shown the morning after is still
    /// laid at the right door, short enough that an agent does not blame its
    /// hunting for the weather.
    pub const AS_LONG_AS_ANYBODY_CONNECTS: u32 = TICKS_PER_DAY * 3;

    /// The most any one consequence can add to a worry.
    ///
    /// Being caught once should make somebody wary, not paralysed. What makes
    /// a man stop doing a thing is being caught at it repeatedly, which this
    /// allows and one bad afternoon does not.
    pub const WHAT_ONE_CONSEQUENCE_IS_WORTH: f32 = 0.5;

    fn forget_what_is_too_old_to_blame(&mut self, now: u32) {
        self.lately
            .retain(|(when, _, _)| now.saturating_sub(*when) <= Self::AS_LONG_AS_ANYBODY_CONNECTS);
    }

    /// Something has just cost this agent future satisfaction of a drive.
    ///
    /// "An agent might avoid stealing to prevent the loss of future
    /// socialization drive demand if the theft is discovered."
    ///
    /// Whatever it has been doing these last few days is what gets the blame,
    /// against the drive that took the loss. That is coarse, and it is meant
    /// to be: an agent has no way of knowing which of the things it did the
    /// camp is angry about, only that it did them and now the camp is angry.
    /// What sorts it out is repetition - the element that is there every time
    /// somebody turns their back accumulates the worry, and the elements that
    /// happened to be there once do not, by exactly the arithmetic that sorts
    /// out what *answers* a need.
    ///
    /// Returns how much worry this laid down, which is what the agent feels.
    pub fn it_cost_me(&mut self, cost_to: DriveType, how_much: f32, now: u32) -> f32 {
        self.forget_what_is_too_old_to_blame(now);

        let blamed = how_much.clamp(0.0, Self::WHAT_ONE_CONSEQUENCE_IS_WORTH);
        if blamed <= 0.0 || self.lately.is_empty() {
            return 0.0;
        }

        let mut laid_down = 0.0;
        // Cloned because the blaming borrows the trails mutably, and what is
        // being blamed is the record of what was done rather than the trails.
        let lately = self.lately.clone();

        for (_, served, elements) in &lately {
            let Some(against) = self.against.get_mut(served) else {
                continue;
            };
            for element in elements {
                if let Some(trail) = against.get_mut(element) {
                    let worry = trail.threatens.entry(cost_to).or_insert(0.0);
                    *worry += blamed;
                    laid_down += blamed;
                }
            }
        }

        laid_down
    }

    /// How much this agent dreads doing this, for this need.
    ///
    /// The sum of what these elements have cost it before. This is the whole
    /// of what worry does to a decision: it is subtracted from what the thing
    /// is expected to pay, so a man weighs the meal against the shunning
    /// rather than only the meal.
    pub fn what_i_dread(&self, need: DriveType, elements: &[Element]) -> f32 {
        let Some(against) = self.against.get(&need) else {
            return 0.0;
        };

        elements
            .iter()
            .filter_map(|element| against.get(element))
            .map(|trail| trail.threatens.values().sum::<f32>())
            .sum()
    }

    /// How much this agent fears for one particular drive, across everything
    /// it does.
    ///
    /// The other half of worry. `what_i_dread` asks "what will this cost me";
    /// this asks "what is at risk", which is what makes a worried agent go and
    /// shore the thing up rather than merely decline to do anything.
    pub fn how_much_i_fear_for(&self, drive: DriveType) -> f32 {
        self.against
            .values()
            .flat_map(|against| against.values())
            .map(|trail| trail.threat_to(drive))
            .sum()
    }

    /// And what this agent dreads across everything it knows: the felt total,
    /// which is what the emotion is.
    pub fn everything_i_dread(&self) -> f32 {
        self.against
            .values()
            .flat_map(|against| against.values())
            .map(|trail| trail.threatens.values().sum::<f32>())
            .sum()
    }

    /// Lay a worry down directly, without anything having happened.
    ///
    /// For worry that was not earned by this agent's own history: what it saw
    /// happen to somebody else, and what it absorbed as a child. A founder who
    /// has never stolen still has something to lose, and without this the only
    /// direction worry could ever move is down.
    pub fn taught_to_dread(
        &mut self,
        need: DriveType,
        element: Element,
        cost_to: DriveType,
        how_much: f32,
    ) {
        if how_much <= 0.0 {
            return;
        }
        let trail = self.against.entry(need).or_default().entry(element).or_default();
        *trail.threatens.entry(cost_to).or_insert(0.0) += how_much;
    }

    /// Note that these elements were there and the need was *not* answered.
    ///
    /// The trail is not erased - a river that was dry today is still a river -
    /// but it is walked back, so ground that stops working stops being worth
    /// the walk. The same arithmetic that generalises a success generalises a
    /// failure: what all the failures share loses the most.
    pub fn it_did_not(&mut self, need: DriveType, elements: &[Element]) {
        let Some(against) = self.against.get_mut(&need) else {
            return;
        };

        for element in elements {
            if let Some(trail) = against.get_mut(element) {
                trail.strength = (trail.strength - Self::WHAT_A_FAILURE_COSTS).max(0.0);
                trail.times = trail.times.saturating_sub(1);
            }
        }

        against.retain(|_, trail| trail.strength > Self::TOO_FAINT_TO_FOLLOW);
    }

    /// What one failure walks back off a trail.
    ///
    /// Less than a good success adds. An agent that tries a good place on a
    /// bad day should not conclude the place is bad; it should conclude it a
    /// little less strongly each time until the place stops being worth it.
    const WHAT_A_FAILURE_COSTS: f32 = 0.3;

    /// Take time off every trail, and forget what has gone too faint.
    ///
    /// Charged by the day: call it as often as you like, it will only take
    /// what the calendar says has passed since it last ran. Worry fades here
    /// too, but on its own clock - see `how_fast_worry_fades`.
    pub fn fade(&mut self, now: u32) {
        let days = now.saturating_sub(self.faded_at) / TICKS_PER_DAY;
        if days == 0 {
            return;
        }
        self.faded_at = now;

        let kept = (1.0 - Self::WHAT_A_DAY_TAKES).powi(days as i32);

        for against in self.against.values_mut() {
            for trail in against.values_mut() {
                trail.strength *= kept;

                for (drive, worry) in trail.threatens.iter_mut() {
                    *worry *= how_fast_worry_fades(*drive).powi(days as i32);
                }
                trail
                    .threatens
                    .retain(|_, worry| *worry > Self::TOO_FAINT_TO_FOLLOW);
            }
            against.retain(|_, trail| {
                trail.strength > Self::TOO_FAINT_TO_FOLLOW || !trail.threatens.is_empty()
            });
        }

        self.against.retain(|_, against| !against.is_empty());
    }

    /// Drop the faintest trails when there are more than anybody holds.
    fn shed_the_faintest(against: &mut BTreeMap<Element, Trail>) {
        if against.len() <= Self::AS_MANY_TRAILS_AS_ANYBODY_HOLDS {
            return;
        }

        let mut by_strength: Vec<(Element, f32)> = against
            .iter()
            .map(|(element, trail)| (element.clone(), trail.strength))
            .collect();
        // Weakest first, and the element breaks the tie so two agents in the
        // same state shed the same trail.
        by_strength.sort_by(|(left_element, left), (right_element, right)| {
            left.partial_cmp(right)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left_element.cmp(right_element))
        });

        let too_many = against.len() - Self::AS_MANY_TRAILS_AS_ANYBODY_HOLDS;
        for (element, _) in by_strength.into_iter().take(too_many) {
            against.remove(&element);
        }
    }

    /// What a child takes from the people who raised it.
    ///
    /// "Worry should come from an Agent's history and childhood." A newborn has
    /// no history, and if worry could only ever be earned then the first thing
    /// every child would learn is that nothing costs anything - it would take
    /// what it liked until the camp turned on it, and every generation would
    /// have to find that out again from scratch. Nobody is raised that way.
    ///
    /// What passes is the worry and not the trails: a child does not inherit
    /// its parents' map, or their favourite bushes, or what they knew how to
    /// do. It inherits what they were wary of - and only a share of it, so
    /// that a fear has to be re-earned to stay as sharp as it was.
    pub fn what_the_child_takes_from(&mut self, raised_by: &[&Patterns]) {
        for theirs in raised_by {
            for (need, against) in &theirs.against {
                for (element, trail) in against {
                    for (cost_to, worry) in &trail.threatens {
                        self.taught_to_dread(
                            *need,
                            element.clone(),
                            *cost_to,
                            worry * Self::WHAT_A_CHILD_TAKES_ON,
                        );
                    }
                }
            }
        }
    }

    /// How much of somebody else's worry a child starts out carrying.
    ///
    /// A third. Enough that a child of careful people is careful, little
    /// enough that it will find out for itself whether the care was warranted.
    pub const WHAT_A_CHILD_TAKES_ON: f32 = 1.0 / 3.0;

    /// How much of a worry somebody picks up from watching it happen to
    /// somebody else.
    ///
    /// Less than being on the receiving end. Seeing a man shunned for taking
    /// food teaches you something about taking food; it does not teach you as
    /// much as being shunned does.
    pub const WHAT_WATCHING_IT_HAPPEN_TEACHES: f32 = 0.04;

    /// How worn one element's path is for a need.
    pub fn strength(&self, need: DriveType, element: &Element) -> f32 {
        self.against
            .get(&need)
            .and_then(|against| against.get(element))
            .map(|trail| trail.strength)
            .unwrap_or(0.0)
    }

    /// The whole trail for one element, if there is one.
    pub fn trail(&self, need: DriveType, element: &Element) -> Option<&Trail> {
        self.against.get(&need)?.get(element)
    }

    /// How often a particular thing has answered a particular need.
    pub fn how_often(&self, need: DriveType, what: &str) -> u32 {
        self.against
            .get(&need)
            .and_then(|against| against.get(&Element::Did(what.to_string())))
            .map(|trail| trail.times)
            .unwrap_or(0)
    }

    /// What this agent has found answers a need: the best-worn thing it knows
    /// how to do about it.
    ///
    /// Only `Did` elements, because this answers "what should I try" and a
    /// season is not something anybody can try.
    pub fn what_answers(&self, need: DriveType) -> Option<(&str, &Trail)> {
        self.against
            .get(&need)?
            .iter()
            .filter_map(|(element, trail)| Some((element.what_was_done()?, trail)))
            .max_by(|(left_what, left), (right_what, right)| {
                left.worth()
                    .partial_cmp(&right.worth())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left_what.cmp(right_what))
            })
    }

    /// Ground worth going back to for a need, if there is any.
    ///
    /// Only a place that has worked often enough to be a habit rather than an
    /// accident, and recently enough to still be there. Worth rather than
    /// strength, so a place an agent has reason to dread is not walked to
    /// merely because it has fed him before.
    pub fn where_it_worked(&self, need: DriveType, now: u32) -> Option<(i32, i32, i32)> {
        self.places_worth_the_walk(need, now)
            .max_by(|(left_place, left), (right_place, right)| {
                left.partial_cmp(right)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left_place.cmp(right_place))
            })
            .map(|(place, _)| place)
    }

    /// Every place this agent would still walk to for a need, with what each
    /// is worth to it.
    pub fn places_worth_the_walk(
        &self,
        need: DriveType,
        now: u32,
    ) -> impl Iterator<Item = ((i32, i32, i32), f32)> + '_ {
        self.against
            .get(&need)
            .into_iter()
            .flat_map(|against| against.iter())
            .filter(move |(_, trail)| trail.times >= Self::A_HABIT_BY_NOW)
            .filter(move |(_, trail)| {
                now.saturating_sub(trail.last_worked) <= Self::STILL_WORTH_THE_WALK
            })
            .filter_map(|(element, trail)| Some((element.place()?, trail.worth())))
            .filter(|(_, worth)| *worth > 0.0)
    }

    /// Which way this agent would set off for a need, when it knows no
    /// particular place.
    ///
    /// The generalisation that a tile cannot make. A man who has eaten off
    /// three different bushes to the east has learned something about the east
    /// even though he has not learned a bush.
    pub fn which_way_it_lies(&self, need: DriveType) -> Option<Bearing> {
        self.against
            .get(&need)?
            .iter()
            .filter_map(|(element, trail)| match element {
                Element::Toward(bearing) => Some((*bearing, trail.worth())),
                _ => None,
            })
            .filter(|(_, worth)| *worth > 0.0)
            .max_by(|(left_bearing, left), (right_bearing, right)| {
                left.partial_cmp(right)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left_bearing.cmp(right_bearing))
            })
            .map(|(bearing, _)| bearing)
    }

    /// How many needs this agent has worked out an answer to.
    pub fn how_much_i_have_worked_out(&self) -> usize {
        self.against
            .values()
            .flat_map(|against| against.values())
            .filter(|trail| trail.times >= Self::A_HABIT_BY_NOW)
            .count()
    }

    /// Whether anything at all has been noticed.
    pub fn is_empty(&self) -> bool {
        self.against.is_empty()
    }

    /// How many trails are held in total. For the instruments.
    pub fn how_many_trails(&self) -> usize {
        self.against.values().map(|against| against.len()).sum()
    }

    /// Every need this agent has any trail against.
    pub fn needs_with_trails(&self) -> impl Iterator<Item = DriveType> + '_ {
        self.against.keys().copied()
    }
}

/// What a day leaves of a worry about a given drive.
///
/// "Worry decreases gradually as time passes but at a rate depending on the
/// importance of the drive demand. A drive demand with little importance
/// should quickly decay, but one with high importance should slowly decay.
/// This should vary from a day to a month."
///
/// So: the half-life runs from one day for the least important thing an agent
/// can worry about to a month for the most. A man who was once caught taking
/// food stays wary of it for weeks; a man who was once embarrassed is over it
/// tomorrow. Returned as what survives a day, which is what `fade` multiplies
/// by.
pub fn how_fast_worry_fades(about: DriveType) -> f32 {
    0.5f32.powf(1.0 / how_long_a_worry_lasts(about))
}

/// The half-life of a worry about a drive, in days.
///
/// Importance is the drive's own rank and not a second opinion about it: the
/// bands already say which drives kill you, which decide whether your people
/// are here in ten years, and which decide what sort of place they live in.
/// A month for the first, a fortnight for the second, a day for the third.
pub fn how_long_a_worry_lasts(about: DriveType) -> f32 {
    use crate::core::drives::DriveRank;
    use crate::environment::seasons::DAYS_PER_MONTH;

    /// The shortest a worry lasts, in days.
    const A_DAY: f32 = 1.0;

    let importance = match about.rank() {
        DriveRank::Primary => 1.0,
        DriveRank::Secondary => 0.5,
        DriveRank::Tertiary => 0.0,
    };

    A_DAY + (DAYS_PER_MONTH as f32 - A_DAY) * importance
}
