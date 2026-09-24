// src/world/sleeping.rs
//! Ground nobody is near sleeps.
//!
//! Every resource node on the map regrew every day wherever it was: about a
//! quarter of a microsecond a node, which on a 1,600-cell map was 85% of what
//! a day cost with nobody in it and rises with the area - see ISSUES_FOUND
//! #248. A node nobody can see or reach does not need to be up to date. It
//! needs to be up to date when somebody gets near.
//!
//! So a node further from every living person than anybody ever looks goes to
//! sleep, and when somebody comes near it wakes and catches up. The catch-up is
//! exact rather than a guess. Everything a wild node's day reads is either
//! fixed for the node - its terrain, its ground's grade - or one value for the
//! whole map that day: the rain, the season, what each kind of country's air
//! and water are doing. So the world keeps a log of those, one small
//! [`AGrowingDay`] a day, and a waking node lives its missed days over from it
//! one at a time, through the same function the live pass uses. It comes out
//! exactly where it would have been had it never slept.
//!
//! Field crops never sleep: they ripen, are brought in and wear their field,
//! and there are only as many of them as a settlement farms.

use serde::{Deserialize, Serialize};

use super::TerrainType;
use crate::environment::seasons::Season;

/// How far from every living person a node has to be before it sleeps, in
/// cells either way.
///
/// The furthest anything anybody decides reads a node from is sixty cells -
/// `HOW_FAR_A_PEOPLE_WILL_MOVE`, which is where a settlement looks for a new
/// camp - and a person walks at most a cell a turn, forty-eight a day. A node
/// is only brought up to date once a day, so what has to hold is that nobody
/// can come within sixty of a node that slept through this morning's pass
/// before the next one wakes it: sixty and forty-eight, and some over.
pub const FAR_ENOUGH_TO_SLEEP: i32 = 128;

/// The side of the patches of map that are woken or left asleep together.
pub const A_PATCH: i32 = 16;

/// Everything a wild node's day reads that is one value for the whole map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AGrowingDay {
    pub today: u32,
    pub this_year: u32,
    pub season: Season,
    pub season_modifier: f32,
    pub precipitation: f32,
    /// The air's temperature and whether the water is ice, for each kind of
    /// country, indexed by `TerrainType as usize`.
    pub by_terrain: Vec<(f32, bool)>,
}

impl AGrowingDay {
    /// The air and the water over this kind of country today.
    pub fn over(&self, terrain: TerrainType) -> (f32, bool) {
        self.by_terrain[terrain as usize]
    }
}

/// Which patches of the map have somebody within `FAR_ENOUGH_TO_SLEEP` of
/// them.
#[derive(Debug, Clone)]
pub struct WhoIsAwake {
    across: usize,
    down: usize,
    awake: Vec<bool>,
}

impl WhoIsAwake {
    /// Mark every patch near anybody standing at `people`, on a map this size.
    pub fn round(people: &[(i32, i32)], width: usize, height: usize) -> Self {
        let across = width.div_ceil(A_PATCH as usize).max(1);
        let down = height.div_ceil(A_PATCH as usize).max(1);
        let mut awake = vec![false; across * down];

        for &(x, y) in people {
            let from_x = ((x - FAR_ENOUGH_TO_SLEEP).max(0) / A_PATCH) as usize;
            let to_x = (((x + FAR_ENOUGH_TO_SLEEP).max(0) / A_PATCH) as usize).min(across - 1);
            let from_y = ((y - FAR_ENOUGH_TO_SLEEP).max(0) / A_PATCH) as usize;
            let to_y = (((y + FAR_ENOUGH_TO_SLEEP).max(0) / A_PATCH) as usize).min(down - 1);
            if from_x > to_x || from_y > to_y {
                continue;
            }
            for py in from_y..=to_y {
                for px in from_x..=to_x {
                    awake[py * across + px] = true;
                }
            }
        }

        Self { across, down, awake }
    }

    /// Whether the ground at `(x, y)` has anybody near it.
    pub fn near_anybody(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return true;
        }
        let px = (x / A_PATCH) as usize;
        let py = (y / A_PATCH) as usize;
        if px >= self.across || py >= self.down {
            return true;
        }
        self.awake[py * self.across + px]
    }
}
