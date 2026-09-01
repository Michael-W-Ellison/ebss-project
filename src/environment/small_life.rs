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
}

/// The small life of a whole country, a hunting ground at a time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmallLife {
    grounds: BTreeMap<(i32, i32), TheSmallLifeHere>,
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

        Self::HEAD_A_GOOD_HECTARE_CARRIES
            * Self::hectares_in_a_hunting_ground(cells_across)
            * cover.clamp(0.0, 1.0)
            * by_climate
            * by_season
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
        self.grounds.entry(ground).or_insert(TheSmallLifeHere {
            grazers: would_carry,
            hunters: would_carry * Self::WHAT_SHARE_ARE_HUNTERS,
            would_carry,
        });
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

        // Ground that will carry nothing loses what is on it rather than
        // holding it for ever - a salt flat in February is not a larder.
        if would_carry <= 0.0 {
            here.grazers = (here.grazers - here.grazers * 0.01 * ticks).max(0.0);
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

        let hunters_it_will_keep = here.grazers * Self::WHAT_SHARE_ARE_HUNTERS;
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
}
