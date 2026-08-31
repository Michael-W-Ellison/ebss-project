// src/environment/fauna.rs
//! Animal life and wildlife system with biome distributions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use super::flora::ClimateZone;
use crate::world::{Grid, TerrainType};
use uuid::Uuid;

/// Maps terrain type to the most likely climate zone for that terrain
pub fn terrain_to_climate_zone(terrain: TerrainType) -> ClimateZone {
    match terrain {
        TerrainType::Desert => ClimateZone::Desert,
        // Most terrain types are temperate
        TerrainType::Plains
        | TerrainType::Forest
        | TerrainType::Hills
        | TerrainType::Meadow
        | TerrainType::Wetland
        | TerrainType::Riverbank
        | TerrainType::Beach
        | TerrainType::Farmland
        | TerrainType::Water
        | TerrainType::Sea
        | TerrainType::SaltMarsh => ClimateZone::Temperate,
        // A salt flat is a shallow sea that dried up, and it dried up for a
        // reason
        TerrainType::SaltFlat => ClimateZone::Desert,
        // Mountains can be cold (arctic adjacent)
        TerrainType::Mountain => ClimateZone::Arctic,
    }
}

/// How much longer an animal waits between litters than its species data says.
///
/// The species numbers give a sheep about eight litters in a lifetime and a
/// wolf about seven. At that rate a herd of forty needs some thirty wolves to
/// hold it level - an inverted pyramid, and one the spawn ratio of four prey
/// groups to one predator group can never supply. Stretching the interval
/// brings herd growth back within what a plausible number of predators can
/// take.
const BREEDING_INTERVAL_SCALE: f32 = 3.0;

/// What the sky is doing, for the plants a grazing animal brings up to date.
///
/// Three loose arguments threaded through two calls that have nothing else to
/// do with the weather, so they travel together and are named for why they are
/// here.
#[derive(Debug, Clone, Copy)]
pub struct GrazingWeather {
    pub precipitation: f32,
    pub now: u32,
    pub season: crate::environment::Season,
}

/// Configuration for naturalistic animal spawning during world generation
#[derive(Debug, Clone)]
pub struct AnimalSpawnConfig {
    /// Base number of herds/groups to spawn per 100x100 tiles
    pub herds_per_10000_tiles: usize,
    /// Whether to spawn predators
    pub spawn_predators: bool,
    /// The most head a world starts with, per ten thousand tiles.
    ///
    /// Ten thousand tiles is a square kilometre. This was an absolute - two
    /// hundred head, whatever the map - which never bound on a fifty by fifty
    /// and bound at once on a hundred square kilometres, where it held the
    /// whole country to two animals a square kilometre. It is a ceiling and
    /// not a stocking rate: what a country actually carries is what its grass
    /// will feed, and that is settled by `what_the_grazers_took` within a few
    /// years of a world opening.
    pub head_per_10000_tiles: usize,
}

impl Default for AnimalSpawnConfig {
    fn default() -> Self {
        Self {
            // Eight herds per ten thousand tiles is two herds on the default
            // fifty-by-fifty world: one of prey and one of predators, four
            // sheep and a fox. Nothing balances at that size - the predators
            // die out and the herbivores run to the population cap - so the
            // density is set to give a world that can actually hold a
            // predator and a prey population at once.
            herds_per_10000_tiles: 40,
            spawn_predators: true,
            // Forty head to the square kilometre, which is about where a
            // country of this sort settles once the grass has had its say -
            // measured at fifty-odd head on a hundred and forty-four hectares
            // over a hundred and fifty years. A world that opens near where it
            // settles spends its first decade being a country rather than
            // being a crash.
            head_per_10000_tiles: 10,
        }
    }
}

/// Animal behavior classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalBehavior {
    Passive,    // Flees from threats
    Neutral,    // Ignores unless provoked
    Defensive,  // Attacks when cornered
    Aggressive, // Attacks on sight
    Territorial, // Attacks near den/territory
}

impl AnimalBehavior {
    /// How much of a thing's teeth count against somebody who is simply
    /// standing there.
    ///
    /// A rabbit has an `attack_damage` of 1.0 and a deer of 5.0, because both
    /// will defend themselves if you go at them. Neither is a threat to a man
    /// walking past, and reading danger off `attack_damage` alone said they
    /// were: once several of a thing began adding up, a herd of twenty
    /// reindeer registered as about as dangerous as a wolf.
    ///
    /// What menaces somebody who has done nothing is a thing that comes after
    /// people. What defends itself is a question for whoever attacks it.
    ///
    /// What it cost, measured over twenty-four worlds: a settlement ran 465
    /// times where it should have run 213, and froze 194 times where it should
    /// have frozen 27 - most of that being children hemmed in by deer.
    /// How readily a thing of this temper turns and faces what is coming at
    /// it, rather than running.
    ///
    /// The other side of `how_much_it_menaces_you`, and the whole of an
    /// animal's courage. A rabbit never stands its ground whatever the odds
    /// are - that is what Passive means, and a rabbit that fights a wolf
    /// because the arithmetic came out that way is not a rabbit. Everything
    /// else weighs the odds, and weighs them more kindly the fiercer it is.
    pub fn how_readily_it_stands_its_ground(&self) -> f32 {
        match self {
            AnimalBehavior::Passive => 0.0,
            AnimalBehavior::Neutral => 0.6,
            AnimalBehavior::Defensive => 0.9,
            AnimalBehavior::Aggressive => 1.2,
            AnimalBehavior::Territorial => 1.3,
        }
    }

    pub fn how_much_it_menaces_you(&self) -> f32 {
        match self {
            // Runs away. Not a threat to anybody, whatever it would do if
            // cornered
            AnimalBehavior::Passive => 0.0,
            // Minds its own business, and is worth an eye
            AnimalBehavior::Neutral => 0.25,
            // Will not start it, but is a bad thing to be near
            AnimalBehavior::Defensive => 0.4,
            // Comes after you
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial => 1.0,
        }
    }
}

/// Animal diet type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DietType {
    Herbivore,
    Carnivore,
    Omnivore,
}

impl AnimalSpecies {
    /// Where this one sits in the chain - see [`TrophicRole`].
    ///
    /// Decided by the largest thing it takes, and by its own size where the
    /// two disagree, whichever puts it higher. What actually makes something
    /// an apex predator is that it can bring down large prey, not that it is
    /// large - and in this registry it has to be the prey, because a wolf and
    /// a fox are both `AnimalSize::Small` and the comment on the enum says so
    /// in as many words ("Small: Foxes, wolves"). Their prey lists do tell
    /// them apart: a fox takes rabbits and a wolf takes deer.
    ///
    /// Own size is a floor and never more: it keeps something big that eats
    /// small things off the bottom of the chain, and it cannot on its own put
    /// anything at the top of one.
    pub fn where_it_sits(&self) -> TrophicRole {
        if self.diet == DietType::Herbivore {
            return TrophicRole::PrimaryConsumer;
        }

        // An omnivore with nothing on its list of prey is a forager whatever
        // its teeth say: a boar turning over roots is not hunting anything.
        if self.prey_species.is_empty() {
            return TrophicRole::PrimaryConsumer;
        }

        // What the size of the thing it brings down says about it.
        fn by_what_it_takes(prey: AnimalSize) -> TrophicRole {
            match prey {
                AnimalSize::Tiny => TrophicRole::SmallPredator,
                AnimalSize::Small => TrophicRole::MidPredator,
                AnimalSize::Medium | AnimalSize::Large | AnimalSize::Huge => {
                    TrophicRole::TopPredator
                }
            }
        }

        // And what its own size says, which is only ever a floor and never
        // reaches the top. Nothing is apex by being big: a bear that eats
        // berries and field mice is not what a wolf pack is, and a boar
        // turning up rabbits is a boar. Reading own size on the same scale as
        // prey size put the boar and the harbour seal in with the tigers,
        // because both are `AnimalSize::Medium` and a medium *prey* animal is
        // what an apex predator eats.
        fn at_least(own: AnimalSize) -> TrophicRole {
            match own {
                AnimalSize::Tiny => TrophicRole::SmallPredator,
                _ => TrophicRole::MidPredator,
            }
        }

        // The registry is the only thing that knows how big a named prey
        // species is, and a species does not carry one - so this is answered
        // from a fresh registry rather than from a field that could disagree
        // with the one the world is using.
        let biggest_it_takes = FaunaRegistry::new()
            .all_species()
            .into_iter()
            .filter(|other| self.prey_species.contains(&other.id))
            .map(|other| other.size)
            .max();

        match biggest_it_takes {
            Some(prey) => by_what_it_takes(prey).max(at_least(self.size)),
            None => at_least(self.size),
        }
    }
}

/// Where a species sits in the chain that runs up from the grass.
///
/// Worked out from what it eats and how big it is rather than declared on
/// each species, because those two already say it: a thing that eats plants
/// is a primary consumer, and among the things that eat meat it is size that
/// decides whether it takes voles, hares or deer. A thirty-fourth
/// hand-written field on thirty-three species is thirty-three chances to say
/// something the other fields already contradict.
///
/// The distinction matters because a country does not hold the same number of
/// each. There are a great many grazers, fewer things that eat mice, fewer
/// again that eat rabbits, and a handful at most that eat deer - and the last
/// of those only where there is enough ground to be worth a territory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrophicRole {
    /// Eats plants: the grazers, the browsers, the seed-eaters.
    PrimaryConsumer,
    /// Amphibians, reptiles, small birds, the smaller mustelids.
    SmallPredator,
    /// Foxes, raptors, snakes, the mesocarnivores.
    MidPredator,
    /// Wolves, the big cats, bears. Wants a great deal of country.
    TopPredator,
}

impl TrophicRole {
    /// Every tier, from the grass upward.
    pub const EVERY_ONE: [TrophicRole; 4] = [
        TrophicRole::PrimaryConsumer,
        TrophicRole::SmallPredator,
        TrophicRole::MidPredator,
        TrophicRole::TopPredator,
    ];

    /// What share of a country's herds and packs are of this tier.
    ///
    /// A pyramid, because that is what a food chain is: what eats has to be
    /// rarer than what it eats, and each step up is rarer again. It was a flat
    /// two-to-one of prey to predators, which put a third of everything on
    /// four legs into the business of eating the other two thirds and made no
    /// distinction at all between a fox and a wolf.
    pub fn share_of_a_country(&self) -> f32 {
        match self {
            TrophicRole::PrimaryConsumer => 0.70,
            TrophicRole::SmallPredator => 0.18,
            TrophicRole::MidPredator => 0.09,
            TrophicRole::TopPredator => 0.03,
        }
    }

    /// How much country, in square kilometres, a map must hold before this
    /// tier belongs on it at all - or `None` if there is no such bar.
    ///
    /// Only the top of the chain is held to this, and that is deliberate. It
    /// is the one tier the specification singles out ("only where habitat
    /// scale supports them"), and it is the one whose territory is large
    /// enough that a map of the size this model is usually run at is a pen
    /// rather than a window on a country: a quarter of a square kilometre
    /// with a wolf pack on it is not a small ecosystem, it is an enclosure,
    /// and the wolves eat everything in it and then starve. A fox on the same
    /// ground is a fox whose range runs off the edge of the map, which is
    /// every animal in this model and is nobody's problem.
    pub fn how_much_country_before_it_belongs(&self) -> Option<f32> {
        match self {
            TrophicRole::TopPredator => Some(20.0),
            _ => None,
        }
    }
}

/// Size classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnimalSize {
    Tiny,      // Rabbits, squirrels
    Small,     // Foxes, wolves
    Medium,    // Deer, sheep
    Large,     // Bears, cattle
    Huge,      // Mammoths, elephants
}

impl AnimalSize {
    /// How common a thing of this size is, against the others.
    ///
    /// A country holds a great many rabbits, a fair number of deer, and a few
    /// cattle. Stocking a map by drawing evenly from the list of herbivores
    /// says the opposite - that mammoths and rabbits are equally likely - and
    /// on a small map, where there are only a handful of herds to deal out,
    /// what came of it was a quarter of a square kilometre carrying cows, elk
    /// and mammoths and not one rabbit or squirrel.
    ///
    /// That is not only odd to look at. Every predator below a wolf in this
    /// registry lives on rabbits, squirrels and fish, so a country with no
    /// small herbivores in it has nothing for a fox to eat, and the whole
    /// middle of the chain goes missing from the map.
    pub fn how_common_a_thing_this_size_is(&self) -> usize {
        match self {
            AnimalSize::Tiny => 16,
            AnimalSize::Small => 8,
            AnimalSize::Medium => 4,
            AnimalSize::Large => 2,
            AnimalSize::Huge => 1,
        }
    }
}

/// An animal species
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalSpecies {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Combat/health stats
    pub health: f32,
    pub attack_damage: f32,
    pub defense: f32,
    pub speed: f32, // Movement speed multiplier

    /// Behavior
    pub behavior: AnimalBehavior,
    pub diet: DietType,
    pub size: AnimalSize,

    /// Habitat
    pub primary_biomes: Vec<ClimateZone>,
    pub secondary_biomes: Vec<ClimateZone>,
    pub group_size: (u32, u32), // Min, max herd/pack size

    /// Drops when hunted/killed
    pub drops: Vec<AnimalDrop>,

    /// Whether this animal can be domesticated
    pub can_domesticate: bool,
    /// Products from living animal (milk, wool, eggs)
    pub living_products: Vec<AnimalProduct>,

    // === LIFECYCLE FIELDS ===
    /// Lifespan in ticks (min, max) - animals die of old age
    pub lifespan: (u32, u32),
    /// Age at which animal reaches maturity
    pub maturity_age: u32,
    /// Breeding cooldown in ticks after reproduction
    pub breeding_cooldown: u32,
    /// Gestation period in ticks (0 for egg-layers)
    pub gestation_period: u32,
    /// Number of offspring per birth (min, max)
    pub litter_size: (u32, u32),
    /// Hunger rate - how fast hunger increases per tick
    pub hunger_rate: f32,
    /// Max hunger before starvation damage begins
    pub max_hunger: f32,
    /// Food value when eaten (for prey animals)
    pub food_value: f32,
    /// Prey species IDs this carnivore/omnivore can hunt
    pub prey_species: Vec<String>,

    // === MIGRATION FIELDS ===
    /// Whether this species migrates seasonally
    pub is_migratory: bool,
    /// Preferred migration direction (dx, dy) per season change
    pub migration_direction: (i32, i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalDrop {
    pub material_id: String,
    pub min_quantity: u32,
    pub max_quantity: u32,
    pub drop_chance: f32, // 0.0 to 1.0
}

impl AnimalDrop {
    pub fn new(material_id: String, min: u32, max: u32) -> Self {
        Self {
            material_id,
            min_quantity: min,
            max_quantity: max,
            drop_chance: 1.0,
        }
    }

    pub fn with_chance(mut self, chance: f32) -> Self {
        self.drop_chance = chance.clamp(0.0, 1.0);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalProduct {
    pub material_id: String,
    pub production_time: u32, // Ticks between production
    pub quantity: u32,
}

/// Animal species database
#[derive(Debug, Clone)]
pub struct FaunaRegistry {
    species: BTreeMap<String, AnimalSpecies>,
}

impl FaunaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            species: BTreeMap::new(),
        };

        registry.register_all_species();
        registry
    }

    fn register(&mut self, species: AnimalSpecies) {
        self.species.insert(species.id.clone(), species);
    }

    pub fn get(&self, id: &str) -> Option<&AnimalSpecies> {
        self.species.get(id)
    }

    pub fn get_by_biome(&self, biome: ClimateZone) -> Vec<&AnimalSpecies> {
        self.species
            .values()
            .filter(|s| s.primary_biomes.contains(&biome) || s.secondary_biomes.contains(&biome))
            .collect()
    }

    pub fn get_by_behavior(&self, behavior: AnimalBehavior) -> Vec<&AnimalSpecies> {
        self.species
            .values()
            .filter(|s| s.behavior == behavior)
            .collect()
    }

    fn register_all_species(&mut self) {
        // Tiny passive animals
        self.register(rabbit());
        self.register(squirrel());
        self.register(chicken());
        self.register(duck());
        self.register(goose());

        // Birds
        self.register(crow());
        self.register(eagle());
        self.register(hawk());
        self.register(owl());
        self.register(parrot());

        // Small animals
        self.register(fox());
        self.register(wolf());
        self.register(snake());

        // Medium herbivores (domesticable)
        self.register(deer());
        self.register(sheep());
        self.register(goat());
        self.register(elk_animal());
        self.register(reindeer_animal());

        // Medium/Large omnivores
        self.register(boar());
        self.register(pig());
        self.register(cow());

        // Large predators
        self.register(bear());
        self.register(lion());
        self.register(tiger());
        self.register(crocodile());

        // Arctic/Desert/Tropical specialists
        self.register(arctic_fox());
        self.register(polar_bear());
        self.register(camel());
        self.register(mammoth());
        self.register(monkey());

        // Aquatic
        self.register(fish());
        self.register(otter());
        self.register(seal());
    }

    pub fn all_species(&self) -> Vec<&AnimalSpecies> {
        self.species.values().collect()
    }

    pub fn get_domesticable(&self) -> Vec<&AnimalSpecies> {
        self.species
            .values()
            .filter(|s| s.can_domesticate)
            .collect()
    }
}

// ============================================================================
// TINY PASSIVE ANIMALS
// ============================================================================

fn rabbit() -> AnimalSpecies {
    AnimalSpecies {
        id: "rabbit".to_string(),
        name: "Rabbit".to_string(),
        description: "Small, quick herbivore, common in grasslands".to_string(),
        health: 15.0,
        attack_damage: 1.0,
        defense: 0.0,
        speed: 1.5,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        group_size: (1, 3),
        drops: vec![
            AnimalDrop::new("rabbit_meat".to_string(), 1, 2),
            AnimalDrop::new("fur".to_string(), 1, 1),
            AnimalDrop::new("leather".to_string(), 1, 1).with_chance(0.7),
        ],
        can_domesticate: true,
        living_products: vec![],
        // Lifecycle
        lifespan: (8000, 12000),      // Short-lived
        maturity_age: 500,             // Mature quickly
        breeding_cooldown: 300,        // Breed often
        gestation_period: 200,         // Quick gestation
        litter_size: (3, 8),           // Large litters
        hunger_rate: 0.15,             // High metabolism
        max_hunger: 100.0,
        food_value: 15.0,              // Small prey
        prey_species: vec![],          // Herbivore
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn squirrel() -> AnimalSpecies {
    AnimalSpecies {
        id: "squirrel".to_string(),
        name: "Squirrel".to_string(),
        description: "Nimble tree-dweller, stores nuts for winter".to_string(),
        health: 10.0,
        attack_damage: 0.5,
        defense: 0.0,
        speed: 1.8,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("squirrel_meat".to_string(), 1, 1),
            AnimalDrop::new("fur".to_string(), 1, 1),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (6000, 10000),
        maturity_age: 400,
        breeding_cooldown: 500,
        gestation_period: 250,
        litter_size: (2, 5),
        hunger_rate: 0.12,
        max_hunger: 80.0,
        food_value: 10.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn chicken() -> AnimalSpecies {
    AnimalSpecies {
        id: "chicken".to_string(),
        name: "Chicken".to_string(),
        description: "Common fowl, easily domesticated for eggs and meat".to_string(),
        health: 12.0,
        attack_damage: 0.5,
        defense: 0.0,
        speed: 1.2,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("chicken_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 3, 5),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 100,
                quantity: 1,
            },
        ],
        lifespan: (5000, 8000),
        maturity_age: 300,
        breeding_cooldown: 200,
        gestation_period: 0, // Egg layer
        litter_size: (1, 1), // Eggs handled separately
        hunger_rate: 0.1,
        max_hunger: 80.0,
        food_value: 12.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// SMALL ANIMALS
// ============================================================================

fn fox() -> AnimalSpecies {
    AnimalSpecies {
        id: "fox".to_string(),
        name: "Fox".to_string(),
        description: "Cunning predator, hunts small game".to_string(),
        health: 30.0,
        attack_damage: 8.0,
        defense: 2.0,
        speed: 1.6,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("fox_meat".to_string(), 2, 3),
            AnimalDrop::new("fur".to_string(), 2, 3),
            AnimalDrop::new("leather".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (10000, 15000),
        maturity_age: 800,
        breeding_cooldown: 1000,
        gestation_period: 400,
        litter_size: (2, 5),
        hunger_rate: 0.08,
        max_hunger: 150.0,
        food_value: 30.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string(), "chicken".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn wolf() -> AnimalSpecies {
    AnimalSpecies {
        id: "wolf".to_string(),
        name: "Wolf".to_string(),
        description: "Pack hunter, dangerous in groups".to_string(),
        health: 45.0,
        attack_damage: 12.0,
        defense: 3.0,
        speed: 1.7,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (3, 7),
        drops: vec![
            AnimalDrop::new("wolf_meat".to_string(), 3, 5),
            AnimalDrop::new("fur".to_string(), 3, 4),
            AnimalDrop::new("leather".to_string(), 2, 3),
            AnimalDrop::new("wolf_fang".to_string(), 1, 2).with_chance(0.8),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 1000,
        breeding_cooldown: 1500,
        gestation_period: 500,
        litter_size: (3, 6),
        hunger_rate: 0.06,
        max_hunger: 200.0,
        food_value: 45.0,
        prey_species: vec!["rabbit".to_string(), "deer".to_string(), "sheep".to_string(), "goat".to_string()],
        is_migratory: true, // Wolves follow prey herds
        migration_direction: (0, -20), // Move south in winter
    }
}

// ============================================================================
// MEDIUM HERBIVORES
// ============================================================================

fn deer() -> AnimalSpecies {
    AnimalSpecies {
        id: "deer".to_string(),
        name: "Deer".to_string(),
        description: "Graceful herbivore, provides quality leather and meat".to_string(),
        health: 60.0,
        attack_damage: 5.0,
        defense: 1.0,
        speed: 1.8,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (2, 6),
        drops: vec![
            AnimalDrop::new("deer_meat".to_string(), 8, 12),
            AnimalDrop::new("leather".to_string(), 4, 6),
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (15000, 25000),
        maturity_age: 1200,
        breeding_cooldown: 2000,
        gestation_period: 800,
        litter_size: (1, 2),
        hunger_rate: 0.05,
        max_hunger: 200.0,
        food_value: 60.0,
        prey_species: vec![],
        is_migratory: true, // Deer migrate seasonally
        migration_direction: (0, -15), // Move south in winter
    }
}

fn sheep() -> AnimalSpecies {
    AnimalSpecies {
        id: "sheep".to_string(),
        name: "Sheep".to_string(),
        description: "Docile wool-producing livestock".to_string(),
        health: 50.0,
        attack_damage: 2.0,
        defense: 1.0,
        speed: 1.0,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        group_size: (4, 12),
        drops: vec![
            AnimalDrop::new("mutton".to_string(), 6, 10),
            AnimalDrop::new("leather".to_string(), 3, 5),
            AnimalDrop::new("wool".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "wool".to_string(),
                production_time: 600,
                quantity: 4,
            },
        ],
        lifespan: (12000, 18000),
        maturity_age: 800,
        breeding_cooldown: 1500,
        gestation_period: 600,
        litter_size: (1, 3),
        hunger_rate: 0.04,
        max_hunger: 180.0,
        food_value: 50.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn goat() -> AnimalSpecies {
    AnimalSpecies {
        id: "goat".to_string(),
        name: "Goat".to_string(),
        description: "Hardy mountain animal, produces milk and leather".to_string(),
        health: 55.0,
        attack_damage: 6.0,
        defense: 2.0,
        speed: 1.3,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("goat_meat".to_string(), 5, 8),
            AnimalDrop::new("leather".to_string(), 4, 6),
            AnimalDrop::new("horn".to_string(), 2, 2).with_chance(0.7),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 200,
                quantity: 1,
            },
        ],
        lifespan: (14000, 20000),
        maturity_age: 900,
        breeding_cooldown: 1200,
        gestation_period: 550,
        litter_size: (1, 3),
        hunger_rate: 0.045,
        max_hunger: 170.0,
        food_value: 55.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// MEDIUM/LARGE OMNIVORES & LIVESTOCK
// ============================================================================

fn boar() -> AnimalSpecies {
    AnimalSpecies {
        id: "boar".to_string(),
        name: "Wild Boar".to_string(),
        description: "Aggressive omnivore with thick hide, dangerous when provoked".to_string(),
        health: 80.0,
        attack_damage: 15.0,
        defense: 5.0,
        speed: 1.4,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Omnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (1, 4),
        drops: vec![
            AnimalDrop::new("pork".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 4, 6),
            AnimalDrop::new("leather".to_string(), 3, 5),
            AnimalDrop::new("boar_tusk".to_string(), 2, 2).with_chance(0.6),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 1000,
        breeding_cooldown: 1500,
        gestation_period: 500,
        litter_size: (4, 8),
        hunger_rate: 0.06,
        max_hunger: 220.0,
        food_value: 80.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()], // Omnivore
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn cow() -> AnimalSpecies {
    AnimalSpecies {
        id: "cow".to_string(),
        name: "Cow".to_string(),
        description: "Large domesticated livestock, provides milk, meat, and leather".to_string(),
        health: 100.0,
        attack_damage: 8.0,
        defense: 3.0,
        speed: 0.9,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (2, 8),
        drops: vec![
            AnimalDrop::new("beef".to_string(), 15, 25),
            AnimalDrop::new("leather".to_string(), 8, 12),
            AnimalDrop::new("thick_hide".to_string(), 2, 4),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 150,
                quantity: 2,
            },
        ],
        lifespan: (18000, 28000),
        maturity_age: 1500,
        breeding_cooldown: 2500,
        gestation_period: 900,
        litter_size: (1, 1),
        hunger_rate: 0.04,
        max_hunger: 300.0,
        food_value: 100.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// LARGE PREDATORS
// ============================================================================

fn bear() -> AnimalSpecies {
    AnimalSpecies {
        id: "bear".to_string(),
        name: "Bear".to_string(),
        description: "Massive predator, extremely dangerous, provides thick fur and hide".to_string(),
        health: 200.0,
        attack_damage: 30.0,
        defense: 8.0,
        speed: 1.3,
        behavior: AnimalBehavior::Territorial,
        diet: DietType::Omnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("bear_meat".to_string(), 20, 30),
            AnimalDrop::new("fur".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 8, 12),
            AnimalDrop::new("leather".to_string(), 6, 10),
            AnimalDrop::new("bear_claw".to_string(), 4, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (25000, 35000),
        maturity_age: 2000,
        breeding_cooldown: 4000,
        gestation_period: 800,
        litter_size: (1, 3),
        hunger_rate: 0.03,
        max_hunger: 400.0,
        food_value: 200.0,
        prey_species: vec!["deer".to_string(), "sheep".to_string(), "boar".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn lion() -> AnimalSpecies {
    AnimalSpecies {
        id: "lion".to_string(),
        name: "Lion".to_string(),
        description: "Apex predator of hot climates, hunts in prides".to_string(),
        health: 180.0,
        attack_damage: 28.0,
        defense: 6.0,
        speed: 1.9,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Desert],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("lion_meat".to_string(), 18, 25),
            AnimalDrop::new("fur".to_string(), 8, 12),
            AnimalDrop::new("thick_hide".to_string(), 6, 10),
            AnimalDrop::new("lion_fang".to_string(), 2, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (20000, 30000),
        maturity_age: 1800,
        breeding_cooldown: 3000,
        gestation_period: 700,
        litter_size: (1, 4),
        hunger_rate: 0.035,
        max_hunger: 350.0,
        food_value: 180.0,
        prey_species: vec!["deer".to_string(), "goat".to_string(), "camel".to_string(), "boar".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// BIOME SPECIALISTS
// ============================================================================

fn arctic_fox() -> AnimalSpecies {
    AnimalSpecies {
        id: "arctic_fox".to_string(),
        name: "Arctic Fox".to_string(),
        description: "White-furred fox adapted to extreme cold".to_string(),
        health: 35.0,
        attack_damage: 7.0,
        defense: 2.0,
        speed: 1.7,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("fox_meat".to_string(), 2, 3),
            AnimalDrop::new("fur".to_string(), 3, 5),
            AnimalDrop::new("leather".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (8000, 12000),
        maturity_age: 600,
        breeding_cooldown: 800,
        gestation_period: 350,
        litter_size: (3, 8),
        hunger_rate: 0.09,
        max_hunger: 140.0,
        food_value: 35.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn camel() -> AnimalSpecies {
    AnimalSpecies {
        id: "camel".to_string(),
        name: "Camel".to_string(),
        description: "Desert beast of burden, stores water and provides transport".to_string(),
        health: 120.0,
        attack_damage: 10.0,
        defense: 4.0,
        speed: 1.1,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Desert],
        secondary_biomes: vec![],
        group_size: (2, 6),
        drops: vec![
            AnimalDrop::new("camel_meat".to_string(), 15, 20),
            AnimalDrop::new("leather".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 4, 6),
            AnimalDrop::new("fur".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "milk".to_string(),
                production_time: 300,
                quantity: 1,
            },
        ],
        lifespan: (30000, 50000),
        maturity_age: 2500,
        breeding_cooldown: 4000,
        gestation_period: 1000,
        litter_size: (1, 1),
        hunger_rate: 0.02, // Low metabolism - desert adapted
        max_hunger: 400.0,
        food_value: 120.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn mammoth() -> AnimalSpecies {
    AnimalSpecies {
        id: "mammoth".to_string(),
        name: "Woolly Mammoth".to_string(),
        description: "Massive ice age giant with long tusks and thick fur".to_string(),
        health: 300.0,
        attack_damage: 40.0,
        defense: 10.0,
        speed: 0.8,
        behavior: AnimalBehavior::Territorial,
        diet: DietType::Herbivore,
        size: AnimalSize::Huge,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (2, 8),
        drops: vec![
            AnimalDrop::new("mammoth_meat".to_string(), 40, 60),
            AnimalDrop::new("fur".to_string(), 20, 30),
            AnimalDrop::new("thick_hide".to_string(), 15, 25),
            AnimalDrop::new("leather".to_string(), 10, 15),
            AnimalDrop::new("ivory_tusk".to_string(), 2, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (50000, 80000),
        maturity_age: 5000,
        breeding_cooldown: 8000,
        gestation_period: 2000,
        litter_size: (1, 1),
        hunger_rate: 0.025,
        max_hunger: 600.0,
        food_value: 300.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// ADDITIONAL DOMESTIC ANIMALS
// ============================================================================

fn duck() -> AnimalSpecies {
    AnimalSpecies {
        id: "duck".to_string(),
        name: "Duck".to_string(),
        description: "Waterfowl, provides eggs, meat, and feathers".to_string(),
        health: 10.0,
        attack_damage: 0.5,
        defense: 0.0,
        speed: 1.3,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (4, 10),
        drops: vec![
            AnimalDrop::new("duck_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 120,
                quantity: 1,
            },
        ],
        lifespan: (5000, 8000),
        maturity_age: 300,
        breeding_cooldown: 200,
        gestation_period: 0,
        litter_size: (1, 1),
        hunger_rate: 0.1,
        max_hunger: 70.0,
        food_value: 10.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn goose() -> AnimalSpecies {
    AnimalSpecies {
        id: "goose".to_string(),
        name: "Goose".to_string(),
        description: "Large waterfowl, aggressive when defending territory".to_string(),
        health: 15.0,
        attack_damage: 3.0,
        defense: 1.0,
        speed: 1.2,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![],
        group_size: (5, 12),
        drops: vec![
            AnimalDrop::new("goose_meat".to_string(), 3, 4),
            AnimalDrop::new("feathers".to_string(), 6, 10),
        ],
        can_domesticate: true,
        living_products: vec![
            AnimalProduct {
                material_id: "egg".to_string(),
                production_time: 150,
                quantity: 1,
            },
        ],
        lifespan: (6000, 10000),
        maturity_age: 350,
        breeding_cooldown: 250,
        gestation_period: 0,
        litter_size: (1, 1),
        hunger_rate: 0.08,
        max_hunger: 90.0,
        food_value: 15.0,
        prey_species: vec![],
        is_migratory: true, // Geese are classic migratory birds
        migration_direction: (0, -30), // Fly far south in winter
    }
}

fn pig() -> AnimalSpecies {
    AnimalSpecies {
        id: "pig".to_string(),
        name: "Pig".to_string(),
        description: "Domesticated boar, excellent meat source".to_string(),
        health: 60.0,
        attack_damage: 5.0,
        defense: 2.0,
        speed: 1.1,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Omnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (3, 8),
        drops: vec![
            AnimalDrop::new("pork".to_string(), 12, 18),
            AnimalDrop::new("leather".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 800,
        breeding_cooldown: 1200,
        gestation_period: 400,
        litter_size: (6, 12),
        hunger_rate: 0.07,
        max_hunger: 200.0,
        food_value: 60.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// BIRDS
// ============================================================================

fn crow() -> AnimalSpecies {
    AnimalSpecies {
        id: "crow".to_string(),
        name: "Crow".to_string(),
        description: "Intelligent scavenger bird, often found near settlements".to_string(),
        health: 8.0,
        attack_damage: 2.0,
        defense: 0.0,
        speed: 2.0,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert, ClimateZone::Arctic],
        group_size: (3, 15),
        drops: vec![
            AnimalDrop::new("crow_meat".to_string(), 1, 1),
            AnimalDrop::new("feathers".to_string(), 2, 3),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (10000, 15000),
        maturity_age: 400,
        breeding_cooldown: 500,
        gestation_period: 0,
        litter_size: (3, 6),
        hunger_rate: 0.12,
        max_hunger: 60.0,
        food_value: 8.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn eagle() -> AnimalSpecies {
    AnimalSpecies {
        id: "eagle".to_string(),
        name: "Eagle".to_string(),
        description: "Majestic bird of prey, hunts from great heights".to_string(),
        health: 25.0,
        attack_damage: 10.0,
        defense: 1.0,
        speed: 2.5,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Desert, ClimateZone::Arctic],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 4, 6),
            AnimalDrop::new("eagle_talon".to_string(), 2, 2).with_chance(0.8),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (20000, 35000),
        maturity_age: 1500,
        breeding_cooldown: 3000,
        gestation_period: 0,
        litter_size: (1, 3),
        hunger_rate: 0.06,
        max_hunger: 120.0,
        food_value: 25.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn hawk() -> AnimalSpecies {
    AnimalSpecies {
        id: "hawk".to_string(),
        name: "Hawk".to_string(),
        description: "Swift predatory bird, can be trained for hunting".to_string(),
        health: 20.0,
        attack_damage: 8.0,
        defense: 1.0,
        speed: 2.3,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Desert],
        secondary_biomes: vec![],
        group_size: (1, 2),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 2, 3),
            AnimalDrop::new("feathers".to_string(), 3, 5),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (15000, 25000),
        maturity_age: 1000,
        breeding_cooldown: 2000,
        gestation_period: 0,
        litter_size: (2, 4),
        hunger_rate: 0.07,
        max_hunger: 100.0,
        food_value: 20.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn owl() -> AnimalSpecies {
    AnimalSpecies {
        id: "owl".to_string(),
        name: "Owl".to_string(),
        description: "Nocturnal hunter, silent and deadly".to_string(),
        health: 18.0,
        attack_damage: 7.0,
        defense: 1.0,
        speed: 2.0,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 1, 2),
            AnimalDrop::new("feathers".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (12000, 20000),
        maturity_age: 800,
        breeding_cooldown: 1500,
        gestation_period: 0,
        litter_size: (2, 5),
        hunger_rate: 0.08,
        max_hunger: 90.0,
        food_value: 18.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn parrot() -> AnimalSpecies {
    AnimalSpecies {
        id: "parrot".to_string(),
        name: "Parrot".to_string(),
        description: "Colorful tropical bird, intelligent and vocal".to_string(),
        health: 12.0,
        attack_damage: 3.0,
        defense: 0.0,
        speed: 1.8,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Omnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        group_size: (2, 8),
        drops: vec![
            AnimalDrop::new("bird_meat".to_string(), 1, 2),
            AnimalDrop::new("feathers".to_string(), 4, 6),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (30000, 60000), // Parrots live very long
        maturity_age: 1500,
        breeding_cooldown: 2000,
        gestation_period: 0,
        litter_size: (2, 4),
        hunger_rate: 0.09,
        max_hunger: 80.0,
        food_value: 12.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// MORE PREDATORS
// ============================================================================

fn snake() -> AnimalSpecies {
    AnimalSpecies {
        id: "snake".to_string(),
        name: "Snake".to_string(),
        description: "Venomous reptile, dangerous despite small size".to_string(),
        health: 20.0,
        attack_damage: 15.0,
        defense: 1.0,
        speed: 1.2,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Desert, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("snake_meat".to_string(), 2, 3),
            AnimalDrop::new("snake_skin".to_string(), 1, 2),
            AnimalDrop::new("venom_sac".to_string(), 1, 1).with_chance(0.5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (15000, 25000),
        maturity_age: 1000,
        breeding_cooldown: 2000,
        gestation_period: 0, // Egg layer
        litter_size: (5, 20),
        hunger_rate: 0.02, // Very low - can go long without eating
        max_hunger: 200.0,
        food_value: 20.0,
        prey_species: vec!["rabbit".to_string(), "squirrel".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn tiger() -> AnimalSpecies {
    AnimalSpecies {
        id: "tiger".to_string(),
        name: "Tiger".to_string(),
        description: "Apex predator of jungles, solitary and deadly".to_string(),
        health: 190.0,
        attack_damage: 32.0,
        defense: 7.0,
        speed: 2.0,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Temperate],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("tiger_meat".to_string(), 18, 25),
            AnimalDrop::new("fur".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 6, 10),
            AnimalDrop::new("tiger_fang".to_string(), 2, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (18000, 28000),
        maturity_age: 2000,
        breeding_cooldown: 4000,
        gestation_period: 700,
        litter_size: (2, 4),
        hunger_rate: 0.04,
        max_hunger: 380.0,
        food_value: 190.0,
        prey_species: vec!["deer".to_string(), "boar".to_string(), "goat".to_string(), "monkey".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn crocodile() -> AnimalSpecies {
    AnimalSpecies {
        id: "crocodile".to_string(),
        name: "Crocodile".to_string(),
        description: "Ancient reptilian predator, lurks in water".to_string(),
        health: 150.0,
        attack_damage: 35.0,
        defense: 12.0,
        speed: 0.9,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Desert],
        group_size: (1, 3),
        drops: vec![
            AnimalDrop::new("crocodile_meat".to_string(), 15, 20),
            AnimalDrop::new("crocodile_scales".to_string(), 10, 15),
            AnimalDrop::new("thick_hide".to_string(), 8, 12),
            AnimalDrop::new("crocodile_tooth".to_string(), 4, 8),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (50000, 80000), // Crocodiles live very long
        maturity_age: 3000,
        breeding_cooldown: 5000,
        gestation_period: 0, // Egg layer
        litter_size: (20, 50),
        hunger_rate: 0.015, // Very low metabolism
        max_hunger: 500.0,
        food_value: 150.0,
        prey_species: vec!["deer".to_string(), "goat".to_string(), "fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn polar_bear() -> AnimalSpecies {
    AnimalSpecies {
        id: "polar_bear".to_string(),
        name: "Polar Bear".to_string(),
        description: "Massive arctic predator, adapted to extreme cold".to_string(),
        health: 220.0,
        attack_damage: 35.0,
        defense: 9.0,
        speed: 1.4,
        behavior: AnimalBehavior::Aggressive,
        diet: DietType::Carnivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (1, 1),
        drops: vec![
            AnimalDrop::new("bear_meat".to_string(), 22, 35),
            AnimalDrop::new("fur".to_string(), 15, 20),
            AnimalDrop::new("thick_hide".to_string(), 10, 15),
            AnimalDrop::new("bear_claw".to_string(), 4, 4),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (25000, 40000),
        maturity_age: 2500,
        breeding_cooldown: 5000,
        gestation_period: 900,
        litter_size: (1, 3),
        hunger_rate: 0.025,
        max_hunger: 450.0,
        food_value: 220.0,
        prey_species: vec!["seal".to_string(), "fish".to_string(), "reindeer".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// LARGE HERBIVORES (MOUNT-COMPATIBLE)
// ============================================================================

fn elk_animal() -> AnimalSpecies {
    AnimalSpecies {
        id: "elk".to_string(),
        name: "Elk".to_string(),
        description: "Large forest herbivore with impressive antlers".to_string(),
        health: 90.0,
        attack_damage: 12.0,
        defense: 3.0,
        speed: 1.6,
        behavior: AnimalBehavior::Defensive,
        diet: DietType::Herbivore,
        size: AnimalSize::Large,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (3, 10),
        drops: vec![
            AnimalDrop::new("elk_meat".to_string(), 12, 18),
            AnimalDrop::new("leather".to_string(), 6, 10),
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.6),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (18000, 28000),
        maturity_age: 1500,
        breeding_cooldown: 2500,
        gestation_period: 850,
        litter_size: (1, 2),
        hunger_rate: 0.045,
        max_hunger: 250.0,
        food_value: 90.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn reindeer_animal() -> AnimalSpecies {
    AnimalSpecies {
        id: "reindeer".to_string(),
        name: "Reindeer".to_string(),
        description: "Arctic herbivore, adapted to snow and cold".to_string(),
        health: 70.0,
        attack_damage: 8.0,
        defense: 2.0,
        speed: 1.7,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Herbivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (5, 20),
        drops: vec![
            AnimalDrop::new("reindeer_meat".to_string(), 10, 15),
            AnimalDrop::new("leather".to_string(), 5, 8),
            AnimalDrop::new("fur".to_string(), 4, 6),
            AnimalDrop::new("antler".to_string(), 2, 2).with_chance(0.7),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (15000, 22000),
        maturity_age: 1200,
        breeding_cooldown: 2000,
        gestation_period: 750,
        litter_size: (1, 1),
        hunger_rate: 0.05,
        max_hunger: 200.0,
        food_value: 70.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// TROPICAL SPECIALISTS
// ============================================================================

fn monkey() -> AnimalSpecies {
    AnimalSpecies {
        id: "monkey".to_string(),
        name: "Monkey".to_string(),
        description: "Agile tree-dweller, intelligent and mischievous".to_string(),
        health: 25.0,
        attack_damage: 5.0,
        defense: 1.0,
        speed: 1.9,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Omnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Tropical],
        secondary_biomes: vec![],
        group_size: (5, 15),
        drops: vec![
            AnimalDrop::new("monkey_meat".to_string(), 2, 4),
            AnimalDrop::new("fur".to_string(), 1, 2),
        ],
        can_domesticate: true,
        living_products: vec![],
        lifespan: (20000, 35000),
        maturity_age: 1500,
        breeding_cooldown: 2000,
        gestation_period: 500,
        litter_size: (1, 2),
        hunger_rate: 0.1,
        max_hunger: 120.0,
        food_value: 25.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// AQUATIC ANIMALS
// ============================================================================

fn fish() -> AnimalSpecies {
    AnimalSpecies {
        id: "fish".to_string(),
        name: "Fish".to_string(),
        description: "Common fish, found in rivers and lakes".to_string(),
        health: 5.0,
        attack_damage: 0.0,
        defense: 0.0,
        speed: 1.5,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Carnivore,
        size: AnimalSize::Tiny,
        primary_biomes: vec![ClimateZone::Temperate, ClimateZone::Tropical],
        secondary_biomes: vec![ClimateZone::Arctic],
        group_size: (10, 50),
        drops: vec![
            AnimalDrop::new("fish_meat".to_string(), 1, 2),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (3000, 8000),
        maturity_age: 200,
        breeding_cooldown: 100,
        gestation_period: 0, // Spawn eggs
        litter_size: (50, 200), // Many eggs
        hunger_rate: 0.05,
        max_hunger: 50.0,
        food_value: 5.0,
        prey_species: vec![],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn otter() -> AnimalSpecies {
    AnimalSpecies {
        id: "otter".to_string(),
        name: "Otter".to_string(),
        description: "Playful aquatic mammal, hunts fish".to_string(),
        health: 30.0,
        attack_damage: 6.0,
        defense: 1.0,
        speed: 1.6,
        behavior: AnimalBehavior::Neutral,
        diet: DietType::Carnivore,
        size: AnimalSize::Small,
        primary_biomes: vec![ClimateZone::Temperate],
        secondary_biomes: vec![ClimateZone::Tropical],
        group_size: (2, 6),
        drops: vec![
            AnimalDrop::new("otter_meat".to_string(), 3, 5),
            AnimalDrop::new("fur".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (12000, 18000),
        maturity_age: 800,
        breeding_cooldown: 1500,
        gestation_period: 400,
        litter_size: (1, 4),
        hunger_rate: 0.1,
        max_hunger: 130.0,
        food_value: 30.0,
        prey_species: vec!["fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

fn seal() -> AnimalSpecies {
    AnimalSpecies {
        id: "seal".to_string(),
        name: "Seal".to_string(),
        description: "Arctic aquatic mammal, thick blubber provides warmth".to_string(),
        health: 80.0,
        attack_damage: 8.0,
        defense: 4.0,
        speed: 1.1,
        behavior: AnimalBehavior::Passive,
        diet: DietType::Carnivore,
        size: AnimalSize::Medium,
        primary_biomes: vec![ClimateZone::Arctic],
        secondary_biomes: vec![],
        group_size: (3, 12),
        drops: vec![
            AnimalDrop::new("seal_meat".to_string(), 10, 15),
            AnimalDrop::new("blubber".to_string(), 8, 12),
            AnimalDrop::new("fur".to_string(), 4, 6),
            AnimalDrop::new("leather".to_string(), 3, 5),
        ],
        can_domesticate: false,
        living_products: vec![],
        lifespan: (25000, 40000),
        maturity_age: 2000,
        breeding_cooldown: 3000,
        gestation_period: 800,
        litter_size: (1, 1),
        hunger_rate: 0.04,
        max_hunger: 250.0,
        food_value: 80.0,
        prey_species: vec!["fish".to_string()],
        is_migratory: false,
        migration_direction: (0, 0),
    }
}

// ============================================================================
// ANIMAL INSTANCE SYSTEM (Individual animals in the world)
// ============================================================================

/// AI state for animal behavior
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalState {
    /// Wandering aimlessly
    Idle,
    /// Moving towards food/grazing
    Grazing,
    /// Seeking water
    Drinking,
    /// Resting to recover health/stamina
    Resting,
    /// Following herd/pack
    Following,
    /// Hunting prey
    Hunting { target_id: Option<Uuid> },
    /// Fleeing from danger
    Fleeing { from_position: (i32, i32) },
    /// Attacking threat
    Attacking { target_id: Uuid },
    /// Dead
    Dead,
}

/// Individual animal instance in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animal {
    pub id: Uuid,
    pub species_id: String,

    /// Position in world
    pub position: (i32, i32),
    pub facing: f32, // Direction in radians

    /// Stats
    pub current_health: f32,
    pub max_health: f32,
    pub stamina: f32,
    pub max_stamina: f32,

    /// AI state
    pub state: AnimalState,
    pub state_timer: u32, // Ticks remaining in current state

    /// Herd/pack affiliation
    pub group_id: Option<Uuid>,

    /// Age (in ticks)
    pub age: u32,
    pub maturity_age: u32, // Age when fully grown
    pub max_lifespan: u32, // Maximum age before death

    /// Domestication
    pub is_domesticated: bool,
    pub tame_level: f32, // 0.0 = wild, 1.0 = fully tamed
    pub owner_id: Option<Uuid>, // Agent who owns this animal

    /// Reproduction
    pub can_reproduce: bool,
    pub reproduction_cooldown: u32,
    pub is_pregnant: bool,
    pub pregnancy_timer: u32, // Ticks until birth
    pub mate_id: Option<Uuid>, // For tracking lineage

    /// Hunger/feeding system
    pub hunger: f32,        // Current hunger level (0 = full, max = starving)
    pub max_hunger: f32,    // Max hunger before starvation damage
    pub hunger_rate: f32,   // Hunger increase per tick
    pub is_starving: bool,  // Taking starvation damage

    /// Living product timers
    pub product_timers: BTreeMap<String, u32>, // material_id -> ticks until production
}

impl Animal {
    pub fn new(species_id: String, position: (i32, i32), species: &AnimalSpecies) -> Self {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let mut product_timers = BTreeMap::new();
        for product in &species.living_products {
            product_timers.insert(product.material_id.clone(), product.production_time);
        }

        // Calculate random lifespan within species range
        let max_lifespan = rng.gen_range(species.lifespan.0..=species.lifespan.1);

        Self {
            id: crate::core::dice::name(),
            species_id,
            position,
            facing: 0.0,
            current_health: species.health,
            max_health: species.health,
            stamina: 100.0,
            max_stamina: 100.0,
            state: AnimalState::Idle,
            state_timer: 0,
            group_id: None,
            age: 0,
            maturity_age: species.maturity_age,
            max_lifespan,
            is_domesticated: false,
            tame_level: 0.0,
            owner_id: None,
            can_reproduce: true,
            reproduction_cooldown: 0,
            is_pregnant: false,
            pregnancy_timer: 0,
            mate_id: None,
            hunger: 0.0,
            max_hunger: species.max_hunger,
            hunger_rate: species.hunger_rate,
            is_starving: false,
            product_timers,
        }
    }

    /// Create a newborn animal (starts at age 0, inherits some traits)
    pub fn new_offspring(species_id: String, position: (i32, i32), species: &AnimalSpecies, parent_group: Option<Uuid>) -> Self {
        let mut offspring = Self::new(species_id, position, species);
        offspring.group_id = parent_group;
        // Newborns start with some hunger
        offspring.hunger = offspring.max_hunger * 0.3;
        offspring
    }

    /// Check if animal is alive
    pub fn is_alive(&self) -> bool {
        self.current_health > 0.0 && self.state != AnimalState::Dead
    }

    /// Check if animal is mature (can reproduce, full stats)
    pub fn is_mature(&self) -> bool {
        self.age >= self.maturity_age
    }

    /// Check if animal is wild (not domesticated)
    pub fn is_wild(&self) -> bool {
        !self.is_domesticated
    }

    /// Damage the animal
    pub fn take_damage(&mut self, amount: f32) {
        self.current_health = (self.current_health - amount).max(0.0);
        if self.current_health == 0.0 {
            self.state = AnimalState::Dead;
        }
    }

    /// Heal the animal
    pub fn heal(&mut self, amount: f32) {
        if self.is_alive() {
            self.current_health = (self.current_health + amount).min(self.max_health);
        }
    }

    /// Consume stamina
    pub fn use_stamina(&mut self, amount: f32) {
        self.stamina = (self.stamina - amount).max(0.0);
    }

    /// Recover stamina
    pub fn recover_stamina(&mut self, amount: f32) {
        self.stamina = (self.stamina + amount).min(self.max_stamina);
    }

    /// Check if exhausted
    pub fn is_exhausted(&self) -> bool {
        self.stamina < 20.0
    }

    /// Tame the animal (increase tame level)
    pub fn tame(&mut self, amount: f32) {
        if !self.is_domesticated {
            self.tame_level = (self.tame_level + amount).min(1.0);
            if self.tame_level >= 1.0 {
                self.is_domesticated = true;
            }
        }
    }

    /// Tick product production timers and return ready products
    pub fn tick_products(&mut self) -> Vec<(String, u32)> {
        let mut produced = Vec::new();

        if !self.is_alive() || !self.is_mature() {
            return produced;
        }

        for (material_id, timer) in self.product_timers.iter_mut() {
            if *timer > 0 {
                *timer -= 1;
            } else {
                // Find the product info to get quantity
                // We'll return the material_id and quantity
                produced.push((material_id.clone(), 1)); // Default quantity
                *timer = 100; // Reset timer (will be updated with actual value)
            }
        }

        produced
    }

    /// Age the animal by one tick
    pub fn tick_age(&mut self) {
        self.age += 1;

        // Update reproduction cooldown
        if self.reproduction_cooldown > 0 {
            self.reproduction_cooldown -= 1;
        }

        // Update pregnancy timer
        if self.is_pregnant && self.pregnancy_timer > 0 {
            self.pregnancy_timer -= 1;
        }
    }

    /// Check if animal has died of old age
    pub fn is_too_old(&self) -> bool {
        self.age >= self.max_lifespan
    }

    /// Check if animal can breed (mature, not pregnant, cooldown expired)
    pub fn can_breed(&self) -> bool {
        self.is_alive()
            && self.is_mature()
            && self.can_reproduce
            && !self.is_pregnant
            && self.reproduction_cooldown == 0
            && !self.is_starving
            // Well fed, not merely coping. At seven tenths of everything it
            // can hold an animal still counts as fit to breed, which let both
            // herds and packs go on multiplying through the lean stretch that
            // should have stopped them.
            && self.hunger < self.max_hunger * 0.4
    }

    /// Start pregnancy with gestation period
    pub fn become_pregnant(&mut self, gestation_period: u32, breeding_cooldown: u32) {
        self.is_pregnant = true;
        self.pregnancy_timer = gestation_period;
        self.reproduction_cooldown = breeding_cooldown;
    }

    /// Check if ready to give birth
    pub fn ready_to_give_birth(&self) -> bool {
        self.is_pregnant && self.pregnancy_timer == 0
    }

    /// Complete birth and reset pregnancy state
    pub fn give_birth(&mut self) {
        self.is_pregnant = false;
        self.pregnancy_timer = 0;
    }

    /// Increase hunger by the animal's hunger rate
    pub fn tick_hunger(&mut self) {
        if !self.is_alive() {
            return;
        }

        self.hunger = (self.hunger + self.hunger_rate).min(self.max_hunger * 1.5);

        // Check starvation threshold
        if self.hunger >= self.max_hunger {
            self.is_starving = true;
            // Take starvation damage proportional to how hungry
            let starvation_damage = (self.hunger - self.max_hunger) * 0.1;
            self.take_damage(starvation_damage);
        } else {
            self.is_starving = false;
        }
    }

    /// Feed the animal, reducing hunger
    pub fn feed(&mut self, amount: f32) {
        self.hunger = (self.hunger - amount).max(0.0);
        if self.hunger < self.max_hunger {
            self.is_starving = false;
        }
    }

    /// Check if animal is hungry enough to seek food
    pub fn is_hungry(&self) -> bool {
        self.hunger > self.max_hunger * 0.5
    }

    /// Check if animal is very hungry (urgent food seeking)
    pub fn is_very_hungry(&self) -> bool {
        self.hunger > self.max_hunger * 0.8
    }

    /// Whether a predator will bother going after something it sees.
    ///
    /// Much lower than `is_hungry`, which is half of everything the animal can
    /// hold. A predator that only hunted when it was half starved killed about
    /// one animal in a thousand ticks - far below what the herds breed - so it
    /// stayed hungry, never bred, and the herbivores it was supposed to be
    /// holding down ran to the population cap. A predator that is not nearly
    /// full will take what is in front of it.
    pub fn will_hunt(&self) -> bool {
        self.hunger > self.max_hunger * 0.15
    }

    /// Get health percentage
    pub fn health_percentage(&self) -> f32 {
        self.current_health / self.max_health
    }

    /// Get stamina percentage
    pub fn stamina_percentage(&self) -> f32 {
        self.stamina / self.max_stamina
    }
}

// ============================================================================
// ANIMAL MANAGER (Manages all animals in the world)
// ============================================================================

/// Manages animal population and AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimalManager {
    animals: Vec<Animal>,

    /// What the grazers have taken off the map altogether, for measuring.
    #[serde(default)]
    forage_taken: f64,
    /// How many animal-passes ended in a mouthful, and how many tried.
    #[serde(default)]
    mouths_fed: u64,
    #[serde(default)]
    mouths_that_tried: u64,

    groups: BTreeMap<Uuid, Vec<Uuid>>, // Group ID -> Animal IDs

    /// Spawning parameters
    spawn_rate: f32, // Chance per tick to spawn
    max_population: usize,

    /// The most of each species this world has ever held, which is what a
    /// depleted population is judged against
    #[serde(default)]
    peak_population: BTreeMap<String, u32>,

    /// Size of the map, so animals wandering in from outside know where the
    /// edge is
    #[serde(default)]
    world_bounds: Option<(i32, i32)>,

    /// Ticks since the last time anything was allowed to wander in
    #[serde(default)]
    ticks_since_migration: u32,

    /// Reference to fauna registry (not serialized)
    #[serde(skip)]
    registry: Option<FaunaRegistry>,
}

impl AnimalManager {
    pub fn new(max_population: usize) -> Self {
        Self {
            animals: Vec::new(),
            forage_taken: 0.0,
            mouths_fed: 0,
            mouths_that_tried: 0,
            groups: BTreeMap::new(),
            spawn_rate: 0.001, // 0.1% chance per tick
            max_population,
            peak_population: BTreeMap::new(),
            world_bounds: None,
            ticks_since_migration: 0,
            registry: Some(FaunaRegistry::new()),
        }
    }

    /// Spawn an animal at a position
    pub fn spawn_animal(&mut self, species_id: String, position: (i32, i32)) -> Option<Uuid> {
        if self.how_many_are_alive() >= self.max_population {
            return None;
        }

        let species = self.registry.as_ref()?.get(&species_id)?;
        let animal = Animal::new(species_id, position, species);
        let id = animal.id;
        self.animals.push(animal);
        Some(id)
    }

    /// Spawn a herd/pack of animals
    /// Put a group of one species on the map.
    ///
    /// Whether there is room for them is the caller's question, not this
    /// one's - `process_breeding`, `spawn_initial_population` and
    /// `process_immigration` each ask it before they get here, and each means
    /// something slightly different by it. This used to ask again on its own
    /// account, which quietly overrode the one caller that had a reason to
    /// say yes: a species that is *gone* is let back into a full map on
    /// purpose - see `process_immigration` - and every one of its arrivals
    /// was refused here.
    pub fn spawn_group(&mut self, species_id: String, center: (i32, i32), count: u32) -> Option<Uuid> {
        let group_id = crate::core::dice::name();
        let mut members = Vec::new();

        let species = self.registry.as_ref()?.get(&species_id)?;

        for i in 0..count {
            // Spawn in a circle around center
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let radius = 3.0;
            let x = center.0 + (angle.cos() * radius) as i32;
            let y = center.1 + (angle.sin() * radius) as i32;

            let mut animal = Animal::new(species_id.clone(), (x, y), species);
            animal.group_id = Some(group_id);

            members.push(animal.id);
            self.animals.push(animal);
        }

        if !members.is_empty() {
            self.groups.insert(group_id, members);
            Some(group_id)
        } else {
            None
        }
    }

    /// Get all animals
    /// What the grazers have taken off the map, how many animal-passes ended
    /// in a mouthful, and how many looked. For measuring only.
    pub fn what_the_grazing_came_to(&self) -> (f64, u64, u64) {
        (self.forage_taken, self.mouths_fed, self.mouths_that_tried)
    }

    pub fn get_all(&self) -> &Vec<Animal> {
        &self.animals
    }

    /// How many animals this world actually holds.
    ///
    /// Not `self.animals.len()`, which is how many animal *records* exist -
    /// and every one of those checks was asking the wrong question, because
    /// nothing ever took a dead animal out of the list. Measured on a world
    /// with nobody in it at all, twenty years in: **898 records of which 9.8
    /// were alive**. The corpses filled `max_population`, so nothing could be
    /// born and nothing could migrate in, the boom cohort aged out together,
    /// and **seventeen of twenty species were extinct in every world**. A map
    /// with no people on it emptied itself of animals.
    ///
    /// One owner, because the cap is asked about in seven places and all
    /// seven meant the living. See ISSUES_FOUND.md #127.
    pub fn how_many_are_alive(&self) -> usize {
        self.animals.iter().filter(|animal| animal.is_alive()).count()
    }

    /// Take the dead off the map.
    ///
    /// A body is read exactly once, in the tick it falls: whatever killed it
    /// looks at it there and then - a predator to feed, a hunter to butcher -
    /// and nothing wants it afterwards. Leaving them in the list made the
    /// world's animal population a tally of everything that had ever lived in
    /// it.
    ///
    /// If something is one day wanted that eats what nobody stood over - see
    /// the open note on a kill left lying - it will want the bodies kept for
    /// a while, and this is where that would go.
    fn bury_the_dead(&mut self) {
        self.animals.retain(|animal| animal.is_alive());
    }

    /// All animals, mutably
    pub fn get_all_mut(&mut self) -> &mut Vec<Animal> {
        &mut self.animals
    }

    /// Get specific animal
    pub fn get(&self, id: &Uuid) -> Option<&Animal> {
        self.animals.iter().find(|a| a.id == *id)
    }

    /// Get mutable animal
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Animal> {
        self.animals.iter_mut().find(|a| a.id == *id)
    }

    /// Get all animals at a position
    pub fn get_at_position(&self, position: (i32, i32)) -> Vec<&Animal> {
        self.animals.iter()
            .filter(|a| a.position == position && a.is_alive())
            .collect()
    }

    /// Get animals in radius
    pub fn get_in_radius(&self, center: (i32, i32), radius: f32) -> Vec<&Animal> {
        self.animals.iter()
            .filter(|a| {
                if !a.is_alive() {
                    return false;
                }
                let dx = (a.position.0 - center.0) as f32;
                let dy = (a.position.1 - center.1) as f32;
                (dx * dx + dy * dy).sqrt() <= radius
            })
            .collect()
    }



    /// Get species from registry
    pub fn get_species(&self, species_id: &str) -> Option<&AnimalSpecies> {
        self.registry.as_ref()?.get(species_id)
    }





    /// Tick all animals (age, products, natural healing, AI behaviors, lifecycle)
    /// A tick of everything with legs, in the world it is standing in.
    ///
    /// It took nothing before, which is how grazing came to feed every animal
    /// out of thin air: there was no ground and no vegetation to take from,
    /// so what a mouthful was worth came down to a headcount per patch
    /// standing in for the food that should have been doing the work. What
    /// sets the size of a herd now is what is growing where it is standing.
    /// `grazing_ticks` is how many ticks of feeding this pass stands for, and
    /// nought means "not this tick". Grazing runs on the same ten-tick
    /// cadence the vegetation does, because it has to look up what is growing
    /// on each tile and building that lookup is a pass over every plant in the
    /// world - eighty thousand of them on a hundred square kilometres, which
    /// at every tick was three-quarters of what a tick cost. It also has to be
    /// the *same* pass, because the lookup holds indices into the plant list
    /// and that list is rebuilt whenever anything dies.
    pub fn tick_in_world(
        &mut self,
        grid: &mut crate::world::Grid,
        plants: &mut crate::environment::PlantManager,
        grazing_ticks: f32,
        weather: GrazingWeather,
    ) {
        if self.registry.is_none() {
            return;
        }

        // First pass: basic updates and lifecycle
        let mut deaths_from_age = Vec::new();
        for (idx, animal) in self.animals.iter_mut().enumerate() {
            if !animal.is_alive() {
                continue;
            }

            // Age
            animal.tick_age();

            // Check for death from old age
            if animal.is_too_old() {
                deaths_from_age.push(idx);
                continue;
            }

            // Hunger system
            animal.tick_hunger();

            // Natural stamina recovery when resting
            if animal.state == AnimalState::Resting {
                animal.recover_stamina(1.0);
            } else if animal.state != AnimalState::Dead {
                // Gradual stamina consumption for active animals
                animal.use_stamina(0.1);
            }

            // Slow natural healing (if not starving)
            if animal.current_health < animal.max_health && !animal.is_starving {
                animal.heal(0.1);
            }

            // Tick products
            animal.tick_products();

            // Decrement state timer
            if animal.state_timer > 0 {
                animal.state_timer -= 1;
            }
        }

        // Kill animals that died of old age
        for idx in deaths_from_age.iter().rev() {
            if let Some(animal) = self.animals.get_mut(*idx) {
                animal.state = AnimalState::Dead;
                animal.current_health = 0.0;
            }
        }

        // Second pass: Births (process pregnant animals ready to give birth)
        self.process_births();

        // Third pass: Breeding attempts
        self.process_breeding();

        // Fourth pass: Predator hunting
        self.process_predation();

        // Animals from beyond the edge of the map, for species that have been
        // wiped out or hunted down to nothing here
        self.process_immigration();

        // Fifth pass: Herbivore feeding - what is taken off the ground, and
        // what goes back onto it
        self.what_the_grazers_took(grid, plants, grazing_ticks, weather);

        // Sixth pass: AI behavior (needs fresh registry borrow)
        let animals_data: Vec<(usize, String, AnimalBehavior, bool, bool)> = {
            let registry = match &self.registry {
                Some(r) => r,
                None => return,
            };
            self.animals
                .iter()
                .enumerate()
                .filter(|(_, a)| a.is_alive())
                .filter_map(|(idx, a)| {
                    let species = registry.get(&a.species_id)?;
                    Some((idx, a.species_id.clone(), species.behavior, a.is_wild(), a.is_hungry()))
                })
                .collect()
        };

        for (idx, _species_id, behavior, is_wild, is_hungry) in animals_data {
            self.update_animal_behavior_with_hunger(idx, behavior, is_wild, is_hungry);
        }

        // And what died this pass goes off the map. Everything that wanted to
        // look at a body has looked at it by now.
        self.bury_the_dead();
    }

    /// What a wild animal does about people.
    ///
    /// Nothing, until now. There was predator hunting in this module and no
    /// other awareness of agents at all, so a deer stood where it stood while
    /// a settlement walked up to it - which is what made a stone-age hunt a
    /// matter of finding an animal rather than of stalking one, and is half of
    /// why food was too easy to come by.
    ///
    /// Most things that live in a wood get out of a person's way. The ones
    /// that do not are the ones that mean to do something about the person:
    /// an aggressive or territorial beast holds its ground, and a tame one
    /// has no reason to run.
    ///
    /// Takes bare positions rather than agents, so that nothing in here has to
    /// know what an agent is.
    pub fn shy_away_from(&mut self, people: &[(i32, i32)]) {
        if people.is_empty() {
            return;
        }

        // Which of them would move off, worked out before the animals are
        // borrowed to be moved. The registry lives beside them and cannot be
        // read while they are held mutably, which is the same dance the AI
        // pass does.
        let skittish: Vec<usize> = {
            let Some(registry) = &self.registry else {
                return;
            };

            self.animals
                .iter()
                .enumerate()
                .filter(|(_, animal)| animal.is_alive() && animal.is_wild())
                .filter(|(_, animal)| {
                    registry.get(&animal.species_id).is_some_and(|species| {
                        matches!(
                            species.behavior,
                            AnimalBehavior::Passive
                                | AnimalBehavior::Neutral
                                | AnimalBehavior::Defensive
                        )
                    })
                })
                .map(|(idx, _)| idx)
                .collect()
        };

        for idx in skittish {
            let animal = &mut self.animals[idx];

            // The nearest person, and only if they are near enough to have
            // been noticed
            let Some(nearest) = people
                .iter()
                .min_by_key(|(x, y)| {
                    (x - animal.position.0).abs().max((y - animal.position.1).abs())
                })
            else {
                continue;
            };

            let how_close = (nearest.0 - animal.position.0)
                .abs()
                .max((nearest.1 - animal.position.1).abs());

            if how_close > Self::NEAR_ENOUGH_TO_SPOOK_IT || how_close == 0 {
                continue;
            }

            // One step directly away. A stone-age hunter is faster over a
            // short dash than a deer is over a long one, which is why hunting
            // works at all; what this does is make him spend the dash.
            let away = |them: i32, it: i32| -> i32 {
                match it.cmp(&them) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Equal => 0,
                }
            };

            animal.position.0 += away(nearest.0, animal.position.0);
            animal.position.1 += away(nearest.1, animal.position.1);
            animal.use_stamina(0.2);
        }
    }

    /// How near somebody has to be before a wild animal thinks better of
    /// standing there.
    ///
    /// A little further than a man can throw, so that walking up to a deer
    /// costs something even when the throw itself would have been easy.
    const NEAR_ENOUGH_TO_SPOOK_IT: i32 = 4;

    /// Process births for pregnant animals
    fn process_births(&mut self) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Collect birth data
        let births: Vec<(String, (i32, i32), Option<Uuid>, u32, u32)> = self.animals
            .iter_mut()
            .filter(|a| a.is_alive() && a.ready_to_give_birth())
            .filter_map(|a| {
                let species = registry.get(&a.species_id)?;
                let litter_size = rng.gen_range(species.litter_size.0..=species.litter_size.1);

                // Complete birth
                a.give_birth();

                Some((
                    a.species_id.clone(),
                    a.position,
                    a.group_id,
                    litter_size,
                    ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32,
                ))
            })
            .collect();

        // Spawn offspring
        for (species_id, position, group_id, litter_size, _cooldown) in births {
            if let Some(species) = registry.get(&species_id) {
                for _ in 0..litter_size {
                    if self.how_many_are_alive() >= self.max_population {
                        break;
                    }

                    // Spawn near parent with some offset
                    let offset_x = rng.gen_range(-2..=2);
                    let offset_y = rng.gen_range(-2..=2);
                    let offspring_pos = (position.0 + offset_x, position.1 + offset_y);

                    let offspring = Animal::new_offspring(
                        species_id.clone(),
                        offspring_pos,
                        species,
                        group_id,
                    );
                    self.animals.push(offspring);
                }
            }
        }
    }

    /// Process breeding attempts for eligible animals
    fn process_breeding(&mut self) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Only attempt breeding occasionally (not every tick)
        if rng.gen::<f32>() > 0.01 {
            return;
        }

        // Find breeding candidates by species.
        //
        // What decides whether the land will carry another is whether the
        // animals already on it are fed, and `can_breed` is where that is
        // asked - it wants hunger under two-fifths of what the animal can
        // stand. Since grazing takes real forage off real plants, a herd that
        // has eaten its ground bare is a hungry herd and a hungry herd does
        // not breed, so carrying capacity comes out of the grass.
        //
        // There used to be a headcount of mouths per six-by-six patch here,
        // with a hard ceiling on it, standing in for exactly that. It had to
        // be there while grazing fed every animal out of thin air; it is a
        // second answer to a question the food now answers, and two answers to
        // one question is how they drift.
        let mut breeding_candidates: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (idx, animal) in self.animals.iter().enumerate() {
            if !animal.can_breed() {
                continue;
            }

            breeding_candidates
                .entry(animal.species_id.clone())
                .or_insert_with(Vec::new)
                .push(idx);
        }

        // For each species with 2+ candidates, attempt breeding
        for (species_id, candidates) in breeding_candidates {
            if candidates.len() < 2 {
                continue;
            }

            // Get species data
            let species = match registry.get(&species_id) {
                Some(s) => s,
                None => continue,
            };

            // Check pairs for proximity (within breeding distance)
            for i in 0..candidates.len() {
                for j in (i + 1)..candidates.len() {
                    let idx_a = candidates[i];
                    let idx_b = candidates[j];

                    let pos_a = self.animals[idx_a].position;
                    let pos_b = self.animals[idx_b].position;

                    // Check proximity. Ten tiles rather than five because
                    // nothing in the model keeps a group together: animals
                    // spawn as a herd and then wander off on their own. A
                    // herd of ten still has pairs in range after that; a pair
                    // of wolves does not, which left predators unable to
                    // breed at all while the herbivores they were supposed to
                    // be holding down ran to the population cap.
                    let distance = ((pos_a.0 - pos_b.0).abs() + (pos_a.1 - pos_b.1).abs()) as f32;
                    if distance <= 10.0 {
                        // Breeding chance based on proximity
                        if rng.gen::<f32>() < 0.3 {
                            // One becomes pregnant (or both go on cooldown for egg-layers)
                            if species.gestation_period > 0 {
                                // Mammal-style: one becomes pregnant
                                self.animals[idx_a].become_pregnant(
                                    species.gestation_period,
                                    ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32,
                                );
                                self.animals[idx_b].reproduction_cooldown = ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32;
                            } else {
                                // Egg-layer: both go on cooldown, eggs spawn immediately
                                self.animals[idx_a].reproduction_cooldown = ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32;
                                self.animals[idx_b].reproduction_cooldown = ((species.breeding_cooldown as f32) * BREEDING_INTERVAL_SCALE) as u32;

                                // Spawn eggs (as new animals with age 0)
                                let litter = rng.gen_range(species.litter_size.0..=species.litter_size.1);
                                for _ in 0..litter {
                                    if self.how_many_are_alive() >= self.max_population {
                                        break;
                                    }
                                    let pos = self.animals[idx_a].position;
                                    let offspring = Animal::new_offspring(
                                        species_id.clone(),
                                        pos,
                                        species,
                                        self.animals[idx_a].group_id,
                                    );
                                    self.animals.push(offspring);
                                }
                            }
                            break; // Only one breeding per species per tick
                        }
                    }
                }
            }
        }
    }

    /// Process predator hunting - carnivores/omnivores hunt prey
    fn process_predation(&mut self) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r,
            None => return,
        };

        // Every hungry predator hunts on its own account.
        //
        // This used to sit behind a single roll for the whole world - one
        // chance in fifty per tick that any predation happened anywhere - so
        // predators were barely a presence and herbivores grew until they hit
        // the population cap. A predator hunts when it is hungry and not
        // otherwise, which is what ties its numbers to the herds.
        const HUNT_ATTEMPT_CHANCE: f32 = 0.05;

        // Find hungry predators and their prey
        let predator_data: Vec<(usize, String, Vec<String>, (i32, i32), f32, bool, AnimalSize)> =
            self.animals
                .iter()
                .enumerate()
                .filter(|(_, a)| a.is_alive() && a.will_hunt())
                .filter_map(|(idx, a)| {
                    let species = registry.get(&a.species_id)?;
                    if species.prey_species.is_empty() {
                        return None;
                    }
                    // The biggest thing it knows how to bring down, which is
                    // what bounds what it will try when it is desperate. A
                    // wolf takes deer, so a goat is fair game; a fox takes
                    // rabbits, and no amount of hunger makes a cow catchable.
                    let usual_limit = species
                        .prey_species
                        .iter()
                        .filter_map(|prey| registry.get(prey))
                        .map(|prey| prey.size)
                        .max()
                        .unwrap_or(species.size);

                    Some((
                        idx,
                        a.species_id.clone(),
                        species.prey_species.clone(),
                        a.position,
                        species.attack_damage,
                        a.is_very_hungry(),
                        usual_limit,
                    ))
                })
                .collect();

        // Who is standing where, in blocks the size of a hunt.
        //
        // A predator used to look at every animal in the world to find one
        // within eight tiles of it, which is every predator against every
        // animal: on a hundred square kilometres carrying four thousand head
        // that is millions of comparisons a tick, most of them string
        // comparisons against a list of prey species, to find the handful of
        // animals actually in front of it. Blocks of `HOW_FAR_A_HUNT_REACHES`
        // mean a predator looks in the nine blocks around it and nowhere else.
        let mut who_is_about: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for (idx, animal) in self.animals.iter().enumerate() {
            if !animal.is_alive() {
                continue;
            }
            who_is_about
                .entry(Self::which_block(animal.position))
                .or_default()
                .push(idx);
        }

        // For each predator, look for nearby prey
        let mut kills = Vec::new();
        for (pred_idx, _pred_species, prey_species, pred_pos, attack, desperate, usual_limit) in
            predator_data
        {
            if rng.gen::<f32>() > HUNT_ATTEMPT_CHANCE {
                continue;
            }

            let hereabouts = Self::which_block(pred_pos);
            let nearby: Vec<usize> = [-1, 0, 1]
                .iter()
                .flat_map(|dy| [-1, 0, 1].iter().map(move |dx| (*dx, *dy)))
                .filter_map(|(dx, dy)| {
                    who_is_about.get(&(hereabouts.0 + dx, hereabouts.1 + dy))
                })
                .flatten()
                .copied()
                .collect();

            // Find nearby prey
            for prey_idx in nearby {
                let prey = &self.animals[prey_idx];
                if !prey.is_alive() || prey_idx == pred_idx {
                    continue;
                }

                // Normally a predator takes what it knows how to take. One
                // that is close to starving takes whatever it can catch: any
                // grazing animal no bigger than itself will do.
                let usual_prey = prey_species.contains(&prey.species_id);
                let worth_trying = usual_prey
                    || (desperate
                        && registry
                            .get(&prey.species_id)
                            .map(|s| s.prey_species.is_empty() && s.size <= usual_limit)
                            .unwrap_or(false));

                if !worth_trying {
                    continue;
                }

                // Check proximity - the blocks are only a sieve, and a
                // neighbouring block reaches further than a hunt does.
                let distance = (pred_pos.0 - prey.position.0).abs()
                    + (pred_pos.1 - prey.position.1).abs();
                if distance > Self::HOW_FAR_A_HUNT_REACHES {
                    continue;
                }

                // Hunt success based on speed comparison and randomness
                let prey_species_data = registry.get(&prey.species_id);
                let prey_speed = prey_species_data.map(|s| s.speed).unwrap_or(1.0);
                let pred_species_data = registry.get(&self.animals[pred_idx].species_id);
                let pred_speed = pred_species_data.map(|s| s.speed).unwrap_or(1.0);

                let chase_chance = (pred_speed / prey_speed).min(1.0) * 0.4;

                if rng.gen::<f32>() < chase_chance {
                    // Successful hunt - attack prey
                    let prey_food_value = prey_species_data.map(|s| s.food_value).unwrap_or(10.0);
                    kills.push((pred_idx, prey_idx, attack, prey_food_value));
                    break; // One hunt per predator per tick
                }
            }
        }

        // Apply kills
        for (pred_idx, prey_idx, damage, food_value) in kills {
            // Damage prey
            if let Some(prey) = self.animals.get_mut(prey_idx) {
                prey.take_damage(damage);
            }

            // If prey died, feed predator
            if let Some(prey) = self.animals.get(prey_idx) {
                if !prey.is_alive() {
                    if let Some(predator) = self.animals.get_mut(pred_idx) {
                        predator.feed(food_value);
                    }
                }
            }
        }
    }

    /// Animals wander in from beyond the edge of the map.
    ///
    /// A species that has been wiped out here, or hunted down to a quarter of
    /// the most this world ever held of it, is not gone from the world
    /// entirely - only from this corner of it - and a few will find their way
    /// back. Only species that have lived here migrate in: the map does not
    /// invent lions for a valley that never had any.
    ///
    /// Deliberately rare. One small group per depleted species every eight
    /// thousand ticks or so, which is a lifetime for most of them. It is meant
    /// to keep a world from emptying out for good, not to be a larder that
    /// refills itself faster than it can be emptied - a settlement that clears
    /// the herds waits a long time for more.
    fn process_immigration(&mut self) {
        use rand::Rng;

        /// How often anything is allowed to arrive at all
        const MIGRATION_INTERVAL: u32 = 2000;

        /// And how often, at those moments, anything actually does
        const MIGRATION_CHANCE: f64 = 0.25;

        /// Below this share of the most this world ever held, a species counts
        /// as needing help
        const DEPLETED_SHARE: f32 = 0.25;

        /// How many arrive at once
        const ARRIVALS: (u32, u32) = (1, 3);

        // What is here now, and the most there has ever been.
        //
        // Counted every pass rather than every two thousand ticks. This used
        // to sit below the interval gate, so a species had to be alive at a
        // migration moment to be remembered at all - anything that came into
        // the world and died inside its first two thousand ticks was recorded
        // as never having lived here, and could never come back. That is what
        // happened to the owl, which was in one world of eight at the start
        // and in none of them ever again. See ISSUES_FOUND.md #127.
        let mut present: BTreeMap<String, u32> = BTreeMap::new();
        for animal in &self.animals {
            if animal.is_alive() {
                *present.entry(animal.species_id.clone()).or_insert(0) += 1;
            }
        }

        for (species_id, count) in &present {
            let peak = self.peak_population.entry(species_id.clone()).or_insert(0);
            *peak = (*peak).max(*count);
        }

        self.ticks_since_migration += 1;
        if self.ticks_since_migration < MIGRATION_INTERVAL {
            return;
        }
        self.ticks_since_migration = 0;

        let bounds = match self.world_bounds {
            Some(bounds) => bounds,
            None => return,
        };

        let depleted: Vec<String> = self
            .peak_population
            .iter()
            .filter(|(species_id, peak)| {
                let here = present.get(*species_id).copied().unwrap_or(0) as f32;
                here < (**peak as f32) * DEPLETED_SHARE
            })
            .map(|(species_id, _)| species_id.clone())
            .collect();

        if depleted.is_empty() {
            return;
        }

        let mut rng = crate::core::dice::roll();

        for species_id in depleted {
            // A species that is *gone* comes back whether the map is full or
            // not. The cap is a rough statement of how much life this country
            // carries, and a country carrying its whole weight in rabbits is
            // exactly the country a fox should walk into - refusing him for
            // want of room is the cap deciding which species exist.
            //
            // Measured before this: with the map pinned at its cap, the
            // immigration pass broke out on the first line every time, so
            // owl and fox went out of every world that had them in the first
            // year and never came back in twenty. A merely thin species still
            // waits for room; an absent one does not.
            let gone = present.get(&species_id).copied().unwrap_or(0) == 0;

            if !gone && self.how_many_are_alive() >= self.max_population {
                continue;
            }

            if !rng.gen_bool(MIGRATION_CHANCE) {
                continue;
            }

            // In from one of the four edges
            let arrival = match rng.gen_range(0..4) {
                0 => (rng.gen_range(0..bounds.0), 0),
                1 => (rng.gen_range(0..bounds.0), bounds.1 - 1),
                2 => (0, rng.gen_range(0..bounds.1)),
                _ => (bounds.0 - 1, rng.gen_range(0..bounds.1)),
            };

            let arriving = rng.gen_range(ARRIVALS.0..=ARRIVALS.1);
            self.spawn_group(species_id, arrival, arriving);
        }
    }

    /// What the grazers took off the ground, and what went back onto it.
    ///
    /// Grazing used to feed every animal out of nothing. There was a crowding
    /// term - a headcount of mouths per patch - standing in for the food that
    /// should have been doing the work, and the comment above `process_breeding`
    /// has said as much since it was written: "grazing feeds every animal
    /// nearly a hundred times what it burns, so hunger never becomes the
    /// limit". Nothing on the map got any smaller for being eaten, so what
    /// stopped a herd growing was a hard number in a field.
    ///
    /// Now a mouthful comes off a plant that is standing there and the plant
    /// is that much less of a plant for it. What is left over after the animal
    /// has taken what it can use lands on the ground behind it - see
    /// `WHAT_AN_ANIMAL_GETS_OUT_OF_A_MOUTHFUL` - so the greater part of what
    /// is grazed comes back to the soil a little further on, which is what
    /// grazing animals are for as far as the ground is concerned.
    fn what_the_grazers_took(
        &mut self,
        grid: &mut crate::world::Grid,
        plants: &mut crate::environment::PlantManager,
        grazing_ticks: f32,
        weather: GrazingWeather,
    ) {
        use crate::world::Position;

        if grazing_ticks <= 0.0 {
            return;
        }

        let registry = match &self.registry {
            Some(r) => r.clone(),
            None => return,
        };

        let flora = crate::environment::FloraRegistry::new();

        // Where the standing growth is. Built once for the pass, and laid out
        // flat rather than as a map keyed by position: asking each animal to
        // search the plant list would be every animal against every plant, and
        // a tree map of eighty thousand entries a pass is not much better -
        // see the canopy in `PlantManager::tick_in_world`, which is the same
        // shape for the same reason. `u32::MAX` means nothing is growing here.
        let (width, height) = (grid.width, grid.height);
        let mut where_it_grows = vec![u32::MAX; width * height];
        for (index, plant) in plants.all_plants().iter().enumerate() {
            let (x, y) = plant.position;
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                where_it_grows[y as usize * width + x as usize] = index as u32;
            }
        }

        // What each plant lost this pass, and whether it was pulled up whole.
        let mut cropped: BTreeMap<usize, f32> = BTreeMap::new();
        let mut pulled_up: std::collections::BTreeSet<usize> =
            std::collections::BTreeSet::new();
        let mut dunged: Vec<((i32, i32), f32)> = Vec::new();
        let mut took_altogether = 0.0f64;
        let mut mouths = 0u64;
        let mut reached = 0u64;

        for animal in &mut self.animals {
            if !animal.is_alive() || animal.state != AnimalState::Grazing {
                continue;
            }

            let Some(species) = registry.get(&animal.species_id) else {
                continue;
            };

            if species.diet == DietType::Carnivore {
                continue;
            }

            let mut wanted = Self::what_it_reaches_for(species) * grazing_ticks;
            let mut taken = 0.0;

            // Underfoot first, then a step in any direction. An animal that is
            // grazing is standing still and eating what is around it, not
            // ranging - the ranging is what `update_animal_behavior_with_hunger`
            // does when it is hungry and there is nothing here.
            for (dx, dy) in Self::WHERE_AN_ANIMAL_CAN_REACH {
                if wanted <= 0.0 {
                    break;
                }

                let (x, y) = (animal.position.0 + dx, animal.position.1 + dy);
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }

                let index = where_it_grows[y as usize * width + x as usize];
                if index == u32::MAX {
                    continue;
                }
                let index = index as usize;

                let plant = &plants.all_plants()[index];
                let Some(kind) = flora.get(&plant.species_id) else {
                    continue;
                };

                // Bring it up to now before taking anything off it. Most of
                // the vegetation waits four months for its zone to come round
                // - see `PlantManager::grow_a_zone` - and a plant something is
                // standing on cannot wait that long, or it would lose
                // condition a hundred and forty-four times for every time it
                // gained any. This is the whole of what "unless there is
                // something within reach of it" means.
                plants.catch_up_one(
                    index,
                    grid,
                    weather.precipitation,
                    weather.now,
                    weather.season,
                );

                let plant = &plants.all_plants()[index];

                let already = cropped.get(&index).copied().unwrap_or(0.0);
                let standing = plant.current_health - already;
                if standing <= 0.0 {
                    continue;
                }

                // A grown tree is browse, not grazing. Nothing eats a trunk;
                // what a deer or a sheep gets off an oak is the shoots and
                // leaves it can reach, which is a mouthful or two and no more
                // however big the tree is - so what a tree offers is a flat
                // small amount rather than a share of its bulk, and cropping
                // it does not touch the tree.
                //
                // Excluding grown trees outright was the first cut and it is
                // wrong on a wooded map: most of what is standing on a fresh
                // map is timber, so twelve sheep on twenty-five hectares had
                // almost nothing in reach, overshot to thirty on the hunger
                // they were born with, and starved.
                let grown_tree = kind.is_tree
                    && !matches!(
                        plant.growth_stage,
                        crate::environment::GrowthStage::Seedling
                            | crate::environment::GrowthStage::Growing
                    );

                let there_to_take = if grown_tree {
                    standing.min(Self::WHAT_A_TREE_OFFERS_A_BROWSER * grazing_ticks - already)
                } else {
                    standing
                };

                if there_to_take <= 0.0 {
                    continue;
                }

                let bite = wanted.min(there_to_take);
                *cropped.entry(index).or_insert(0.0) += bite;
                wanted -= bite;
                taken += bite;

                // A bear does not crop a root, it digs it up, and what has
                // been dug up does not come back. Which animals do that is
                // which animals feed by digging: the big omnivores. It is the
                // manner of the feeding rather than a list of plants that
                // decides it, so nothing here has to keep a hand-written
                // vocabulary of what counts as a root.
                if Self::does_it_dig(species) && !kind.is_tree {
                    pulled_up.insert(index);
                }
            }

            reached += 1;

            if taken <= 0.0 {
                // Nothing within reach. An animal that cannot feed where it
                // is standing walks until it can, and until now nothing in
                // this module did: a hungry animal either stood still or
                // shuffled a cell or two at random once its `state_timer` ran
                // out, which will not carry a herd off ground it has eaten
                // bare. Twelve sheep on a fifty by fifty map cropped their own
                // few tiles to nothing by tick two thousand eight hundred and
                // then took not one further mouthful in three thousand ticks,
                // with six hundred plants and thirty-eight thousand of
                // standing growth on the map around them.
                if let Some(towards) =
                    Self::where_there_is_something_growing(animal.position, &where_it_grows, width, height)
                {
                    animal.position = towards;
                }
                continue;
            }

            animal.feed(taken * Self::WHAT_A_MOUTHFUL_IS_WORTH);
            took_altogether += taken as f64;
            mouths += 1;

            // And what came straight through lands where the animal is now.
            dunged.push((
                animal.position,
                taken * (1.0 - Self::WHAT_AN_ANIMAL_GETS_OUT_OF_A_MOUTHFUL),
            ));
        }

        // Take it off the plants, and take up what was pulled up.
        let standing = plants.all_plants_mut();
        for (index, lost) in cropped {
            if let Some(plant) = standing.get_mut(index) {
                plant.current_health -= lost;
                if pulled_up.contains(&index) {
                    plant.current_health = 0.0;
                }
            }
        }

        self.forage_taken += took_altogether;
        self.mouths_fed += mouths;
        self.mouths_that_tried += reached;

        // The dead go back into the ground on the plants' own pass - see
        // `PlantManager::what_died`, which is what reads a plant at nothing.

        for (at, muck) in dunged {
            let here = Position::new(at.0, at.1);
            if let Some(tile) = grid.get_tile_mut(&here) {
                tile.soil.add_leaf_litter(muck);
            }
        }
    }

    /// The tile an animal is on, and the eight around it.
    const WHERE_AN_ANIMAL_CAN_REACH: [(i32, i32); 9] = [
        (0, 0),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];

    /// How much standing growth an animal reaches for in a tick.
    ///
    /// Worked out from what it costs to be that animal - its own
    /// `hunger_rate` - rather than from a table of appetites by size. Size is
    /// what sets `hunger_rate` in the first place, so a second table by size
    /// is a second answer to one question, and the first cut of this had one:
    /// it set the appetites so that a mouthful came out worth what the old
    /// flat rates gave, which preserved a number the module's own comments
    /// call out as feeding an animal "nearly a hundred times what it burns".
    /// Five thousand seven hundred head on a hundred and forty-four hectares
    /// with a mean hunger of 0.30 - which is to say the grass was still
    /// infinite, only now it was infinite by arithmetic instead of by
    /// omission.
    ///
    /// The margin over what it burns is what lets a well-fed animal put
    /// condition on and get to breeding; on ground that will not give it that
    /// much it takes what there is and stays hungry.
    fn what_it_reaches_for(species: &AnimalSpecies) -> f32 {
        /// How much over its own burn an animal eats when it can.
        const MORE_THAN_IT_BURNS: f32 = 3.0;

        species.hunger_rate * MORE_THAN_IT_BURNS / Self::WHAT_A_MOUTHFUL_IS_WORTH
    }

    /// A step towards the nearest ground with something growing on it.
    ///
    /// Rings outwards from where the animal is standing and stops at the
    /// first thing it finds, so it walks towards the near side of the next
    /// patch rather than across the map. A cell is ten metres, so
    /// `HOW_FAR_AN_ANIMAL_WILL_LOOK` is a couple of hundred metres - about as
    /// far as an animal can see over open ground, and as far as it is worth
    /// spending on the search.
    fn where_there_is_something_growing(
        from: (i32, i32),
        where_it_grows: &[u32],
        width: usize,
        height: usize,
    ) -> Option<(i32, i32)> {
        const HOW_FAR_AN_ANIMAL_WILL_LOOK: i32 = 20;

        for ring in 2..=HOW_FAR_AN_ANIMAL_WILL_LOOK {
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    // Only the edge of this ring; the inside has been looked at.
                    if dx.abs() != ring && dy.abs() != ring {
                        continue;
                    }

                    let (x, y) = (from.0 + dx, from.1 + dy);
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        continue;
                    }
                    if where_it_grows[y as usize * width + x as usize] == u32::MAX {
                        continue;
                    }

                    // A step of the size an animal covers between grazing
                    // passes, in the direction of what it has found.
                    let step = |d: i32| d.signum() * d.abs().min(Self::HOW_FAR_AN_ANIMAL_WALKS);
                    return Some((from.0 + step(dx), from.1 + step(dy)));
                }
            }
        }

        None
    }

    /// How far an animal moves in a grazing pass, in cells.
    ///
    /// Ten ticks is most of a day and a cell is ten metres, so this is a few
    /// hundred metres of walking - which is what a grazing animal does in a
    /// day when the ground it is on has been eaten off.
    const HOW_FAR_AN_ANIMAL_WALKS: i32 = 3;

    /// How much a grown tree gives a browsing animal, per tick.
    ///
    /// A flat amount rather than a share of the tree: what is within reach of
    /// something on four legs is the same handful of shoots whether the tree
    /// is a birch or a sequoia. Enough that a wood will carry a few animals
    /// and nothing like what a meadow carries, which is the right way round.
    const WHAT_A_TREE_OFFERS_A_BROWSER: f32 = 0.05;

    /// How much hunger a point of standing growth answers.
    ///
    /// One for one. Plant condition and animal hunger are both arbitrary
    /// scales and there is no reason to put a rate between them; what matters
    /// is that the two ends are real - how fast a plant puts condition back on
    /// (`HOW_FAST_A_PLANT_COMES_BACK`, scaled by what the ground gives it) and
    /// how fast an animal burns it (`hunger_rate`) - because their ratio is
    /// what a piece of country will carry.
    ///
    /// As it comes out: a grass has five points of condition and puts back
    /// about eight a year on middling ground, and a deer burns two hundred a
    /// year. So a deer wants the whole yield of something like seventy or
    /// eighty patches of sward, and a hundred and forty-four hectares carrying
    /// ten thousand of them will feed deer in the low hundreds.
    const WHAT_A_MOUTHFUL_IS_WORTH: f32 = 1.0;

    /// How much of a mouthful an animal actually gets out of it.
    ///
    /// A grazing animal digests something over half of what it eats and the
    /// rest goes through and lands on the ground behind it. Of the half it
    /// does digest, nearly all is burnt for warmth and movement and leaves as
    /// breath and water; only a few per cent ever becomes animal. So what the
    /// ground gets back is what came straight through, and what it loses for
    /// good is what the animal burned.
    const WHAT_AN_ANIMAL_GETS_OUT_OF_A_MOUTHFUL: f32 = 0.55;

    /// How far a predator will chase, in cells.
    const HOW_FAR_A_HUNT_REACHES: i32 = 8;

    /// The block of country a position falls in, for finding what is near it
    /// without asking about everything that is not.
    fn which_block(at: (i32, i32)) -> (i32, i32) {
        (
            at.0.div_euclid(Self::HOW_FAR_A_HUNT_REACHES),
            at.1.div_euclid(Self::HOW_FAR_A_HUNT_REACHES),
        )
    }

    /// Whether this animal feeds by digging.
    ///
    /// A big omnivore does: a bear turns ground over for roots and grubs and
    /// what it turns over does not grow back. A deer crops what is above the
    /// ground and the plant puts it back.
    fn does_it_dig(species: &AnimalSpecies) -> bool {
        species.diet == DietType::Omnivore
            && matches!(species.size, AnimalSize::Large | AnimalSize::Huge)
    }

    /// Update animal behavior with hunger consideration
    fn update_animal_behavior_with_hunger(&mut self, animal_idx: usize, behavior: AnimalBehavior, is_wild: bool, is_hungry: bool) {
        let animal = &mut self.animals[animal_idx];

        // If state timer is active, continue current behavior
        if animal.state_timer > 0 {
            return;
        }

        // Hungry animals prioritize food seeking
        if is_hungry && animal.is_very_hungry() {
            animal.state = AnimalState::Grazing; // Or hunting for carnivores
            animal.state_timer = 40;
            // Move while seeking food
            let offset = (crate::core::dice::any::<i32>() % 5 - 2, crate::core::dice::any::<i32>() % 5 - 2);
            animal.position.0 += offset.0;
            animal.position.1 += offset.1;
            return;
        }

        // Normal behavior based on type
        match behavior {
            AnimalBehavior::Passive => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 50;
                } else if is_hungry || crate::core::dice::any::<f32>() < 0.3 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 30;
                    let offset = (crate::core::dice::any::<i32>() % 3 - 1, crate::core::dice::any::<i32>() % 3 - 1);
                    animal.position.0 += offset.0;
                    animal.position.1 += offset.1;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 20;
                }
            }
            AnimalBehavior::Neutral => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 40;
                } else if is_hungry {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 35;
                } else if crate::core::dice::any::<f32>() < 0.2 {
                    animal.state = AnimalState::Drinking;
                    animal.state_timer = 25;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 25;
                }
            }
            AnimalBehavior::Defensive => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 45;
                } else if is_hungry || crate::core::dice::any::<f32>() < 0.5 {
                    animal.state = AnimalState::Grazing;
                    animal.state_timer = 35;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 20;
                }
            }
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial => {
                if animal.is_exhausted() {
                    animal.state = AnimalState::Resting;
                    animal.state_timer = 60;
                } else if is_wild && (is_hungry || crate::core::dice::any::<f32>() < 0.3) {
                    animal.state = AnimalState::Hunting { target_id: None };
                    animal.state_timer = 50;
                    let offset = (crate::core::dice::any::<i32>() % 5 - 2, crate::core::dice::any::<i32>() % 5 - 2);
                    animal.position.0 += offset.0;
                    animal.position.1 += offset.1;
                } else {
                    animal.state = AnimalState::Idle;
                    animal.state_timer = 30;
                }
            }
        }
    }

    /// Count living animals
    pub fn population_count(&self) -> usize {
        self.animals.iter().filter(|a| a.is_alive()).count()
    }

    /// Count animals by species
    pub fn count_by_species(&self, species_id: &str) -> usize {
        self.animals.iter()
            .filter(|a| a.species_id == species_id && a.is_alive())
            .count()
    }

    /// Get total living animals by behavior
    pub fn count_by_behavior(&self, behavior: AnimalBehavior) -> usize {
        let registry = match &self.registry {
            Some(r) => r,
            None => return 0,
        };

        self.animals.iter()
            .filter(|a| {
                if !a.is_alive() {
                    return false;
                }
                registry.get(&a.species_id)
                    .map(|s| s.behavior == behavior)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Spawn animals naturalistically based on terrain during world generation
    ///
    /// This method spawns animals in appropriate biomes based on terrain types:
    /// - Herbivores spawn in herds in plains, meadows, and forests
    /// - Predators spawn in smaller numbers
    /// - Aquatic animals spawn near water
    /// - Mountain animals spawn in highlands
    pub fn spawn_naturalistic(&mut self, grid: &Grid, config: &AnimalSpawnConfig) {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        let registry = match &self.registry {
            Some(r) => r.clone(),
            None => return,
        };

        self.world_bounds = Some((grid.width as i32, grid.height as i32));

        let total_tiles = grid.width * grid.height;
        let total_herds = (total_tiles * config.herds_per_10000_tiles) / 10000;
        // Never below what a small map was always allowed, so that turning
        // this from an absolute into a density does not quietly empty the
        // fifty by fifty every test is built on: a quarter of a square
        // kilometre at ten head to the kilometre is two animals, where the
        // herd counts alone put thirty-odd on it and always have. On anything
        // big enough for the density to mean something, the density wins.
        const WHAT_EVEN_A_SMALL_MAP_MAY_HOLD: usize = 200;

        let at_the_very_outside = ((total_tiles * config.head_per_10000_tiles) / 10000)
            .max(WHAT_EVEN_A_SMALL_MAP_MAY_HOLD);

        // And what each tier of it may hold, by the same shares the herds are
        // dealt out on. One pool, filled first-come, meant the grazers spent
        // the whole of it before anything that eats them was placed at all -
        // which is why a hundred square kilometres came out with a thousand
        // head on it and not one wolf.
        let room_for = |role: TrophicRole| -> usize {
            ((at_the_very_outside as f32) * role.share_of_a_country()) as usize
        };

        // Categorize species by diet for balanced spawning
        let herbivores: Vec<_> = registry.all_species()
            .into_iter()
            .filter(|s| s.diet == DietType::Herbivore)
            .collect();
        let predators: Vec<_> = registry.all_species()
            .into_iter()
            .filter(|s| s.diet == DietType::Carnivore || s.diet == DietType::Omnivore)
            .filter(|s| !s.prey_species.is_empty())
            .collect();

        if herbivores.is_empty() {
            return;
        }

        // How many herds and packs of each tier this country carries.
        //
        // A pyramid rather than a ratio - see `TrophicRole::share_of_a_country`
        // - and one that the size of the map can veto at the top: what wants
        // twenty square kilometres to a territory has no business on a quarter
        // of one. A hundred square kilometres carries wolves; a fifty by fifty
        // test map carries foxes and nothing above them, which is what "only
        // where habitat scale supports them" comes to.
        let country = grid.how_much_ground();

        let mut how_many_of: BTreeMap<TrophicRole, usize> = BTreeMap::new();
        for role in TrophicRole::EVERY_ONE {
            // At least one group of anything that belongs here at all. Three
            // hundredths of the ten herds a fifty by fifty gets is nought
            // groups, and rounding a whole tier out of existence is not the
            // same statement as the map being too small for it - the veto
            // below is the thing that means that, and it should be the only
            // thing that does.
            let wanted = ((total_herds as f32 * role.share_of_a_country()) as usize).max(1);

            let holds = match role.how_much_country_before_it_belongs() {
                Some(room) if country < room => 0,
                _ => wanted,
            };

            how_many_of.insert(role, holds);
        }

        if !config.spawn_predators {
            for role in TrophicRole::EVERY_ONE {
                if role != TrophicRole::PrimaryConsumer {
                    how_many_of.insert(role, 0);
                }
            }
        }

        let prey_herds = how_many_of
            .get(&TrophicRole::PrimaryConsumer)
            .copied()
            .unwrap_or(0);

        // Collect terrain positions by climate zone
        let mut positions_by_climate: BTreeMap<ClimateZone, Vec<(i32, i32)>> = BTreeMap::new();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let terrain = grid.tiles[y][x].terrain.terrain_type;
                // Skip water for land animals
                if terrain == TerrainType::Water {
                    continue;
                }
                let climate = terrain_to_climate_zone(terrain);
                positions_by_climate.entry(climate)
                    .or_insert_with(Vec::new)
                    .push((x as i32, y as i32));
            }
        }

        // The sorts of ground a species lives on that this map actually has.
        //
        // Drawing a climate out of the species and then asking whether the map
        // has any of it loses the animal when it has not: the pack asked for
        // is thrown away rather than put somewhere it could live. On a small
        // map most of the registry's biomes are absent, so a fifty by fifty
        // came out with no predators at all - one pack wanted, one draw, and
        // the draw was an arctic fox.
        let ground_for = |species: &AnimalSpecies| -> Vec<ClimateZone> {
            let wanted: Vec<ClimateZone> = if species.primary_biomes.is_empty() {
                vec![ClimateZone::Temperate]
            } else {
                species.primary_biomes.clone()
            };

            wanted
                .into_iter()
                .filter(|climate| {
                    positions_by_climate
                        .get(climate)
                        .map(|ground| !ground.is_empty())
                        .unwrap_or(false)
                })
                .collect()
        };

        // Spawn herbivore herds
        let mut spawned = 0;
        let grazers_may_have = room_for(TrophicRole::PrimaryConsumer).max(1);
        let mut prey_present: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

        // Only the ones that could live here, so that a herd asked for is a
        // herd placed - and each of them entered as many times as a thing of
        // its size is common, so that the draw is not a coin between a rabbit
        // and a mammoth. See `AnimalSize::how_common_a_thing_this_size_is`.
        let herbivores: Vec<_> = herbivores
            .into_iter()
            .filter(|species| !ground_for(species).is_empty())
            .flat_map(|species| {
                std::iter::repeat(species).take(species.size.how_common_a_thing_this_size_is())
            })
            .collect();
        if herbivores.is_empty() {
            return;
        }

        for _ in 0..prey_herds {
            if spawned >= grazers_may_have
                || self.how_many_are_alive() >= self.max_population
            {
                break;
            }

            // Pick a random herbivore species
            let species = &herbivores[rng.gen_range(0..herbivores.len())];

            // Find a position in an appropriate biome
            let at_home = ground_for(species);
            let climate = at_home[rng.gen_range(0..at_home.len())];

            if let Some(positions) = positions_by_climate.get(&climate) {
                if !positions.is_empty() {
                    let pos = positions[rng.gen_range(0..positions.len())];
                    let herd_size = rng.gen_range(species.group_size.0..=species.group_size.1);

                    if let Some(_) = self.spawn_group(species.id.clone(), pos, herd_size) {
                        spawned += herd_size as usize;
                        prey_present.insert(species.id.clone());
                    }
                }
            }
        }

        // Spawn predator groups (smaller).
        //
        // Only predators that eat something living here. Drawing the two lists
        // independently put foxes, which eat rabbits and squirrels, into
        // worlds of sheep and cattle: they never found a meal in eight
        // thousand ticks, their hunger climbed in a straight line from birth
        // to death, and the herds they should have been holding down ran to
        // the population cap unopposed.
        let feedable: Vec<_> = predators
            .iter()
            .filter(|species| {
                species
                    .prey_species
                    .iter()
                    .any(|prey| prey_present.contains(prey))
            })
            .collect();

        if config.spawn_predators && !feedable.is_empty() {
            // Each tier drawn from its own kind, so that thinning the top of
            // the pyramid does not thin the bottom of it as well. Drawing them
            // all from one bag meant that a map too small for wolves simply
            // had fewer foxes.
            let mut packs = Vec::new();
            for role in TrophicRole::EVERY_ONE {
                if role == TrophicRole::PrimaryConsumer {
                    continue;
                }

                let of_this_tier: Vec<_> = feedable
                    .iter()
                    .copied()
                    .filter(|species| species.where_it_sits() == role)
                    .filter(|species| !ground_for(species).is_empty())
                    .collect();

                if of_this_tier.is_empty() {
                    continue;
                }

                for _ in 0..how_many_of.get(&role).copied().unwrap_or(0) {
                    packs.push(of_this_tier[rng.gen_range(0..of_this_tier.len())]);
                }
            }

            let mut put_down_of: BTreeMap<TrophicRole, usize> = BTreeMap::new();

            for species in packs {
                if self.how_many_are_alive() >= self.max_population {
                    break;
                }

                let role = species.where_it_sits();
                let already = put_down_of.get(&role).copied().unwrap_or(0);
                if already >= room_for(role).max(1) {
                    continue;
                }

                // Find a position in an appropriate biome
                let at_home = ground_for(species);
                let climate = at_home[rng.gen_range(0..at_home.len())];

                if let Some(positions) = positions_by_climate.get(&climate) {
                    if !positions.is_empty() {
                        let pos = positions[rng.gen_range(0..positions.len())];
                        // Predator packs are typically smaller
                        let pack_size = rng.gen_range(1..=species.group_size.1.min(4));

                        if let Some(_) = self.spawn_group(species.id.clone(), pos, pack_size) {
                            spawned += pack_size as usize;
                            *put_down_of.entry(role).or_insert(0) += pack_size as usize;
                        }
                    }
                }
            }
        }
    }

    /// Get a summary of spawned animals by species
    pub fn population_summary(&self) -> BTreeMap<String, usize> {
        let mut summary = BTreeMap::new();
        for animal in &self.animals {
            if animal.is_alive() {
                *summary.entry(animal.species_id.clone()).or_insert(0) += 1;
            }
        }
        summary
    }


}

impl Default for AnimalManager {
    fn default() -> Self {
        Self::new(1000) // Default max 1000 animals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fauna_registry() {
        let registry = FaunaRegistry::new();

        assert!(registry.get("rabbit").is_some());
        assert!(registry.get("bear").is_some());
        assert!(registry.get("sheep").is_some());
    }

    #[test]
    fn test_biome_filtering() {
        let registry = FaunaRegistry::new();

        let arctic_animals = registry.get_by_biome(ClimateZone::Arctic);
        assert!(!arctic_animals.is_empty());

        // Mammoth should be in arctic
        assert!(arctic_animals.iter().any(|a| a.id == "mammoth"));
    }

    #[test]
    fn test_behavior_filtering() {
        let registry = FaunaRegistry::new();

        let aggressive = registry.get_by_behavior(AnimalBehavior::Aggressive);
        assert!(!aggressive.is_empty());

        // Wolf and bear should be aggressive/territorial
        assert!(aggressive.iter().any(|a| a.id == "wolf"));
    }

    #[test]
    fn test_domestication() {
        let sheep = sheep();
        let bear = bear();

        assert!(sheep.can_domesticate);
        assert!(!bear.can_domesticate);
    }

    #[test]
    fn test_living_products() {
        let sheep = sheep();
        let rabbit = rabbit();

        assert!(!sheep.living_products.is_empty());
        assert!(rabbit.living_products.is_empty());

        // Sheep should produce wool
        assert!(sheep.living_products.iter().any(|p| p.material_id == "wool"));
    }

    #[test]
    fn test_animal_drops() {
        let bear = bear();

        // Bear should drop fur and hide
        assert!(bear.drops.iter().any(|d| d.material_id == "fur"));
        assert!(bear.drops.iter().any(|d| d.material_id == "thick_hide"));
    }

    #[test]
    fn test_size_categories() {
        let rabbit = rabbit();
        let deer = deer();
        let mammoth = mammoth();

        assert_eq!(rabbit.size, AnimalSize::Tiny);
        assert_eq!(deer.size, AnimalSize::Medium);
        assert_eq!(mammoth.size, AnimalSize::Huge);
    }

    // ========================================================================
    // LIFECYCLE TESTS
    // ========================================================================

    #[test]
    fn test_animal_aging() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        let initial_age = animal.age;
        animal.tick_age();
        assert_eq!(animal.age, initial_age + 1);
    }

    #[test]
    fn test_animal_death_from_old_age() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        // Set age beyond lifespan
        animal.age = species.lifespan.1 + 100;
        animal.max_lifespan = species.lifespan.1;

        assert!(animal.is_too_old());
    }

    #[test]
    fn test_animal_maturity() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        // Young animal
        animal.age = 10;
        animal.maturity_age = 500;
        assert!(!animal.can_breed());

        // Mature animal with cooldown 0
        animal.age = 600;
        animal.reproduction_cooldown = 0;
        assert!(animal.can_breed());
    }

    #[test]
    fn test_animal_hunger_system() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        let initial_hunger = animal.hunger;
        animal.tick_hunger();
        assert!(animal.hunger > initial_hunger);

        // Feed the animal
        let hunger_before_feed = animal.hunger;
        animal.feed(50.0);
        assert!(animal.hunger < hunger_before_feed);
    }

    #[test]
    fn test_animal_starvation() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        animal.hunger = animal.max_hunger + 10.0;
        animal.tick_hunger();

        assert!(animal.is_starving);
        assert!(animal.current_health < species.health);
    }

    #[test]
    fn test_pregnancy_and_birth() {
        let species = rabbit();
        let mut animal = Animal::new("rabbit".to_string(), (0, 0), &species);

        // Make animal pregnant
        animal.become_pregnant(100, 1000);
        assert!(animal.is_pregnant);
        assert_eq!(animal.pregnancy_timer, 100);

        // Advance pregnancy
        for _ in 0..100 {
            animal.tick_age(); // This decrements pregnancy timer
        }

        assert!(animal.ready_to_give_birth());

        // Give birth
        animal.give_birth();
        assert!(!animal.is_pregnant);
        assert!(animal.reproduction_cooldown > 0);
    }

    #[test]
    fn test_offspring_creation() {
        let species = rabbit();
        let parent_pos = (10, 20);
        let offspring = Animal::new_offspring(
            "rabbit".to_string(),
            parent_pos,
            &species,
            None,
        );

        assert_eq!(offspring.species_id, "rabbit");
        assert_eq!(offspring.age, 0);
        assert!(!offspring.can_breed()); // Too young
        assert!(offspring.is_alive());
    }

    #[test]
    fn test_predator_prey_species() {
        let wolf = wolf();
        let rabbit = rabbit();

        // Wolf should have prey species
        assert!(!wolf.prey_species.is_empty());
        assert!(wolf.prey_species.contains(&"rabbit".to_string()));

        // Rabbit should not have prey species (herbivore)
        assert!(rabbit.prey_species.is_empty());
    }

    #[test]
    fn test_animal_manager_tick_aging() {
        let mut grid = crate::world::Grid::new(8, 8);
        grid.generate_terrain();
        grid.settle_soil();
        let mut plants = crate::environment::PlantManager::new(16);

        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("rabbit".to_string(), (0, 0));

        let initial_age = manager.animals[0].age;
        manager.tick_in_world(
            &mut grid,
            &mut plants,
            10.0,
            GrazingWeather {
                precipitation: 40.0,
                now: 10,
                season: crate::environment::Season::Summer,
            },
        );

        // Animal should have aged
        assert!(manager.animals[0].age > initial_age);
    }

    // ========================================================================
    // WHAT A WILD ANIMAL DOES ABOUT PEOPLE
    //
    // Nothing, until ISSUES_FOUND #57. There was predator hunting in this
    // module and no other awareness of agents at all, so a deer stood where it
    // stood while a settlement walked up to it.
    // ========================================================================

    #[test]
    fn a_deer_does_not_stand_still_while_you_walk_up_to_it() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("deer".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[(8, 10)]);
        let now = manager.animals[0].position;

        assert_ne!(was, now, "it should have moved");
        assert!(
            (now.0 - 8).abs() > (was.0 - 8).abs(),
            "and moved away from the man, not towards him: {was:?} -> {now:?}"
        );
    }

    #[test]
    fn something_across_the_valley_has_not_noticed_you() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("deer".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[(10 + AnimalManager::NEAR_ENOUGH_TO_SPOOK_IT + 1, 10)]);

        assert_eq!(
            manager.animals[0].position, was,
            "a man a long way off is not a reason to move"
        );
    }

    #[test]
    fn a_wolf_does_not_get_out_of_your_way() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("wolf".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[(9, 10)]);

        assert_eq!(
            manager.animals[0].position, was,
            "a thing that means to do something about you holds its ground"
        );
    }

    #[test]
    fn an_empty_country_spooks_nothing() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("deer".to_string(), (10, 10));

        let was = manager.animals[0].position;
        manager.shy_away_from(&[]);

        assert_eq!(manager.animals[0].position, was);
    }

    #[test]
    fn test_animal_manager_population_summary() {
        let mut manager = AnimalManager::new(100);
        manager.spawn_animal("rabbit".to_string(), (0, 0));
        manager.spawn_animal("rabbit".to_string(), (1, 0));
        manager.spawn_animal("wolf".to_string(), (5, 5));

        let summary = manager.population_summary();
        assert_eq!(summary.get("rabbit"), Some(&2));
        assert_eq!(summary.get("wolf"), Some(&1));
    }

    #[test]
    fn test_terrain_to_climate_zone() {
        assert_eq!(terrain_to_climate_zone(TerrainType::Desert), ClimateZone::Desert);
        assert_eq!(terrain_to_climate_zone(TerrainType::Mountain), ClimateZone::Arctic);
        assert_eq!(terrain_to_climate_zone(TerrainType::Plains), ClimateZone::Temperate);
        assert_eq!(terrain_to_climate_zone(TerrainType::Forest), ClimateZone::Temperate);
    }

    #[test]
    fn test_animal_spawn_config_default() {
        let config = AnimalSpawnConfig::default();
        assert!(config.herds_per_10000_tiles > 0);
        assert!(config.spawn_predators);
        assert!(config.head_per_10000_tiles > 0);
    }
}
