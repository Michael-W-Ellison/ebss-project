// src/world/soil.rs
//! What is under a plant's feet.
//!
//! Soil is two things, and they are kept apart because they cost apart.
//!
//! **What kind of ground it is, and how good.** A tile's ground has a *type*
//! - loam, silt, peat, sand, stone, salt - and a *grade* on a ladder from
//! exhausted to very rich. Wild ground is whatever it was made: its type and
//! grade follow from its terrain and never change, so they are worked out
//! when asked rather than stored, and a map costs nothing for them however
//! large it is. Only people move a grade, and only on ground they have broken:
//! a crop taken off a field wears it down, a bean crop, a season-old muck
//! spreading or a year's rest builds it back. What a field has been through is
//! a [`Field`], and the grid keeps one for each field and nothing for anywhere
//! else.
//!
//! This replaced a pool of nutrient and two pools of litter on every tile,
//! rotted every day over the whole map by the weather. Every leaf that fell
//! and every beast that dunged moved the ground somewhere, so after a year
//! three tiles in five had drifted from where they began and none of it could
//! be left unstored. See ISSUES_FOUND #246.
//!
//! **What somebody has left on it.** Muck, the seed in it, and on a field the
//! weeds and vermin. These start at nought everywhere and arrive only where
//! somebody does something, which is what the [`Soil`] on a tile still holds.

use serde::{Deserialize, Serialize};

use super::TerrainType;

/// How good a piece of ground is: one rung of a ladder.
///
/// Each rung is a multiplier on what anything growing there yields, wild or
/// sown. The rungs are listed once, in [`SoilGrade::LADDER`], and everything
/// that climbs or descends goes through it - so a rung is added by adding it
/// to the enum and to the ladder, and nothing else counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SoilGrade {
    Exhausted,
    Depleted,
    Ordinary,
    Rich,
    VeryRich,
}

impl SoilGrade {
    /// Every rung, poorest first. The last is the top.
    pub const LADDER: [SoilGrade; 5] = [
        SoilGrade::Exhausted,
        SoilGrade::Depleted,
        SoilGrade::Ordinary,
        SoilGrade::Rich,
        SoilGrade::VeryRich,
    ];

    /// What this ground makes of anything growing in it.
    pub fn multiplier(self) -> f32 {
        match self {
            SoilGrade::Exhausted => 0.25,
            SoilGrade::Depleted => 0.5,
            SoilGrade::Ordinary => 1.0,
            SoilGrade::Rich => 1.5,
            SoilGrade::VeryRich => 2.0,
        }
    }

    /// The best ground there is.
    pub fn the_top() -> Self {
        Self::LADDER[Self::LADDER.len() - 1]
    }

    fn rung(self) -> usize {
        Self::LADDER
            .iter()
            .position(|rung| *rung == self)
            .expect("every grade is on the ladder")
    }

    /// One rung up, or where it is if it is at the top.
    pub fn richer(self) -> Self {
        Self::LADDER[(self.rung() + 1).min(Self::LADDER.len() - 1)]
    }

    /// One rung down, or where it is if it is at the bottom.
    pub fn poorer(self) -> Self {
        Self::LADDER[self.rung().saturating_sub(1)]
    }

    /// One rung nearer `aim`, from either side, and never past it.
    pub fn towards(self, aim: SoilGrade) -> Self {
        match self.cmp(&aim) {
            std::cmp::Ordering::Less => self.richer(),
            std::cmp::Ordering::Greater => self.poorer(),
            std::cmp::Ordering::Equal => self,
        }
    }

    /// The rung whose multiplier is nearest this one.
    ///
    /// For somebody working out what a field is from what it grew: nobody is
    /// told the grade of anything.
    pub fn nearest_to(multiplier: f32) -> Self {
        *Self::LADDER
            .iter()
            .min_by(|a, b| {
                (a.multiplier() - multiplier)
                    .abs()
                    .partial_cmp(&(b.multiplier() - multiplier).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("the ladder has rungs")
    }

    /// What people would call it.
    pub fn called(self) -> &'static str {
        match self {
            SoilGrade::Exhausted => "exhausted",
            SoilGrade::Depleted => "depleted",
            SoilGrade::Ordinary => "ordinary",
            SoilGrade::Rich => "rich",
            SoilGrade::VeryRich => "very rich",
        }
    }
}

/// What kind of ground it is.
///
/// Kept for ever, whatever happens to the grade: a loam worn down to nothing
/// is still a loam, and rested it comes back to being the loam it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SoilType {
    Loam,
    Silt,
    Peat,
    Sand,
    Stone,
    Salt,
}

impl SoilType {
    /// What people would call it.
    pub fn called(self) -> &'static str {
        match self {
            SoilType::Loam => "loam",
            SoilType::Silt => "silt",
            SoilType::Peat => "peat",
            SoilType::Sand => "sand",
            SoilType::Stone => "stone",
            SoilType::Salt => "salt",
        }
    }

    /// Whether anything grows in it at all.
    ///
    /// Not on a salt flat, which is the point of one.
    pub fn grows_anything(self) -> bool {
        !matches!(self, SoilType::Salt)
    }

    /// What ground of this terrain is, before anybody has done anything to it.
    ///
    /// The grades are read off what the old nutrient model settled at on wild
    /// ground in its second year, when the litter every tile was born with had
    /// rotted in: a riverbank or a marsh at 0.94 of what ground could hold, a
    /// wood at 0.83, a meadow at 0.71, open plains at 0.51, hills at a third,
    /// sand and rock at a tenth or so. Ordinary ground was half, so each
    /// terrain takes the rung whose multiplier is nearest twice its figure,
    /// and wild food comes up about where it came up before. Measured over two
    /// seeded years on a hundred-cell map; see ISSUES_FOUND #246.
    pub fn natural_to(terrain: TerrainType) -> (SoilType, SoilGrade) {
        match terrain {
            TerrainType::Wetland => (SoilType::Peat, SoilGrade::VeryRich),
            TerrainType::Riverbank => (SoilType::Silt, SoilGrade::VeryRich),
            TerrainType::SaltMarsh => (SoilType::Silt, SoilGrade::VeryRich),
            TerrainType::Forest => (SoilType::Loam, SoilGrade::Rich),
            TerrainType::Meadow => (SoilType::Loam, SoilGrade::Rich),
            TerrainType::Plains => (SoilType::Loam, SoilGrade::Ordinary),
            // Ground that was broken before anybody kept a record of it.
            // Nothing is generated as farmland; this is for a field a test or
            // an old save put down without a `Field` to go with it.
            TerrainType::Farmland => (SoilType::Loam, SoilGrade::Ordinary),
            TerrainType::Water | TerrainType::Sea => (SoilType::Silt, SoilGrade::Ordinary),
            TerrainType::Hills => (SoilType::Stone, SoilGrade::Depleted),
            TerrainType::Beach => (SoilType::Sand, SoilGrade::Exhausted),
            TerrainType::Desert => (SoilType::Sand, SoilGrade::Exhausted),
            TerrainType::Mountain => (SoilType::Stone, SoilGrade::Exhausted),
            TerrainType::SaltFlat => (SoilType::Salt, SoilGrade::Exhausted),
        }
    }
}

/// How many times what the same plant yields wild, a plant yields on broken
/// ground.
///
/// "Wild plants produce yields 1/4th that of plants in tilled farmland." One
/// number for every crop: the difference between crops is in how fast each
/// kind grows and what it is worth to eat, not in what the plough does for it.
pub const WHAT_BROKEN_GROUND_YIELDS_OVER_WILD: f32 = 4.0;

/// A season and a year on the world clock, which counts ticks.
pub const A_SEASON: u32 =
    crate::environment::seasons::TICKS_PER_DAY * crate::environment::seasons::DAYS_PER_SEASON;
pub const A_YEAR: u32 = crate::environment::seasons::TICKS_PER_YEAR;

/// Ground somebody has broken, and what has happened to it since.
///
/// The only soil state the world stores. Wild ground is its terrain's natural
/// soil and has no `Field`; a tile gets one when it is broken and loses it
/// when it has gone back to the wild.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// What kind of ground it is, which it keeps whatever is done to it.
    pub soil: SoilType,
    /// What it was before anybody broke it, which a rest brings it back to.
    pub natural: SoilGrade,
    /// What it is now.
    pub grade: SoilGrade,
    /// The terrain it was, which it goes back to when it is given up.
    pub was: TerrainType,

    /// Since when nothing has been taken off it, or since a year's rest last
    /// did its work.
    pub rested_since: u32,

    /// When anybody last did anything with it: broke it, cropped it, weeded
    /// it, mucked it.
    pub worked_at: u32,

    /// Since when it has been back at its natural grade, if it is.
    pub natural_since: Option<u32>,

    /// Muck in the ground that has not yet come to a rung.
    pub muck_in_it: f32,

    /// When the muck that has come to a rung will have rotted in.
    pub mucked_ready_at: Option<u32>,
}

/// What became of a field on its day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhatBecameOfIt {
    StillAField,
    GoneBackToTheWild,
}

impl Field {
    /// How much muck it takes to bring ground up a rung.
    ///
    /// Ten units of spoiled food, or somewhat over one spoiled fish - see
    /// `MUCK_PER_UNIT` and `MUCK_PER_FISH`. A handful of rotten berries tipped
    /// on a field does not make it rich; a season's spoil from a household
    /// does. Small tippings add up.
    pub const ENOUGH_MUCK_FOR_A_RUNG: f32 = 1.2;

    /// Ground broken out of `terrain`, today.
    pub fn broken_out_of(terrain: TerrainType, now: u32) -> Self {
        let was = if terrain == TerrainType::Farmland {
            TerrainType::Plains
        } else {
            terrain
        };
        let (soil, natural) = SoilType::natural_to(terrain);
        Self {
            soil,
            natural,
            grade: natural,
            was,
            rested_since: now,
            worked_at: now,
            natural_since: Some(now),
            muck_in_it: 0.0,
            mucked_ready_at: None,
        }
    }

    fn now_at(&mut self, grade: SoilGrade, now: u32) {
        if grade != self.grade {
            self.grade = grade;
            self.natural_since = (grade == self.natural).then_some(now);
        }
    }

    /// Somebody worked it: weeded it, broke it again, walked a crop in.
    pub fn somebody_worked_it(&mut self, now: u32) {
        self.worked_at = now;
    }

    /// Something came off the crop standing on it today.
    ///
    /// Cropped is not rested, and somebody is working it.
    pub fn a_crop_came_off(&mut self, now: u32) {
        self.worked_at = now;
        self.rested_since = now;
    }

    /// The harvest is in: three quarters of a ripe crop has come off it.
    ///
    /// "Harvests from tilled farmland are much more abundant but their harvest
    /// reduces the future production capability of the farmland." A rung for
    /// each harvest - except a pod crop's, which takes nothing from the ground
    /// it grows in.
    pub fn the_harvest_is_in(&mut self, pods: bool, now: u32) {
        self.a_crop_came_off(now);
        if pods {
            return;
        }
        let poorer = self.grade.poorer();
        self.now_at(poorer, now);
    }

    /// A bean crop on it has come to maturity, which is when it ripens - once
    /// a crop, since a ripe crop does not grow.
    pub fn a_bean_crop_came_in(&mut self, now: u32) {
        let richer = self.grade.richer();
        self.now_at(richer, now);
    }

    /// Somebody put muck on it, worth `worth`.
    ///
    /// Returns whether it will come to anything: a field already at the top
    /// has nowhere to go, and one with a rung's worth already rotting in has
    /// to wait for that before the next can start.
    pub fn somebody_mucked_it(&mut self, worth: f32, now: u32) -> bool {
        self.worked_at = now;
        if self.grade == SoilGrade::the_top() {
            return false;
        }
        self.muck_in_it += worth.max(0.0);
        if self.mucked_ready_at.is_none() && self.muck_in_it >= Self::ENOUGH_MUCK_FOR_A_RUNG {
            self.muck_in_it -= Self::ENOUGH_MUCK_FOR_A_RUNG;
            self.mucked_ready_at = Some(now + A_SEASON);
        }
        true
    }

    /// A day goes by.
    ///
    /// Muck that has had its season comes to a rung. A year with nothing
    /// taken off brings the ground one rung back towards what it was, from
    /// either side, and never past it. And ground that has sat at its natural
    /// grade for a year with nobody doing anything with it is not a field any
    /// more.
    pub fn a_day_goes_by(&mut self, now: u32) -> WhatBecameOfIt {
        if let Some(ready) = self.mucked_ready_at {
            if now >= ready {
                self.mucked_ready_at = None;
                let richer = self.grade.richer();
                self.now_at(richer, now);
                if self.muck_in_it >= Self::ENOUGH_MUCK_FOR_A_RUNG {
                    self.muck_in_it -= Self::ENOUGH_MUCK_FOR_A_RUNG;
                    self.mucked_ready_at = Some(now + A_SEASON);
                }
            }
        }

        if now.saturating_sub(self.rested_since) >= A_YEAR {
            self.rested_since = now;
            let back = self.grade.towards(self.natural);
            self.now_at(back, now);
        }

        let a_year_at_its_natural = self
            .natural_since
            .is_some_and(|since| now.saturating_sub(since) >= A_YEAR);
        let a_year_untouched = now.saturating_sub(self.worked_at) >= A_YEAR;

        if a_year_at_its_natural && a_year_untouched && self.mucked_ready_at.is_none() {
            WhatBecameOfIt::GoneBackToTheWild
        } else {
            WhatBecameOfIt::StillAField
        }
    }

    /// What people would call it: "very rich loam".
    pub fn called(&self) -> String {
        format!("{} {}", self.grade.called(), self.soil.called())
    }
}

/// What somebody has left on a tile.
///
/// Muck, the seed in it, and on a field the weeds and vermin. All of it starts
/// at nought everywhere and only arrives where somebody does something.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Soil {
    /// Fresh dung lying on the surface: what a midden smells of.
    ///
    /// Its own question rather than part of the ground's grade. This is about
    /// what the ground smells of today, and it goes off much faster than
    /// anything that would change a field.
    #[serde(default)]
    pub fouling: f32,

    /// Seeds passed through somebody, waiting on the ground they were dropped
    /// on.
    ///
    /// "Seeds from the plants they have eaten should sprout." They come out
    /// with the waste, sit until the fouling has broken down enough to be soil
    /// rather than muck, and then come up.
    #[serde(default)]
    pub seeds_dropped: f32,

    /// What is growing in a field that nobody wanted there.
    ///
    /// "Farmers should not just drop seeds and get crops. They need to
    /// maintain the fields, clearing weeds and removing pests." Broken ground
    /// is the best ground there is and everything else knows it.
    #[serde(default)]
    pub weeds: f32,

    /// And what is eating it.
    #[serde(default)]
    pub pests: f32,
}

impl Soil {
    /// What one unit of food, eaten, leaves for the ground.
    ///
    /// These size a midden and nothing else now: what a camp leaves on a tile,
    /// how foul it gets and how much seed is in it. They are the figures they
    /// were when they also fed a nutrient pool, so a midden smells exactly as
    /// it did.
    pub const WASTE_PER_MEAL: f32 = 0.0015;

    /// And one that spoiled before anybody could eat it: nothing took a share
    /// of this on the way.
    pub const WASTE_PER_SPOILED: f32 = Self::WASTE_PER_MEAL / 0.6;

    /// What a fish leaves, forty times what a unit of crop does: it was grown
    /// at sea on a whole catchment and walked into reach on its own.
    pub const WHAT_A_FISH_LEAVES: f32 = Self::WASTE_PER_SPOILED * 40.0;

    /// What is left of a fish, eaten. A third went as guts at the waterside,
    /// and a body keeps a quarter of what it eats.
    pub const WASTE_PER_FISH_EATEN: f32 = Self::WHAT_A_FISH_LEAVES * 0.4;

    /// And of one that turned before anybody got to it.
    pub const WASTE_PER_FISH_SPOILED: f32 = Self::WHAT_A_FISH_LEAVES * 0.65;

    /// Whether a thing in somebody's pack came out of the water.
    ///
    /// Matched on the name, because that is all an untracked stack carries.
    pub fn came_out_of_the_water(item_id: &str) -> bool {
        let name = item_id.to_lowercase();
        name.contains("fish") || name.contains("salmon") || name.contains("trout")
    }

    /// What one unit of this, eaten, leaves.
    pub fn waste_from_eating(item_id: &str) -> f32 {
        if Self::came_out_of_the_water(item_id) {
            Self::WASTE_PER_FISH_EATEN
        } else {
            Self::WASTE_PER_MEAL
        }
    }

    /// What one unit of this, spoiled and never eaten, leaves.
    pub fn waste_from_spoilage(item_id: &str) -> f32 {
        if Self::came_out_of_the_water(item_id) {
            Self::WASTE_PER_FISH_SPOILED
        } else {
            Self::WASTE_PER_SPOILED
        }
    }

    /// How wet this ground is, from the country it is in and the weather over
    /// it.
    ///
    /// What decides how fast a midden airs out, and what a plant has to drink.
    pub fn humidity(terrain: TerrainType, precipitation: f32) -> f32 {
        let ground = match terrain {
            TerrainType::Water
            | TerrainType::Wetland
            | TerrainType::Sea
            | TerrainType::SaltMarsh => 1.0,
            TerrainType::Riverbank => 0.85,
            TerrainType::Forest => 0.7,
            TerrainType::Meadow => 0.55,
            TerrainType::Farmland => 0.5,
            TerrainType::Plains => 0.45,
            TerrainType::Beach => 0.4,
            TerrainType::Hills => 0.35,
            TerrainType::Mountain => 0.25,
            TerrainType::Desert => 0.05,
            TerrainType::SaltFlat => 0.03,
        };

        (ground + precipitation.clamp(0.0, 1.0) * 0.3).clamp(0.0, 1.0)
    }

    /// What somebody has just passed, with whatever was in it.
    ///
    /// A smell, and seeds. Not a change to the ground's grade: people void
    /// where they happen to be, and only muck carried to a field on purpose
    /// builds one - see `Field::somebody_mucked_it`.
    ///
    /// This is soil on its own, and knows nothing about the map it sits in.
    /// If you have a grid in your hand, call `Grid::somebody_voided_on`
    /// instead: fouling and seed are the two things the ground register is
    /// keeping track of, and a tile fouled behind its back never gets visited.
    pub fn somebody_voided_here(&mut self, amount: f32) {
        self.fouling = (self.fouling + amount).clamp(0.0, Self::AS_FOUL_AS_IT_GETS);
        self.seeds_dropped =
            (self.seeds_dropped + amount * Self::WHAT_COMES_THROUGH_WHOLE).clamp(0.0, 1.0);
    }

    /// How foul ground can get before more of it makes no difference.
    pub const AS_FOUL_AS_IT_GETS: f32 = 2.0;

    /// Ground fouler than this is ground people will not sit on.
    pub const FOUL_ENOUGH_TO_WALK_AWAY_FROM: f32 = 0.35;

    /// What share of what goes in comes out able to grow.
    ///
    /// Most of a berry is digested. The pips are not, which is the whole
    /// mechanism: a hedge grows where the birds sit.
    pub const WHAT_COMES_THROUGH_WHOLE: f32 = 0.05;

    /// How much seed has to be lying on a tile before anything comes up.
    ///
    /// Two units of waste on the same ground, which is about what a camp
    /// leaves on one tile over a season. This was five times higher to begin
    /// with, and measured over six thousand turns it meant that of a thousand
    /// tiles carrying seed not one carried enough: people move about, and no
    /// single tile ever caught up.
    pub const ENOUGH_TO_COME_UP: f32 = 0.1;

    /// And how far the fouling has to have gone off before the ground under it
    /// is soil rather than muck.
    ///
    /// This is the wait the specification describes: "over time the waste
    /// should break down and seeds from the plants they have eaten should
    /// sprout". Nothing grows out of a fresh midden.
    pub const BROKEN_DOWN_ENOUGH_TO_GROW_IN: f32 = 0.1;

    /// Whether this ground is fouled enough that people will not stay on it.
    pub fn is_foul(&self) -> bool {
        self.fouling >= Self::FOUL_ENOUGH_TO_WALK_AWAY_FROM
    }

    /// Whether what was dropped here is ready to come up.
    ///
    /// The ground has to be able to grow anything at all as well, which is a
    /// question for the grid - see `Grid::will_anything_grow_on`.
    pub fn ready_to_sprout(&self) -> bool {
        self.seeds_dropped >= Self::ENOUGH_TO_COME_UP
            && self.fouling <= Self::BROKEN_DOWN_ENOUGH_TO_GROW_IN
    }

    /// Take the seed off the ground, because it has come up.
    pub fn it_came_up(&mut self) -> f32 {
        std::mem::take(&mut self.seeds_dropped)
    }

    /// Let a midden air out.
    ///
    /// `humidity` runs 0.0 to 1.0 and does most of the work: dry ground holds
    /// its smell a long time. Run only over the ground somebody has left
    /// something on - see `Grid::where_the_ground_is_doing_something` - since
    /// nowhere else has anything to air.
    ///
    /// The rate is the one the fouling always went off at: a tenth of a midden
    /// in a wet day, an order of magnitude faster than the rot under it used
    /// to go, which is why the ground people walked away from a season ago is
    /// ground they will sit on again. See ISSUES_FOUND #218 for why it is per
    /// day and divided into ticks.
    pub fn air_out(&mut self, humidity: f32, ticks: f32) {
        const FOULING_IN_A_DAY: f32 = 0.072;
        let a_day = crate::environment::seasons::TICKS_PER_DAY as f32;
        let fouling_rate = FOULING_IN_A_DAY / a_day;

        let wetness = humidity.clamp(0.0, 1.0);
        let activity = wetness * wetness;
        if activity <= 0.0 {
            return;
        }

        self.fouling = (self.fouling - self.fouling * fouling_rate * activity * ticks).max(0.0);
    }

    /// Whether somebody has left something on this ground.
    ///
    /// Muck and the seed in it: the two things that are *put* on a tile by
    /// something happening there. Two phases of the turn - what comes up out
    /// of a midden, and what a midden smells of - read this through the
    /// ground register rather than walking every tile in the world. See
    /// ISSUES_FOUND.md #128.
    pub fn has_somebody_left_something_here(&self) -> bool {
        self.fouling > 0.0 || self.seeds_dropped > 0.0
    }
}

impl Soil {
    /// How thick weed and vermin get before there is nothing left worth
    /// harvesting.
    pub const OVERRUN: f32 = 1.0;

    /// How fast a field goes back to meadow, per turn of growing weather.
    ///
    /// A season of neglect takes a field most of the way. Nothing comes in on
    /// ground that is not broken - a meadow cannot get any weedier than it
    /// already is.
    pub const WHAT_A_FIELD_LOSES_TO_NEGLECT: f32 = 0.004;

    /// And how much of it one visit from a farmer puts right.
    ///
    /// Not all of it. Weeding is a thing you do again next week.
    pub const WHAT_ONE_VISIT_PUTS_RIGHT: f32 = 0.45;

    /// Let a season of nobody looking after it tell on a field.
    ///
    /// `growing` is how good the weather is for growing anything at all, which
    /// is the same weather the crop wants: weeds do best exactly when the
    /// wheat does.
    pub fn nobody_weeded_this(&mut self, growing: f32, turns: f32) {
        let coming_on = growing.clamp(0.0, 1.0) * Self::WHAT_A_FIELD_LOSES_TO_NEGLECT * turns;

        self.weeds = (self.weeds + coming_on).clamp(0.0, Self::OVERRUN);
        // Vermin follow the crop rather than the weather, and are slower to
        // find a field than the weeds in it are.
        self.pests = (self.pests + coming_on * 0.6).clamp(0.0, Self::OVERRUN);
    }

    /// Somebody spent a turn in the field pulling things up and picking things
    /// off.
    pub fn somebody_worked_this_field(&mut self) {
        self.weeds = (self.weeds - Self::WHAT_ONE_VISIT_PUTS_RIGHT).max(0.0);
        self.pests = (self.pests - Self::WHAT_ONE_VISIT_PUTS_RIGHT).max(0.0);
    }

    /// Whether this field is worth a farmer's turn.
    pub fn wants_working(&self) -> bool {
        self.weeds + self.pests >= Self::WORTH_A_TURN
    }

    /// How far gone a field has to be before somebody walks over to it.
    pub const WORTH_A_TURN: f32 = 0.3;

    /// What share of what a field would carry actually comes off it.
    ///
    /// Weeds take the ground's share, vermin take the crop's. Both together,
    /// left alone, come to nearly nothing - which is the difference between
    /// dropping seed and farming.
    pub fn what_the_crop_keeps(&self) -> f32 {
        let taken = (self.weeds + self.pests) / (2.0 * Self::OVERRUN);
        (1.0 - taken.clamp(0.0, 1.0) * 0.9).clamp(0.1, 1.0)
    }
}
