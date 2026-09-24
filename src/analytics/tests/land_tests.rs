// src/analytics/tests/land_tests.rs
//! Tests for the ground: what plants need out of it, what changes it, and how
//! a settlement works that out for itself.
//!
//! The ground here used to be a pool of nutrient on every tile with two pools
//! of litter rotting into it. It is a kind and a grade now: wild ground is
//! what its terrain made it, and only people move a field's grade - see
//! `world::soil` and ISSUES_FOUND #246. The tests that were about the rot -
//! `what_rots_depends_on_where_it_fell`, `dense_matter_outlasts_soft`,
//! `what_rots_feeds_the_ground`, `a_crop_draws_the_ground_down`,
//! `a_wood_feeds_itself` - have gone with it. What a field does is held in
//! `rotation_tests` and `soil_ladder_tests`.

use crate::agents::practices::{Practice, Practices};
use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::flora::GrowingConditions;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// The ground under a marsh is not the ground under a dune.
#[test]
fn the_country_decides_what_the_soil_is_worth() {
    use crate::world::SoilType;

    let (_, marsh) = SoilType::natural_to(TerrainType::Wetland);
    let (_, wood) = SoilType::natural_to(TerrainType::Forest);
    let (_, dune) = SoilType::natural_to(TerrainType::Desert);

    assert!(
        marsh > wood && wood > dune,
        "marsh, then wood, then sand: {} {} {}",
        marsh.called(),
        wood.called(),
        dune.called()
    );

    // And each keeps what kind of ground it is
    assert_eq!(SoilType::natural_to(TerrainType::Wetland).0, SoilType::Peat);
    assert_eq!(SoilType::natural_to(TerrainType::Desert).0, SoilType::Sand);
}

/// A plant grows at the pace of whatever it has least of.
#[test]
fn the_scarcest_thing_sets_the_pace() {
    let plenty = GrowingConditions::ideal();
    assert_eq!(plenty.growth_share(), 1.0);

    let parched = GrowingConditions {
        water: 0.1,
        ..GrowingConditions::ideal()
    };
    let starved = GrowingConditions {
        nutrients: 0.1,
        ..GrowingConditions::ideal()
    };
    let shaded = GrowingConditions {
        light: 0.1,
        ..GrowingConditions::ideal()
    };

    for (name, conditions) in [
        ("water", parched),
        ("nutrients", starved),
        ("light", shaded),
    ] {
        assert!(
            conditions.growth_share() < 0.3,
            "short of {name} should hold a plant back: {:.2}",
            conditions.growth_share()
        );
    }

    // And having plenty of the others does not make up for it
    let one_thing_missing = GrowingConditions {
        water: 1.0,
        light: 1.0,
        nutrients: 0.2,
        uptake: 1.0,
    };
    assert!(
        (one_thing_missing.growth_share() - 0.2).abs() < 0.001,
        "all the sun and rain in the world will not feed a plant"
    );
}

/// Broken ground helps a plant get at what is there. It does not make the
/// plant grow faster than its kind can grow.
#[test]
fn a_field_helps_a_plant_feed_itself_but_cannot_hurry_it() {
    let thin_ground = GrowingConditions {
        water: 1.0,
        light: 1.0,
        nutrients: 0.3,
        uptake: 1.0,
    };
    let same_ground_worked = GrowingConditions {
        uptake: 2.5,
        ..thin_ground
    };

    assert!(
        same_ground_worked.growth_share() > thin_ground.growth_share(),
        "a worked field should get more out of the same soil"
    );

    // But never past the natural best
    let rich_and_worked = GrowingConditions {
        water: 1.0,
        light: 1.0,
        nutrients: 1.0,
        uptake: 4.0,
    };
    assert_eq!(
        rich_and_worked.growth_share(),
        1.0,
        "nothing grows faster than its kind grows"
    );
}

/// Ground that has been worked out carries almost nothing.
///
/// The yield floor used to be four tenths of the full crop whatever the ground
/// was like, so a field mined down to a twentieth of its fertility still
/// nominally carried nearly half a crop. On the ladder the fall is the grade's
/// multiplier and nothing props it up: exhausted ground carries an eighth of
/// what very rich ground does.
#[test]
fn a_worked_out_field_carries_almost_nothing() {
    use crate::world::SoilGrade;

    let field = ResourceNode::new(ResourceType::Grain, Position::new(5, 5), 100);

    let rich = field.how_heavy_a_crop_it_carries(SoilGrade::VeryRich.multiplier());
    let fair = field.how_heavy_a_crop_it_carries(SoilGrade::Ordinary.multiplier());
    let spent = field.how_heavy_a_crop_it_carries(SoilGrade::Exhausted.multiplier());

    assert_eq!(rich, 100, "the richest wild ground carries a full crop");
    assert_eq!(fair, 50, "ordinary ground carries half of one");
    assert!(
        spent < 15,
        "exhausted ground should carry next to nothing, not {spent}"
    );
}

/// Thin ground grows less, and a field grows more than the same ground wild.
///
/// This used to add that a worked field could not beat ground that simply had
/// more in it. Under "wild plants produce yields 1/4th that of plants in
/// tilled farmland" it can: a field of depleted loam grows as fast as the
/// richest wild ground there is.
#[test]
fn a_patch_grows_as_well_as_the_ground_it_stands_in() {
    use crate::world::soil::WHAT_BROKEN_GROUND_YIELDS_OVER_WILD;
    use crate::world::SoilGrade;

    fn grown_in(grade: SoilGrade, cultivated: bool) -> u32 {
        // Room enough that nothing here runs into the ceiling: what the
        // ground can carry is a separate question from how fast it fills, and
        // this test is about the second one
        let mut patch = ResourceNode::new(ResourceType::Grain, Position::new(5, 5), 20_000);
        patch.amount = 0;
        let yields = grade.multiplier()
            * if cultivated {
                WHAT_BROKEN_GROUND_YIELDS_OVER_WILD
            } else {
                1.0
            };

        for _ in 0..200 {
            patch.regenerate_in_ground(20.0, 0.6, 1.0, yields, 1.0, crate::world::ResourceNode::WHAT_THESE_RATES_WERE_FITTED_TO);
        }

        patch.amount
    }

    let rich = grown_in(SoilGrade::VeryRich, false);
    let thin = grown_in(SoilGrade::Exhausted, false);
    let thin_but_worked = grown_in(SoilGrade::Exhausted, true);

    assert!(
        rich > thin,
        "rich ground should outgrow thin: {rich} against {thin}"
    );
    assert!(
        thin_but_worked > thin,
        "and a worked field should beat the same ground left wild: {thin_but_worked} against {thin}"
    );
}

/// A world starts with vegetation standing on it.
#[test]
fn a_world_has_plants_growing_on_it() {
    let world = World::new(WorldConfig::default());

    assert!(
        !world.plants.all_plants().is_empty(),
        "nothing had ever created a plant: the flora system ran over an empty list"
    );

    // And they are where they should be
    let on_land = world.plants.all_plants().iter().all(|plant| {
        world
            .grid
            .get_tile(&Position::new(plant.position.0, plant.position.1))
            .map(|tile| tile.terrain.terrain_type != TerrainType::Water)
            .unwrap_or(false)
    });

    assert!(on_land, "plants should not be growing in open water");
}

/// Tipping spoiled food on a field enriches it, a season on.
#[test]
fn muck_makes_the_ground_richer() {
    use crate::environment::Action;
    use crate::world::soil::A_SEASON;

    let mut world = World::new(WorldConfig::default());
    let field = Position::new(25, 25);
    if let Some(tile) = world.grid.get_tile_mut(&field) {
        tile.terrain = crate::world::Terrain::new(TerrainType::Plains);
    }
    assert!(world.grid.break_ground(&field, 0));
    let started = world.grid.field_at(&field).expect("just broken").grade;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);

    // A pack of food that has turned: enough of it to come to a rung
    let database = FoodDatabase::new();
    let mut rotten = InventoryItem::new_with_weight("food".to_string(), 12, 0.5);
    let mut food_data = database
        .create_food_data(&ItemType::Food, 0)
        .expect("generic food should be in the database");
    food_data.freshness = 0.05;
    rotten.food_data = Some(food_data);
    simulation.population.agents[0].inventory.add_item(rotten);

    let result = simulation.execute_action(&Action::SpreadMuck, 0);
    assert!(result.success, "tipping a basket out should work: {:?}", result.message);

    let ready = simulation
        .world
        .grid
        .field_at(&field)
        .and_then(|field| field.mucked_ready_at)
        .expect("a dozen rotten units on a field is a rung's worth of muck");
    assert!(ready >= simulation.world.turn + A_SEASON, "and it wants its season");

    simulation.world.grid.a_day_goes_by_for_the_fields(ready);
    assert_eq!(
        simulation.world.grid.field_at(&field).expect("still a field").grade,
        started.richer(),
        "a season on, the field is a rung richer"
    );

    // And the agent is no longer carrying it
    assert!(
        !simulation.population.agents[0]
            .inventory
            .has_item("food", 1),
        "what was tipped out is not still in the pack"
    );

    // It also formed an opinion about whether that was worth doing
    assert!(
        simulation.population.agents[0]
            .practices
            .attempts(Practice::SpreadingMuck)
            > 0,
        "an agent that tries something should remember trying it"
    );
}

/// A practice is tried, judged, and either kept or dropped.
#[test]
fn a_practice_is_learned_by_trying_it() {
    let mut works = Practices::new();
    let mut does_not = Practices::new();

    // Nobody starts out believing in anything
    assert!(!works.is_established(Practice::SpreadingMuck));
    assert_eq!(works.confidence(Practice::SpreadingMuck), 0.0);

    for _ in 0..4 {
        works.record_outcome(Practice::SpreadingMuck, true);
        does_not.record_outcome(Practice::SpreadingMuck, false);
    }

    assert!(
        works.is_established(Practice::SpreadingMuck),
        "something that keeps working becomes what you do"
    );
    assert!(
        !does_not.is_established(Practice::SpreadingMuck),
        "something that never works does not"
    );

    // And an agent that has tried it and got nothing stops trying
    for _ in 0..4 {
        does_not.record_outcome(Practice::SpreadingMuck, false);
    }
    assert!(
        !does_not.would_try(Practice::SpreadingMuck, 1.0, 0.0),
        "a practice tried and found useless is dropped"
    );
}

/// Watching somebody else do it counts, and counts for less than doing it.
#[test]
fn a_practice_spreads_by_being_seen() {
    let mut watcher = Practices::new();
    let mut doer = Practices::new();

    for _ in 0..3 {
        watcher.learn_from_watching(Practice::SpreadingMuck);
        doer.record_outcome(Practice::SpreadingMuck, true);
    }

    assert!(
        watcher.confidence(Practice::SpreadingMuck) > 0.0,
        "seeing a thing done should count for something"
    );
    assert!(
        doer.confidence(Practice::SpreadingMuck) > watcher.confidence(Practice::SpreadingMuck),
        "and for less than doing it: {:.2} watching against {:.2} doing",
        watcher.confidence(Practice::SpreadingMuck),
        doer.confidence(Practice::SpreadingMuck)
    );

    // Enough watching does eventually settle it
    for _ in 0..10 {
        watcher.learn_from_watching(Practice::SpreadingMuck);
    }
    assert!(
        watcher.is_established(Practice::SpreadingMuck),
        "a practice everybody around you follows becomes what you do too"
    );
}

// --- how much country there is -----------------------------------------------

/// A map four times the size carries four times as much.
///
/// Every count in `ResourceConfig` is written for a fifty by fifty map and
/// spread over whatever map is actually being built. Without that a hundred
/// square kilometres came out with the same three hundred and sixty-odd nodes
/// a quarter of a square kilometre had, spread over four hundred times the
/// ground: a country with a berry bush every half-mile, which is not country
/// anybody could live in.
#[test]
fn a_bigger_map_carries_more() {
    crate::core::dice::seed(31);
    let small = World::new(WorldConfig::default().with_size(60, 60));
    crate::core::dice::seed(31);
    let large = World::new(WorldConfig::default().with_size(120, 120));

    let per_tile = |world: &World| {
        world.resources.len() as f32 / (world.grid.width * world.grid.height) as f32
    };

    let thin = per_tile(&small);
    let thick = per_tile(&large);

    assert!(
        large.resources.len() > small.resources.len() * 3,
        "four times the ground carried {} against {}",
        large.resources.len(),
        small.resources.len()
    );

    // And it is the same country, not a richer one: density within a tenth.
    assert!(
        (thick - thin).abs() < thin * 0.1,
        "{thin:.4} nodes a tile on the small map, {thick:.4} on the large"
    );
}

/// Stocking a map leaves no two things standing on one tile.
///
/// Placement used to ask `is_position_occupied`, which walks the whole
/// resource list, once for every node it put down - the square of the map,
/// which a hundred square kilometres would not finish. The scan is hoisted
/// into a register carried through the three spawners now, and a register is
/// a second representation of something the map already says. This is what
/// says the two still agree.
#[test]
fn stocking_a_map_leaves_no_two_things_on_a_tile() {
    use std::collections::BTreeSet;

    crate::core::dice::seed(17);
    let world = World::new(WorldConfig::default().with_size(80, 80));

    // What the register covers is the ground the two spawners that consult it
    // put things on. The naturalistic spawner works off its own list and has
    // never asked, so its clusters land on top of each other and on top of
    // everything else - see ISSUES_FOUND.md #130. Held out here rather than
    // quietly folded in, because a test that passes by lowering its sights is
    // worse than no test.
    let cluster_kinds = [
        ResourceType::Clay,
        ResourceType::Sand,
        ResourceType::Coal,
        ResourceType::Grain,
        ResourceType::Flax,
        ResourceType::Herbs,
        ResourceType::Cotton,
        ResourceType::Honey,
        ResourceType::Fish,
    ];

    let mut seen: BTreeSet<(i32, i32)> = BTreeSet::new();
    let mut doubled = Vec::new();

    for resource in &world.resources {
        if cluster_kinds.contains(&resource.resource_type) {
            continue;
        }
        if !seen.insert((resource.position.x, resource.position.y)) {
            doubled.push((resource.position.x, resource.position.y, resource.resource_type));
        }
    }

    assert!(
        doubled.is_empty(),
        "{} tiles have two things on them: {:?}",
        doubled.len(),
        &doubled[..doubled.len().min(5)]
    );
}

/// A cell is ten metres, and a hundred square kilometres is what it says.
#[test]
fn the_map_an_ecology_needs_is_a_hundred_square_kilometres() {
    use crate::world::Grid;

    let config = WorldConfig::big_enough_for_an_ecology();
    let (width, height) = config.size;
    let metres = Grid::METRES_PER_CELL * Grid::METRES_PER_CELL;
    let square_kilometres = (width * height) as f32 * metres / 1_000_000.0;

    assert!(
        (square_kilometres - 100.0).abs() < 0.001,
        "{width}x{height} at {} metres a cell is {square_kilometres} km2",
        Grid::METRES_PER_CELL
    );
}
