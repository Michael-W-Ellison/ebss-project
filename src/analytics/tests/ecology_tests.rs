// src/analytics/tests/ecology_tests.rs
//! The map has to stand up with nobody on it.
//!
//! A world with no people in it should still be there in thirty years: the
//! hedgerows bearing, the ground no poorer, and the same animals in it. It was
//! not. Run empty, a world lost its greens at five per cent a year for ever
//! and was empty of animals inside twenty, with seventeen of twenty species
//! extinct in every world.
//!
//! Two defects, both of the kind this document keeps naming - a number that
//! meant two different things, and a thing that left the world without going
//! anywhere.
//!
//! - What fell off a plant nobody picked was **deleted** rather than dropped,
//!   so every growing tile was mined out by its own crop with nobody near it.
//!   See `bearing_tests` for that half.
//! - `animals.len()` was read as "how many animals this world holds" by every
//!   one of the seven places that asks whether there is room for another, and
//!   nothing ever took a dead animal out of the list. Twenty years in: **898
//!   records of which 9.8 were alive**. The corpses held every slot, so
//!   nothing could be born and nothing could migrate in.
//!
//! See ISSUES_FOUND.md #127.

use crate::agents::Population;
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::world::{World, WorldConfig};
use std::collections::BTreeSet;

/// A world with nobody in it.
fn an_empty_world() -> Simulation {
    let world = World::new(WorldConfig::default());
    Simulation::new(world, Population::new())
}

fn how_many_years(simulation: &mut Simulation, years: u32) {
    for _ in 0..(years * 360 * TICKS_PER_DAY) {
        simulation.tick();
    }
}

fn what_lives_here(simulation: &Simulation) -> BTreeSet<String> {
    simulation
        .world
        .animals
        .get_all()
        .iter()
        .filter(|animal| animal.is_alive())
        .map(|animal| animal.species_id.clone())
        .collect()
}

// --------------------------------------------------------------------------
// The dead are not the living
// --------------------------------------------------------------------------

/// A dead animal is not one of the animals this world holds.
#[test]
fn a_corpse_is_not_counted_among_the_living() {
    let mut world = World::new(WorldConfig::default());

    let before = world.animals.how_many_are_alive();
    assert!(before > 0, "a fresh world has animals in it");

    // Kill one where it stands
    if let Some(animal) = world.animals.get_all_mut().iter_mut().find(|a| a.is_alive()) {
        animal.current_health = 0.0;
    }

    assert_eq!(
        world.animals.how_many_are_alive(),
        before - 1,
        "the tally of the living went down by one"
    );
}

/// And it is taken off the map, rather than sitting in the list for ever.
///
/// Nothing reads a body after the tick it falls in - a predator feeds off it
/// there and then, a hunter butchers it there and then - so what is left is
/// only a slot nobody can use.
#[test]
fn the_dead_are_taken_off_the_map() {
    let mut world = World::new(WorldConfig::default());

    for animal in world.animals.get_all_mut().iter_mut() {
        animal.current_health = 0.0;
    }

    world.tick();

    assert_eq!(
        world.animals.get_all().len(),
        world.animals.how_many_are_alive(),
        "every record left is an animal that is alive"
    );
}

/// Which is what keeps the corpses from filling the world.
///
/// The number, measured: twenty years into an empty world, 898 animal records
/// of which 9.8 were alive. Every gate that asks whether there is room for
/// another animal was counting the dead.
#[test]
fn corpses_do_not_fill_up_the_world() {
    let mut simulation = an_empty_world();
    how_many_years(&mut simulation, 3);

    let records = simulation.world.animals.get_all().len();
    let alive = simulation.world.animals.how_many_are_alive();

    assert_eq!(
        records, alive,
        "three years in, {records} records and {alive} of them alive"
    );
}

// --------------------------------------------------------------------------
// A world nobody is in
// --------------------------------------------------------------------------

/// An empty world still has animals in it years later.
///
/// It did not. Twenty years of nobody at all left a mean of 9.8 living
/// animals in a world that started with 35, and seventeen of twenty species
/// gone from every world.
#[test]
fn a_world_with_nobody_in_it_does_not_empty_of_animals() {
    let mut simulation = an_empty_world();

    let at_the_start = what_lives_here(&simulation);
    assert!(!at_the_start.is_empty(), "a fresh world has animals in it");

    how_many_years(&mut simulation, 5);

    let alive = simulation.world.animals.how_many_are_alive();
    assert!(
        alive >= at_the_start.len(),
        "five years with nobody in it and only {alive} animals left"
    );
}

/// And most of what lived there still lives there.
///
/// Not all of it: a solitary predator in a world that only ever held one or
/// two of them can genuinely die out, and immigration is deliberately slow.
/// What must not happen is the wholesale emptying that was measured.
#[test]
fn most_of_what_lived_here_still_lives_here() {
    let mut simulation = an_empty_world();

    let at_the_start = what_lives_here(&simulation);

    how_many_years(&mut simulation, 5);

    let now = what_lives_here(&simulation);
    let held: usize = at_the_start.intersection(&now).count();

    assert!(
        held * 2 >= at_the_start.len(),
        "of {} species this world started with, {held} are still in it: {:?}",
        at_the_start.len(),
        at_the_start.difference(&now).collect::<Vec<_>>()
    );
}

/// The hedgerows are still bearing, too.
///
/// The other half of the same question: standing crop held its level rather
/// than falling away. Measured before the fix, greens went from 3,516 units
/// to 2,260 over nine years with nobody picking any.
#[test]
fn the_hedgerows_are_no_thinner_a_few_years_on() {
    use crate::world::ResourceType;

    let mut simulation = an_empty_world();

    let standing = |simulation: &Simulation| -> u32 {
        simulation
            .world
            .resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Greens)
            .map(|r| r.amount)
            .sum()
    };

    // A year in, so the first spring has run and the world has settled off
    // whatever it was seeded with.
    how_many_years(&mut simulation, 1);
    let after_a_year = standing(&simulation);

    how_many_years(&mut simulation, 4);
    let after_five = standing(&simulation);

    assert!(
        after_five as f32 >= after_a_year as f32 * 0.8,
        "the greens thinned out with nobody eating them: {after_a_year} then {after_five}"
    );
}

// --------------------------------------------------------------------------
// What is gone can come back
// --------------------------------------------------------------------------

/// A species that has gone from this world finds its way back to it.
///
/// Deliberately slow - one small group per depleted species every two
/// thousand ticks or so, and only a one-in-four chance at each of those - so
/// this gives it years rather than months.
///
/// Two things had to be true for it to work at all, and neither was. The
/// migration pass broke out of its loop the moment the map was at its cap,
/// which it always was once the corpses had filled it; and a species was only
/// remembered as having lived here if it happened to be alive at a migration
/// moment, so anything that died inside its first two thousand ticks was
/// forgotten and could never come back. See ISSUES_FOUND.md #127.
#[test]
fn something_that_is_gone_finds_its_way_back() {
    crate::core::dice::seed(4);

    let mut world = World::new(WorldConfig::default());

    // Long enough for the world to have seen what lives in it. Nothing comes
    // back that this country never held, and a country holds what it has
    // actually carried - see `process_immigration`.
    for _ in 0..TICKS_PER_DAY {
        world.tick();
    }

    let gone = world
        .animals
        .get_all()
        .iter()
        .find(|a| a.is_alive())
        .map(|a| a.species_id.clone())
        .expect("a fresh world has animals in it");

    // Take every one of them off the map
    for animal in world.animals.get_all_mut().iter_mut() {
        if animal.species_id == gone {
            animal.current_health = 0.0;
        }
    }
    world.tick();

    assert!(
        !what_lives_in(&world).contains(&gone),
        "{gone} is gone from this world"
    );

    for _ in 0..(TICKS_PER_DAY * 360 * 10) {
        world.tick();
        if what_lives_in(&world).contains(&gone) {
            return;
        }
    }

    panic!("ten years and no {gone} ever found its way back");
}

fn what_lives_in(world: &World) -> BTreeSet<String> {
    world
        .animals
        .get_all()
        .iter()
        .filter(|animal| animal.is_alive())
        .map(|animal| animal.species_id.clone())
        .collect()
}

// --- the ground worth visiting ----------------------------------------------

/// Every piece of foul ground is on the list of ground worth visiting.
///
/// The register in `Grid` is a second representation of something the map
/// already says, and two representations of one fact drift apart. This is the
/// test that says they have not. It walks the whole map and asks the register
/// about every tile it finds muck on: a tile fouled behind the register's back
/// would be a midden that never smells, never comes up in food, and never
/// breaks down, and none of that would show as a crash.
#[test]
fn the_ground_register_and_the_map_agree() {
    use crate::world::Position;

    crate::core::dice::seed(11);
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(crate::agents::AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    // Long enough for people to have voided on the ground and for some of
    // them to have died on it.
    for _ in 0..TICKS_PER_DAY * 30 {
        simulation.tick();
    }

    let noted: BTreeSet<(i32, i32)> = simulation
        .world
        .grid
        .where_the_ground_is_doing_something()
        .into_iter()
        .map(|at| (at.x, at.y))
        .collect();

    let mut missed = Vec::new();
    for y in 0..simulation.world.grid.height {
        for x in 0..simulation.world.grid.width {
            let at = Position::new(x as i32, y as i32);
            let Some(tile) = simulation.world.grid.get_tile(&at) else {
                continue;
            };
            if tile.soil.has_somebody_left_something_here() && !noted.contains(&(at.x, at.y)) {
                missed.push((at.x, at.y, tile.soil.fouling, tile.soil.seeds_dropped));
            }
        }
    }

    assert!(
        missed.is_empty(),
        "ground with muck on it that nothing will ever visit: {missed:?}"
    );

    // And the other way about, so that the register is not simply the whole
    // map: a world of fifty by fifty has two and a half thousand tiles in it
    // and a dozen people do not foul all of them.
    assert!(
        noted.len() < simulation.world.grid.width * simulation.world.grid.height / 2,
        "the register holds {} of {} tiles, which is not a register",
        noted.len(),
        simulation.world.grid.width * simulation.world.grid.height
    );
}

// --- what the grazers take, and what they give back --------------------------

/// The sky, on the pass'th grazing pass. Ten ticks apart, which is the
/// cadence a live world grazes on.
fn grazing_weather(pass: u32) -> crate::environment::GrazingWeather {
    crate::environment::GrazingWeather {
        precipitation: 40.0,
        now: pass * 10,
        season: crate::environment::Season::Summer,
    }
}

/// A world with one plant and one animal standing on it.
fn a_beast_on_a_plant(
    beast: &str,
    plant: &str,
) -> (
    crate::world::Grid,
    crate::environment::PlantManager,
    crate::environment::AnimalManager,
) {
    use crate::environment::{AnimalManager, AnimalState, PlantManager};
    use crate::world::{Grid, Terrain, TerrainType};

    let mut grid = Grid::new(16, 16);
    for row in grid.tiles.iter_mut() {
        for tile in row.iter_mut() {
            tile.terrain = Terrain::new(TerrainType::Meadow);
        }
    }
    grid.settle_soil();

    let mut plants = PlantManager::new(64);
    plants.spawn_plant(plant.to_string(), (8, 8), 0);

    let mut animals = AnimalManager::new(16);
    animals.spawn_animal(beast.to_string(), (8, 8));
    for animal in animals.get_all_mut() {
        animal.state = AnimalState::Grazing;
    }

    (grid, plants, animals)
}

/// Grazing takes something off the map.
///
/// It took nothing at all before: `process_grazing` fed an animal out of thin
/// air, and the comment above the breeding pass has said so in as many words
/// since it was written. What stopped a herd growing was a hard number in a
/// field rather than the grass running out.
#[test]
fn a_grazing_animal_takes_the_plant_down_with_it() {
    let (mut grid, mut plants, mut animals) = a_beast_on_a_plant("deer", "grass");

    let before = plants.all_plants()[0].current_health;
    assert!(before > 0.0, "the fixture has no plant standing in it");

    for pass in 0..20u32 {
        animals.tick_in_world(&mut grid, &mut plants, 10.0, grazing_weather(pass));
    }

    let after = plants
        .all_plants()
        .first()
        .map(|plant| plant.current_health)
        .unwrap_or(0.0);

    assert!(
        after < before,
        "a deer stood on a patch of grass for two hundred ticks and the grass \
         is no smaller: {before:.2} to {after:.2}"
    );
}

/// And what it does not use lands on the ground behind it.
///
/// Most of a mouthful goes straight through. That is what a grazing animal
/// does for the ground it walks on, and until now nothing in the model did it:
/// what an animal ate came from nowhere and went nowhere.
#[test]
fn what_an_animal_passes_goes_back_into_the_ground() {
    use crate::world::Position;

    let (mut grid, mut plants, mut animals) = a_beast_on_a_plant("deer", "grass");

    let underfoot = Position::new(8, 8);
    let before = grid
        .get_tile(&underfoot)
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);

    for pass in 0..20u32 {
        animals.tick_in_world(&mut grid, &mut plants, 10.0, grazing_weather(pass));
    }

    let after = grid
        .get_tile(&underfoot)
        .map(|tile| tile.soil.litter())
        .unwrap_or(0.0);

    assert!(
        after > before,
        "the ground under a feeding deer is no richer: {before:.4} to {after:.4}"
    );
}

/// A bear digs a plant up; a deer crops it and it comes back.
///
/// The difference is in how the animal feeds rather than in a hand-written
/// list of which plants count as roots.
#[test]
fn what_is_dug_up_does_not_come_back() {
    let (mut grid, mut plants, mut bears) = a_beast_on_a_plant("bear", "potato");
    let (mut deer_ground, mut deer_plants, mut deer) = a_beast_on_a_plant("deer", "potato");

    for pass in 0..3u32 {
        bears.tick_in_world(&mut grid, &mut plants, 10.0, grazing_weather(pass));
        deer.tick_in_world(
            &mut deer_ground,
            &mut deer_plants,
            10.0,
            grazing_weather(pass),
        );
    }

    let dug = plants.all_plants()[0].current_health;
    let cropped = deer_plants.all_plants()[0].current_health;

    assert!(
        dug <= 0.0,
        "a bear fed on this and left {dug:.2} of it standing"
    );
    assert!(
        cropped > 0.0,
        "a deer took the whole plant rather than cropping it: {cropped:.2}"
    );
}

/// The size of a herd comes from the land, not from a number in a field.
///
/// A hundred and twenty by a hundred and twenty carries a ceiling of 5,760
/// animals. With grazing feeding every animal out of nothing the herds went
/// to that ceiling inside three years and sat on it - 5,750 head on a hundred
/// and forty-four hectares, mean hunger 0.30, which is to say the grass was
/// infinite. What they settle at now is what the ground grows.
#[test]
fn a_herd_settles_at_what_the_ground_will_feed() {
    use crate::world::WorldConfig;

    crate::core::dice::seed(23);
    let mut world = World::new(WorldConfig::default().with_size(120, 120));

    let at_the_very_outside = 5760;
    let started_with = world.animals.how_many_are_alive();

    // Five years is well past where the old model was pinned to its ceiling.
    for _ in 0..(5 * crate::environment::seasons::TICKS_PER_YEAR) {
        world.tick();
    }

    let alive = world.animals.how_many_are_alive();

    assert!(
        alive > 0,
        "the country carried {started_with} head and now carries none"
    );
    assert!(
        alive < at_the_very_outside / 4,
        "{alive} head on a hundred and forty-four hectares, against a ceiling \
         of {at_the_very_outside}: the herd is still bounded by the array and \
         not by the grass"
    );
}
