// src/analytics/tests/soil_ladder_tests.rs
//! Tests for the soil ladder: what a field is, what wears it and what builds
//! it, one rule to a test, in the words it was asked for in.
//!
//! "Wild plants produce yields 1/4th that of plants in tilled farmland.
//! Harvesting from a wild plant should not change the grade of the soil.
//! Harvests from tilled farmland are much more abundant but their harvest
//! reduces the future production capability of the farmland." See
//! `world::soil` and ISSUES_FOUND #246.

use crate::world::soil::{Field, SoilGrade, SoilType, WhatBecameOfIt, A_SEASON, A_YEAR};
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

// --- the ladder -------------------------------------------------------------

/// The rungs and what each is worth, as they were asked for.
#[test]
fn the_rungs_are_the_ones_asked_for() {
    let asked_for = [
        (SoilGrade::VeryRich, 2.0),
        (SoilGrade::Rich, 1.5),
        (SoilGrade::Ordinary, 1.0),
        (SoilGrade::Depleted, 0.5),
        (SoilGrade::Exhausted, 0.25),
    ];
    for (grade, multiplier) in asked_for {
        assert_eq!(grade.multiplier(), multiplier, "{}", grade.called());
    }
    assert_eq!(SoilGrade::the_top(), SoilGrade::VeryRich, "the top is very rich, for now");
}

/// The ladder is one list, poorest first, and everything that climbs it
/// climbs that list - so a rung added to it is a rung everything can reach.
#[test]
fn the_ladder_is_climbed_one_rung_at_a_time_and_goes_no_further() {
    for pair in SoilGrade::LADDER.windows(2) {
        assert!(pair[0].multiplier() < pair[1].multiplier(), "poorest first");
        assert_eq!(pair[0].richer(), pair[1]);
        assert_eq!(pair[1].poorer(), pair[0]);
    }
    let bottom = SoilGrade::LADDER[0];
    assert_eq!(bottom.poorer(), bottom, "nothing below the bottom");
    assert_eq!(SoilGrade::the_top().richer(), SoilGrade::the_top(), "nothing above the top");
}

/// Wild ground's kind and grade follow from its terrain, and a map stores
/// nothing for them.
#[test]
fn wild_ground_is_what_its_terrain_made_it_and_costs_nothing() {
    let world = World::new(WorldConfig::default());

    for y in 0..world.grid.height {
        for x in 0..world.grid.width {
            let at = Position::new(x as i32, y as i32);
            let terrain = world.grid.get_tile(&at).unwrap().terrain.terrain_type;
            assert_eq!(world.grid.soil_at(&at), Some(SoilType::natural_to(terrain)));
        }
    }
    assert_eq!(world.grid.every_field().count(), 0, "a new world has no fields in it");
}

// --- what a field yields ----------------------------------------------------

/// A field yields four times what the same ground does wild.
#[test]
fn broken_ground_yields_four_times_wild() {
    let mut world = World::new(WorldConfig::default());
    let here = Position::new(10, 10);
    world.grid.get_tile_mut(&here).unwrap().terrain =
        crate::world::Terrain::new(TerrainType::Plains);

    let wild = world.grid.what_it_yields_here(&here);
    assert!(world.grid.break_ground(&here, 0));
    let broken = world.grid.what_it_yields_here(&here);

    assert_eq!(broken, wild * 4.0, "{broken} against {wild}");
}

// --- what wears it ----------------------------------------------------------

/// A field goes down a rung once a whole crop's worth has come off it, and not
/// once a trip.
#[test]
fn a_whole_crop_wears_a_field_a_rung_and_a_trip_does_not() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    let a_whole_crop = 100;

    for trip in 0..9 {
        field.a_crop_came_off(11, a_whole_crop, false, trip);
    }
    assert_eq!(field.grade, SoilGrade::Ordinary, "ninety-nine units is not a crop");

    field.a_crop_came_off(11, a_whole_crop, false, 10);
    assert_eq!(field.grade, SoilGrade::Depleted, "a hundred and ten is");
    assert_eq!(field.taken_since_it_last_fell, 10, "and what is over counts towards the next");
}

/// And down to exhausted, which is where it stops.
#[test]
fn a_field_cropped_without_end_is_exhausted_and_no_worse() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    for crop in 0..10 {
        field.a_crop_came_off(100, 100, false, crop);
    }
    assert_eq!(field.grade, SoilGrade::Exhausted);
}

/// Harvesting from a wild plant does not change the grade of the soil.
#[test]
fn a_wild_harvest_leaves_the_ground_as_it_was() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    let here = Position::new(12, 12);
    world.grid.get_tile_mut(&here).unwrap().terrain =
        crate::world::Terrain::new(TerrainType::Meadow);

    let mut bush = ResourceNode::new(ResourceType::Food, here, 60);
    bush.amount = 60;
    world.resources.push(bush);
    let before = world.grid.soil_at(&here);

    for _ in 0..20 {
        let all = world.resources[0].amount;
        world.resources[0].harvest(all);
        for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
            world.take_a_turn();
        }
    }

    assert_eq!(world.grid.soil_at(&here), before, "stripped twenty days running");
    assert!(world.grid.field_at(&here).is_none());
}

/// What comes off a field by hand is counted against it however it came off -
/// and what was handed straight back is not.
#[test]
fn what_comes_off_a_field_is_the_net_of_what_was_taken() {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    let here = Position::new(12, 12);
    world.grid.get_tile_mut(&here).unwrap().terrain =
        crate::world::Terrain::new(TerrainType::Plains);
    assert!(world.grid.break_ground(&here, 0));

    let mut crop = ResourceNode::new(ResourceType::Grain, here, 80);
    let full = crop.how_heavy_a_crop_it_carries(world.grid.what_it_yields_here(&here));
    crop.amount = full;
    world.resources.push(crop);

    // Taken and handed straight back, the way a full pack does it
    world.resources[0].harvest(full);
    world.resources[0].put_it_back(full);
    for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
        world.take_a_turn();
    }
    assert_eq!(
        world.grid.field_at(&here).unwrap().grade,
        SoilGrade::Ordinary,
        "nothing was kept, so nothing is counted"
    );

    // And the whole of it kept. Stood up again first: the day that went by
    // did what days do to a crop out of its season.
    world.resources[0].amount = full;
    assert_eq!(world.resources[0].harvest(full), full);
    for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
        world.take_a_turn();
    }
    assert_eq!(world.grid.field_at(&here).unwrap().grade, SoilGrade::Depleted);
}

// --- what builds it ---------------------------------------------------------

/// Manure needs a season after it goes on before it raises the grade.
#[test]
fn muck_wants_a_season_before_it_tells() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    assert!(field.somebody_mucked_it(Field::ENOUGH_MUCK_FOR_A_RUNG, 100));

    assert_eq!(field.a_day_goes_by(100 + A_SEASON - 1), WhatBecameOfIt::StillAField);
    assert_eq!(field.grade, SoilGrade::Ordinary, "a day short of the season");

    field.a_day_goes_by(100 + A_SEASON);
    assert_eq!(field.grade, SoilGrade::Rich, "and on the day");
}

/// A little muck does little; tippings add up to a rung.
#[test]
fn small_tippings_add_up_to_a_rung() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    let a_third = Field::ENOUGH_MUCK_FOR_A_RUNG / 3.0 + 0.001;

    field.somebody_mucked_it(a_third, 0);
    field.somebody_mucked_it(a_third, 1);
    assert!(field.mucked_ready_at.is_none(), "two thirds of a rung is not a rung");

    field.somebody_mucked_it(a_third, 2);
    assert_eq!(field.mucked_ready_at, Some(2 + A_SEASON), "three thirds is, from the third");
}

/// Muck and beans can take a field past what it was, up to the top.
#[test]
fn muck_and_beans_build_past_the_natural_grade() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    assert_eq!(field.natural, SoilGrade::Ordinary);

    field.a_bean_crop_came_in(0);
    field.somebody_mucked_it(Field::ENOUGH_MUCK_FOR_A_RUNG, 0);
    field.a_day_goes_by(A_SEASON);

    assert_eq!(field.grade, SoilGrade::VeryRich, "ordinary, a bean crop, and a season's muck");
}

// --- rest -------------------------------------------------------------------

/// A year's rest brings a worn field one rung back towards what it was.
#[test]
fn a_year_fallow_brings_a_worn_field_a_rung_back() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    field.a_crop_came_off(100, 100, false, 0);
    field.a_crop_came_off(100, 100, false, 0);
    assert_eq!(field.grade, SoilGrade::Exhausted);

    field.a_day_goes_by(A_YEAR - 1);
    assert_eq!(field.grade, SoilGrade::Exhausted, "not before the year is out");

    field.a_day_goes_by(A_YEAR);
    assert_eq!(field.grade, SoilGrade::Depleted, "a rung for the year");

    field.a_day_goes_by(2 * A_YEAR);
    assert_eq!(field.grade, SoilGrade::Ordinary, "and another for the next");

    field.a_day_goes_by(3 * A_YEAR);
    assert_eq!(field.grade, SoilGrade::Ordinary, "and never past what it was");
}

/// And a year's rest brings a built-up field a rung back down, from the other
/// side: rest restores what the ground was, not more.
#[test]
fn a_year_fallow_lets_a_built_up_field_back_down_a_rung() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    field.a_bean_crop_came_in(0);
    field.a_crop_came_off(1, 1_000, true, 0);
    field.a_bean_crop_came_in(0);
    assert_eq!(field.grade, SoilGrade::VeryRich);

    field.a_day_goes_by(A_YEAR);
    assert_eq!(field.grade, SoilGrade::Rich);
}

/// A crop taken off it is not rest: the year starts again.
#[test]
fn cropping_is_not_rest() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    field.a_crop_came_off(100, 100, false, 0);

    field.a_crop_came_off(10, 100, false, A_YEAR / 2);
    field.a_day_goes_by(A_YEAR);
    assert_eq!(field.grade, SoilGrade::Depleted, "cropped half way through the year");
}

// --- given up ---------------------------------------------------------------

/// Abandoned farmland is fallow until it is back at its natural grade; a year
/// at that grade with nobody working it and it is wild land again.
#[test]
fn a_field_given_up_goes_back_to_the_wild() {
    let mut world = World::new(WorldConfig::default());
    let here = Position::new(14, 14);
    world.grid.get_tile_mut(&here).unwrap().terrain =
        crate::world::Terrain::new(TerrainType::Meadow);
    assert!(world.grid.break_ground(&here, 0));
    world.grid.field_at_mut(&here).unwrap().a_crop_came_off(100, 100, false, 0);
    assert_eq!(world.grid.field_at(&here).unwrap().grade, SoilGrade::Ordinary);

    // A year's rest brings it back to the rich loam it was
    world.grid.a_day_goes_by_for_the_fields(A_YEAR);
    assert_eq!(world.grid.field_at(&here).unwrap().grade, SoilGrade::Rich);
    assert!(
        world.grid.get_tile(&here).unwrap().terrain.is_cultivated(),
        "back at its grade, and still a field"
    );

    // And a year at it with nobody near
    world.grid.a_day_goes_by_for_the_fields(2 * A_YEAR - 1);
    assert!(world.grid.field_at(&here).is_some(), "a day short");
    world.grid.a_day_goes_by_for_the_fields(2 * A_YEAR);

    assert!(world.grid.field_at(&here).is_none(), "not a field any more");
    assert_eq!(
        world.grid.get_tile(&here).unwrap().terrain.terrain_type,
        TerrainType::Meadow,
        "it is the meadow it was"
    );
    assert_eq!(
        world.grid.soil_at(&here),
        Some(SoilType::natural_to(TerrainType::Meadow))
    );
}

/// A field somebody keeps weeding is not given up, whatever its grade.
#[test]
fn a_field_somebody_works_is_not_given_up() {
    let mut field = Field::broken_out_of(TerrainType::Plains, 0);
    for month in 1..=36 {
        let now = month * A_YEAR / 12;
        field.somebody_worked_it(now);
        assert_eq!(field.a_day_goes_by(now), WhatBecameOfIt::StillAField, "month {month}");
    }
}

// --- what a farmer makes of it ---------------------------------------------

/// A field, a crop on it, and one farmer standing in it.
fn a_farmer_on_a_field(crop: ResourceType, grade: SoilGrade) -> (crate::analytics::Simulation, Position) {
    use crate::agents::{AgentConfig, Population};

    let mut world = World::new(WorldConfig::default().with_size(40, 40));
    world.animals.get_all_mut().clear();
    world.resources.clear();

    let here = Position::new(20, 20);
    world.grid.get_tile_mut(&here).unwrap().terrain =
        crate::world::Terrain::new(TerrainType::Plains);
    assert!(world.grid.break_ground(&here, 0));
    world.grid.field_at_mut(&here).unwrap().grade = grade;
    world.resources.push(ResourceNode::new(crop, here, 80));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = crate::analytics::Simulation::new(world, population);
    simulation.population.agents[0].state.position = (20, 20, 0);
    (simulation, here)
}

fn a_full_stand(simulation: &mut crate::analytics::Simulation, here: &Position) {
    let yields = simulation.world.grid.what_it_yields_here(here);
    let full = simulation.world.resources[0].how_heavy_a_crop_it_carries(yields);
    simulation.world.resources[0].amount = full;
}

/// Nobody is told the grade of a field. A full stand of crop on it says what
/// it is.
#[test]
fn a_full_stand_tells_a_farmer_what_his_field_is() {
    for grade in SoilGrade::LADDER {
        let (mut simulation, here) = a_farmer_on_a_field(ResourceType::Grain, grade);
        assert_eq!(
            simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)),
            None,
            "he has no opinion of a field he has not seen a crop on"
        );

        a_full_stand(&mut simulation, &here);
        simulation.looking_at_the_crop(0, 0);

        assert_eq!(
            simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)),
            Some(grade),
            "a full stand on {} ground",
            grade.called()
        );
    }
}

/// And when the field wears, the next full stand tells him that too.
#[test]
fn a_farmer_finds_out_his_field_is_tired_from_the_next_crop() {
    let (mut simulation, here) = a_farmer_on_a_field(ResourceType::Grain, SoilGrade::Rich);
    a_full_stand(&mut simulation, &here);
    simulation.looking_at_the_crop(0, 0);

    simulation.world.grid.field_at_mut(&here).unwrap().grade = SoilGrade::Depleted;
    assert_eq!(
        simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)),
        Some(SoilGrade::Rich),
        "he goes on believing what the last crop told him"
    );

    a_full_stand(&mut simulation, &here);
    simulation.looking_at_the_crop(0, 0);
    assert_eq!(
        simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)),
        Some(SoilGrade::Depleted),
        "until the next one tells him otherwise"
    );
}

/// A crop still filling only says the ground is at least that good.
#[test]
fn a_thin_stand_still_growing_tells_a_farmer_nothing_new() {
    let (mut simulation, here) = a_farmer_on_a_field(ResourceType::Grain, SoilGrade::VeryRich);

    // A crop a quarter grown on the best ground there is looks like a crop on
    // poor ground, so on a field he has no opinion of it gives him none
    simulation.world.resources[0].amount = 10;
    simulation.looking_at_the_crop(0, 0);
    assert_eq!(simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)), None);

    // And once he has one, a thin stand does not lower it
    a_full_stand(&mut simulation, &here);
    simulation.looking_at_the_crop(0, 0);
    simulation.world.resources[0].amount = 10;
    simulation.looking_at_the_crop(0, 0);
    assert_eq!(
        simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)),
        Some(SoilGrade::VeryRich)
    );
}

/// Gathering off a field is a look at it.
#[test]
fn gathering_off_a_field_is_a_look_at_it() {
    let (mut simulation, here) = a_farmer_on_a_field(ResourceType::Grain, SoilGrade::Rich);
    simulation.world.climate.calendar.day_of_year = (0..crate::environment::seasons::DAYS_PER_YEAR)
        .find(|day| ResourceType::Grain.is_it_bearing(*day))
        .expect("grain bears some time in the year");
    a_full_stand(&mut simulation, &here);

    simulation.execute_action(
        &crate::environment::Action::Gather {
            resource_type: "grain".to_string(),
        },
        0,
    );

    assert_eq!(
        simulation.population.agents[0].what_i_make_of_the_field_at((here.x, here.y)),
        Some(SoilGrade::Rich)
    );
}

/// A farmer who thinks a field is worn to nothing ploughs in what stands on
/// it, and sows the bare field again - with beans, if he has them, because
/// that is what tired ground gets. That is a rotation.
#[test]
fn a_field_thought_worn_out_is_ploughed_in_and_sown_to_beans() {
    use crate::agents::InventoryItem;
    use crate::environment::Action;

    let (mut simulation, here) = a_farmer_on_a_field(ResourceType::Grain, SoilGrade::Exhausted);
    a_full_stand(&mut simulation, &here);
    simulation.looking_at_the_crop(0, 0);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("legumes".to_string(), 5, 0.1));
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("grain".to_string(), 5, 0.1));

    let turned_under = simulation.execute_action(&Action::TillSoil, 0);
    assert!(turned_under.success, "{:?}", turned_under.message);
    assert!(
        !simulation.world.resources.iter().any(|r| r.position == here),
        "the worn crop went under"
    );

    let sown = simulation.execute_action(&Action::TillSoil, 0);
    assert!(sown.success, "a bare field takes seed again: {:?}", sown.message);
    let what = simulation
        .world
        .resources
        .iter()
        .find(|r| r.position == here)
        .map(|r| r.resource_type);
    assert_eq!(what, Some(ResourceType::Legumes), "and tired ground gets the pod row");
}

/// A field somebody has no opinion of is not ploughed in on a guess.
#[test]
fn nobody_ploughs_in_a_crop_on_a_field_they_know_nothing_about() {
    let (mut simulation, here) = a_farmer_on_a_field(ResourceType::Grain, SoilGrade::Exhausted);
    a_full_stand(&mut simulation, &here);

    let result = simulation.execute_action(&crate::environment::Action::TillSoil, 0);
    assert!(!result.success, "he has never seen a crop come off it");
    assert!(simulation.world.resources.iter().any(|r| r.position == here));
}
