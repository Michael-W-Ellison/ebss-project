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

/// Everything a stone-age people can put together, and the three steps beyond
/// it that they have to find out for themselves.
pub const EVERY_STEP: &[Making] = &[
    LASHING,
    LASHING_FROM_COTTON,
    KNAPPED_TIP,
    SPEAR,
    HAND_AXE,
    STONE_KNIFE,
    SHINY_LUMP,
    METAL_BLADE,
    METAL_KNIFE,
];

/// The steps nobody arrives knowing.
pub fn everything_to_find_out() -> impl Iterator<Item = &'static Making> {
    EVERY_STEP.iter().filter(|step| !step.obvious)
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
];

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
