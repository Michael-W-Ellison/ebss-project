// src/analytics/tests/bearing_tests.rs
//! Tests for the turning year in the hedgerows.
//!
//! Growth was seasonal from the beginning and what was *standing* was not, so
//! a berry bush that had grown all summer still had its berries on it in
//! February. A settlement that can pick fruit in the snow has no reason to
//! put anything by, no lean season to be lean in, and no use for a store — so
//! every one of the last three batches was building machinery against a
//! scarcity the world never produced.
//!
//! Bearing was then a *set of seasons*, which was written when a season was
//! twenty-four days long. At ninety it made a year of four uniform blocks:
//! three months of leaf, three more of leaf, three months of harvest, three
//! months of nothing. A window is written in the calendar's own early / deep
//! / late instead, so a thing comes on and goes over inside a season and a
//! season can hold the end of one food and the beginning of another.
//!
//! The year, as this world now keeps it: greens run the whole growing year
//! and are the thinnest food in it; roots open with them and run past them
//! into early winter; fruit comes on at midsummer; grain is a harvest of
//! weeks; and winter past its first fortnight gives nothing at all.

use crate::analytics::Simulation;
use crate::agents::{AgentConfig, Population};
use crate::environment::seasons::{
    first_day_of, last_day_of, PartOfSeason, Season, DAYS_PER_SEASON, DAYS_PER_YEAR,
    TICKS_PER_DAY,
};
use crate::world::{Bearing, ItemType, Position, ResourceNode, ResourceType, World, WorldConfig};

fn a_world() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    Simulation::new(world, population)
}

fn turn_the_year_to(simulation: &mut Simulation, wanted: Season) {
    for _ in 0..(TICKS_PER_DAY * 400) {
        if simulation.world.climate.current_season() == wanted {
            return;
        }
        simulation.world.tick();
    }
    panic!("the year never reached {wanted:?}");
}

// --------------------------------------------------------------------------
// What bears when
// --------------------------------------------------------------------------

/// The parts of a season are two weeks, eight weeks and two weeks, and they
/// tile it exactly. Everything below is written in them, so if this is wrong
/// every window is wrong.
#[test]
fn the_parts_of_a_season_tile_it() {
    assert_eq!(PartOfSeason::Early.first_day_of_season(), 0);
    assert_eq!(
        PartOfSeason::Deep.first_day_of_season(),
        PartOfSeason::Early.last_day_of_season() + 1
    );
    assert_eq!(
        PartOfSeason::Late.first_day_of_season(),
        PartOfSeason::Deep.last_day_of_season() + 1
    );
    assert_eq!(PartOfSeason::Late.last_day_of_season(), DAYS_PER_SEASON - 1);

    // Two weeks at each end and eight in the middle
    assert_eq!(PartOfSeason::Early.last_day_of_season() + 1, 15);
    assert_eq!(PartOfSeason::Late.first_day_of_season(), 75);

    // And every day of a season falls in the part that claims it
    for day in 0..DAYS_PER_SEASON {
        let part = PartOfSeason::from_day_of_year(day);
        assert!(
            day >= part.first_day_of_season() && day <= part.last_day_of_season(),
            "day {day} is {part:?} and is not inside {:?}..={:?}",
            part.first_day_of_season(),
            part.last_day_of_season()
        );
    }
}

/// Fruit comes on at midsummer, not in September.
///
/// Three months of high summer with nothing ripe on any bush was the plainest
/// thing wrong with the old table: `Food` bore in autumn and in no other
/// season, which at twenty-four days to a season was a short wait and at
/// ninety is a quarter of a life.
#[test]
fn fruit_comes_on_at_midsummer_and_runs_to_the_end_of_autumn() {
    let opens = first_day_of(Season::Summer, PartOfSeason::Deep);
    let closes = last_day_of(Season::Fall, PartOfSeason::Late);

    assert!(!ResourceType::Food.is_it_bearing(opens - 1), "not the day before");
    assert!(ResourceType::Food.is_it_bearing(opens), "the day it opens");
    assert!(ResourceType::Food.is_it_bearing(closes), "the day it closes");
    assert!(
        !ResourceType::Food.is_it_bearing((closes + 1) % DAYS_PER_YEAR),
        "and nothing the day after"
    );

    // Early summer is still too soon, and so is any of spring
    assert!(!ResourceType::Food.is_it_bearing(first_day_of(Season::Summer, PartOfSeason::Early)));
    assert!(!ResourceType::Food.is_it_bearing(first_day_of(Season::Spring, PartOfSeason::Deep)));
}

/// Greens run the whole growing year, because there is always leaf while
/// anything is growing — and go over with the frosts.
#[test]
fn greens_run_from_the_first_warmth_to_the_frosts() {
    assert!(ResourceType::Greens.is_it_bearing(first_day_of(Season::Spring, PartOfSeason::Early)));
    assert!(ResourceType::Greens.is_it_bearing(first_day_of(Season::Summer, PartOfSeason::Deep)));
    assert!(ResourceType::Greens.is_it_bearing(first_day_of(Season::Fall, PartOfSeason::Early)));

    // Autumn used to have no leaf at all: greens stopped dead on the last day
    // of summer, which is not what a hedge does
    assert!(ResourceType::Greens.is_it_bearing(first_day_of(Season::Fall, PartOfSeason::Deep)));

    // And they are gone by the last fortnight of autumn
    assert!(!ResourceType::Greens.is_it_bearing(first_day_of(Season::Fall, PartOfSeason::Late)));
    assert!(!ResourceType::Greens.is_it_bearing(first_day_of(Season::Winter, PartOfSeason::Early)));
}

/// Roots open with the greens and run past everything else into early winter.
///
/// Last year's root in the hungry gap, this year's swollen root in autumn,
/// and the winter dig out of hard ground — which is what a root is for, and
/// why it is the food that ends the year.
#[test]
fn roots_are_the_food_that_ends_the_year() {
    for opens in [
        first_day_of(Season::Spring, PartOfSeason::Early),
        first_day_of(Season::Summer, PartOfSeason::Late),
        first_day_of(Season::Fall, PartOfSeason::Late),
        first_day_of(Season::Winter, PartOfSeason::Early),
    ] {
        assert!(
            ResourceType::Roots.is_it_bearing(opens),
            "there should be roots to dig on day {opens}"
        );
    }

    // Roots outlast greens, which is the point of them
    let greens = ResourceType::Greens.bearing_window().how_many_days();
    let roots = ResourceType::Roots.bearing_window().how_many_days();
    assert!(roots > greens, "roots {roots} days against greens {greens}");

    // And they stop too: deep winter is bare
    assert!(!ResourceType::Roots.is_it_bearing(first_day_of(Season::Winter, PartOfSeason::Deep)));
}

/// A harvest is weeks, not a season.
#[test]
fn grain_is_a_harvest_of_weeks() {
    let grain = ResourceType::Grain.bearing_window();

    assert!(grain.covers(first_day_of(Season::Summer, PartOfSeason::Late)));
    assert!(grain.covers(first_day_of(Season::Fall, PartOfSeason::Deep)));
    assert!(!grain.covers(first_day_of(Season::Summer, PartOfSeason::Deep)));
    assert!(!grain.covers(first_day_of(Season::Fall, PartOfSeason::Late)));

    assert!(
        grain.how_many_days() < DAYS_PER_SEASON + DAYS_PER_SEASON / 2,
        "a harvest that runs half the year is not a harvest: {} days",
        grain.how_many_days()
    );
}

/// Every season has something on the land a body can live on, and winter has
/// only its first fortnight.
///
/// The old table gave spring and summer nothing but leaf and one root, autumn
/// everything at once, and winter nothing — and a settlement died in the
/// spring standing in a full meadow, because leaf is six energy against
/// ordinary forage's twenty-five.
#[test]
fn every_season_but_deep_winter_has_something_dense_on_it() {
    use crate::world::nutrition::FoodDatabase;

    let database = FoodDatabase::new();
    let dense_enough = |what: ResourceType, day: u32| {
        Simulation::edible_resources()
            .into_iter()
            .filter(|(kind, _)| *kind == what)
            .any(|(_, item)| {
                database
                    .get(&item)
                    .is_some_and(|food| food.base_nutrition.energy >= 20.0)
            })
            && what.is_it_bearing(day)
    };

    let grown = [
        ResourceType::Food,
        ResourceType::Grain,
        ResourceType::Greens,
        ResourceType::Roots,
    ];

    for season in [Season::Spring, Season::Summer, Season::Fall] {
        for part in [PartOfSeason::Early, PartOfSeason::Deep, PartOfSeason::Late] {
            let day = first_day_of(season, part);
            assert!(
                grown.iter().any(|what| dense_enough(*what, day)),
                "{part:?} {season:?} (day {day}) has nothing on the land worth eating"
            );
        }
    }
}

/// And winter, past its first fortnight, gives nothing whatever.
#[test]
fn nothing_edible_bears_in_deep_winter() {
    let midwinter = first_day_of(Season::Winter, PartOfSeason::Deep);

    for what in [
        ResourceType::Food,
        ResourceType::Grain,
        ResourceType::Greens,
        ResourceType::Roots,
        ResourceType::Honey,
    ] {
        assert!(
            !what.is_it_bearing(midwinter),
            "{what:?} should give nothing in deep winter — that is the whole point of a store"
        );
    }
}

/// A river does not stop being a river in February, and nor does a rock stop
/// being a rock. Only growing things have a season.
#[test]
fn what_does_not_grow_does_not_stop() {
    for what in [
        ResourceType::Water,
        ResourceType::Stone,
        ResourceType::Wood,
        ResourceType::Clay,
    ] {
        assert_eq!(what.bearing_window(), Bearing::NeverStops);
        for day in 0..DAYS_PER_YEAR {
            assert!(
                what.is_it_bearing(day),
                "{what:?} is not bearing, so it cannot stop"
            );
        }
    }
}

/// A window that runs round the turn of the year is legal, and counts the
/// days it actually covers.
#[test]
fn a_window_can_run_round_the_turn_of_the_year() {
    let over_the_turn = Bearing::Between { opens: DAYS_PER_YEAR - 5, closes: 4 };

    assert!(over_the_turn.covers(DAYS_PER_YEAR - 1));
    assert!(over_the_turn.covers(0));
    assert!(over_the_turn.covers(4));
    assert!(!over_the_turn.covers(5));
    assert!(!over_the_turn.covers(DAYS_PER_YEAR - 6));
    assert_eq!(over_the_turn.how_many_days(), 10);
}

// --------------------------------------------------------------------------
// And it actually tells on the world
// --------------------------------------------------------------------------

/// A world opens on the day it opens on, and carries what that day carries.
///
/// It used to be made with every bush in full fruit whatever the date. The
/// year opens in spring, so every settlement ever run began with berries,
/// standing grain and full hives on the hedges around it - 216 units of
/// fruit, 254 of grain and 34 of honey over sixteen worlds, about a day and a
/// third of food for twelve people - which then fell off over the following
/// ten days as the shedding rule caught up with it.
///
/// It mattered out of all proportion to its size, because those ten days are
/// exactly the ones in which the founders eat down the reserve they arrive
/// with. Removing it costs about five per cent of the person-days a settlement
/// lives, against the half a per cent its share of a year would suggest.
#[test]
fn a_world_opens_with_nothing_on_it_that_is_out_of_season() {
    let simulation = a_world();

    let opening_day = simulation.world.climate.calendar.day_of_year;
    let standing = |what: ResourceType| -> u32 {
        simulation
            .world
            .resources
            .iter()
            .filter(|resource| resource.resource_type == what)
            .map(|resource| resource.amount)
            .sum()
    };

    for what in [
        ResourceType::Food,
        ResourceType::Grain,
        ResourceType::Greens,
        ResourceType::Roots,
        ResourceType::Honey,
    ] {
        if what.is_it_bearing(opening_day) {
            assert!(
                standing(what) > 0,
                "{what:?} bears on day {opening_day} and the world should hold some"
            );
        } else {
            assert_eq!(
                standing(what),
                0,
                "{what:?} does not bear on day {opening_day}, so none of it should be standing"
            );
        }
    }

    // And the year does open in spring, so this test is about a spring world:
    // fruit, grain and honey are all out of season and greens and roots are
    // not. If the opening day ever moves, the loop above still holds and this
    // line is what will say so.
    assert_eq!(Season::from_day_of_year(opening_day), Season::Spring);
    assert_eq!(standing(ResourceType::Food), 0);
    assert!(standing(ResourceType::Greens) > 0);
}

/// A tree still has wood in it in February, and the world is stocked with it
/// whatever the date.
#[test]
fn what_never_bears_is_stocked_whatever_the_date() {
    let simulation = a_world();

    for what in [ResourceType::Wood, ResourceType::Stone, ResourceType::Water] {
        assert!(
            simulation
                .world
                .resources
                .iter()
                .any(|resource| resource.resource_type == what && resource.amount > 0),
            "{what:?} does not bear, so it cannot be out of season"
        );
    }
}

/// A bush carrying fruit sheds it once its season goes by.
#[test]
fn what_a_bush_carries_falls_off_out_of_season() {
    let mut simulation = a_world();
    simulation.world.resources.clear();

    let where_it_is = Position::new(25, 25);
    let mut bush = ResourceNode::new(ResourceType::Food, where_it_is, 60);
    bush.amount = 60;
    simulation.world.resources.push(bush);

    turn_the_year_to(&mut simulation, Season::Winter);

    // A good few weeks of winter
    for _ in 0..(TICKS_PER_DAY * 20) {
        simulation.world.tick();
    }

    let still_on_it = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == where_it_is)
        .map(|resource| resource.amount)
        .unwrap_or(0);

    assert_eq!(
        still_on_it, 0,
        "there should be nothing on that bush by midwinter"
    );
}

/// And it does not shed in the season it bears in.
#[test]
fn a_bush_keeps_what_it_carries_in_its_own_season() {
    let mut simulation = a_world();
    simulation.world.resources.clear();

    let where_it_is = Position::new(25, 25);
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        where_it_is,
        60,
    ));

    // Reach autumn first and *then* put the fruit on it - winding the year
    // forward runs it through three seasons in which it bears nothing
    turn_the_year_to(&mut simulation, Season::Fall);
    if let Some(bush) = simulation
        .world
        .resources
        .iter_mut()
        .find(|resource| resource.position == where_it_is)
    {
        bush.amount = 40;
    }

    for _ in 0..(TICKS_PER_DAY * 5) {
        simulation.world.tick();
    }

    let still_on_it = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == where_it_is)
        .map(|resource| resource.amount)
        .unwrap_or(0);

    assert!(
        still_on_it >= 40,
        "autumn is when it bears, and this one lost {} of it",
        40u32.saturating_sub(still_on_it)
    );
}

/// Shedding always takes at least one, so a patch actually empties rather
/// than creeping down by fractions for ever.
#[test]
fn a_nearly_bare_patch_finishes_emptying() {
    let mut bush = ResourceNode::new(ResourceType::Food, Position::new(1, 1), 60);
    bush.amount = 1;

    let mut ground = crate::world::soil::Soil::for_terrain(
        crate::world::TerrainType::Plains,
    );

    bush.what_it_carries_falls_off(0.0001, &mut ground);

    assert_eq!(bush.amount, 0);
}

/// And what falls off goes into the ground it fell on.
///
/// A crop nobody picked used to be deleted, so a growing tile was mined out
/// by its own plants with nobody near it. See ISSUES_FOUND.md #127.
#[test]
fn what_nobody_picks_goes_back_into_the_ground() {
    let mut bush = ResourceNode::new(ResourceType::Food, Position::new(1, 1), 60);
    bush.amount = 40;

    let mut ground = crate::world::soil::Soil::for_terrain(
        crate::world::TerrainType::Plains,
    );
    ground.leaf_litter = 0.0;

    bush.what_it_carries_falls_off(0.5, &mut ground);

    assert!(bush.amount < 40, "some of it came off the bush");
    assert!(
        ground.leaf_litter > 0.0,
        "and it is lying on the ground under it, not gone out of the world"
    );
}

/// A patch nobody ever touches breaks even on the ground it grows in.
///
/// This is the whole of what makes the map self-sufficient: growing a unit
/// takes nutrient out of the tile and puts half of it straight back as root
/// and stalk, and the other half comes back when the uneaten crop falls. Over
/// a year of growing and shedding with nobody near it, the ground it grew on
/// should be no poorer than it started.
///
/// Measured from the fifth year rather than from the first, which is a change
/// to when the question is asked and not to the question. A world now opens
/// with less standing growth on it than it settles at, and the difference is
/// laid down over the first few years - so a tile does genuinely and rightly
/// lose ground in year one, into the plants standing on it. What has to break
/// even is the steady state, and by year five it is one.
#[test]
fn ground_nobody_harvests_is_no_poorer_a_year_later() {
    use crate::environment::seasons::TICKS_PER_DAY;

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let a_year = TICKS_PER_DAY * 360;

    for _ in 0..(a_year * 5) {
        world.tick();
    }
    let before = mean_fertility_under(&world, ResourceType::Greens);

    for _ in 0..a_year {
        world.tick();
    }
    let after = mean_fertility_under(&world, ResourceType::Greens);

    assert!(
        after >= before * 0.98,
        "a meadow nobody walked on lost its fertility: {before:.3} then {after:.3}"
    );
}

/// The mean fertility of every tile carrying this kind of growing thing.
fn mean_fertility_under(world: &World, kind: ResourceType) -> f32 {
    let mut total = 0.0;
    let mut n = 0.0;
    for r in world.resources.iter().filter(|r| r.resource_type == kind) {
        if let Some(tile) = world.grid.get_tile(&r.position) {
            total += tile.soil.fertility();
            n += 1.0;
        }
    }
    if n == 0.0 { 0.0 } else { total / n }
}

// --------------------------------------------------------------------------
// What a person can do about it
// --------------------------------------------------------------------------

/// Greens and roots are things a person can eat, and the foraging pass knows
/// it. Without this a settlement would starve in spring standing in a meadow.
#[test]
fn greens_and_roots_are_food() {
    assert!(ResourceType::Greens.is_edible());
    assert!(ResourceType::Roots.is_edible());

    let known: Vec<ItemType> = Simulation::edible_resources()
        .into_iter()
        .map(|(_, item)| item)
        .collect();

    assert!(known.contains(&ItemType::Greens));
    assert!(known.contains(&ItemType::Roots));
}

/// Greens are thin stuff: a great deal of what a body needs a little of, and
/// almost no energy at all. That is what makes spring hungry even when the
/// meadows are full.
#[test]
fn greens_are_thin_stuff() {
    use crate::world::nutrition::FoodDatabase;

    let database = FoodDatabase::new();
    let greens = database.get(&ItemType::Greens).expect("in the database");
    let berries = database.get(&ItemType::Food).expect("in the database");

    assert!(
        greens.base_nutrition.energy < berries.base_nutrition.energy,
        "there is nothing to live on in a leaf"
    );
    assert!(
        greens.base_nutrition.micronutrients > berries.base_nutrition.micronutrients,
        "and everything else in it"
    );
}

/// A world has greens and roots standing in it, or spring is a death
/// sentence.
#[test]
fn a_world_has_something_for_spring_and_summer() {
    let simulation = a_world();

    for what in [ResourceType::Greens, ResourceType::Roots] {
        assert!(
            simulation
                .world
                .resources
                .iter()
                .any(|resource| resource.resource_type == what),
            "{what:?} should be growing somewhere"
        );
    }
}
