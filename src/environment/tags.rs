// src/environment/tags.rs
//! What a thing *is*, as against what it is called, and what a job *wants*,
//! as against which named thing it wants.
//!
//! Everything in this model has been addressed by name. A recipe asks for
//! `"lashing"`, a tent asks for `ResourceType::Hides`, a pit asks whether it
//! holds a `"bowl"` or a `"basket"`. That works exactly until the world has
//! two things that would do, and then it silently prefers the one somebody
//! happened to type: a settlement holding four fired pots and no bowl has
//! nothing to line a pit with, because the pit was written before the pot
//! existed.
//!
//! Two vocabularies here, and the difference between them is the whole point:
//!
//! - **[`Tag`] is what a thing is.** Descriptive, ungraded, and a thing has
//!   several: a fired pot is a food container, a water container and a
//!   *fragile* container all at once, and the third is why you do not take it
//!   hunting. Tags do not compare - there is no such thing as being more of a
//!   pole than something else is.
//! - **[`Capability`] is what a job wants**, and it *is* graded, because "I
//!   need something to dig with" has better and worse answers. A shovel and a
//!   pointed stick both answer it; they do not answer it equally.
//!
//! ## The coefficient is a view, not a second opinion
//!
//! A capability coefficient says how much of the best available advantage a
//! thing delivers: 1.0 is the best thing in the world for that job, 0.0 is
//! bare hands.
//!
//! For four of the eight capabilities that fact **is already written down** -
//! [`crate::environment::making::EVERY_TOOL`] has said for a long time that a
//! shovel multiplies mining by 1.9 and a digging stick by 1.2 - only on the
//! axis of the *trade* rather than of the *capability*. Writing a second table
//! of digging coefficients beside it would be two spellings of one question,
//! which is a mistake this codebase has made and paid for at least three
//! times (see `could_bring_it_down`, and ISSUES_FOUND #203). So those four are
//! **derived** from the tool table and cannot drift from it: add a better
//! shovel and every coefficient on that capability renormalises itself.
//!
//! The other four - carrying, holding water, roofing, and making holes - have
//! no trade behind them, so they are declared here and the tests assert that
//! no capability is described in both places.

use crate::agents::skills::SkillType;
use super::making::{Tool, EVERY_TOOL};

/// What a thing is.
///
/// Ungraded and plural. A thing carries every tag that is true of it, and the
/// ones that sound like drawbacks are as load-bearing as the ones that sound
/// like virtues: `FragileContainer` is why a pot is not what you carry on a
/// hunt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tag {
    // ---- containers ------------------------------------------------------
    /// Will hold food without it falling out or going over faster
    FoodContainer,
    /// Will hold a liquid
    WaterContainer,
    /// Breaks if it is dropped, and so is not what you take out with you
    FragileContainer,
    /// Holds a load on the move
    CarryingContainer,

    // ---- weapons ---------------------------------------------------------
    /// Made for bringing an animal down
    HuntingWeapon,
    /// Kills by going in rather than by landing heavily
    PiercingWeapon,
    /// Used at arm's length and a bit: the reach of a shaft
    MediumRangeMelee,
    /// Used in the hand, at the length of the hand
    ShortRangeMelee,
    /// Leaves the hand on its way to the thing
    ThrownWeapon,

    // ---- fibre and cordage ----------------------------------------------
    /// Something cordage is made *of*
    FiberSource,
    /// Cordage made of it will not take much
    LowStrengthCordageMaterial,
    /// Cordage: the finished thing, whatever it was made of
    Cordage,

    // ---- shelter ---------------------------------------------------------
    /// Sheds weather and will go over a frame: hide, thatch, a woven mat
    FlexibleCovering,
    /// Will stand up and hold something else up
    Pole,
    /// Will not bend, and so is a wall rather than a roof
    RigidBuildingMaterial,

    // ---- stock -----------------------------------------------------------
    /// Will take an edge if it is struck right
    Knappable,
    /// Has metal in it, or is metal
    Metallic,
    /// Came off something that was alive and is still soft
    Hide,
    /// Grew as wood
    Timber,
    /// Earth that has been through a fire
    FiredEarth,

    // ---- food ------------------------------------------------------------
    /// Will keep, having been dried, salted, smoked or buried
    Preserved,
    /// Will not keep
    Perishable,
}

impl Tag {
    /// The name this tag goes by in the specification.
    pub fn called(&self) -> &'static str {
        match self {
            Tag::FoodContainer => "food_container",
            Tag::WaterContainer => "water_container",
            Tag::FragileContainer => "fragile_container",
            Tag::CarryingContainer => "carrying_container",
            Tag::HuntingWeapon => "hunting_weapon",
            Tag::PiercingWeapon => "piercing_weapon",
            Tag::MediumRangeMelee => "medium_range_melee",
            Tag::ShortRangeMelee => "short_range_melee",
            Tag::ThrownWeapon => "thrown_weapon",
            Tag::FiberSource => "fiber_source",
            Tag::LowStrengthCordageMaterial => "low_strength_cordage_material",
            Tag::Cordage => "cordage",
            Tag::FlexibleCovering => "flexible_covering",
            Tag::Pole => "pole",
            Tag::RigidBuildingMaterial => "rigid_building_material",
            Tag::Knappable => "knappable",
            Tag::Metallic => "metallic",
            Tag::Hide => "hide",
            Tag::Timber => "timber",
            Tag::FiredEarth => "fired_earth",
            Tag::Preserved => "preserved",
            Tag::Perishable => "perishable",
        }
    }
}

/// Everything a job can ask for by capability rather than by name.
///
/// Eight, and graded: each one has a best answer in this world, and everything
/// else that answers it does so less well. See [`how_well_this_serves`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// Something to dig with
    DiggingTool,
    /// Something to cut with
    CuttingTool,
    /// Something to make a hole with
    PiercingTool,
    /// Something to carry a load in
    CarryingContainer,
    /// Something to carry water in
    WaterContainer,
    /// Something to build a roof out of
    ShelterMaterial,
    /// Something to take a fish with
    FishingGear,
    /// Something to bring an animal down with
    HuntingWeapon,
}

impl Capability {
    /// The name this capability goes by in the specification.
    pub fn called(&self) -> &'static str {
        match self {
            Capability::DiggingTool => "digging_tool",
            Capability::CuttingTool => "cutting_tool",
            Capability::PiercingTool => "piercing_tool",
            Capability::CarryingContainer => "carrying_container",
            Capability::WaterContainer => "water_container",
            Capability::ShelterMaterial => "shelter_material",
            Capability::FishingGear => "fishing_gear",
            Capability::HuntingWeapon => "hunting_weapon",
        }
    }

    /// The trade whose tool table already answers this, where one does.
    ///
    /// Four of the eight are jobs somebody has a *skill* at, and this model
    /// has priced every tool against every trade since long before capability
    /// tags existed. Those four read that table rather than restating it.
    ///
    /// A caveat worth stating rather than hiding: a capability is only as
    /// fine-grained as the trade behind it. `Mining` in this model is both
    /// quarrying stone and digging a hole, so a metal axe outranks a shovel
    /// as a `DiggingTool` - which is right for a seam of flint and wrong for
    /// a storage pit. Splitting the trade is the fix, and splitting a trade
    /// moves every measurement in the project; the capability layer is not
    /// the place to paper over it with a second number.
    pub fn the_trade_behind_it(&self) -> Option<SkillType> {
        match self {
            Capability::DiggingTool => Some(SkillType::Mining),
            Capability::CuttingTool => Some(SkillType::Leatherworking),
            Capability::FishingGear => Some(SkillType::Fishing),
            Capability::HuntingWeapon => Some(SkillType::Hunting),

            // No trade is "making holes", "carrying", "holding water" or
            // "being a roof", so these four are declared below.
            Capability::PiercingTool
            | Capability::CarryingContainer
            | Capability::WaterContainer
            | Capability::ShelterMaterial => None,
        }
    }

    /// Every capability there is, so nothing can be added and forgotten.
    pub fn every_one() -> &'static [Capability] {
        &[
            Capability::DiggingTool,
            Capability::CuttingTool,
            Capability::PiercingTool,
            Capability::CarryingContainer,
            Capability::WaterContainer,
            Capability::ShelterMaterial,
            Capability::FishingGear,
            Capability::HuntingWeapon,
        ]
    }
}

/// One thing, and everything that is true of it.
#[derive(Debug, Clone, Copy)]
pub struct Tagged {
    /// What it is called, as it is carried
    pub called: &'static str,
    /// What it is
    pub is: &'static [Tag],
}

/// What everything in this world is.
///
/// The three worked examples the specification gives, written against this
/// world's vocabulary: a fired pot is its gourd, a spear is its flint spear,
/// and flax and cotton are its cordage grass.
pub const EVERYTHING_TAGGED: &[Tagged] = &[
    // ---- vessels ---------------------------------------------------------
    //
    // The specification's gourd. Three tags and the third is the interesting
    // one: a pot will hold water and food and will not survive being dropped,
    // and a model that only recorded the two useful facts could never explain
    // why anybody would prefer a leather bag.
    Tagged {
        called: "claypot",
        is: &[
            Tag::FoodContainer,
            Tag::WaterContainer,
            Tag::FragileContainer,
            Tag::CarryingContainer,
            Tag::FiredEarth,
        ],
    },
    Tagged {
        called: "stoneware",
        is: &[
            Tag::FoodContainer,
            Tag::WaterContainer,
            Tag::FragileContainer,
            Tag::FiredEarth,
        ],
    },
    Tagged {
        called: "bowl",
        is: &[Tag::FoodContainer, Tag::WaterContainer, Tag::Timber],
    },
    Tagged {
        called: "basket",
        is: &[Tag::FoodContainer, Tag::CarryingContainer],
    },
    Tagged {
        called: "leatherbag",
        is: &[
            Tag::FoodContainer,
            Tag::WaterContainer,
            Tag::CarryingContainer,
            Tag::Hide,
        ],
    },
    Tagged {
        called: "travois",
        is: &[Tag::CarryingContainer],
    },
    Tagged {
        called: "handcart",
        is: &[Tag::CarryingContainer],
    },

    // ---- weapons ---------------------------------------------------------
    //
    // The specification's flint spear, exactly as it lists it.
    Tagged {
        called: "spear",
        is: &[
            Tag::HuntingWeapon,
            Tag::PiercingWeapon,
            Tag::MediumRangeMelee,
            Tag::ThrownWeapon,
        ],
    },
    Tagged {
        called: "metalspear",
        is: &[
            Tag::HuntingWeapon,
            Tag::PiercingWeapon,
            Tag::MediumRangeMelee,
            Tag::ThrownWeapon,
            Tag::Metallic,
        ],
    },
    Tagged {
        called: "sharpenedstick",
        is: &[
            Tag::HuntingWeapon,
            Tag::PiercingWeapon,
            Tag::MediumRangeMelee,
            Tag::Timber,
        ],
    },
    Tagged {
        called: "sling",
        is: &[Tag::HuntingWeapon, Tag::ThrownWeapon],
    },
    Tagged {
        called: "bow",
        is: &[Tag::HuntingWeapon, Tag::PiercingWeapon, Tag::ThrownWeapon],
    },
    Tagged {
        called: "handaxe",
        is: &[Tag::ShortRangeMelee, Tag::Knappable],
    },
    Tagged {
        called: "stoneknife",
        is: &[Tag::ShortRangeMelee, Tag::PiercingWeapon],
    },
    Tagged {
        called: "metalknife",
        is: &[Tag::ShortRangeMelee, Tag::PiercingWeapon, Tag::Metallic],
    },
    Tagged {
        called: "metalaxe",
        is: &[Tag::ShortRangeMelee, Tag::Metallic],
    },

    // ---- fibre and cordage ----------------------------------------------
    //
    // The specification's cordage grass. Flax and cotton are what this world
    // grows instead, and the distinction it draws - a fibre *source* is not
    // cordage, and weak cordage is not the same thing as cordage - is the
    // distinction the recipe chain already makes in named steps.
    Tagged {
        called: "flax",
        is: &[Tag::FiberSource, Tag::LowStrengthCordageMaterial],
    },
    Tagged {
        called: "cotton",
        is: &[Tag::FiberSource, Tag::LowStrengthCordageMaterial],
    },
    Tagged {
        called: "rettedflax",
        is: &[Tag::FiberSource],
    },
    Tagged {
        called: "lashing",
        is: &[Tag::Cordage],
    },

    // ---- the stuff of the world -----------------------------------------
    Tagged {
        called: "wood",
        is: &[Tag::Pole, Tag::Timber, Tag::RigidBuildingMaterial],
    },
    Tagged {
        called: "hides",
        is: &[Tag::FlexibleCovering, Tag::Hide],
    },
    Tagged {
        called: "leather",
        is: &[Tag::FlexibleCovering, Tag::Hide],
    },
    Tagged {
        called: "stone",
        is: &[Tag::RigidBuildingMaterial, Tag::Knappable],
    },
    Tagged {
        called: "flint",
        is: &[Tag::Knappable],
    },
    Tagged {
        called: "knappedtip",
        is: &[Tag::PiercingWeapon, Tag::Knappable],
    },
    Tagged {
        called: "bricks",
        is: &[Tag::RigidBuildingMaterial, Tag::FiredEarth],
    },
    Tagged {
        called: "iron",
        is: &[Tag::Metallic],
    },
    Tagged {
        called: "shinylump",
        is: &[Tag::Metallic],
    },
    Tagged {
        called: "metalblade",
        is: &[Tag::PiercingWeapon, Tag::Metallic],
    },

    // ---- what keeps and what does not -----------------------------------
    Tagged {
        called: "meatstrips",
        is: &[Tag::Preserved],
    },
    Tagged {
        called: "fishstrips",
        is: &[Tag::Preserved],
    },
    Tagged {
        called: "meatportions",
        is: &[Tag::Perishable],
    },
    Tagged {
        called: "fishportions",
        is: &[Tag::Perishable],
    },
];

/// What this thing is, if anybody has said.
///
/// An untagged thing comes back empty rather than guessing. Nothing in this
/// world should be assumed to be a container because its name ends in a
/// certain way.
pub fn what_this_is(called: &str) -> &'static [Tag] {
    EVERYTHING_TAGGED
        .iter()
        .find(|tagged| tagged.called == called)
        .map(|tagged| tagged.is)
        .unwrap_or(&[])
}

/// Whether this thing is one of those.
pub fn is_this_a(called: &str, tag: Tag) -> bool {
    what_this_is(called).contains(&tag)
}

/// Everything in the world that is one of those.
pub fn everything_that_is(tag: Tag) -> impl Iterator<Item = &'static str> {
    EVERYTHING_TAGGED
        .iter()
        .filter(move |tagged| tagged.is.contains(&tag))
        .map(|tagged| tagged.called)
}

/// One thing that answers a capability no trade stands behind, and how well.
#[derive(Debug, Clone, Copy)]
pub struct Serves {
    pub called: &'static str,
    pub capability: Capability,
    /// Where it stands between bare hands (0.0) and the best there is (1.0)
    pub how_well: f32,
}

/// The four capabilities that no trade answers, declared.
///
/// Carrying a load, holding water, roofing a frame and making a hole are not
/// skills anybody in this model has, so there is no tool table to read and
/// these are stated outright. Each family is normalised so that the best
/// thing in it reads 1.0, the same as the derived four.
pub const EVERYTHING_THAT_SERVES: &[Serves] = &[
    // ---- carrying --------------------------------------------------------
    //
    // A cart against a bowl, and the ladder between them is most of what the
    // transport chain is for.
    Serves { called: "handcart", capability: Capability::CarryingContainer, how_well: 1.0 },
    Serves { called: "travois", capability: Capability::CarryingContainer, how_well: 0.7 },
    Serves { called: "basket", capability: Capability::CarryingContainer, how_well: 0.5 },
    Serves { called: "leatherbag", capability: Capability::CarryingContainer, how_well: 0.45 },
    Serves { called: "claypot", capability: Capability::CarryingContainer, how_well: 0.25 },

    // ---- holding water ---------------------------------------------------
    //
    // The thing a settlement has never had - see the standing task about
    // carrying water - and the reason the want can now at least be *stated*:
    // "something that holds water" is a question with four answers here and
    // had none before, because every caller asked for a waterskin by name.
    Serves { called: "stoneware", capability: Capability::WaterContainer, how_well: 1.0 },
    Serves { called: "claypot", capability: Capability::WaterContainer, how_well: 0.9 },
    Serves { called: "leatherbag", capability: Capability::WaterContainer, how_well: 0.6 },
    Serves { called: "bowl", capability: Capability::WaterContainer, how_well: 0.4 },

    // ---- roofing ---------------------------------------------------------
    //
    // Hide over poles is the best a stone-age people can do and is the thing
    // they can actually reach; fired brick is better and is four technologies
    // away.
    Serves { called: "bricks", capability: Capability::ShelterMaterial, how_well: 1.0 },
    Serves { called: "hides", capability: Capability::ShelterMaterial, how_well: 0.9 },
    Serves { called: "leather", capability: Capability::ShelterMaterial, how_well: 0.9 },
    Serves { called: "wood", capability: Capability::ShelterMaterial, how_well: 0.8 },
    Serves { called: "stone", capability: Capability::ShelterMaterial, how_well: 0.7 },
    Serves { called: "lashing", capability: Capability::ShelterMaterial, how_well: 0.4 },

    // ---- making holes ----------------------------------------------------
    //
    // There is no awl in this world and no trade of boring holes, so this is
    // the pointed things ranked by how fine a point they carry. Declared
    // rather than derived precisely because nothing stands behind it: it is a
    // capability the world can express and has no job for yet, which is a gap
    // somebody can count.
    Serves { called: "metalknife", capability: Capability::PiercingTool, how_well: 1.0 },
    Serves { called: "metalblade", capability: Capability::PiercingTool, how_well: 0.9 },
    Serves { called: "stoneknife", capability: Capability::PiercingTool, how_well: 0.6 },
    Serves { called: "knappedtip", capability: Capability::PiercingTool, how_well: 0.5 },
    Serves { called: "flint", capability: Capability::PiercingTool, how_well: 0.25 },
];

/// What bare hands are worth at any of this.
///
/// Nought, by construction: the coefficient measures the *advantage* a thing
/// gives over no thing at all, so the thing that is no thing at all sits at
/// the bottom of the scale rather than somewhere in the middle of it.
pub const WHAT_EMPTY_HANDS_SERVE: f32 = 0.0;

/// How well this thing answers that want.
///
/// `0.0` for something that does not answer it at all, up to `1.0` for the
/// best answer this world has. For the four capabilities with a trade behind
/// them this is read off the tool table and renormalised, so it cannot say
/// something the tool table does not already say.
pub fn how_well_this_serves(called: &str, capability: Capability) -> f32 {
    match capability.the_trade_behind_it() {
        Some(trade) => {
            let best = the_best_advantage_at(trade);
            let mine = EVERY_TOOL
                .iter()
                .filter(|tool| tool.helps == trade && tool.called == called)
                .map(|tool| tool.how_much_better)
                .fold(0.0_f32, f32::max);

            where_it_stands(mine, best)
        }
        None => EVERYTHING_THAT_SERVES
            .iter()
            .find(|serves| serves.capability == capability && serves.called == called)
            .map(|serves| serves.how_well)
            .unwrap_or(WHAT_EMPTY_HANDS_SERVE),
    }
}

/// The best multiplier anything in the world brings to a trade.
fn the_best_advantage_at(trade: SkillType) -> f32 {
    EVERY_TOOL
        .iter()
        .filter(|tool: &&Tool| tool.helps == trade)
        .map(|tool| tool.how_much_better)
        .fold(1.0_f32, f32::max)
}

/// Where a multiplier stands between bare hands and the best there is.
///
/// The fraction of the *advantage*, not of the multiplier: a tool that is no
/// better than hands is worth nothing at the job however large its number
/// looks, and the scale has to say so.
fn where_it_stands(mine: f32, best: f32) -> f32 {
    if best <= 1.0 || mine <= 1.0 {
        return WHAT_EMPTY_HANDS_SERVE;
    }

    ((mine - 1.0) / (best - 1.0)).clamp(0.0, 1.0)
}

/// Everything that answers a want, best first.
///
/// The ordering is the point. A job that wants something to dig with should
/// take the best thing to hand and not the first one somebody typed into a
/// match arm.
pub fn everything_that_answers(capability: Capability) -> Vec<(&'static str, f32)> {
    let mut answers: Vec<(&'static str, f32)> = match capability.the_trade_behind_it() {
        Some(trade) => {
            let best = the_best_advantage_at(trade);
            EVERY_TOOL
                .iter()
                .filter(|tool| tool.helps == trade)
                .map(|tool| (tool.called, where_it_stands(tool.how_much_better, best)))
                .filter(|(_, how_well)| *how_well > WHAT_EMPTY_HANDS_SERVE)
                .collect()
        }
        None => EVERYTHING_THAT_SERVES
            .iter()
            .filter(|serves| serves.capability == capability)
            .map(|serves| (serves.called, serves.how_well))
            .collect(),
    };

    // Best first, and by name where two are worth the same, so the answer is
    // the same on every run. A world that picked differently from an
    // unordered table would not be repeatable - see `repeatable_tests`.
    answers.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    answers.dedup_by(|a, b| a.0 == b.0);
    answers
}

/// The best answer to a want that is actually in somebody's hands.
///
/// `holding` answers how many of a named thing they have. Returns what they
/// would use and what it is worth, or nothing if they have no answer at all.
pub fn the_best_to_hand(
    capability: Capability,
    holding: &impl Fn(&str) -> u32,
) -> Option<(&'static str, f32)> {
    everything_that_answers(capability)
        .into_iter()
        .find(|(called, _)| holding(called) > 0)
}

/// The best thing they hold that *is* one of these, by name.
///
/// The ungraded counterpart of [`the_best_to_hand`], for the tags that do not
/// rank: any pole will do to hold a tent up, and the question is only whether
/// there is one.
pub fn something_that_is(tag: Tag, holding: &impl Fn(&str) -> u32) -> Option<&'static str> {
    everything_that_is(tag).find(|called| holding(called) > 0)
}

/// One of the things a job asks for, by class rather than by name.
#[derive(Debug, Clone, Copy)]
pub struct Wanted {
    /// What sort of thing
    pub is: Tag,
    /// How much of it
    pub how_much: u32,
}

/// What it takes to put up a hide tent.
///
/// **The specification's worked example, and a live defect it uncovered.**
/// `BuildingType::SkinTent` has always declared that it wants eight wood and
/// four hides, and two separate comments in the source say so. The builder
/// resolved `ResourceType` to an item name with a `match` of three arms -
/// wood, stone, iron - and `continue`d on everything else, in the checking
/// loop *and* in the consuming loop. So the hides were neither required nor
/// taken: every tent ever raised in this model was made of poles and air.
///
/// Stated as classes, it is the specification's own list - flexible covering,
/// cordage, poles - and a settlement that has leather but no raw hides can
/// now roof with the leather, which by name it could not.
pub const WHAT_A_TENT_TAKES: &[Wanted] = &[
    Wanted { is: Tag::Pole, how_much: 8 },
    Wanted { is: Tag::FlexibleCovering, how_much: 4 },
    Wanted { is: Tag::Cordage, how_much: 2 },
];

/// What a burrow takes: a morning, and nothing else.
///
/// Declared as an empty list rather than left out, so that "this building has
/// no class-based requirement yet" and "this building genuinely wants nothing"
/// are different answers. The burrow is the way out for a people with neither
/// timber nor skins, and the entire point of it is that there is nothing to
/// be short of.
pub const WHAT_A_BURROW_TAKES: &[Wanted] = &[];

/// What it takes to put up a roof of this kind, where that is stated in
/// classes rather than in named resources.
///
/// `None` for everything still stated as `ResourceType` quantities. Those are
/// the stone-and-iron buildings a stone-age people cannot reach at all, and
/// restating them in classes would be inventing requirements rather than
/// translating them.
pub fn what_this_roof_takes(
    building: crate::world::BuildingType,
) -> Option<&'static [Wanted]> {
    match building {
        crate::world::BuildingType::SkinTent => Some(WHAT_A_TENT_TAKES),
        crate::world::BuildingType::Burrow => Some(WHAT_A_BURROW_TAKES),
        _ => None,
    }
}

/// Whether these hands can meet a list of wants, and what is short if not.
///
/// Returns the things that would be used and how many of each, or the first
/// class that cannot be met and how many more of it are wanted. The shortfall
/// is the useful half: a refusal that says "short two cordage" is the next
/// job, and one that says "cannot build" is a wasted turn.
pub fn what_would_be_used(
    wants: &[Wanted],
    holding: &impl Fn(&str) -> u32,
) -> Result<Vec<(&'static str, u32)>, (Tag, u32)> {
    let mut using: Vec<(&'static str, u32)> = Vec::new();

    for want in wants {
        let mut still_wanting = want.how_much;

        for called in everything_that_is(want.is) {
            if still_wanting == 0 {
                break;
            }

            // What is already spoken for by an earlier want in the same list
            // does not count twice. A pole and a covering are different
            // classes and could in principle be answered by one thing.
            let spoken_for: u32 = using
                .iter()
                .filter(|(already, _)| *already == called)
                .map(|(_, how_many)| *how_many)
                .sum();

            let to_hand = holding(called).saturating_sub(spoken_for);
            let taking = to_hand.min(still_wanting);

            if taking > 0 {
                using.push((called, taking));
                still_wanting -= taking;
            }
        }

        if still_wanting > 0 {
            return Err((want.is, still_wanting));
        }
    }

    Ok(using)
}

#[cfg(test)]
#[path = "tests/tag_tests.rs"]
mod tag_tests;
