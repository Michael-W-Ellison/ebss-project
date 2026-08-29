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

/// A stick with a hardened point, for getting roots out of the ground.
///
/// The oldest tool there is and the one this model did not have. Every trade
/// with a tool behind it - wood, stone, hunting, fishing, butchering, crafting
/// - had one that founders arrive carrying or knowing, and **Herbalism, which
/// is what most turns of most days are spent on, had none at all**. So an
/// agent weighing a better tool against the one in its hand always found there
/// was nothing to weigh: `what_i_would_rather_have` returned nothing for every
/// trade, and `would_a_better_tool_pay` reached its arithmetic twenty-one
/// times in fourteen thousand agent-turns.
///
/// It is a stick and an afternoon, which is the point: the first upgrade a
/// people can afford has to be one they can afford on their first day.
pub const DIGGING_STICK: Making = Making {
    makes: "diggingstick",
    how_many: 1,
    needs: &[("wood", 1)],
    hands: SkillType::Crafting,
    effort: 5.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: Some("stoneknife"),
};

/// Cord and a pouch of stone: what a people hunts with when it has no spear.
///
/// Arrived-with, like the sling, the rod and the shovel below it. The line
/// between what a people knows and what it has to find out is drawn at
/// *invention*, not at usefulness: a sling, a line and a hafted blade are the
/// same three ideas as the handaxe these founders already carry, put to
/// different ends. A bow, a net and a wheel are not - each is a thing somebody
/// had to think of.
///
/// The whole ladder was found-out at first, and measured that way nobody ever
/// climbed a rung of it: two digging sticks in a run and nothing else, because
/// a settlement dies at about a hundred days and discovery is slower than
/// that. A ladder whose first rung is above the ceiling is not a ladder.
///
/// Below the spear, and that is the point of it. A spear wants a knapped tip
/// and a haft; a sling wants a length of cordage and something to put in it,
/// so it is what a man whose spear has broken in a place with no flint can
/// still make. `what_i_would_rather_have` only proposes what beats the tool in
/// hand, so nobody carrying a good spear will ever want one - and somebody
/// carrying nothing will.
pub const SLING: Making = Making {
    makes: "sling",
    how_many: 1,
    needs: &[("lashing", 1), ("hides", 1)],
    hands: SkillType::Crafting,
    effort: 5.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// A stave, and a cord strung tight enough to throw.
///
/// The top of the stone-age hunting ladder and a thing that has to be found
/// out. No arrows in it: a bow that spends ammunition wants a whole model of
/// ammunition behind it - fletching, recovery, running out mid-hunt - and
/// there is none, so this is a bow as the rest of the tools are, a multiplier
/// that wears out. Arrows are worth building when there is something for
/// running out of them to mean.
pub const BOW: Making = Making {
    makes: "bow",
    how_many: 1,
    needs: &[("wood", 2), ("lashing", 2)],
    hands: SkillType::Crafting,
    effort: 12.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: Some("stoneknife"),
};

/// A pole, a line, and patience.
///
/// The `Fish` action has looked for something with "rod" in its name since the
/// fishery was built, and has been giving a fifth of a chance to nobody at all,
/// because nothing in the making chain ever produced one. This is that thing,
/// so it tells twice: once through the tool multiplier and once through the
/// odds of a cast.
pub const FISHING_ROD: Making = Making {
    makes: "fishingrod",
    how_many: 1,
    needs: &[("wood", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 6.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: Some("stoneknife"),
};

/// Cordage, and a great deal of it.
///
/// The best thing a stone-age people can put in the water and the most
/// expensive: four lashings is a season of retting and twisting, and it is the
/// clearest case in the chain of a tool that is plainly worth it and plainly
/// out of reach on the first day.
pub const FISHING_NET: Making = Making {
    makes: "fishingnet",
    how_many: 1,
    needs: &[("lashing", 4)],
    hands: SkillType::Crafting,
    effort: 14.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: None,
};

/// A blade lashed to a haft, for ground rather than for wood.
///
/// A pit is twenty-two energy of digging with bare hands, which is most of a
/// turn's work, and a settlement that cannot dig cheaply cannot build a larder.
pub const SHOVEL: Making = Making {
    makes: "shovel",
    how_many: 1,
    needs: &[("wood", 1), ("knappedtip", 1), ("lashing", 1)],
    hands: SkillType::Crafting,
    effort: 9.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: Some("stoneknife"),
};

/// Woven cordage: what a berry goes into when it is not going into a mouth.
///
/// "An agent can eat from a berry bush but cannot carry additional berries
/// unless they are carrying a pack or container. This means that the act of
/// walking to and from the berry patch each time the agent is hungry will take
/// additional time, making it less efficient."
///
/// So carrying is a thing you need a thing for. A pair of hands holds what a
/// pair of hands holds, and everything past that wants a basket - which is why
/// this is the cheapest making in the chain and the one founders arrive with.
/// A people that knows how to twist cordage knows how to weave it.
pub const BASKET: Making = Making {
    makes: "basket",
    how_many: 1,
    needs: &[("lashing", 2)],
    hands: SkillType::Crafting,
    effort: 5.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: None,
};

/// Two poles and a hide, dragged.
///
/// "A cart should be a rather advanced piece of technology. An initial method
/// of moving things would likely be more of a travois." Which is right, and
/// the cart was standing in the travois's place: a stone-age people had one
/// rung between a basket and a wagon, and it had wheels on it.
///
/// A travois is two poles, a lashing and something to lie across them. No
/// wheel, no axle, no bearing - the load drags, so it costs more of the
/// walking than a cart does and carries less. That is the whole difference
/// between them, and it is why one comes first.
pub const TRAVOIS: Making = Making {
    makes: "travois",
    how_many: 1,
    needs: &[("wood", 2), ("lashing", 1), ("hides", 1)],
    hands: SkillType::Crafting,
    effort: 9.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: Some("handaxe"),
};

/// The wheel itself, which is the advanced part.
///
/// Not the cart: the cart is a box on poles and anybody can see how to build
/// one. What nobody can see how to build is a disc that turns true on an axle,
/// and that is the thing that has to be found out. Two of them and a bed makes
/// a cart; without them the same wood and lashing makes a travois.
pub const WHEEL: Making = Making {
    makes: "wheel",
    how_many: 1,
    needs: &[("wood", 2), ("lashing", 1)],
    hands: SkillType::Carpentry,
    effort: 16.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: Some("handaxe"),
};

/// A bed on two wheels, which is what the wheels were for.
///
/// Advanced, now, in the only way that means anything: it wants two wheels and
/// nobody is born knowing how to make one. It was four lengths of wood and a
/// lashing that founders could turn out on their first afternoon, which put
/// the wheel in the same bracket as a digging stick.
///
/// What it does is carry, and `TransportSystem` has been able to model exactly
/// that since it was written - capacity already summed into
/// `Inventory::max_weight`, speed already multiplied into
/// `movement_speed_at_tick`. Nothing had ever put a transport into it.
pub const HANDCART: Making = Making {
    makes: "handcart",
    how_many: 1,
    needs: &[("wheel", 2), ("wood", 3), ("lashing", 2)],
    hands: SkillType::Carpentry,
    effort: 20.0,
    obvious: false,
    over_a_fire: false,
    wants_in_hand: Some("handaxe"),
};

/// A stick with a point burnt into it.
///
/// "Hunting any larger animal requires at least a spear. A wooden spear is
/// enough, but should take several attacks to kill the animal, depending on
/// the size of the animal. A flint spear should reduce the number of attacks."
/// So there are two spears, and this is the first: no tip, no lashing, one
/// length of wood and an evening at the fire.
pub const SHARPENED_STICK: Making = Making {
    makes: "sharpenedstick",
    how_many: 1,
    needs: &[("wood", 1)],
    hands: SkillType::Crafting,
    effort: 4.0,
    obvious: true,
    over_a_fire: false,
    wants_in_hand: Some("stoneknife"),
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

/// A hide scraped down into workable leather.
///
/// Scraping, not cutting. Taking a flint to a hide removes the hair and turns
/// the skin into leather; cutting a hide gets you two smaller hides. This is
/// what leatherworking *is*, and it is the one step in the chain where the
/// skill belongs - what comes afterwards is sewing, which is making like any
/// other making.
pub const SCRAPE_A_HIDE: Working = Working {
    verb: "scrape",
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

/// Wet clay worked into a shape and left to stand.
///
/// The first half of "playing with it". Clay is the one material in this
/// world that will hold whatever shape you press it into, and finding that
/// out costs nothing but an idle afternoon with a lump of it - which is why
/// this is a `Working` rather than a `Making`: no other material, no tool, no
/// fire. Just somebody turning a thing over in their hands.
///
/// What comes out is worth almost nothing. A shape in unfired clay holds
/// nothing, keeps nothing and comes apart in the rain. All of its value is in
/// what a fire does to it afterwards, and nobody here knows that yet.
pub const MOLD_CLAY: Working = Working {
    verb: "mold",
    to: "clay",
    how_much: 2,
    makes: "claypot",
    how_many: 1,
    hands: SkillType::Crafting,
    effort: 5.0,
    obvious: false,
    holds: None,
    feeds: None,
    // Clay out of a riverbank is already wet, and asking for carried water
    // here would be a circular precondition of the kind this project keeps
    // turning up: you would need a vessel to carry the water to make the
    // vessel.
    wants_water: 0.0,
    over_a_fire: false,
};

/// And the same shape put in a fire, which stops it being clay.
///
/// This is the technology. A fired pot holds water, holds food, and does not
/// come apart in the rain - the first thing this people can make that keeps
/// something else. `ResourceType::Pottery` has been an enum variant with
/// nothing behind it since the project began.
///
/// It can be found out two ways. A curious agent sitting at a fire with a
/// shape in unfired clay tries putting it in - that is this working. Or a
/// lump of clay in somebody's pack comes out of the embers hard, which nobody
/// intended and everybody sees: see `Simulation::what_the_embers_did`.
pub const FIRE_A_POT: Working = Working {
    verb: "fire",
    to: "claypot",
    how_much: 1,
    makes: "stoneware",
    how_many: 1,
    hands: SkillType::Crafting,
    effort: 8.0,
    obvious: false,
    holds: Some(WHAT_A_FIRED_POT_HOLDS),
    feeds: None,
    wants_water: 0.0,
    over_a_fire: true,
};

/// And clay fired in a block rather than a pot, which is a brick.
///
/// "Other developments like bricks could emerge from similar exploration and
/// curiosity." So it does: it is a separate thing to find out, off the same
/// material and the same fire, and nothing hands it down. A people that has
/// fired a pot has not thereby learned to make a wall.
pub const FIRE_BRICKS: Working = Working {
    verb: "fire",
    to: "clay",
    how_much: 4,
    makes: "bricks",
    how_many: 2,
    hands: SkillType::Crafting,
    effort: 10.0,
    obvious: false,
    holds: None,
    feeds: None,
    wants_water: 0.0,
    over_a_fire: true,
};

/// What a fired pot will hold, in units of water.
///
/// More than twice what a carved wooden bowl holds, because it does not leak
/// and it does not have to be hollowed out of anything.
///
/// This said "a little more than a carved wooden bowl" and was set to exactly
/// what a carved wooden bowl holds, so firing clay bought a people nothing
/// whatever over carving wood and there was no reason on earth to bother with
/// pottery. The comment was right and the number was wrong.
pub const WHAT_A_FIRED_POT_HOLDS: f32 = 9.0;

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

/// Leather sewn into a bag, which is how a person carries a great deal more
/// than their arms hold.
///
/// A basket is flax and holds what flax holds. A leather bag is the other
/// answer to the same problem and a better one, and what it costs is the
/// *material*: hides come off something that had to be killed, and a hide is
/// not leather until somebody has scraped the hair off it.
///
/// Sewing a bag is crafting, not leatherworking. The skill sits one step
/// earlier, on the scraping - which is what leatherworking is. Putting it here
/// as well would have paid a man twice for one trade, and it is the material
/// that gates this rather than the hand.
pub const SEW_A_BAG: Working = Working {
    verb: "weave",
    to: "leather",
    how_much: 3,
    makes: "leatherbag",
    how_many: 1,
    hands: SkillType::Crafting,
    effort: 10.0,
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
    // Obvious, where it used to want discovering. Weaving a basket out of
    // flax is obvious in this table and hollowing out a block of wood is no
    // greater a leap - and gating it kept the *entire fluid family* inert:
    // no vessel meant no carried water, no boiling, and no salt. A people
    // that carves a spear can hollow a log.
    obvious: true,
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
    SCRAPE_A_HIDE,
    SCRAPE_A_STICK,
    CRUSH_GRAIN,
    MOLD_CLAY,
    FIRE_A_POT,
    FIRE_BRICKS,
    CUT_MEAT_INTO_PORTIONS,
    CUT_FISH_INTO_PORTIONS,
    CUT_FISH_INTO_STRIPS,
    CUT_MEAT_INTO_STRIPS,
    WEAVE_A_BASKET,
    SEW_A_BAG,
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
    DIGGING_STICK,
    BASKET,
    TRAVOIS,
    WHEEL,
    SHARPENED_STICK,
    SLING,
    BOW,
    FISHING_ROD,
    FISHING_NET,
    SHOVEL,
    HANDCART,
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

/// The same, but only proposing a step that could actually be carried out.
///
/// `what_to_do_first_knowing` checks the *materials* and nothing else, so it
/// will happily name a step that wants a handaxe in the hand of a man who has
/// none, or one that wants a fire where there is no fire. Measured, that was
/// **2,378 refused crafts a world** out of 2,719 attempted: 1,421 of them
/// "wants a handaxe" and 957 "no fire burning here".
///
/// A proposal that comes straight back refused is a turn gone, and the agent
/// learns from the refusal, so it is worse than a turn gone - it teaches a man
/// that making knives does not work.
pub fn what_to_do_first_that_can_be_done(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
    in_hand: &impl Fn(&str) -> bool,
    a_fire_is_to_hand: bool,
) -> Option<&'static Making> {
    let step = step_towards(what, holding, knows, 0)?;

    if step.over_a_fire && !a_fire_is_to_hand {
        return None;
    }

    if let Some(wanted) = step.wants_in_hand {
        if !in_hand(wanted) {
            return None;
        }
    }

    Some(step)
}

/// How many turns of work stand between this agent and a finished thing.
///
/// `step_towards` names the next step and nothing about how many follow it.
/// An agent deciding whether a better axe is worth stopping for has to know
/// the price, and the price is the whole chain from what is in the pack to the
/// thing in the hand - "eight hours with this axe, or two hours making a
/// better one and six with that" needs the two.
///
/// Counted the same way the chain is walked, so the answer agrees with what
/// the agent will actually end up doing. `None` where the chain cannot be
/// finished at all from here, which is the honest answer to "how long would
/// this take" when it is short of something that has to be found.
pub fn how_many_turns_to_make(
    what: &str,
    holding: &impl Fn(&str) -> u32,
    knows: &impl Fn(&Making) -> bool,
) -> Option<u32> {
    fn count(
        what: &str,
        holding: &impl Fn(&str) -> u32,
        knows: &impl Fn(&Making) -> bool,
        how_far_back: usize,
    ) -> Option<u32> {
        if how_far_back >= AS_FAR_BACK_AS_ANYBODY_PLANS {
            return None;
        }

        // Already in the pack: nothing to do
        if holding(what) > 0 {
            return Some(0);
        }

        every_way_to_make(what)
            .filter(|step| knows(step))
            .filter_map(|step| {
                let mut turns = 1;
                for (needed, how_many) in step.needs {
                    if holding(needed) >= *how_many || *needed == what {
                        continue;
                    }
                    if !is_made_not_found(needed) {
                        // Short of something that has to be found. Fetching it
                        // is a turn too, and only one - the trip is priced
                        // where trips are priced.
                        turns += 1;
                        continue;
                    }
                    turns += count(needed, holding, knows, how_far_back + 1)?;
                }
                Some(turns)
            })
            .min()
    }

    count(what, holding, knows, 0)
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

/// Crude cutting: the handaxe taking a carcass apart.
///
/// "The most basic tool should be a stone hand axe. This tool allows for crude
/// cutting, digging, and chopping." It did digging and chopping - Mining and
/// Woodcutting - and not cutting, so a people with an axe and no knife could
/// fell a tree and could not butcher what it killed. Crude, and well below the
/// flake that is made for the job, but not nothing.
pub const AXE_FOR_BUTCHERING: Tool = Tool {
    called: "handaxe",
    helps: SkillType::Leatherworking,
    how_much_better: 1.25,
    how_long_it_lasts: 40.0,
};

/// What a root is dug with, and what a patch of ground is turned with.
///
/// Modest and cheap, which is what a digging stick is. Its whole importance is
/// that it exists at all: it is the first rung of a ladder that begins for
/// most people at gathering, and without it the trade that fills most of the
/// day had no ladder to stand on.
pub const STICK_FOR_DIGGING: Tool = Tool {
    called: "diggingstick",
    helps: SkillType::Herbalism,
    how_much_better: 1.5,
    how_long_it_lasts: 30.0,
};

/// The same stick, put to the ground a field is broken with.
pub const STICK_FOR_FARMING: Tool = Tool {
    called: "diggingstick",
    helps: SkillType::Farming,
    how_much_better: 1.4,
    how_long_it_lasts: 30.0,
};

/// A point burnt into a stick: enough to bring a deer down, several throws in.
pub const STICK_FOR_HUNTING: Tool = Tool {
    called: "sharpenedstick",
    helps: SkillType::Hunting,
    how_much_better: 1.4,
    how_long_it_lasts: 15.0,
};

/// What a people hunts with when it has nothing better.
pub const SLING_FOR_HUNTING: Tool = Tool {
    called: "sling",
    helps: SkillType::Hunting,
    how_much_better: 1.5,
    how_long_it_lasts: 20.0,
};

/// And what it hunts with when it has worked out how.
pub const BOW_FOR_HUNTING: Tool = Tool {
    called: "bow",
    helps: SkillType::Hunting,
    how_much_better: 3.0,
    how_long_it_lasts: 60.0,
};

/// A line beats a thrust, and a net beats a line.
pub const ROD_FOR_FISHING: Tool = Tool {
    called: "fishingrod",
    helps: SkillType::Fishing,
    how_much_better: 1.9,
    how_long_it_lasts: 35.0,
};

/// The best thing a stone-age people puts in the water.
pub const NET_FOR_FISHING: Tool = Tool {
    called: "fishingnet",
    helps: SkillType::Fishing,
    how_much_better: 2.6,
    how_long_it_lasts: 40.0,
};

/// For ground, which is what a pit is dug out of.
pub const SHOVEL_FOR_DIGGING: Tool = Tool {
    called: "shovel",
    helps: SkillType::Mining,
    how_much_better: 1.9,
    how_long_it_lasts: 45.0,
};

/// And for the footings of anything anybody puts up.
pub const SHOVEL_FOR_BUILDING: Tool = Tool {
    called: "shovel",
    helps: SkillType::Construction,
    how_much_better: 1.6,
    how_long_it_lasts: 45.0,
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
    AXE_FOR_BUTCHERING,
    STICK_FOR_DIGGING,
    STICK_FOR_FARMING,
    STICK_FOR_HUNTING,
    SLING_FOR_HUNTING,
    BOW_FOR_HUNTING,
    ROD_FOR_FISHING,
    NET_FOR_FISHING,
    SHOVEL_FOR_DIGGING,
    SHOVEL_FOR_BUILDING,
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
