// src/world/resources.rs
//! Resource nodes and harvestable materials.

use serde::{Deserialize, Serialize};
use crate::environment::seasons::Season;
use crate::world::{Position, Soil, TerrainType};

/// Types of resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ResourceType {
    // === Basic Resources (Existing) ===
    Wood,
    Stone,
    Iron,
    Food, // Generic food (berries, generic edibles)
    Water, // Fresh water from rivers, wells, springs

    /// Something growing that nobody has tried yet.
    ///
    /// "A curious agent might taste a random plant. If the plant is edible,
    /// the agent survives and thrives. If the plant is toxic or inedible, the
    /// agent dies or starves." Whether a given kind of strange plant feeds you
    /// or kills you is a property of the world and is not written anywhere an
    /// agent can read. The only way to find out is for somebody to eat one.
    StrangePlant,

    /// Wild leafy greens and fresh shoots: what a hedgerow gives in spring.
    ///
    /// Thin stuff - almost no energy in it and a great deal of what a body
    /// needs a little of - and it is what there is to eat in the months when
    /// nothing has ripened yet.
    Greens,

    /// The first roots and pods to come on, which is what summer gives.
    ///
    /// Better than greens and nothing like a harvest.
    Roots,

    // === Raw Materials (Agricultural) ===
    Grain,      // Wheat, barley, etc. - for flour, bread, beer
    Flax,       // For linen, rope
    Herbs,      // For medicine, dyes
    Cotton,     // For cloth

    // === Raw Materials (Animal) ===
    Hides,      // Raw animal skins - for leather
    Wool,       // From sheep - for cloth
    Meat,       // Butchered meat
    Milk,       // For cheese, butter
    Fish,       // From fishing
    Honey,      // From beekeeping

    // === Raw Materials (Mineral) ===
    Clay,       // For bricks, pottery
    Sand,       // For glass
    Coal,       // For fuel/charcoal

    /// What is left where a shallow sea dried up, and what sits in rare seams
    /// in the hills. The one thing that will keep meat through a winter
    /// without a fire and without a week of sun, and until now there was none
    /// of it anywhere in this world - so `PreparationState::Salted` was
    /// written, tested and unreachable.
    Salt,

    // === Processed Materials ===
    Flour,      // Grain → Miller → Flour
    Leather,    // Hides → Tanner → Leather
    Cloth,      // Flax/Wool/Cotton → Weaver → Cloth
    Linen,      // Flax → Weaver → Linen (specific cloth)
    Glass,      // Sand → Glassblower → Glass
    Bricks,     // Clay → Brickmaker → Bricks
    Charcoal,   // Wood → Charcoal Maker → Charcoal
    Rope,       // Flax → Ropemaker → Rope
    Paper,      // Various → Papermaker → Paper
    Dye,        // Herbs → Dyer → Dye

    // === Finished Goods (Food) ===
    Bread,      // Flour → Baker → Bread
    Ale,        // Grain → Brewer → Ale
    Cheese,     // Milk → Cheesemaker → Cheese

    // === Finished Goods (Items) ===
    Clothing,   // Cloth → Tailor → Clothing
    Shoes,      // Leather → Cobbler → Shoes
    Tools,      // Wood + Iron → Carpenter/Blacksmith → Tools
    Weapons,    // Wood + Iron → Bowyer/Blacksmith → Weapons
    Armor,      // Leather/Iron → Leatherworker/Armorer → Armor
    Pottery,    // Clay → Potter → Pottery
    Furniture,  // Wood → Carpenter → Furniture
    Jewelry,    // Iron/Gold → Goldsmith → Jewelry
}

/// The stretch of the year a thing carries something worth taking.
///
/// Days of the year rather than seasons, because a season is ninety days and
/// a hedgerow is not in fruit for ninety days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bearing {
    /// Not a growing thing, so it has no season: a river does not stop being
    /// a river in February, and nor does a rock stop being a rock.
    NeverStops,

    /// Carries from one day of the year to another, both included, and
    /// nothing the rest of the year. A window that closes before it opens has
    /// run round the turn of the year, which is legal and is why this is not
    /// a range.
    Between { opens: u32, closes: u32 },
}

impl Bearing {
    /// A window written the way the calendar talks: from one part of a season
    /// to another.
    pub fn from(
        opens: (crate::environment::seasons::Season, crate::environment::seasons::PartOfSeason),
        closes: (crate::environment::seasons::Season, crate::environment::seasons::PartOfSeason),
    ) -> Self {
        use crate::environment::seasons::{first_day_of, last_day_of};
        Bearing::Between {
            opens: first_day_of(opens.0, opens.1),
            closes: last_day_of(closes.0, closes.1),
        }
    }

    /// Whether this day of the year falls inside the window.
    pub fn covers(&self, day_of_year: u32) -> bool {
        let day = day_of_year % crate::environment::seasons::DAYS_PER_YEAR;
        match *self {
            Bearing::NeverStops => true,
            Bearing::Between { opens, closes } if opens <= closes => {
                day >= opens && day <= closes
            }
            // Round the turn of the year
            Bearing::Between { opens, closes } => day >= opens || day <= closes,
        }
    }

    /// How many days of the year this window covers.
    pub fn how_many_days(&self) -> u32 {
        use crate::environment::seasons::DAYS_PER_YEAR;
        match *self {
            Bearing::NeverStops => DAYS_PER_YEAR,
            Bearing::Between { opens, closes } if opens <= closes => closes - opens + 1,
            Bearing::Between { opens, closes } => DAYS_PER_YEAR - opens + closes + 1,
        }
    }
}

impl ResourceType {
    /// How well this crop repays being sown rather than found.
    ///
    /// "They need to discover which plants are suitable for farming." Not every
    /// plant is. Grain was made out of a grass by people who kept putting the
    /// heaviest heads back in the ground, and it answers a plough with several
    /// times what it gives wild. A berry bush set in rows is still a berry
    /// bush. A stand of herbs is worth nothing more for being weeded.
    ///
    /// Nobody is told which is which. An agent sows what it has in its pack,
    /// walks back at harvest, and finds out.
    pub fn takes_to_the_plough(&self) -> f32 {
        match self {
            ResourceType::Grain => 3.0,
            ResourceType::Flax | ResourceType::Cotton => 1.6,
            ResourceType::Food => 1.15,
            ResourceType::Herbs => 1.0,
            _ => 1.0,
        }
    }

    /// The kind of thing a name asks for.
    ///
    /// The names are the ones an inventory carries and an `Action::Gather`
    /// asks by, so that a chain of makings can say "flax" and have the
    /// gathering path go and find some.
    pub fn called(name: &str) -> Option<Self> {
        Some(match name {
            "wood" => ResourceType::Wood,
            "stone" => ResourceType::Stone,
            "iron" => ResourceType::Iron,
            "food" => ResourceType::Food,
            "water" => ResourceType::Water,
            "flax" => ResourceType::Flax,
            "cotton" => ResourceType::Cotton,
            "hides" => ResourceType::Hides,
            "wool" => ResourceType::Wool,
            _ => return None,
        })
    }

    /// How strongly this gives itself away by smell where it lies untouched,
    /// as a fraction of an agent's full smelling range.
    ///
    /// Human noses are poor. Berries on the bush and standing grain are close
    /// to odourless until you are almost on top of them - they are found by
    /// looking, not by sniffing. Flesh carries further. Nothing raw on the
    /// ground competes with cooking or with rot, which are what a nose is
    /// actually good for.
    ///
    /// The list this was written as had drifted off the list of what food is,
    /// and the world only has one smell for a thing to eat: everything with a
    /// scent that is not water is given off as `ScentType::Food`. So it named
    /// **herbs**, which nobody in this model can eat, and did not name
    /// **greens or roots**, which are most of what anybody eats. A starving
    /// agent smelling herbs walks to them and gathers nothing, over and over,
    /// and never gets as far as deciding the country is finished with - which
    /// is ISSUES #229. Herbs no doubt smell of something; they do not smell of
    /// dinner, and until there is a scent for what they are they are better
    /// off smelling of nothing than of food that is not there.
    ///
    /// So this asks `is_it_food` rather than keeping a second list. What is
    /// left here is only how far a thing carries.
    pub fn raw_scent_strength(&self) -> f32 {
        // Damp ground and vegetation, faintly. Not food, and the one other
        // thing a nose is for in this world.
        if *self == ResourceType::Water {
            return 0.12;
        }

        // Wood, stone and ore have no smell worth the name
        if !self.is_it_food() {
            return 0.0;
        }

        match self {
            // Flesh gives itself away from further off
            ResourceType::Meat | ResourceType::Fish => 0.24,

            // Barely detectable: you have to be standing among them
            _ => 0.08,
        }
    }
    /// When this thing bears something a person can eat.
    ///
    /// Growth was seasonal from the beginning and what was *standing* was
    /// not, so a berry bush that had grown all summer still had its berries
    /// on it in February. A hedgerow does not work like that. It carries
    /// nothing at all for most of the year and then, for a few weeks,
    /// everything at once.
    ///
    /// That last sentence was written when a season was twenty-four days
    /// long, and the code under it never said it: bearing was a set of
    /// seasons, so a thing came on for the first day of a season and went
    /// over on the last. At ninety days to a season that is a three-month
    /// flat step, and it made a year of four long uniform blocks - three
    /// months of leaf, three months of leaf, three months of harvest, three
    /// months of nothing - which is not a year anybody has ever foraged in.
    ///
    /// A window is written in the vocabulary the calendar already keeps:
    /// early, deep and late, two weeks at each end of a season and eight in
    /// the middle. So a thing opens in late spring and closes in deep autumn
    /// and those are real dates, and a season can hold the end of one food
    /// and the beginning of another.
    ///
    /// The year, as this world keeps it:
    ///
    /// - **Greens** run the whole growing year, and are the thinnest thing
    ///   in it. There is always leaf while anything grows.
    /// - **Roots** open with the greens and run past them into early winter.
    ///   Last year's root in the hungry gap, this year's swollen root in
    ///   autumn, and the winter dig out of cold ground - which is what a
    ///   root is *for*, and why it is the food that ends the year.
    /// - **Fruit** comes on at midsummer, not in September. Three months of
    ///   high summer with nothing ripe on any bush was the plainest thing
    ///   wrong with the old table.
    /// - **Grain** is a harvest: late summer into deep autumn, and weeks
    ///   rather than a season.
    /// - **Winter**, past its first fortnight, gives nothing whatever. That
    ///   is the whole point of a store and it does not move.
    ///
    /// Anything that is not a growing thing - stone, clay, water - never
    /// bears, so it never stops.
    pub fn bearing_window(&self) -> Bearing {
        use crate::environment::seasons::PartOfSeason::{Deep, Early, Late};
        use crate::environment::seasons::Season::{Fall, Spring, Summer, Winter};

        match self {
            // Leaf and shoot come with the first warmth and go over with the
            // frosts. The longest window in the year and the thinnest food in
            // it: a body living on greens alone is eating four times the
            // volume for the same energy.
            ResourceType::Greens => Bearing::from((Spring, Early), (Fall, Deep)),

            // Cattail and dandelion are dug when the top growth is young and
            // the root still holds last year's store - which is exactly what
            // makes them worth digging before anything has ripened - and they
            // are dug again out of hard ground when there is nothing else.
            // What they ask for is legs: a root patch is dug out and does not
            // come back this year, so a people living on them moves on.
            ResourceType::Roots => Bearing::from((Spring, Early), (Winter, Early)),

            // Wild fruit: strawberry and the first soft fruit at midsummer,
            // then the autumn glut of bramble, elder, sloe and haw.
            ResourceType::Food => Bearing::from((Summer, Deep), (Fall, Late)),

            // A harvest, and everybody knows when it is
            ResourceType::Grain => Bearing::from((Summer, Late), (Fall, Deep)),

            // A colony has built something worth robbing by midsummer, and by
            // late autumn it is defended and dwindling
            ResourceType::Honey => Bearing::from((Summer, Deep), (Fall, Early)),

            // Fibre and physic are cut green, before the stem goes woody
            ResourceType::Flax | ResourceType::Cotton | ResourceType::Herbs => {
                Bearing::from((Spring, Deep), (Summer, Late))
            }

            // Nobody has any idea what these do, including when they bear
            ResourceType::StrangePlant => Bearing::from((Fall, Early), (Fall, Late)),

            // Everything that is not a growing thing. Wood off a standing
            // tree, stone out of the ground, water in a river, fish coming up
            // it: none of it bears, so none of it stops.
            _ => Bearing::NeverStops,
        }
    }

    /// Whether there is anything on it to take, on this day of the year.
    pub fn is_it_bearing(&self, day_of_year: u32) -> bool {
        self.bearing_window().covers(day_of_year)
    }

    /// Whether this is a thing that grows out of the ground, and so a thing
    /// the ground's condition has a say in.
    ///
    /// Every kind of resource there is.
    ///
    /// Wanted by anything that has to ask a question of the whole set - which
    /// day of the year anything is bearing, for one. The exhaustive match in
    /// `every_resource_is_listed` below fails to compile if a variant is added
    /// and not put here, so this cannot quietly fall behind the enum.
    pub fn all() -> [ResourceType; 43] {
        [
        ResourceType::Wood,
        ResourceType::Stone,
        ResourceType::Iron,
        ResourceType::Food,
        ResourceType::Water,
        ResourceType::StrangePlant,
        ResourceType::Greens,
        ResourceType::Roots,
        ResourceType::Grain,
        ResourceType::Flax,
        ResourceType::Herbs,
        ResourceType::Cotton,
        ResourceType::Hides,
        ResourceType::Wool,
        ResourceType::Meat,
        ResourceType::Milk,
        ResourceType::Fish,
        ResourceType::Honey,
        ResourceType::Clay,
        ResourceType::Sand,
        ResourceType::Coal,
        ResourceType::Salt,
        ResourceType::Flour,
        ResourceType::Leather,
        ResourceType::Cloth,
        ResourceType::Linen,
        ResourceType::Glass,
        ResourceType::Bricks,
        ResourceType::Charcoal,
        ResourceType::Rope,
        ResourceType::Paper,
        ResourceType::Dye,
        ResourceType::Bread,
        ResourceType::Ale,
        ResourceType::Cheese,
        ResourceType::Clothing,
        ResourceType::Shoes,
        ResourceType::Tools,
        ResourceType::Weapons,
        ResourceType::Armor,
        ResourceType::Pottery,
        ResourceType::Furniture,
        ResourceType::Jewelry,
        ]
    }

    /// How fast a patch of this comes back once something has been taken off
    /// it, in units per growing pass before the weather and the ground have
    /// their say. Nought means it does not come back at all.
    ///
    /// **This is the one owner of what renews.** `is_renewable` used to keep
    /// its own list of the same question and `remove_depleted_resources`
    /// leaned on it to decide what to delete off the map when it was emptied -
    /// and both lists had the same hole. Greens and Roots came in with the
    /// rebuilt bearing year and neither list learned about them, so **63.6% of
    /// the food on a map** - 3,308 units of greens and 1,550 of roots against
    /// 2,784 of fish, with no berries and no grain standing at the turn of the
    /// year - grew at nought a day *and was deleted from the world the moment
    /// somebody finished a patch*. The comment on
    /// `World::remove_depleted_resources` states the case against exactly what
    /// it was doing: "deleting it would make berry patches and fish runs
    /// single-use and drain the world of food permanently."
    ///
    /// Measured before: an empty map produced three units a day where a person
    /// eats 11.5, twelve founders ate the country from 7,641 units down to 886
    /// in a hundred days, and nine of the twelve were dead by the end of
    /// spring. It was never a winter problem.
    pub fn how_fast_it_comes_back(&self) -> f32 {
        match self {
            // Renewable resources
            ResourceType::Wood => 0.01,       // Trees grow slowly
            ResourceType::Food => 0.025,      // Berries and fruit, in their own time

            // Leaf, which is the quickest thing there is and the reason
            // there is anything to eat in April.
            //
            // This and `Roots` below were **not in this table at all**, and
            // fell through to `_ => 0.0` with the minerals. They are 63% of
            // the food on a map - 3,308 units of greens and 1,550 of roots
            // against 2,784 of fish and, at the turn of the year, no berries
            // and no grain standing at all - so nearly two thirds of what a
            // settlement lives on was a **stock that never came back**. Eaten
            // once and gone for good.
            //
            // Measured: a map with nobody on it produced 3 units a day where a
            // person eats 11.5, and twelve founders ate the country from 7,641
            // units down to 886 in a hundred days and went from twelve people
            // to two and a half doing it. That is not a winter problem and
            // never was; the ground simply did not grow anything.
            //
            // The cause is the one this project keeps finding: a hand-written
            // list that did not learn about a variant added elsewhere. Greens
            // and Roots came in with the bearing year - see
            // `ResourceType::bearing_window` - and this match was written
            // before them. `raw_scent_strength` had the same hole in the same
            // week. The guard is below in `every_food_grows_back`.
            ResourceType::Greens => 0.04,

            // And a root is a season's work, so slower than a berry.
            ResourceType::Roots => 0.02,

            ResourceType::StrangePlant => 0.025, // Whatever they are, they grow
            ResourceType::Grain => 0.015,     // Wild grain is thin stuff
            ResourceType::Herbs => 0.04,      // Herbs grow quickly
            ResourceType::Flax => 0.03,
            ResourceType::Cotton => 0.03,
            ResourceType::Honey => 0.02,      // Bees produce honey steadily

            // Slow renewable
            ResourceType::Fish => 0.02,       // Fish populations regenerate

            // Water is fed by what carries it, which is worked out from the
            // ground it sits on rather than from a flat rate - see
            // `water_inflow`.
            ResourceType::Water => 0.0,

            // Non-renewable (mineral resources don't regenerate)
            ResourceType::Stone |
            ResourceType::Iron |
            ResourceType::Clay |
            ResourceType::Sand |
            ResourceType::Coal => 0.0,

            // Processed/finished goods don't regenerate naturally
            _ => 0.0,
        }
    }

    /// Whether a person can eat this.
    ///
    /// The same six the decision layer forages for. It lived there as
    /// `edible_resources`, which is analytics-private, so anything outside
    /// that layer wanting to know what counts as food had to write its own
    /// list - and a second list is a list that drifts. What is edible is a
    /// fact about a resource, so it lives on the resource.
    pub fn is_it_food(&self) -> bool {
        matches!(
            self,
            ResourceType::Food
                | ResourceType::Grain
                | ResourceType::Greens
                | ResourceType::Roots
                | ResourceType::Fish
                | ResourceType::Meat
        )
    }

    /// A seam of clay does not care how rich the topsoil over it is; a
    /// hedgerow does.
    pub fn is_it_grown(&self) -> bool {
        matches!(
            self,
            ResourceType::Food
                | ResourceType::Grain
                | ResourceType::Greens
                | ResourceType::Roots
                | ResourceType::Herbs
                | ResourceType::Flax
                | ResourceType::Cotton
                | ResourceType::StrangePlant
                | ResourceType::Wood
        )
    }

    /// Whether an agent can eat this straight from the land.
    ///
    /// Which is [`ResourceType::is_it_food`] under another name. Both claimed
    /// to be the single answer to "is this food" and both wrote the six out by
    /// hand, so the promise was kept by nothing but the two of them happening
    /// to agree. Now one of them asks the other.
    pub fn is_edible(&self) -> bool {
        self.is_it_food()
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            // Something nobody has tried
            ResourceType::StrangePlant => '?',
            ResourceType::Greens => 'v',
            ResourceType::Roots => 'r',
            ResourceType::Salt => '*',

            // Basic
            ResourceType::Wood => 't',
            ResourceType::Stone => 's',
            ResourceType::Iron => 'i',
            ResourceType::Food => 'f',
            ResourceType::Water => 'w',

            // Agricultural
            ResourceType::Grain => 'g',
            ResourceType::Flax => 'x',
            ResourceType::Herbs => 'h',
            ResourceType::Cotton => 'c',

            // Animal
            ResourceType::Hides => 'H',
            ResourceType::Wool => 'W',
            ResourceType::Meat => 'm',
            ResourceType::Milk => 'M',
            ResourceType::Fish => '~',
            ResourceType::Honey => 'y',

            // Mineral
            ResourceType::Clay => 'C',
            ResourceType::Sand => 'd',
            ResourceType::Coal => 'o',

            // Processed
            ResourceType::Flour => 'F',
            ResourceType::Leather => 'L',
            ResourceType::Cloth => 'l',
            ResourceType::Linen => 'n',
            ResourceType::Glass => 'G',
            ResourceType::Bricks => 'B',
            ResourceType::Charcoal => 'k',
            ResourceType::Rope => 'r',
            ResourceType::Paper => 'p',
            ResourceType::Dye => 'D',

            // Finished Food
            ResourceType::Bread => 'b',
            ResourceType::Ale => 'a',
            ResourceType::Cheese => 'e',

            // Finished Items
            ResourceType::Clothing => 'T',
            ResourceType::Shoes => 'S',
            ResourceType::Tools => 'O',
            ResourceType::Weapons => 'w',
            ResourceType::Armor => 'A',
            ResourceType::Pottery => 'P',
            ResourceType::Furniture => 'R',
            ResourceType::Jewelry => 'J',
        }
    }

    /// Get color code for terminal rendering
    pub fn color_code(&self) -> &'static str {
        match self {
            ResourceType::StrangePlant => "\x1b[35m",  // Magenta: unknown
            ResourceType::Greens => "\x1b[92m",        // Bright green: new leaf
            ResourceType::Roots => "\x1b[33m",         // Yellow/brown
            ResourceType::Salt => "\x1b[97m",          // Bright white

            // Basic - Original colors
            ResourceType::Wood => "\x1b[33m",      // Yellow/Brown
            ResourceType::Stone => "\x1b[37;1m",   // Bright White
            ResourceType::Iron => "\x1b[90m",      // Dark Gray
            ResourceType::Food => "\x1b[92m",      // Bright Green
            ResourceType::Water => "\x1b[96m",     // Bright Cyan (water)

            // Agricultural - Green shades
            ResourceType::Grain => "\x1b[93m",     // Bright Yellow (wheat)
            ResourceType::Flax => "\x1b[36m",      // Cyan
            ResourceType::Herbs => "\x1b[32m",     // Green
            ResourceType::Cotton => "\x1b[97m",    // Bright White

            // Animal - Brown/Red shades
            ResourceType::Hides => "\x1b[33m",     // Yellow/Brown
            ResourceType::Wool => "\x1b[37m",      // White
            ResourceType::Meat => "\x1b[31m",      // Red
            ResourceType::Milk => "\x1b[97m",      // Bright White
            ResourceType::Fish => "\x1b[96m",      // Bright Cyan
            ResourceType::Honey => "\x1b[93m",     // Bright Yellow

            // Mineral - Gray/Brown shades
            ResourceType::Clay => "\x1b[33m",      // Yellow/Brown
            ResourceType::Sand => "\x1b[93m",      // Bright Yellow
            ResourceType::Coal => "\x1b[90m",      // Dark Gray

            // Processed - Varied colors
            ResourceType::Flour => "\x1b[97m",     // Bright White
            ResourceType::Leather => "\x1b[33m",   // Yellow/Brown
            ResourceType::Cloth => "\x1b[36m",     // Cyan
            ResourceType::Linen => "\x1b[37m",     // White
            ResourceType::Glass => "\x1b[96m",     // Bright Cyan
            ResourceType::Bricks => "\x1b[31m",    // Red
            ResourceType::Charcoal => "\x1b[90m",  // Dark Gray
            ResourceType::Rope => "\x1b[33m",      // Yellow/Brown
            ResourceType::Paper => "\x1b[97m",     // Bright White
            ResourceType::Dye => "\x1b[35m",       // Magenta

            // Finished Food - Warm colors
            ResourceType::Bread => "\x1b[33m",     // Yellow/Brown
            ResourceType::Ale => "\x1b[93m",       // Bright Yellow
            ResourceType::Cheese => "\x1b[93m",    // Bright Yellow

            // Finished Items - Various
            ResourceType::Clothing => "\x1b[36m",  // Cyan
            ResourceType::Shoes => "\x1b[33m",     // Yellow/Brown
            ResourceType::Tools => "\x1b[37m",     // White
            ResourceType::Weapons => "\x1b[37;1m", // Bright White
            ResourceType::Armor => "\x1b[37;1m",   // Bright White
            ResourceType::Pottery => "\x1b[33m",   // Yellow/Brown
            ResourceType::Furniture => "\x1b[33m", // Yellow/Brown
            ResourceType::Jewelry => "\x1b[93m",   // Bright Yellow
        }
    }


    /// Check if this is a raw/harvestable resource (found in world)
    pub fn is_harvestable(&self) -> bool {
        matches!(
            self,
            ResourceType::Wood | ResourceType::Stone | ResourceType::Iron | ResourceType::Food |
            ResourceType::Water | // Water from rivers, wells, springs
            ResourceType::Grain | ResourceType::Flax | ResourceType::Herbs | ResourceType::Cotton |
            ResourceType::Clay | ResourceType::Sand | ResourceType::Coal |
            ResourceType::Fish | ResourceType::Honey
        )
    }

    /// Whether this grows in water rather than out of the ground it sits beside.
    ///
    /// A fish is not grown from the bank it is caught on. It is grown in the
    /// sea and in the whole catchment above, and it swims into a settlement's
    /// reach under its own power. That makes a fishery the one food a
    /// settlement can take without the land paying for it - and, once the
    /// leavings go on a field, the one food that brings the land something
    /// from outside.
    ///
    /// Before this, fish drew nutrient out of the riverbank exactly as a crop
    /// draws it out of a field, which had the ground feeding the river.
    pub fn grows_in_water(&self) -> bool {
        matches!(self, ResourceType::Fish)
    }




    /// Check if this is food/consumable
    pub fn is_consumable(&self) -> bool {
        matches!(
            self,
            ResourceType::Food | ResourceType::Bread | ResourceType::Ale |
            ResourceType::Cheese | ResourceType::Meat | ResourceType::Fish | ResourceType::Honey |
            ResourceType::Greens | ResourceType::Roots
        )
    }

    /// Get category description
    pub fn category(&self) -> &'static str {
        match self {
            ResourceType::StrangePlant => "Unidentified",
            ResourceType::Wood | ResourceType::Stone | ResourceType::Iron | ResourceType::Food | ResourceType::Water => "Basic Resource",
            ResourceType::Grain | ResourceType::Flax | ResourceType::Herbs | ResourceType::Cotton => "Agricultural",
            ResourceType::Greens | ResourceType::Roots => "Agricultural",
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk => "Animal Product",
            ResourceType::Fish | ResourceType::Honey => "Animal Product",
            ResourceType::Clay | ResourceType::Sand | ResourceType::Coal | ResourceType::Salt => "Mineral",
            ResourceType::Flour | ResourceType::Leather | ResourceType::Cloth | ResourceType::Linen |
            ResourceType::Glass | ResourceType::Bricks | ResourceType::Charcoal | ResourceType::Rope |
            ResourceType::Paper | ResourceType::Dye => "Processed Material",
            ResourceType::Bread | ResourceType::Ale | ResourceType::Cheese => "Finished Food",
            ResourceType::Clothing | ResourceType::Shoes | ResourceType::Tools | ResourceType::Weapons |
            ResourceType::Armor | ResourceType::Pottery | ResourceType::Furniture | ResourceType::Jewelry => "Finished Good",
        }
    }
}

/// A resource node in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub position: Position,
    pub amount: u32,
    pub max_amount: u32,

    /// Fraction of a unit carried over between regeneration passes, so a
    /// trickle eventually amounts to something
    #[serde(default)]
    pub inflow_carried: f32,

    /// Which sort of thing this is, where the sort matters and the resource
    /// type does not say. Only strange plants use it: two patches of
    /// `StrangePlant` with different kinds are different plants, one of which
    /// may be supper and the other of which may not.
    #[serde(default)]
    pub kind: u8,

    /// What a spring puts out between one pass of the resource tick and the
    /// next, and the least that can be standing in it.
    ///
    /// Water is the one thing here that is a **flow and not a stock**. A
    /// spring does not hold a set amount of water: it recharges, steadily, out
    /// of a catchment that is not in this model, and what limits what you can
    /// draw from it in an afternoon is its rate. Twelve people cannot drain a
    /// decent spring, and this is the sentence in the code that says so.
    ///
    /// Kept on the node because the number is worked out from terrain in
    /// `World::regenerate_resources`, which knows what tile a source is
    /// standing on, and spent in `harvest`, which does not.
    ///
    /// Zero on everything that is not water, and on a water source nobody has
    /// regenerated yet, in which case it is simply not yet a floor.
    #[serde(default)]
    pub flow: f32,
}

impl ResourceNode {

    /// Put back what would not fit in somebody's pack.
    ///
    /// A harvest comes off the node before anything asks whether the person
    /// picking it has room, so this is how it goes back on. What you cannot
    /// carry stays where it fell - see ISSUES #165, which states the principle
    /// and never reached the gathering branch.
    pub fn put_it_back(&mut self, how_much: u32) {
        self.amount = (self.amount + how_much).min(self.max_amount);
    }
    pub fn new(resource_type: ResourceType, position: Position, amount: u32) -> Self {
        Self {
            resource_type,
            position,
            amount,
            max_amount: amount,
            inflow_carried: 0.0,
            kind: 0,
            flow: 0.0,
        }
    }

    /// The same, for one of the several sorts of strange plant
    pub fn of_kind(
        resource_type: ResourceType,
        position: Position,
        amount: u32,
        kind: u8,
    ) -> Self {
        Self {
            kind,
            ..Self::new(resource_type, position, amount)
        }
    }

    /// Harvest resource from this node
    pub fn harvest(&mut self, amount: u32) -> u32 {
        let harvested = amount.min(self.what_can_be_taken());
        self.amount -= harvested;
        harvested
    }

    /// How much of what is standing here can actually be taken away.
    ///
    /// All of it, for everything that is a stock. A berry patch stripped bare
    /// is bare, a seam mined out is mined out, and that is what those things
    /// are.
    ///
    /// Not water. A spring cannot be drunk below what it puts out, because
    /// what is standing in it is not a barrel of water but this pass's flow
    /// arriving - so drawing it down to nothing would be drawing tomorrow's
    /// water out of it today. Twelve people at a spring take twelve people's
    /// worth and the spring goes on running.
    ///
    /// Before this, a settlement drank eight of its twenty-one sources down to
    /// two units out of four hundred and left them there for the rest of the
    /// world's life. See ISSUES_FOUND #46 and #53.
    pub fn what_can_be_taken(&self) -> u32 {
        if self.resource_type != ResourceType::Water {
            return self.amount;
        }

        self.amount.saturating_sub(self.springline())
    }

    /// A drink taken from the flow itself, at a spring that is down to its
    /// springline.
    ///
    /// The pool is what has gathered; the springline is what is arriving. A
    /// man kneeling at a spring that is down to its springline is not looking
    /// at a dry hole - he is looking at water coming out of the ground - and
    /// what he does is drink it as it comes. So he gets his mouthful and the
    /// pool does not move.
    ///
    /// This is the difference between a spring having a *rate* and a spring
    /// having a *closing time*. Without it the flow model put the failure rate
    /// up by half a point on its own and left "Gather: Resource source was
    /// empty" as the fourth largest refusal in the model, which is a strange
    /// thing to be able to say about a running spring.
    pub fn a_mouthful_from_the_flow(&self) -> u32 {
        if self.resource_type != ResourceType::Water {
            return 0;
        }

        // A source that has run right down to nothing has nothing arriving
        // either - a frozen seep in February, say - and there is genuinely
        // no drink to be had there.
        u32::from(self.amount > 0 && self.flow >= 1.0)
    }

    /// What a water source keeps back, so that tomorrow's water is not drunk
    /// today.
    ///
    /// A source whose flow fills its whole bed between one pass and the next
    /// keeps back **nothing**, because there is nothing to protect: a reach of
    /// running water is full again by morning whatever was taken out of it.
    /// Getting this wrong is worth writing down - the first cut set the
    /// springline to the flow for every source, and a river's flow is larger
    /// than its bed, so **rivers became undrinkable** and the failure rate
    /// went up rather than down.
    ///
    /// A spring or a pool, whose flow is a fraction of its bed, keeps back one
    /// pass's worth. That is the sentence that says twelve people cannot drain
    /// a spring.
    fn springline(&self) -> u32 {
        let flow = self.flow.max(0.0);

        if flow >= self.max_amount as f32 {
            return 0;
        }

        flow as u32
    }

    /// Shed what is on it, because the season it bears in has gone by.
    ///
    /// Fruit falls, leaf goes over, and a seed head that nobody cut shatters.
    /// Always takes at least one, so a patch actually empties rather than
    /// creeping down by fractions for ever.
    pub fn what_it_carries_falls_off(&mut self, share: f32) {
        if self.amount == 0 {
            return;
        }

        let falling = ((self.amount as f32) * share).ceil() as u32;
        self.amount = self.amount.saturating_sub(falling.max(1));
    }

    /// Check if node is depleted
    pub fn is_depleted(&self) -> bool {
        self.amount == 0
    }

    /// Get percentage remaining
    pub fn percentage_remaining(&self) -> f32 {
        if self.max_amount == 0 {
            return 0.0;
        }
        (self.amount as f32 / self.max_amount as f32) * 100.0
    }

    /// Whether this resource regrows on its own once harvested
    pub fn is_renewable(&self) -> bool {
        // A river is not used up by the people drinking from it, and it is
        // fed by `water_inflow` rather than by growing, so it is the one
        // thing that renews without a growth rate.
        self.resource_type == ResourceType::Water
            || self.resource_type.how_fast_it_comes_back() > 0.0
    }

    /// Take in a fractional amount of water, carrying the remainder over.
    ///
    /// Inflow is a rate, not a whole number of units: a pool on open ground
    /// gains a tenth of a unit a pass and would otherwise never gain anything
    /// at all, because resource amounts are integers.
    pub fn take_inflow(&mut self, amount: f32) {
        if self.amount >= self.max_amount {
            self.inflow_carried = 0.0;
            return;
        }

        self.inflow_carried += amount;

        let whole = self.inflow_carried.floor();
        if whole >= 1.0 {
            self.inflow_carried -= whole;
            self.amount = (self.amount + whole as u32).min(self.max_amount);
        }
    }

    /// How readily a plant here can take up what is in the ground.
    ///
    /// This is what breaking ground actually buys. A field is weeded, watered
    /// and worked, so the crop on it gets at far more of what the soil holds
    /// than the same plant would growing wild - it reaches its natural best
    /// pace on ground that would only half feed a hedgerow. What it does not do
    /// is make a plant grow faster than its kind can grow: uptake enters the
    /// rate as a factor that is capped at one.
    pub fn uptake_multiplier(cultivated: bool) -> f32 {
        if cultivated {
            2.5
        } else {
            1.0
        }
    }

    /// The most this patch can be carrying at once, given how well fed the
    /// ground is.
    ///
    /// The other half of what a field buys: yield. Rich ground carries a
    /// heavier crop, thin ground a lighter one, and a field that has had muck
    /// spread on it carries more than one that has not.
    ///
    /// The floor used to be four tenths of the full crop, which meant ground
    /// worked down to nothing still nominally carried nearly half of what it
    /// had when it was rich. Over a long run that hid the cost of farming: a
    /// settlement's fields fell to a twentieth of their fertility while their
    /// stated yield fell by four per cent. A worked-out field now carries
    /// almost nothing, which is what a worked-out field does.
    pub fn standing_capacity(&self, fertility: f32) -> u32 {
        let share = Self::MIN_YIELD_SHARE
            + (1.0 - Self::MIN_YIELD_SHARE) * fertility.clamp(0.0, 1.0);
        ((self.max_amount as f32) * share).round() as u32
    }

    /// What ground with nothing left in it still carries: the odd volunteer
    /// plant living off what blows in, and not a crop.
    const MIN_YIELD_SHARE: f32 = 0.05;

    /// The same, for ground that has been broken and sown.
    ///
    /// Breaking ground buys uptake and it buys this, and nothing else: a field
    /// of grain stands far thicker than the same ground would carry wild,
    /// because it is all one plant and all of it wanted. What it does not buy
    /// is speed - a crop grows at the pace its kind grows at, worked or not.
    ///
    /// How much thicker depends on what was sown. This is where suitability
    /// tells: a field of grain carries three times what the ground would
    /// otherwise, a field of berry bushes barely more than the hedge it came
    /// out of. See [`ResourceType::takes_to_the_plough`].
    pub fn how_heavy_a_crop_it_carries(&self, fertility: f32, cultivated: bool) -> u32 {
        let standing = self.standing_capacity(fertility) as f32;

        if !cultivated {
            return standing.round() as u32;
        }

        (standing * self.resource_type.takes_to_the_plough()).round() as u32
    }

    /// How fast this water refills, given the ground it sits on and the
    /// weather over it.
    ///
    /// Water does not grow back the way berries do. A river carries it in from
    /// somewhere upstream and is effectively bottomless; a spring in the hills
    /// keeps giving whatever the weather does; a pool on open ground is
    /// standing water and lives on the rain. It used to regenerate at nothing
    /// at all and was not counted as renewable, so every drink took a unit out
    /// of the world for good and a lake drunk dry was deleted. A world lost
    /// more than half its water in fifteen thousand ticks.
    ///
    /// Returns units per regeneration pass, which runs every ten world ticks.
    /// What a reach of running water gives back in a pass: all of it.
    ///
    /// Not a number so much as a statement that a river is not a stock. It is
    /// larger than any water source's `max_amount`, so `take_inflow` fills the
    /// source and drops the rest.
    pub const WHATEVER_WAS_DRAWN: f32 = 1000.0;

    pub fn water_inflow(&self, terrain: TerrainType, precipitation: f32, freezing: bool) -> f32 {
        if self.resource_type != ResourceType::Water {
            return 0.0;
        }

        // What the ground itself brings. Water sources are scattered across
        // the map rather than sitting on water tiles - they are the streams,
        // springs and ponds of the country they are in, and what feeds them
        // depends on which.
        //
        // These are reckoned per pass of the resource tick, which comes round
        // once every ten ticks, and they have to be read against what a
        // settlement draws: a drink is a unit or two, and forty people drink
        // something like thirty units in the time between two passes. The
        // first cut of this had a spring giving back **1.5**, which is a
        // twentieth of that. Measured over six thousand ticks, eight of a
        // world's twenty-one sources were drawn down to 2 units out of four
        // hundred and stayed there, and "no water sources nearby" was the
        // single largest refusal in the model - a settlement standing in the
        // middle of its own dry springs, walking further every year for a
        // drink.
        //
        // A stream is a flow and not a stock. What limits what you can draw
        // from a spring in an afternoon is the spring's rate, and that rate is
        // why a village sits on one.
        let source = match terrain {
            // Running water: whatever is drawn is replaced from upstream.
            // The comment said this before the numbers did.
            TerrainType::Water | TerrainType::Riverbank => Self::WHATEVER_WAS_DRAWN,

            // Springs and snowmelt come off high ground, and will carry a
            // camp
            TerrainType::Mountain | TerrainType::Hills => 20.0,

            // Seeps and marsh hold what they get, which is less
            TerrainType::Wetland | TerrainType::Forest => 12.0,

            // Anywhere else it is standing water, and lives mostly on the sky
            _ => 6.0,
        };

        // Rain tops everything up; a dry spell is felt most by the pools
        let rain = precipitation.clamp(0.0, 1.0);
        let from_sky = rain * 6.0;

        // Frozen ground gives nothing up, and the rain falls as snow
        let flow = source + from_sky;
        if freezing {
            flow * 0.25
        } else {
            flow
        }
    }

    /// What the run brings into a reach of water, per pass of the resource
    /// tick (one pass every ten ticks, as `water_inflow` is also reckoned).
    ///
    /// Fish do not grow back the way a berry patch grows back. A berry patch
    /// regrows out of what is left of itself, in the ground it stands in, so
    /// fishing a reach out would end it the way harvesting a field to nothing
    /// ends the field. Fish arrive: they are spawned upstream and fed at sea,
    /// and they come back up the rivers under their own power whatever was
    /// taken out of this particular pool last year.
    ///
    /// So the run does not depend on how many are left here. That is what
    /// makes a fishery worth having and what makes it nearly inexhaustible:
    /// it is fed from outside the country a settlement can see, by water that
    /// is not the settlement's to use up. What bounds a catch is the season,
    /// the reach, and how many hours somebody is willing to stand in a river.
    ///
    /// The runs are the point of the year in a fishing people's calendar.
    /// Spring and autumn are heavy; high summer is thin because the run is
    /// past; winter is thinnest of all, and a frozen river gives up almost
    /// nothing.
    pub fn fish_run(&self, terrain: TerrainType, season: Season, freezing: bool) -> f32 {
        if !self.resource_type.grows_in_water() {
            return 0.0;
        }

        // How much water this reach connects to. A river carries a run; a
        // beach gets what comes along the shore; a pond gets whatever
        // wandered in.
        let reach = match terrain {
            TerrainType::Water | TerrainType::Riverbank => 1.0,
            TerrainType::Beach => 0.7,
            TerrainType::Wetland => 0.4,
            _ => 0.2,
        };

        // The run itself
        let run = match season {
            Season::Spring => 1.0,
            Season::Fall => 0.85,
            Season::Summer => 0.4,
            Season::Winter => 0.15,
        };

        let flow = Self::FISH_PER_PASS_AT_FULL_RUN * reach * run;

        if freezing {
            flow * 0.2
        } else {
            flow
        }
    }

    /// What a full spring run brings into one reach of river in one pass.
    ///
    /// Set so that a reach fished down to nothing is full again inside a year,
    /// most of it arriving in the two runs: a spring season of twenty-four days
    /// is twenty-eight or nine passes, which at this rate is a good half of
    /// what a reach holds. That is the shape of the thing - a river is empty
    /// enough to be worth nobody's time for most of the year and thick with
    /// fish twice in it, and a people who live on one arrange the rest of what
    /// they do around those two stretches.
    const FISH_PER_PASS_AT_FULL_RUN: f32 = 1.0;

    /// Regenerate resources based on climate and weather conditions
    /// Returns the amount regenerated
    pub fn regenerate(&mut self, temperature: f32, precipitation: f32, season_modifier: f32) -> u32 {
        self.regenerate_on(temperature, precipitation, season_modifier, false)
    }

    /// Regenerate, saying whether this is growing on broken ground
    pub fn regenerate_on(
        &mut self,
        temperature: f32,
        precipitation: f32,
        season_modifier: f32,
        cultivated: bool,
    ) -> u32 {
        let mut nowhere = Soil::for_terrain(TerrainType::Plains);
        self.regenerate_from_soil(
            temperature,
            precipitation,
            season_modifier,
            cultivated,
            &mut nowhere,
        )
    }

    /// Regenerate, drawing on the ground it is growing in.
    ///
    /// Growth used to be a number per species multiplied by the weather, with
    /// nothing taken out of the ground and nothing put back: a patch picked
    /// bare regrew as fast on bare rock as in river silt. What it can manage
    /// now is bounded by whichever of warmth, rain and nutrient is scarcest,
    /// and what it grows with, it takes.
    pub fn regenerate_from_soil(
        &mut self,
        temperature: f32,
        precipitation: f32,
        season_modifier: f32,
        cultivated: bool,
        soil: &mut Soil,
    ) -> u32 {
        self.regenerate_in_ground(
            temperature,
            precipitation,
            season_modifier,
            cultivated,
            soil,
        )
    }

    /// Regenerate, drawing on the ground, where `precipitation` is how wet
    /// that ground is rather than whether it happens to be raining.
    ///
    /// Plants drink from the soil, not from the sky directly. Passing the
    /// hour's rainfall in here meant every plant in the world was in drought on
    /// any day it was not actively raining, which cut growth to a fifth
    /// wherever a marsh and a dune were treated alike.
    pub fn regenerate_in_ground(
        &mut self,
        temperature: f32,
        precipitation: f32,
        season_modifier: f32,
        cultivated: bool,
        soil: &mut Soil,
    ) -> u32 {
        if self.amount >= self.how_heavy_a_crop_it_carries(soil.fertility(), cultivated) {
            return 0; // As heavy a crop as this ground will carry
        }

        // Base regeneration rate per tick (0-1 units).
        //
        // Wild food comes back slowly: a hedge of berries feeds a few people
        // and no more, which is what a settlement of a dozen lives on and what
        // a settlement of forty starves against. Ground that has been broken
        // and sown is a different matter - see `cultivated_multiplier`.
        // This is the flow, and the standing crop above is only a buffer on
        // top of it - which is why thinning the hedgerows by half, a world's
        // edible standing crop from 7,413 to 3,944, changed **nothing
        // measurable** at thirty-two worlds a side. A patch tops out sooner
        // and goes on producing at the same pace.
        //
        // Halving these numbers as well was measured and **reverted**. It did
        // not make a winter bite: the population did not move, and efficiency
        // went from 0.74 to 0.70 (t = -3.0) with more rotting in packs and
        // more left on the ground. Scarcer food did not make a settlement
        // careful, it made it range further and hoard worse. The waste in this
        // model is a behaviour and not a supply artefact, and starving people
        // does not fix a behaviour. See ISSUES_FOUND #57.
        let base_rate = self.resource_type.how_fast_it_comes_back();

        if base_rate == 0.0 {
            return 0;
        }

        // Apply temperature modifier (most resources prefer moderate temps)
        let temp_modifier = match self.resource_type {
            ResourceType::Food
            | ResourceType::Grain
            | ResourceType::Greens
            | ResourceType::Roots
            | ResourceType::Herbs
            | ResourceType::StrangePlant => {
                // Plants prefer 15-25°C
                if temperature >= 15.0 && temperature <= 25.0 {
                    1.5 // Ideal conditions
                } else if temperature >= 5.0 && temperature <= 35.0 {
                    1.0 // Acceptable
                } else if temperature < -10.0 || temperature > 40.0 {
                    0.1 // Extreme temps slow growth severely
                } else {
                    0.5 // Suboptimal
                }
            },
            ResourceType::Wood => {
                // Trees are hardier
                if temperature >= -5.0 && temperature <= 30.0 {
                    1.0
                } else {
                    0.3
                }
            },
            ResourceType::Cotton => {
                // Cotton prefers warmer climates
                if temperature >= 20.0 && temperature <= 30.0 {
                    1.5
                } else if temperature >= 15.0 {
                    1.0
                } else {
                    0.3
                }
            },
            _ => 1.0, // No temperature preference
        };

        // Apply precipitation modifier (water availability)
        let precip_modifier = match self.resource_type {
            ResourceType::Food
            | ResourceType::Grain
            | ResourceType::Greens
            | ResourceType::Roots
            | ResourceType::Herbs
            | ResourceType::Flax => {
                // Most crops need moderate precipitation
                if precipitation >= 0.4 && precipitation <= 0.8 {
                    1.5 // Good rainfall
                } else if precipitation >= 0.2 {
                    1.0 // Adequate
                } else if precipitation < 0.1 {
                    0.2 // Drought
                } else {
                    0.7 // Too dry or too wet
                }
            },
            ResourceType::Wood => {
                // Trees need regular water
                if precipitation >= 0.3 {
                    1.2
                } else {
                    0.5
                }
            },
            ResourceType::Cotton => {
                // Cotton prefers drier conditions
                if precipitation >= 0.2 && precipitation <= 0.5 {
                    1.3
                } else if precipitation > 0.8 {
                    0.6 // Too wet
                } else {
                    0.8
                }
            },
            _ => 1.0,
        };

        // What the ground can give, and how well this plant can get at it.
        // Capped at one: uptake helps a crop reach the best pace its kind is
        // capable of, it does not carry it past that.
        let nutrient_factor =
            (soil.fertility() * Self::uptake_multiplier(cultivated)).clamp(0.0, 1.0);

        if nutrient_factor <= 0.0 {
            return 0;
        }

        // And what is taking it before the farmer does. A field is the best
        // ground there is and everything else knows it: what a crop keeps is
        // what the weeds and the vermin leave. On unbroken ground this is one -
        // a meadow cannot get any weedier than it already is.
        let kept = if cultivated {
            soil.what_the_crop_keeps()
        } else {
            1.0
        };


        // Calculate total regeneration
        let regen_amount = base_rate
            * temp_modifier
            * precip_modifier
            * season_modifier
            * nutrient_factor
            * kept;

        // Carry the fraction over rather than rounding it away: wild food
        // regrows slowly enough that rounding to whole units each pass loses most
        // of it
        self.inflow_carried += regen_amount * 100.0;
        let regen_units = self.inflow_carried.floor();
        self.inflow_carried -= regen_units;

        // Add regenerated amount, capped at what this ground will carry
        let capacity = self.how_heavy_a_crop_it_carries(soil.fertility(), cultivated);
        let headroom = capacity.saturating_sub(self.amount);
        let actual_regen = (regen_units as u32).min(headroom);
        self.amount += actual_regen;

        // What grew, came out of the ground - and most of the plant stays in
        // the ground it grew in. Roots, stalk and leaf go back into this same
        // tile; only the part somebody carries away is gone from it.
        //
        // What grew in the water is a different matter: it takes nothing from
        // the bank and leaves nothing on it.
        if actual_regen > 0 && !self.resource_type.grows_in_water() {
            soil.draw(actual_regen as f32 * Soil::NUTRIENT_PER_UNIT_GROWN);
            soil.add_leaf_litter(actual_regen as f32 * Soil::RESIDUE_PER_UNIT_GROWN);
        }

        actual_regen
    }
}

/// Resource for tracking what's needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl Resource {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self {
            resource_type,
            amount,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_node_creation() {
        let pos = Position::new(5, 5);
        let node = ResourceNode::new(ResourceType::Wood, pos, 100);

        assert_eq!(node.resource_type, ResourceType::Wood);
        assert_eq!(node.position, pos);
        assert_eq!(node.amount, 100);
        assert_eq!(node.max_amount, 100);
    }

    #[test]
    fn test_resource_harvest() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Wood, pos, 100);

        let harvested = node.harvest(30);
        assert_eq!(harvested, 30);
        assert_eq!(node.amount, 70);

        // Try to harvest more than available
        let harvested = node.harvest(100);
        assert_eq!(harvested, 70); // Only 70 left
        assert_eq!(node.amount, 0);
        assert!(node.is_depleted());
    }

    #[test]
    fn test_resource_percentage() {
        let pos = Position::new(5, 5);
        let mut node = ResourceNode::new(ResourceType::Stone, pos, 100);

        assert!((node.percentage_remaining() - 100.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 50.0).abs() < 0.1);

        node.harvest(50);
        assert!((node.percentage_remaining() - 0.0).abs() < 0.1);
    }
}

#[cfg(test)]
mod all_resources_tests {
    use super::ResourceType;

    /// `ResourceType::all()` has to list every variant, and nothing but a
    /// match the compiler checks can promise that. Adding a variant to the
    /// enum without adding it to `all()` fails to compile here.
    #[test]
    fn every_resource_is_listed() {
        fn exhaustive(what: ResourceType) {
            match what {
            ResourceType::Wood => {}
            ResourceType::Stone => {}
            ResourceType::Iron => {}
            ResourceType::Food => {}
            ResourceType::Water => {}
            ResourceType::StrangePlant => {}
            ResourceType::Greens => {}
            ResourceType::Roots => {}
            ResourceType::Grain => {}
            ResourceType::Flax => {}
            ResourceType::Herbs => {}
            ResourceType::Cotton => {}
            ResourceType::Hides => {}
            ResourceType::Wool => {}
            ResourceType::Meat => {}
            ResourceType::Milk => {}
            ResourceType::Fish => {}
            ResourceType::Honey => {}
            ResourceType::Clay => {}
            ResourceType::Sand => {}
            ResourceType::Coal => {}
            ResourceType::Salt => {}
            ResourceType::Flour => {}
            ResourceType::Leather => {}
            ResourceType::Cloth => {}
            ResourceType::Linen => {}
            ResourceType::Glass => {}
            ResourceType::Bricks => {}
            ResourceType::Charcoal => {}
            ResourceType::Rope => {}
            ResourceType::Paper => {}
            ResourceType::Dye => {}
            ResourceType::Bread => {}
            ResourceType::Ale => {}
            ResourceType::Cheese => {}
            ResourceType::Clothing => {}
            ResourceType::Shoes => {}
            ResourceType::Tools => {}
            ResourceType::Weapons => {}
            ResourceType::Armor => {}
            ResourceType::Pottery => {}
            ResourceType::Furniture => {}
            ResourceType::Jewelry => {}
            }
        }

        let all = ResourceType::all();
        for what in all {
            exhaustive(what);
        }

        let mut seen = all.to_vec();
        seen.sort_by_key(|what| format!("{what:?}"));
        seen.dedup();
        assert_eq!(seen.len(), all.len(), "a resource is listed twice in all()");
    }
}
