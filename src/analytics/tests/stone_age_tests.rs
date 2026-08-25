// src/analytics/tests/stone_age_tests.rs
//! Tests that a settlement can do the things its drives ask for.
//!
//! Measured before this: about three fifths of every action a settlement took
//! came to nothing. Store failed 100% of the time on a placeholder item
//! string; Craft 99.3% because the one recipe agents reach for needs a skill
//! that only crafting can teach; Build 100% because the cheapest shelter in
//! the game needs thirty stone that founders have no way to quarry and were
//! never carrying.

use crate::agents::practices::{Lessons, Undertaking};
use crate::agents::{Agent, AgentConfig, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{BuildingType, ResourceType, World, WorldConfig};

/// A founder arrives able to do the things a grown person can do.
#[test]
fn a_founder_arrives_with_the_hands_of_a_grown_person() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    for trade in [
        SkillType::Herbalism,
        SkillType::Cooking,
        SkillType::Crafting,
        SkillType::Construction,
        SkillType::Leatherworking,
    ] {
        let hand = founder
            .skills
            .get_skill_if_exists(trade)
            .map(|s| s.level)
            .unwrap_or(-10);
        assert!(
            hand > -10,
            "{trade:?} should not be at the floor for somebody who has lived \
             a life already"
        );
    }
}

/// And still has almost everything to learn.
#[test]
fn and_still_has_almost_everything_to_learn() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    for skill in founder.skills.get_all_skills().values() {
        assert!(
            skill.level < 0,
            "a founder is nearer the bottom of the climb than the top, and \
             {:?} stood at {}",
            skill.skill_type,
            skill.level
        );
    }
}

/// They carry tools and not a stockpile.
#[test]
fn they_carry_tools_and_not_a_stockpile() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let founder = &population.agents[0];

    // The name changed with the making chain: what they carry is now the
    // same named thing their own hands can turn out again.
    assert!(
        founder.inventory.get_item("handaxe").is_some(),
        "a stone-age people have stone tools"
    );

    // Twenty-five people who can all raise a tent on the first tick all try
    // to, crowd the same ground, and spend their lives looking for somewhere
    // to put one. Measured, it cost three quarters of the settlement.
    let tent = BuildingType::SkinTent.requirements();
    let could_build_at_once = tent.iter().all(|needed| {
        let name = format!("{:?}", needed.resource_type).to_lowercase();
        founder
            .inventory
            .get_item(&name)
            .map(|item| item.quantity)
            .unwrap_or(0)
            >= needed.amount
    });
    assert!(
        !could_build_at_once,
        "they should have to gather what a tent takes, not arrive with it"
    );
}

/// The first shelter is one a people with no quarry can actually raise.
#[test]
fn the_first_shelter_needs_no_stone() {
    let tent = BuildingType::SkinTent.requirements();
    assert!(
        !tent
            .iter()
            .any(|needed| needed.resource_type == ResourceType::Stone),
        "a skin tent is hides over poles"
    );
    assert!(!tent.is_empty(), "and it still costs something");
}

/// Building is not attempted with an empty pack.
#[test]
fn nobody_sets_about_building_with_nothing_to_build_from() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].inventory = Default::default();

    let here = simulation.population.agents[0].state.position;
    let answer = simulation.raising_a_roof(&simulation.population.agents[0], here);

    match answer {
        Some(Action::Gather { .. }) => {}
        other => panic!(
            "an agent with nothing should go and get something, not try to \
             build: {other:?}"
        ),
    }
}

/// And is attempted with a full one.
#[test]
fn a_man_with_the_hides_and_poles_puts_a_tent_up() {
    use crate::agents::InventoryItem;

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        for needed in BuildingType::SkinTent.requirements() {
            let name = format!("{:?}", needed.resource_type).to_lowercase();
            agent.inventory.add_item(InventoryItem::new_with_weight(
                name,
                needed.amount * 2,
                1.0,
            ));
        }
    }

    let here = simulation.population.agents[0].state.position;
    let answer = simulation.raising_a_roof(&simulation.population.agents[0], here);

    assert!(
        matches!(answer, Some(Action::Build { .. })),
        "a man with the hides and the poles puts the tent up: {answer:?}"
    );
}

/// Putting something by names a thing the agent actually has.
#[test]
fn putting_something_by_names_a_real_thing() {
    use crate::agents::InventoryItem;

    let mut agent = Agent::new(AgentConfig::default());
    agent.inventory = Default::default();
    assert_eq!(
        agent.what_i_can_spare(),
        None,
        "a man with nothing spare has nothing to put by"
    );

    agent.inventory.add_item(InventoryItem::new_with_weight(
        "wood".to_string(),
        30,
        1.0,
    ));
    let (what, how_many) = agent
        .what_i_can_spare()
        .expect("thirty logs is more than anybody carries about");
    assert_eq!(what, "wood");
    assert!(how_many > 0 && how_many < 30, "he keeps some to hand");
}

/// An action that keeps failing gets tried less.
#[test]
fn what_keeps_failing_gets_tried_less() {
    let mut lessons = Lessons::new();
    let hopeful = lessons.how_likely_to_try_this("gather:water");

    for _ in 0..30 {
        lessons.record_particular("gather:water", false);
    }

    assert!(
        lessons.how_likely_to_try_this("gather:water") < hopeful,
        "thirty failures should tell"
    );
    assert!(
        lessons.how_likely_to_try_this("gather:water") >= Lessons::NEVER_QUITE_GIVES_UP,
        "but a man never quite gives up, or he never finds out the world has \
         changed"
    );
}

/// And one that works gets tried more.
#[test]
fn what_works_gets_tried_more() {
    let mut lessons = Lessons::new();

    for _ in 0..30 {
        lessons.record_particular("gather:food", true);
    }
    for _ in 0..30 {
        lessons.record_particular("gather:stone", false);
    }

    assert!(
        lessons.how_likely_to_try_this("gather:food")
            > lessons.how_likely_to_try_this("gather:stone"),
        "what pays should be reached for before what does not"
    );
}

/// Failing at one thing does not put an agent off everything of that kind.
#[test]
fn failing_at_water_does_not_stop_a_man_looking_for_food() {
    let mut lessons = Lessons::new();
    for _ in 0..30 {
        lessons.record_particular("gather:water", false);
    }

    assert_eq!(
        lessons.how_likely_to_try_this("gather:food"),
        Lessons::NEVER_QUITE_CERTAIN,
        "a dried-up river teaches nothing about berries; the coarse record \
         answers what sort of man this is and cannot be what he decides on"
    );
}

/// A few goes at something is not a verdict.
#[test]
fn a_few_goes_at_something_is_not_a_verdict() {
    let mut lessons = Lessons::new();
    for _ in 0..4 {
        lessons.record_particular("cook", false);
    }

    assert_eq!(
        lessons.how_likely_to_try_this("cook"),
        Lessons::NEVER_QUITE_CERTAIN,
        "the first few goes at anything are spent getting into position"
    );
}

/// The coarse record still answers its own question.
#[test]
fn the_coarse_record_still_says_what_sort_of_man_this_is() {
    let mut lessons = Lessons::new();
    for _ in 0..20 {
        lessons.record(Undertaking::Hunting, true);
        lessons.record(Undertaking::Farming, false);
    }
    assert_eq!(lessons.what_works_best(), Some(Undertaking::Hunting));
}

/// A settlement stops doing what does not work.
#[test]
fn a_settlement_stops_doing_what_does_not_work() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..25 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    for _ in 0..6000 {
        simulation.tick();
    }

    let taken: u64 = simulation.actions_taken.values().sum();
    let failed: u64 = simulation.actions_failed.values().sum();
    let futile = failed as f64 / taken.max(1) as f64;

    assert!(
        futile < 0.35,
        "a settlement that learns should not spend most of its life on things \
         that do not work; {:.1}% of everything failed",
        futile * 100.0
    );
}
