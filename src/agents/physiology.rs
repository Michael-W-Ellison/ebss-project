// src/agents/physiology.rs
//! What a body runs on, kept in minutes.
//!
//! Everything else in this model counts turns. A turn is a decision - the unit
//! at which an agent looks around and picks something to do - and there are
//! twelve of them in a day, so a turn is two hours of living.
//!
//! A body does not work at that resolution. Water leaves it steadily over
//! three days; a meal sits in the stomach for half an hour before anything
//! moves, and is gone from it in six; what leaves the stomach is worth nothing
//! for a further day. None of that can be said in two-hour steps.
//!
//! So the body keeps its own clock, in minutes, and `MINUTES_PER_TURN` of it
//! passes every turn. The physiology below is written in the units it was
//! specified in and does not care how coarse the decision loop is. If the
//! calendar is ever made finer, `MINUTES_PER_TURN` follows it down and nothing
//! here changes.
//!
//! This replaces `ticks_without_food` and `ticks_without_water`, which counted
//! turns against thresholds written for a one-minute tick and were therefore a
//! hundred and twenty times too slow to ever fire. See ISSUES #73.

use serde::{Deserialize, Serialize};

/// Minutes in a day.
pub const MINUTES_PER_DAY: u32 = 1440;

/// How much living one turn of the simulation covers.
///
/// Derived, so that making the decision loop finer makes the body's steps
/// smaller rather than making the body wrong.
pub const MINUTES_PER_TURN: u32 = MINUTES_PER_DAY / crate::environment::seasons::TICKS_PER_DAY;

/// Three days without water and an adult is dead.
pub const MINUTES_TO_DIE_OF_THIRST: u32 = 3 * MINUTES_PER_DAY;

/// Three weeks without food and an adult is dead.
pub const MINUTES_TO_STARVE: u32 = 21 * MINUTES_PER_DAY;

/// What a body burns in a day at an ordinary level of activity.
///
/// One unit a minute. That is where the figure comes from, and it is why a day
/// and a day's food are the same number.
pub const UNITS_BURNED_IN_AN_ORDINARY_DAY: f32 = MINUTES_PER_DAY as f32;

/// What an adult's stomach holds.
pub const STOMACH_CAPACITY: f32 = 600.0;

/// What one gathered thing comes to in a stomach - a berry, a fish, a root.
///
/// "A fish provides 4-6 units of food at 25 food energy per unit." Five, then:
/// an item in a pack is a handful of something rather than a day's worth, and
/// what a stomach holds is a hundred and twenty of them.
pub const UNITS_IN_ONE_ITEM: f32 = 5.0;

/// What a body is trying to get out of one sitting down to eat.
///
/// A third of a day, in energy. Not a volume: how much *food* that comes to
/// depends entirely on what the food is, which is the whole point of caloric
/// density. Twenty units of ordinary forage, four of a fat carcass, eighty of
/// spring leaf.
pub const WHAT_A_SITTING_AIMS_AT: f32 = UNITS_BURNED_IN_AN_ORDINARY_DAY / 3.0;

/// How long what has left the stomach sits in the gut before it is worth
/// anything.
pub const MINUTES_TO_DIGEST: u32 = MINUTES_PER_DAY;

/// How long a drink takes to tell.
pub const MINUTES_FOR_A_DRINK_TO_TELL: u32 = 20;

/// The reserve a full-grown body carries, in food units.
///
/// Three weeks of an ordinary day's burn, which is the same statement as "an
/// adult starves in three weeks" written the other way round.
pub const RESERVE_OF_A_GROWN_BODY: f32 = MINUTES_TO_STARVE as f32;

/// What a good long drink at the water is worth.
///
/// A day's water. Somebody who has walked to the spring drinks their fill
/// while they are there, and that is the day's drinking done - which is the
/// same reckoning as "carrying a container means that for the rest of the day
/// the agent has drinking water", for somebody with no container.
///
/// It cannot be a whole skinful, or the bands would never be felt: a body that
/// could go from empty to full in one action would never go short. At a day's
/// worth a body at rest drinks about once a day and one working hard more
/// often, and three days of not finding water still kills it.
pub const A_DRINK_IS_WORTH: f32 = 1.0 / 3.0;

/// The energy figure in `NutritionalContent` that counts as ordinary food.
///
/// The database runs from six (spring greens) to eighty (fat and nuts). The
/// middle of that range is forty, but the middle of the range is not what
/// anybody eats: a forager lives on berries and roots, which sit at twenty to
/// thirty, and the eighties are fat and nuts that come in windfalls. Pricing
/// ordinary food at forty made everything actually being eaten worth about
/// half a unit, so a settlement on three meals a day still lost five hundred
/// units of reserve a day and starved with a full belly.
///
/// Used now only to price the *work* of getting a food - see
/// `how_much_work_this_food_is`. What a unit is worth to eat is the food's own
/// energy figure and nothing else; dividing by this was a twenty-five-fold
/// error that put every thin food below maintenance.
pub const ENERGY_OF_ORDINARY_FOOD: f32 = 25.0;

/// How far into its reserve a body has to be before going without counts as
/// starving rather than as having missed a meal.
///
/// Three days of a three-week reserve.
///
/// The gut empties about thirty hours after a meal, by which time a body is a
/// day and a quarter into its reserve - so three days is clear of anything
/// that happens between meals, and is what "starving" means in ordinary use.
/// Under it, a body with an empty gut has simply not eaten since yesterday,
/// which happens to everybody and is not a reason to do anything differently.
pub const DAYS_OF_RESERVE_BEFORE_IT_IS_STARVATION: f32 = 3.0;

/// What a unit of milk is worth against a unit of ordinary forage.
///
/// Rich - fat and sugar - which is the whole reason an infant can live on it
/// while its stomach holds a quarter of what its mother's does and it burns
/// more for its size than she does.
pub const WHAT_MILK_IS_WORTH: f32 = 2.0;

/// Below this share of its water a body starts to go short.
const FIRST_BAND: f32 = 0.75;

/// The point at which a body wants a drink.
///
/// Well above the point at which going without starts to cost it anything.
/// People drink several times a day and are never near going short; they do
/// not wait until their capabilities drop.
///
/// It matters more than it sounds. Thirst used to reach the drive's threshold
/// at four fifths of a full body, by which time an agent had spent half a day
/// on something else and might be a long walk from the water. Seven founders
/// in twelve died of thirst in spring, in a world with twenty-one springs in
/// it that never once ran dry.
const WANTS_A_DRINK_AT: f32 = 0.92;

/// How full a stomach stops a body with its reserve intact from wanting more.
///
/// A tenth of a sitting: near enough empty. Somebody who is not short of
/// anything eats three times a day and does not think about food in between.
const A_FULL_BODY_STOPS_AT: f32 = 0.10;

/// And how full stops a body that has eaten its reserve down to nothing.
///
/// Three quarters of a sitting still in the stomach and it is looking for the
/// next one. "Instead of waiting for their stomach to become empty, an agent
/// might get hungry while their stomach is still half full."
const A_SPENT_BODY_STOPS_AT: f32 = 0.75;

/// How the stomach empties into the gut, as (minutes since the meal, share of
/// it gone by then).
///
/// Nothing moves for the first half hour. Then an eighth every half hour for
/// two and a half hours - five eighths gone by three hours - and the last
/// three eighths an hour apart, so the stomach is empty six hours after the
/// meal. Read between the points rather than stepped, so that advancing the
/// clock two hours at a time gives the same answer as advancing it a minute at
/// a time.
const HOW_THE_STOMACH_EMPTIES: [(u32, f32); 10] = [
    (0, 0.0),
    (30, 0.0),
    (60, 1.0 / 8.0),
    (90, 2.0 / 8.0),
    (120, 3.0 / 8.0),
    (150, 4.0 / 8.0),
    (180, 5.0 / 8.0),
    (240, 6.0 / 8.0),
    (300, 7.0 / 8.0),
    (360, 8.0 / 8.0),
];

/// How long a meal keeps a body from wanting the next one, in minutes.
///
/// The last entry in the gastric schedule: six hours, after which the stomach
/// is empty. "An empty stomach is what a body feels, and it feels it about
/// five hours after eating, which is what puts three meals in a day."
pub const MINUTES_A_MEAL_HOLDS: u32 = HOW_THE_STOMACH_EMPTIES[HOW_THE_STOMACH_EMPTIES.len() - 1].0;

/// The same, in decision turns.
pub const TURNS_A_MEAL_HOLDS: f32 = MINUTES_A_MEAL_HOLDS as f32 / MINUTES_PER_TURN as f32;

/// What the three hunger tables come to for a body that is not in trouble.
///
/// A full reserve is one, an empty stomach is two, and a gut with less than
/// half a day behind it is two. This is the ordinary case - somebody who ate
/// six hours ago and has not eaten since - and it is what the Hunger drive's
/// climb is sized against, so that a body wants its next meal when its stomach
/// is empty rather than at a rate somebody picked.
pub const AN_ORDINARY_APPETITE: f32 = 1.0 * 2.0 * 2.0;

/// What share of a meal has left the stomach by the time it is this old.
pub fn share_of_a_meal_gone_by(age_in_minutes: u32) -> f32 {
    let age = age_in_minutes as f32;
    let last = HOW_THE_STOMACH_EMPTIES[HOW_THE_STOMACH_EMPTIES.len() - 1];
    if age_in_minutes >= last.0 {
        return last.1;
    }

    for pair in HOW_THE_STOMACH_EMPTIES.windows(2) {
        let (from_min, from_share) = pair[0];
        let (to_min, to_share) = pair[1];
        if age_in_minutes >= from_min && age_in_minutes < to_min {
            let across = (to_min - from_min) as f32;
            let into = age - from_min as f32;
            return from_share + (to_share - from_share) * (into / across);
        }
    }

    0.0
}

/// What one unit of this food is worth, in the energy a body burns.
///
/// The food's own figure, and nothing else done to it. "If a green supplies 6
/// energy per unit, and the agents can eat a maximum of 2,400 units of food per
/// day, how come they are not reaching the 14,400 energy units that they are
/// capable of?" - which is the arithmetic, and the answer was that this
/// function divided by `ENERGY_OF_ORDINARY_FOOD` first.
///
/// That division made a unit of leaf worth **a quarter of a unit of food**
/// rather than six units of energy: a twenty-five-fold error, and it put every
/// thin food below what a body burns however much of it there was. Greens are
/// meant to be livable - "they can still provide enough energy for the agents
/// to live off of plants alone, especially once they start farming" - and at a
/// quarter they never could be.
///
/// The scale is pinned by the fishery, which was specified with numbers that
/// close: a fish every two hours is one a turn, a fish is four to six units, a
/// unit of fish is twenty-five energy, so a day of fishing is 1,200 to 1,800
/// energy against the 1,440 a body burns. A people can live by fishing and it
/// is a full day's work, which is what was asked for.
///
/// This supersedes one line of the earlier specification - "three full meals
/// would result in an intake of 1800 food, which would exceed the 1440 needed"
/// - which reads the stomach's six hundred as being in the same currency as
/// the day's fourteen hundred and forty. It cannot be both: under that reading
/// twelve fish a day is three per cent of a day's food and nobody could live by
/// fishing. The fishery's numbers are the later and the more specific, so they
/// are the ones taken. The stomach's six hundred is a volume, and it is a
/// ceiling on gorging rather than a daily target.
pub fn what_a_unit_of_this_is_worth(energy: f32) -> f32 {
    energy.max(0.5)
}

/// How much work this food is to win and prepare, as a multiplier.
///
/// A separate question from what it is worth to eat, and it was being answered
/// with the same number. A carcass wants butchering and a root wants digging
/// where leaf is picked off the hedge and eaten where it stands, so density
/// stands in for the work - but only as a *ratio*, bounded, because a forage is
/// never eighty times the work of another forage.
pub fn how_much_work_this_food_is(energy: f32) -> f32 {
    (energy / ENERGY_OF_ORDINARY_FOOD).clamp(0.15, 2.5)
}

/// What a body is doing, as a multiplier on what it burns and sweats out.
///
/// "Increased physical activity should increase the rate at which hunger and
/// thirst increase." The argument is the energy an action cost, which the
/// action matrix already reckons: sleeping costs nothing, eating from a pack
/// costs one, walking and working cost five and up. A body asleep burns half
/// what an ordinary day burns; one working hard burns half again as much.
pub fn what_the_work_costs(energy_spent: f32) -> f32 {
    const AN_ORDINARY_TURN_COSTS: f32 = 5.0;
    const AT_REST: f32 = 0.5;
    const WORKING_HARD: f32 = 1.5;

    let share = (energy_spent / (AN_ORDINARY_TURN_COSTS * 2.0)).clamp(0.0, 1.0);
    AT_REST + (WORKING_HARD - AT_REST) * share
}

/// A meal in the stomach, emptying into the gut on its own clock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meal {
    /// The body's minute this was eaten on
    pub eaten_at: u32,
    /// What it was to start with
    pub initial: f32,
    /// What is left of it in the stomach
    pub remaining: f32,
    /// What a unit of it is worth
    pub richness: f32,
}

/// What has left the stomach and is not yet worth anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InTheGut {
    /// The body's minute this arrived on
    pub arrived_at: u32,
    pub units: f32,
    pub richness: f32,
}

/// A body's water, food and reserves, on a clock of its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Physiology {
    /// The body's own clock, in minutes lived
    pub minute: u32,

    /// One is watered, nought is dead of thirst
    pub hydration: f32,

    /// Drinks swallowed and not yet told, as (the minute it lands, how much)
    swallowed: Vec<(u32, f32)>,

    /// Meals still in the stomach
    pub stomach: Vec<Meal>,

    /// What has left the stomach and is not yet worth anything
    pub gut: Vec<InTheGut>,

    /// What the body has to go on, in food units
    pub reserve: f32,

    /// What this body's reserve holds when full - a child's is smaller
    pub reserve_capacity: f32,

    /// What this body's stomach holds - likewise
    pub stomach_capacity: f32,

    /// What is waiting to be passed
    pub waste: f32,

    /// What this body has burned since the day turned over
    #[serde(default)]
    pub burned_today: f32,

    /// What it burns in a day, by its own count rather than from a table
    ///
    /// "The agents should calculate their average food consumption." A big
    /// body working hard eats more than a small one resting, and it is its own
    /// consumption an agent has to lay in against. Seeded with an ordinary
    /// day's burn so a body that has not yet lived a day still has an answer.
    #[serde(default)]
    pub what_i_burn_in_a_day: f32,

    /// How much has ever gone down, and how many sittings it took
    #[serde(default)]
    pub units_ever_eaten: f32,
    #[serde(default)]
    pub meals_ever_eaten: u32,
}

impl Physiology {
    /// A grown body, watered and fed.
    pub fn new() -> Self {
        Self::for_a_body_of(1.0)
    }

    /// A body with this share of a grown one's reserves.
    ///
    /// `LifeStage::hunger_reserve` is the share. A child carries days where an
    /// adult carries weeks, so the same famine takes the young first without
    /// anybody having written that down.
    pub fn for_a_body_of(share: f32) -> Self {
        let share = share.clamp(0.05, 1.0);
        Self {
            minute: 0,
            hydration: 1.0,
            swallowed: Vec::new(),
            stomach: Vec::new(),
            gut: Vec::new(),
            reserve: RESERVE_OF_A_GROWN_BODY * share,
            reserve_capacity: RESERVE_OF_A_GROWN_BODY * share,
            stomach_capacity: STOMACH_CAPACITY * share,
            waste: 0.0,
            burned_today: 0.0,
            what_i_burn_in_a_day: UNITS_BURNED_IN_AN_ORDINARY_DAY * share,
            units_ever_eaten: 0.0,
            meals_ever_eaten: 0,
        }
    }

    /// Grow, or age, into a body of this size.
    pub fn now_a_body_of(&mut self, share: f32) {
        let share = share.clamp(0.05, 1.0);
        self.reserve_capacity = RESERVE_OF_A_GROWN_BODY * share;
        self.stomach_capacity = STOMACH_CAPACITY * share;
        self.reserve = self.reserve.min(self.reserve_capacity);
    }

    /// Whether there is room for another mouthful.
    ///
    /// Against this body's own stomach rather than a grown one's: a fifth of
    /// an infant's stomach is a smaller thing than a fifth of its father's.
    pub fn room_for_another_mouthful(&self) -> bool {
        // One mouthful, not a fifth of a stomach. "They can eat if they have
        // the room in their stomach and if they have a hunger drive" - a body
        // that is a little short and has a little room sits down to what there
        // is, rather than waiting for the stomach to clear.
        self.room_in_the_stomach() >= UNITS_IN_ONE_ITEM * self.how_fast_this_body_burns()
    }

    /// What this body burns against what a grown one burns.
    ///
    /// The share itself, because the share *is* the food a body this age
    /// needs - `what_a_body_this_age_eats` is a table of exactly that, year by
    /// year, and there is nothing left for a scaling law to add.
    ///
    /// It was the three-quarter power of the share, which is the right shape
    /// for real animals and was a guess standing in for a figure nobody had
    /// given. It made a child need more meals a day than its father while
    /// carrying a quarter of the stomach to take them in.
    pub fn how_fast_this_body_burns(&self) -> f32 {
        self.reserve_capacity / RESERVE_OF_A_GROWN_BODY
    }

    /// What is in the stomach now.
    pub fn in_the_stomach(&self) -> f32 {
        self.stomach.iter().map(|m| m.remaining).sum()
    }

    /// What is in the gut now.
    pub fn in_the_gut(&self) -> f32 {
        self.gut.iter().map(|c| c.units).sum()
    }

    /// What is in the stomach, in the energy it will be worth.
    ///
    /// The tables want to know whether there is a meal in there, and a meal is
    /// a quantity of energy rather than a volume: nineteen units of ordinary
    /// forage and eighty of spring leaf are the same supper.
    pub fn energy_in_the_stomach(&self) -> f32 {
        self.stomach.iter().map(|m| m.remaining * m.richness).sum()
    }

    /// And what is behind it in the gut, likewise.
    pub fn energy_in_the_gut(&self) -> f32 {
        self.gut.iter().map(|c| c.units * c.richness).sum()
    }

    /// How much more this body could get down right now.
    pub fn room_in_the_stomach(&self) -> f32 {
        (self.stomach_capacity - self.in_the_stomach()).max(0.0)
    }

    /// Take a drink. It tells twenty minutes from now.
    ///
    /// `share` is of a full body of water, so a good long drink at a spring is
    /// a fair fraction of one.
    pub fn drink(&mut self, share: f32) {
        if share <= 0.0 {
            return;
        }
        let lands_at = self.minute + MINUTES_FOR_A_DRINK_TO_TELL;
        self.swallowed.push((lands_at, share));
    }

    /// Sit down to food, and report how much of it actually went down.
    ///
    /// What will not fit stays where it was: a body with a full stomach cannot
    /// eat, however hungry the reserve says it is, which is the whole reason
    /// somebody who has gone without cannot put it right in one sitting.
    pub fn eat(&mut self, units_offered: f32, richness: f32) -> f32 {
        let taken = units_offered.min(self.room_in_the_stomach()).max(0.0);
        if taken <= 0.0 {
            return 0.0;
        }
        self.units_ever_eaten += taken;
        self.meals_ever_eaten += 1;
        self.stomach.push(Meal {
            eaten_at: self.minute,
            initial: taken,
            remaining: taken,
            richness,
        });
        taken
    }

    /// Live for this many minutes, having spent this much energy doing it.
    pub fn advance(&mut self, minutes: u32, energy_spent: f32) {
        if minutes == 0 {
            return;
        }
        let effort = what_the_work_costs(energy_spent);
        let was = self.minute;
        let now = self.minute + minutes;

        // A drink swallowed before this step tells during it
        let mut landed = 0.0;
        self.swallowed.retain(|(lands_at, share)| {
            if *lands_at <= now {
                landed += *share;
                false
            } else {
                true
            }
        });

        // Water goes out steadily, and faster for working
        let dries_out = minutes as f32 / MINUTES_TO_DIE_OF_THIRST as f32 * effort;
        self.hydration = (self.hydration - dries_out + landed).clamp(0.0, 1.0);

        // The stomach empties into the gut on each meal's own clock
        for meal in self.stomach.iter_mut() {
            let before = share_of_a_meal_gone_by(was.saturating_sub(meal.eaten_at));
            let after = share_of_a_meal_gone_by(now.saturating_sub(meal.eaten_at));
            let moved = (meal.initial * (after - before)).min(meal.remaining).max(0.0);
            if moved > 0.0 {
                meal.remaining -= moved;
                self.gut.push(InTheGut {
                    arrived_at: now,
                    units: moved,
                    richness: meal.richness,
                });
            }
        }
        self.stomach.retain(|m| m.remaining > 0.01);

        // What has been in the gut a day is worth something at last, and the
        // rest of it is waste
        let mut won = 0.0;
        self.gut.retain(|c| {
            if now.saturating_sub(c.arrived_at) >= MINUTES_TO_DIGEST {
                won += c.units * c.richness;
                false
            } else {
                true
            }
        });
        if won > 0.0 {
            self.waste += won * 0.25;
            self.reserve = (self.reserve + won).min(self.reserve_capacity);
        }

        // Keep a count of what a day actually costs this body, rolled over
        // when the day turns. A quarter weight on the newest day, so a hard
        // week moves it without one idle afternoon undoing the reckoning.
        let burned_now = minutes as f32 * effort * self.how_fast_this_body_burns();
        self.burned_today += burned_now;
        if now / MINUTES_PER_DAY > was / MINUTES_PER_DAY {
            if self.what_i_burn_in_a_day <= 0.0 {
                self.what_i_burn_in_a_day = self.burned_today;
            } else {
                self.what_i_burn_in_a_day =
                    self.what_i_burn_in_a_day * 0.75 + self.burned_today * 0.25;
            }
            self.burned_today = 0.0;
        }

        // And the body burns what it burns, which depends on how big it is.
        //
        // Not in proportion, though: a small body burns more for its size than
        // a large one, which is why a child eats a third of what its father
        // eats rather than a quarter, and why a famine takes the young first
        // even though they need less food in absolute terms. Three quarters is
        // the usual exponent for this.
        self.reserve = (self.reserve - burned_now).max(0.0);

        self.minute = now;
    }

    /// Leave what there is to leave, and report how much.
    pub fn pass_waste(&mut self) -> f32 {
        std::mem::take(&mut self.waste)
    }

    /// What share of itself this body can bring to anything, for want of water.
    ///
    /// "An agent can operate at 100% as long as their hydration level is
    /// greater than 75%. If it falls below 75%, their capabilities drop to
    /// 75%... below 50%, to 50%... below 25%, to 25%."
    pub fn capability(&self) -> f32 {
        if self.hydration > 0.75 {
            1.0
        } else if self.hydration > 0.50 {
            0.75
        } else if self.hydration > 0.25 {
            0.50
        } else {
            0.25
        }
    }

    /// Dead of thirst.
    pub fn died_of_thirst(&self) -> bool {
        self.hydration <= 0.0
    }

    /// Dead of hunger.
    pub fn starved(&self) -> bool {
        self.reserve <= 0.0
    }

    /// Going short of water, though not yet dying of it.
    pub fn is_parched(&self) -> bool {
        self.hydration <= FIRST_BAND
    }

    /// Living on the reserve rather than on anything eaten lately.
    ///
    /// Nothing in the stomach and nothing in the gut: about thirty hours after
    /// the last meal, since what left the stomach takes a day to be worth
    /// anything. This is the felt state and what everything else asks about.
    /// It is not yet doing harm - see `is_wasting`.
    pub fn is_starving(&self) -> bool {
        // And a week of the reserve gone with it.
        //
        // The gut clause alone is only about thirty hours since the last meal,
        // which is a long morning rather than starvation, and it is exactly
        // what happens between meals when one is missed. The reserve clause
        // was "any of it at all has been drawn on" - which every body that has
        // lived a day satisfies, so it excluded nothing but the newly made.
        // Bodies carrying sixteen and nineteen days of food read as starving
        // because their gut happened to be empty, and `immediate_needs_met`
        // took that as a reason not to have children. See ISSUES #77.
        //
        // Three days into a three-week reserve, with nothing coming in behind
        // it, is starving on anybody's reading - and the gut is only thirty
        // hours empty by the time a body is a day and a quarter in, so this
        // cannot fire on a missed meal.
        self.in_the_stomach() <= 0.0
            && self.in_the_gut() <= 0.0
            && self.days_into_the_reserve() >= DAYS_OF_RESERVE_BEFORE_IT_IS_STARVATION
    }

    /// How many days of its own reserve this body has already eaten through.
    ///
    /// Reckoned against what *this* body burns, so a child a week into its
    /// reserve and its father a week into his are both a week in, though the
    /// numbers of units differ.
    pub fn days_into_the_reserve(&self) -> f32 {
        let a_day = UNITS_BURNED_IN_AN_ORDINARY_DAY * self.how_fast_this_body_burns();
        (self.reserve_capacity - self.reserve) / a_day.max(1.0)
    }

    /// Far enough into the reserve that the body is taking it out of itself.
    ///
    /// Half of three weeks. Going a day without food is not this; going ten
    /// days is.
    pub fn is_wasting(&self) -> bool {
        self.reserve < self.reserve_capacity * 0.5
    }

    /// Put this body where it would be after this long without food.
    ///
    /// For tests and for setting a scene. The argument is minutes, which is
    /// the scale the old `ticks_without_food` figures were written on.
    pub fn gone_without_food_for(&mut self, minutes: u32) {
        self.stomach.clear();
        self.gut.clear();
        self.reserve = (self.reserve_capacity
            - minutes as f32 * self.how_fast_this_body_burns())
        .max(0.0);
        self.minute += minutes;
    }

    /// Likewise, without water.
    pub fn gone_without_water_for(&mut self, minutes: u32) {
        self.swallowed.clear();
        self.hydration =
            (1.0 - minutes as f32 / MINUTES_TO_DIE_OF_THIRST as f32).clamp(0.0, 1.0);
        self.minute += minutes;
    }

    /// How long this body has, if it never drinks again.
    pub fn minutes_before_thirst_kills_me(&self) -> f32 {
        self.hydration * MINUTES_TO_DIE_OF_THIRST as f32
    }

    /// How long this body has, if it never eats again.
    ///
    /// What is in the stomach and the gut still counts: it is coming, even if
    /// it is a day off being worth anything.
    pub fn minutes_before_hunger_kills_me(&self) -> f32 {
        let coming: f32 = self
            .stomach
            .iter()
            .map(|m| m.remaining * m.richness)
            .chain(self.gut.iter().map(|c| c.units * c.richness))
            .sum();
        self.reserve + coming
    }

    /// How much this body wants water, as a drive.
    ///
    /// At the Thirst drive's own threshold by the time a body wants a drink,
    /// and at its maximum by the time going without is costing it something.
    pub fn thirst(&self) -> f32 {
        const THE_DRIVE_ACTS_AT: f32 = 0.75;

        let short_by = (1.0 - self.hydration).max(0.0);
        let wants_one = 1.0 - WANTS_A_DRINK_AT;
        let going_short = 1.0 - FIRST_BAND;

        if short_by <= wants_one {
            // Up to wanting one
            short_by / wants_one * THE_DRIVE_ACTS_AT
        } else {
            // And from wanting one to going short, the rest of the way
            let past = (short_by - wants_one) / (going_short - wants_one);
            (THE_DRIVE_ACTS_AT + past * (1.0 - THE_DRIVE_ACTS_AT)).min(1.0)
        }
    }

    /// How much this body wants food, as a drive.
    ///
    /// "Hunger drive should be based on the agent's total caloric energy,
    /// stomach fullness level, and amount of food in the intestines." An empty
    /// stomach is what a body feels, and it feels it about five hours after
    /// eating, which is what puts three meals in a day. An empty gut is the
    /// day after that. An empty reserve is what actually kills, and it sets a
    /// floor under the other two rather than being averaged with them - a man
    /// three weeks hungry is not made comfortable by a mouthful.
    pub fn hunger(&self) -> f32 {
        self.how_fast_hunger_rises()
    }

    /// How fast the hunger drive rises, as the three tables have it.
    ///
    /// "Hunger drive should be based on the agent's total caloric energy,
    /// stomach fullness level, and amount of food in the intestines" - and
    /// each of the three is given as a step table rather than a curve, so
    /// each is a step table here.
    ///
    /// They multiply rather than average. The reserve table is the one that
    /// runs away - one at nine tenths full, four below a tenth - and the two
    /// gut tables gate it: a body with a full stomach and a day's food behind
    /// it scores nought whatever its reserve says, because it has just eaten
    /// and there is nothing to be done about the reserve for a day yet.
    ///
    /// "If an agent has nearly full internal/stored energy and enough food to
    /// replenish what it normally requires in a day, then there is little need
    /// to eat." Both halves of that: what stops the drive rising is a full
    /// stomach **and** a reserve that does not need topping up. A full stomach
    /// still stops it whatever the reserve says - nobody eats while full - but
    /// what counts as full moves with the reserve. A body that has plenty in
    /// hand waits until its stomach is nearly empty; a body that has eaten
    /// into its reserve is hungry again with the stomach still half full, and
    /// so eats sooner and oftener rather than harder.
    ///
    /// This is a **rate**, not a level - the tables are headed "Hunger Drive
    /// Increase". Read as a level it says a body with a day's food in its gut
    /// is never hungry at all, which stops it eating until the gut runs dry
    /// and then it is too late; measured that way every settlement died twice
    /// as fast.
    pub fn how_fast_hunger_rises(&self) -> f32 {
        let out_of = self.how_fast_this_body_burns().max(0.05);

        // What the body has to go on
        let share_of_reserve = (self.reserve / self.reserve_capacity.max(1.0)).clamp(0.0, 1.0);
        let by_reserve = if share_of_reserve > 0.90 {
            1.0
        } else if share_of_reserve > 0.80 {
            1.2
        } else if share_of_reserve > 0.70 {
            1.4
        } else if share_of_reserve > 0.60 {
            1.6
        } else if share_of_reserve > 0.50 {
            1.8
        } else if share_of_reserve > 0.40 {
            2.0
        } else if share_of_reserve > 0.30 {
            2.3
        } else if share_of_reserve > 0.20 {
            2.6
        } else if share_of_reserve > 0.10 {
            3.0
        } else {
            4.0
        };

        // What is in the stomach, as a share of a sitting down to eat.
        //
        // In energy, not in volume. The table was written when a meal was a
        // fixed four hundred and eighty *units* whatever it was made of, so
        // its rungs were unit counts; now that a unit is worth its own food's
        // energy, nineteen units of ordinary forage and eighty of spring leaf
        // are the same supper and only the energy can say so. Read as volume,
        // a body with a full supper of anything dense in it read as empty.
        let belly = self.energy_in_the_stomach() / (WHAT_A_SITTING_AIMS_AT * out_of);

        // How full is full enough to stop wanting more, for this body now.
        //
        // A body with its reserve intact wants nothing until its stomach is
        // nearly empty. One that has eaten into its reserve is hungry again
        // with the stomach still half full: "a low reserve should force an
        // agent to eat sooner instead of eating while full."
        //
        // This is the whole of what a low reserve does to the stomach table,
        // and it is deliberately not the other thing it could do. Letting the
        // reserve raise hunger *through* a full stomach was tried, and it is
        // wrong on its face: a body cannot answer a hunger it has no room for,
        // so all a drive rising against a full stomach does is spend turns on
        // meals that will not go down. Eating sooner is an answer. Eating
        // while full is not.
        let stops_wanting_at = A_FULL_BODY_STOPS_AT
            + (A_SPENT_BODY_STOPS_AT - A_FULL_BODY_STOPS_AT) * (1.0 - share_of_reserve);
        let by_belly = if belly >= stops_wanting_at {
            0.0
        } else {
            // And below that point it climbs the rest of the way to an empty
            // stomach, on the table's own shape.
            let how_far_down = 1.0 - (belly / stops_wanting_at.max(1e-3)).clamp(0.0, 1.0);
            1.0 + how_far_down
        };

        // And what is behind it in the gut, as a share of a day's food.
        //
        // The same argument: a day's food behind the stomach settles an
        // ordinary body, and a body that has been living off its reserve wants
        // more than a day's worth in hand before it stops looking for the next
        // meal.
        let gut = self.energy_in_the_gut() / (UNITS_BURNED_IN_AN_ORDINARY_DAY * out_of);
        let enough_behind_it = 1.0 + (1.0 - share_of_reserve);
        let by_gut = if gut >= enough_behind_it {
            0.0
        } else if gut >= enough_behind_it * 2.0 / 3.0 {
            1.0
        } else if gut >= enough_behind_it / 3.0 {
            1.5
        } else {
            2.0
        };

        // Full is full. What the reserve changes is *when* full arrives, which
        // both tables above have already taken from it.
        if by_belly == 0.0 || by_gut == 0.0 {
            return 0.0;
        }

        by_reserve * by_belly * by_gut
    }
}

impl Default for Physiology {
    fn default() -> Self {
        Self::new()
    }
}
