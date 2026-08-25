// src/analytics/tests/tool_wear_tests.rs
//! Tests that a tool is worth having and does not last.
//!
//! Before this a tool was a thing an agent counted. `Inventory` had carried
//! durability fields since the beginning and only clothing used them; a man
//! with a stone axe felled timber at exactly the rate of a man with his bare
//! hands, so there was no reason to make one and no cost to never making
//! another.

use crate::agents::skills::Quality;
use crate::agents::{Agent, AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::making::{self, AXE_FOR_WOOD, SPEAR_FOR_HUNTING};
use crate::world::{World, WorldConfig};

fn carrying(agent: &mut Agent, what: &str, how_many: u32) {
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.5));
}

fn one_agent_world() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let world = World::new(WorldConfig::default());
    Simulation::new(world, population)
}

/// Every tool in the table is a thing the chain knows how to make.
#[test]
fn every_tool_is_a_thing_these_people_can_make() {
    for tool in making::EVERY_TOOL {
        assert!(
            making::is_made_not_found(tool.called),
            "{} is a tool nobody can make",
            tool.called
        );
        assert!(
            tool.how_much_better > 1.0,
            "{} should be better than bare hands",
            tool.called
        );
        assert!(
            tool.how_long_it_lasts <= 50.0,
            "{} lasts {} pieces of work, which is not stone-age",
            tool.called,
            tool.how_long_it_lasts
        );
    }
}

/// A founder carries tools that have a life in them.
#[test]
fn a_founder_carries_tools_with_a_life_in_them() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    let axe = founder
        .inventory
        .get_item("handaxe")
        .expect("a founder carries an axe");
    assert!(
        axe.current_durability.unwrap_or(0.0) > 0.0,
        "and it is not already worn through"
    );
    assert_eq!(
        axe.durability_percentage(),
        1.0,
        "a tool he arrives with is a tool he has just made"
    );
}

/// A founder makes crude things, because he is not an expert at anything.
#[test]
fn a_founder_is_nobody_special_at_making_things() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    let made = founder.a_tool_fresh_from_these_hands("spear", 1, 2.0);
    let quality = made.quality.expect("a made tool has a quality");
    assert!(
        quality <= Quality::Basic,
        "a founder should turn out crude work, not {quality:?}"
    );
}

/// The same hands, better practised, turn out a better thing.
#[test]
fn a_practised_hand_makes_a_thing_that_lasts_longer() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let beginner = {
        let agent = &mut population.agents[0];
        agent.skills.set_skill_level(SkillType::Crafting, -8);
        agent.a_tool_fresh_from_these_hands("spear", 1, 2.0)
    };

    let practised = {
        let agent = &mut population.agents[0];
        agent.skills.set_skill_level(SkillType::Crafting, 8);
        agent.a_tool_fresh_from_these_hands("spear", 1, 2.0)
    };

    assert!(
        practised.max_durability.unwrap() > beginner.max_durability.unwrap(),
        "practice should tell on the thing made: {:?} against {:?}",
        practised.max_durability,
        beginner.max_durability
    );
    assert!(
        practised.quality.unwrap() > beginner.quality.unwrap(),
        "and on what it is worth"
    );
}

/// Having the tool is worth something; not having it is worth nothing.
#[test]
fn a_tool_in_the_pack_makes_the_work_go_better() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    agent.inventory.remove_item("handaxe", 1);

    assert_eq!(
        agent.how_much_my_tools_help(SkillType::Woodcutting),
        1.0,
        "bare hands are bare hands"
    );

    let axe = agent.a_tool_fresh_from_these_hands("handaxe", 1, 2.0);
    agent.inventory.add_item(axe);

    assert!(
        agent.how_much_my_tools_help(SkillType::Woodcutting) > 1.0,
        "an axe should make felling timber go faster"
    );
    assert!(
        agent.how_much_my_tools_help(SkillType::Woodcutting) <= AXE_FOR_WOOD.how_much_better,
        "but no faster than the tool is worth"
    );
}

/// A tool half worn through is worth less than a new one.
#[test]
fn a_worn_tool_is_worth_less_than_a_new_one() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    let fresh = agent.how_much_my_tools_help(SkillType::Woodcutting);

    let axe = agent.inventory.get_item_mut("handaxe").unwrap();
    let max = axe.max_durability.unwrap();
    axe.current_durability = Some(max * 0.1);

    let nearly_done = agent.how_much_my_tools_help(SkillType::Woodcutting);
    assert!(
        nearly_done < fresh,
        "a blunt axe should be worth less than a sharp one: {nearly_done} against {fresh}"
    );
    assert!(nearly_done > 1.0, "but still better than nothing");
}

/// Work wears the tool, and enough work finishes it.
#[test]
fn enough_work_wears_a_tool_out() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    let started_with = agent
        .inventory
        .get_item("handaxe")
        .unwrap()
        .current_durability
        .unwrap();

    agent.wear_what_i_worked_with(SkillType::Woodcutting);
    let after_one = agent
        .inventory
        .get_item("handaxe")
        .unwrap()
        .current_durability
        .unwrap();
    assert!(after_one < started_with, "one trip out should tell");

    let mut broke = None;
    for _ in 0..(AXE_FOR_WOOD.how_long_it_lasts as u32 * 3) {
        if let Some(what) = agent.wear_what_i_worked_with(SkillType::Woodcutting) {
            broke = Some(what);
            break;
        }
    }

    assert_eq!(
        broke.as_deref(),
        Some("handaxe"),
        "a stone axe should not outlast the man who made it"
    );
    assert_eq!(
        agent.how_much_my_tools_help(SkillType::Woodcutting),
        1.0,
        "and a worn-through axe is no axe"
    );
}

/// A worn-through tool is a reason to make another.
#[test]
fn a_broken_tool_is_a_reason_to_make_a_new_one() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "wood", 4);
    carrying(agent, "stone", 8);
    carrying(agent, "flax", 8);

    // Wear the axe out entirely.
    while agent
        .wear_what_i_worked_with(SkillType::Woodcutting)
        .is_none()
    {}

    assert_eq!(
        agent.how_many_i_have("handaxe"),
        0,
        "a broken axe is not an axe"
    );

    // He is holding a broken axe, so what he wants is a whole one. Put him in
    // a world and let the real crafting path work through the chain.
    let mut simulation = one_agent_world();
    simulation.population.agents[0] = population.agents.remove(0);

    let mut made_an_axe = false;
    for _ in 0..12 {
        let Some(what) = simulation.population.agents[0].what_i_would_make() else {
            break;
        };
        let result = simulation.execute_action(
            &crate::environment::Action::Craft { item_type: what.clone() },
            0,
        );
        assert!(result.success, "making {what} failed: {:?}", result.message);
        if what == "handaxe" {
            made_an_axe = true;
            break;
        }
    }

    assert!(
        made_an_axe,
        "a man with a broken axe and the makings of one should make one"
    );
    assert!(
        simulation.population.agents[0].how_much_my_tools_help(SkillType::Woodcutting) > 1.0,
        "and be back in business"
    );
}

/// The tool tells on the work the simulation actually does.
#[test]
fn gathering_with_an_axe_brings_back_more_than_gathering_without() {
    use crate::environment::Action;
    use crate::world::ResourceType;

    // Put a big stand of timber next to the agent, gather it a hundred times
    // with an axe and a hundred without, and compare what came back.
    fn timber_taken(with_an_axe: bool) -> u32 {
        let mut simulation = one_agent_world();
        let position = simulation.population.agents[0].state.position;

        {
            let agent = &mut simulation.population.agents[0];
            agent.inventory.remove_item("handaxe", 1);
            if with_an_axe {
                // Fresh every trip, so this measures the tool and not its wear
                let axe = agent.a_tool_fresh_from_these_hands("handaxe", 1, 2.0);
                agent.inventory.add_item(axe);
            }
        }

        let where_it_is = crate::world::Position::new(position.0, position.1);
        simulation.world.resources.push(crate::world::ResourceNode::new(
            ResourceType::Wood,
            where_it_is,
            100_000,
        ));

        let mut taken = 0;
        for _ in 0..200 {
            {
                let agent = &mut simulation.population.agents[0];
                let had = agent.inventory.count_item("wood");
                agent.inventory.remove_item("wood", had);
                if with_an_axe {
                    let axe = agent.inventory.get_item_mut("handaxe").unwrap();
                    axe.current_durability = axe.max_durability;
                }
            }
            simulation.execute_action(
                &Action::Gather { resource_type: "wood".to_string() },
                0,
            );
            taken += simulation.population.agents[0].inventory.count_item("wood");
        }
        taken
    }

    let with = timber_taken(true);
    let without = timber_taken(false);

    assert!(
        with > without,
        "two hundred trips with an axe brought back {with} and without {without}"
    );
}

/// A spear makes hunting worth trying.
#[test]
fn a_spear_makes_a_hunter_of_somebody() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    assert_eq!(
        agent.how_much_my_tools_help(SkillType::Hunting),
        1.0,
        "a founder arrives without a spear"
    );

    let spear = agent.a_tool_fresh_from_these_hands("spear", 1, 2.0);
    agent.inventory.add_item(spear);

    let helped = agent.how_much_my_tools_help(SkillType::Hunting);
    assert!(helped > 1.0, "a spear should count for something in a hunt");
    assert!(helped <= SPEAR_FOR_HUNTING.how_much_better);
}

/// The tenth spear a man makes is a better spear than his first.
#[test]
fn a_practised_hand_makes_a_tool_that_works_better() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    fn helped(agent: &mut Agent, hand: i32) -> f32 {
        agent.skills.set_skill_level(SkillType::Crafting, hand);
        agent.inventory.remove_item("spear", 1);
        let spear = agent.a_tool_fresh_from_these_hands("spear", 1, 2.0);
        agent.inventory.add_item(spear);
        agent.how_much_my_tools_help(SkillType::Hunting)
    }

    let agent = &mut population.agents[0];
    let first = helped(agent, -8);
    let tenth = helped(agent, 8);

    assert!(
        tenth > first,
        "practice should tell on how well the thing works: {tenth} against {first}"
    );
    assert!(
        tenth <= SPEAR_FOR_HUNTING.how_much_better * 1.5,
        "and not turn one man into three"
    );
}

/// A badly made tool is still better than no tool.
#[test]
fn even_crude_work_beats_bare_hands() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    agent.skills.set_skill_level(SkillType::Crafting, -10);

    let spear = agent.a_tool_fresh_from_these_hands("spear", 1, 2.0);
    assert_eq!(spear.quality, Some(Quality::Pathetic));
    agent.inventory.add_item(spear);

    assert!(
        agent.how_much_my_tools_help(SkillType::Hunting) > 1.0,
        "the worst spear anybody ever made is still a spear"
    );
}

/// Doing the thing over and over is what makes the thing better.
///
/// The whole loop, through the real crafting path: make, get better at making,
/// and find that what comes out of your hands has improved.
#[test]
fn making_the_same_thing_over_and_over_improves_it() {
    use crate::environment::Action;

    let mut simulation = one_agent_world();

    fn spear_from(simulation: &mut Simulation) -> (Option<crate::agents::skills::Quality>, f32) {
        {
            let agent = &mut simulation.population.agents[0];
            agent.inventory.remove_item("spear", 1);
            carrying(agent, "wood", 1);
            carrying(agent, "knappedtip", 1);
            carrying(agent, "lashing", 1);
        }
        let result = simulation.execute_action(
            &Action::Craft { item_type: "spear".to_string() },
            0,
        );
        assert!(result.success, "{:?}", result.message);
        let made = simulation.population.agents[0]
            .inventory
            .get_item("spear")
            .expect("he is holding the spear he just made");
        (made.quality, made.max_durability.unwrap_or(0.0))
    }

    let started_at = simulation.population.agents[0]
        .skills
        .get_skill_mut(SkillType::Crafting)
        .level;
    let (first_quality, first_life) = spear_from(&mut simulation);

    // Knap flakes until the hand has learned something. Two hundred is more
    // than it takes and bounds the loop.
    for _ in 0..200 {
        {
            let agent = &mut simulation.population.agents[0];
            agent.inventory.remove_item("knappedtip", 1);
            carrying(agent, "stone", 2);
        }
        simulation.execute_action(
            &Action::Craft { item_type: "knappedtip".to_string() },
            0,
        );
        if simulation.population.agents[0]
            .skills
            .get_skill_mut(SkillType::Crafting)
            .level
            > started_at
        {
            break;
        }
    }

    let ended_at = simulation.population.agents[0]
        .skills
        .get_skill_mut(SkillType::Crafting)
        .level;
    assert!(
        ended_at > started_at,
        "knapping two hundred flakes should teach a man something: still at {ended_at}"
    );

    let (later_quality, later_life) = spear_from(&mut simulation);
    assert!(
        later_life > first_life,
        "the spear made by the practised hand should outlast the first: \
         {later_life} against {first_life}"
    );
    assert!(
        later_quality >= first_quality,
        "and be no worse a spear: {later_quality:?} against {first_quality:?}"
    );
}
