// src/analytics/tests/specialisation_tests.rs
//! Tests that a trade is worth having, and that nobody has all of them.
//!
//! Experience was granted for *looking* rather than doing: the resource
//! discovery pass filtered on the tick a thing was found and ran every tick,
//! so a thing seen once paid out on ten consecutive ticks - fifty Farming
//! experience for walking past a grain field, in a settled world holding
//! ninety of them. A level cost a flat hundred at every level. Between them,
//! skill measured how much of the map somebody had wandered over: across
//! nearly three hundred agents, Farming stood at 9.9 out of 10 and
//! Leatherworking, which nothing could be discovered for, at -9.2. Nobody had
//! earned any of it, and none of it did anything, because `speed_multiplier`,
//! `perform_check` and `determine_quality` were built and had no callers.

use crate::agents::skills::{Quality, Skill, SkillType, Skills};
use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, TerrainType, World, WorldConfig};

/// Getting the hang of a thing is quick; getting good at it is not.
#[test]
fn the_last_step_of_a_trade_costs_more_than_the_first() {
    let first = Skill::experience_for_next_level(-10);
    let middle = Skill::experience_for_next_level(0);
    let last = Skill::experience_for_next_level(9);

    assert!(middle > first, "the middle of a trade is harder than the start");
    assert!(last > middle, "and the top is harder than the middle");
    assert!(
        last > first * 4,
        "the last step should cost several times the first: {last} against {first}"
    );
}

/// A life at one trade gets you to the top of it; the same effort split eight
/// ways gets you to the top of nothing.
#[test]
fn a_life_at_one_trade_beats_a_life_at_eight() {
    const A_WORKING_LIFE: u32 = 250 * 8; // goes at a trade, times what a go is worth

    let mut devoted = Skills::new();
    devoted.practise(SkillType::Farming, A_WORKING_LIFE, 1_000);

    let mut scattered = Skills::new();
    let trades = [
        SkillType::Farming,
        SkillType::Herbalism,
        SkillType::Cooking,
        SkillType::Fishing,
        SkillType::Hunting,
        SkillType::Woodcutting,
        SkillType::Mining,
        SkillType::Leatherworking,
    ];
    for trade in trades {
        scattered.practise(trade, A_WORKING_LIFE / trades.len() as u32, 1_000);
    }

    let master = devoted.get_skill_if_exists(SkillType::Farming).unwrap().level;
    let jack = scattered
        .get_skill_if_exists(SkillType::Farming)
        .unwrap()
        .level;

    assert!(
        master >= 6,
        "a life given to one trade should reach the top of it, not {master}"
    );
    assert!(
        jack < 0,
        "and the same life split eight ways should not, yet reached {jack}"
    );
    assert!(
        devoted.hand_for(SkillType::Farming) > scattered.hand_for(SkillType::Farming) * 1.5,
        "the specialist should be worth half again as much at the work"
    );
}

/// A hand that stops doing the work loses it.
#[test]
fn a_trade_not_practised_goes() {
    let mut skills = Skills::new();
    skills.practise(SkillType::Leatherworking, 3_000, 1_000);
    let at_its_height = skills
        .get_skill_if_exists(SkillType::Leatherworking)
        .unwrap()
        .level;
    assert!(at_its_height > 0, "the test needs somebody who was good");

    // A season away costs nothing
    skills.let_unused_skills_rust(1_000 + Skills::KEEPS_FOR / 2);
    assert_eq!(
        skills
            .get_skill_if_exists(SkillType::Leatherworking)
            .unwrap()
            .level,
        at_its_height,
        "a few months off does not lose somebody their trade"
    );

    // Years away costs a good deal
    let mut now = 1_000;
    for _ in 0..12 {
        now += Skills::KEEPS_FOR;
        skills.let_unused_skills_rust(now);
    }

    let after = skills
        .get_skill_if_exists(SkillType::Leatherworking)
        .unwrap()
        .level;

    assert!(
        after < at_its_height,
        "years away from a trade should cost something: still {after}"
    );
    assert!(
        after >= Skills::NEVER_QUITE_FORGOTTEN,
        "but nobody forgets a trade entirely; this one fell to {after}"
    );
}

/// And a hand kept in does not.
#[test]
fn a_trade_kept_up_does_not_go() {
    let mut skills = Skills::new();
    skills.practise(SkillType::Farming, 3_000, 0);
    let was = skills.get_skill_if_exists(SkillType::Farming).unwrap().level;

    let mut now = 0;
    for _ in 0..20 {
        now += Skills::KEEPS_FOR / 2;
        skills.practise(SkillType::Farming, 1, now);
        skills.let_unused_skills_rust(now);
    }

    assert!(
        skills.get_skill_if_exists(SkillType::Farming).unwrap().level >= was,
        "somebody who keeps at their trade keeps it"
    );
}

/// Seeing a field is not farming it.
#[test]
fn walking_past_a_field_does_not_make_a_farmer() {
    let mut world = World::new(WorldConfig::default());
    world
        .resources
        .retain(|r| r.resource_type != ResourceType::Grain);

    // A country thick with grain, and one agent walking about in it
    for x in 0..12 {
        for y in 0..12 {
            world
                .resources
                .push(ResourceNode::new(ResourceType::Grain, Position::new(x, y), 50));
        }
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (6, 6, 0);

    let mut simulation = Simulation::new(world, population);
    for _ in 0..600 {
        simulation.tick();
    }

    let farming = simulation.population.agents[0]
        .skills
        .get_skill_if_exists(SkillType::Farming)
        .map(|skill| skill.level)
        .unwrap_or(-10);

    assert!(
        farming < 0,
        "a hundred and forty fields in sight for six hundred ticks should not \
         make somebody a farmer; this one reached {farming}"
    );
}

/// A practised hand brings back more from the same ground.
#[test]
fn a_dedicated_farmer_brings_back_more_than_a_casual_one() {
    fn what_a_season_brings(level: i32) -> u32 {
        let mut world = World::new(WorldConfig::default());
        world.resources.clear();

        for x in 0..3 {
            for y in 0..3 {
                if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                    tile.terrain.terrain_type = TerrainType::Plains;
                }
            }
        }

        let mut node = ResourceNode::new(ResourceType::Food, Position::new(1, 1), 100_000);
        node.max_amount = 100_000;
        world.resources.push(node);

        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        population.agents[0].state.position = (1, 1, 0);
        population.agents[0]
            .skills
            .get_skill_mut(SkillType::Herbalism)
            .level = level;

        let mut simulation = Simulation::new(world, population);

        let before = simulation.world.resources[0].amount;
        for _ in 0..300 {
            simulation.population.agents[0].state.position = (1, 1, 0);
            simulation.population.agents[0]
                .skills
                .get_skill_mut(SkillType::Herbalism)
                .level = level;
            let _ = simulation.execute_action(
                &Action::Gather {
                    resource_type: "food".to_string(),
                },
                0,
            );
        }

        before - simulation.world.resources[0].amount
    }

    let casual = what_a_season_brings(-8);
    let dedicated = what_a_season_brings(8);

    assert!(
        dedicated > casual,
        "a practised hand should bring back more off the same ground: \
         {dedicated} against {casual}"
    );
    assert!(
        dedicated as f32 > casual as f32 * 1.5,
        "and meaningfully more, not a rounding error: {dedicated} against {casual}"
    );
}

/// A tailor who has made a hundred coats wastes fewer than one who has made
/// none, and makes better ones.
#[test]
fn a_dedicated_tailor_wastes_less_and_makes_better() {
    fn a_run_of_work(level: i32) -> (u32, Quality) {
        let world = World::new(WorldConfig::default());
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());

        let mut simulation = Simulation::new(world, population);

        let recipe = crate::agents::equipment::GARMENT_RECIPES
            .iter()
            .find(|recipe| recipe.material_item == "flax")
            .expect("there is something made of flax");

        let mut finished = 0;
        let mut why_not: Option<String> = None;
        for _ in 0..200 {
            let agent = &mut simulation.population.agents[0];
            agent.skills.get_skill_mut(SkillType::Leatherworking).level = level;

            // A fresh bench every time: nothing worn, nothing in the pack but
            // the material, so what varies between the two runs is the hand
            // and nothing else
            agent.inventory = crate::agents::Inventory::new(40, 400.0);
            agent.equipment = crate::agents::equipment::EquipmentManager::new(80.0);
            agent.inventory.add_item(InventoryItem::new(
                recipe.material_item.to_string(),
                recipe.material_amount,
            ));

            let made = simulation.execute_action(
                &Action::MakeClothing {
                    garment: recipe.id.to_string(),
                },
                0,
            );

            if made.success {
                finished += 1;
            } else if why_not.is_none() {
                why_not = made.message.clone();
            }
        }

        assert!(
            finished > 0,
            "the fixture should be able to make anything at all at level \
             {level}; first refusal was {why_not:?}"
        );

        let agent = &mut simulation.population.agents[0];
        agent.skills.get_skill_mut(SkillType::Leatherworking).level = level;
        let quality = Simulation::expected_garment_quality(agent);

        (finished, quality)
    }

    let (beginner_made, beginner_quality) = a_run_of_work(-9);
    let (master_made, master_quality) = a_run_of_work(9);

    assert!(
        master_made > beginner_made,
        "a master should finish more coats out of the same two hundred \
         attempts: {master_made} against {beginner_made}"
    );
    assert!(
        master_quality > beginner_quality,
        "and better ones: {master_quality:?} against {beginner_quality:?}"
    );
}

/// What a hand is worth runs the whole way from clumsy to twice as good.
#[test]
fn a_hand_is_worth_between_half_and_double() {
    let raw = Skill::new(SkillType::Farming);
    assert!((raw.hand() - 0.5).abs() < 0.01, "a beginner is half a hand");

    let master = Skill::with_level(SkillType::Farming, 10);
    assert!((master.hand() - 2.0).abs() < 0.01, "a master is two");

    let ordinary = Skill::with_level(SkillType::Farming, 0);
    assert!(
        ordinary.hand() > raw.hand() && ordinary.hand() < master.hand(),
        "and the middle is in the middle"
    );
}
