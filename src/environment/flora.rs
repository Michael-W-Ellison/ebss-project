// src/environment/flora.rs
//! Plant life and vegetation system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Climate zone classification (broad categories)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ClimateZone {
    Arctic,
    Temperate,
    Desert,
    Tropical,
}

/// Growth stage of a plant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthStage {
    Seedling,
    Growing,
    Mature,
    Flowering,
    Fruiting,
}

/// A plant species
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantSpecies {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Health/durability when harvesting
    pub health: f32,
    /// How long it takes to grow to maturity (ticks)
    pub growth_time: u32,
    /// Whether it regrows after harvest
    pub regrows: bool,
    /// Time to regrow if applicable
    pub regrow_time: u32,

    /// Primary biomes where this plant thrives
    pub primary_biomes: Vec<ClimateZone>,
    /// Secondary biomes where it can grow (lower yield)
    pub secondary_biomes: Vec<ClimateZone>,

    /// Materials dropped when harvested
    pub drops: Vec<PlantDrop>,

    /// Whether this is a tree
    pub is_tree: bool,
    /// Size category
    pub size: PlantSize,
}

impl PlantSpecies {
    /// How long one of these lives, in years.
    ///
    /// Worked out from what kind of thing it is rather than written out
    /// fifty-one times. Fifty-one hand-written numbers is fifty-one numbers to
    /// get wrong and then to let drift apart, which this document has a
    /// standing entry about; and what decides how long a plant lives is how
    /// big it gets and whether it is woody, which `size` and `is_tree` already
    /// say.
    ///
    /// The figures are the ones a field guide gives. A grass or a herb is one
    /// or two seasons. A bush is a few decades - a hazel or a bramble stool
    /// will go thirty years and a willow rather longer. A birch is eighty, an
    /// oak two or three hundred, and the very largest trees run into the
    /// better part of a thousand. The one this is plainly short on is the
    /// sequoia, which really does go two or three thousand years; eight
    /// hundred of them is three and a half million ticks, and a run that long
    /// is a long way past anything anybody has measured here.
    pub fn lives_for_years(&self) -> f32 {
        if self.is_tree {
            match self.size {
                PlantSize::Tiny | PlantSize::Small => 40.0,
                PlantSize::Medium => 80.0,
                PlantSize::Large => 250.0,
                PlantSize::Huge => 800.0,
            }
        } else {
            match self.size {
                // A grass, a herb, a corn: up in the spring and gone by the
                // second winter, whether or not anybody cut it.
                PlantSize::Tiny | PlantSize::Small => 2.0,
                // A bush.
                PlantSize::Medium => 30.0,
                PlantSize::Large | PlantSize::Huge => 60.0,
            }
        }
    }

    /// The same, in ticks, which is what a plant actually counts in.
    pub fn lives_for_ticks(&self) -> u32 {
        (self.lives_for_years() * crate::environment::seasons::TICKS_PER_YEAR as f32) as u32
    }

    /// How likely one of these is to put seed on the ground in a pass.
    ///
    /// Set by what it takes to replace itself. A grass has two years to leave
    /// a successor and an oak has two hundred and fifty, and they shed
    /// accordingly - the grass many times over in its short life, the oak a
    /// handful of times a century as far as seed that comes to anything goes.
    ///
    /// One flat rate for everything is what killed the grass. At a fixed
    /// chance per pass an oak seeded a hundred times over in its life and a
    /// grass a fraction of once, so every open sward on a map went to wood
    /// inside five years and the small plants were gone by year four.
    ///
    /// What the number is aimed at: a plant leaves a couple of dozen seed
    /// over its whole life, of which a small share on open ground take, so a
    /// species holding its ground replaces itself with a little to spare and
    /// one being crowded out goes. Ground that is already taken is what turns
    /// "a little to spare" into a population that stops climbing - see
    /// `PlantManager::how_likely_a_seed_takes`.
    pub fn seeds_per_pass(&self) -> f32 {
        /// Seed that arrives somewhere with a chance of coming to something,
        /// over a whole life. Not seed shed, which for a grass is thousands
        /// and nearly all of it eaten, on rock, or under the parent.
        ///
        /// It has to clear what the two filters downstream take. Counted over
        /// twenty years on a hundred and twenty by a hundred and twenty: two
        /// seed in five land on country of a kind their species cannot live
        /// on at all, and of the three that do land somewhere possible about
        /// one in twenty gets a root down. So a seed is worth about three
        /// hundredths of a plant, and a plant that leaves twenty-five of them
        /// leaves two-thirds of a successor - which is a species on its way
        /// out, and at twenty-five every class on the map was on its way out.
        /// The bushes went first, being neither long-lived enough to wait it
        /// out like a tree nor quick enough to flood the ground like a grass:
        /// a hundred and forty-five of them at the start and none at all by
        /// year a hundred and fifty.
        ///
        /// A hundred leaves about three, which is a margin rather than a
        /// knife edge. What brings three back down to one is ground that is
        /// already taken, and after that it is whatever is eating the
        /// seedlings.
        const WHAT_A_PLANT_LEAVES_IN_ITS_LIFE: f32 = 40.0;

        let passes = (self.lives_for_ticks() as f32 / 10.0).max(1.0);
        (WHAT_A_PLANT_LEAVES_IN_ITS_LIFE / passes).clamp(0.0, 1.0)
    }

    /// How long a seed of this will keep in the ground before it is no good.
    ///
    /// A seed that falls on ground its kind cannot live on does not sit there
    /// for ever waiting to be wrong: it rots. An acorn on a salt flat is
    /// finished by the following autumn. Small dry seed keeps far longer - a
    /// grass seed will lie in the soil through a couple of seasons and come up
    /// when something disturbs it - so the split is the same one that decides
    /// everything else here.
    pub fn seed_keeps_for_ticks(&self) -> u32 {
        use crate::environment::seasons::{DAYS_PER_SEASON, TICKS_PER_DAY};

        // A season for an acorn, two for small dry seed. This is seed lying
        // on ground of a kind its species cannot live on at all - a beach, a
        // mountain, a salt flat - not seed worked into good soil, so it is
        // the short end of what a seed keeps rather than the long one. It is
        // also what stops the bank being the biggest thing in the model: at a
        // year and two years there were three and a half seed lying for every
        // tile on the map, and walking them was most of what a tick cost.
        let seasons = if self.is_tree { 1 } else { 2 };
        seasons * DAYS_PER_SEASON * TICKS_PER_DAY
    }

    /// Whether this is ground one of these could live on at all.
    ///
    /// Primary country is where it thrives and secondary is where it manages,
    /// and a seed will take in either. Anywhere else it comes to nothing.
    pub fn could_live_on(&self, terrain: crate::world::TerrainType) -> bool {
        let zone = crate::environment::fauna::terrain_to_climate_zone(terrain);
        self.primary_biomes.contains(&zone) || self.secondary_biomes.contains(&zone)
    }

    /// How much of a place this takes, for deciding who gets the ground.
    ///
    /// A seedling of something bigger comes up through what is already
    /// standing and takes the tile - a sapling in a sward shades the sward
    /// out - and nothing smaller ever displaces something larger, however
    /// much seed it sheds.
    ///
    /// Without this the ground went to whoever shed the most seed, and that
    /// is whoever lives the shortest: one flat count of seed per lifetime
    /// makes seed per year the reciprocal of the lifetime, so a grass at two
    /// years puts out a hundred and fifty times what an oak at three hundred
    /// does. A hundred and twenty by a hundred and twenty went to pure grass
    /// in a century and a half, the last bush gone by year one hundred and
    /// fifty and the trees down a third and still falling. What actually
    /// stops that in a field is not seed at all, it is that the bramble wins.
    pub fn how_much_ground_it_claims(&self) -> u8 {
        let by_size = match self.size {
            PlantSize::Tiny => 0,
            PlantSize::Small => 1,
            PlantSize::Medium => 2,
            PlantSize::Large => 3,
            PlantSize::Huge => 4,
        };

        // Woody beats soft of the same bulk, and beats everything softer.
        if self.is_tree {
            by_size + 5
        } else {
            by_size
        }
    }

    /// And whether this is the country it actually belongs in.
    pub fn thrives_on(&self, terrain: crate::world::TerrainType) -> bool {
        self.primary_biomes
            .contains(&crate::environment::fauna::terrain_to_climate_zone(terrain))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantSize {
    Tiny,    // Moss, small herbs
    Small,   // Flowers, small bushes
    Medium,  // Large bushes, young trees
    Large,   // Mature trees
    Huge,    // Ancient trees
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantDrop {
    pub material_id: String,
    pub min_quantity: u32,
    pub max_quantity: u32,
    /// Only available at certain growth stages
    pub required_stage: Option<GrowthStage>,
}

impl PlantDrop {
    pub fn new(material_id: String, min: u32, max: u32) -> Self {
        Self {
            material_id,
            min_quantity: min,
            max_quantity: max,
            required_stage: None,
        }
    }

    pub fn at_stage(mut self, stage: GrowthStage) -> Self {
        self.required_stage = Some(stage);
        self
    }
}

/// Plant species database
#[derive(Debug, Clone)]
pub struct FloraRegistry {
    species: BTreeMap<String, PlantSpecies>,
}

impl FloraRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            species: BTreeMap::new(),
        };

        registry.register_all_species();
        registry
    }

    fn register(&mut self, species: PlantSpecies) {
        self.species.insert(species.id.clone(), species);
    }

    pub fn get(&self, id: &str) -> Option<&PlantSpecies> {
        self.species.get(id)
    }

    pub fn get_by_biome(&self, biome: ClimateZone) -> Vec<&PlantSpecies> {
        self.species
            .values()
            .filter(|s| s.primary_biomes.contains(&biome) || s.secondary_biomes.contains(&biome))
            .collect()
    }

    pub fn all_species(&self) -> Vec<&PlantSpecies> {
        self.species.values().collect()
    }

    pub fn get_trees(&self) -> Vec<&PlantSpecies> {
        self.species.values().filter(|s| s.is_tree).collect()
    }

    pub fn get_crops(&self) -> Vec<&PlantSpecies> {
        self.species
            .values()
            .filter(|s| s.id.contains("wheat") || s.id.contains("barley") ||
                       s.id.contains("corn") || s.id.contains("rice") ||
                       s.id.contains("potato") || s.id.contains("carrot") ||
                       s.id.contains("onion") || s.id.contains("cabbage") ||
                       s.id.contains("tomato"))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.species.len()
    }

    fn register_all_species(&mut self) {
        // Trees (Basic)
        self.register(oak_tree());
        self.register(pine_tree());
        self.register(birch_tree());
        self.register(palm_tree());
        self.register(cactus());

        // Trees (Fruit)
        self.register(apple_tree());
        self.register(pear_tree());
        self.register(cherry_tree());
        self.register(orange_tree());
        self.register(banana_tree());
        self.register(olive_tree());

        // Trees (Tropical/Exotic)
        self.register(mahogany_tree());
        self.register(bamboo());
        self.register(mangrove_tree());

        // Trees (Ancient/Special)
        self.register(sequoia_tree());
        self.register(baobab_tree());

        // Fiber plants
        self.register(flax_plant());
        self.register(cotton_plant());
        self.register(hemp_plant());

        // Bushes and shrubs
        self.register(berry_bush());
        self.register(willow_shrub());
        self.register(tea_bush());
        self.register(coffee_bush());

        // Crops (Grains)
        self.register(wheat());
        self.register(barley());
        self.register(corn());
        self.register(rice());

        // Crops (Vegetables)
        self.register(potato());
        self.register(carrot());
        self.register(onion());
        self.register(cabbage());
        self.register(tomato());
        self.register(pumpkin());

        // Flowers and decorative
        self.register(rose());
        self.register(lavender());
        self.register(tulip());
        self.register(sunflower());

        // Grasses and herbs
        self.register(grass());
        self.register(medicinal_herb());
        self.register(mint());
        self.register(sage());

        // Medicinal/Alchemical
        self.register(aloe());
        self.register(ginseng());
        self.register(chamomile());
        self.register(mandrake());

        // Fungi
        self.register(mushroom());
        self.register(poisonous_mushroom());
        self.register(shelf_fungus());

        // Aquatic
        self.register(reeds());
        self.register(lotus());
        self.register(seaweed());
    }
}

// ============================================================================
// TREES
// ============================================================================

fn oak_tree() -> PlantSpecies {
    PlantSpecies {
        id: "oak_tree".to_string(),
        name: "Oak Tree".to_string(),
        description: "Sturdy hardwood tree with strong bark and dense wood".to_string(),
        health: 200.0,
        growth_time: 5000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 15, 25),
            PlantDrop::new("bark".to_string(), 8, 12),
            // The mast. An oak was standing timber and nothing else in this
            // model, which leaves out the single most important thing an oak
            // wood does for anybody living in it.
            PlantDrop::new("acorns".to_string(), 20, 60).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn pine_tree() -> PlantSpecies {
    PlantSpecies {
        id: "pine_tree".to_string(),
        name: "Pine Tree".to_string(),
        description: "Coniferous tree adapted to cold climates, provides resin".to_string(),
        health: 180.0,
        growth_time: 4500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Arctic, ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 12, 20),
            PlantDrop::new("bark".to_string(), 6, 10),
            PlantDrop::new("resin".to_string(), 2, 4),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn birch_tree() -> PlantSpecies {
    PlantSpecies {
        id: "birch_tree".to_string(),
        name: "Birch Tree".to_string(),
        description: "White-barked tree with flexible branches and medicinal bark".to_string(),
        health: 150.0,
        growth_time: 3500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Arctic, ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 10, 15),
            PlantDrop::new("bark".to_string(), 10, 15), // More bark than other trees
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn palm_tree() -> PlantSpecies {
    PlantSpecies {
        id: "palm_tree".to_string(),
        name: "Palm Tree".to_string(),
        description: "Tropical tree with fibrous bark and edible fruits".to_string(),
        health: 140.0,
        growth_time: 3000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("wood".to_string(), 8, 12),
            PlantDrop::new("plant_fiber".to_string(), 15, 20),
            PlantDrop::new("coconut".to_string(), 3, 6).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn cactus() -> PlantSpecies {
    PlantSpecies {
        id: "cactus".to_string(),
        name: "Cactus".to_string(),
        description: "Water-storing desert plant with thick fibrous interior".to_string(),
        health: 100.0,
        growth_time: 2000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Desert],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 8, 12),
            PlantDrop::new("cactus_water".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// FIBER PLANTS
// ============================================================================

fn flax_plant() -> PlantSpecies {
    PlantSpecies {
        id: "flax".to_string(),
        name: "Flax Plant".to_string(),
        description: "Slender plant with blue flowers, produces strong fibers for linen".to_string(),
        health: 20.0,
        growth_time: 180,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("flax_fiber".to_string(), 3, 5),
            PlantDrop::new("flax_seeds".to_string(), 1, 3).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn cotton_plant() -> PlantSpecies {
    PlantSpecies {
        id: "cotton".to_string(),
        name: "Cotton Plant".to_string(),
        description: "Produces soft, fluffy fibers ideal for comfortable clothing".to_string(),
        health: 25.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("cotton".to_string(), 4, 6).at_stage(GrowthStage::Fruiting),
            PlantDrop::new("cotton_seeds".to_string(), 2, 4).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn hemp_plant() -> PlantSpecies {
    PlantSpecies {
        id: "hemp".to_string(),
        name: "Hemp Plant".to_string(),
        description: "Hardy plant with strong fibers, grows quickly".to_string(),
        health: 30.0,
        growth_time: 150,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 5, 8),
            PlantDrop::new("hemp_seeds".to_string(), 2, 4).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// BUSHES AND SHRUBS
// ============================================================================

fn berry_bush() -> PlantSpecies {
    PlantSpecies {
        id: "berry_bush".to_string(),
        name: "Berry Bush".to_string(),
        description: "Thorny bush producing edible berries".to_string(),
        health: 40.0,
        growth_time: 500,
        regrows: true,
        regrow_time: 300,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("berries".to_string(), 5, 10).at_stage(GrowthStage::Fruiting),
            PlantDrop::new("plant_fiber".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn willow_shrub() -> PlantSpecies {
    PlantSpecies {
        id: "willow_shrub".to_string(),
        name: "Willow Shrub".to_string(),
        description: "Flexible branches useful for weaving and crafting".to_string(),
        health: 35.0,
        growth_time: 400,
        regrows: true,
        regrow_time: 250,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("willow_branches".to_string(), 3, 6),
            PlantDrop::new("plant_fiber".to_string(), 2, 3),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// GRASSES AND HERBS
// ============================================================================

fn grass() -> PlantSpecies {
    PlantSpecies {
        id: "grass".to_string(),
        name: "Grass".to_string(),
        description: "Common ground cover providing basic plant fibers".to_string(),
        health: 5.0,
        growth_time: 50,
        regrows: true,
        regrow_time: 30,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 1, 2),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn medicinal_herb() -> PlantSpecies {
    PlantSpecies {
        id: "medicinal_herb".to_string(),
        name: "Medicinal Herb".to_string(),
        description: "Aromatic plant with healing properties".to_string(),
        health: 10.0,
        growth_time: 120,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("medicinal_herbs".to_string(), 2, 4).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn mint() -> PlantSpecies {
    PlantSpecies {
        id: "mint".to_string(),
        name: "Mint".to_string(),
        description: "Refreshing aromatic herb that spreads easily".to_string(),
        health: 8.0,
        growth_time: 80,
        regrows: true,
        regrow_time: 60,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("mint_leaves".to_string(), 3, 6),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn sage() -> PlantSpecies {
    PlantSpecies {
        id: "sage".to_string(),
        name: "Sage".to_string(),
        description: "Woody herb with culinary and medicinal uses".to_string(),
        health: 12.0,
        growth_time: 150,
        regrows: true,
        regrow_time: 100,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("sage_leaves".to_string(), 2, 5),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// FRUIT TREES
// ============================================================================

fn apple_tree() -> PlantSpecies {
    PlantSpecies {
        id: "apple_tree".to_string(),
        name: "Apple Tree".to_string(),
        description: "Fruit-bearing tree producing crisp, sweet apples".to_string(),
        health: 160.0,
        growth_time: 4000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 10, 16),
            PlantDrop::new("apples".to_string(), 8, 15).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn pear_tree() -> PlantSpecies {
    PlantSpecies {
        id: "pear_tree".to_string(),
        name: "Pear Tree".to_string(),
        description: "Delicate fruit tree bearing sweet, juicy pears".to_string(),
        health: 155.0,
        growth_time: 3800,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 10, 15),
            PlantDrop::new("pears".to_string(), 6, 12).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn cherry_tree() -> PlantSpecies {
    PlantSpecies {
        id: "cherry_tree".to_string(),
        name: "Cherry Tree".to_string(),
        description: "Beautiful flowering tree with delicious fruit".to_string(),
        health: 145.0,
        growth_time: 3500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 8, 14),
            PlantDrop::new("cherries".to_string(), 10, 20).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn orange_tree() -> PlantSpecies {
    PlantSpecies {
        id: "orange_tree".to_string(),
        name: "Orange Tree".to_string(),
        description: "Citrus tree with fragrant flowers and vitamin-rich fruit".to_string(),
        health: 150.0,
        growth_time: 3600,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("wood".to_string(), 9, 14),
            PlantDrop::new("oranges".to_string(), 8, 16).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

fn banana_tree() -> PlantSpecies {
    PlantSpecies {
        id: "banana_tree".to_string(),
        name: "Banana Tree".to_string(),
        description: "Fast-growing tropical plant with large edible fruit clusters".to_string(),
        health: 120.0,
        growth_time: 2500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("plant_fiber".to_string(), 10, 15),
            PlantDrop::new("bananas".to_string(), 5, 10).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false, // Technically not a tree
        size: PlantSize::Medium,
    }
}

fn olive_tree() -> PlantSpecies {
    PlantSpecies {
        id: "olive_tree".to_string(),
        name: "Olive Tree".to_string(),
        description: "Hardy Mediterranean tree producing oil-rich fruit".to_string(),
        health: 180.0,
        growth_time: 4500,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("wood".to_string(), 12, 18),
            PlantDrop::new("olives".to_string(), 10, 20).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

// ============================================================================
// TROPICAL/EXOTIC TREES
// ============================================================================

fn mahogany_tree() -> PlantSpecies {
    PlantSpecies {
        id: "mahogany_tree".to_string(),
        name: "Mahogany Tree".to_string(),
        description: "Valuable hardwood tree with rich reddish-brown timber".to_string(),
        health: 250.0,
        growth_time: 6000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("mahogany_wood".to_string(), 20, 30),
            PlantDrop::new("bark".to_string(), 10, 15),
        ],
        is_tree: true,
        size: PlantSize::Huge,
    }
}

fn bamboo() -> PlantSpecies {
    PlantSpecies {
        id: "bamboo".to_string(),
        name: "Bamboo".to_string(),
        description: "Fast-growing woody grass with countless uses".to_string(),
        health: 80.0,
        growth_time: 500,
        regrows: true,
        regrow_time: 300,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("bamboo".to_string(), 8, 15),
            PlantDrop::new("bamboo_shoots".to_string(), 2, 4).at_stage(GrowthStage::Growing),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn mangrove_tree() -> PlantSpecies {
    PlantSpecies {
        id: "mangrove_tree".to_string(),
        name: "Mangrove Tree".to_string(),
        description: "Coastal tree with salt-tolerant roots that stabilize shorelines".to_string(),
        health: 170.0,
        growth_time: 4200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 12, 18),
            PlantDrop::new("mangrove_bark".to_string(), 8, 12),
        ],
        is_tree: true,
        size: PlantSize::Large,
    }
}

// ============================================================================
// ANCIENT/SPECIAL TREES
// ============================================================================

fn sequoia_tree() -> PlantSpecies {
    PlantSpecies {
        id: "sequoia_tree".to_string(),
        name: "Sequoia Tree".to_string(),
        description: "Massive ancient conifer, one of the largest living things".to_string(),
        health: 500.0,
        growth_time: 10000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 40, 60),
            PlantDrop::new("bark".to_string(), 20, 30),
            PlantDrop::new("resin".to_string(), 5, 10),
        ],
        is_tree: true,
        size: PlantSize::Huge,
    }
}

fn baobab_tree() -> PlantSpecies {
    PlantSpecies {
        id: "baobab_tree".to_string(),
        name: "Baobab Tree".to_string(),
        description: "Iconic African tree with massive water-storing trunk".to_string(),
        health: 400.0,
        growth_time: 8000,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Desert, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wood".to_string(), 30, 50),
            PlantDrop::new("baobab_fruit".to_string(), 10, 15).at_stage(GrowthStage::Fruiting),
            PlantDrop::new("plant_fiber".to_string(), 15, 25),
        ],
        is_tree: true,
        size: PlantSize::Huge,
    }
}

// ============================================================================
// BUSHES (ADDITIONAL)
// ============================================================================

fn tea_bush() -> PlantSpecies {
    PlantSpecies {
        id: "tea_bush".to_string(),
        name: "Tea Bush".to_string(),
        description: "Evergreen shrub whose leaves produce the world's most popular beverage".to_string(),
        health: 45.0,
        growth_time: 600,
        regrows: true,
        regrow_time: 200,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("tea_leaves".to_string(), 4, 8),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn coffee_bush() -> PlantSpecies {
    PlantSpecies {
        id: "coffee_bush".to_string(),
        name: "Coffee Bush".to_string(),
        description: "Tropical shrub bearing cherries containing stimulating coffee beans".to_string(),
        health: 50.0,
        growth_time: 700,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("coffee_beans".to_string(), 6, 12).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// CROPS - GRAINS
// ============================================================================

fn wheat() -> PlantSpecies {
    PlantSpecies {
        id: "wheat".to_string(),
        name: "Wheat".to_string(),
        description: "Staple grain crop, ground into flour for bread".to_string(),
        health: 15.0,
        growth_time: 250,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("wheat".to_string(), 8, 12).at_stage(GrowthStage::Mature),
            PlantDrop::new("straw".to_string(), 4, 6),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn barley() -> PlantSpecies {
    PlantSpecies {
        id: "barley".to_string(),
        name: "Barley".to_string(),
        description: "Hardy grain used for food, brewing, and animal feed".to_string(),
        health: 18.0,
        growth_time: 230,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("barley".to_string(), 8, 14).at_stage(GrowthStage::Mature),
            PlantDrop::new("straw".to_string(), 3, 5),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn corn() -> PlantSpecies {
    PlantSpecies {
        id: "corn".to_string(),
        name: "Corn".to_string(),
        description: "Tall grain plant producing large edible kernels on cobs".to_string(),
        health: 25.0,
        growth_time: 280,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("corn".to_string(), 4, 8).at_stage(GrowthStage::Mature),
            PlantDrop::new("corn_stalks".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn rice() -> PlantSpecies {
    PlantSpecies {
        id: "rice".to_string(),
        name: "Rice".to_string(),
        description: "Water-loving grain, staple food for billions".to_string(),
        health: 12.0,
        growth_time: 300,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("rice".to_string(), 10, 16).at_stage(GrowthStage::Mature),
            PlantDrop::new("rice_straw".to_string(), 3, 5),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// CROPS - VEGETABLES
// ============================================================================

fn potato() -> PlantSpecies {
    PlantSpecies {
        id: "potato".to_string(),
        name: "Potato".to_string(),
        description: "Underground tuber, highly nutritious and storable".to_string(),
        health: 20.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("potatoes".to_string(), 4, 8).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn carrot() -> PlantSpecies {
    PlantSpecies {
        id: "carrot".to_string(),
        name: "Carrot".to_string(),
        description: "Root vegetable rich in vitamins, stores well".to_string(),
        health: 12.0,
        growth_time: 150,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("carrots".to_string(), 3, 6).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn onion() -> PlantSpecies {
    PlantSpecies {
        id: "onion".to_string(),
        name: "Onion".to_string(),
        description: "Pungent bulb vegetable, essential flavoring and preservative".to_string(),
        health: 14.0,
        growth_time: 180,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("onions".to_string(), 2, 5).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn cabbage() -> PlantSpecies {
    PlantSpecies {
        id: "cabbage".to_string(),
        name: "Cabbage".to_string(),
        description: "Leafy vegetable forming dense heads, stores well".to_string(),
        health: 22.0,
        growth_time: 220,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("cabbage".to_string(), 1, 2).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn tomato() -> PlantSpecies {
    PlantSpecies {
        id: "tomato".to_string(),
        name: "Tomato".to_string(),
        description: "Juicy fruit (botanically) used as a vegetable, needs warmth".to_string(),
        health: 16.0,
        growth_time: 240,
        regrows: true,
        regrow_time: 80,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("tomatoes".to_string(), 3, 6).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn pumpkin() -> PlantSpecies {
    PlantSpecies {
        id: "pumpkin".to_string(),
        name: "Pumpkin".to_string(),
        description: "Large vine fruit, nutritious flesh and edible seeds".to_string(),
        health: 30.0,
        growth_time: 280,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("pumpkin".to_string(), 1, 2).at_stage(GrowthStage::Mature),
            PlantDrop::new("pumpkin_seeds".to_string(), 10, 20).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// FLOWERS AND DECORATIVE
// ============================================================================

fn rose() -> PlantSpecies {
    PlantSpecies {
        id: "rose".to_string(),
        name: "Rose".to_string(),
        description: "Beautiful flowering plant with thorny stems and fragrant petals".to_string(),
        health: 18.0,
        growth_time: 400,
        regrows: true,
        regrow_time: 150,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("rose_petals".to_string(), 4, 8).at_stage(GrowthStage::Flowering),
            PlantDrop::new("rose_hips".to_string(), 2, 4).at_stage(GrowthStage::Fruiting),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn lavender() -> PlantSpecies {
    PlantSpecies {
        id: "lavender".to_string(),
        name: "Lavender".to_string(),
        description: "Aromatic herb with calming purple flowers".to_string(),
        health: 14.0,
        growth_time: 350,
        regrows: true,
        regrow_time: 120,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        drops: vec![
            PlantDrop::new("lavender".to_string(), 3, 6).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn tulip() -> PlantSpecies {
    PlantSpecies {
        id: "tulip".to_string(),
        name: "Tulip".to_string(),
        description: "Elegant spring flower growing from bulbs".to_string(),
        health: 8.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("tulip_petals".to_string(), 2, 4).at_stage(GrowthStage::Flowering),
            PlantDrop::new("tulip_bulb".to_string(), 1, 1),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn sunflower() -> PlantSpecies {
    PlantSpecies {
        id: "sunflower".to_string(),
        name: "Sunflower".to_string(),
        description: "Tall plant with large yellow flowers that track the sun".to_string(),
        health: 28.0,
        growth_time: 250,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("sunflower_petals".to_string(), 8, 15).at_stage(GrowthStage::Flowering),
            PlantDrop::new("sunflower_seeds".to_string(), 20, 40).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

// ============================================================================
// MEDICINAL/ALCHEMICAL
// ============================================================================

fn aloe() -> PlantSpecies {
    PlantSpecies {
        id: "aloe".to_string(),
        name: "Aloe Vera".to_string(),
        description: "Succulent with gel-filled leaves, powerful healing properties".to_string(),
        health: 20.0,
        growth_time: 300,
        regrows: true,
        regrow_time: 150,
        primary_biomes: vec![ClimateZone::Desert, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("aloe_gel".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn ginseng() -> PlantSpecies {
    PlantSpecies {
        id: "ginseng".to_string(),
        name: "Ginseng".to_string(),
        description: "Rare root with potent medicinal and energizing properties".to_string(),
        health: 15.0,
        growth_time: 800,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("ginseng_root".to_string(), 1, 2).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn chamomile() -> PlantSpecies {
    PlantSpecies {
        id: "chamomile".to_string(),
        name: "Chamomile".to_string(),
        description: "Daisy-like flower with calming and medicinal properties".to_string(),
        health: 10.0,
        growth_time: 180,
        regrows: true,
        regrow_time: 90,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("chamomile_flowers".to_string(), 3, 6).at_stage(GrowthStage::Flowering),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn mandrake() -> PlantSpecies {
    PlantSpecies {
        id: "mandrake".to_string(),
        name: "Mandrake".to_string(),
        description: "Mystical root plant, powerful but dangerous in alchemy".to_string(),
        health: 18.0,
        growth_time: 600,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("mandrake_root".to_string(), 1, 1).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// FUNGI
// ============================================================================

fn mushroom() -> PlantSpecies {
    PlantSpecies {
        id: "mushroom".to_string(),
        name: "Mushroom".to_string(),
        description: "Common edible fungus found in forests".to_string(),
        health: 3.0,
        growth_time: 60,
        regrows: true,
        regrow_time: 40,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("mushrooms".to_string(), 1, 3),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn poisonous_mushroom() -> PlantSpecies {
    PlantSpecies {
        id: "poisonous_mushroom".to_string(),
        name: "Poisonous Mushroom".to_string(),
        description: "Toxic fungus with bright warning colors, dangerous but useful in alchemy".to_string(),
        health: 3.0,
        growth_time: 70,
        regrows: true,
        regrow_time: 50,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("poison_mushrooms".to_string(), 1, 2),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

fn shelf_fungus() -> PlantSpecies {
    PlantSpecies {
        id: "shelf_fungus".to_string(),
        name: "Shelf Fungus".to_string(),
        description: "Hardy bracket fungus growing on trees, useful for tinder and medicine".to_string(),
        health: 8.0,
        growth_time: 200,
        regrows: false,
        regrow_time: 0,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic, ClimateZone::Tropical],
        drops: vec![
            PlantDrop::new("tinder_fungus".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Tiny,
    }
}

// ============================================================================
// AQUATIC PLANTS
// ============================================================================

fn reeds() -> PlantSpecies {
    PlantSpecies {
        id: "reeds".to_string(),
        name: "Reeds".to_string(),
        description: "Tall wetland grasses useful for weaving and construction".to_string(),
        health: 12.0,
        growth_time: 150,
        regrows: true,
        regrow_time: 100,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        drops: vec![
            PlantDrop::new("reeds".to_string(), 4, 8),
            PlantDrop::new("plant_fiber".to_string(), 2, 4),
        ],
        is_tree: false,
        size: PlantSize::Medium,
    }
}

fn lotus() -> PlantSpecies {
    PlantSpecies {
        id: "lotus".to_string(),
        name: "Lotus".to_string(),
        description: "Sacred water plant with beautiful flowers and edible roots".to_string(),
        health: 15.0,
        growth_time: 250,
        regrows: true,
        regrow_time: 120,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        drops: vec![
            PlantDrop::new("lotus_petals".to_string(), 3, 6).at_stage(GrowthStage::Flowering),
            PlantDrop::new("lotus_root".to_string(), 1, 3).at_stage(GrowthStage::Mature),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

fn seaweed() -> PlantSpecies {
    PlantSpecies {
        id: "seaweed".to_string(),
        name: "Seaweed".to_string(),
        description: "Marine algae, nutritious and useful for fertilizer".to_string(),
        health: 8.0,
        growth_time: 100,
        regrows: true,
        regrow_time: 60,
        primary_biomes: vec![ClimateZone::Tropical, ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        drops: vec![
            PlantDrop::new("seaweed".to_string(), 3, 6),
        ],
        is_tree: false,
        size: PlantSize::Small,
    }
}

// ============================================================================
// PLANT INSTANCES AND MANAGEMENT
// ============================================================================

/// Individual plant instance in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plant {
    pub id: Uuid,
    pub species_id: String,
    pub position: (i32, i32),
    pub current_health: f32,
    pub max_health: f32,
    pub growth_stage: GrowthStage,
    pub growth_progress: f32, // 0.0 to 1.0 for current stage
    pub age_ticks: u32,
    pub is_harvestable: bool,
    pub has_been_harvested: bool,
    pub regrow_timer: u32,
    pub planted_by: Option<Uuid>, // Agent who planted it (for farming)
    pub is_cultivated: bool, // Whether it's a farm plant vs wild

    /// The tick this plant has been grown up to.
    ///
    /// Vegetation is not worked out every tick any more - most of it waits
    /// for its zone's turn, and only the ground somebody is standing on is
    /// asked oftener than that (see `PlantManager::grow_a_zone` and
    /// `grow_where_somebody_is`). So there are two paths that can grow the
    /// same plant, and the one thing they must not do is disagree about how
    /// long it has been growing. Neither of them is told how many ticks to
    /// stand for: each works it out from here and writes it back, so a plant
    /// grows exactly once for each tick that has passed however it is
    /// reached.
    #[serde(default)]
    pub grown_up_to: u32,

    /// What was standing over this plant when its zone last came round.
    ///
    /// Worked out fresh for the whole band on a zone pass and remembered
    /// here, so that a plant something is standing on can be brought up to
    /// date on its own without gathering the canopy for the whole map again.
    /// It is at most one zone pass out of date, and shade is a number that
    /// moves on the timescale of a tree growing.
    #[serde(default)]
    pub shade_on_it: f32,
}

/// What a plant has to work with where it is standing.
///
/// All three run 0.0 to 1.0. Growth takes the worst of them, not the average:
/// a plant with all the water and light in the world and nothing in the ground
/// is still a plant with nothing in the ground. This is the whole of why a
/// field is worth manuring and a hedgerow picked bare takes years to come back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrowingConditions {
    /// Rain, ground water, and what the terrain holds
    pub water: f32,
    /// Sun reaching this plant, which is mostly a question of what is standing
    /// over it
    pub light: f32,
    /// What the soil has to give
    pub nutrients: f32,
    /// How readily this plant can take up what is there. Broken ground and a
    /// tended crop take up more than the same plant would growing wild.
    pub uptake: f32,
}

impl GrowingConditions {
    /// Everything a plant could ask for, which is what an untended world used
    /// to give every plant unconditionally
    pub fn ideal() -> Self {
        Self {
            water: 1.0,
            light: 1.0,
            nutrients: 1.0,
            uptake: 1.0,
        }
    }

    /// The share of its natural best pace a plant grows at here.
    ///
    /// Liebig's rule: whatever is scarcest sets the pace, and nothing makes up
    /// for it. Uptake helps a plant get at what is there; it cannot conjure
    /// what is not, and it never carries growth past the natural maximum.
    pub fn growth_share(&self) -> f32 {
        let water = self.water.clamp(0.0, 1.0);
        let light = self.light.clamp(0.0, 1.0);
        let nutrients = (self.nutrients * self.uptake.max(0.0)).clamp(0.0, 1.0);

        water.min(light).min(nutrients)
    }

    /// How much nutrient a plant growing here draws out of the ground per tick
    pub fn draw_per_tick(&self) -> f32 {
        const APPETITE: f32 = 0.00015;

        APPETITE * self.uptake.max(0.0) * self.growth_share()
    }
}

impl Plant {
    /// Create a new plant instance
    pub fn new(species_id: String, position: (i32, i32)) -> Self {
        Self {
            id: crate::core::dice::name(),
            species_id,
            position,
            current_health: 0.0, // Set from species
            max_health: 0.0,
            growth_stage: GrowthStage::Seedling,
            growth_progress: 0.0,
            age_ticks: 0,
            is_harvestable: false,
            has_been_harvested: false,
            regrow_timer: 0,
            planted_by: None,
            is_cultivated: false,
            grown_up_to: 0,
            shade_on_it: 0.0,
        }
    }

    /// Initialize with species data
    pub fn with_species(mut self, species: &PlantSpecies) -> Self {
        self.max_health = species.health;
        self.current_health = species.health;
        self
    }

    /// Mark as cultivated/farmed
    pub fn cultivated(mut self, planter_id: Uuid) -> Self {
        self.is_cultivated = true;
        self.planted_by = Some(planter_id);
        self
    }

    /// Advance growth by one tick
    pub fn grow(&mut self, species: &PlantSpecies) -> bool {
        self.grow_in(species, GrowingConditions::ideal(), 1.0)
    }

    /// Grow, given what this plant actually has to work with.
    ///
    /// `grow` used to advance a plant one step per tick regardless of water,
    /// light or soil - the biome lists on every species were declared and never
    /// read - so a cactus grew as fast in a bog as an oak did, and neither ever
    /// took anything out of the ground.
    pub fn grow_in(
        &mut self,
        species: &PlantSpecies,
        conditions: GrowingConditions,
        ticks: f32,
    ) -> bool {
        // A plant gets older whether or not the ground lets it grow, and
        // whether or not it has been cut. This used to sit below the two
        // early returns and below the `share <= 0.0` one, so a plant on
        // ground too poor to grow on, or a coppiced stool waiting to come
        // back, did not age at all - which is most of why nothing in this
        // world had ever died of being old.
        self.age_ticks = self.age_ticks.saturating_add(ticks.max(0.0) as u32);

        if self.has_been_harvested && self.regrow_timer > 0 {
            self.regrow_timer = self.regrow_timer.saturating_sub(ticks.max(1.0) as u32);
            if self.regrow_timer == 0 {
                // Reset for regrowth
                self.growth_stage = GrowthStage::Seedling;
                self.growth_progress = 0.0;
                self.has_been_harvested = false;
                self.is_harvestable = false;
            }
            return false;
        }

        if self.has_been_harvested && !species.regrows {
            return false; // Dead plant
        }


        // Calculate growth for current stage, at whatever share of its natural
        // best pace this ground allows. Nothing here can exceed that pace: the
        // most a plant can do is grow as well as its kind grows.
        let share = conditions.growth_share();
        if share <= 0.0 {
            return false;
        }

        let stage_duration = self.stage_duration(species);
        self.growth_progress += share * ticks / stage_duration as f32;

        // As many stages as the time it stands for is worth, not one.
        //
        // A pass used to be ten ticks and could never carry a plant through
        // more than one stage, so advancing one and throwing the remainder
        // away cost nothing. A pass is up to fourteen hundred and forty ticks
        // now - see `PlantManager::grow_a_zone` - which is several stages for
        // anything quick, and discarding the rest would leave a grass stuck
        // one step short of bearing for ever.
        let mut fully_grown = false;
        while self.growth_progress >= 1.0 {
            self.growth_progress -= 1.0;
            match self.growth_stage {
                GrowthStage::Seedling => self.growth_stage = GrowthStage::Growing,
                GrowthStage::Growing => self.growth_stage = GrowthStage::Mature,
                GrowthStage::Mature => self.growth_stage = GrowthStage::Flowering,
                GrowthStage::Flowering => self.growth_stage = GrowthStage::Fruiting,
                GrowthStage::Fruiting => {
                    // The end of the ladder. What is left over is not growth
                    // any more, so it is dropped here rather than spun on.
                    self.growth_progress = 0.0;
                    self.is_harvestable = true;
                    fully_grown = true;
                    break;
                }
            }
        }

        if fully_grown {
            return true;
        }

        // Check if harvestable at current stage
        if self.growth_stage == GrowthStage::Mature || self.growth_stage == GrowthStage::Fruiting {
            self.is_harvestable = true;
        }

        false
    }

    fn stage_duration(&self, species: &PlantSpecies) -> u32 {
        // Divide total growth time among stages
        species.growth_time / 5
    }

    /// Harvest the plant
    pub fn harvest(&mut self, species: &PlantSpecies) -> Vec<PlantDrop> {
        if !self.is_harvestable {
            return Vec::new();
        }

        self.has_been_harvested = true;

        if species.regrows {
            self.regrow_timer = species.regrow_time;
        }

        // Return drops that are available at current stage
        species.drops
            .iter()
            .filter(|drop| {
                drop.required_stage.is_none() ||
                drop.required_stage == Some(self.growth_stage)
            })
            .cloned()
            .collect()
    }

    /// Damage the plant
    pub fn damage(&mut self, amount: f32) -> bool {
        self.current_health -= amount;
        self.current_health <= 0.0
    }

    /// Get plant status string
    pub fn status(&self) -> String {
        if self.has_been_harvested {
            if self.regrow_timer > 0 {
                format!("Harvested (regrows in {} ticks)", self.regrow_timer)
            } else {
                "Dead".to_string()
            }
        } else {
            format!("{:?} ({:.0}% through stage)", self.growth_stage, self.growth_progress * 100.0)
        }
    }
}

/// A tally of what has happened to the vegetation, for measuring only.
///
/// Each figure is counted against the plant's own size class, because the
/// question that keeps coming up is not how many plants there are but which
/// kind is losing ground and at what step: seed that never fell, seed that
/// fell on the wrong country, seed that lost its one throw, or plants that
/// something bigger came up through.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlantLedger {
    pub seed_dropped: [u64; 3],
    pub seed_took: [u64; 3],
    pub seed_lost_its_throw: [u64; 3],
    pub seed_rotted_on_wrong_ground: [u64; 3],
    pub died_of_age: [u64; 3],
    pub died_of_the_ground: [u64; 3],
    pub shaded_out: [u64; 3],
}

impl PlantLedger {
    /// Small, woody-and-middling, or tree - the three the tallies are kept in.
    pub fn which_class(species: &PlantSpecies) -> usize {
        if species.is_tree {
            2
        } else if matches!(
            species.size,
            PlantSize::Medium | PlantSize::Large | PlantSize::Huge
        ) {
            1
        } else {
            0
        }
    }
}

/// Seed that has fallen and not yet come to anything.
///
/// It is either waiting for the ground it is on to be free, or it is on
/// ground its kind cannot live on and is quietly going off. Either way it is
/// on a clock: see `PlantSpecies::seed_keeps_for_ticks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    pub species_id: String,
    pub position: (i32, i32),

    /// The tick it fell on.
    ///
    /// How old it is, is `now` less this. It was an `age_ticks` that
    /// something had to remember to wind on, which is a second clock to keep
    /// in step with the first, and seed is only looked at when its zone comes
    /// round - see `PlantManager::grow_a_zone`.
    pub dropped_at: u32,
}

/// Manages plant population and growth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantManager {
    plants: Vec<Plant>,

    /// What has been happening to the vegetation, for measuring only.
    #[serde(default)]
    ledger: PlantLedger,

    /// What has fallen and not yet come up.
    ///
    /// A seed is far smaller than a plant and most of them come to nothing,
    /// so this is deliberately a separate list rather than a growth stage
    /// before `Seedling`: a Seedling is something standing on a tile and
    /// taking up its light, and a seed is not.
    #[serde(default)]
    seeds: Vec<Seed>,

    max_population: usize,
    natural_spawn_rate: f32,
    #[serde(skip)]
    registry: Option<FloraRegistry>,
}

impl PlantManager {
    /// How far seed falls from what dropped it, in cells.
    ///
    /// A cell is ten metres, so this is forty metres at the outside: under
    /// the parent and a little beyond, which is where nearly all seed goes.
    /// What carries further than that is carried by something, and the things
    /// that carry seed here are animals - see the midden, which has always
    /// brought up what went through somebody.
    ///
    /// It was a hundred metres, which threw two seed in five onto country of
    /// a kind the parent's own species cannot live on. Seed does not usually
    /// travel far enough to leave the ground its parent is growing in.
    const HOW_FAR_A_SEED_FALLS: i32 = 4;

    /// The most seed the ground can be holding at once.
    ///
    /// A safety valve, and it has to be well clear of what the bank actually
    /// settles at or it stops being a valve and starts being the thing that
    /// decides the ecology: while the bank is full nothing seeds at all, so
    /// which species holds the ground comes down to who was in the list
    /// first. At a quarter of the plant ceiling it was full from year three
    /// onwards.
    ///
    /// What the bank holds is only seed that landed on country of a kind its
    /// species cannot live on - everything else is spent on the pass after it
    /// falls - so its size is the seed going astray times how long seed keeps,
    /// and two seed in five go astray.
    fn how_much_seed_the_ground_holds(&self) -> usize {
        self.max_population * 2
    }
}

impl PlantManager {
    pub fn new(max_population: usize) -> Self {
        Self {
            plants: Vec::new(),
            ledger: PlantLedger::default(),
            seeds: Vec::new(),
            max_population,
            natural_spawn_rate: 0.01,
            registry: Some(FloraRegistry::new()),
        }
    }


    /// Spawn a plant at a position
    /// Put a plant on the ground, as of `now`.
    ///
    /// `now` is not decoration. A plant carries the tick it has been grown up
    /// to, and its zone works out how long a pass stands for by subtracting
    /// that from the tick it is asked about - so a plant that comes up in year
    /// twelve with a clock still reading nought is a plant that ages twelve
    /// years the first time its zone comes round, which for a grass is six
    /// times over its whole life. Every grass and herb on a hundred and twenty
    /// by a hundred and twenty was gone by year fifteen and every bush by year
    /// forty-five, with the trees left standing because a tree can afford it.
    /// So the tick is a parameter and every caller has to say which one.
    pub fn spawn_plant(
        &mut self,
        species_id: String,
        position: (i32, i32),
        now: u32,
    ) -> Option<Uuid> {
        if self.plants.len() >= self.max_population {
            return None;
        }

        let species = self.registry.as_ref()?.get(&species_id)?;

        let mut plant = Plant::new(species_id.clone(), position).with_species(species);
        plant.grown_up_to = now;

        let id = plant.id;
        self.plants.push(plant);
        Some(id)
    }

    /// Spawn a cultivated plant (farmed)
    pub fn plant_crop(
        &mut self,
        species_id: String,
        position: (i32, i32),
        planter_id: Uuid,
        now: u32,
    ) -> Option<Uuid> {
        if self.plants.len() >= self.max_population {
            return None;
        }

        let species = self.registry.as_ref()?.get(&species_id)?;

        let mut plant = Plant::new(species_id.clone(), position)
            .with_species(species)
            .cultivated(planter_id);
        plant.grown_up_to = now;

        let id = plant.id;
        self.plants.push(plant);
        Some(id)
    }

    /// Spawn multiple plants in an area (forest, field, etc.)
    pub fn spawn_patch(
        &mut self,
        species_id: String,
        center: (i32, i32),
        radius: u32,
        density: f32,
        now: u32,
    ) -> Vec<Uuid> {
        let mut spawned = Vec::new();
        let count = ((radius * radius) as f32 * density) as u32;

        for _ in 0..count {
            let offset_x = (crate::core::dice::any::<i32>() % (radius as i32 * 2)) - radius as i32;
            let offset_y = (crate::core::dice::any::<i32>() % (radius as i32 * 2)) - radius as i32;

            let pos = (center.0 + offset_x, center.1 + offset_y);

            if let Some(id) = self.spawn_plant(species_id.clone(), pos, now) {
                spawned.push(id);
            }
        }

        spawned
    }

    /// Get plant by ID
    pub fn get(&self, id: &Uuid) -> Option<&Plant> {
        self.plants.iter().find(|p| &p.id == id)
    }

    /// Get mutable plant by ID
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Plant> {
        self.plants.iter_mut().find(|p| &p.id == id)
    }

    /// Get all plants at a position
    pub fn get_at_position(&self, position: (i32, i32)) -> Vec<&Plant> {
        self.plants.iter().filter(|p| p.position == position).collect()
    }

    /// Get all plants in radius
    pub fn get_in_radius(&self, center: (i32, i32), radius: f32) -> Vec<&Plant> {
        self.plants
            .iter()
            .filter(|p| {
                let dx = (p.position.0 - center.0) as f32;
                let dy = (p.position.1 - center.1) as f32;
                (dx * dx + dy * dy).sqrt() <= radius
            })
            .collect()
    }

    /// Get harvestable plants in radius
    pub fn get_harvestable_in_radius(&self, center: (i32, i32), radius: f32) -> Vec<&Plant> {
        self.get_in_radius(center, radius)
            .into_iter()
            .filter(|p| p.is_harvestable && !p.has_been_harvested)
            .collect()
    }

    /// Harvest a plant
    pub fn harvest_plant(&mut self, id: &Uuid) -> Option<Vec<PlantDrop>> {
        // Get species ID first to avoid borrow checker issues
        let species_id = self.get(id)?.species_id.clone();

        // Clone species data to avoid borrow checker issues
        let species = self.registry.as_ref()?.get(&species_id)?.clone();

        // Now we can mutably borrow the plant
        let plant = self.get_mut(id)?;
        Some(plant.harvest(&species))
    }

    /// Get all plants
    pub fn get_all(&self) -> &Vec<Plant> {
        &self.plants
    }

    /// Get species from registry
    pub fn get_species(&self, species_id: &str) -> Option<&PlantSpecies> {
        self.registry.as_ref()?.get(species_id)
    }

    /// Tick all plants (growth, regrowth)
    pub fn tick(&mut self) {
        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Remove dead plants that don't regrow
        self.plants.retain(|p| {
            if p.has_been_harvested {
                if let Some(species) = registry.get(&p.species_id) {
                    species.regrows || p.regrow_timer > 0
                } else {
                    false
                }
            } else {
                true
            }
        });

        // Grow plants
        for plant in &mut self.plants {
            if let Some(species) = registry.get(&plant.species_id) {
                plant.grow(species);
            }
        }
    }

    /// Stock a freshly generated world with the vegetation its country would
    /// carry.
    ///
    /// Nothing had ever created a plant. `spawn_plant`, `plant_crop` and
    /// `spawn_patch` existed, were tested, and had no callers outside the
    /// world's own pass-through wrappers, so `world.plants` was empty in every
    /// run that has ever been made and `tick` iterated nothing.
    pub fn spawn_naturalistic(&mut self, grid: &crate::world::Grid) {
        use rand::Rng;

        let registry = match &self.registry {
            Some(registry) => registry.clone(),
            None => return,
        };

        let mut rng = crate::core::dice::roll();

        // What grows where, and how thickly
        for y in 0..grid.height {
            for x in 0..grid.width {
                if self.plants.len() >= self.max_population {
                    return;
                }

                let terrain = grid.tiles[y][x].terrain.terrain_type;

                // How thickly the ground carries growth when a world opens.
                //
                // These were four to five times thinner, from when a `Plant`
                // was a fixture that nothing ate and nothing replaced: a
                // fifty by fifty map opened with 212 plants on 2,500 tiles.
                // Left alone for fifteen years the same country settles at
                // about two-fifths covered, so the old figures were not a
                // sparser world, they were the same world before it had
                // filled in - and in the meantime there was nothing on it for
                // a grazing animal to eat. A dozen sheep on twenty-five
                // hectares, which is light stocking for real ground, starved.
                //
                // A world opens at roughly where it settles now, which is
                // also what stops the first fifteen years of every run being
                // spent growing the vegetation the map was supposed to start
                // with.
                let (density, want_trees) = match terrain {
                    crate::world::TerrainType::Forest => (0.85, true),
                    crate::world::TerrainType::Meadow => (0.80, false),
                    crate::world::TerrainType::Wetland => (0.60, false),
                    crate::world::TerrainType::Riverbank => (0.55, false),
                    crate::world::TerrainType::Plains => (0.45, false),
                    crate::world::TerrainType::Hills => (0.25, false),
                    crate::world::TerrainType::Desert => (0.05, false),
                    _ => (0.0, false),
                };

                if density <= 0.0 || rng.gen::<f32>() > density {
                    continue;
                }

                let candidates: Vec<&PlantSpecies> = registry
                    .all_species()
                    .into_iter()
                    .filter(|species| species.is_tree == want_trees)
                    .filter(|species| {
                        species
                            .primary_biomes
                            .contains(&crate::environment::fauna::terrain_to_climate_zone(terrain))
                    })
                    .collect();

                if candidates.is_empty() {
                    continue;
                }

                let species = candidates[rng.gen_range(0..candidates.len())];
                let species_id = species.id.clone();

                let how_old = rng.gen::<f32>();

                if self.spawn_plant(species_id, (x as i32, y as i32), 0).is_some() {
                    // A world does not start as bare seedlings: what is
                    // standing has been standing a while.
                    //
                    // The one just planted is the one on the end. Looking it
                    // up by id meant walking every plant already standing for
                    // every plant put down, which is the square of the map:
                    // stocking a hundred square kilometres took a quarter of
                    // an hour and most of it was this.
                    if let Some(plant) = self.plants.last_mut() {
                        plant.growth_stage = GrowthStage::Mature;
                        plant.growth_progress = how_old;
                        plant.is_harvestable = true;

                        // And they have not all been standing the same while.
                        // A wood put down all at once is a wood that comes
                        // down all at once: every tree in it born on tick
                        // zero reaches two hundred and fifty years within a
                        // few passes of every other, and the country goes
                        // from full timber to bare ground inside a season.
                        // What makes a wood a wood is that there is an age
                        // of everything in it, so they start scattered
                        // across a lifetime.
                        plant.age_ticks =
                            (how_old * species.lives_for_ticks() as f32) as u32;
                    }
                }
            }
        }
    }

    /// How much this plant shades what is under and around it
    fn canopy_of(size: PlantSize, is_tree: bool) -> f32 {
        let base = match size {
            PlantSize::Huge => 0.9,
            PlantSize::Large => 0.7,
            PlantSize::Medium => 0.4,
            PlantSize::Small => 0.15,
            PlantSize::Tiny => 0.05,
        };

        if is_tree {
            base
        } else {
            base * 0.5
        }
    }

    /// What a plant sheds, for what it has just drawn out of the ground.
    ///
    /// A plant that has finished growing gives back everything it takes: it
    /// is not putting anything on, so what goes up the stem comes down again
    /// as leaf and root and stalk. One that is still building itself keeps
    /// half - the same half `Soil::RESIDUE_PER_UNIT_GROWN` holds back for a
    /// crop node - and that half comes back when it dies. Both are grossed up
    /// by `KEPT_FROM_ROT` the same way, because litter loses some of itself to
    /// the air on the way to becoming soil again.
    ///
    /// This used to be two unrelated tables: an appetite that took no notice
    /// of what kind of plant it was, and a leaf fall by size that took no
    /// notice of what the plant had drawn. A small plant took two and a half
    /// times out of its tile what it put back, and a meadow with nobody near
    /// it lost a tenth of its fertility in a year. It is the third accounting
    /// of the same physics in this model and it was the only one nobody had
    /// balanced against itself.
    fn what_a_plant_sheds_for_what_it_drew(drawn: f32, still_growing: bool) -> f32 {
        use crate::world::soil::Soil;

        let goes_back = if still_growing { 0.5 } else { 1.0 };
        drawn * goes_back / Soil::KEPT_FROM_ROT
    }

    /// How much leaf fall this plant puts on the ground each tick
    #[allow(dead_code)]
    fn leaf_fall_of(size: PlantSize) -> f32 {
        match size {
            PlantSize::Huge => 0.00040,
            PlantSize::Large => 0.00025,
            PlantSize::Medium => 0.00010,
            PlantSize::Small => 0.00004,
            PlantSize::Tiny => 0.00001,
        }
    }

    /// How many zones the map is cut into for growing.
    pub const HOW_MANY_ZONES: usize = 24;

    /// How often one zone's turn comes round.
    ///
    /// Five days, so the whole map is grown through every hundred and twenty.
    /// It was a bare `60` in `World::tick` and a bare `60` again in two test
    /// fixtures here that drive the same sweep - and *those two numbers have
    /// to agree*, because they each work out which zone it is by dividing the
    /// tick by their own copy. Disagree and the wrong quarter of the country
    /// grows. Stated in days it stays five days at any turn length; stated in
    /// ticks it would have been a day and a quarter the moment the turn got
    /// shorter, and the whole map would have been grown four times over in a
    /// season. See ISSUES_FOUND #205.
    pub const HOW_OFTEN_A_ZONE_COMES_ROUND: u32 =
        crate::environment::seasons::TICKS_PER_DAY * 5;

    /// Grow one zone of what is standing, on what the ground and sky give it.
    ///
    /// This is where the vegetation and the soil meet. Each plant takes its
    /// water from the country and the weather, its light from whatever is
    /// standing over it, and its nutrient out of the ground - which it depletes
    /// - and puts leaf fall back where it stands, which in time becomes more
    /// nutrient. A wood feeds itself. A hedgerow stripped bare on thin ground
    /// does not.
    ///
    /// One zone in twenty-four, so a plant is worked out once in fourteen
    /// hundred and forty ticks - four months - unless something is standing on
    /// it, in which case `catch_up_one` brings it up to date between mouthfuls.
    /// Nothing a plant does on its own happens faster than four months. What
    /// this buys is that a hundred square kilometres carries a quarter of a
    /// million plants and only about ten thousand of them are looked at in any
    /// pass, one zone at a time so that no single tick carries the lot.
    ///
    /// No caller says how many ticks the pass stands for. Each plant carries
    /// the tick it has been grown up to and works out its own span, which is
    /// the only thing that makes it safe for the same plant to be reached by a
    /// zone pass and by something grazing it.
    pub fn grow_a_zone(
        &mut self,
        grid: &mut crate::world::Grid,
        precipitation: f32,
        now: u32,
        season: crate::environment::Season,
        zone: usize,
    ) {
        use crate::world::soil::Soil;
        use crate::world::Position;

        let registry = match &self.registry {
            Some(registry) => registry.clone(),
            None => return,
        };

        // Clear out what has died and will not come back
        self.plants.retain(|plant| {
            if plant.has_been_harvested {
                registry
                    .get(&plant.species_id)
                    .map(|species| species.regrows || plant.regrow_timer > 0)
                    .unwrap_or(false)
            } else {
                true
            }
        });

        // The rows this zone is. Bands of rows rather than blocks, because a
        // band is one comparison per plant to test and it covers the map
        // exactly however the height divides.
        let band = Self::what_rows_a_zone_is(grid.height, zone);
        if band.is_empty() {
            return;
        }

        // And the rows whose plants can throw shade into it, which is the band
        // and one row either side.
        let shading = band.start.saturating_sub(1)..(band.end + 1).min(grid.height);

        // What is standing over each tile, gathered once. Doing this per plant
        // would be a comparison against every other plant in the world.
        //
        // One number a tile, laid out flat, rather than a map keyed by
        // position: a wooded hundred square kilometres carries eighty thousand
        // plants, each of which puts five entries into this, and four hundred
        // thousand tree-map inserts a pass was seven of the ten milliseconds a
        // tick cost. Four megabytes of floats and a memset is cheaper than the
        // tree by an order of magnitude, and the map was never sparse anyway.
        let width = grid.width;
        let height = grid.height;
        let mut canopy = vec![0.0f32; width * height];

        let mut shade_at = |x: i32, y: i32, by: f32, canopy: &mut Vec<f32>| {
            if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                return;
            }
            canopy[y as usize * width + x as usize] += by;
        };

        for plant in &self.plants {
            if !Self::is_in(&shading, plant.position.1) {
                continue;
            }

            let species = match registry.get(&plant.species_id) {
                Some(species) => species,
                None => continue,
            };

            if !matches!(
                plant.growth_stage,
                GrowthStage::Mature | GrowthStage::Flowering | GrowthStage::Fruiting
            ) {
                continue;
            }

            let shade = Self::canopy_of(species.size, species.is_tree);
            shade_at(plant.position.0, plant.position.1, shade, &mut canopy);

            // Big things shade their neighbours too
            if shade >= 0.4 {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    shade_at(
                        plant.position.0 + dx,
                        plant.position.1 + dy,
                        shade * 0.35,
                        &mut canopy,
                    );
                }
            }
        }

        // Before the growing loop, because it reads each plant's own span off
        // a clock that loop is about to wind on.
        self.what_bore_seed_this_pass(&registry, now, &band);

        for plant in &mut self.plants {
            if !Self::is_in(&band, plant.position.1) {
                continue;
            }

            // How long it is since this plant was last grown, which is what
            // this pass stands for. Nought means something has already brought
            // it up to date this tick.
            let ticks = now.saturating_sub(plant.grown_up_to) as f32;
            plant.grown_up_to = now;
            if ticks <= 0.0 {
                continue;
            }

            let species = match registry.get(&plant.species_id) {
                Some(species) => species,
                None => continue,
            };

            let position = Position::new(plant.position.0, plant.position.1);

            let tile = match grid.get_tile_mut(&position) {
                Some(tile) => tile,
                None => continue,
            };

            let terrain = tile.terrain.terrain_type;

            // Water: what the ground holds, and what falls on it
            let water = Soil::humidity(terrain, precipitation);

            // Light: full sun less whatever is standing over it. A plant does
            // not shade itself out of existence, so its own canopy is
            // discounted.
            let over_it = if plant.position.0 >= 0
                && plant.position.1 >= 0
                && (plant.position.0 as usize) < width
                && (plant.position.1 as usize) < height
            {
                canopy[plant.position.1 as usize * width + plant.position.0 as usize]
            } else {
                0.0
            };
            let own = if matches!(
                plant.growth_stage,
                GrowthStage::Mature | GrowthStage::Flowering | GrowthStage::Fruiting
            ) {
                Self::canopy_of(species.size, species.is_tree)
            } else {
                0.0
            };
            // A short day is less light, whatever is or is not standing over
            // the plant. This is what makes a winter a winter for a plant:
            // nine hours of sun against summer's fifteen.
            let daylight = season.day_length() / 15.0;
            let light = ((1.0 - (over_it - own).max(0.0)) * daylight).clamp(0.05, 1.0);

            // Remembered so that something standing on this plant can bring it
            // up to date without the canopy having to be gathered again.
            plant.shade_on_it = (over_it - own).max(0.0);

            // Nutrient: what is in the ground, and how readily this plant can
            // get at it. Broken ground is worked, weeded and watered, so a crop
            // on it takes up far more of what is there - it does not grow
            // faster than its kind can grow.
            let uptake = if tile.terrain.is_cultivated() { 2.5 } else { 1.0 };

            let conditions = GrowingConditions {
                water,
                light,
                nutrients: tile.soil.fertility(),
                uptake,
            };

            plant.grow_in(species, conditions, ticks);

            // And whether it can hold its own where it is standing.
            //
            // Growth alone had no downward half: a plant on ground that gave
            // it nothing simply did not grow, and sat there not growing for
            // ever. That is what let a wood fill up - every seedling that
            // ever took under a closed canopy stayed a seedling and stayed on
            // its tile, and the tile was never free again. A plant that
            // cannot make a living where it is now goes back, and how fast
            // depends on how far short the ground is falling.
            let living = conditions.growth_share();
            if living < Self::WHAT_A_PLANT_NEEDS_TO_HOLD_ITS_OWN {
                plant.current_health -= Self::what_a_bad_pass_costs(plant.max_health, living, ticks);
            } else {
                // What it puts back on is what the ground and the sky give it,
                // so a plant on poor ground comes back slowly and one in a
                // wet meadow comes back fast. This is what makes a sward
                // forage rather than a stock: something crops it, and it grows
                // again out of the same water and light and nutrient
                // everything else here runs on.
                plant.current_health = (plant.current_health
                    + plant.max_health * Self::HOW_FAST_A_PLANT_COMES_BACK * living * ticks)
                    .min(plant.max_health);
            }

            // What it grows with, it takes out of the ground
            let wanted = conditions.draw_per_tick() * ticks;
            if wanted > 0.0 {
                tile.soil.draw(wanted);
            }
            let drawn = wanted;

            // And what it sheds, it puts back.
            //
            // At every stage, not only once grown. A seedling drops leaves
            // too, and only counting the grown ones left every young plant on
            // the map drawing nutrient out of the ground and putting nothing
            // back - which did not matter while nothing ever came up from
            // seed, and matters now that most of what is standing on a tile
            // in any given decade has come up from seed. A meadow nobody
            // touched lost a tenth of its fertility in a year.
            //
            // What a young plant sheds is less than what a grown one does,
            // in proportion to how much of the plant there is yet.
            // A plant still building itself keeps half of what it draws; one
            // that has finished growing is not putting on anything and gives
            // back everything it takes.
            let keeps_some_of_it = matches!(
                plant.growth_stage,
                GrowthStage::Seedling | GrowthStage::Growing
            );
            tile.soil
                .add_leaf_litter(Self::what_a_plant_sheds_for_what_it_drew(drawn, keeps_some_of_it));
        }

        self.what_came_up_and_what_rotted(grid, &registry, &canopy, now, &band);
        self.what_died(grid, &registry);
    }

    /// The rows of the map a given zone stands for.
    ///
    /// Bands rather than blocks, and worked out from the height every time
    /// rather than stored, so that the twenty-four of them tile the map
    /// exactly whatever its size and there is no second answer to which zone
    /// a row is in.
    fn what_rows_a_zone_is(height: usize, zone: usize) -> std::ops::Range<usize> {
        let zone = zone % Self::HOW_MANY_ZONES;
        let from = height * zone / Self::HOW_MANY_ZONES;
        let to = height * (zone + 1) / Self::HOW_MANY_ZONES;
        from..to
    }

    /// Whether a row, as a plant holds it, falls inside a band.
    fn is_in(band: &std::ops::Range<usize>, y: i32) -> bool {
        y >= 0 && band.contains(&(y as usize))
    }

    /// Bring one plant up to now, because something is standing on it.
    ///
    /// A plant waits for its zone, which is four months. Something grazing it
    /// takes a bite every ten ticks, so without this a grazed plant would lose
    /// condition a hundred and forty-four times for every time it gained any,
    /// and the first patch of ground a herd stood on would be the last.
    ///
    /// What it cannot do on its own is gather the canopy, so it uses the shade
    /// the plant remembers from its last zone pass - see `Plant::shade_on_it`.
    /// Everything else it needs is on the tile in front of it.
    pub fn catch_up_one(
        &mut self,
        which: usize,
        grid: &mut crate::world::Grid,
        precipitation: f32,
        now: u32,
        season: crate::environment::Season,
    ) {
        use crate::world::soil::Soil;
        use crate::world::Position;

        let Some(registry) = self.registry.clone() else {
            return;
        };
        let Some(plant) = self.plants.get_mut(which) else {
            return;
        };

        let ticks = now.saturating_sub(plant.grown_up_to) as f32;
        if ticks <= 0.0 {
            return;
        }
        plant.grown_up_to = now;

        let Some(species) = registry.get(&plant.species_id) else {
            return;
        };

        let here = Position::new(plant.position.0, plant.position.1);
        let Some(tile) = grid.get_tile_mut(&here) else {
            return;
        };

        let terrain = tile.terrain.terrain_type;
        let water = Soil::humidity(terrain, precipitation);

        let own = if matches!(
            plant.growth_stage,
            GrowthStage::Mature | GrowthStage::Flowering | GrowthStage::Fruiting
        ) {
            Self::canopy_of(species.size, species.is_tree)
        } else {
            0.0
        };
        let daylight = season.day_length() / 15.0;
        let light =
            ((1.0 - (plant.shade_on_it - own).max(0.0)) * daylight).clamp(0.05, 1.0);

        let uptake = if tile.terrain.is_cultivated() { 2.5 } else { 1.0 };

        let conditions = GrowingConditions {
            water,
            light,
            nutrients: tile.soil.fertility(),
            uptake,
        };

        plant.grow_in(species, conditions, ticks);

        let living = conditions.growth_share();
        if living < Self::WHAT_A_PLANT_NEEDS_TO_HOLD_ITS_OWN {
            plant.current_health -= Self::what_a_bad_pass_costs(plant.max_health, living, ticks);
        } else {
            plant.current_health = (plant.current_health
                + plant.max_health * Self::HOW_FAST_A_PLANT_COMES_BACK * living * ticks)
                .min(plant.max_health);
        }

        let drawn = conditions.draw_per_tick() * ticks;
        if drawn > 0.0 {
            tile.soil.draw(drawn);
        }

        let still_growing = matches!(
            plant.growth_stage,
            GrowthStage::Seedling | GrowthStage::Growing
        );
        tile.soil
            .add_leaf_litter(Self::what_a_plant_sheds_for_what_it_drew(drawn, still_growing));
    }

    /// What a plant leaves on the ground when it finally goes over.
    ///
    /// A tree is mostly wood, which lies for years; a herb is soft and gone
    /// in a season. The two litters already break down at their own rates -
    /// see `Soil::decay` - so all this has to decide is how much of which,
    /// and that follows from how big the thing was. A dead oak is the largest
    /// single thing that ever happens to a tile of soil in this model, which
    /// is right: a fallen tree is what makes the ground under a wood.
    fn what_a_dead_plant_leaves(size: PlantSize, is_tree: bool) -> (f32, f32) {
        let bulk = match size {
            PlantSize::Huge => 3.0,
            PlantSize::Large => 2.0,
            PlantSize::Medium => 0.8,
            PlantSize::Small => 0.2,
            PlantSize::Tiny => 0.05,
        };

        if is_tree {
            // Mostly timber, some leaf
            (bulk * 0.25, bulk * 0.75)
        } else {
            (bulk * 0.9, bulk * 0.1)
        }
    }

    /// The least share of its best pace a plant can live on.
    ///
    /// Below this it is not growing slowly, it is dying slowly. The number
    /// has to sit under what a short winter's day gives - `day_length` at
    /// midwinter is nine hours against fifteen, so light alone drops to 0.6
    /// and everything would be starving every January - and over what a
    /// closed canopy leaves, which is 0.05.
    const WHAT_A_PLANT_NEEDS_TO_HOLD_ITS_OWN: f32 = 0.12;

    /// How much of itself a plant loses per tick when the ground falls right
    /// away under it. Two thousand ticks, half a year, from full to gone.
    const HOW_FAST_A_PLANT_GOES_BACK: f32 = 0.0005;

    /// And the most it can lose in any one pass, however long the pass is.
    ///
    /// A pass reads the water, the light and the soil once and then applies
    /// that reading for as long as the pass stands for. At ten or twenty ticks
    /// that is a fair account of the weather; at fourteen hundred and forty it
    /// is four months of drought inferred from one wet afternoon or one dry
    /// one, and without a limit a plant caught on a bad reading loses
    /// seven-tenths of itself between one look and the next. Three bad passes
    /// in a row - a year of them - still kills it, which is about what a year
    /// of the wrong ground should do.
    const THE_MOST_A_PLANT_LOSES_IN_ONE_PASS: f32 = 1.0 / 3.0;

    /// And how fast it puts condition back on, per tick, at its best pace.
    ///
    /// A plant cropped to nothing is back to full in about a month given
    /// everything it wants, and longer than that on any real ground, because
    /// this is scaled by `growth_share`. That rate is what decides how many
    /// mouths a piece of country will feed: what a grazing animal can take in
    /// the long run is what grows back, not what is standing.
    ///
    /// It was fifteen times slower and not scaled by anything, which was fine
    /// while nothing ate a plant and hopeless the moment something did: a
    /// grass has five points of condition and put back a fiftieth of one in a
    /// pass, so a dozen sheep on twenty-five hectares - light stocking for
    /// real ground - ate it bare and starved.
    ///
    /// A month is what a sward takes to come back after it is grazed off,
    /// which is the case this number has to be right for. It is too fast for
    /// an oak, and nothing crops an oak.
    const HOW_FAST_A_PLANT_COMES_BACK: f32 = 0.003;

    /// What a pass on ground that will not keep a plant takes off it.
    ///
    /// Held to `THE_MOST_A_PLANT_LOSES_IN_ONE_PASS` however long the pass is,
    /// because the conditions it is working from are one reading and not an
    /// average of the span.
    fn what_a_bad_pass_costs(max_health: f32, living: f32, ticks: f32) -> f32 {
        let short = (Self::WHAT_A_PLANT_NEEDS_TO_HOLD_ITS_OWN - living)
            / Self::WHAT_A_PLANT_NEEDS_TO_HOLD_ITS_OWN;

        (max_health * Self::HOW_FAST_A_PLANT_GOES_BACK * short * ticks)
            .min(max_health * Self::THE_MOST_A_PLANT_LOSES_IN_ONE_PASS)
    }

    /// Everything that has come to the end of its life, and what it leaves.
    ///
    /// Two ends: old age, and a plant that could not make a living where it
    /// stood for long enough. Nothing here had either before. A plant aged,
    /// and its age was read by nothing: a hedgerow put down when the world
    /// was made was the same hedgerow thirty years later, and the only way
    /// anything ever left the map was being harvested by somebody. So a map
    /// with nobody on it had exactly the vegetation it started with, for
    /// ever, and no room anywhere for anything new to come up.
    fn what_died(&mut self, grid: &mut crate::world::Grid, registry: &FloraRegistry) {
        use crate::world::Position;

        let mut fell = Vec::new();
        let mut tally = self.ledger.clone();

        self.plants.retain(|plant| {
            let Some(species) = registry.get(&plant.species_id) else {
                return true;
            };

            let of_old_age = plant.age_ticks >= species.lives_for_ticks();
            let of_the_ground = plant.current_health <= 0.0;

            if !of_old_age && !of_the_ground {
                return true;
            }

            let class = PlantLedger::which_class(species);
            if of_old_age {
                tally.died_of_age[class] += 1;
            } else {
                tally.died_of_the_ground[class] += 1;
            }

            fell.push((
                plant.position,
                Self::what_a_dead_plant_leaves(species.size, species.is_tree),
            ));
            false
        });

        self.ledger.died_of_age = tally.died_of_age;
        self.ledger.died_of_the_ground = tally.died_of_the_ground;

        for (at, (soft, woody)) in fell {
            let here = Position::new(at.0, at.1);
            if let Some(tile) = grid.get_tile_mut(&here) {
                tile.soil.add_leaf_litter(soft);
                tile.soil.add_woody_litter(woody);
            }
        }
    }

    /// What dropped seed, and where it fell.
    ///
    /// Only what is bearing: a seedling has nothing to drop and a plant that
    /// has just been picked has had its seed taken. Where it falls is within
    /// `HOW_FAR_A_SEED_FALLS` of the parent, which is under it and a little
    /// beyond.
    fn what_bore_seed_this_pass(
        &mut self,
        registry: &FloraRegistry,
        now: u32,
        band: &std::ops::Range<usize>,
    ) {
        use rand::Rng;

        let room = self.how_much_seed_the_ground_holds();
        if self.seeds.len() >= room {
            return;
        }

        let mut rng = crate::core::dice::roll();

        let mut fell = Vec::new();

        for plant in &self.plants {
            if !Self::is_in(band, plant.position.1) {
                continue;
            }

            // Each plant's own span, taken before the growing loop winds its
            // clock on - which is why this runs first. A plant that comes into
            // bearing during the span seeds from the next pass rather than
            // this one, which over four months is neither here nor there.
            let pass = now.saturating_sub(plant.grown_up_to) as f32 / 10.0;
            if pass <= 0.0 {
                continue;
            }

            if !matches!(
                plant.growth_stage,
                GrowthStage::Flowering | GrowthStage::Fruiting
            ) {
                continue;
            }

            if plant.has_been_harvested {
                continue;
            }

            let Some(species) = registry.get(&plant.species_id) else {
                continue;
            };

            // How much seed this plant put out over the span, as a count
            // rather than a coin. A pass used to be ten ticks and a chance
            // under one; a pass now stands for up to fourteen hundred and
            // forty, and a chance clamped to one would have a grass drop a
            // single seed where it should have dropped eight.
            let expected = species.seeds_per_pass() * pass;
            let mut how_many = expected.floor() as u32;
            if rng.gen::<f32>() < expected.fract() {
                how_many += 1;
            }

            let reach = Self::HOW_FAR_A_SEED_FALLS;
            for _ in 0..how_many {
                self.ledger.seed_dropped[PlantLedger::which_class(species)] += 1;

                fell.push(Seed {
                    species_id: plant.species_id.clone(),
                    position: (
                        plant.position.0 + rng.gen_range(-reach..=reach),
                        plant.position.1 + rng.gen_range(-reach..=reach),
                    ),
                    dropped_at: now,
                });

                if self.seeds.len() + fell.len() >= room {
                    break;
                }
            }

            if self.seeds.len() + fell.len() >= room {
                break;
            }
        }

        // Seed of something this world does not have a species for is not
        // seed, it is a name nobody can look up.
        fell.retain(|seed| registry.get(&seed.species_id).is_some());
        self.seeds.extend(fell);
    }

    /// What came up, and what went off waiting to.
    ///
    /// A seed comes up when the ground under it will carry its kind and
    /// nothing is standing on that ground already. A seed on ground its kind
    /// cannot live on never comes up, and does not sit there for ever either:
    /// it keeps for `PlantSpecies::seed_keeps_for_ticks` and then it has
    /// rotted, which puts the little it was back into the litter.
    fn what_came_up_and_what_rotted(
        &mut self,
        grid: &mut crate::world::Grid,
        registry: &FloraRegistry,
        canopy: &[f32],
        now: u32,
        band: &std::ops::Range<usize>,
    ) {
        use crate::world::Position;
        use rand::Rng;

        if self.seeds.is_empty() {
            return;
        }

        // What is standing on each tile and how much of a place it takes, as
        // `how_much_ground_it_claims` plus one so that nought means free. One
        // pass over the plants and a flat lookup, rather than a search of
        // every plant for every seed - see the canopy above, which is laid
        // out the same way and for the same reason.
        let (width, height) = (grid.width, grid.height);
        let mut standing = vec![0u8; width * height];
        for plant in &self.plants {
            let (x, y) = plant.position;
            if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                continue;
            }
            if !Self::is_in(band, y) {
                continue;
            }
            let claim = registry
                .get(&plant.species_id)
                .map(|species| species.how_much_ground_it_claims() + 1)
                .unwrap_or(1);
            let at = y as usize * width + x as usize;
            standing[at] = standing[at].max(claim);
        }

        let mut rng = crate::core::dice::roll();
        let mut coming_up = Vec::new();
        let mut rotted = Vec::new();
        let mut shaded_out = Vec::new();

        // The ledger cannot be borrowed inside the retain, which is holding
        // `self.seeds`, so the tallies are gathered aside and folded back in.
        let mut tally = self.ledger.clone();

        self.seeds.retain_mut(|seed| {
            // Only the seed lying in the band this pass is about; the rest
            // waits for its own zone, and how old it is, is when it fell.
            if !Self::is_in(band, seed.position.1) {
                return true;
            }

            let Some(species) = registry.get(&seed.species_id) else {
                return false;
            };

            let (x, y) = seed.position;
            let here = Position::new(x, y);

            let Some(tile) = grid.get_tile(&here) else {
                return false; // off the edge of the world
            };

            if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                return false;
            }

            let at = y as usize * width + x as usize;

            // Free ground, or ground held by something this would come up
            // through. Equal claims stand: a grass does not displace a grass.
            let room = standing[at] == 0
                || species.how_much_ground_it_claims() + 1 > standing[at];

            // Seed that has landed on ground of the kind this plant lives on
            // is spent on the next pass, whichever way it goes. That is one
            // throw, which is what actually happens - a seed germinates once,
            // and the seedling either gets a root down or it does not - and
            // it is also the only way the light gate can bite. A fresh throw
            // every pass, over the four hundred passes a seed keeps for, took
            // every seed that ever landed on free ground however dark it was.
            //
            // Ground already held by something counts as a failure, not as a
            // wait. Seed does fall into a gap that opens later, but not the
            // greater part of it, and letting every seed queue for a tile
            // meant the bank filled to its ceiling and stayed there - and a
            // full bank stops all further seeding, so which species held the
            // ground came down to who happened to be in the list first.
            if species.could_live_on(tile.terrain.terrain_type) {
                let took = room
                    && rng.gen::<f32>() < Self::how_likely_a_seed_takes(canopy[at]);

                let class = PlantLedger::which_class(species);
                if took {
                    if standing[at] != 0 {
                        shaded_out.push(seed.position);
                    }
                    coming_up.push((seed.species_id.clone(), seed.position));
                    standing[at] = species.how_much_ground_it_claims() + 1;
                    tally.seed_took[class] += 1;
                } else {
                    rotted.push((seed.position, Self::WHAT_A_SEED_IS_WORTH));
                    tally.seed_lost_its_throw[class] += 1;
                }
                return false;
            }

            // On ground of a kind it cannot live on it never comes up at all,
            // and it does not sit there for ever either: it keeps for its
            // season or two and then it has rotted.
            if now.saturating_sub(seed.dropped_at) >= species.seed_keeps_for_ticks() {
                rotted.push((seed.position, Self::WHAT_A_SEED_IS_WORTH));
                tally.seed_rotted_on_wrong_ground[PlantLedger::which_class(species)] += 1;
                return false;
            }

            true
        });

        self.ledger.seed_took = tally.seed_took;
        self.ledger.seed_lost_its_throw = tally.seed_lost_its_throw;
        self.ledger.seed_rotted_on_wrong_ground = tally.seed_rotted_on_wrong_ground;

        // What was standing where something bigger has just come up goes back
        // into the ground it was standing in, the same as anything else that
        // dies.
        if !shaded_out.is_empty() {
            let gone: std::collections::BTreeSet<(i32, i32)> =
                shaded_out.into_iter().collect();
            let mut left_behind = Vec::new();

            self.plants.retain(|plant| {
                if !gone.contains(&plant.position) {
                    return true;
                }
                if let Some(species) = registry.get(&plant.species_id) {
                    left_behind.push((
                        plant.position,
                        Self::what_a_dead_plant_leaves(species.size, species.is_tree),
                    ));
                    self.ledger.shaded_out[PlantLedger::which_class(species)] += 1;
                }
                false
            });

            for (at, (soft, woody)) in left_behind {
                let here = Position::new(at.0, at.1);
                if let Some(tile) = grid.get_tile_mut(&here) {
                    tile.soil.add_leaf_litter(soft);
                    tile.soil.add_woody_litter(woody);
                }
            }
        }

        for (species_id, at) in coming_up {
            self.spawn_plant(species_id, at, now);
        }

        for (at, worth) in rotted {
            let here = Position::new(at.0, at.1);
            if let Some(tile) = grid.get_tile_mut(&here) {
                tile.soil.add_leaf_litter(worth);
            }
        }
    }

    /// How likely a seed on ground that would suit it is to actually take.
    ///
    /// Seed is cheap and a seedling is not, and nearly everything that falls
    /// fails. What decides it here is light, which is the one thing the model
    /// already knows about the ground over a tile - see the canopy in
    /// `tick_in_world`. Under a closed wood almost nothing comes up, which is
    /// why a wood has a floor rather than a thicket and why open ground stays
    /// open. Cubed rather than straight, because a half-shaded tile is a good
    /// deal worse than half as good for something trying to get a root down.
    ///
    /// Without this a seed took the moment it landed on free ground, and a
    /// hundred and twenty by a hundred and twenty went from a thousand plants
    /// to ten thousand in twenty years with the count still climbing, every
    /// grass and herb on the map crowded out by year thirteen.
    fn how_likely_a_seed_takes(shade: f32) -> f32 {
        const ON_OPEN_GROUND: f32 = 0.06;

        let sun = (1.0 - shade).clamp(0.0, 1.0);
        ON_OPEN_GROUND * sun * sun * sun
    }

    /// How much a seed that came to nothing puts back into the ground.
    ///
    /// Almost nothing, which is the point: what is being closed here is the
    /// loop, not the books. A seed that rots is a hundredth of what the plant
    /// that dropped it will leave when it goes over.
    const WHAT_A_SEED_IS_WORTH: f32 = 0.002;

    /// How much seed is lying in the ground.
    pub fn how_much_seed_is_waiting(&self) -> usize {
        self.seeds.len()
    }

    /// A running count of what has happened to the vegetation, for measuring.
    ///
    /// Not used by anything in the model. It is here because the only way to
    /// tell a species that is losing its ground from one that never had any
    /// seed on the ground in the first place is to count both.
    pub fn what_has_been_happening(&self) -> &PlantLedger {
        &self.ledger
    }

    /// Get count of plants by species
    pub fn count_species(&self, species_id: &str) -> usize {
        self.plants.iter().filter(|p| p.species_id == species_id).count()
    }

    /// Get count of harvestable plants by species
    pub fn count_harvestable(&self, species_id: &str) -> usize {
        self.plants
            .iter()
            .filter(|p| p.species_id == species_id && p.is_harvestable && !p.has_been_harvested)
            .count()
    }

    /// Get all plants
    /// The plants, to be reached into. For fixtures and tests: nothing in the
    /// model changes a plant except through this module's own passes.
    pub fn all_plants_mut(&mut self) -> &mut Vec<Plant> {
        &mut self.plants
    }

    pub fn all_plants(&self) -> &Vec<Plant> {
        &self.plants
    }

    /// Get total plant count
    pub fn total_count(&self) -> usize {
        self.plants.len()
    }

    /// Get summary of all plants
    pub fn plant_summary(&self) -> Vec<String> {
        let mut summaries = Vec::new();

        for plant in &self.plants {
            if let Some(registry) = &self.registry {
                if let Some(species) = registry.get(&plant.species_id) {
                    let cultivated = if plant.is_cultivated { " (cultivated)" } else { "" };
                    summaries.push(format!(
                        "{}{} at ({}, {}): {}",
                        species.name,
                        cultivated,
                        plant.position.0,
                        plant.position.1,
                        plant.status()
                    ));
                }
            }
        }

        summaries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Season;

    #[test]
    fn test_flora_registry() {
        let registry = FloraRegistry::new();

        // Should have all species registered
        assert!(registry.get("oak_tree").is_some());
        assert!(registry.get("flax").is_some());
        assert!(registry.get("cotton").is_some());
    }

    #[test]
    fn test_biome_filtering() {
        let registry = FloraRegistry::new();

        let temperate_plants = registry.get_by_biome(ClimateZone::Temperate);
        assert!(!temperate_plants.is_empty());

        // Oak should be in temperate
        assert!(temperate_plants.iter().any(|p| p.id == "oak_tree"));
    }

    #[test]
    fn test_plant_drops() {
        let flax = flax_plant();
        assert!(!flax.drops.is_empty());

        // Should drop flax fiber
        assert!(flax.drops.iter().any(|d| d.material_id == "flax_fiber"));
    }

    #[test]
    fn test_trees_vs_plants() {
        let oak = oak_tree();
        let flax = flax_plant();

        assert!(oak.is_tree);
        assert!(!flax.is_tree);
        assert_eq!(oak.size, PlantSize::Large);
        assert_eq!(flax.size, PlantSize::Small);
    }

    #[test]
    fn test_regrowth() {
        let grass = grass();
        let oak = oak_tree();

        assert!(grass.regrows);
        assert!(!oak.regrows);
    }

// --- a plant's own clock ------------------------------------------------------

/// A grass is finished in two seasons and an oak is not finished in two.
///
/// Lifespan is worked out from what kind of thing a plant is rather than
/// written out once per species, so what this actually checks is that the
/// derivation puts the fifty-one species in the right order of magnitude.
#[test]
fn a_grass_and_an_oak_do_not_live_the_same_length_of_time() {
    use crate::environment::seasons::TICKS_PER_YEAR;

    let registry = FloraRegistry::new();
    let grass = registry.get("grass").expect("there is grass in this world");
    let oak = registry.get("oak_tree").expect("and there are oaks");
    let bush = registry.get("berry_bush").expect("and berry bushes");

    assert!(
        (1.0..=3.0).contains(&grass.lives_for_years()),
        "a grass lives {} years",
        grass.lives_for_years()
    );
    assert!(
        (20.0..=60.0).contains(&bush.lives_for_years()),
        "a bush lives {} years",
        bush.lives_for_years()
    );
    assert!(
        (150.0..=400.0).contains(&oak.lives_for_years()),
        "an oak lives {} years",
        oak.lives_for_years()
    );

    assert_eq!(
        oak.lives_for_ticks(),
        (oak.lives_for_years() * TICKS_PER_YEAR as f32) as u32
    );
}

/// A short life means seeding hard; a long one means it can take its time.
///
/// What has to hold is that the two come out even over a lifetime, because
/// otherwise the ground goes to whatever lives the shortest whatever else is
/// true about it - which is what happened, and what took the bushes off the
/// map entirely.
#[test]
fn what_lives_briefly_seeds_the_harder_for_it() {
    let registry = FloraRegistry::new();
    let grass = registry.get("grass").unwrap();
    let oak = registry.get("oak_tree").unwrap();

    assert!(
        grass.seeds_per_pass() > oak.seeds_per_pass() * 50.0,
        "grass {} against oak {}",
        grass.seeds_per_pass(),
        oak.seeds_per_pass()
    );

    let over_a_life = |species: &PlantSpecies| {
        species.seeds_per_pass() * species.lives_for_ticks() as f32 / 10.0
    };

    let (short, long) = (over_a_life(grass), over_a_life(oak));
    assert!(
        (short - long).abs() < short * 0.05,
        "over a whole life: grass {short:.0}, oak {long:.0}"
    );
}

/// A sapling comes up through a sward. A sward never comes up through a wood.
#[test]
fn something_bigger_takes_the_ground_from_something_smaller() {
    let registry = FloraRegistry::new();
    let grass = registry.get("grass").unwrap();
    let bush = registry.get("berry_bush").unwrap();
    let oak = registry.get("oak_tree").unwrap();

    assert!(grass.how_much_ground_it_claims() < bush.how_much_ground_it_claims());
    assert!(bush.how_much_ground_it_claims() < oak.how_much_ground_it_claims());
}

/// Ground a plant cannot live on is ground a plant cannot live on.
#[test]
fn a_plant_knows_what_country_it_belongs_in() {
    use crate::world::TerrainType;

    let registry = FloraRegistry::new();
    let cactus = registry.get("cactus").expect("there are cacti");

    assert!(cactus.could_live_on(TerrainType::Desert));
    assert!(!cactus.could_live_on(TerrainType::Wetland));
}

/// Something that has stood for its whole lifetime is not standing any more.
#[test]
fn a_plant_that_has_had_its_years_goes_over() {
    use crate::environment::seasons::TICKS_PER_YEAR;
    use crate::world::{Grid, Position};

    let mut grid = Grid::new(12, 12);
    grid.generate_terrain();
    grid.settle_soil();

    let mut plants = PlantManager::new(100);
    plants.spawn_plant("grass".to_string(), (5, 5), 0);

    let litter_before = grid
        .get_tile(&Position::new(5, 5))
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);

    // A grass lives two years. Three of them is well past it. Every zone in
    // its turn, because the plant is only looked at when its own comes round.
    for tick in (0..(3 * TICKS_PER_YEAR))
        .step_by(PlantManager::HOW_OFTEN_A_ZONE_COMES_ROUND as usize)
    {
        let zone = (tick / PlantManager::HOW_OFTEN_A_ZONE_COMES_ROUND) as usize
            % PlantManager::HOW_MANY_ZONES;
        plants.grow_a_zone(&mut grid, 40.0, tick, Season::Summer, zone);
    }

    assert!(
        !plants.all_plants().iter().any(|plant| plant.position == (5, 5)),
        "a grass three years old is still standing"
    );

    let litter_after = grid
        .get_tile(&Position::new(5, 5))
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);
    assert!(
        litter_after > litter_before,
        "and it left nothing behind: {litter_before:.4} to {litter_after:.4}"
    );
}

/// Seed on ground that will not carry it never comes up, and does not lie
/// there for ever either.
#[test]
fn seed_on_the_wrong_ground_rots_instead_of_waiting_for_ever() {
    use crate::world::{Grid, Position, Terrain, TerrainType};

    // A whole map of one kind of ground, so that nothing a desert plant drops
    // can land anywhere that would suit it.
    let mut grid = Grid::new(20, 20);
    for y in 0..20 {
        for x in 0..20 {
            grid.tiles[y][x].terrain = Terrain::new(TerrainType::Wetland);
        }
    }
    grid.settle_soil();

    let mut plants = PlantManager::new(400);

    // A cactus, standing where a cactus cannot live, still sheds.
    plants.spawn_plant("cactus".to_string(), (10, 10), 0);
    if let Some(plant) = plants.all_plants_mut().last_mut() {
        plant.growth_stage = GrowthStage::Fruiting;
    }

    let registry = FloraRegistry::new();
    let cactus = registry.get("cactus").unwrap();

    // Long enough for seed to have fallen and for the first of it to be gone.
    let ticks = cactus.seed_keeps_for_ticks() * 3;
    for tick in (0..ticks).step_by(PlantManager::HOW_OFTEN_A_ZONE_COMES_ROUND as usize) {
        let zone = (tick / PlantManager::HOW_OFTEN_A_ZONE_COMES_ROUND) as usize
            % PlantManager::HOW_MANY_ZONES;
        plants.grow_a_zone(&mut grid, 40.0, tick, Season::Summer, zone);
    }

    let ledger = plants.what_has_been_happening();
    let class = PlantLedger::which_class(cactus);

    assert!(
        ledger.seed_dropped[class] > 0,
        "nothing was ever shed, so this test proves nothing"
    );
    assert!(
        ledger.seed_rotted_on_wrong_ground[class] > 0,
        "seed on ground it cannot live on never rotted"
    );
    assert_eq!(
        ledger.seed_took[class], 0,
        "a cactus came up in a marsh"
    );

    let _ = Position::new(0, 0);
}


// --- growing a zone at a time ------------------------------------------------

/// The twenty-four zones tile the map exactly: every row in one, none in two.
#[test]
fn every_row_of_the_map_is_in_exactly_one_zone() {
    for height in [1usize, 23, 24, 25, 50, 120, 1000] {
        let mut covered = vec![0u32; height];

        for zone in 0..PlantManager::HOW_MANY_ZONES {
            for row in PlantManager::what_rows_a_zone_is(height, zone) {
                covered[row] += 1;
            }
        }

        assert!(
            covered.iter().all(|&n| n == 1),
            "a map {height} rows deep: {:?}",
            covered
                .iter()
                .enumerate()
                .filter(|(_, &n)| n != 1)
                .collect::<Vec<_>>()
        );
    }
}

/// A plant that comes up in year twelve is twelve years old, not nought.
///
/// A plant works out how long a pass stands for by subtracting the tick it
/// was last grown up to from the tick it is asked about. Something that comes
/// up mid-run with that clock still reading nought ages the whole run the
/// first time its zone comes round - which for a grass is six times its own
/// lifetime, so it is dead before it has grown. Every grass and herb on a
/// hundred and twenty by a hundred and twenty was gone by year fifteen.
#[test]
fn a_plant_that_comes_up_late_is_not_born_old() {
    let mut plants = PlantManager::new(16);

    let a_long_way_in = 12 * crate::environment::seasons::TICKS_PER_YEAR;
    plants.spawn_plant("grass".to_string(), (3, 3), a_long_way_in);

    let planted = plants.all_plants().last().expect("it was planted");
    assert_eq!(
        planted.grown_up_to, a_long_way_in,
        "a plant put down at tick {a_long_way_in} thinks it was grown up to \
         {}",
        planted.grown_up_to
    );
}

/// Growing in one long stride ends up near where many short ones would.
///
/// The whole of what the zones buy is that a plant is worked out once in
/// fourteen hundred and forty ticks instead of once in ten. That is only
/// sound if the long stride and the short ones agree, and there are two
/// places they might not: a stage the plant would have passed through and out
/// the other side of, and seed it would have shed on the way.
#[test]
fn one_long_stride_gets_to_much_the_same_place_as_many_short_ones() {
    let registry = FloraRegistry::new();
    let grass = registry.get("grass").expect("there is grass");

    let ideal = GrowingConditions::ideal();
    let span = 1440.0;

    let mut in_one_go = Plant::new("grass".to_string(), (0, 0)).with_species(grass);
    in_one_go.grow_in(grass, ideal, span);

    let mut step_by_step = Plant::new("grass".to_string(), (0, 0)).with_species(grass);
    for _ in 0..144 {
        step_by_step.grow_in(grass, ideal, 10.0);
    }

    assert_eq!(
        in_one_go.age_ticks, step_by_step.age_ticks,
        "one stride aged it {} and a hundred and forty-four aged it {}",
        in_one_go.age_ticks, step_by_step.age_ticks
    );
    assert_eq!(
        in_one_go.growth_stage, step_by_step.growth_stage,
        "one stride left it {:?} and a hundred and forty-four left it {:?}",
        in_one_go.growth_stage, step_by_step.growth_stage
    );
}

/// A plant something is standing on does not wait four months for its zone.
#[test]
fn ground_somebody_is_standing_on_is_brought_up_to_date() {
    use crate::world::{Grid, Terrain, TerrainType};

    // Ground that will actually keep a plant, so that what is being measured
    // is the catching up and not the tile.
    let mut grid = Grid::new(8, 8);
    for row in grid.tiles.iter_mut() {
        for tile in row.iter_mut() {
            tile.terrain = Terrain::new(TerrainType::Meadow);
        }
    }
    grid.settle_soil();

    let mut plants = PlantManager::new(8);
    plants.spawn_plant("grass".to_string(), (4, 4), 0);

    // Cropped to nothing, the way something grazing would leave it.
    plants.all_plants_mut()[0].current_health = 0.5;
    let cropped = plants.all_plants()[0].current_health;

    // Half a zone's turn later - too soon for its own pass to have come round.
    plants.catch_up_one(0, &mut grid, 60.0, 700, Season::Summer);

    let after = plants.all_plants()[0].current_health;
    assert!(
        after > cropped,
        "a cropped plant with something standing on it put nothing back in \
         seven hundred ticks: {cropped:.3} then {after:.3}"
    );

    assert_eq!(
        plants.all_plants()[0].grown_up_to,
        700,
        "and it did not write down when it was brought up to"
    );
}

}
