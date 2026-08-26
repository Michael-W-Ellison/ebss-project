// src/environment/verbs.rs
//! Every verb a person in this world can be the subject of, and the three
//! things that define one.
//!
//! "Every action must be defined by what it targets, what it requires
//! (tool/free-hand), and what state-change it triggers."
//!
//! Before this the actions were a list of thirty-odd enum variants, each with
//! its own executor arm that resolved its own target its own way and checked
//! its own preconditions its own way or not at all. There was no answer to
//! "what does this verb want in its hands", because the question was never
//! asked in one place: `Cook` looked for a fire, `Craft` looked for a fire
//! only sometimes, `Gather` consulted a tool for a multiplier but would happily
//! proceed bare-handed, and `TendField` asked for nothing at all.
//!
//! This is that one place. A verb declares:
//!
//! - **what it targets** — the ground underfoot, a thing in the pack, a thing
//!   growing here, a person, an animal, a fire, a place. A verb with no valid
//!   target here cannot be done here, and that is one check rather than thirty.
//! - **what it wants in hand** — bare hands, a free hand, any tool that helps
//!   with a given trade, or one particular named thing. This is the half that
//!   was missing entirely: a man with no knife could butcher as well as a man
//!   with one, only slower.
//! - **what it changes** — where somebody is, what they are holding, what a
//!   thing *is*, the ground, a body, what is known, a bond, or the world's
//!   heat. Declared so that a verb which changes nothing is visible as a verb
//!   which changes nothing.
//!
//! And, honestly, **what performs it** — the `Action` that actually carries the
//! verb out, or `None` where the verb is named in the matrix and nothing in the
//! simulation does it yet. A matrix that quietly implied sixty-eight working
//! verbs would be worse than no matrix. See `EVERY_VERB` and the tests.

use crate::agents::skills::SkillType;

/// What a verb acts on.
///
/// Not the same as what it changes: `Butcher` targets an animal and changes
/// what is in the pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Targets {
    /// Nothing outside the actor. Sleeping, dodging, dismounting.
    Nobody,
    /// Somewhere else on the map
    APlace,
    /// Something already in the pack
    AThingHeld,
    /// Something lying or growing on the tile underfoot
    AThingUnderfoot,
    /// The tile itself: its terrain, its soil
    TheGroundUnderfoot,
    /// Another person
    APerson,
    /// An animal, wild or otherwise
    AnAnimal,
    /// A fire, lit or laid
    AFire,
    /// A building or other raised thing
    AStructure,
    /// Water: a river, a pool, what is in a skin
    Water,
}

/// What a verb wants in the hands doing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    /// Nothing. Hands, and they may be full.
    BareHands,
    /// A hand with nothing in it. Picking a thing up needs somewhere to put it.
    AFreeHand,
    /// Anything that helps with this trade — see
    /// [`crate::environment::making::what_helps_with`]. A verb that wants a
    /// tool cannot be done without one; how well it goes still depends on
    /// which tool and how worn.
    AToolFor(SkillType),
    /// One particular thing, by name, held and not used up
    ThisInHand(&'static str),
    /// Something that holds a liquid, with something in it.
    ///
    /// The whole fluid family wants one, and none of the fluid family could
    /// exist until somebody could hollow out a bowl - which is what the
    /// container machinery in this codebase had been waiting for since it was
    /// written. A verb that wants a vessel wants a full one: an empty bowl is
    /// a bowl and not a means.
    AVessel,
}

impl Wants {
    /// Whether these hands will do.
    ///
    /// `holding` answers how many usable ones of a named thing are in the pack;
    /// `helped_by` answers whether anything in the pack helps with a trade.
    pub fn satisfied_by(
        &self,
        holding: &impl Fn(&str) -> u32,
        helped_by: &impl Fn(SkillType) -> bool,
        a_hand_to_spare: bool,
    ) -> bool {
        self.satisfied_by_hands(holding, helped_by, a_hand_to_spare, 0.0)
    }

    /// The same, for hands that may be carrying a vessel with something in it.
    pub fn satisfied_by_hands(
        &self,
        holding: &impl Fn(&str) -> u32,
        helped_by: &impl Fn(SkillType) -> bool,
        a_hand_to_spare: bool,
        carrying_liquid: f32,
    ) -> bool {
        match self {
            Wants::BareHands => true,
            Wants::AFreeHand => a_hand_to_spare,
            Wants::AToolFor(trade) => helped_by(*trade),
            Wants::ThisInHand(what) => holding(what) > 0,
            Wants::AVessel => carrying_liquid > 0.0,
        }
    }
}

/// What a verb alters when it is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changes {
    /// Where somebody is
    Where,
    /// What is in a pack, and whose
    WhatIsHeld,
    /// What a thing is: a stone becomes a flake, a hide becomes a thong
    WhatAThingIs,
    /// The terrain or the soil of a tile
    TheGround,
    /// A body: its health, what it has eaten, how tired it is
    ABody,
    /// What somebody knows or believes
    WhatIsKnown,
    /// What two people are to each other
    ABond,
    /// Fire and heat in the world
    TheWorldsHeat,
    /// Nothing at all. Declared, and worth being able to see.
    Nothing,
}

/// Which of the twelve families a verb belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Locomotion,
    Manipulation,
    Disruption,
    Thermal,
    Fluid,
    Assembly,
    Subterranean,
    Survival,
    Combat,
    Exchange,
    Equipment,
    Sensory,
}

/// One verb, fully declared.
#[derive(Debug, Clone, Copy)]
pub struct Verb {
    /// What it is called
    pub called: &'static str,
    /// Which family it belongs to
    pub family: Family,
    /// What it acts on
    pub targets: Targets,
    /// What it wants in the hands
    pub wants: Wants,
    /// What it alters
    pub changes: &'static [Changes],
    /// The `Action` that performs it, named as
    /// [`crate::agents::Agent::what_was_tried`] names it — or `None` where the
    /// verb is declared and nothing does it yet.
    pub done_by: Option<&'static str>,
    /// Or, for a verb nobody chooses, what brings it about.
    ///
    /// Some verbs are not decisions. Nobody decides to get a spear between
    /// himself and a wolf; it is what happens when the wolf arrives and there
    /// is a spear in his hand. Those are carried out by the world rather than
    /// by an `Action`, and this says what occasions them.
    pub happens_when: Option<&'static str>,
    /// Whether the action that performs it does so every time.
    ///
    /// `Craft` is whichever of heating, lashing and attaching the step in hand
    /// calls for, so none of those three is a thing every craft does, and none
    /// of them can be demanded of every craft. A hunt is always a going after
    /// something with a spear, and only sometimes a butchering.
    ///
    /// This is what decides whether a verb's `wants` is enforced against the
    /// action before it runs — see
    /// [`crate::environment::verbs::what_this_action_cannot_do_without`].
    pub always: bool,
}

impl Verb {
    /// Whether anything in the simulation actually performs this verb,
    /// whether by somebody's choosing or by the world's.
    pub fn is_live(&self) -> bool {
        self.done_by.is_some() || self.happens_when.is_some()
    }

    /// Whether it is a thing somebody decides to do, as against a thing that
    /// happens to them.
    pub fn is_chosen(&self) -> bool {
        self.done_by.is_some()
    }

    /// Whether this verb wants something in the hands to be done at all
    pub fn wants_something_in_hand(&self) -> bool {
        !matches!(self.wants, Wants::BareHands)
    }
}

/// A shorthand for the common case: a verb that wants nothing and changes one
/// thing.
const fn verb(
    called: &'static str,
    family: Family,
    targets: Targets,
    wants: Wants,
    changes: &'static [Changes],
    done_by: Option<&'static str>,
) -> Verb {
    Verb {
        called,
        family,
        targets,
        wants,
        changes,
        done_by,
        happens_when: None,
        always: true,
    }
}

/// The same, for a verb nobody chooses: it happens when the thing named
/// happens, and what it wants in the hand decides whether it happens at all.
const fn happens_when(
    called: &'static str,
    family: Family,
    targets: Targets,
    wants: Wants,
    changes: &'static [Changes],
    occasion: &'static str,
) -> Verb {
    Verb {
        happens_when: Some(occasion),
        ..verb(called, family, targets, wants, changes, None)
    }
}

/// The same, for a verb an action carries out only when the job in hand calls
/// for it
const fn sometimes(
    called: &'static str,
    family: Family,
    targets: Targets,
    wants: Wants,
    changes: &'static [Changes],
    done_by: Option<&'static str>,
) -> Verb {
    Verb {
        always: false,
        ..verb(called, family, targets, wants, changes, done_by)
    }
}

// ---------------------------------------------------------------------------
// 1. Spatial and locomotive
// ---------------------------------------------------------------------------

pub const MOVE_TO: Verb = verb(
    "move to",
    Family::Locomotion,
    Targets::APlace,
    Wants::BareHands,
    &[Changes::Where],
    Some("move"),
);

/// Closing on a particular person or animal rather than a particular tile.
/// The difference matters because the target moves.
pub const APPROACH: Verb = verb(
    "approach",
    Family::Locomotion,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::Where],
    None,
);

/// Putting distance between yourself and a thing, which is not the same as
/// going somewhere: what decides the direction is what you are running from.
pub const FLEE_FROM: Verb = verb(
    "flee from",
    Family::Locomotion,
    Targets::AnAnimal,
    Wants::BareHands,
    &[Changes::Where],
    Some("fleefrom"),
);

pub const ENTER: Verb = verb(
    "enter",
    Family::Locomotion,
    Targets::AStructure,
    Wants::BareHands,
    &[Changes::Where],
    Some("seekshelter"),
);

pub const EXIT: Verb = verb(
    "exit",
    Family::Locomotion,
    Targets::AStructure,
    Wants::BareHands,
    &[Changes::Where],
    None,
);

// ---------------------------------------------------------------------------
// 2. Object manipulation
// ---------------------------------------------------------------------------

pub const PICK_UP: Verb = verb(
    "pick up",
    Family::Manipulation,
    Targets::AThingUnderfoot,
    Wants::AFreeHand,
    &[Changes::WhatIsHeld],
    Some("pickup"),
);

pub const PLACE_DOWN: Verb = verb(
    "place down",
    Family::Manipulation,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatIsHeld],
    Some("putdown"),
);

/// Not an act so much as a state: what carrying costs is paid every tick.
pub const CARRY: Verb = happens_when(
    "carry",
    Family::Manipulation,
    Targets::AThingHeld,
    Wants::AFreeHand,
    &[Changes::ABody],
    "a loaded pack is paid for with every step taken under it",
);

pub const DROP: Verb = verb(
    "drop",
    Family::Manipulation,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatIsHeld, Changes::TheGround],
    Some("spreadmuck"),
);

/// The state a thing is in between being taken up and being put away. Not
/// something anybody decides once `equip` exists - it is what `equip` leaves
/// behind, and what makes the tool worth more than the same tool in the bag.
pub const HOLD: Verb = happens_when(
    "hold",
    Family::Manipulation,
    Targets::AThingHeld,
    Wants::AFreeHand,
    &[Changes::WhatIsHeld],
    "a thing taken up stays in the hand until it is put away",
);

pub const RELEASE: Verb = verb(
    "release",
    Family::Manipulation,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatIsHeld],
    Some("putdown"),
);

// ---------------------------------------------------------------------------
// 3. Mechanical disruption
//
// What a tool is for. Every one of these wants something in the hand, because
// that is the whole point of the family: a man with no edge cannot cut.
// ---------------------------------------------------------------------------

pub const SMASH: Verb = verb(
    "smash",
    Family::Disruption,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Mining),
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("smash"),
);

pub const CRUSH: Verb = verb(
    "crush",
    Family::Disruption,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Mining),
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("crush"),
);

pub const CUT: Verb = verb(
    "cut",
    Family::Disruption,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Leatherworking),
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("cut"),
);

pub const SCRAPE: Verb = verb(
    "scrape",
    Family::Disruption,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Leatherworking),
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("scrape"),
);

pub const PIERCE: Verb = verb(
    "pierce",
    Family::Disruption,
    Targets::AnAnimal,
    Wants::ThisInHand("spear"),
    &[Changes::ABody],
    None,
);

pub const DRILL: Verb = verb(
    "drill",
    Family::Disruption,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Crafting),
    &[Changes::WhatAThingIs],
    None,
);

pub const SPLIT: Verb = verb(
    "split",
    Family::Disruption,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Woodcutting),
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    None,
);

// ---------------------------------------------------------------------------
// 4. Thermal and energy
// ---------------------------------------------------------------------------

pub const HEAT: Verb = sometimes(
    "heat",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("craft"),
);

/// Laying food out in the air, or hanging it over a fire.
///
/// Not one of the specification's sixty-eight - it was added when food was
/// first put on a clock it could actually rot against. `PreparationState`
/// has carried Dried, Smoked, Salted, Pickled and Fermented since it was
/// written, with a spoilage multiplier for each, and nothing had ever set any
/// of them: there was no reason to preserve anything in a world where meat
/// took a year and a quarter to turn.
pub const DRY: Verb = verb(
    "dry",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("dry"),
);

/// Rubbing salt into food, which is the third way of keeping a thing and the
/// only one that needs neither a week of sun nor a fire kept going.
pub const SALT: Verb = verb(
    "salt",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("salt"),
);

/// Holding a thing in a fire until it stops being what it was.
///
/// Distinct from `heat`, which is what a `Making` does over a fire and is
/// carried out by `Craft`. This is the other thing a fire does: not warming a
/// thing up on the way to making something else out of it, but changing what
/// the thing *is*. Clay goes in and pottery comes out, and there is no going
/// back.
pub const FIRE: Verb = verb(
    "fire",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("fire"),
);

pub const COOL: Verb = verb(
    "cool",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    None,
);

pub const QUENCH: Verb = verb(
    "quench",
    Family::Thermal,
    Targets::Water,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    None,
);

pub const IGNITE: Verb = verb(
    "ignite",
    Family::Thermal,
    Targets::TheGroundUnderfoot,
    Wants::ThisInHand("wood"),
    &[Changes::TheWorldsHeat],
    Some("lightfire"),
);

pub const MELT: Verb = sometimes(
    "melt",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("craft"),
);

pub const ROAST: Verb = verb(
    "roast",
    Family::Thermal,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("cook"),
);

// ---------------------------------------------------------------------------
// 5. Chemical and fluid
// ---------------------------------------------------------------------------

pub const MIX: Verb = verb(
    "mix",
    Family::Fluid,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    None,
);

pub const POUR: Verb = verb(
    "pour",
    Family::Fluid,
    Targets::TheGroundUnderfoot,
    Wants::ThisInHand("waterskin"),
    &[Changes::TheGround, Changes::WhatIsHeld],
    None,
);

pub const SOAK: Verb = verb(
    "soak",
    Family::Fluid,
    Targets::Water,
    Wants::AVessel,
    &[Changes::WhatAThingIs],
    Some("soak"),
);

pub const COAT: Verb = verb(
    "coat",
    Family::Fluid,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    None,
);

pub const BOIL: Verb = verb(
    "boil",
    Family::Fluid,
    Targets::AFire,
    Wants::AVessel,
    &[Changes::WhatAThingIs],
    Some("boil"),
);

pub const LEACH: Verb = verb(
    "leach",
    Family::Fluid,
    Targets::Water,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    None,
);

pub const FERMENT: Verb = verb(
    "ferment",
    Family::Fluid,
    Targets::AThingHeld,
    Wants::AVessel,
    &[Changes::WhatAThingIs],
    Some("ferment"),
);

// ---------------------------------------------------------------------------
// 6. Assembly and construction
// ---------------------------------------------------------------------------

pub const LASH: Verb = sometimes(
    "lash",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::ThisInHand("lashing"),
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("craft"),
);

pub const WEAVE: Verb = verb(
    "weave",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("weave"),
);

pub const CARVE: Verb = verb(
    "carve",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::AToolFor(SkillType::Crafting),
    &[Changes::WhatAThingIs],
    Some("carve"),
);

/// Pressing a soft thing into a shape it keeps.
///
/// Live at last. Clay is the only material in this world that will do it, and
/// it is where every fired thing starts.
pub const MOLD: Verb = verb(
    "mold",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    Some("mold"),
);

pub const FOLD: Verb = verb(
    "fold",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    None,
);

pub const STACK: Verb = verb(
    "stack",
    Family::Assembly,
    Targets::TheGroundUnderfoot,
    Wants::AFreeHand,
    &[Changes::TheGround, Changes::WhatIsHeld],
    None,
);

pub const FRAME: Verb = verb(
    "frame",
    Family::Assembly,
    Targets::TheGroundUnderfoot,
    Wants::ThisInHand("wood"),
    &[Changes::TheGround],
    Some("build"),
);

pub const ATTACH: Verb = sometimes(
    "attach",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("craft"),
);

/// Stitching wants a hand free, not an edge.
///
/// The first cut of this wanted a leatherworking tool, which is true of
/// sewing and was measured to be false of this economy. Over eight worlds it
/// took the agents who ended up dressed from 47 to 23 and drove the number of
/// clothing attempts from 774 to 5,694, almost all of them refusals: stone
/// knives wear through faster than a people replaces them, so a requirement
/// for an edge is a requirement most people cannot meet most of the time, and
/// what it bought was a settlement in its shirtsleeves.
///
/// A hand free is a real requirement and an enforced one - a man with an axe
/// in one hand and a spear in the other is not stitching anything - and it is
/// the one this economy can actually carry. What a knife is worth to the work
/// is still what it always was: how well the garment comes out.
pub const SEW: Verb = verb(
    "sew",
    Family::Assembly,
    Targets::AThingHeld,
    Wants::AFreeHand,
    &[Changes::WhatAThingIs, Changes::WhatIsHeld],
    Some("makeclothing"),
);

// ---------------------------------------------------------------------------
// 7. Subterranean
// ---------------------------------------------------------------------------

pub const DIG: Verb = verb(
    "dig",
    Family::Subterranean,
    Targets::TheGroundUnderfoot,
    Wants::BareHands,
    &[Changes::TheGround],
    Some("tillsoil"),
);

/// Digging yourself into the ground, because there is nothing to build with.
///
/// Live at last. `build` is framing - it wants poles in the hand, which is
/// right for a tent and quite wrong for a hole - so a burrow is its own verb
/// rather than a kind of building. What it wants is something to dig with.
pub const BURROW: Verb = verb(
    "burrow",
    Family::Subterranean,
    Targets::TheGroundUnderfoot,
    Wants::AToolFor(SkillType::Mining),
    &[Changes::TheGround, Changes::Where],
    Some("burrow"),
);

/// Digging a hole in the ground to keep things in, and what comes out of it.
pub const EXCAVATE: Verb = verb(
    "excavate",
    Family::Subterranean,
    Targets::TheGroundUnderfoot,
    Wants::AToolFor(SkillType::Mining),
    &[Changes::TheGround, Changes::WhatIsHeld],
    Some("excavate"),
);

/// Putting the earth back over what you have just put in the hole. Cold
/// ground with a lid of soil on it is what a people this far along has instead
/// of a cellar, and it keeps food four times as long as a pack does.
pub const COVER: Verb = verb(
    "cover",
    Family::Subterranean,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::TheGround, Changes::WhatIsHeld],
    Some("cover"),
);

// ---------------------------------------------------------------------------
// 8. Survival and biology
// ---------------------------------------------------------------------------

pub const HARVEST: Verb = verb(
    "harvest",
    Family::Survival,
    Targets::AThingUnderfoot,
    Wants::BareHands,
    &[Changes::WhatIsHeld],
    Some("gather"),
);

pub const HUNT: Verb = verb(
    "hunt",
    Family::Survival,
    Targets::AnAnimal,
    Wants::ThisInHand("spear"),
    &[Changes::ABody, Changes::WhatIsHeld],
    Some("hunt"),
);

pub const BUTCHER: Verb = sometimes(
    "butcher",
    Family::Survival,
    Targets::AnAnimal,
    Wants::AToolFor(SkillType::Leatherworking),
    &[Changes::WhatIsHeld, Changes::WhatAThingIs],
    Some("hunt"),
);

pub const EAT: Verb = verb(
    "eat",
    Family::Survival,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::ABody, Changes::WhatIsHeld],
    Some("eat"),
);

pub const DRINK: Verb = verb(
    "drink",
    Family::Survival,
    Targets::Water,
    Wants::BareHands,
    &[Changes::ABody],
    Some("gather"),
);

pub const TASTE: Verb = verb(
    "taste",
    Family::Survival,
    Targets::AThingUnderfoot,
    Wants::BareHands,
    &[Changes::ABody, Changes::WhatIsKnown],
    Some("taste"),
);

// ---------------------------------------------------------------------------
// 9. Combat and defence
// ---------------------------------------------------------------------------

pub const ATTACK_WITH: Verb = verb(
    "attack with",
    Family::Combat,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::ABody, Changes::ABond],
    Some("attack"),
);

/// Nobody decides to do this. It is what happens when something comes at you
/// and there is a shaft in your hand, and it is why carrying a spear is worth
/// something even to a man who never hunts.
pub const DEFEND_WITH: Verb = happens_when(
    "defend with",
    Family::Combat,
    Targets::Nobody,
    Wants::AToolFor(SkillType::MeleeCombat),
    &[Changes::ABody, Changes::WhatAThingIs],
    "something comes at you",
);

pub const THROW: Verb = verb(
    "throw",
    Family::Combat,
    Targets::AnAnimal,
    Wants::ThisInHand("spear"),
    &[Changes::ABody, Changes::WhatIsHeld],
    Some("hunt"),
);

pub const AIM: Verb = verb(
    "aim",
    Family::Combat,
    Targets::AnAnimal,
    Wants::AToolFor(SkillType::Archery),
    &[Changes::Nothing],
    None,
);

/// Nobody decides to dodge either. It is what a body does when something
/// comes at it, and how much of it a body manages is what standing your ground
/// has taught it - see `Agent::what_a_blow_costs_me`.
pub const DODGE: Verb = happens_when(
    "dodge",
    Family::Combat,
    Targets::Nobody,
    Wants::BareHands,
    &[Changes::ABody],
    "something comes at you",
);

/// The third answer to a thing that would kill you, and the one nobody
/// arrives at on purpose. It is what is left when a body can neither run nor
/// raise a hand: see `Simulation::how_this_one_answers_a_threat`.
///
/// Not in the specification's twelve families - it was added when the
/// fight-or-flight decision was given the rest of its tree, because a
/// decision with two branches and no answer for "neither" is not a decision.
pub const FREEZE: Verb = verb(
    "freeze",
    Family::Combat,
    Targets::Nobody,
    Wants::BareHands,
    &[Changes::Nothing],
    Some("freeze"),
);

pub const PARRY: Verb = verb(
    "parry",
    Family::Combat,
    Targets::Nobody,
    Wants::AToolFor(SkillType::MeleeCombat),
    &[Changes::ABody],
    None,
);

// ---------------------------------------------------------------------------
// 10. Social and exchange
// ---------------------------------------------------------------------------

pub const GIVE_TO: Verb = verb(
    "give to",
    Family::Exchange,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::WhatIsHeld, Changes::ABond],
    Some("giveto"),
);

pub const TAKE_FROM: Verb = verb(
    "take from",
    Family::Exchange,
    Targets::APerson,
    Wants::AFreeHand,
    &[Changes::WhatIsHeld, Changes::ABond],
    Some("takefrom"),
);

pub const TRADE: Verb = verb(
    "trade",
    Family::Exchange,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::WhatIsHeld, Changes::ABond],
    Some("trade"),
);

pub const SHARE: Verb = verb(
    "share",
    Family::Exchange,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::WhatIsHeld, Changes::ABond],
    Some("store"),
);

pub const COMMUNICATE: Verb = verb(
    "communicate",
    Family::Exchange,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::WhatIsKnown, Changes::ABond],
    Some("shareinformation"),
);

/// Asking somebody how a thing of theirs came about.
///
/// The other half of `communicate`, and the half that was missing. Sharing is
/// somebody deciding to say something; this is somebody deciding to ask, which
/// is how a discovery gets out of the head that made it and into the head that
/// needed it. A settlement of forty could work the same thing out forty times
/// over and be no further on than the first man who worked it out.
pub const ASK_ABOUT: Verb = verb(
    "ask about",
    Family::Exchange,
    Targets::APerson,
    Wants::BareHands,
    &[Changes::WhatIsKnown],
    Some("ask"),
);

// ---------------------------------------------------------------------------
// 11. Equipment and utilities
// ---------------------------------------------------------------------------

pub const WEAR: Verb = verb(
    "wear",
    Family::Equipment,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::ABody, Changes::WhatIsHeld],
    Some("wearclothing"),
);

pub const EQUIP: Verb = verb(
    "equip",
    Family::Equipment,
    Targets::AThingHeld,
    Wants::AFreeHand,
    &[Changes::WhatIsHeld],
    Some("equip"),
);

pub const UNEQUIP: Verb = verb(
    "unequip",
    Family::Equipment,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatIsHeld],
    Some("unequip"),
);

/// A tool put to its purpose. Nobody chooses this: it is what happens to the
/// axe when the tree comes down, and it is why an axe is a finite thing.
pub const USE: Verb = happens_when(
    "use",
    Family::Equipment,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatAThingIs],
    "a tool does a piece of work and is the worse for it",
);

// ---------------------------------------------------------------------------
// 12. Sensory and experimentation
// ---------------------------------------------------------------------------

pub const EXAMINE: Verb = verb(
    "examine",
    Family::Sensory,
    Targets::AThingHeld,
    Wants::BareHands,
    &[Changes::WhatIsKnown],
    Some("examine"),
);

/// Nobody decides to smell something. What is rotting, what is cooking and
/// what is standing in the next field give themselves away, and a nose that
/// is there picks them up - see `Simulation::emit_scents`.
pub const SMELL: Verb = happens_when(
    "smell",
    Family::Sensory,
    Targets::AThingUnderfoot,
    Wants::BareHands,
    &[Changes::WhatIsKnown],
    "something nearby gives itself away",
);

/// And nobody decides to overhear. Anything said within earshot is heard by
/// whoever is standing there, which is how a lie gets found out and how
/// anybody learns where the water is without walking to it.
pub const LISTEN: Verb = happens_when(
    "listen",
    Family::Sensory,
    Targets::Nobody,
    Wants::BareHands,
    &[Changes::WhatIsKnown],
    "somebody says something in earshot",
);

/// The whole matrix, in the order of the twelve families.
pub const EVERY_VERB: &[Verb] = &[
    // 1
    MOVE_TO, APPROACH, FLEE_FROM, ENTER, EXIT,
    // 2
    PICK_UP, PLACE_DOWN, CARRY, DROP, HOLD, RELEASE,
    // 3
    SMASH, CRUSH, CUT, SCRAPE, PIERCE, DRILL, SPLIT,
    // 4
    HEAT, DRY, SALT, FIRE, COOL, QUENCH, IGNITE, MELT, ROAST,
    // 5
    MIX, POUR, SOAK, COAT, BOIL, LEACH, FERMENT,
    // 6
    LASH, WEAVE, CARVE, MOLD, FOLD, STACK, FRAME, ATTACH, SEW,
    // 7
    DIG, BURROW, EXCAVATE, COVER,
    // 8
    HARVEST, HUNT, BUTCHER, EAT, DRINK, TASTE,
    // 9
    ATTACK_WITH, DEFEND_WITH, THROW, AIM, DODGE, PARRY, FREEZE,
    // 10
    GIVE_TO, TAKE_FROM, TRADE, SHARE, COMMUNICATE, ASK_ABOUT,
    // 11
    WEAR, EQUIP, UNEQUIP, USE,
    // 12
    EXAMINE, SMELL, LISTEN,
];

/// The verb of that name, if there is one.
pub fn what_that_verb_is(called: &str) -> Option<&'static Verb> {
    EVERY_VERB.iter().find(|verb| verb.called == called)
}

/// Every verb something in the simulation actually performs.
pub fn everything_anybody_can_do() -> impl Iterator<Item = &'static Verb> {
    EVERY_VERB.iter().filter(|verb| verb.is_live())
}

/// Every verb declared and not yet carried out by anything.
pub fn everything_still_to_build() -> impl Iterator<Item = &'static Verb> {
    EVERY_VERB.iter().filter(|verb| !verb.is_live())
}

/// The verbs an action carries out.
///
/// One action can be several verbs — `Hunt` is both a piercing and a
/// butchering, `Craft` is whichever of heating, lashing and attaching the step
/// in question calls for — which is why this returns a list rather than one.
pub fn what_that_action_does(named: &str) -> Vec<&'static Verb> {
    EVERY_VERB
        .iter()
        .filter(|verb| verb.done_by == Some(named))
        .collect()
}

/// What the matrix says this action cannot be done at all without.
///
/// Every verb the action always performs has to have its `wants` met. An
/// action whose verbs are alternatives — `Craft`, which is heating or lashing
/// or attaching depending on the step — is held to none of them here, because
/// which one applies is a question the step answers and not this.
///
/// This is the whole of what makes the matrix a mechanism rather than a
/// document: the requirement is declared in one place and enforced in one
/// place, and adding a verb to the table is what makes it enforced.
pub fn what_this_action_cannot_do_without(named: &str) -> Vec<Wants> {
    let mut wanted: Vec<Wants> = Vec::new();

    for verb in EVERY_VERB
        .iter()
        .filter(|verb| verb.done_by == Some(named) && verb.always)
    {
        if matches!(verb.wants, Wants::BareHands) {
            continue;
        }

        // Two verbs of one action can want the same thing - a hunt is a
        // throwing and a hunting and both want the spear - and asking for it
        // twice is asking for it once.
        if !wanted.contains(&verb.wants) {
            wanted.push(verb.wants);
        }
    }

    wanted
}

/// How many hands a person has to work with.
///
/// Two, and this is where that number lives rather than being assumed in
/// several places at once.
pub const A_PAIR_OF_HANDS: u32 = 2;
