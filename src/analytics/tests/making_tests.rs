// src/analytics/tests/making_tests.rs
//! Tests that a thing can be made out of a thing that was made.
//!
//! Before this the whole of a people's toolmaking was three logs into an axe,
//! because the live crafting table took its inputs as `ResourceType` - things
//! dug or picked out of the ground - and could not say that the thing you
//! made last is what you need for the thing you are making now.

use crate::agents::{Agent, AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::making::{
    self, HAND_AXE, KNAPPED_TIP, LASHING, LASHING_FROM_COTTON, SPEAR, STONE_KNIFE,
};
use crate::environment::Action;
use crate::world::{World, WorldConfig};

/// Put a named thing in a pack.
fn carrying(agent: &mut Agent, what: &str, how_many: u32) {
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.5));
}

/// Take everything out of a pack, by the only door there is.
fn empty_the_pack(agent: &mut Agent) {
    let everything: Vec<(String, u32)> = agent
        .inventory
        .get_all_items()
        .iter()
        .map(|(what, item)| (what.clone(), item.quantity))
        .collect();
    for (what, how_many) in everything {
        agent.inventory.remove_item(&what, how_many);
    }
}

fn holding_of(agent: &Agent) -> impl Fn(&str) -> u32 + '_ {
    move |what: &str| {
        agent
            .inventory
            .get_item(what)
            .map(|item| item.quantity)
            .unwrap_or(0)
    }
}

/// The tools a founder would set about getting, by name.
///
/// What a pair of hands wants is stated as the *work* it wants to be equipped
/// for, because the best tool for a job changes as a people finds things out.
/// For somebody who knows only what he was born knowing it comes to the three
/// stone-age tools.
fn what_a_founder_wants() -> Vec<&'static str> {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    Agent::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO
        .iter()
        .filter_map(|trade| {
            making::what_helps_with(*trade)
                .filter(|tool| founder.knows_how_to_make(tool.called))
                .max_by(|a, b| {
                    a.how_much_better
                        .partial_cmp(&b.how_much_better)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|tool| tool.called)
        })
        .collect()
}

/// A world with one agent in it, ready to be told to do something.
fn one_agent_world() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let world = World::new(WorldConfig::default());
    Simulation::new(world, population)
}

// --- the table itself -------------------------------------------------------

/// A spear is made of things, and one of those things is made of things.
#[test]
fn a_spear_is_made_of_something_that_was_itself_made() {
    let made_parts: Vec<&str> = SPEAR
        .needs
        .iter()
        .map(|(what, _)| *what)
        .filter(|what| making::is_made_not_found(what))
        .collect();

    assert!(
        made_parts.contains(&"knappedtip") && made_parts.contains(&"lashing"),
        "a spear should want a tip and a lashing, both of them made rather \
         than found; it wants {:?}",
        SPEAR.needs
    );
}

/// Every part of every step is either found on the ground or made by a step.
#[test]
fn nothing_in_the_chain_asks_for_a_thing_that_cannot_be_had() {
    // What the ground offers, by the names `Action::Gather` answers to.
    let out_of_the_ground = ["wood", "stone", "iron", "food", "water", "flax", "cotton",
                             "hides", "wool", "grain",
                             // And what does not come off a bush: flesh off a
                             // kill and a fish out of the river. Both are
                             // gathered by name once they are lying about,
                             // and this list had not caught up with hunting
                             // and fishing being wired in.
                             "meat", "fish",
                             // And what the ground gives that nobody had a
                             // word for until lately: clay off a riverbank,
                             // salt off a flat, and the thin stuff a hedgerow
                             // offers before anything has ripened.
                             "clay", "salt", "greens", "roots"];

    // And what comes off a thing that is worked down rather than assembled -
    // a flake off a core, leather off a hide, shavings off a stick
    let worked_out_of_something: Vec<&str> = making::EVERY_WORKING
        .iter()
        .map(|working| working.makes)
        .collect();

    for step in making::EVERY_STEP {
        for (needed, _) in step.needs {
            assert!(
                out_of_the_ground.contains(needed)
                    || making::is_made_not_found(needed)
                    || worked_out_of_something.contains(needed),
                "{} wants {needed}, which is neither gathered nor made nor worked",
                step.makes
            );
        }
    }

    // The same of the workings themselves: nothing is worked out of a thing
    // that does not exist
    for working in making::EVERY_WORKING {
        assert!(
            out_of_the_ground.contains(&working.to)
                || making::is_made_not_found(working.to)
                || worked_out_of_something.contains(&working.to),
            "{} works on {}, which is neither gathered nor made nor worked",
            working.verb,
            working.to
        );
    }
}

/// Cordage can be had from either fibrous thing that grows here.
#[test]
fn there_is_more_than_one_way_to_make_a_lashing() {
    let ways: Vec<_> = making::every_way_to_make("lashing").collect();
    assert_eq!(
        ways.len(),
        3,
        "flax, cotton, and flax that has been left in water"
    );
    assert!(ways.iter().any(|w| w.needs == LASHING.needs));
    assert!(ways.iter().any(|w| w.needs == LASHING_FROM_COTTON.needs));
    assert!(
        ways.iter()
            .any(|w| w.needs.iter().any(|(what, _)| *what == "rettedflax")),
        "and retting is what gets three times as much out of the same field"
    );
}

// --- working back from a thing wanted ---------------------------------------

/// A man with the makings of a spear is told to make a spear.
#[test]
fn a_man_with_everything_makes_the_thing_itself() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "wood", 4);
    carrying(agent, "knappedtip", 2);
    carrying(agent, "lashing", 2);

    let step = making::what_to_do_first("spear", &holding_of(agent))
        .expect("a spear should be makeable out of its own parts");
    assert_eq!(step.makes, "spear");
}

/// A man with flax and stone and no spear is told to twist cordage, not that
/// he cannot have a spear.
#[test]
fn a_man_short_of_a_part_is_told_to_make_the_part() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "wood", 1);
    carrying(agent, "flax", 4);

    let step = making::what_to_do_first("spear", &holding_of(agent))
        .expect("flax in the pack is a step towards a spear");
    assert_eq!(
        step.makes, "lashing",
        "the part of a spear he can do today is the cordage"
    );
}

/// The chain walks back more than one link.
#[test]
fn a_man_with_only_stone_is_sent_to_knap() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "wood", 1);
    carrying(agent, "stone", 4);

    let step = making::what_to_do_first("spear", &holding_of(agent))
        .expect("stone in the pack is a step towards a spear");
    assert_eq!(step.makes, "knappedtip");
}

/// A man with nothing is not stuck: he is short of something the ground has.
#[test]
fn a_man_with_nothing_is_told_what_to_go_and_get() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    empty_the_pack(agent);

    assert!(
        making::what_to_do_first("spear", &holding_of(agent)).is_none(),
        "there is no step to take with an empty pack"
    );

    let wanting = making::what_is_wanting("spear", &holding_of(agent))
        .expect("an empty pack is short of something findable");
    assert!(
        ["wood", "stone", "flax", "cotton"].contains(&wanting),
        "a spear wants wood, stone or fibre; it asked for {wanting}"
    );
}

/// Having twisted enough cordage, a man stops twisting cordage.
#[test]
fn a_man_with_plenty_of_a_part_stops_making_it() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "flax", 40);
    carrying(agent, "lashing", making::A_FEW_SPARE + 1);

    let step = making::what_to_do_first("spear", &holding_of(agent));
    assert!(
        step.map(|s| s.makes) != Some("lashing"),
        "a man standing in a flax meadow with rope to spare should not spend \
         his life making more rope"
    );
}

// --- what an agent decides --------------------------------------------------

/// A founder arrives carrying the same named things his people know how to
/// make, so what he wears through he can replace.
#[test]
fn a_founder_carries_what_his_people_can_make() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    for (what, _, _) in [("handaxe", 0, 0.0), ("stoneknife", 0, 0.0)] {
        assert!(
            founder.inventory.get_item(what).is_some(),
            "a founder should be carrying a {what}"
        );
        assert!(
            making::is_made_not_found(what),
            "and his people should know how to make another {what}"
        );
    }
}

/// A founder wants a spear, because he has not got one.
#[test]
fn a_founder_sets_about_getting_himself_a_spear() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    carrying(agent, "wood", 2);
    carrying(agent, "stone", 4);
    carrying(agent, "flax", 4);

    let what = agent
        .what_i_would_make()
        .expect("a man with wood, stone and flax has something to be getting on with");
    assert!(
        ["spear", "knappedtip", "lashing"].contains(&what.as_str()),
        "he should be working towards a spear, not making {what}"
    );
}

/// A man who already has all three tools has nothing he needs to make.
#[test]
fn a_man_with_his_tools_about_him_asks_for_nothing() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    for want in what_a_founder_wants() {
        carrying(agent, want, 1);
    }
    carrying(agent, "wood", 9);
    carrying(agent, "stone", 9);
    carrying(agent, "flax", 9);

    assert_eq!(agent.what_i_would_make(), None);
    assert_eq!(agent.what_i_must_find(), None);
}

// --- doing it ---------------------------------------------------------------

/// Making a lashing costs flax and leaves a lashing.
#[test]
fn making_a_thing_spends_the_makings_and_leaves_the_thing() {
    let mut simulation = one_agent_world();
    let agent = &mut simulation.population.agents[0];
    empty_the_pack(agent);
    carrying(agent, "flax", 2);

    let result = simulation.execute_action(&Action::Craft { item_type: "lashing".to_string() }, 0);
    assert!(result.success, "{:?}", result.message);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        agent.inventory.count_item("flax"),
        0,
        "the flax should be gone"
    );
    assert_eq!(
        agent.inventory.count_item("lashing"),
        LASHING.how_many,
        "and cordage should be there instead"
    );
}

/// A tip and a lashing and a stick make a spear, in that order, one turn each.
#[test]
fn a_spear_can_be_made_out_of_what_was_made_before_it() {
    let mut simulation = one_agent_world();
    let agent = &mut simulation.population.agents[0];
    empty_the_pack(agent);
    carrying(agent, "wood", 1);
    carrying(agent, "stone", 2);
    carrying(agent, "flax", 2);

    // Whatever he decides to do, he should end up holding a spear.
    for tick in 1..=6u32 {
        let Some(what) = simulation.population.agents[0].what_i_would_make() else {
            break;
        };
        let result = simulation.execute_action(&Action::Craft { item_type: what.clone() }, 0);
        assert!(result.success, "making {what} failed: {:?}", result.message);
    }

    let agent = &simulation.population.agents[0];
    assert_eq!(
        agent.inventory.count_item("spear"),
        1,
        "three turns of work on the right things should be a spear; he is \
         holding {:?}",
        agent.inventory.get_all_items().keys().collect::<Vec<_>>()
    );
}

/// Being short of a part is a failure that names the part.
#[test]
fn a_man_short_of_a_part_is_told_which_part() {
    let mut simulation = one_agent_world();
    let agent = &mut simulation.population.agents[0];
    empty_the_pack(agent);
    carrying(agent, "wood", 1);

    let result = simulation.execute_action(&Action::Craft { item_type: "spear".to_string() }, 0);
    assert!(!result.success);
    assert!(
        result
            .message
            .as_deref()
            .is_some_and(|said| said.contains("knappedtip") || said.contains("lashing")),
        "the failure should name what is missing, not just say no: {:?}",
        result.message
    );
}

/// Making something teaches the hand that made it.
#[test]
fn making_a_thing_teaches_the_hand_that_made_it() {
    let mut simulation = one_agent_world();
    let agent = &mut simulation.population.agents[0];
    empty_the_pack(agent);
    carrying(agent, "stone", 2);
    let before = agent.skills.get_skill_mut(SkillType::Crafting).experience;

    let result = simulation.execute_action(&Action::Craft { item_type: "knappedtip".to_string() }, 0);
    assert!(result.success, "{:?}", result.message);

    let after = simulation.population.agents[0]
        .skills
        .get_skill_mut(SkillType::Crafting)
        .experience;
    assert!(after > before, "knapping should teach knapping");
}

/// The three tools a stone-age people carry are all reachable from the ground.
#[test]
fn every_tool_a_people_wants_can_be_got_from_raw_material() {
    for want in what_a_founder_wants() {
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        let agent = &mut population.agents[0];
        empty_the_pack(agent);
        carrying(agent, "wood", 4);
        carrying(agent, "stone", 8);
        carrying(agent, "flax", 8);

        let mut got_there = false;
        for _ in 0..8 {
            let step = {
                let holding = holding_of(&population.agents[0]);
                match making::what_to_do_first(want, &holding) {
                    Some(step) => *step,
                    None => break,
                }
            };
            let agent = &mut population.agents[0];
            for (needed, how_many) in step.needs {
                agent.inventory.remove_item(needed, *how_many);
            }
            carrying(agent, step.makes, step.how_many);
            if step.makes == want {
                got_there = true;
                break;
            }
        }

        assert!(got_there, "{want} should be reachable from wood, stone and flax");
    }
}

/// The stone age is what a people arrives knowing; anything past it is not.
#[test]
fn a_stone_age_people_arrives_knowing_the_stone_age() {
    for step in [KNAPPED_TIP, SPEAR, HAND_AXE, STONE_KNIFE, LASHING] {
        assert!(
            step.obvious,
            "{} is what a stone-age people brought with them",
            step.makes
        );
    }
    assert!(
        making::everything_to_find_out().count() > 0,
        "and there should be something left for them to find out"
    );
    for step in [KNAPPED_TIP, SPEAR, HAND_AXE, STONE_KNIFE, LASHING] {
        assert!(step.effort > 0.0, "{} should cost something to do", step.makes);
    }
}
