// src/analytics/tests/farming_tests.rs
//! Tests for farming as something a people has to work out and then keep up.
//!
//! "The agents need to learn to farm. They need to discover which plants are
//! suitable for farming. Farmers should not just drop seeds and get crops. They
//! need to maintain the fields, clearing 'weeds' and removing 'pests'."
//!
//! Three things had to become true. A field left alone has to go over: weeds
//! and vermin take it, and what an agent walks back to is bare ground. Working
//! it has to be an action with a cost, chosen because the field wants working.
//! And breaking ground in the first place has to be a thing nobody starts out
//! knowing - what teaches it is the midden, where a people that voided the pips
//! of what it ate walks past a season later and finds the same plants standing
//! in its own refuse.

use crate::agents::practices::Practice;
use crate::agents::{AgentConfig, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::soil::Soil;
use crate::world::{Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig};

/// One agent standing on one tile of broken ground, and nothing else in the way
fn a_farmer_on_a_field(where_it_is: Position) -> Simulation {
    let mut world = World::new(WorldConfig::default());

    world
        .resources
        .retain(|resource| resource.position != where_it_is);

    if let Some(tile) = world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Farmland);
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (where_it_is.x, where_it_is.y, 0);
    simulation
}

// --------------------------------------------------------------------------
// A field left alone goes over
// --------------------------------------------------------------------------

/// Weeds and pests come on in ground that is growing something and nobody is
/// looking after.
#[test]
fn a_field_nobody_works_goes_over_to_weeds() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);

    assert_eq!(soil.weeds, 0.0, "fresh ground starts clean");
    assert_eq!(soil.pests, 0.0);

    // A season's worth of growing weather with nobody in the field
    for _ in 0..(crate::environment::seasons::DAYS_PER_SEASON
        * crate::environment::seasons::TICKS_PER_DAY)
    {
        soil.nobody_weeded_this(1.0, 1.0);
    }

    assert!(
        soil.weeds > 0.5,
        "a season unattended should be thick with weeds, not {:.2}",
        soil.weeds
    );
    assert!(
        soil.pests > 0.2,
        "and carrying vermin, not {:.2}",
        soil.pests
    );
    assert!(
        soil.wants_working(),
        "and it should be obvious to anybody standing in it that it wants working"
    );
}

/// Nothing comes on in ground with nothing growing in it: this is what a field
/// loses, not what any ground loses.
#[test]
fn bare_ground_grows_no_weeds_worth_pulling() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);

    for _ in 0..1000 {
        soil.nobody_weeded_this(0.0, 1.0);
    }

    assert_eq!(soil.weeds, 0.0, "nothing growing, nothing to compete with");
    assert!(!soil.wants_working());
}

/// What the weeds and the vermin leave is what the farmer gets.
#[test]
fn what_the_weeds_leave_is_what_the_crop_keeps() {
    let clean = Soil::for_terrain(TerrainType::Plains);
    assert_eq!(
        clean.what_the_crop_keeps(),
        1.0,
        "a clean field loses nothing"
    );

    let mut going_over = Soil::for_terrain(TerrainType::Plains);
    for _ in 0..200 {
        going_over.nobody_weeded_this(1.0, 1.0);
    }

    let mut overrun = Soil::for_terrain(TerrainType::Plains);
    for _ in 0..2000 {
        overrun.nobody_weeded_this(1.0, 1.0);
    }

    assert!(
        going_over.what_the_crop_keeps() < 1.0,
        "a field going over carries less"
    );
    assert!(
        overrun.what_the_crop_keeps() < going_over.what_the_crop_keeps(),
        "and one that has gone right over carries less again: {:.2} against {:.2}",
        overrun.what_the_crop_keeps(),
        going_over.what_the_crop_keeps()
    );
    assert!(
        overrun.what_the_crop_keeps() >= 0.1,
        "though never nothing at all - somebody gets something off it"
    );
}

/// The whole of the point: two identical fields, one worked and one not, and
/// the difference is a crop.
#[test]
fn a_worked_field_carries_a_crop_and_a_neglected_one_does_not() {
    fn grown(worked: bool) -> u32 {
        let mut soil = Soil::for_terrain(TerrainType::Plains);
        // Large enough that neither run reaches the ceiling: this is about
        // what the weeds take out of the growing, not what the ground carries
        let mut field = ResourceNode::new(ResourceType::Grain, Position::new(10, 10), 40000);
        field.amount = 0;

        for tick in 0..1200 {
            soil.nutrients = 0.6;
            soil.nobody_weeded_this(1.0, 1.0);

            // A turn round the field every few days, which is what a farmer
            // with a field actually does with a season
            if worked && tick % 30 == 0 {
                soil.somebody_worked_this_field();
            }

            field.regenerate_in_ground(20.0, 0.6, 1.0, true, &mut soil);
        }

        field.amount
    }

    let tended = grown(true);
    let abandoned = grown(false);

    assert!(
        tended > abandoned * 2,
        "working the field should be most of the crop: {tended} against {abandoned}"
    );
}

/// One turn round the field puts a good deal of it right, and it goes over
/// again if nobody comes back.
#[test]
fn working_a_field_puts_it_right_for_a_while() {
    let mut soil = Soil::for_terrain(TerrainType::Plains);

    for _ in 0..400 {
        soil.nobody_weeded_this(1.0, 1.0);
    }

    let gone_over = soil.weeds + soil.pests;
    soil.somebody_worked_this_field();
    let after = soil.weeds + soil.pests;

    assert!(
        after < gone_over,
        "a turn round the field should tell: {after:.2} against {gone_over:.2}"
    );

    for _ in 0..800 {
        soil.nobody_weeded_this(1.0, 1.0);
    }

    assert!(
        soil.weeds + soil.pests > after,
        "and it goes over again the moment nobody comes back"
    );
}

// --------------------------------------------------------------------------
// Working a field is an action with a cost
// --------------------------------------------------------------------------

/// An agent standing in a field that wants working can work it.
#[test]
fn an_agent_can_work_a_field_that_wants_working() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_farmer_on_a_field(where_it_is);

    if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
        for _ in 0..400 {
            tile.soil.nobody_weeded_this(1.0, 1.0);
        }
    }

    let before = simulation
        .world
        .grid
        .get_tile(&where_it_is)
        .map(|tile| tile.soil.weeds + tile.soil.pests)
        .unwrap_or(0.0);

    assert!(before > 0.0, "the field should have gone over first");

    let result = simulation.execute_action(&Action::TendField, 0);
    assert!(
        result.success,
        "a field that wants working can be worked: {:?}",
        result.message
    );
    assert!(result.energy_cost > 0.0, "and it is a day's work");

    let after = simulation
        .world
        .grid
        .get_tile(&where_it_is)
        .map(|tile| tile.soil.weeds + tile.soil.pests)
        .unwrap_or(0.0);

    assert!(
        after < before,
        "and the weeds should be down: {after:.2} against {before:.2}"
    );
}

/// There is nothing to do in a field that is already clean, and nothing to do
/// on ground nobody has broken.
#[test]
fn there_is_nothing_to_work_in_clean_ground_or_open_country() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_farmer_on_a_field(where_it_is);

    let clean = simulation.execute_action(&Action::TendField, 0);
    assert!(!clean.success, "a clean field wants nothing doing");

    // And the same tile left as open grass
    if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Plains);
        for _ in 0..400 {
            tile.soil.nobody_weeded_this(1.0, 1.0);
        }
    }

    let meadow = simulation.execute_action(&Action::TendField, 0);
    assert!(
        !meadow.success,
        "a meadow is not a field, however weedy: {:?}",
        meadow.message
    );
}

/// A practised hand gets round more of the field in a turn than a beginner.
#[test]
fn a_practised_farmer_clears_more_in_a_turn() {
    fn cleared(experience: u32) -> f32 {
        let where_it_is = Position::new(25, 25);
        let mut simulation = a_farmer_on_a_field(where_it_is);

        simulation.population.agents[0]
            .skills
            .practise(SkillType::Farming, experience, 0);

        if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
            tile.soil.weeds = Soil::OVERRUN;
            tile.soil.pests = Soil::OVERRUN;
        }

        let before = 2.0 * Soil::OVERRUN;
        simulation.execute_action(&Action::TendField, 0);

        before
            - simulation
                .world
                .grid
                .get_tile(&where_it_is)
                .map(|tile| tile.soil.weeds + tile.soil.pests)
                .unwrap_or(0.0)
    }

    let beginner = cleared(0);
    let old_hand = cleared(200_000);

    assert!(
        old_hand > beginner,
        "years in a field should show: {old_hand:.2} against {beginner:.2}"
    );
}

// --------------------------------------------------------------------------
// Which plants are suitable
// --------------------------------------------------------------------------

/// Not every plant repays a plough.
#[test]
fn grain_repays_the_plough_and_a_berry_bush_does_not() {
    assert!(
        ResourceType::Grain.takes_to_the_plough() > ResourceType::Food.takes_to_the_plough(),
        "grain is the plant farming was invented for"
    );
    assert!(
        ResourceType::Food.takes_to_the_plough() < 1.5,
        "and a berry bush in rows is still a berry bush"
    );

    let grain = ResourceNode::new(ResourceType::Grain, Position::new(1, 1), 100);
    let berries = ResourceNode::new(ResourceType::Food, Position::new(1, 1), 100);

    assert!(
        grain.how_heavy_a_crop_it_carries(0.6, true)
            > berries.how_heavy_a_crop_it_carries(0.6, true),
        "a field of grain stands thicker than a field of berry bushes"
    );

    assert_eq!(
        grain.how_heavy_a_crop_it_carries(0.6, false),
        grain.standing_capacity(0.6),
        "and unbroken ground carries what it always carried"
    );
}

/// An agent sows what it has, and prefers what it has found works.
#[test]
fn an_agent_sows_what_it_has_come_to_trust() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_farmer_on_a_field(where_it_is);

    // Open grass again, so there is ground to break
    if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Plains);
    }

    // A pack with both in it, and three seasons of walking back to a bare
    // field of berries behind it
    {
        let agent = &mut simulation.population.agents[0];
        agent
            .inventory
            .add_item(crate::agents::InventoryItem::new_with_weight(
                "grain".to_string(),
                10,
                0.5,
            ));
        agent
            .inventory
            .add_item(crate::agents::InventoryItem::new_with_weight(
                "food".to_string(),
                10,
                0.5,
            ));

        for _ in 0..40 {
            agent.lessons.record_particular("sow:food", false);
            agent.lessons.record_particular("sow:grain", true);
        }
    }

    let result = simulation.execute_action(&Action::TillSoil, 0);
    assert!(result.success, "there is ground and there is seed");

    let sown = simulation
        .world
        .resources
        .iter()
        .find(|resource| resource.position == where_it_is)
        .map(|resource| resource.resource_type);

    assert_eq!(
        sown,
        Some(ResourceType::Grain),
        "a farmer who has learned what grain does sows grain"
    );
}

/// And a field that comes to nothing teaches the agent that walks back to it.
#[test]
fn a_field_that_comes_to_nothing_teaches_what_not_to_sow() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_farmer_on_a_field(where_it_is);

    // A sown field of berry bushes with nothing standing in it
    let mut sown = ResourceNode::new(ResourceType::Food, where_it_is, 80);
    sown.amount = 0;
    simulation.world.resources.push(sown);

    if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
        tile.soil.weeds = Soil::OVERRUN;
        tile.soil.pests = Soil::OVERRUN;
    }

    let before = simulation.population.agents[0]
        .lessons
        .how_likely_to_try_this("sow:food");

    for _ in 0..30 {
        simulation.execute_action(&Action::TendField, 0);

        // Left to go over again between visits
        if let Some(tile) = simulation.world.grid.get_tile_mut(&where_it_is) {
            tile.soil.weeds = Soil::OVERRUN;
            tile.soil.pests = Soil::OVERRUN;
        }
    }

    let after = simulation.population.agents[0]
        .lessons
        .how_likely_to_try_this("sow:food");

    assert!(
        after < before,
        "thirty walks out to a bare field should tell: {after:.2} against {before:.2}"
    );
}

// --------------------------------------------------------------------------
// Farming as something discovered
// --------------------------------------------------------------------------

/// Nobody starts out believing in it.
#[test]
fn nobody_is_born_knowing_that_seed_in_the_ground_comes_back() {
    let mut population = Population::new();
    for _ in 0..20 {
        population.spawn_agent(AgentConfig::default());
    }

    for agent in &population.agents {
        assert_eq!(
            agent.practices.confidence(Practice::Farming),
            0.0,
            "a founder has never seen it done"
        );
        assert!(!agent.practices.is_established(Practice::Farming));
    }
}

/// And an agent that has not worked it out only breaks ground out of curiosity,
/// where one that has does it as a matter of course.
#[test]
fn breaking_ground_is_a_hunch_until_it_is_a_practice() {
    let mut unconvinced = crate::agents::practices::Practices::new();
    let mut convinced = crate::agents::practices::Practices::new();

    convinced.saw_it_work(Practice::Farming);
    convinced.saw_it_work(Practice::Farming);

    assert!(
        convinced.is_established(Practice::Farming),
        "two sights of the thing itself settle it"
    );

    // Over a hundred opportunities apiece, at the same curiosity
    let rolls: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();

    let hunches = rolls
        .iter()
        .filter(|roll| unconvinced.would_try(Practice::Farming, 0.5, **roll))
        .count();
    let habits = rolls
        .iter()
        .filter(|roll| convinced.would_try(Practice::Farming, 0.5, **roll))
        .count();

    assert!(
        hunches < 20,
        "an agent that has never seen it work should rarely bother: {hunches} in 100"
    );
    assert_eq!(habits, 100, "one that has, does it whenever the chance comes");

    // And something tried and tried and found useless is dropped
    for _ in 0..8 {
        unconvinced.record_outcome(Practice::Farming, false);
    }
    assert!(
        !unconvinced.would_try(Practice::Farming, 1.0, 0.0),
        "eight goes at nothing is enough for anybody"
    );
}

/// Seeing the outcome is worth a great deal more than watching somebody do it.
#[test]
fn seeing_it_come_up_teaches_more_than_watching_somebody_dig() {
    let mut saw_it = crate::agents::practices::Practices::new();
    let mut heard_of_it = crate::agents::practices::Practices::new();

    saw_it.saw_it_work(Practice::Farming);
    heard_of_it.learn_from_watching(Practice::Farming);

    assert!(
        saw_it.confidence(Practice::Farming) > heard_of_it.confidence(Practice::Farming),
        "the crop standing in the midden is the argument, not the digging"
    );
}

/// A crop carried home off broken ground settles the question.
#[test]
fn a_crop_off_a_field_is_what_settles_it() {
    let where_it_is = Position::new(25, 25);
    let mut simulation = a_farmer_on_a_field(where_it_is);

    // A field with something standing in it, and an agent beside it
    let mut standing = ResourceNode::new(ResourceType::Grain, where_it_is, 80);
    standing.amount = 60;
    simulation.world.resources.push(standing);

    assert_eq!(
        simulation.population.agents[0]
            .practices
            .confidence(Practice::Farming),
        0.0
    );

    for _ in 0..4 {
        simulation.execute_action(
            &Action::Gather {
                resource_type: "food".to_string(),
            },
            0,
        );
    }

    let agent = &simulation.population.agents[0];
    assert!(
        agent.practices.is_established(Practice::Farming),
        "four armfuls off a field is an argument nobody talks you out of: {:.2}",
        agent.practices.confidence(Practice::Farming)
    );
    assert!(
        agent.lessons.how_likely_to_try_this("sow:grain")
            >= crate::agents::practices::Lessons::UNTRIED,
        "and it was grain that did it"
    );
}

/// The midden closes the loop: what a people threw away comes up, and whoever
/// is standing near enough to see it learns what seed in the ground does.
#[test]
fn a_volunteer_on_the_midden_teaches_whoever_sees_it() {
    let where_it_is = Position::new(25, 25);
    let mut world = World::new(WorldConfig::default());

    world
        .resources
        .retain(|resource| resource.position != where_it_is);

    // A midden broken down far enough for what was voided in it to come up
    if let Some(tile) = world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Plains);
        for _ in 0..40 {
            tile.soil.somebody_voided_here(1.0);
        }
        for _ in 0..2000 {
            tile.soil.decay(1.0, 20.0);
        }
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (where_it_is.x + 1, where_it_is.y, 0);
    simulation.population.agents[1].state.position = (where_it_is.x + 40, where_it_is.y, 0);

    assert!(
        simulation
            .world
            .grid
            .get_tile(&where_it_is)
            .map(|tile| tile.soil.ready_to_sprout())
            .unwrap_or(false),
        "the midden should be ready to come up"
    );

    simulation.what_was_dropped_comes_up();

    assert!(
        simulation
            .world
            .resources
            .iter()
            .any(|resource| resource.position == where_it_is),
        "something should have come up out of it"
    );

    assert!(
        simulation.population.agents[0]
            .practices
            .confidence(Practice::Farming)
            > 0.0,
        "the one standing beside it learns what it is looking at"
    );
    assert_eq!(
        simulation.population.agents[1]
            .practices
            .confidence(Practice::Farming),
        0.0,
        "and the one forty tiles away learns nothing"
    );
}
