// src/agents/hire.rs
//! Work done for somebody else, paid for in goods.
//!
//! "There should be trading mechanism which enables the trading of work for
//! other goods. Agents will need to calculate if giving something in trade
//! will result in a larger return. If meat would spoil, but giving some away
//! would result in more meat being stored as food, then this should be capable
//! of occurring." (#374, after #331.)
//!
//! Barter (`between_us::exchange`) swaps one thing in hand for another. This
//! is the other half of trading: one person's hands, for a share of what they
//! move. Both sides reckon it before it is agreed:
//!
//! - **whoever offers** asks whether what comes home to them *after* paying
//!   is more than what would come home without help
//!   (`what_comes_home_to_me`). A beast one man can carry home in time is
//!   never shared out: paying a carrier would only cost him;
//! - **whoever is asked** asks whether their pay is worth more than the
//!   walking and cutting cost them, and is not hungry
//!   (`Simulation::would_this_one_carry_for_a_share`).
//!
//! Trade needs something to be somebody's to trade, so a kill is its
//! killer's: anybody else may cut from it only if they are kin, were told
//! where it is (which is giving it away), or are carrying it for a share
//! (`Simulation::may_i_cut_from`). Nobody tells of a kill being carried.
//!
//! The one kind of work so far is carrying a kill home: from the carcass to a
//! store the owner may use as their own, with the carrier keeping a share of
//! every load. The agreement lives on the worker (`Agent::working_for`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The work somebody has agreed to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Work {
    /// Cut joints off the carcass at `from` and put them in the store at `to`.
    CarryHome { from: (i32, i32), to: (i32, i32) },
}

/// Somebody's hands, taken on by somebody else for a share of what they move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hire {
    /// Whose work it is.
    pub by: Uuid,
    pub work: Work,
    /// Of everything taken under this agreement, the part that is the
    /// worker's to keep.
    pub share: f32,
    /// What the worker reckoned on carrying when they agreed, which is what
    /// the next carrier asked is weighed against.
    pub reckoned: u32,
    /// How much has been taken in hand under it so far.
    pub taken: u32,
    /// And how much of that has been put where it was to go.
    pub handed_over: u32,
    /// The tick after which it is off, whatever is left.
    pub until: u32,
}

impl Hire {
    /// The customary pay for carrying a kill home: one joint in three.
    ///
    /// High enough to be worth a fed man's afternoon at a carcass a short
    /// walk off, and low enough that the man who killed it still gets most of
    /// what he would not have got at all.
    pub const A_CARRIERS_SHARE: f32 = 1.0 / 3.0;

    /// Loads a pair of hands brings home off a carcass in a day.
    ///
    /// One. The walk is nothing - a carcass a day's walk off is twenty
    /// minutes at this map's pace - but the cutting, the eating, the drinking
    /// and the sleeping are not, and everybody has more to do in a day than
    /// carry somebody's deer. A load is what there is room for in the pack
    /// (`a_load`).
    pub const LOADS_HOME_A_DAY: u32 = 1;

    /// What of what has been taken is still owed to the store: everything
    /// but the worker's share, less what is already in it.
    pub fn still_owed(&self) -> u32 {
        let owed = (self.taken as f32 * (1.0 - self.share)).round() as u32;
        owed.saturating_sub(self.handed_over)
    }

    /// Where the work is to be done, and where what it moves goes.
    pub fn from_and_to(&self) -> ((i32, i32), (i32, i32)) {
        match self.work {
            Work::CarryHome { from, to } => (from, to),
        }
    }
}

/// How many joints a pair of hands carries in one load.
pub fn a_load(how_much_a_pack_holds: f32, what_one_weighs: f32) -> u32 {
    if what_one_weighs <= 0.0 {
        return 0;
    }
    (how_much_a_pack_holds / what_one_weighs).floor().max(0.0) as u32
}

/// What one pair of hands can bring home off a carcass before it goes:
/// a load a day for each whole day left once they have got there.
///
/// `ticks_left` is how long the carcass has, and `ticks_to_get_there` how long
/// the walk to it is; a day not yet begun is a day's load, and the day it is
/// is one too while there is half of it left.
pub fn what_one_pair_of_hands_brings_home(ticks_left: u32, ticks_to_get_there: u32, a_load: u32) -> u32 {
    let day = crate::environment::seasons::TICKS_PER_DAY;
    let working = ticks_left.saturating_sub(ticks_to_get_there);
    let days = working / day + u32::from(working % day >= day / 2);
    days * Hire::LOADS_HOME_A_DAY * a_load
}

/// What comes home to the owner of `on_it`, carrying `mine` himself, with
/// carriers bringing `theirs` each and keeping `share` of what they bring.
///
/// Everybody works the carcass side by side until it is gone or rotten, so
/// what comes home is what is on it or what all the hands can carry,
/// whichever is less, and each carries their part of that.
pub fn what_comes_home_to_me(on_it: u32, mine: u32, theirs: &[u32], share: f32) -> f32 {
    let carriers: u32 = theirs.iter().sum();
    let all = mine + carriers;
    if all == 0 {
        return 0.0;
    }
    let home = on_it.min(all) as f32;
    let theirs_of_it = home * carriers as f32 / all as f32;
    home - theirs_of_it * share
}

/// Whether one more carrier, who would bring `his`, leaves more coming home
/// to the owner after their pay than he has without them, and what they would
/// be paid if so.
///
/// `None` when it does not pay: the carcass is no bigger than the hands
/// already on it, so a carrier only takes a share of what would have come
/// home anyway.
pub fn is_another_pair_of_hands_worth_it(
    on_it: u32,
    mine: u32,
    theirs: &[u32],
    his: u32,
    share: f32,
) -> Option<f32> {
    if his == 0 {
        return None;
    }
    let without = what_comes_home_to_me(on_it, mine, theirs, share);
    let mut with: Vec<u32> = theirs.to_vec();
    with.push(his);
    let with_him = what_comes_home_to_me(on_it, mine, &with, share);
    if with_him <= without {
        return None;
    }
    let all = mine + with.iter().sum::<u32>();
    let home = on_it.min(all) as f32;
    Some(home * his as f32 / all as f32 * share)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::seasons::TICKS_PER_DAY;

    #[test]
    fn a_beast_one_man_can_carry_home_is_never_shared_out() {
        // A deer of ten joints, a man who carries six a day, four days left
        let mine = what_one_pair_of_hands_brings_home(4 * TICKS_PER_DAY, 0, 6);
        assert_eq!(mine, 24);
        assert_eq!(is_another_pair_of_hands_worth_it(10, mine, &[], 6, Hire::A_CARRIERS_SHARE), None);
    }

    #[test]
    fn a_beast_too_big_to_carry_in_time_is_worth_paying_to_have_carried() {
        // A bear of thirty joints with a day and a half left: he brings home
        // twelve alone, and with a carrier the two of them bring twenty-four,
        // of which he pays four
        let mine = what_one_pair_of_hands_brings_home(TICKS_PER_DAY * 3 / 2, 0, 6);
        assert_eq!(mine, 12);
        let pay = is_another_pair_of_hands_worth_it(30, mine, &[], 12, Hire::A_CARRIERS_SHARE)
            .expect("a carrier brings home more than he costs");
        assert!((pay - 4.0).abs() < 1e-4);
        let alone = what_comes_home_to_me(30, mine, &[], Hire::A_CARRIERS_SHARE);
        let helped = what_comes_home_to_me(30, mine, &[12], Hire::A_CARRIERS_SHARE);
        assert!((alone - 12.0).abs() < 1e-4);
        assert!((helped - 20.0).abs() < 1e-4);
    }

    #[test]
    fn carriers_are_taken_on_only_while_another_still_pays() {
        // Thirty joints, twelve his own: the first carrier of twelve pays,
        // the second brings the hands past what is on it and still pays a
        // little, and a third only takes a share of what was coming home
        let mine = 12;
        assert!(is_another_pair_of_hands_worth_it(30, mine, &[], 12, Hire::A_CARRIERS_SHARE).is_some());
        assert!(is_another_pair_of_hands_worth_it(30, mine, &[12], 12, Hire::A_CARRIERS_SHARE).is_some());
        assert_eq!(is_another_pair_of_hands_worth_it(30, mine, &[12, 12], 12, Hire::A_CARRIERS_SHARE), None);
    }

    #[test]
    fn the_walk_there_comes_off_the_days_left() {
        let day = TICKS_PER_DAY;
        assert_eq!(what_one_pair_of_hands_brings_home(2 * day, day, 6), 6);
        assert_eq!(what_one_pair_of_hands_brings_home(day, 2 * day, 6), 0);
        // Under half a day is not a load
        assert_eq!(what_one_pair_of_hands_brings_home(day / 3, 0, 6), 0);
    }

    #[test]
    fn what_is_owed_is_everything_but_the_share_less_what_is_in() {
        let mut hire = Hire {
            by: Uuid::nil(),
            work: Work::CarryHome { from: (0, 0), to: (5, 5) },
            share: Hire::A_CARRIERS_SHARE,
            reckoned: 12,
            taken: 6,
            handed_over: 0,
            until: 0,
        };
        assert_eq!(hire.still_owed(), 4);
        hire.handed_over = 4;
        assert_eq!(hire.still_owed(), 0);
        hire.taken = 9;
        assert_eq!(hire.still_owed(), 2);
    }
}
