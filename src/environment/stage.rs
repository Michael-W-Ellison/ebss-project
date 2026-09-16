// src/environment/stage.rs
//! Stage 0: what a people already knows on the day the world starts.
//!
//! Every technology in this model arrives one of two ways. Either somebody
//! finds it out - `Making::obvious == false`, and `everything_to_find_out()`
//! is the list - or a people simply has it, because it has always had it and
//! nobody alive remembers not having it. The second kind is Stage 0, and
//! until now it had no name and no list. It was spelled once per recipe, in a
//! boolean, with the reasoning in a doc comment beside it, and there was no
//! way to ask the model what a people starts with, still less to ask what it
//! *ought* to start with and be told where the two disagree.
//!
//! This is that list. Thirty-five development paths, each with the state it
//! begins in: what a people has in its hands, what that lets it do, and what
//! it cannot do until the path advances. The stages after this one are the
//! user's to add; the shape here - `Stage { number, .. }` - is meant to take
//! them without changing.
//!
//! # What this table is for, and what it is not
//!
//! It is a **declaration**, in the same sense as `Strategy::reach`: a way of
//! answering a way. A path that is named and short is a gap anybody can
//! count; a path nobody wrote down is a gap nobody can see. Nineteen of these
//! paths this model carries whole, twelve it carries half of, and four it does
//! not carry at all - and before this file there was no way to say that
//! sentence, let alone check it.
//!
//! It is **not** a second source of truth about recipes. Where a path says a
//! people is born knowing how to make a thing, `born_knowing` names the thing
//! and `the_stage_table_and_the_making_tables_cannot_drift` holds the two to
//! each other: a product claimed here that still wants discovering in
//! `making.rs` fails the suite. That is the whole of the coupling, and it is
//! deliberate - the recipe tables stay the place a recipe lives.
//!
//! # The one disagreement, and what measuring it turned up
//!
//! Writing the table down made one disagreement fall out immediately.
//! **Hand-shaping a clay vessel is Stage 0** - "Hand-shaped clay vessels, pit
//! firing, low temperature firing, porous pottery" - and in this model it is
//! a discovery.
//!
//! It was made obvious to match, and it cost **4.9% of person-days and eight
//! of twenty-one first winters** over sixty-four seeded worlds, on both
//! blocks. So it is back, and the reason it cost that is worth more than the
//! flag was: `what_i_would_work_on` breaks down anything a person knows the
//! working for and has the makings of, **without asking whether what comes
//! out is worth having**. Every other obvious working makes something a
//! person eats, carries things in or builds with. A shape in unfired clay is
//! the first that makes nothing at all, and a people born knowing how spends
//! its winters making them.
//!
//! `Path::Stoneware` therefore reads `Short`, with that as the reason. A
//! table that says where a people ought to start is more useful for saying so
//! and being contradicted than for being quietly bent to fit. See
//! `MOLD_CLAY` and ISSUES_FOUND.md #199.

/// One line of technology, from what a people starts with to wherever it gets.
///
/// One variant per path in the specification, in the order they were given.
/// Several of them overlap in the world - a basket is transport and it is
/// textile and it is storage - and they are still separate paths, because
/// they advance separately and are held back by different things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Path {
    GoodsTransport,
    Textile,
    Clothing,
    Stoneware,
    Storage,
    Footwear,
    FoodPreservation,
    CookingAndFoodProcessing,
    Leatherworking,
    MiningAndExtraction,
    Woodworking,
    Cordage,
    TradeAndExchange,
    WasteAndSanitation,
    WritingAndCounting,
    Communication,
    MaritimeAndBoatbuilding,
    GovernanceAndLabour,
    MeasurementAndStandards,
    DefensiveInfrastructure,
    DomesticGoods,
    FishingAndAquaculture,
    PotteryBeyondStoneware,
    ShelterSystems,
    AnimalPoweredAndMechanical,
    Tools,
    Weapons,
    Metalworking,
    WaterSystems,
    MedicineAndPublicHealth,
    FireFuelAndEnergy,
    Agriculture,
    Construction,
    Masonry,
    AnimalHusbandry,
}

/// Whether this model actually stands where the stage says a people stands.
///
/// The honest half. A stage is a claim about the world at tick zero, and the
/// claim is either carried by machinery or it is not; saying which, and
/// naming what carries it, is the only thing that stops the table becoming a
/// wish list. `Part` exists because most of these are neither - a people that
/// can spear a fish and cannot gather a shellfish has half of the fishing
/// path, and rounding that to either yes or no loses what is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// The world carries it, and here is what does.
    Stands(&'static str),
    /// Half of it, and here is the half that is missing.
    Part(&'static str),
    /// Declared, and nothing in the world carries it yet.
    Short(&'static str),
}

impl Standing {
    /// Whether a people can be said to have this at all.
    pub fn any_of_it(&self) -> bool {
        !matches!(self, Standing::Short(_))
    }

    /// What was said about it, whichever it was.
    pub fn what_was_said(&self) -> &'static str {
        match self {
            Standing::Stands(what) | Standing::Part(what) | Standing::Short(what) => what,
        }
    }
}

/// One state of one path: where a people is, what it can do from there, and
/// what stops it going further.
#[derive(Debug, Clone, Copy)]
pub struct Stage {
    /// How far along. Zero is where everybody starts; the rest are the
    /// user's to add.
    pub number: u8,
    /// What this state of affairs is called.
    pub called: &'static str,
    /// What is in a people's hands here.
    pub starts_with: &'static [&'static str],
    /// What having those lets it do.
    pub can_do: &'static [&'static str],
    /// What it still cannot do, which is what the next stage is for.
    pub held_back_by: &'static [&'static str],
    /// Whether this model carries it.
    pub standing: Standing,
    /// The products of `making.rs` that this stage says everybody knows how
    /// to make.
    ///
    /// Named by product, because that is how `Making::obvious` and
    /// `Agent::found_out` are both keyed. Every name here must be the output
    /// of at least one step the tables call obvious - see
    /// `the_stage_table_and_the_making_tables_cannot_drift`.
    pub born_knowing: &'static [&'static str],
}

/// Every path, for counting what a people has and what it has not.
pub const EVERY_PATH: &[Path] = &[
    Path::GoodsTransport,
    Path::Textile,
    Path::Clothing,
    Path::Stoneware,
    Path::Storage,
    Path::Footwear,
    Path::FoodPreservation,
    Path::CookingAndFoodProcessing,
    Path::Leatherworking,
    Path::MiningAndExtraction,
    Path::Woodworking,
    Path::Cordage,
    Path::TradeAndExchange,
    Path::WasteAndSanitation,
    Path::WritingAndCounting,
    Path::Communication,
    Path::MaritimeAndBoatbuilding,
    Path::GovernanceAndLabour,
    Path::MeasurementAndStandards,
    Path::DefensiveInfrastructure,
    Path::DomesticGoods,
    Path::FishingAndAquaculture,
    Path::PotteryBeyondStoneware,
    Path::ShelterSystems,
    Path::AnimalPoweredAndMechanical,
    Path::Tools,
    Path::Weapons,
    Path::Metalworking,
    Path::WaterSystems,
    Path::MedicineAndPublicHealth,
    Path::FireFuelAndEnergy,
    Path::Agriculture,
    Path::Construction,
    Path::Masonry,
    Path::AnimalHusbandry,
];

impl Path {
    /// What this path is called, short and stable.
    pub fn called(&self) -> &'static str {
        match self {
            Path::GoodsTransport => "goods transport",
            Path::Textile => "textile",
            Path::Clothing => "clothing",
            Path::Stoneware => "stoneware",
            Path::Storage => "storage",
            Path::Footwear => "footwear",
            Path::FoodPreservation => "food preservation",
            Path::CookingAndFoodProcessing => "cooking",
            Path::Leatherworking => "leatherworking",
            Path::MiningAndExtraction => "mining",
            Path::Woodworking => "woodworking",
            Path::Cordage => "cordage",
            Path::TradeAndExchange => "trade",
            Path::WasteAndSanitation => "sanitation",
            Path::WritingAndCounting => "recordkeeping",
            Path::Communication => "communication",
            Path::MaritimeAndBoatbuilding => "boatbuilding",
            Path::GovernanceAndLabour => "governance",
            Path::MeasurementAndStandards => "measurement",
            Path::DefensiveInfrastructure => "defence",
            Path::DomesticGoods => "domestic goods",
            Path::FishingAndAquaculture => "fishing",
            Path::PotteryBeyondStoneware => "pottery",
            Path::ShelterSystems => "shelter systems",
            Path::AnimalPoweredAndMechanical => "mechanical power",
            Path::Tools => "tools",
            Path::Weapons => "weapons",
            Path::Metalworking => "metalworking",
            Path::WaterSystems => "water",
            Path::MedicineAndPublicHealth => "medicine",
            Path::FireFuelAndEnergy => "fire and fuel",
            Path::Agriculture => "agriculture",
            Path::Construction => "construction",
            Path::Masonry => "masonry",
            Path::AnimalHusbandry => "animal husbandry",
        }
    }

    /// Where a people stands on this path on the day the world starts.
    pub fn stage_zero(&self) -> Stage {
        match self {
            // ---- carrying things ------------------------------------------
            Path::GoodsTransport => Stage {
                number: 0,
                called: "human carrying",
                starts_with: &[
                    "hand-carrying",
                    "armloads",
                    "back-carrying",
                    "head-loading",
                    "improvised slings",
                    "dragging bundles by hand",
                ],
                can_do: &["move what one pair of hands can hold, as far as one pair of legs can walk"],
                held_back_by: &[
                    "very low volume",
                    "short range",
                    "high fatigue",
                    "little weather protection for goods",
                ],
                // A travois is two poles and a hide, which is dragging a
                // bundle with the bundle held off the ground. The wheel and
                // the handcart above it are discoveries, correctly.
                standing: Standing::Stands(
                    "a pack with a capacity, a sling and a travois in the obvious half of \
                     the making table, and a wheel and a handcart in the discovered half",
                ),
                born_knowing: &["sling", "travois", "basket"],
            },

            // ---- what covers a body ---------------------------------------
            Path::Textile => Stage {
                number: 0,
                called: "pre-textile coverings",
                starts_with: &[
                    "untanned hides",
                    "fur pelts",
                    "bark sheets",
                    "woven grass, reeds or leaf mats",
                    "plant-fibre cordage",
                ],
                can_do: &["basic body covering", "weather protection", "carrying slings and bindings"],
                held_back_by: &[
                    "heavy",
                    "stiff",
                    "rot-prone",
                    "limited comfort and fit",
                    "poor washability",
                ],
                standing: Standing::Part(
                    "hides, cordage and woven flax are all here; there is no cloth, no \
                     spinning and no loom, so the path has a start and no next step",
                ),
                born_knowing: &["lashing", "basket"],
            },

            Path::Clothing => Stage {
                number: 0,
                called: "basic body covering",
                starts_with: &[
                    "draped hides",
                    "furs",
                    "bark sheets",
                    "leaf wraps",
                    "grass skirts",
                    "simple belts or ties",
                ],
                can_do: &["modesty and social marking", "basic weather protection"],
                held_back_by: &["nothing fits", "nothing is fastened", "nothing sheds water"],
                standing: Standing::Stands(
                    "`clothing_recipes.rs` and the exposure model: garments have \
                     insulation, are worn on slots, and cold is what makes somebody make one",
                ),
                born_knowing: &["leather", "leatherbag"],
            },

            Path::Footwear => Stage {
                number: 0,
                called: "foot wraps",
                starts_with: &["raw hide wraps", "fur wraps", "grass padding", "cord-tied soles"],
                can_do: &["minimal material requirements", "quick to make"],
                held_back_by: &[
                    "poor durability",
                    "poor water resistance once soaked",
                    "minimal support",
                ],
                standing: Standing::Part(
                    "bark boots exist in `clothing_recipes.rs`; nothing in the model asks \
                     for them, because feet are not a place the exposure model wounds",
                ),
                born_knowing: &[],
            },

            // ---- earth and fire -------------------------------------------
            Path::Stoneware => Stage {
                number: 0,
                called: "sun-dried and low-fired clay",
                starts_with: &[
                    "hand-shaped clay vessels",
                    "pit firing",
                    "low temperature firing",
                    "porous pottery",
                ],
                can_do: &["bowls", "storage jars", "cooking pots", "figurines", "hearth fittings"],
                held_back_by: &["brittle", "porous", "inconsistent firing"],
                // The one the table and the model disagree about, and the
                // disagreement was measured rather than argued. Shaping clay
                // was made obvious to match the specification and **cost 4.9%
                // of person-days and eight of twenty-one first winters over
                // sixty-four seeded worlds**, because nothing asks whether
                // what a working makes is worth making. See `MOLD_CLAY` and
                // ISSUES_FOUND.md #199.
                standing: Standing::Short(
                    "the specification starts a people here and this model cannot yet \
                     afford to: shaping clay stays a discovery, because a people that \
                     starts knowing how spends its winters making shapes worth nothing",
                ),
                born_knowing: &[],
            },

            Path::PotteryBeyondStoneware => Stage {
                number: 0,
                called: "sun-dried and low-fired clay objects",
                starts_with: &[
                    "figurines",
                    "beads",
                    "crude bowls",
                    "clay-lined hearth pieces",
                    "sun-dried bricks in some dry regions",
                ],
                can_do: &["ornament", "small containers", "lining a hearth"],
                held_back_by: &["fragile", "porous", "weather-sensitive"],
                standing: Standing::Short(
                    "clay makes pots and bricks and nothing else: there is no ornament in \
                     this model, and nothing values a thing for being looked at",
                ),
                born_knowing: &[],
            },

            Path::FireFuelAndEnergy => Stage {
                number: 0,
                called: "opportunistic fire use",
                starts_with: &[
                    "use of naturally occurring fire",
                    "carrying embers",
                    "simple hearths",
                    "dry wood, brush, dung and peat as found",
                ],
                can_do: &["warmth", "cooking on coals", "keeping animals off at night"],
                held_back_by: &[
                    "fire hard to start reliably",
                    "heat control poor",
                    "fuel use inefficient",
                ],
                standing: Standing::Part(
                    "hearths, firewood and cooking over a fire all work; nobody carries an \
                     ember, so a fire that goes out is started from nothing every time",
                ),
                born_knowing: &[],
            },

            // ---- keeping things -------------------------------------------
            Path::Storage => Stage {
                number: 0,
                called: "immediate and opportunistic storage",
                starts_with: &[
                    "food kept near camp",
                    "hanging meats or plants",
                    "piles of stone, wood, fuel and hides",
                    "caches in cool shade",
                    "natural caves",
                ],
                can_do: &["a few days in hand", "somewhere to put down what cannot be carried"],
                held_back_by: &["pest loss", "theft and scavenging", "spoilage", "little organisation"],
                standing: Standing::Stands(
                    "the larder: a dug pit with a cover, a temperature, and a spoilage \
                     clock that a pit slows and the open air does not",
                ),
                born_knowing: &["basket", "leatherbag"],
            },

            Path::FoodPreservation => Stage {
                number: 0,
                called: "basic passive preservation",
                starts_with: &["sun drying", "air drying", "cool storage in caves or shaded pits"],
                can_do: &[
                    "meat strips",
                    "fish",
                    "seeds",
                    "nuts",
                    "herbs",
                    "fruit slices",
                ],
                held_back_by: &["weather dependent", "past exposure", "inconsistent quality"],
                // `THAT_LAYING_IT_OUT_KEEPS_IT` is the born-knowing lesson
                // that made drying something a people does on purpose rather
                // than stumbles into. See ISSUES_FOUND #124.
                standing: Standing::Stands(
                    "cutting strips is obvious, laying them out is the one thing everybody \
                     is born knowing, and the weather decides whether it works",
                ),
                born_knowing: &["meatstrips", "fishstrips"],
            },

            Path::CookingAndFoodProcessing => Stage {
                number: 0,
                called: "raw and ember cooking",
                starts_with: &[
                    "raw consumption where tolerable",
                    "roasting directly on coals",
                    "ash baking",
                    "simple skewering",
                    "stone heating for rudimentary cooking",
                ],
                can_do: &["make meat safe", "make hard things softer"],
                held_back_by: &[
                    "limited control",
                    "high loss and waste",
                    "little ability to process grains or hard foods",
                ],
                // Grinding grain and boiling flour are both discoveries,
                // which is exactly the third limit spelled as a flag.
                standing: Standing::Stands(
                    "portioning is obvious, cooking wants a fire and ruins what should not \
                     have gone near one, and grinding grain is still a discovery",
                ),
                born_knowing: &["meatportions", "fishportions"],
            },

            // ---- the raw materials ----------------------------------------
            Path::Leatherworking => Stage {
                number: 0,
                called: "raw hide use",
                starts_with: &[
                    "fresh hides used as wraps, lashings, bedding and coverings",
                    "scraped hide dried in shape",
                    "fur-on hide kept for warmth",
                ],
                can_do: &["immediate use after a kill", "basic containers, cloaks, shelter panels"],
                held_back_by: &[
                    "stiff when dry",
                    "rots easily when wet",
                    "shrinks and hardens",
                    "limited flexibility and lifespan",
                ],
                standing: Standing::Stands(
                    "scraping a hide is obvious and is where the leatherworking skill \
                     lives; sewing a bag out of what comes off it is obvious too",
                ),
                born_knowing: &["leather", "leatherbag"],
            },

            Path::MiningAndExtraction => Stage {
                number: 0,
                called: "surface gathering",
                starts_with: &[
                    "picking up useful stone from the ground",
                    "collecting clay from streambanks",
                    "gathering ochre, chalk, salt crusts, peat and loose gravel",
                    "using exposed timber, fallen wood and naturally broken rock",
                ],
                can_do: &["tool stone", "pigments", "clay", "salt in crude form", "simple building stone"],
                held_back_by: &["highly local", "inconsistent quality", "easily exhausted"],
                standing: Standing::Stands(
                    "surface deposits of stone, clay and salt, a mining skill, and a \
                     deposit that runs out and is remembered as having run out",
                ),
                born_knowing: &["flint"],
            },

            Path::Woodworking => Stage {
                number: 0,
                called: "opportunistic wood use",
                starts_with: &[
                    "sticks and branches as clubs, digging sticks, stakes and pegs",
                    "wood split by fracture rather than careful shaping",
                    "natural forks used as supports",
                ],
                can_do: &["cutting, crushing, digging and propping"],
                held_back_by: &["minimal shaping", "low precision", "short-lived products"],
                standing: Standing::Stands(
                    "a digging stick and a sharpened stick are both obvious, and a carved \
                     bowl - the one real shaping step - is obvious too",
                ),
                born_knowing: &["diggingstick", "sharpenedstick", "bowl"],
            },

            Path::Cordage => Stage {
                number: 0,
                called: "natural bindings",
                starts_with: &[
                    "vines",
                    "strips of bark",
                    "rawhide thongs",
                    "twisted grasses",
                    "tendon and sinew",
                ],
                can_do: &["lashing one thing to another", "binding a bundle", "weaving"],
                held_back_by: &["weak", "inconsistent", "rot-prone", "short lengths"],
                // Retting - soaking flax until the stem lets go - is what
                // gets past "short lengths", and it is a discovery. The
                // fishing net wants four lengths and is a discovery for the
                // same reason.
                standing: Standing::Stands(
                    "lashing from flax or cotton is obvious and holds nearly everything \
                     else in the table together; retting past it is a discovery",
                ),
                born_knowing: &["lashing"],
            },

            // ---- what people do with each other ---------------------------
            Path::TradeAndExchange => Stage {
                number: 0,
                called: "sharing and reciprocal exchange",
                starts_with: &[
                    "household sharing",
                    "gift exchange",
                    "reciprocity",
                    "immediate barter within a small group",
                ],
                can_do: &["move a surplus to somebody who needs it, at the price of a relationship"],
                held_back_by: &["no real market", "no standard pricing", "tied to personal relationships"],
                standing: Standing::Stands(
                    "bartering a surplus, gated on trust rather than on proximity, and a \
                     relationship that a bad trade costs",
                ),
                born_knowing: &[],
            },

            Path::GovernanceAndLabour => Stage {
                number: 0,
                called: "household and kin authority",
                starts_with: &[
                    "family heads",
                    "elders",
                    "informal prestige-based leadership",
                    "consensus decision-making",
                    "ad hoc cooperation for hunting, shelter and defence",
                ],
                can_do: &["a household that feeds its children", "a hunt more than one person joins"],
                held_back_by: &[
                    "small scale only",
                    "authority is personal, not institutional",
                    "weak coordination of unrelated households",
                ],
                standing: Standing::Part(
                    "households, kinship and feeding children from a parent's stores are \
                     all here; there is no leader, and nothing anybody can tell anybody to do",
                ),
                born_knowing: &[],
            },

            Path::Communication => Stage {
                number: 0,
                called: "direct speech and local signalling",
                starts_with: &[
                    "speech",
                    "gestures",
                    "shouting",
                    "drumming or striking objects",
                    "firelight and torch visibility at short range",
                ],
                can_do: &["ask somebody what they know", "be overheard", "be lied to"],
                held_back_by: &["short range", "no persistence", "heavily terrain dependent"],
                standing: Standing::Stands(
                    "asking, answering, earshot, and a claim that can be a lie and can be \
                     caught being one",
                ),
                born_knowing: &[],
            },

            Path::WritingAndCounting => Stage {
                number: 0,
                called: "oral memory and direct witness",
                starts_with: &[
                    "verbal agreement",
                    "memory-based genealogy and obligation",
                    "physical witnessing by the group",
                    "marks of ownership based on recognition rather than notation",
                ],
                can_do: &["remember who did what", "know whose thing this is by knowing whose it was"],
                held_back_by: &["fragile memory", "short-range administration", "poor scalability"],
                standing: Standing::Stands(
                    "memory that fades at a rate set by how the thing was learned, lessons \
                     keyed on a situation, and ownership held as recognition and nothing else",
                ),
                born_knowing: &[],
            },

            Path::MeasurementAndStandards => Stage {
                number: 0,
                called: "relative and body-based measures",
                starts_with: &[
                    "a handful",
                    "a basketful",
                    "an arm's length",
                    "a footstep",
                    "a finger width",
                    "a day's walk",
                    "enough for a meal",
                ],
                can_do: &["reckon a store in days rather than in units"],
                held_back_by: &["highly variable", "hard to scale beyond local trust"],
                standing: Standing::Part(
                    "the larder reckoning counts days of food and a pack counts what it \
                     holds; both are exact numbers nobody could actually have measured",
                ),
                born_knowing: &[],
            },

            Path::WasteAndSanitation => Stage {
                number: 0,
                called: "immediate disposal and neglect",
                starts_with: &[
                    "waste dropped away from the sleeping area",
                    "refuse left at the camp edge",
                    "human waste in open ground",
                    "carcasses abandoned or scavenged",
                    "dirty water dumped nearby",
                ],
                can_do: &["keep the worst of it out of the sleeping place"],
                held_back_by: &[
                    "high contamination risk",
                    "insects and vermin",
                    "foul odour",
                    "water fouling",
                    "high disease burden in repeated-use camps",
                ],
                standing: Standing::Stands(
                    "waste that can be smelled, avoided and learned from, and ground that \
                     takes what is dropped on it back into the soil",
                ),
                born_knowing: &[],
            },

            // ---- getting food ---------------------------------------------
            Path::Agriculture => Stage {
                number: 0,
                called: "gathering and opportunistic tending",
                starts_with: &[
                    "wild seed, fruit, nut, tuber and greens collection",
                    "seasonal return to productive patches",
                    "accidental reseeding near camps",
                    "protection of useful wild plants from trampling or fire",
                ],
                can_do: &["low labour input", "high ecological dependence", "limited predictability"],
                held_back_by: &["seasonal scarcity", "unstable yields", "limited storage surplus"],
                standing: Standing::Stands(
                    "foraging on a bearing year, a remembered patch to go back to, grain \
                     thrown out that comes up, and farming proper as a discovery past it",
                ),
                born_knowing: &[],
            },

            Path::FishingAndAquaculture => Stage {
                number: 0,
                called: "opportunistic capture",
                starts_with: &[
                    "hand gathering of shellfish",
                    "spearing fish in shallow water",
                    "collecting stranded fish",
                    "crude clubs, baskets and digging tools for aquatic harvest",
                ],
                can_do: &["crabs, molluscs, frogs, turtles and aquatic plants"],
                held_back_by: &["very local", "labour-intensive", "seasonal", "low output"],
                standing: Standing::Part(
                    "a rod is obvious, the run is on the calendar and a net is a discovery; \
                     there is no shellfish and no wading, so a river is a thing you fish and \
                     not a thing you pick over",
                ),
                born_knowing: &["fishingrod"],
            },

            Path::AnimalHusbandry => Stage {
                number: 0,
                called: "opportunistic agent-animal relationship",
                starts_with: &[
                    "hunting pressure shapes prey behaviour",
                    "camp-following animals exploit waste",
                    "people protect or cull animals informally",
                ],
                can_do: &["proto-dogs from scavenging canids", "managed tolerance of semi-wild herds"],
                held_back_by: &["nothing is owned", "nothing is bred", "nothing is fed"],
                standing: Standing::Part(
                    "hunting pressure does shape prey - animals grow shy where they are \
                     hunted - and nothing follows a camp, scavenges waste, or is tolerated",
                ),
                born_knowing: &[],
            },

            // ---- tools and what is done with them --------------------------
            Path::Tools => Stage {
                number: 0,
                called: "unshaped or minimally shaped stone tools",
                starts_with: &[
                    "hammerstones",
                    "hand axes",
                    "choppers",
                    "digging sticks",
                    "bone splinters",
                    "wooden clubs",
                ],
                can_do: &["cutting", "crushing", "scraping", "digging"],
                held_back_by: &["blunt", "wears out fast", "made badly by hands that have not practised"],
                standing: Standing::Stands(
                    "knapping ordinary stone, a hand axe, a stone knife and a shovel are \
                     all obvious; knapping flint proper is the discovery past them",
                ),
                born_knowing: &["knappedtip", "handaxe", "stoneknife", "shovel"],
            },

            Path::Weapons => Stage {
                number: 0,
                called: "simple impact weapons",
                starts_with: &[
                    "wooden clubs",
                    "throwing sticks",
                    "stones",
                    "hardened wooden spears",
                ],
                can_do: &["simple, low-skill production", "effective at close range"],
                held_back_by: &["close range only", "nothing that kills at a distance"],
                standing: Standing::Stands(
                    "a sharpened stick and a stone-tipped spear are obvious and a bow is a \
                     discovery, which is the whole of the Stage 0 limit spelled as a flag",
                ),
                born_knowing: &["sharpenedstick", "spear", "sling"],
            },

            Path::Metalworking => Stage {
                number: 0,
                called: "pre-metal use",
                starts_with: &[
                    "stone, bone, antler and wood tools",
                    "occasional use of native metal as ornament",
                ],
                can_do: &["everything the stone tools do, and nothing else"],
                held_back_by: &["no smelting", "no alloy", "no forge"],
                standing: Standing::Stands(
                    "the whole metal chain - a shiny lump, a blade, a knife, an axe, a \
                     spear - is a discovery, and none of it is obvious",
                ),
                born_knowing: &[],
            },

            Path::AnimalPoweredAndMechanical => Stage {
                number: 0,
                called: "direct human labour only",
                starts_with: &[
                    "carrying",
                    "pulling",
                    "pushing",
                    "hand grinding",
                    "hand drilling",
                    "hand lifting",
                ],
                can_do: &["what a pair of arms can do in a day"],
                held_back_by: &["very low throughput", "high labour cost", "limited scale"],
                standing: Standing::Stands(
                    "every step in the making table costs effort out of a pair of hands and \
                     nothing else does any work anywhere in the model",
                ),
                born_knowing: &[],
            },

            // ---- where people live -----------------------------------------
            Path::Construction => Stage {
                number: 0,
                called: "temporary shelters",
                starts_with: &[
                    "lean-tos",
                    "windbreaks",
                    "brush huts",
                    "snow shelters in cold regions",
                    "skin tents",
                ],
                can_do: &["fast", "portable", "low skill requirement"],
                held_back_by: &["poor durability", "minimal insulation", "little storage or security"],
                standing: Standing::Stands(
                    "a shelter that is built out of what is to hand, that shelters against \
                     the exposure model, and that a people without timber digs instead",
                ),
                born_knowing: &[],
            },

            Path::ShelterSystems => Stage {
                number: 0,
                called: "minimal environmental protection",
                starts_with: &[
                    "a simple windbreak",
                    "one fire",
                    "a shared sleeping area",
                    "ground bedding",
                    "a basic roof cover",
                ],
                can_do: &["keep the weather off several people at once"],
                held_back_by: &[
                    "smoke accumulation",
                    "cold floors",
                    "dampness",
                    "poor privacy",
                    "pests",
                ],
                standing: Standing::Part(
                    "a shelter has a condition and keeps the weather off; there is no \
                     smoke, no floor, no damp and no ventilation inside it",
                ),
                born_knowing: &[],
            },

            Path::Masonry => Stage {
                number: 0,
                called: "dry stone use",
                starts_with: &[
                    "unmortared fieldstone walls",
                    "cairns",
                    "hearth rings",
                    "simple retaining walls",
                ],
                can_do: &["a wall that stands without anything holding it together"],
                held_back_by: &["no mortar", "low height", "nothing spans an opening"],
                standing: Standing::Short(
                    "stone is a material for tools and for building, and nothing is ever \
                     stacked: there is no wall, no cairn and no hearth ring",
                ),
                born_knowing: &[],
            },

            Path::DomesticGoods => Stage {
                number: 0,
                called: "minimal domestic equipment",
                starts_with: &[
                    "a bed of leaves, furs or grass",
                    "simple wraps",
                    "hand tools",
                    "gourds, shells, stones and sticks for serving or carrying",
                ],
                can_do: &["somewhere to sleep and something to eat out of"],
                held_back_by: &[
                    "little comfort",
                    "low cleanliness",
                    "high loss and breakage",
                    "poor indoor organisation",
                    "no dedicated furniture",
                ],
                standing: Standing::Part(
                    "a carved bowl, a woven basket and a sewn bag are all obvious; nothing \
                     furnishes a shelter, and sleeping is not a thing a place is better at",
                ),
                born_knowing: &["bowl", "basket", "leatherbag"],
            },

            Path::WaterSystems => Stage {
                number: 0,
                called: "direct natural access",
                starts_with: &[
                    "drinking from streams, springs, lakes and rain puddles",
                    "carrying water in skins, gourds, shells or lined baskets",
                ],
                can_do: &["drink where the water is", "carry a day's drink away from it"],
                held_back_by: &[
                    "high contamination risk",
                    "low carrying volume",
                    "settlement constrained by immediate water access",
                ],
                // The carved bowl is the only vessel a people actually gets,
                // and almost nobody makes one - see ISSUES_FOUND #292.
                standing: Standing::Part(
                    "rivers, springs and rain are all drinkable and salt water is worse \
                     than nothing; the carrying half is one carved bowl that almost nobody \
                     ever makes",
                ),
                born_knowing: &["bowl"],
            },

            Path::MedicineAndPublicHealth => Stage {
                number: 0,
                called: "folk care and survival responses",
                starts_with: &[
                    "rest",
                    "splinting with sticks",
                    "pressure on wounds",
                    "cooling and warming",
                    "ritual healing",
                    "empirical use of a few soothing plants",
                    "ad hoc isolation of the obviously sick",
                ],
                can_do: &["make an illness shorter, sometimes"],
                held_back_by: &[
                    "very high mortality",
                    "little anatomical understanding",
                    "no systematic sanitation",
                ],
                standing: Standing::Part(
                    "medicinal plants that do something, and illness that kills nine deaths \
                     in ten with nothing between the two - see ISSUES_FOUND #202",
                ),
                born_knowing: &[],
            },

            Path::DefensiveInfrastructure => Stage {
                number: 0,
                called: "avoidance and improvised defence",
                starts_with: &[
                    "choosing defensible camps",
                    "fire-hardened stakes",
                    "thorn barriers",
                    "night watches",
                    "hidden stores",
                    "using cliffs, rivers and dense woods",
                ],
                can_do: &["not be where the danger is"],
                held_back_by: &[
                    "temporary",
                    "weak against organised attack",
                    "little protection for a surplus",
                ],
                standing: Standing::Part(
                    "avoidance is the whole of it - fear, fleeing, and a remembered place \
                     that is dangerous; nothing is built, nobody keeps watch, and a store is \
                     not hidden",
                ),
                born_knowing: &[],
            },

            Path::MaritimeAndBoatbuilding => Stage {
                number: 0,
                called: "floating aids and improvised crossings",
                starts_with: &["swimming aids", "logs", "bundled reeds", "simple floats", "inflated skins"],
                can_do: &["get across something narrow"],
                held_back_by: &["very low capacity", "dangerous", "short-distance only"],
                standing: Standing::Short(
                    "water is something to drink and fish in and nothing crosses it: there \
                     is no swimming, no float and no crossing",
                ),
                born_knowing: &[],
            },
        }
    }
}

/// Every product a people is born knowing how to make, across all the paths.
///
/// The union of `born_knowing`. What it is for is the drift test: a name here
/// that `making.rs` still wants discovered is a disagreement between what the
/// stage table claims a people starts with and what the recipe tables let it
/// do, and one of the two is wrong.
pub fn everything_a_people_starts_knowing() -> std::collections::BTreeSet<&'static str> {
    EVERY_PATH
        .iter()
        .flat_map(|path| path.stage_zero().born_knowing.iter().copied())
        .collect()
}

/// How much of Stage 0 this world actually carries: whole, half, and none.
///
/// For counting, and for saying the count out loud in a report rather than
/// having to read thirty-five doc comments to find out.
pub fn how_much_of_stage_zero_stands() -> (usize, usize, usize) {
    let mut whole = 0;
    let mut half = 0;
    let mut none = 0;
    for path in EVERY_PATH {
        match path.stage_zero().standing {
            Standing::Stands(_) => whole += 1,
            Standing::Part(_) => half += 1,
            Standing::Short(_) => none += 1,
        }
    }
    (whole, half, none)
}

/// What a people cannot yet do at Stage 0 that the specification says it
/// should be able to.
///
/// The gap list, in one call. Everything `Part` or `Short`, with the reason.
pub fn what_is_missing_from_stage_zero() -> Vec<(Path, &'static str)> {
    EVERY_PATH
        .iter()
        .filter_map(|path| {
            let stage = path.stage_zero();
            match stage.standing {
                Standing::Stands(_) => None,
                Standing::Part(why) | Standing::Short(why) => Some((*path, why)),
            }
        })
        .collect()
}
