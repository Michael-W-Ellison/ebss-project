//! How fast things move across the country, on the clock the model keeps.
//!
//! A cell is ten metres a side (`Grid::METRES_PER_CELL`) and a turn is half an
//! hour. A person used to cross one cell in a turn: twenty metres an hour,
//! about two hundred and fifty times slower than anybody walks. Every errand
//! cost what it would cost to crawl, a kilometre burned two days' food, and a
//! people lived its whole life within a few hundred metres of camp. See
//! ISSUES_FOUND #268 and #272.
//!
//! "An agent should be capable of walking at a speed of 5 kph. An agent
//! should easily be capable of walking a kilometer in half an hour while
//! accomplishing other tasks such as checking traps. The rate of food burning
//! per distance should be similar to that of humans." The half-hour turn is
//! there to keep decisions from being taken every minute, not to hold anybody
//! to a walking pace nobody has.

/// How many cells a person covers in a whole half hour of walking.
///
/// Five kilometres an hour is two and a half in a turn, and a cell is ten
/// metres.
pub const CELLS_IN_A_HALF_HOUR_OF_WALKING: u32 = 250;

/// What walking burns, against an ordinary day's rate, while it lasts.
///
/// Walking at five kilometres an hour is about 3.5 METs; a day's average for
/// somebody who works about the place is about 1.4. That comes to twenty-nine
/// units a kilometre, 2% of a day's burning - near enough what a person spends
/// walking one. Times the load on the back
/// (`Simulation::what_this_load_costs`).
pub const WHAT_WALKING_BURNS: f32 = 2.45;

/// How much further an animal goes in a turn than it did while people moved
/// a cell a turn.
///
/// The paces in the fauna table were set against people crawling: a sheep at
/// pace one crossed two cells a turn, forty metres an hour. Twenty-five times
/// that is a sheep grazing its way across a kilometre an hour and a wolf
/// covering nearly two, with the same order among them as before. People and
/// game have to move on the same clock, or a man at five kilometres an hour
/// outruns everything alive fifty times over.
pub const HOW_MUCH_FURTHER_ANIMALS_GO: i32 = 25;

/// How much further is worth walking than was, now that walking is walking.
///
/// For the distances that were never about space but about how long a walk
/// would take: how far a person goes for water, round a trapline, out of
/// curiosity, or to a new camp. Ten times, which at the new pace is still
/// less of a half hour than the old distances were. What a hand reaches, what
/// a spear reaches and how close prey has to be to be worth a stalk are about
/// space and are not scaled - scaling those sent the hungry after anything
/// within half a kilometre, and they fought it (ISSUES_FOUND #273).
pub const HOW_MUCH_FURTHER_IS_WORTH_WALKING: u32 = 10;

/// How many turns walking this many cells takes, as a fraction.
pub fn turns_to_walk(cells: u32) -> f32 {
    cells as f32 / CELLS_IN_A_HALF_HOUR_OF_WALKING as f32
}

/// And in whole turns, rounded up, for the estimates that count in turns.
pub fn whole_turns_to_walk(cells: u32) -> u32 {
    cells.div_ceil(CELLS_IN_A_HALF_HOUR_OF_WALKING)
}
