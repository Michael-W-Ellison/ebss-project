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

    // Whatever the world put on this ground goes with it: springs sit on
    // shores now, and the shore this river is laid over may have had one
    // (#315)
    world
        .resources
        .retain(|r| !(r.position.x < 6 && r.position.y < 3));

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

/// One person standing on the bank of this file's river, beside a reach
/// holding this many fish.
fn somebody_at_a_reach_holding(fish: u32) -> Simulation {
    let mut world = a_world_with_a_river();
    world.resources.retain(|r| r.resource_type != ResourceType::Fish);
    let mut reach = ResourceNode::new(ResourceType::Fish, Position::new(2, 0), 60);
    reach.amount = fish;
    world.resources.push(reach);

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (2, 1, 0);
    Simulation::new(world, population)
}

/// Picking over the shallows: Stage 0 of the fishing path, and steady.
///
/// Somebody with nothing in their hands, at a reach that is nearly fished
/// out, gets more off its edge by turning over stones than by standing in it
/// waiting to thrust - and is sent to do that.
#[test]
fn the_shallows_feed_somebody_with_nothing_in_their_hands() {
    let thin = 4;
    let picked = |doing: &Action| {
        crate::core::dice::seed(4_101);
        let mut simulation = somebody_at_a_reach_holding(thin);
        for _ in 0..40 {
            simulation.population.agents[0].state.position = (2, 1, 0);
            simulation.execute_action(doing, 0);
        }
        thin - what_is_in_the_reach(&simulation.world)
    };
    let by_picking = picked(&Action::ForageShallows);
    let by_thrusting = picked(&Action::Fish);

    assert!(by_picking > 0, "the edge of a thin reach still gives something");
    assert!(
        by_picking >= by_thrusting,
        "and gives a bare hand more than a thrust does: {by_picking} picked against {by_thrusting} speared"
    );

    let simulation = somebody_at_a_reach_holding(thin);
    assert_eq!(
        simulation.how_to_work_this_water(&simulation.population.agents[0], Position::new(2, 0)),
        Action::ForageShallows,
        "somebody with no tackle at a thin reach picks over the shallows"
    );

    let mut simulation = somebody_at_a_reach_holding(60);
    let net = simulation.population.agents[0].a_tool_fresh_from_these_hands("fishingnet", 1, 1.0);
    simulation.population.agents[0].inventory.add_item(net);
    assert_eq!(
        simulation.how_to_work_this_water(&simulation.population.agents[0], Position::new(2, 0)),
        Action::Fish,
        "and somebody with a net at a full reach casts it"
    );
}

/// A trap set in a reach fills from the reach while nobody watches it, and
/// the one who set it empties it.
#[test]
fn a_fish_trap_fills_by_itself_and_is_emptied() {
    use crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
    use crate::world::FishTrap;

    crate::core::dice::seed(4_102);
    let mut simulation = somebody_at_a_reach_holding(60);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("fishtrap".to_string(), 1));

    let set = simulation.execute_action(&Action::SetFishTrap, 0);
    assert!(set.success, "a trap in the pack at a reach can be set: {:?}", set.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("fishtrap"), 0, "the trap left the pack");
    assert_eq!(simulation.world.fish_traps.len(), 1);

    for _ in 0..2 * PLANNING_PERIODS_PER_DAY {
        simulation.world.take_a_turn();
    }
    let holds = simulation.world.fish_traps[0].holds;
    assert!(
        holds > 0 && holds <= FishTrap::WHAT_A_TRAP_HOLDS,
        "two days in a full reach puts fish in a trap, and no more than it holds: {holds}"
    );

    let emptied = simulation.execute_action(&Action::CheckFishTraps, 0);
    assert!(emptied.success, "{:?}", emptied.message);
    assert_eq!(simulation.world.fish_traps[0].holds, 0, "the trap is empty again");
    assert!(
        simulation.population.agents[0].how_many_i_have("fish") >= holds,
        "and what was in it is in the pack"
    );
}

/// What sends somebody to the traps: fish in one of their own, near enough.
/// And somebody who knows how to make a trap and has the makings, standing at
/// an untrapped reach, makes one there and then sets it.
#[test]
fn a_hungry_trapper_goes_to_the_traps_and_sets_more() {
    use crate::core::drives::DriveType;
    use crate::world::FishTrap;

    let mut simulation = somebody_at_a_reach_holding(60);
    let agent = &mut simulation.population.agents[0];
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 1.0;
    }
    agent.found_out_how_to("fishtrap");
    agent.inventory.add_item(InventoryItem::new("wood".to_string(), 2));
    agent.inventory.add_item(InventoryItem::new("lashing".to_string(), 2));
    assert!(agent.knows_how_to_do("SetFishTrap"), "setting a trap comes with knowing how to make one");

    let at = (2, 1, 0);
    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.fishing_action(agent, at),
        Some(Action::Craft { item_type: "fishtrap".to_string() }),
        "the makings and the knowing, at an untrapped reach: make a trap"
    );

    let id = agent.id;
    simulation.world.fish_traps.push(FishTrap {
        at: Position::new(5, 0),
        set_by: id,
        set_at: 0,
        holds: 3,
        robbed: None,
    });
    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.fishing_action(agent, at),
        Some(Action::CheckFishTraps),
        "fish in a trap of his own within reach come first"
    );
    assert_eq!(
        simulation.fishing_action(agent, (12, 1, 0)),
        Some(Action::Move { target: (5, 0, 0) }),
        "and one a short walk off is walked to"
    );
}

/// A trap is made and carried to the water in a spare turn, as a snare line
/// is set: somebody who knows how, wanting to put food by, makes one where
/// they stand, walks it to a reach nobody has trapped, and sets it there.
#[test]
fn a_trap_is_made_carried_to_the_water_and_set() {
    use crate::core::drives::DriveType;

    let mut simulation = somebody_at_a_reach_holding(60);
    let agent = &mut simulation.population.agents[0];
    if let Some(putting_by) = agent.drives.get_mut(DriveType::Preparedness) {
        putting_by.value = 1.0;
    }
    agent.found_out_how_to("fishtrap");
    agent.inventory.add_item(InventoryItem::new("wood".to_string(), 2));
    agent.inventory.add_item(InventoryItem::new("lashing".to_string(), 2));

    let away = (8, 4, 0);
    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.putting_traps_in_the_water(agent, away),
        Some(Action::Craft { item_type: "fishtrap".to_string() }),
        "the makings and the knowing, with water in walking distance: make a trap"
    );

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("fishtrap".to_string(), 1));
    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.putting_traps_in_the_water(agent, away),
        Some(Action::Move { target: (2, 0, 0) }),
        "a trap in the pack is walked to the water"
    );
    assert_eq!(
        simulation.putting_traps_in_the_water(agent, (2, 1, 0)),
        Some(Action::SetFishTrap),
        "and set in it"
    );
}

/// An owner and somebody else at this file's river: the owner's trap holds
/// four fish, the owner is far off, and the other stands at the trap wanting
/// food as much as `hunger` says.
fn a_full_trap_and_somebody_at_it(hunger: f32) -> Simulation {
    use crate::core::drives::DriveType;
    use crate::world::FishTrap;

    let mut simulation = somebody_at_a_reach_holding(60);
    simulation.population.spawn_agent(AgentConfig::default());
    let owner = simulation.population.agents[1].id;
    simulation.population.agents[1].state.position = (40, 40, 0);
    simulation.world.fish_traps.push(FishTrap {
        at: Position::new(2, 0),
        set_by: owner,
        set_at: 0,
        holds: 4,
        robbed: None,
    });
    if let Some(drive) = simulation.population.agents[0].drives.get_mut(DriveType::Hunger) {
        drive.value = hunger;
    }
    simulation
}

/// Trap robbing: a hungry person who owes the owner nothing empties the
/// trap, and the owner, who was not there, finds out on the next round and
/// holds it against them.
#[test]
fn a_robbed_trap_is_found_out_on_the_round() {
    let mut simulation = a_full_trap_and_somebody_at_it(1.0);
    let (thief, owner) = (simulation.population.agents[0].id, simulation.population.agents[1].id);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.a_trap_worth_robbing(agent, (2, 1, 0)),
        Some(owner),
        "a starving stranger at a full trap takes what is in it"
    );

    let robbed = simulation.execute_action(&Action::RobFishTrap { whose: owner }, 0);
    assert!(robbed.success, "{:?}", robbed.message);
    assert_eq!(simulation.population.agents[0].how_many_i_have("fish"), 4);
    assert_eq!(simulation.world.fish_traps[0].holds, 0);
    assert_eq!(simulation.world.fish_traps[0].robbed, Some((thief, 4)), "the trap remembers who");

    simulation.population.agents[1].state.position = (2, 1, 0);
    let before = simulation.population.agents[1]
        .relationships
        .get_relationship(&thief)
        .map_or(0.0, |bond| bond.bond_strength);
    simulation.execute_action(&Action::CheckFishTraps, 1);
    let after = simulation.population.agents[1]
        .relationships
        .get_relationship(&thief)
        .map_or(0.0, |bond| bond.bond_strength);
    assert!(
        after < before && after < 0.0,
        "the owner comes round, finds it emptied, and thinks the less of the thief: {before} -> {after}"
    );
    assert_eq!(simulation.world.fish_traps[0].robbed, None, "and it is found out once");
}

/// Nobody robs a partner's trap, and a fed friend leaves one alone.
#[test]
fn a_partner_or_a_fed_friend_leaves_a_trap_alone() {
    let mut simulation = a_full_trap_and_somebody_at_it(1.0);
    let owner = simulation.population.agents[1].id;
    simulation.population.agents[0].pair_with(owner, 0);
    let agent = &simulation.population.agents[0];
    assert_eq!(simulation.a_trap_worth_robbing(agent, (2, 1, 0)), None, "a partner's fish are supper");

    let mut simulation = a_full_trap_and_somebody_at_it(0.2);
    let owner = simulation.population.agents[1].id;
    simulation.population.agents[0]
        .relationships
        .get_or_create_relationship(owner, 0)
        .bond_strength = 0.8;
    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.a_trap_worth_robbing(agent, (2, 1, 0)),
        None,
        "somebody barely hungry does not rob a friend"
    );
}
