// src/environment/making.rs
//! The steps a stone-age people can put together, and what comes of them.
//!
//! "There is the world, there are objects in the world, there are actions
//! which can be taken, and there are outcomes of those actions."
//!
//! The crafting table this replaces for these purposes took its inputs as
//! `ResourceType` - things dug or picked out of the ground - and its outputs as
//! `ItemType`. That is a table of one-step recipes and it cannot express a
//! chain: there is no way to say that the thing you made last is what you need
//! for the thing you are making now. Everything a people can do had therefore
//! to be reachable in a single move from raw material, which is why the whole
//! of their toolmaking was three logs into an axe.
//!
//! A step here takes named things and makes a named thing, so what one step
//! produces the next can consume:
//!
//! ```text
//! fibrous plant                   -> lashing
//! flint + a stone to knap with    -> a knapped tip
//! stick + tip + lashing           -> spear
//! stick + tip + lashing           -> hand axe
//! hides + sticks + lashing        -> tent
//! ```
//!
//! Nothing here is a discovery yet - see `Making::obvious` - and nothing here
//! is done well. A founder knows how to lash a stone to a stick and is bad at
//! it, which is what a stone-age start means.

use crate::agents::skills::SkillType;

/// One thing a person can do with what they are holding.
#[derive(Debug, Clone, Copy)]
pub struct Making {
    /// What the thing made is called, which is also how it is asked for
    pub makes: &'static str,
    /// How many come of one doing
    pub how_many: u32,
    /// What it takes, by name and count
    pub needs: &'static [(&'static str, u32)],
    /// The hand it wants
    pub hands: SkillType,
    /// What it costs to do once, in energy
    pub effort: f32,
    /// Whether a people arriving here already know it.
    ///
    /// The things a stone-age people bring with them: cordage, knapping, a
    /// hafted tool, a tent. Everything else is a thing to find out.
    pub obvious: bool,
    /// Whether it wants a fire burning where the work is done.
    ///
    /// This is the condition half of "rock + fire = ?": a man holding a bright
    /// stone learns nothing from it until he is sitting at a fire, and then he
    /// learns it all at once.
    pub over_a_fire: bool,
    /// A tool that must be in hand to do it, and is not used up by it.
    ///
    /// A hammerstone is not part of the blade; it is what the blade is beaten
    /// out with, and it wears like anything else that gets used.
    pub wants_in_hand: Option<&'static str>,
}

impl Making {
    /// Whether these makings are all to hand.
    pub fn makings_to_hand(&self, holding: impl Fn(&str) -> u32) -> bool {
        self.needs
            .iter()
            .all(|(what, how_many)| holding(what) >= *how_many)
    }

    /// What is missing, and how much of it, if anything.
    pub fn short_of(&self, holding: impl Fn(&str) -> u32) -> Option<(&'static str, u32)> {
        self.needs
            .iter()
            .filter_map(|(what, how_many)| {
                let have = holding(what);
                if have >= *how_many {
                    None
                } else {
                    Some((*what, how_many - have))
                }
            })
            .max_by_key(|(_, missing)| *missing)
    }
}

/// Cordage. Everything else in the list is held together with it.
pub const LASHING: Making = Making {
    makes: "lashing",
    how_many: 2,
    needs: &[("flax", 2)],
    hands: SkillType::Crafting,
    effort: 4.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// And from flax that has been left in water until the stem let go of it.
///
/// Three times the cordage out of the same field, which is what retting is
/// for - see `SOAK_FLAX`.
pub const LASHING_FROM_RETTED: Making = Making {
    makes: "lashing",
    how_many: 3,
    needs: &[("rettedflax", 1)],
    hands: SkillType::Crafting,
    effort: 3.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// The same, from the other fibrous thing that grows here.
pub const LASHING_FROM_COTTON: Making = Making {
    makes: "lashing",
    how_many: 2,
    needs: &[("cotton", 2)],
    hands: SkillType::Crafting,
    effort: 4.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// A flake struck off a core: the edge that every stone tool is built round.
pub const KNAPPED_TIP: Making = Making {
    makes: "knappedtip",
    how_many: 1,
    needs: &[("stone", 2)],
    hands: SkillType::Crafting,
    effort: 6.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// The same, from a flake already struck off a core.
///
/// Half the stone for the same edge: this is what smashing a core buys.
pub const KNAPPED_TIP_FROM_FLINT: Making = Making {
    makes: "knappedtip",
    how_many: 1,
    needs: &[("flint", 1)],
    hands: SkillType::Crafting,
    effort: 4.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// Stick, tip, lashing.
pub const SPEAR: Making = Making {
    makes: "spear",
    how_many: 1,
    needs: &[("wood", 1), ("knappedtip", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 8.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// The same three things, put together the other way.
pub const HAND_AXE: Making = Making {
    makes: "handaxe",
    how_many: 1,
    needs: &[("wood", 1), ("knappedtip", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 8.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// A knife wants no handle to speak of.
pub const STONE_KNIFE: Making = Making {
    makes: "stoneknife",
    how_many: 1,
    needs: &[("knappedtip", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 5.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// A bright stone, held in a fire long enough, stops being a stone.
///
/// The specification's "rock + fire = ?": nothing an agent can work out by
/// looking at it, and obvious the moment the conditions are right. Iron is
/// what the ground here offers that glitters, and it is picked up because it
/// glitters, not because anybody yet knows what it is for.
pub const SHINY_LUMP: Making = Making {
    makes: "shinylump",
    how_many: 1,
    needs: &[("iron", 2)],
    hands: SkillType::Crafting,
    effort: 10.0,
    obvious: false,
    over_a_fire: true,
    wants_in_hand: None,
};

/// Shiny lump + hammer = a crude blade.
pub const METAL_BLADE: Making = Making {
    makes: "metalblade",
    how_many: 1,
    needs: &[("shinylump", 1)],
    hands: SkillType::Crafting,
    effort: 12.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: Some("handaxe"),
};

/// And a blade wants a handle, like every other blade these people make.
pub const METAL_KNIFE: Making = Making {
    makes: "metalknife",
    how_many: 1,
    needs: &[("metalblade", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 6.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: None,
};

/// Working a thing until it is a different thing.
///
/// The other half of what a tool is for. A `Making` puts several things
/// together; a `Working` takes one thing and reduces it — a core is smashed
/// into flakes, a hide is cut into leather, a stick is scraped into shavings.
/// The difference matters because the verb is different and because it wants
/// a different sort of thing in the hand: you assemble with your fingers and
/// you reduce with an edge.
///
/// What each of these wants in the hand is not written here. It is written in
/// the verb matrix — see [`crate::environment::verbs`] — and enforced there,
/// which is the point of having a matrix: a working declares what it turns
/// into what, and the verb it is done with declares what that verb needs.
#[derive(Debug, Clone, Copy)]
pub struct Working {
    /// The verb it is done with, as the matrix names it
    pub verb: &'static str,
    /// What it is done to
    pub to: &'static str,
    /// How much of that goes in
    pub how_much: u32,
    /// What comes out
    pub makes: &'static str,
    /// And how much of it
    pub how_many: u32,
    /// The hand it wants
    pub hands: SkillType,
    /// What it costs to do once
    pub effort: f32,
    /// Whether a people arriving here already know it
    pub obvious: bool,
    /// How much liquid what comes out will hold, if it holds any.
    ///
    /// A carved bowl is not a lump of wood with a name; it is a thing you can
    /// put water in and walk away from the river with. The container machinery
    /// was written long ago and nothing in the world ever made one.
    pub holds: Option<f32>,
    /// What comes out, as the food tables know it, if it is food.
    ///
    /// Ground grain is not grain. A third more of what is in it comes out in
    /// the eating, and it keeps rather less well, which is the whole reason to
    /// grind it when you mean to eat it rather than when you harvest it.
    pub feeds: Option<crate::world::ItemType>,
    /// How much liquid out of a carried vessel it takes.
    ///
    /// The whole fluid family wants water, and water has to be carried to
    /// where the work is - which is why nothing in this family could be built
    /// until somebody could hollow out a bowl.
    pub wants_water: f32,
    /// Whether it wants a fire burning where the work is done
    pub over_a_fire: bool,
}

/// A core broken down into flakes. Half the work of a stone tool, and the
/// half a people brings with it.
pub const SMASH_A_CORE: Working = Working {
    verb: "smash",
    to: "stone",
    how_much: 2,
    makes: "flint",
    how_many: 3,
    hands: SkillType::Mining,
    effort: 5.0,
    obvious: true,
    holds: None,
    feeds: None,
    wants_water: 0.0,
    over_a_fire: false,
};

/// A hide cut down into workable leather.
pub const CUT_A_HIDE: Working = Working {
    verb: "cut",
    to: "hides",
    how_much: 1,
    makes: "leather",
    how_many: 2,
    hands: SkillType::Leatherworking,
    effort: 6.0,
    obvious: true,
    holds: None,
    feeds: None,
    wants_water: 0.0,
    over_a_fire: false,
};

/// Shavings off a stick, which catch where a log will not.
///
/// Not obvious. Everybody knows a fire wants wood; that a fire wants wood cut
/// small enough to catch is a thing somebody works out with a scraper in his
/// hand and a fire that will not light.
pub const SCRAPE_A_STICK: Working = Working {
    verb: "scrape",
    to: "wood",
    how_much: 1,
    makes: "tinder",
    how_many: 3,
    hands: SkillType::Leatherworking,
    effort: 3.0,
    obvious: false,
    holds: None,
    feeds: None,
    wants_water: 0.0,
    over_a_fire: false,
};

/// Grain between two stones. A third more of what is in it comes out in the
/// eating, and it keeps rather less well.
///
/// Not obvious. That a seed can be opened rather than swallowed whole is a
/// thing somebody works out with a hammerstone in his hand.
pub const CRUSH_GRAIN: Working = Working {
    verb: "crush",
    to: "grain",
    how_much: 3,
    makes: "flour",
    how_many: 3,
    hands: SkillType::Mining,
    effort: 7.0,
    obvious: false,
    holds: None,
    feeds: Some(crate::world::ItemType::Flour),
    wants_water: 0.0,
    over_a_fire: false,
};

/// A carcass taken apart into joints.
///
/// Everybody is born knowing this. There is nothing to discover about a
/// carcass coming apart - it is what a knife and an afternoon are for - and
/// before this existed an agent ate a two-kilo lump of raw beast in one bite
/// straight off the kill, because nothing in the model stood between the
/// animal and the mouth.
///
/// A joint is what one person cooks and eats now. What it is *not* is a thing
/// worth keeping, which is what the strips below are for.
pub const CUT_MEAT_INTO_PORTIONS: Working = Working {
    verb: "cut",
    to: "meat",
    how_much: 1,
    makes: "meatportions",
    how_many: 3,
    hands: SkillType::Leatherworking,
    effort: 3.0,
    obvious: true,
    holds: None,
    feeds: Some(crate::world::ItemType::Meat),
    wants_water: 0.0,
    over_a_fire: false,
};

/// And a fish, which comes apart more easily and gives less.
pub const CUT_FISH_INTO_PORTIONS: Working = Working {
    verb: "cut",
    to: "fish",
    how_much: 1,
    makes: "fishportions",
    how_many: 2,
    hands: SkillType::Leatherworking,
    effort: 2.0,
    obvious: true,
    holds: None,
    feeds: Some(crate::world::ItemType::Fish),
    wants_water: 0.0,
    over_a_fire: false,
};

/// A fish opened out and cut down into strips.
///
/// Obvious - anybody with an edge and a fish works out that a fish comes
/// apart - and worth nothing on its own. What it is worth is what the sun
/// does to it afterwards: a whole fish left out in the sun goes off, and
/// strips of the same fish dry. Nobody here knows that until they have seen
/// it happen, which is the point.
pub const CUT_FISH_INTO_STRIPS: Working = Working {
    verb: "cut",
    to: "fishportions",
    how_much: 2,
    makes: "fishstrips",
    how_many: 2,
    hands: SkillType::Leatherworking,
    effort: 4.0,
    obvious: true,
    holds: None,
    feeds: Some(crate::world::ItemType::Fish),
    wants_water: 0.0,
    over_a_fire: false,
};

/// And the same with a joint of meat.
pub const CUT_MEAT_INTO_STRIPS: Working = Working {
    verb: "cut",
    to: "meatportions",
    how_much: 2,
    makes: "meatstrips",
    how_many: 2,
    hands: SkillType::Leatherworking,
    effort: 5.0,
    obvious: true,
    holds: None,
    feeds: Some(crate::world::ItemType::Meat),
    wants_water: 0.0,
    over_a_fire: false,
};

/// Flax worked into a basket, which is how a person carries more than their
/// arms hold.
pub const WEAVE_A_BASKET: Working = Working {
    verb: "weave",
    to: "flax",
    how_much: 3,
    makes: "basket",
    how_many: 1,
    hands: SkillType::Crafting,
    effort: 8.0,
    obvious: true,
    holds: None,
    feeds: None,
    wants_water: 0.0,
    over_a_fire: false,
};

/// A block of wood hollowed out, which is how water travels.
///
/// Not obvious, and the thing the whole container machinery was waiting for:
/// an agent with a bowl fills it at the river and drinks from it a day's walk
/// away.
pub const CARVE_A_BOWL: Working = Working {
    verb: "carve",
    to: "wood",
    how_much: 2,
    makes: "bowl",
    how_many: 1,
    hands: SkillType::Crafting,
    effort: 10.0,
    obvious: false,
    holds: Some(4.0),
    feeds: None,
    wants_water: 0.0,
    over_a_fire: false,
};

/// Flax left in water until the stem lets go of the fibre.
///
/// Retting: the first real step of making linen, and the one that turns a
/// handful of stalks into three times the cordage they would otherwise give.
/// It wants a vessel of water, which is why nobody could do it until somebody
/// had carved a bowl.
pub const SOAK_FLAX: Working = Working {
    verb: "soak",
    to: "flax",
    how_much: 2,
    makes: "rettedflax",
    how_many: 3,
    hands: SkillType::Crafting,
    effort: 4.0,
    obvious: false,
    holds: None,
    feeds: None,
    wants_water: 2.0,
    over_a_fire: false,
};

/// Fruit and water left alone until it turns into something that keeps.
///
/// "The agents will need to plant, care for, harvest, and store crops to have
/// a steady food supply." This is the storing: berries go over in hours and
/// what they ferment into keeps a fortnight.
pub const FERMENT_FRUIT: Working = Working {
    verb: "ferment",
    to: "food",
    how_much: 4,
    makes: "ale",
    how_many: 3,
    hands: SkillType::Cooking,
    effort: 5.0,
    obvious: false,
    holds: None,
    feeds: Some(crate::world::ItemType::Ale),
    wants_water: 3.0,
    over_a_fire: false,
};

/// A pot of flour and water over a fire.
///
/// Flour is one of the things a fire on its own ruins - see
/// `ItemType::cooking_outcome`, where whole grain improves and ground grain
/// does not - so until there was a vessel there was no way to cook it at all.
/// This is the last link of a chain that starts at a seed: grain, crushed
/// between two stones, boiled in a pot, is bread.
pub const BOIL_FLOUR: Working = Working {
    verb: "boil",
    to: "flour",
    how_much: 3,
    makes: "bread",
    how_many: 3,
    hands: SkillType::Cooking,
    effort: 6.0,
    obvious: false,
    holds: None,
    feeds: Some(crate::world::ItemType::Bread),
    wants_water: 2.0,
    over_a_fire: true,
};

/// Everything that can be done to a thing to make it another thing.
pub const EVERY_WORKING: &[Working] = &[
    SMASH_A_CORE,
    CUT_A_HIDE,
    SCRAPE_A_STICK,
    CRUSH_GRAIN,
    CUT_MEAT_INTO_PORTIONS,
    CUT_FISH_INTO_PORTIONS,
    CUT_FISH_INTO_STRIPS,
    CUT_MEAT_INTO_STRIPS,
    WEAVE_A_BASKET,
    CARVE_A_BOWL,
    SOAK_FLAX,
    FERMENT_FRUIT,
    BOIL_FLOUR,
];

/// The working of that verb on that thing, if there is one.
pub fn how_to_work(verb: &str, to: &str) -> Option<&'static Working> {
    EVERY_WORKING
        .iter()
        .find(|working| working.verb == verb && working.to == to)
}

/// Everything that can be done to a named thing.
pub fn what_can_be_done_to(what: &str) -> impl Iterator<Item = &'static Working> + '_ {
    EVERY_WORKING.iter().filter(move |working| working.to == what)
}

/// The workings nobody arrives knowing.
pub fn every_working_to_find_out() -> impl Iterator<Item = &'static Working> {
    EVERY_WORKING.iter().filter(|working| !working.obvious)
}

/// Putting the wrong thing in the right place.
///
/// "Knowing that a stone tool requires the use of specific sub-components, an
/// agent might substitute known sub-components for new/random things."
///
/// A man who can haft a flake to a stick knows the shape of the job: a shaft,
/// a head, something to bind them. Given that shape and a pack with something
/// unexpected in it, he can put the unexpected thing where the head goes and
/// see what he ends up with. Almost always he ends up with nothing and has
/// wasted a good stick. Occasionally he ends up with something nobody had.
///
/// This table is what happens when he is right. Everything not in it is the
/// wasted stick.
#[derive(Debug, Clone, Copy)]
pub struct Swap {
    /// The step being attempted, named by what it normally makes
    pub instead_of_making: &'static str,
    /// The part left out
    pub instead_of: &'static str,
    /// And what went in its place
    pub put_in: &'static str,
    /// What comes out
    pub makes: &'static str,
    pub how_many: u32,
}

/// A hide cut round and round makes a longer thong than a handful of flax
/// makes cord, and a better one.
pub const THONG_FOR_CORD: Swap = Swap {
    instead_of_making: "lashing",
    instead_of: "flax",
    put_in: "hides",
    makes: "lashing",
    how_many: 3,
};

/// A hafted metal blade is an axe, and a far better one than a flake.
pub const BLADE_FOR_FLAKE_IN_AN_AXE: Swap = Swap {
    instead_of_making: "handaxe",
    instead_of: "knappedtip",
    put_in: "metalblade",
    makes: "metalaxe",
    how_many: 1,
};

/// The same blade lashed the other way round is a spear that goes in.
pub const BLADE_FOR_FLAKE_IN_A_SPEAR: Swap = Swap {
    instead_of_making: "spear",
    instead_of: "knappedtip",
    put_in: "metalblade",
    makes: "metalspear",
    how_many: 1,
};

/// Everything that comes of putting the wrong thing in the right place.
pub const EVERY_SWAP: &[Swap] = &[
    THONG_FOR_CORD,
    BLADE_FOR_FLAKE_IN_AN_AXE,
    BLADE_FOR_FLAKE_IN_A_SPEAR,
];

/// What comes of this particular substitution, if anything does.
pub fn what_comes_of_swapping(
    instead_of_making: &str,
    instead_of: &str,
    put_in: &str,
) -> Option<&'static Swap> {
    EVERY_SWAP.iter().find(|swap| {
        swap.instead_of_making == instead_of_making
            && swap.instead_of == instead_of
            && swap.put_in == put_in
    })
}

/// How a try at a substitution is written down, so that nobody spends a life
/// putting the same wrong thing in the same right place.
pub fn what_that_swap_is_called(
    instead_of_making: &str,
    instead_of: &str,
    put_in: &str,
) -> String {
    format!("swap:{instead_of_making}:{instead_of}:{put_in}")
}

/// A hafted metal blade, once anybody has worked out that it can be done.
///
/// The same three parts as a hand axe with a blade where the flake goes. It is
/// here rather than only in the swap table because a discovery nobody can
/// repeat is not a discovery: the first one is found by putting the wrong
/// thing in the right place, and every one after that is made on purpose.
pub const METAL_AXE: Making = Making {
    makes: "metalaxe",
    how_many: 1,
    needs: &[("wood", 1), ("metalblade", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 9.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: None,
};

/// And the same, lashed the other way round.
pub const METAL_SPEAR: Making = Making {
    makes: "metalspear",
    how_many: 1,
    needs: &[("wood", 1), ("metalblade", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 9.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: None,
};

/// Everything a stone-age people can put together, and the steps beyond it
/// that they have to find out for themselves.
pub const EVERY_STEP: &[Making] = &[
    LASHING,
    LASHING_FROM_COTTON,
    LASHING_FROM_RETTED,
    KNAPPED_TIP,
    KNAPPED_TIP_FROM_FLINT,
    SPEAR,
    HAND_AXE,
    STONE_KNIFE,
    SHINY_LUMP,
    METAL_BLADE,
    METAL_KNIFE,
    METAL_AXE,
    METAL_SPEAR,
];

/// The steps nobody arrives knowing.
pub fn everything_to_find_out() -> impl Iterator<Item = &'static Making> {
    EVERY_STEP.iter().filter(|step| !step.obvious)
}

/// Whether a named thing is already part of something everybody understands.
///
/// A length of cord is a thing every person here has handled a thousand times:
/// turning one over in your hands tells you nothing, whatever else in the
/// world happens to be held together with cord. A lump of bright stone is not,
/// and that is the difference between a thing worth looking at and a thing
/// worth using.
///
/// This is what keeps looking closely from being a way of reading the whole
/// table: without it, examining a piece of cordage announced the metal knife,
/// because a metal knife happens to be lashed together.
pub fn is_a_familiar_thing(what: &str) -> bool {
    EVERY_STEP
        .iter()
        .filter(|step| step.obvious)
        .any(|step| {
            step.makes == what || step.needs.iter().any(|(needs, _)| *needs == what)
        })
        || EVERY_WORKING
            .iter()
            .filter(|working| working.obvious)
            .any(|working| working.to == what || working.makes == what)
}

/// The step that makes a named thing, if there is one.
pub fn how_to_make(what: &str) -> Option<&'static Making> {
    EVERY_STEP.iter().find(|step| step.makes == what)
}

/// Every way of making a named thing.
pub fn every_way_to_make(what: &str) -> impl Iterator<Item = &'static Making> + '_ {
    EVERY_STEP.iter().filter(move |step| step.makes == what)
}

/// Whether a named thing is something a person makes rather than finds.
pub fn is_made_not_found(what: &str) -> bool {
    how_to_make(what).is_some()
}

/// How far back a person will work from the thing he wants.
///
/// Four is more than the chain is deep. It is here so that a table with a
/// loop in it cannot hang the simulation.
const AS_FAR_BACK_AS_ANYBODY_PLANS: usize = 4;

/// The step to take now towards a named thing.
///
/// A man who wants a spear and is holding nothing but flax and stone does not
/// want to be told he cannot have a spear. He wants to be told to twist some
/// cordage, because that is the part of a spear he can do today. This works
/// back from the thing wanted through what it is short of, and returns the
/// first step along the way whose makings are actually in the pack.
pub fn what_to_do_first(what: &str, holding: &impl Fn(&str) -> u32) -> Option<&'static Making> {
    what_to_do_first_knowing(what, holding, &|step: &Making| step.obvious)
}

/// The same, for somebody who knows more than he was born knowing.
///
/// `knows` says whether this particular person can do a step at all. A man
/// who has never seen a bright stone come out of a fire cannot plan a metal
/// knife, and cannot see that the lump in his pack is a step towards one.
pub fn what_to_do_first_knowing(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
) -> Option<&'static Making> {
    step_towards(what, holding, knows, 0)
}

fn step_towards(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
    how_far_back: usize,
) -> Option<&'static Making> {
    if how_far_back >= AS_FAR_BACK_AS_ANYBODY_PLANS {
        return None;
    }

    // Anything that can be done right now is done right now, unless there
    // are already plenty of them in the pack.
    for step in every_way_to_make(what).filter(|step| knows(step)) {
        if step.makings_to_hand(holding) && holding(step.makes) < A_FEW_SPARE {
            return Some(step);
        }
    }

    // Otherwise the thing wanted is short of something. If that something is
    // itself made rather than found, the making of it is the job in hand.
    for step in every_way_to_make(what).filter(|step| knows(step)) {
        for (needed, how_many) in step.needs {
            if holding(needed) >= *how_many || *needed == what || !is_made_not_found(needed) {
                continue;
            }
            if let Some(earlier) = step_towards(needed, holding, knows, how_far_back + 1) {
                return Some(earlier);
            }
        }
    }

    None
}

/// How many of a made part a person keeps about him before he stops making
/// more of them.
///
/// Cordage and struck flakes are worth having spare, and the chain would
/// otherwise stand a man in a flax meadow twisting rope for the rest of his
/// life because rope is the one step he can always take.
pub const A_FEW_SPARE: u32 = 4;

/// The found thing a chain is waiting on, if it is waiting on one.
///
/// The other half of `what_to_do_first`: when no step along the way can be
/// taken, this says which raw thing to go out and get. Working back from a
/// spear with an empty pack it answers wood, then stone, then flax, in the
/// order the spear needs them.
pub fn what_is_wanting(what: &str, holding: &impl Fn(&str) -> u32) -> Option<&'static str> {
    wanting(what, holding, &|step: &Making| step.obvious, 0)
}

fn wanting(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
    how_far_back: usize,
) -> Option<&'static str> {
    if how_far_back >= AS_FAR_BACK_AS_ANYBODY_PLANS {
        return None;
    }

    for step in every_way_to_make(what).filter(|step| knows(step)) {
        for (needed, how_many) in step.needs {
            if holding(needed) >= *how_many {
                continue;
            }
            if !is_made_not_found(needed) {
                return Some(needed);
            }
            if *needed == what {
                continue;
            }
            if let Some(further_back) = wanting(needed, holding, knows, how_far_back + 1) {
                return Some(further_back);
            }
        }
    }

    None
}

/// A made thing that helps with a kind of work.
///
/// Before this, a tool was a thing an agent counted and nothing else: a man
/// with a stone axe felled timber at exactly the rate of a man with his bare
/// hands, and the whole of `Inventory`'s durability machinery was used by
/// clothing alone. A tool is here what it is for, how much better it makes the
/// work, and how long it lasts before it is fit for nothing.
#[derive(Debug, Clone, Copy)]
pub struct Tool {
    /// What it is called, as it is carried
    pub called: &'static str,
    /// The work it is for
    pub helps: SkillType,
    /// What it multiplies that work by, in the hands of somebody who has one
    pub how_much_better: f32,
    /// How many pieces of work it has in it before it is done
    pub how_long_it_lasts: f32,
}

/// A stone edge on a wooden haft, for wood.
pub const AXE_FOR_WOOD: Tool = Tool {
    called: "handaxe",
    helps: SkillType::Woodcutting,
    how_much_better: 1.8,
    how_long_it_lasts: 40.0,
};

/// The same tool, turned on the ground.
pub const AXE_FOR_STONE: Tool = Tool {
    called: "handaxe",
    helps: SkillType::Mining,
    how_much_better: 1.5,
    how_long_it_lasts: 40.0,
};

/// A spear is the whole of stone-age hunting.
pub const SPEAR_FOR_HUNTING: Tool = Tool {
    called: "spear",
    helps: SkillType::Hunting,
    how_much_better: 2.0,
    how_long_it_lasts: 25.0,
};

/// And for fish, which is the same work done slower.
pub const SPEAR_FOR_FISHING: Tool = Tool {
    called: "spear",
    helps: SkillType::Fishing,
    how_much_better: 1.6,
    how_long_it_lasts: 25.0,
};

/// A knife is for taking a carcass apart.
pub const KNIFE_FOR_BUTCHERING: Tool = Tool {
    called: "stoneknife",
    helps: SkillType::Leatherworking,
    how_much_better: 1.8,
    how_long_it_lasts: 30.0,
};

/// A knife is also what a stone-age people cuts fibre and hide with.
pub const KNIFE_FOR_CRAFTING: Tool = Tool {
    called: "stoneknife",
    helps: SkillType::Crafting,
    how_much_better: 1.3,
    how_long_it_lasts: 30.0,
};

/// A metal edge holds where a stone one chips.
pub const METAL_KNIFE_FOR_BUTCHERING: Tool = Tool {
    called: "metalknife",
    helps: SkillType::Leatherworking,
    how_much_better: 2.4,
    how_long_it_lasts: 90.0,
};

/// The same edge, at the bench.
pub const METAL_KNIFE_FOR_CRAFTING: Tool = Tool {
    called: "metalknife",
    helps: SkillType::Crafting,
    how_much_better: 1.7,
    how_long_it_lasts: 90.0,
};

/// What a man gets between himself and something that is coming at him.
///
/// Not the same job as hunting with it: a thrown spear is a throw and a
/// braced one is a fence. Both wear the shaft, and the second wears it faster,
/// because what it is being asked to do is stop something.
pub const SPEAR_FOR_KEEPING_IT_OFF: Tool = Tool {
    called: "spear",
    helps: SkillType::MeleeCombat,
    how_much_better: 1.9,
    how_long_it_lasts: 25.0,
};

pub const AXE_FOR_KEEPING_IT_OFF: Tool = Tool {
    called: "handaxe",
    helps: SkillType::MeleeCombat,
    how_much_better: 1.5,
    how_long_it_lasts: 40.0,
};

/// An axe with a metal head. Twice the tool a stone one is, and it lasts.
pub const METAL_AXE_FOR_WOOD: Tool = Tool {
    called: "metalaxe",
    helps: SkillType::Woodcutting,
    how_much_better: 2.6,
    how_long_it_lasts: 110.0,
};

pub const METAL_AXE_FOR_STONE: Tool = Tool {
    called: "metalaxe",
    helps: SkillType::Mining,
    how_much_better: 2.1,
    how_long_it_lasts: 110.0,
};

/// And a spear that goes in rather than bruising.
pub const METAL_SPEAR_FOR_HUNTING: Tool = Tool {
    called: "metalspear",
    helps: SkillType::Hunting,
    how_much_better: 3.0,
    how_long_it_lasts: 100.0,
};

pub const METAL_SPEAR_FOR_FISHING: Tool = Tool {
    called: "metalspear",
    helps: SkillType::Fishing,
    how_much_better: 2.2,
    how_long_it_lasts: 100.0,
};

/// Every tool these people have, and what each is for.
pub const EVERY_TOOL: &[Tool] = &[
    AXE_FOR_WOOD,
    AXE_FOR_STONE,
    SPEAR_FOR_HUNTING,
    SPEAR_FOR_FISHING,
    KNIFE_FOR_BUTCHERING,
    KNIFE_FOR_CRAFTING,
    METAL_KNIFE_FOR_BUTCHERING,
    METAL_KNIFE_FOR_CRAFTING,
    METAL_AXE_FOR_WOOD,
    METAL_AXE_FOR_STONE,
    METAL_SPEAR_FOR_HUNTING,
    METAL_SPEAR_FOR_FISHING,
    SPEAR_FOR_KEEPING_IT_OFF,
    AXE_FOR_KEEPING_IT_OFF,
    METAL_SPEAR_FOR_KEEPING_IT_OFF,
];

/// And the same in metal, which stops a good deal more.
pub const METAL_SPEAR_FOR_KEEPING_IT_OFF: Tool = Tool {
    called: "metalspear",
    helps: SkillType::MeleeCombat,
    how_much_better: 2.4,
    how_long_it_lasts: 100.0,
};

/// The tools that are any use for a kind of work.
pub fn what_helps_with(trade: SkillType) -> impl Iterator<Item = &'static Tool> {
    EVERY_TOOL.iter().filter(move |tool| tool.helps == trade)
}

/// How long a newly made thing lasts in the hands that made it.
///
/// Stone and wood wear out fast, and a badly made one wears out faster. A
/// beginner's spear is a third of what a practised hand turns out, which is
/// the reason to keep making them.
///
/// `hand` is the skill multiplier - see `Skills::hand_for` - where 1.0 is an
/// ordinary untrained adult.
pub fn how_long_this_one_lasts(tool: &Tool, hand: f32) -> f32 {
    (tool.how_long_it_lasts * hand.clamp(0.4, 2.0)).max(1.0)
}

/// Every found thing a chain is short of, in the order the steps ask for them.
///
/// `what_is_wanting` names one. This names all of them, because a lashing can
/// be had from flax or from cotton and a man standing in a meadow of the
/// second should not spend his life walking after the first.
pub fn everything_wanting(what: &str, holding: &impl Fn(&str) -> u32) -> Vec<&'static str> {
    everything_wanting_knowing(what, holding, &|step: &Making| step.obvious)
}

/// The same, for somebody who knows more than he was born knowing.
pub fn everything_wanting_knowing(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
) -> Vec<&'static str> {
    let mut wanting = Vec::new();
    gather_wanting(what, holding, knows, 0, &mut wanting);
    wanting
}

fn gather_wanting(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
    how_far_back: usize,
    into: &mut Vec<&'static str>,
) {
    if how_far_back >= AS_FAR_BACK_AS_ANYBODY_PLANS {
        return;
    }

    for step in every_way_to_make(what).filter(|step| knows(step)) {
        for (needed, how_many) in step.needs {
            if holding(needed) >= *how_many {
                continue;
            }
            if !is_made_not_found(needed) {
                if !into.contains(needed) {
                    into.push(needed);
                }
            } else if *needed != what {
                gather_wanting(needed, holding, knows, how_far_back + 1, into);
            }
        }
    }
}
