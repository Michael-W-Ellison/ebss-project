// src/environment/small_life.rs
//! The lower tiers of the food web, held as a population rather than as
//! records.
//!
//! A rabbit is a bad thing to model as a record. It breeds fast, it dies
//! easily, and there are supposed to be hundreds of it - which on a discrete
//! model means small numbers with enormous variance and an absorbing barrier
//! at zero. Measured over five years on a hundred and forty-four hectares the
//! whole fauna ran 165, 858, 67, 27, 19, 7; on a hundred square kilometres
//! rabbits went 3.5, 60.5, 21, 2, 4 between years. That is not an ecology, it
//! is a random walk that ends at extinction, and no amount of tuning the birth
//! rate fixes it because the trouble is the representation.
//!
//! The other half of the same mistake was already here. `what_the_small_life
//! _gives` - the mice and voles a stoat lives on, which are assumed rather
//! than counted - was `cover x size-fit / hunters-sharing-it`, a constant. It
//! could not be drawn down, could not boom, could not crash, and nothing an
//! agent did could touch it. So the model held records where a record is
//! least reliable and an abstraction where an abstraction could not respond
//! to anything.
//!
//! This is the abstraction with a stock behind it. Each hunting ground - 80
//! cells square, which at ten metres a cell is sixty-four hectares - carries
//! a head of **grazers** (rabbits, voles, squirrels: what a trap catches and
//! what a stoat lives on) and a head of **hunters** (foxes, stoats, weasels:
//! what competes for them, and what steals a rabbit out of a snare before the
//! man who set it gets back). Both are numbers. Neither is ever a record, so
//! neither can go extinct by bad luck, and both answer to what the land will
//! carry.
//!
//! What the land will carry comes from the ground and the climate and the
//! season, which is the point of the exercise: an agent trapping a wood in
//! June and a salt flat in February are doing two different things, and until
//! now they were the same roll.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::flora::ClimateZone;
use super::seasons::Season;

/// What is living on one hunting ground, in head.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TheSmallLifeHere {
    /// Rabbits, voles, squirrels - what a snare catches.
    pub grazers: f32,
    /// Foxes, stoats, weasels - what catches them, and what takes a rabbit
    /// out of a snare before its owner gets back to it.
    pub hunters: f32,
    /// What this ground would carry at full stock, this season.
    ///
    /// Kept here rather than recomputed by every reader, because the readers
    /// are in other modules and most of them have no season to hand.
    /// `tick_a_ground` is the one thing that ever writes it, so it is the
    /// tick's output rather than a second opinion about the ground.
    pub would_carry: f32,

    /// Mice, voles and shrews: the band under the rabbits.
    ///
    /// The layer that was missing, and its absence is what emptied the sky.
    /// A kestrel is not a small fox - it does not live on rabbits at all,
    /// it lives on voles, and an owl the same. Held apart from the grazers
    /// rather than folded into them because the two behave differently in
    /// every way that matters: there are two orders of magnitude more of
    /// them, they come back four times as fast, and a snare is no use
    /// against them. Fold them together and a trapline is catching mice,
    /// which is not what a trapline is.
    #[serde(default)]
    pub rodents: f32,
    /// And what the ground would carry of them, this season.
    #[serde(default)]
    pub would_carry_rodents: f32,
}

impl TheSmallLifeHere {
    /// How thick on the ground the small life is, against what this ground
    /// would carry at its best, nought to one.
    ///
    /// The one number everything downstream reads: how likely a snare is to
    /// have anything in it, and how long a catch sits there unclaimed.
    pub fn how_thick_it_is(&self) -> f32 {
        if self.would_carry <= 0.0 {
            return 0.0;
        }
        (self.grazers / self.would_carry).clamp(0.0, 1.0)
    }

    /// And the same for the rodents, which is what a kestrel or an owl is
    /// actually reading when it looks at a field.
    pub fn how_thick_the_rodents_are(&self) -> f32 {
        if self.would_carry_rodents <= 0.0 {
            return 0.0;
        }
        (self.rodents / self.would_carry_rodents).clamp(0.0, 1.0)
    }

    /// Everything standing on this ground, counted in head of grazer.
    ///
    /// One owner for "how much small life is here", so that the two bands
    /// can be added without anything having to know the exchange rate twice.
    /// Every rule that was written about the grazers before the rodents
    /// existed - what keeps the foxes up, how fast a snared rabbit is stolen
    /// - reads this instead, which is why adding a band under them did not
    /// silently treble the foxes or empty the traplines.
    pub fn in_head_of_grazer(&self) -> f32 {
        self.grazers + self.rodents / SmallLife::HOW_MANY_RODENTS_MAKE_A_GRAZER
    }

    /// And what the ground would carry, on the same footing.
    pub fn would_carry_in_head_of_grazer(&self) -> f32 {
        self.would_carry + self.would_carry_rodents / SmallLife::HOW_MANY_RODENTS_MAKE_A_GRAZER
    }

    /// How thick the whole of the small life is here, both bands together.
    ///
    /// What a hunter deciding whether to stay is actually reading. Asking
    /// `how_thick_it_is` there is asking after the rabbits alone, which sends
    /// a kestrel off a field thick with voles.
    pub fn how_thick_the_small_life_is(&self) -> f32 {
        let would = self.would_carry_in_head_of_grazer();
        if would <= 0.0 {
            return 0.0;
        }
        (self.in_head_of_grazer() / would).clamp(0.0, 1.0)
    }
}

/// What the snares of a country have done, for measuring.
///
/// The same pattern as `WhatCarriedThemOff`: a tally nothing reads to make a
/// decision, kept so that a claim about trapping can be checked rather than
/// asserted. `caught` against `taken` is the number the specification is
/// about - how much of what went into the snares the people who set them
/// actually got.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WhatTheSnaresDid {
    /// Went into a snare
    pub caught: u64,
    /// And was gone before its owner got back
    pub robbed: u64,
    /// And was carried home
    pub taken: u64,
}

/// The small life of a whole country, a hunting ground at a time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmallLife {
    grounds: BTreeMap<(i32, i32), TheSmallLifeHere>,

    /// See [`WhatTheSnaresDid`].
    #[serde(default)]
    pub snare_tally: WhatTheSnaresDid,
}

impl SmallLife {
    /// Head of small grazers a hectare of the best ground carries.
    ///
    /// Wild rabbit densities run from one to about fifteen a hectare and the
    /// voles and squirrels beside them run far higher, but this counts *head
    /// worth catching* rather than every mouse in the field - which is the
    /// same thing `what_the_small_life_gives` has always meant by the small
    /// life. Eight to the hectare over sixty-four hectares is five hundred
    /// odd on the best ground in the country, and that is a wood you could
    /// run a trapline in.
    pub const HEAD_A_GOOD_HECTARE_CARRIES: f32 = 8.0;

    /// And how much of a hunting ground is a hectare.
    ///
    /// Derived, not written down twice: a hunting ground is
    /// `HOW_BIG_A_HUNTING_GROUND_IS` cells square and a cell is ten metres,
    /// so the number of hectares follows from those two and cannot drift from
    /// them.
    pub fn hectares_in_a_hunting_ground(cells_across: i32) -> f32 {
        const METRES_A_CELL: f32 = 10.0;
        const SQUARE_METRES_IN_A_HECTARE: f32 = 10_000.0;

        let side = cells_across as f32 * METRES_A_CELL;
        side * side / SQUARE_METRES_IN_A_HECTARE
    }

    /// Head of rodents a hectare of the best ground carries.
    ///
    /// Vole and mouse densities run from a few dozen to several hundred a
    /// hectare and swing by an order of magnitude between years; this is a
    /// quiet-year figure for all of them together. On ground that was all
    /// good it would be seven and a half thousand to a hunting ground and
    /// better than a million on a hundred square kilometres; what a real map
    /// carries once the cover, the climate and the season have had their say
    /// was measured at a hundred and fifty-five thousand, against ten
    /// thousand of grazers beside it. That is the scale the specification is
    /// about: "if there are tens of thousands of assumed mice across the
    /// 100km^2 then the hawks should have plenty of prey to hunt".
    ///
    /// **This is what was missing, and it is why the sky emptied.** With the
    /// rabbits alone a hunting ground grew a surplus of about two hundredths
    /// of a head a tick, and one kestrel eats that much by itself - so
    /// sixty-four hectares kept about one small predator, and every kestrel,
    /// heron, owl, eagle and otter on a hundred square kilometres was dead
    /// inside two years while its fields stood at half stock. The rodents
    /// are fifteen times the head and come back four times as fast, so the
    /// surplus under a hawk is nearer sixty times larger - which is the
    /// order of magnitude it was short by.
    pub const RODENTS_A_GOOD_HECTARE_CARRIES: f32 = 120.0;

    /// What one head of the grazer layer weighs.
    ///
    /// A rabbit, and the same two kilogrammes the rabbit record carried
    /// before it became a number. It is here rather than in the fauna table
    /// because the record it came from is gone: this is now the only place
    /// the model says how big the thing in a snare is, and what decides
    /// whether a hunter can lift one reads it.
    pub const WHAT_A_GRAZER_WEIGHS: f32 = 2.0;

    /// How much of a grazer one rodent is worth to something eating it.
    ///
    /// From the specification's own arithmetic, which is a better source
    /// than the weights: "a hawk can eat a rabbit a day, but a rabbit can
    /// also last two days" and "hawks will also hunt rodents like mice and
    /// will eat four of them in a day". Two days of hawk against a quarter
    /// of a day of hawk is eight to one.
    ///
    /// Not the ratio of the weights, which is nearer seventy to one. A hawk
    /// does not eat all of a rabbit and does eat all of a mouse, and this is
    /// the number about food rather than about carcases.
    pub const HOW_MANY_RODENTS_MAKE_A_GRAZER: f32 = 8.0;

    /// How fast the rodents come back, a tick at a time.
    ///
    /// Four times the grazers. A vole is breeding at three weeks old and a
    /// good year multiplies them severalfold; a field trapped bare in March
    /// is back by midsummer. This is what makes the layer under a small
    /// predator something it can lean on all year rather than something it
    /// eats once.
    pub const HOW_FAST_THE_RODENTS_COME_BACK: f32 = 0.006;

    /// What share of the grazers the ground will keep hunters for.
    ///
    /// Foxes proper run about one to the square kilometre, which is well
    /// under one to a hunting ground; stoats and weasels run far denser. What
    /// this stands for is all of them together, and it comes out at three or
    /// four on the best ground - enough that a snare left out is worth
    /// worrying about and not so many that nothing is ever in one.
    pub const WHAT_SHARE_ARE_HUNTERS: f32 = 0.008;

    /// How fast the grazers come back, a tick at a time.
    ///
    /// A rabbit population trebles in a season when it is let alone. At this
    /// rate a ground trapped down to a tenth is most of the way back inside a
    /// year, which is what makes trapping a thing you can overdo and recover
    /// from rather than a thing you do once.
    pub const HOW_FAST_THE_GRAZERS_COME_BACK: f32 = 0.0015;

    /// And the hunters, which is slower - they breed once a year and they are
    /// waiting on the grazers besides.
    pub const HOW_FAST_THE_HUNTERS_FOLLOW: f32 = 0.0004;

    /// What a snare on ground carrying all it can takes, in a tick.
    ///
    /// Twelve ticks to the day, so this is about a fifth of a chance a day
    /// and something in the snare inside four or five days. A real line is
    /// several snares and catches oftener than that; an agent that wants
    /// oftener sets more of them, which is what a trapline is.
    pub const WHAT_A_SNARE_TAKES_ON_FULL_GROUND: f32 = 0.02;

    /// What a whole hunting ground gives a trapline in a tick, at full stock,
    /// however many snares are on it.
    ///
    /// Set just under what the ground actually grows. A logistic population
    /// at capacity `K` with growth `r` has a surplus of `rK/4` - here 0.0015
    /// times five hundred over four, near enough a fifth of a head a tick, or
    /// two and a quarter a day off sixty-four hectares. This is a shade under
    /// that, so a full line is *just* sustainable and two settlements working
    /// one wood are not. That is the specification's "agents could tip the
    /// scale", and it is a scale rather than a switch.
    pub const WHAT_A_GROUND_GIVES_A_LINE: f32 = 0.15;

    /// What takes the catch out of a snare in a settled country, in a tick.
    ///
    /// Most of a week before something finds it in a country with plenty in
    /// it, which is what makes a trapline worth keeping at all.
    ///
    /// **Measured down from a third of this.** At 0.03 a tick - a couple of
    /// days - a settlement of twelve caught 213 over a year and carried home
    /// **28**. Losing seven catches in eight in a full wood is not a pinch,
    /// it is a trapline that does not work, and it made the whole activity
    /// pointless in exactly the case it should pay best. The pinch belongs at
    /// the other end of the scale, where `WHAT_A_HUNGRY_COUNTRY_TAKES` is,
    /// and the cap does that.
    pub const WHAT_A_QUIET_COUNTRY_TAKES: f32 = 0.01;

    /// And the most it can ever be, when the game is gone and the foxes are
    /// not.
    ///
    /// Half a chance a tick: a catch left one turn is likely gone. That is
    /// the pinch the specification asks for - trapping a ground out does not
    /// only make the snares emptier, it makes the ones that do fill worth
    /// less, because you have to be standing there.
    pub const WHAT_A_HUNGRY_COUNTRY_TAKES: f32 = 0.5;

    /// What a hunting ground will carry, in head of small grazers.
    ///
    /// The specification's "the climate and area could dictate the carrying
    /// capacity". Area is the hunting ground, which is one size everywhere;
    /// what varies is what grows on it and how hard the year is.
    ///
    /// `cover` stands for how much the ground grows, exactly as it does in
    /// `what_the_small_life_gives` - a wood and a reed bed are thick with it,
    /// a salt flat has none - so the two read one number and cannot disagree
    /// about which ground is rich.
    pub fn what_this_ground_will_carry(
        cover: f32,
        climate: ClimateZone,
        season: Season,
        cells_across: i32,
    ) -> f32 {
        Self::HEAD_A_GOOD_HECTARE_CARRIES
            * Self::what_a_hectare_of_this_is_worth(cover, climate, season)
            * Self::hectares_in_a_hunting_ground(cells_across)
    }

    /// How many rodents a piece of ground carries for every grazer on it.
    ///
    /// Derived rather than written down twice. Both bands live on what the
    /// ground grows, so the same cover, the same climate and the same season
    /// decide both and only the density differs - which means a ground's
    /// rodent stock follows from its grazer stock and the two can never come
    /// to disagree about which month is hard or which field is poor. It is
    /// also why nothing that ticks a ground has to be told about the rodents
    /// separately.
    pub const RODENTS_TO_A_GRAZER_ON_THE_GROUND: f32 =
        Self::RODENTS_A_GOOD_HECTARE_CARRIES / Self::HEAD_A_GOOD_HECTARE_CARRIES;

    /// And what it will carry of rodents, which is the same ground read for
    /// a different animal.
    pub fn what_this_ground_will_carry_of_rodents(
        cover: f32,
        climate: ClimateZone,
        season: Season,
        cells_across: i32,
    ) -> f32 {
        Self::what_this_ground_will_carry(cover, climate, season, cells_across)
            * Self::RODENTS_TO_A_GRAZER_ON_THE_GROUND
    }

    /// What a hectare of this ground, in this climate, in this season, is
    /// worth against a hectare of the best of it in summer.
    fn what_a_hectare_of_this_is_worth(cover: f32, climate: ClimateZone, season: Season) -> f32 {
        let by_climate = match climate {
            ClimateZone::Temperate => 1.0,
            ClimateZone::Tropical => 1.1,
            ClimateZone::Arctic => 0.15,
            ClimateZone::Desert => 0.08,
        };

        // A hard year does not kill the ground, it thins what stands on it.
        // Nought would mean a country that empties every winter and refills
        // every spring, which is a worse lie than no seasons at all.
        let by_season = match season {
            Season::Spring => 0.85,
            Season::Summer => 1.0,
            Season::Fall => 1.0,
            Season::Winter => 0.45,
        };

        cover.clamp(0.0, 1.0) * by_climate * by_season
    }

    /// What is on this ground now.
    ///
    /// Ground nobody has asked about reads as empty rather than as stocked,
    /// which is why `settle` exists and is called before anything draws.
    pub fn here(&self, ground: (i32, i32)) -> TheSmallLifeHere {
        self.grounds.get(&ground).copied().unwrap_or_default()
    }

    /// Stock a ground that has never been looked at, and bring it up to now.
    ///
    /// A country is not empty of rabbits on the morning it is made. The first
    /// time anything asks about a piece of ground it is found already carrying
    /// what it will carry, and after that it grows or is drawn down like
    /// anything else.
    pub fn settle(&mut self, ground: (i32, i32), would_carry: f32) {
        let rodents = would_carry * Self::RODENTS_TO_A_GRAZER_ON_THE_GROUND;
        self.grounds.entry(ground).or_insert(TheSmallLifeHere {
            grazers: would_carry,
            hunters: (would_carry + rodents / Self::HOW_MANY_RODENTS_MAKE_A_GRAZER)
                * Self::WHAT_SHARE_ARE_HUNTERS,
            would_carry,
            rodents,
            would_carry_rodents: rodents,
        });
    }

    /// Take up to `wanted` head of rodents off this ground.
    ///
    /// Held apart from `take` rather than converted into it, because what
    /// comes off a ground has to come off the band it was actually taken
    /// from: an owl working a field all winter thins the voles and leaves
    /// the rabbits, and a trapline does the reverse.
    pub fn take_rodents(&mut self, ground: (i32, i32), wanted: f32) -> f32 {
        let Some(here) = self.grounds.get_mut(&ground) else {
            return 0.0;
        };

        let got = wanted.max(0.0).min(here.rodents.max(0.0));
        here.rodents = (here.rodents - got).max(0.0);
        got
    }

    /// Take up to `wanted` head off this ground, and say what was actually
    /// there to take.
    ///
    /// This is the whole of how the small life stops being infinite. A stoat
    /// working a wood that has been trapped out gets what is left, which is
    /// less than it wants, and that is a stoat that leaves.
    pub fn take(&mut self, ground: (i32, i32), wanted: f32) -> f32 {
        let Some(here) = self.grounds.get_mut(&ground) else {
            return 0.0;
        };

        let got = wanted.max(0.0).min(here.grazers.max(0.0));
        here.grazers = (here.grazers - got).max(0.0);
        got
    }

    /// Bring one ground on by a tick.
    ///
    /// Grazers grow logistically towards what the land will carry. Hunters
    /// grow logistically towards a share of the grazers, which is the whole
    /// of the coupling: they follow the game up and they follow it down,
    /// with a lag, and they never oscillate the way a proper predator-prey
    /// pair does. That is deliberate. A model that swings is a model that
    /// empties a ground of foxes every few years by arithmetic rather than by
    /// anything that happened, and the point of taking the small life out of
    /// records was to stop exactly that.
    pub fn tick_a_ground(&mut self, ground: (i32, i32), would_carry: f32, ticks: f32) {
        self.settle(ground, would_carry);
        let Some(here) = self.grounds.get_mut(&ground) else {
            return;
        };

        here.would_carry = would_carry;
        here.would_carry_rodents = would_carry * Self::RODENTS_TO_A_GRAZER_ON_THE_GROUND;

        // Ground that will carry nothing loses what is on it rather than
        // holding it for ever - a salt flat in February is not a larder.
        if would_carry <= 0.0 {
            here.grazers = (here.grazers - here.grazers * 0.01 * ticks).max(0.0);
            here.rodents = (here.rodents - here.rodents * 0.01 * ticks).max(0.0);
            here.hunters = (here.hunters - here.hunters * 0.01 * ticks).max(0.0);
            return;
        }

        // A ground trapped down to nothing has to be able to come back, and a
        // logistic curve through nought never leaves it. What comes back in
        // is what walks in from the ground next door, which nothing here has
        // to model: a floor of one head is what "there are always a few
        // about" comes to.
        const ALWAYS_A_FEW_ABOUT: f32 = 1.0;
        here.grazers = here.grazers.max(ALWAYS_A_FEW_ABOUT.min(would_carry));

        let room = 1.0 - (here.grazers / would_carry).clamp(0.0, 1.0);
        here.grazers =
            (here.grazers + here.grazers * Self::HOW_FAST_THE_GRAZERS_COME_BACK * room * ticks)
                .clamp(0.0, would_carry);

        // And the rodents, on the same curve and four times as fast.
        here.rodents = here
            .rodents
            .max((ALWAYS_A_FEW_ABOUT * Self::RODENTS_TO_A_GRAZER_ON_THE_GROUND)
                .min(here.would_carry_rodents));
        let room_below = 1.0 - (here.rodents / here.would_carry_rodents).clamp(0.0, 1.0);
        here.rodents = (here.rodents
            + here.rodents * Self::HOW_FAST_THE_RODENTS_COME_BACK * room_below * ticks)
            .clamp(0.0, here.would_carry_rodents);

        // What keeps the foxes up is everything under them, not the rabbits
        // alone. Reading the grazers by themselves here is how adding a band
        // beneath them would have quietly emptied the stoats out of a field
        // full of voles.
        let hunters_it_will_keep = here.in_head_of_grazer() * Self::WHAT_SHARE_ARE_HUNTERS;
        if hunters_it_will_keep <= 0.0 {
            here.hunters = 0.0;
            return;
        }

        // Towards it from either side: up when the game is thick, down when
        // it has been trapped out, at the same rate.
        let short_by = (hunters_it_will_keep - here.hunters) / hunters_it_will_keep;
        here.hunters = (here.hunters
            + hunters_it_will_keep * Self::HOW_FAST_THE_HUNTERS_FOLLOW * short_by * ticks)
            .max(0.0);
    }

    /// How much of the difference between two neighbouring grounds crosses
    /// between them in a tick.
    ///
    /// Slow. This is animals working outwards into ground that is emptier
    /// than the ground they are on, not a herd migrating: at this rate a
    /// wholly trapped-out block is drawing meaningfully on its neighbours
    /// within a season and is not refilled overnight by them.
    pub const HOW_FAST_THEY_SPREAD: f32 = 0.002;

    /// Let the small life work outwards into ground that is emptier than
    /// where it is.
    ///
    /// A trapped-out wood used to come back only off its own floor - "there
    /// are always a few about" - which is a source of animals from nowhere
    /// and says nothing about what is around it. What actually refills a
    /// worked ground is the ground next door, and having that means the
    /// country is joined up: a settlement that traps one block hard draws on
    /// the blocks around it, and a block surrounded by worked ground stays
    /// thin however long it is left.
    ///
    /// It moves on **crowding rather than head count** - the share of what
    /// each ground carries, not how many are on it - so a rich block does
    /// not drain into a barren one just because the barren one is emptier in
    /// absolute terms. Nothing lives on a salt flat however many rabbits are
    /// in the wood beside it.
    ///
    /// Each unordered pair is visited once and the flow is subtracted from
    /// one side and added to the other, so head is conserved exactly. That
    /// matters: an exchange written as "move towards the average of my
    /// neighbours" is not symmetric, and quietly invents or destroys animals
    /// every tick.
    pub fn let_them_spread(&mut self, ticks: f32) {
        let grounds: Vec<(i32, i32)> = self.grounds.keys().copied().collect();
        let mut moves: Vec<((i32, i32), (i32, i32), f32, f32)> = Vec::new();

        for &(gx, gy) in &grounds {
            let here = self.here((gx, gy));
            if here.would_carry <= 0.0 {
                continue;
            }

            // East and south only, which is how each unordered pair is
            // reached exactly once.
            for (dx, dy) in [(1, 0), (0, 1)] {
                let over_there = (gx + dx, gy + dy);
                let Some(there) = self.grounds.get(&over_there).copied() else {
                    continue;
                };
                if there.would_carry <= 0.0 {
                    continue;
                }

                let across = Self::HOW_FAST_THEY_SPREAD
                    * (here.how_thick_it_is() - there.how_thick_it_is())
                    * here.would_carry.min(there.would_carry)
                    * ticks;

                // The rodents work outwards on the same rule and their own
                // crowding. A field thick with voles beside one that has
                // been hunted out is the same statement as a wood thick with
                // rabbits beside a trapped one, and the two bands are not
                // in step: an owl can hunt the voles out of ground whose
                // rabbits nobody has touched.
                let below = Self::HOW_FAST_THEY_SPREAD
                    * (here.how_thick_the_rodents_are() - there.how_thick_the_rodents_are())
                    * here.would_carry_rodents.min(there.would_carry_rodents)
                    * ticks;

                if across.abs() > f32::EPSILON || below.abs() > f32::EPSILON {
                    moves.push(((gx, gy), over_there, across, below));
                }
            }
        }

        for (from, to, across, below) in moves {
            // Never move more than is actually standing there, whichever way
            // it is going.
            let grazers = if across > 0.0 {
                across.min(self.here(from).grazers)
            } else {
                -((-across).min(self.here(to).grazers))
            };
            let rodents = if below > 0.0 {
                below.min(self.here(from).rodents)
            } else {
                -((-below).min(self.here(to).rodents))
            };

            if let Some(here) = self.grounds.get_mut(&from) {
                here.grazers = (here.grazers - grazers).max(0.0);
                here.rodents = (here.rodents - rodents).max(0.0);
            }
            if let Some(there) = self.grounds.get_mut(&to) {
                there.grazers = (there.grazers + grazers).max(0.0);
                there.rodents = (there.rodents + rodents).max(0.0);
            }
        }
    }

    /// Every ground this country has looked at, for measuring and for tests.
    pub fn all_grounds(&self) -> impl Iterator<Item = (&(i32, i32), &TheSmallLifeHere)> {
        self.grounds.iter()
    }

    /// The head of small grazers in the whole country.
    pub fn how_many_grazers(&self) -> f32 {
        self.grounds.values().map(|here| here.grazers).sum()
    }

    /// And of small hunters.
    pub fn how_many_hunters(&self) -> f32 {
        self.grounds.values().map(|here| here.hunters).sum()
    }

    /// And of rodents, which is the biggest number in the model and the one
    /// the sky is standing on.
    pub fn how_many_rodents(&self) -> f32 {
        self.grounds.values().map(|here| here.rodents).sum()
    }
}

/// A snare set in the ground, and whatever has gone into it.
///
/// The agent's way into the abstracted tier. Nothing else reaches it: a
/// person cannot stalk a number, and there is no rabbit record to walk up to
/// any more, so a settlement that wants small meat sets a line and goes round
/// it. That is what people actually did, and it is the reason abstracting the
/// lower tiers does not cost the agents anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snare {
    pub at: (i32, i32),
    pub set_by: uuid::Uuid,
    pub set_at: u32,
    /// The tick something went into it, if anything has.
    pub caught_at: Option<u32>,
}

impl Snare {
    /// Whether there is anything in this one worth walking to.
    pub fn is_holding_something(&self) -> bool {
        self.caught_at.is_some()
    }
}

impl TheSmallLifeHere {
    /// The chance, in one tick, that a snare on this ground takes something,
    /// with `sharing_it` snares set on the same ground.
    ///
    /// "The rate of success and speed of catch could be based on the total
    /// population." Two things bound it, and both had to be there.
    ///
    /// The first is how thick the ground is, in straight proportion: a snare
    /// in a full wood catches inside about four days, one in a wood that has
    /// been worked all winter catches when it catches.
    ///
    /// The second is that **the ground gives what it gives, however much
    /// string is on it** - the same rule `what_the_small_life_gives` applies
    /// to hunters sharing a range, for the same reason. Without it a line is
    /// a multiplier: twelve agents at a dozen snares each put a hundred and
    /// forty-four on one sixty-four-hectare block, which at a snare's own
    /// rate is **thirty head a day against a ground whose whole surplus is
    /// two**. Measured: the camp's ground went to eight thousandths of what
    /// it carries inside three months and stayed there, every catch was
    /// robbed before anyone reached it because a ground with no game on it is
    /// a ground of hungry foxes, and a settlement of twelve took **one**
    /// rabbit in a year.
    ///
    /// So a longer line reaches the ground's yield sooner and more reliably,
    /// and never exceeds it. That is what a trapline is.
    pub fn how_likely_a_snare_takes_something(&self, sharing_it: usize) -> f32 {
        let a_snares_own_rate = SmallLife::WHAT_A_SNARE_TAKES_ON_FULL_GROUND;
        let its_share_of_the_ground =
            SmallLife::WHAT_A_GROUND_GIVES_A_LINE / sharing_it.max(1) as f32;

        a_snares_own_rate.min(its_share_of_the_ground) * self.how_thick_it_is()
    }

    /// And the chance, in one tick, that something else gets to the catch
    /// first.
    ///
    /// "A decrease in rabbit population could decrease the time an agent has
    /// to recover a trapped rabbit before a fox steals the catch." It falls
    /// out of the lag rather than being written down: the hunters track a
    /// share of the grazers *behind* them, so when the game is trapped out
    /// the foxes are still there and there is nothing else for them to eat.
    /// Hunters-per-head-of-game is that, and at full stock it is exactly
    /// `WHAT_SHARE_ARE_HUNTERS` - so this reads one against the other and a
    /// settled country comes out at the quiet rate by construction.
    pub fn how_likely_the_catch_is_taken(&self) -> f32 {
        // Everything under the foxes, on the footing `tick_a_ground` keeps
        // them on. Reading the rabbits alone here while the foxes are fed by
        // rabbits *and* voles is the same question answered in two places,
        // and it would have made a settled country look like a hungry one -
        // three times the foxes to a head of game, so every catch robbed at
        // the hungry rate and the trapline pointless in a full wood.
        let under_them = self.in_head_of_grazer();
        if under_them <= 0.0 {
            return SmallLife::WHAT_A_HUNGRY_COUNTRY_TAKES;
        }

        let per_head = self.hunters / under_them;
        let against_a_settled_country = per_head / SmallLife::WHAT_SHARE_ARE_HUNTERS;

        (SmallLife::WHAT_A_QUIET_COUNTRY_TAKES * against_a_settled_country)
            .clamp(0.0, SmallLife::WHAT_A_HUNGRY_COUNTRY_TAKES)
    }
}

impl SmallLife {
    /// Bring every snare in the country on by a tick: what goes into them,
    /// and what takes it out again before its owner gets back.
    ///
    /// The catch comes off the ground it was taken on, so a settlement that
    /// works one wood hard thins that wood - and thins it for the stoats and
    /// foxes living there as well, which is "agents could tip the scale".
    ///
    /// A snare a person is standing beside is not robbed: the whole point of
    /// going round the line is being there, and a catch taken in the same
    /// tick its owner reaches it is a catch he got.
    pub fn tick_the_snares<F>(
        &mut self,
        snares: &mut [Snare],
        now: u32,
        whose_ground: F,
        rng: &mut rand::rngs::StdRng,
    ) where
        F: Fn((i32, i32)) -> (i32, i32),
    {
        use rand::Rng;

        // How much string is on each piece of ground, which is what decides
        // a snare's share of what that ground gives.
        let mut lines_on: BTreeMap<(i32, i32), usize> = BTreeMap::new();
        for snare in snares.iter() {
            *lines_on.entry(whose_ground(snare.at)).or_insert(0) += 1;
        }

        for snare in snares.iter_mut() {
            let ground = whose_ground(snare.at);
            let here = self.here(ground);
            let sharing_it = lines_on.get(&ground).copied().unwrap_or(1);

            match snare.caught_at {
                None => {
                    if rng.gen::<f32>() < here.how_likely_a_snare_takes_something(sharing_it) {
                        // One head, off this ground. A snare takes one thing.
                        let got = self.take(ground, 1.0);
                        if got > 0.0 {
                            snare.caught_at = Some(now);
                            self.snare_tally.caught += 1;
                        }
                    }
                }
                Some(_) => {
                    if rng.gen::<f32>() < here.how_likely_the_catch_is_taken() {
                        // Something else had it. The head is already off the
                        // ground - it was caught - so nothing else comes off
                        // here; what is lost is the agent's morning.
                        snare.caught_at = None;
                        self.snare_tally.robbed += 1;
                    }
                }
            }
        }
    }
}
