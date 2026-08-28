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

/// What one sitting down to eat comes to, when there is enough to hand.
///
/// Three of these is an ordinary day's food exactly. Three *full* stomachs
/// would be eighteen hundred, which is a quarter more than a body needs, so an
/// agent with plenty in front of it still has no reason to eat until full.
pub const UNITS_IN_A_PORTION: f32 = UNITS_BURNED_IN_AN_ORDINARY_DAY / 3.0;

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
/// A unit of ordinary forage is worth one unit of energy; spring greens are
/// worth a quarter of that and fat two and a half times it, which is the whole
/// of "caloric density should be based on the type of food".
const ENERGY_OF_ORDINARY_FOOD: f32 = 25.0;

/// Below this share of its water a body starts to go short.
const FIRST_BAND: f32 = 0.75;

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

/// What a unit of this food is worth against a unit of ordinary food.
pub fn how_rich_this_food_is(energy: f32) -> f32 {
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
        self.room_in_the_stomach() > self.stomach_capacity * 0.2
    }

    /// What this body burns against what a grown one burns.
    ///
    /// A small body burns more for its size, so this is the three-quarter
    /// power of its share rather than the share itself. A quarter-sized body
    /// burns thirty-five per cent of an adult's, not twenty-five.
    pub fn how_fast_this_body_burns(&self) -> f32 {
        (self.reserve_capacity / RESERVE_OF_A_GROWN_BODY).powf(0.75)
    }

    /// What is in the stomach now.
    pub fn in_the_stomach(&self) -> f32 {
        self.stomach.iter().map(|m| m.remaining).sum()
    }

    /// What is in the gut now.
    pub fn in_the_gut(&self) -> f32 {
        self.gut.iter().map(|c| c.units).sum()
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

        // And the body burns what it burns, which depends on how big it is.
        //
        // Not in proportion, though: a small body burns more for its size than
        // a large one, which is why a child eats a third of what its father
        // eats rather than a quarter, and why a famine takes the young first
        // even though they need less food in absolute terms. Three quarters is
        // the usual exponent for this.
        self.reserve = (self.reserve - minutes as f32 * effort * self.how_fast_this_body_burns()).max(0.0);

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
        // And the reserve has actually been drawn on. Without that clause a
        // body that has never eaten anything - every agent at the moment it is
        // made - reads as starving, because an empty stomach and an empty gut
        // is exactly how everyone starts.
        self.in_the_stomach() <= 0.0
            && self.in_the_gut() <= 0.0
            && self.reserve < self.reserve_capacity * 0.95
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
    /// Pressing by the time the first band is reached, so an agent goes to the
    /// water before it starts going short rather than after.
    pub fn thirst(&self) -> f32 {
        ((1.0 - self.hydration) / (1.0 - FIRST_BAND)).clamp(0.0, 1.0)
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
        let belly = 1.0 - (self.in_the_stomach() / self.stomach_capacity).clamp(0.0, 1.0);
        let gut = 1.0 - (self.in_the_gut() / UNITS_IN_A_PORTION).clamp(0.0, 1.0);
        let spent = 1.0 - (self.reserve / self.reserve_capacity).clamp(0.0, 1.0);

        // An empty stomach on its own has to be enough to cross the hunger
        // drive's threshold. Weighted evenly with the gut it was not: a body
        // eating once a day keeps five hundred units in the gut at all times,
        // which pinned that term at nothing and capped hunger at six tenths
        // against a threshold of seven. Agents ate once a day, burned three
        // times what they took in, and starved with food five paces off.
        (0.75 * belly + 0.15 * gut + 0.10 * spent).max(spent)
    }
}

impl Default for Physiology {
    fn default() -> Self {
        Self::new()
    }
}
