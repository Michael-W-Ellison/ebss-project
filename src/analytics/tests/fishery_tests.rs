// src/analytics/tests/fishery_tests.rs
//! Tests for the one food a settlement does not grow.
//!
//! Every other return path in this simulation gives the ground back some part
//! of what the ground already paid out, and rot takes its cut on the way, so
//! the best a farming people can do is run down slowly. A fish is different in
//! kind. It was grown at sea and fed on a whole catchment, and it comes up the
//! river under its own power whatever last year's fishing left behind. What is
//! left of it, put on a field, makes the country richer than it was.
//!
//! Which is why people who had rivers buried fish with the seed corn for
//! thousands of years before anybody could say what nitrogen was.

use crate::agents::practices::Undertaking;
use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::{Season, DAYS_PER_SEASON, ONCE_A_DAY, PLANNING_PERIODS_PER_YEAR};
use crate::environment::Action;
use crate::world::soil::Soil;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// A patch of river, and a bank beside it.
fn a_world_with_a_river() -> World {
    let mut world = World::new(WorldConfig::default());

    for x in 0..6 {
        for y in 0..3 {
            let here = Position::new(x, y);
            if let Some(tile) = world.grid.get_tile_mut(&here) {
                tile.terrain.terrain_type = if y == 0 {
                    TerrainType::Water
                } else {
                    TerrainType::Riverbank
                };
            }
        }
    }

    world
}

/// A reach fished down to nothing fills again from upstream.
///
/// This is the whole difference between a fishery and a berry hedge. A hedge
/// regrows out of what is left of itself, so taking all of it ends it. Fish
/// are spawned upstream and fed at sea and come back regardless.
///
/// **A year, because a year is what the fishery is fitted to.** This used to
/// wind on four thousand turns and assert that a reach of sixty was half full.
/// Four thousand turns is eighty-three days - not quite a season - and at the
/// per-pass rate the run then had, a single spring brought ninety fish into a
/// reach that holds sixty, so eighty-three days was ample and the assertion
/// was easy. It is the wrong question either way: what
/// `WHAT_A_FULL_RUN_BRINGS_IN_A_SEASON` is fitted to is that a reach comes
/// back *across a year*, most of it in the two runs, and a test that gives it
/// a quarter of one is testing something nobody designed.
#[test]
fn a_reach_fished_out_fills_again() {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);

    let mut reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);
    reach.amount = 0; // fished to nothing
    world.resources.push(reach);

    let holds = 60;
    let mut after_a_season = 0;

    for turn in 0..PLANNING_PERIODS_PER_YEAR {
        world.take_a_turn();

        if turn == PLANNING_PERIODS_PER_YEAR / 4 {
            after_a_season = what_is_in_the_reach(&world);
        }
    }

    let after_a_year = what_is_in_the_reach(&world);

    assert!(
        after_a_season > 0,
        "the spring run should have started putting fish back inside a season; \
         the reach held {after_a_season}"
    );
    assert!(
        after_a_season < holds,
        "and should not have filled a whole reach in one season, or there is \
         no lean stretch for anybody to arrange a year around: {after_a_season}"
    );
    assert!(
        after_a_year > holds / 2,
        "a reach fished to nothing comes back across a year, most of it in the \
         two runs; after one it held {after_a_year} of {holds}"
    );
}

/// What is in the one reach this file's worlds have.
fn what_is_in_the_reach(world: &World) -> u32 {
    world
        .resources
        .iter()
        .find(|r| r.resource_type == ResourceType::Fish)
        .map(|r| r.amount)
        .unwrap_or(0)
}

/// A season's run is a season's run, whatever the tick happens to be.
///
/// The thing the fishery holds fixed. A run is something that happens to a
/// river over a spring; how many times the world looks at the river while it
/// is happening is an implementation detail of the tick, and used to be the
/// number the fishery was written in. When a pass went from twenty hours to a
/// day and a season from twenty-four days to ninety, a full spring quietly
/// went from twenty-nine fish to ninety.
#[test]
fn a_full_run_brings_the_same_season_whatever_the_pass_is() {
    let reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);
    let wanted = ResourceNode::WHAT_A_FULL_RUN_BRINGS_IN_A_SEASON;

    // Counted a day at a time, which is what the world does.
    let a_day = ONCE_A_DAY;
    let daily = reach.fish_run(TerrainType::Water, Season::Spring, false, a_day)
        * DAYS_PER_SEASON as f32;

    assert!(
        (daily - wanted).abs() < 0.01,
        "a spring of daily passes should bring {wanted}, and brought {daily}"
    );

    // And half a day at a time, which it does not - but the fishery must not
    // notice the difference, because that is the whole point of stating the
    // season rather than the pass.
    let twice_a_day = ONCE_A_DAY / 2;
    let halved = reach.fish_run(TerrainType::Water, Season::Spring, false, twice_a_day)
        * (DAYS_PER_SEASON * 2) as f32;

    assert!(
        (halved - wanted).abs() < 0.01,
        "twice as many passes should each bring half as much, and brought {halved}"
    );
}

/// The run is a run: heavy twice a year and thin between.
#[test]
fn the_run_comes_in_spring_and_autumn() {
    let reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);

    let a_day = ONCE_A_DAY;
    let spring = reach.fish_run(TerrainType::Water, Season::Spring, false, a_day);
    let summer = reach.fish_run(TerrainType::Water, Season::Summer, false, a_day);
    let fall = reach.fish_run(TerrainType::Water, Season::Fall, false, a_day);
    let winter = reach.fish_run(TerrainType::Water, Season::Winter, false, a_day);

    assert!(spring > fall, "the spring run is the heavier of the two");
    assert!(fall > summer, "high summer is after the run, not in it");
    assert!(summer > winter, "and winter is thinnest of all");
    assert!(winter > 0.0, "but a river is never quite empty");

    let frozen = reach.fish_run(TerrainType::Water, Season::Winter, true, a_day);
    assert!(frozen < winter, "a frozen river gives up almost nothing");

    // A river carries a run; a pond gets whatever wandered in
    let pond = reach.fish_run(TerrainType::Plains, Season::Spring, false, a_day);
    assert!(
        pond < spring,
        "the run comes up the river, not across the fields"
    );
}

/// Nothing on land is worth what a fish is worth to a field.
#[test]
fn a_fish_is_worth_many_times_a_turnip() {
    assert!(
        Soil::waste_from_eating("fish") > Soil::waste_from_eating("grain") * 10.0,
        "a crop meal returns what the ground already paid out; a fish meal is \
         something the ground never had"
    );

    assert!(
        Soil::waste_from_spoilage("fish") > Soil::waste_from_eating("fish"),
        "a fish nobody got to still has in it everything a body would have kept"
    );

    assert!(Soil::came_out_of_the_water("cooked_fish"));
    assert!(Soil::came_out_of_the_water("salmon"));
    assert!(!Soil::came_out_of_the_water("grain"));
    assert!(!Soil::came_out_of_the_water("berries"));
}

/// Eating a fish loads a body with more to pass than eating anything else.
#[test]
fn what_a_fish_leaves_reaches_the_ground() {
    let mut on_fish = Agent::new(AgentConfig::default());
    let mut on_grain = Agent::new(AgentConfig::default());

    on_fish
        .inventory
        .add_item(InventoryItem::new("fishportions".to_string(), 4));
    on_grain
        .inventory
        .add_item(InventoryItem::new("grain".to_string(), 4));

    let _ = on_fish.eat_food_item("fishportions", 100);
    let _ = on_grain.eat_food_item("grain", 100);

    assert!(
        on_fish.state.waste_carried > on_grain.state.waste_carried,
        "a meal of fish leaves more behind than a meal of grain: {} against {}",
        on_fish.state.waste_carried,
        on_grain.state.waste_carried
    );
}

/// An agent standing at the water takes fish out of it - and takes far more
/// out of it with something in its hands.
///
/// "Fishing can be accomplished by hand but is highly inefficient. Spear
/// fishing is more efficient, pole fishing is better than spear fishing, and
/// net fishing is even better." So the count is not the thing to assert; the
/// ladder is. Bare hands landed ten casts in forty here, which is what "highly
/// inefficient" comes to.
#[test]
fn an_agent_at_the_water_catches_something() {
    // Counted in fish rather than in casts: good tackle lands more often *and*
    // takes more at a time, and the odds of a cast are capped, so casts alone
    // understate what a net is for.
    let fish_taken = |with: Option<&str>| {
        // Every rung off the same seed, or the four are not comparable: this
        // draws from wherever the stream happens to be, and building the world
        // consumes a different number of draws whenever anything about a world
        // changes. Measured unseeded on one such change: hands 25, spear 52,
        // rod 75, net 0 - a net that landed nothing in sixty casts, which is
        // not a statement about nets. See ISSUES_FOUND.md #132.
        crate::core::dice::seed(4_100);

        let mut world = a_world_with_a_river();
        world.resources.retain(|r| r.resource_type != ResourceType::Fish);
        world
            .resources
            .push(ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 400));

        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        population.agents[0].state.position = (2, 1, 0);
        if let Some(what) = with {
            let tool = population.agents[0].a_tool_fresh_from_these_hands(what, 1, 1.0);
            population.agents[0].inventory.add_item(tool);
        }

        let mut simulation = Simulation::new(world, population);
        let before = simulation.world.resources.iter()
            .filter(|r| r.resource_type == ResourceType::Fish)
            .map(|r| r.amount)
            .sum::<u32>();
        for _ in 0..60 {
            simulation.population.agents[0].state.position = (2, 1, 0);
            simulation.execute_action(&Action::Fish, 0);
        }
        let after = simulation.world.resources.iter()
            .filter(|r| r.resource_type == ResourceType::Fish)
            .map(|r| r.amount)
            .sum::<u32>();
        before.saturating_sub(after)
    };

    let by_hand = fish_taken(None);
    let with_a_spear = fish_taken(Some("spear"));
    let with_a_rod = fish_taken(Some("fishingrod"));
    let with_a_net = fish_taken(Some("fishingnet"));

    assert!(
        by_hand > 0,
        "fishing by hand is poor work, not impossible: {by_hand}"
    );
    assert!(
        with_a_spear > by_hand,
        "a spear should beat bare hands: {with_a_spear} against {by_hand}"
    );
    assert!(
        with_a_net >= with_a_rod && with_a_rod >= with_a_spear,
        "the ladder should climb: hands {by_hand}, spear {with_a_spear}, \
         rod {with_a_rod}, net {with_a_net}"
    );
    assert!(
        with_a_net > by_hand * 2,
        "a net should be worth several times a pair of hands: \
         {with_a_net} against {by_hand}"
    );
}

/// And what is caught goes into the pack.
#[test]
fn what_is_caught_goes_into_the_pack() {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);
    world
        .resources
        .push(ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60));

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (2, 1, 0);

    let mut simulation = Simulation::new(world, population);
    for _ in 0..40 {
        simulation.population.agents[0].state.position = (2, 1, 0);
        simulation.execute_action(&Action::Fish, 0);
    }

    let carried = simulation.population.agents[0]
        .inventory
        .get_item("fish")
        .map(|item| item.quantity)
        .unwrap_or(0);

    assert!(carried > 0, "and the fish should be in the pack");

    assert!(
        simulation.population.agents[0].state.waste_carried > 0.0,
        "the guts go into the pack as waste at the waterside"
    );

    let left = simulation
        .world
        .resources
        .iter()
        .find(|r| r.resource_type == ResourceType::Fish)
        .map(|r| r.amount)
        .unwrap_or(0);

    assert!(left < 60, "and they came out of the river");
}

/// Fishing an empty reach teaches an agent that fishing does not pay.
#[test]
fn standing_in_an_empty_river_teaches_something() {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);

    let mut reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);
    reach.amount = 0;
    world.resources.push(reach);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (2, 1, 0);

    let mut simulation = Simulation::new(world, population);

    for _ in 0..30 {
        simulation.population.agents[0].state.position = (2, 1, 0);
        let result = simulation.execute_action(&Action::Fish, 0);
        simulation.population.agents[0].learn_from(&Action::Fish, result.success);
    }

    assert!(
        !simulation.population.agents[0]
            .lessons
            .worth_trying(Undertaking::Fishing),
        "somebody who has stood in an empty river thirty times stops going"
    );
}

/// A settlement beside a river holds its fields where one inland does not.
///
/// The slow one. It runs two worlds side by side for long enough that the
/// difference is the fishery rather than the weather: the same country, the
/// same people, and in one of them the fish have been taken out of the water.
#[test]
#[ignore = "slow: two worlds to fifteen thousand turns"]
fn a_river_settlement_keeps_its_ground() {
    fn farmed_fertility(simulation: &Simulation) -> f32 {
        let mut total = 0.0;
        let mut fields = 0;

        for resource in &simulation.world.resources {
            if !matches!(
                resource.resource_type,
                ResourceType::Food | ResourceType::Grain
            ) {
                continue;
            }
            total += simulation
                .world
                .grid
                .how_good_the_ground_is(&resource.position);
            fields += 1;
        }

        total / fields.max(1) as f32
    }

    let mut with_fish = 0.0;
    let mut without = 0.0;
    const WORLDS: usize = 3;

    for _ in 0..WORLDS {
        let world = World::new(WorldConfig::default());

        // The same country twice, but one of them has no fish in it
        let mut dry = world.clone();
        dry.resources
            .retain(|resource| resource.resource_type != ResourceType::Fish);

        for (world, total) in [(world, &mut with_fish), (dry, &mut without)] {
            let mut population = Population::new();
            for _ in 0..12 {
                population.spawn_agent(AgentConfig::default());
            }
            let mut simulation = Simulation::new(world, population);
            for _ in 0..15_000 {
                simulation.take_a_turn();
            }
            *total += farmed_fertility(&simulation);
        }
    }

    let with_fish = with_fish / WORLDS as f32;
    let without = without / WORLDS as f32;

    assert!(
        with_fish > without,
        "a settlement with a river to fish should hold its fields better than \
         the same settlement without one: {with_fish:.3} against {without:.3}"
    );
}
