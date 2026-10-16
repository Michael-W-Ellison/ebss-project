// src/analytics/tests/practice_tests.rs
//! A hand at a thing comes of having made it (ISSUES_FOUND #335).
//!
//! "Crafting is a skill which should improve over time with experience. The
//! result should be lesser chance of injury, faster Crafting, and higher
//! quality output. Crafting should not be a general skill, but based on the
//! object being crafted." These cover:
//! - the hand at a thing climbs with the makings of that thing, not another
//! - the trade carries some of the way to a thing never made
//! - a practised hand is cut less, spoils less, is quicker and makes better
//! - founders arrive as practised at their kit as at their trade

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType, Skills};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn one_person_with_nothing() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.file_the_nodes();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.get_all_items_mut().clear();
    simulation
}

/// The first few makings teach the most, and a lifetime's teach the rest.
#[test]
fn the_hand_at_a_thing_climbs_with_the_makings_of_it() {
    assert_eq!(Skills::level_from_makings(0), -10, "never made it");
    assert!(Skills::level_from_makings(10) > Skills::level_from_makings(0));
    assert!(Skills::level_from_makings(30) > Skills::level_from_makings(10));
    assert!(Skills::level_from_makings(150) >= 9, "a lifetime at it is mastery");

    // And the first ten are worth more than the second ten
    let first = Skills::level_from_makings(10) - Skills::level_from_makings(0);
    let second = Skills::level_from_makings(20) - Skills::level_from_makings(10);
    assert!(first > second, "{first} then {second}");

    // The inverse lands where it says
    for level in [-8, -4, 0, 5] {
        assert!(
            Skills::level_from_makings(Skills::makings_for_level(level)) >= level,
            "{level}"
        );
    }
}

/// A hundred lashings make a man good at lashings, and only as good at a bow
/// as the trade carries him.
#[test]
fn making_one_thing_is_not_making_another() {
    let mut skills = Skills::new();
    for _ in 0..100 {
        skills.made_one("lashing");
    }

    let at_lashings = skills.level_at_making("lashing", SkillType::Crafting);
    let at_bows = skills.level_at_making("bow", SkillType::Crafting);
    assert!(at_lashings >= 8, "{at_lashings}");
    assert_eq!(at_bows, -10, "no trade, never made one");

    // With a trade behind him he starts a new thing a little under it
    skills.set_skill_level(SkillType::Crafting, 4);
    assert_eq!(
        skills.level_at_making("bow", SkillType::Crafting),
        4 - Skills::WHAT_A_NEW_THING_COSTS_THE_TRADE
    );
    // And what he has made himself is not dragged down to the trade
    assert!(skills.level_at_making("lashing", SkillType::Crafting) >= 8);
}

/// Lesser chance of injury, of spoiling the makings, and better work.
#[test]
fn a_practised_hand_is_cut_less_spoils_less_and_makes_better() {
    let mut skills = Skills::new();
    let new_to_it = skills.skill_at_making("spear", SkillType::Crafting);
    for _ in 0..90 {
        skills.made_one("spear");
    }
    let practised = skills.skill_at_making("spear", SkillType::Crafting);

    let (new, old) = (new_to_it.category(), practised.category());
    assert!(old.small_injury_chance() < new.small_injury_chance());
    assert!(old.large_injury_chance() < new.large_injury_chance());
    assert!(old.failure_chance() < new.failure_chance());
    assert!(practised.hand() > new_to_it.hand());
}

/// And what comes out of a practised hand is better and lasts longer.
#[test]
fn a_spear_from_a_practised_hand_is_the_better_spear() {
    let simulation = one_person_with_nothing();
    let mut new_hand = simulation.population.agents[0].clone();
    let mut old_hand = simulation.population.agents[0].clone();
    // The same tools in both pairs of hands, so only the hand differs
    for agent in [&mut new_hand, &mut old_hand] {
        agent.skills = Skills::new();
        agent.inventory.get_all_items_mut().clear();
    }
    for _ in 0..120 {
        old_hand.skills.made_one("spear");
    }

    let new_spear = new_hand.a_tool_fresh_from_these_hands("spear", 1, 1.0);
    let old_spear = old_hand.a_tool_fresh_from_these_hands("spear", 1, 1.0);
    assert!(
        old_spear.max_durability.unwrap_or(0.0) > new_spear.max_durability.unwrap_or(0.0),
        "{:?} against {:?}",
        old_spear.max_durability,
        new_spear.max_durability
    );
    assert!(old_spear.quality >= new_spear.quality);
}

/// A beginner is a while at a making that a practised hand gets through in a
/// period, and spends more of himself on it.
#[test]
fn a_practised_hand_is_quicker() {
    let mut simulation = one_person_with_nothing();
    // A handaxe is a making of ordinary size: a haft, a tip and a lashing,
    // put back in after every go so spoiled makings do not run him out.
    let try_until_one_comes = |simulation: &mut Simulation| {
        for _ in 0..40 {
            for what in ["wood", "knappedtip", "lashing"] {
                simulation.population.agents[0]
                    .inventory
                    .add_item(InventoryItem::new_with_weight(what.to_string(), 1, 1.0));
            }
            simulation.population.agents[0].busy_until = 0;
            let result = simulation
                .execute_action(&Action::Craft { item_type: "handaxe".to_string() }, 0);
            if result.success {
                return result;
            }
        }
        panic!("forty goes and never a handaxe");
    };

    simulation.population.agents[0].skills = Skills::new();
    let beginner = try_until_one_comes(&mut simulation);

    simulation.population.agents[0].skills = Skills::new();
    for _ in 0..150 {
        simulation.population.agents[0].skills.made_one("handaxe");
    }
    let master = try_until_one_comes(&mut simulation);

    assert_eq!(master.took_periods, 1, "a master does it inside the half hour");
    assert!(
        beginner.took_periods > master.took_periods,
        "a beginner takes {} periods",
        beginner.took_periods
    );
    assert!(
        master.energy_cost < beginner.energy_cost,
        "{} against {}",
        master.energy_cost,
        beginner.energy_cost
    );
}

/// A founder is as good at a spear as the day before he walked in.
#[test]
fn founders_arrive_practised_at_what_they_have_always_made() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &mut population.agents[0];
    founder.give_them_a_stone_age_start();

    let trade = founder
        .skills
        .get_skill_if_exists(SkillType::Crafting)
        .map(|skill| skill.level)
        .expect("a founder knows the crafting trade");

    assert!(founder.skills.times_made("spear") > 0);
    assert!(
        founder.skills.level_at_making("spear", SkillType::Crafting) >= trade,
        "a spear is a thing he has made"
    );
    // A bow is not, and he comes to it a little under the trade
    assert_eq!(founder.skills.times_made("bow"), 0);
    assert!(founder.skills.level_at_making("bow", SkillType::Crafting) < trade);
}

/// Somebody who has found out how to make a thing makes a few to see it
/// happen, and then stops: the novelty is spent. Before this a settlement made
/// hundreds of bows and wheels a year that nobody used.
#[test]
fn curiosity_makes_a_few_and_then_stops() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];
    agent.inventory.get_all_items_mut().clear();
    agent.found_out_how_to("bow");
    let knife = agent.a_tool_fresh_from_these_hands("stoneknife", 1, 0.5);
    agent.inventory.add_item(knife);
    agent.inventory.add_item(InventoryItem::new_with_weight("wood".to_string(), 2, 1.0));
    agent.inventory.add_item(InventoryItem::new_with_weight("lashing".to_string(), 2, 0.1));

    assert_eq!(agent.what_i_would_try_out(), Some("bow".to_string()));

    for _ in 0..crate::agents::Agent::MADE_OFTEN_ENOUGH_TO_HAVE_SEEN_IT {
        agent.skills.made_one("bow");
    }
    assert_eq!(agent.what_i_would_try_out(), None, "seen it made often enough");
}
