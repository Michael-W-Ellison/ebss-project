// src/analytics/tests/survival_pressure_tests.rs
//! Tests for a settlement that has to reckon with what it is doing to itself.
//!
//! Thirty thousand ticks of tracing showed a settlement that overshoots does
//! not correct - it slides. Four things were missing, and all four are here:
//! ground that carries less as it is worked out, a need that presses harder
//! the longer it is denied, breeding that waits for a surplus rather than for
//! a full stomach, and somewhere else to go when the ground has stopped
//! giving. Children, who have no reserves to speak of, now feel a famine
//! before the adults around them do.

use crate::agents::practices::{Lessons, Undertaking};
use crate::agents::{Agent, AgentConfig, InventoryItem, LifeStage, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, Position, ResourceNode, ResourceType, World, WorldConfig};

fn fed_adult() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    // Years, not ticks. This said 4,000 - ticks, from the calendar where a
    // year was about eleven hundred of them. A year is 4,320 now, so "a fed
    // adult" was a body in its first year, and anything here that asked
    // whether a grown person would do something was asking it of an infant.
    // The same correction as in `a_hungry_year_takes_the_children_first`
    // below, which found it first.
    agent.state.now_this_many_years_old(30);
    agent
}

fn give_food(agent: &mut Agent, quantity: u32) {
    let database = FoodDatabase::new();
    let mut item = InventoryItem::new_with_weight("food".to_string(), quantity, 0.1);
    item.food_data = database.create_food_data(&ItemType::Food, 0);
    agent.inventory.add_item(item);
}

/// Ground worked out carries a smaller crop, not merely a slower one.
#[test]
fn the_crop_falls_with_the_ground() {
    let field = ResourceNode::new(ResourceType::Grain, Position::new(5, 5), 80);

    let fresh = field.standing_capacity(0.55);
    let tired = field.standing_capacity(0.25);
    let spent = field.standing_capacity(0.03);

    assert!(fresh > tired && tired > spent, "{fresh} {tired} {spent}");
    assert!(
        spent < fresh / 4,
        "ground worked from 0.55 to 0.03 should lose most of its yield: {fresh} to {spent}"
    );
}

/// A person who has been hungry for days acts on it more single-mindedly than
/// one who missed a meal.
#[test]
fn hunger_that_is_ignored_takes_an_agent_over() {
    let mut patient = fed_adult();
    let mut desperate = fed_adult();

    for agent in [&mut patient, &mut desperate] {
        if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.8;
        }
    }

    // One of them is left hungry for ten days of world time
    for _ in 0..120 {
        if let Some(hunger) = desperate.drives.get_mut(DriveType::Hunger) {
            hunger.tick();
            hunger.value = 0.8;
        }
    }

    let patient_hunger = patient.drives.get(DriveType::Hunger).unwrap();
    let desperate_hunger = desperate.drives.get(DriveType::Hunger).unwrap();

    assert!(
        desperate_hunger.urgency() > patient_hunger.urgency() * 2.0,
        "ten days of going without should make a far louder case: {:.2} against {:.2}",
        desperate_hunger.urgency(),
        patient_hunger.urgency()
    );
}

/// Nobody has a child on the strength of one good meal.
#[test]
fn a_child_waits_on_a_surplus_and_not_on_a_full_stomach() {
    let mut just_eaten = fed_adult();
    let mut provided_for = fed_adult();

    // What it takes: the pair's eating for as long as the land gives nothing.
    //
    // This was twelve items in a pack, and it passed because the gate had an
    // `|| a full belly` in it and the pack was never the binding question.
    // Twelve items is one day for one grown body. It is also, as it happens,
    // most of what a pack will hold - the pack takes twelve weight, which is
    // twenty-four items of food and about two days' eating - so a surplus
    // worth breeding on could never have been a thing somebody was *carrying*
    // in the first place. It is the camp's stores, which is what
    // `what_the_larder_says` reckons and what this now reads.
    let gap = crate::agents::provision::how_long_the_land_gives_nothing() as f32;
    let a_day = provided_for.state.physiology.what_i_burn_in_a_day;
    let for_two = a_day * (1.0 + crate::agents::agent::what_a_body_this_age_eats(0));
    let winter = 90.0;

    let stocked = crate::agents::provision::WhatIsPutBy::reckon(for_two * gap, a_day, winter, 0);
    provided_for.state.what_the_larder_says = Some(stocked);

    assert!(
        !just_eaten.should_attempt_reproduction(),
        "a full stomach and an empty pack is not a plan"
    );
    assert!(
        provided_for.should_attempt_reproduction(),
        "put by for the two of them through the gap is"
    );

    // And the same amount, if all of it is inside the agent rather than put
    // by, is not. A meal already swallowed cannot feed anybody next season.
    let mut just_a_big_supper = fed_adult();
    just_a_big_supper.state.what_the_larder_says = Some(
        crate::agents::provision::WhatIsPutBy::reckon(for_two * gap, a_day, winter, 0)
            .of_which_in_the_body(for_two * gap),
    );
    assert!(
        !just_a_big_supper.should_attempt_reproduction(),
        "what is in the stomach is not what is put by"
    );

    // And somebody who has been going short recently does not, however much
    // is in the ground now
    just_eaten.state.what_the_larder_says =
        Some(crate::agents::provision::WhatIsPutBy::reckon(for_two * gap, a_day, winter, 0));
    if let Some(hunger) = just_eaten.drives.get_mut(DriveType::Hunger) {
        hunger.value = 0.9;
        for _ in 0..40 {
            hunger.tick();
            hunger.value = 0.9;
        }
        hunger.value = 0.1;
    }

    assert!(
        !just_eaten.should_attempt_reproduction(),
        "a stretch of going short should still be telling"
    );
}

/// A famine takes the young before it takes the grown.
#[test]
fn a_hungry_year_takes_the_children_first() {
    /// How many days of famine this body lasts before its health is gone.
    ///
    /// **The time to death, not the health at a chosen moment.** This read
    /// health after a fixed span twice over and got nought both times, twice
    /// for the same reason: at two thousand ticks and again at three weeks
    /// both bodies are already dead, so the comparison could not come out
    /// either way whatever the model did. Picking a horizon at which the
    /// answer is visible is picking the answer; asking when each one goes is
    /// the question the test's own title asks.
    fn days_of_famine_survived(years: u32) -> f32 {
        let mut agent = Agent::new(AgentConfig::default());
        // Years. This took *ticks*, and passed 900 and 4000 for "a child" and
        // "an adult" - figures from the calendar where a year was about eleven
        // hundred ticks. A year is 4,320 now, so both fixtures were nought
        // years old and the test was comparing an infant with an infant.
        agent.state.now_this_many_years_old(years);
        agent.state.health = 100.0;
        agent.state.energy = 100.0;
        agent.state.last_ate_tick = 0;

        let a_long_time = 60 * crate::environment::seasons::TICKS_PER_DAY;
        for tick in 1..=a_long_time {
            // Watered, so that what kills this body is the famine.
            //
            // It was not, and both bodies died on day six of **thirst** -
            // which does not scale with body size, so the answer was 6.0
            // against 6.0 and the test could not see a famine at all. A
            // fixture that means to ask one question has to hold the other
            // clocks off; this is the third place in the suite that has been
            // caught not doing it.
            agent.state.physiology.hydration = 1.0;
            agent.state.last_drank_tick = tick;
            agent.state.ticks_without_water = 0;

            agent.state.age_tick_with_modifier(tick, 1.0);
            if agent.state.health <= 0.0 {
                return tick as f32 / crate::environment::seasons::TICKS_PER_DAY as f32;
            }
        }

        f32::INFINITY
    }

    let child = days_of_famine_survived(8);
    let adult = days_of_famine_survived(30);

    assert!(
        adult.is_finite(),
        "a grown body should starve to death inside two months of eating nothing"
    );
    assert!(
        child < adult,
        "a child should go first in a famine: the child lasted {child:.1} days \
         against the grown body's {adult:.1}"
    );
    assert_eq!(
        LifeStage::from_years(8),
        LifeStage::Child,
        "the fixture should be testing a child"
    );
    assert!(
        LifeStage::Child.hunger_reserve() < LifeStage::Adult.hunger_reserve(),
        "a small body has less put by than a grown one"
    );
}

/// Left hungry long enough, an agent gives up on the country it is in.
#[test]
fn a_starving_agent_walks_out_of_country_that_will_not_feed_it() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.age = 4000;
    simulation.population.agents[0].update_life_stage();
    simulation.population.agents[0].state.position = (25, 25, 0);

    let here = simulation.population.agents[0].state.position;

    // Hungry, but only just: nobody abandons a settlement over one missed meal
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 0.9;
    }

    let agent = &simulation.population.agents[0];
    assert!(
        simulation.migration_action(agent, here).is_none(),
        "one hungry afternoon is not a reason to leave"
    );

    // Ten days of being hungry and not being fed
    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        for _ in 0..130 {
            hunger.tick();
            hunger.value = 0.9;
        }
    }

    let agent = &simulation.population.agents[0];
    let leaving = simulation
        .migration_action(agent, here)
        .expect("ten days of going hungry should send somebody looking elsewhere");

    match leaving {
        crate::environment::Action::Move { target } => {
            let distance = (target.0 - here.0).abs().max((target.1 - here.1).abs());
            assert!(
                distance >= 15,
                "leaving means going somewhere else, not next door: {here:?} to {target:?}"
            );
        }
        other => panic!("expected to be walking somewhere, got {other:?}"),
    }
}

/// Somebody who is being fed does not wander off.
#[test]
fn a_fed_agent_stays_where_it_is() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    let here = simulation.population.agents[0].state.position;

    let agent = &simulation.population.agents[0];
    assert!(
        simulation.migration_action(agent, here).is_none(),
        "a fed agent has no reason to go anywhere"
    );
}

/// An agent stops doing what has never once worked, and keeps at what does.
#[test]
fn an_agent_gives_up_on_what_never_works() {
    let mut unlucky = Lessons::new();
    let mut capable = Lessons::new();

    for _ in 0..12 {
        unlucky.record(Undertaking::Hunting, false);
        capable.record(Undertaking::Hunting, true);
    }

    assert!(
        !unlucky.worth_trying(Undertaking::Hunting),
        "twelve empty-handed hunts should be enough to stop"
    );
    assert!(
        capable.worth_trying(Undertaking::Hunting),
        "twelve kills should not be"
    );
    assert!(capable.belief(Undertaking::Hunting) > unlucky.belief(Undertaking::Hunting));
}

/// Nothing is written off before it has been tried.
#[test]
fn nothing_is_given_up_on_before_it_is_tried() {
    let lessons = Lessons::new();

    for undertaking in [
        Undertaking::Hunting,
        Undertaking::Cooking,
        Undertaking::Farming,
        Undertaking::Clothing,
    ] {
        assert!(
            lessons.worth_trying(undertaking),
            "{undertaking:?} should get the benefit of the doubt"
        );
    }

    // One bad result is not a pattern either
    let mut once_burnt = Lessons::new();
    once_burnt.record(Undertaking::Cooking, false);
    assert!(once_burnt.worth_trying(Undertaking::Cooking));
}

/// The agent can say what has served it best, without anybody having told it.
#[test]
fn an_agent_can_say_what_has_served_it_best() {
    let mut lessons = Lessons::new();

    for _ in 0..8 {
        lessons.record(Undertaking::Farming, true);
        lessons.record(Undertaking::Hunting, false);
        lessons.record(Undertaking::Cooking, true);
        lessons.record(Undertaking::Cooking, false);
    }

    assert_eq!(
        lessons.what_works_best(),
        Some(Undertaking::Farming),
        "the best record should be the one it names"
    );
}

/// Doing things in a running simulation actually writes to that record.
#[test]
fn what_agents_do_in_a_run_becomes_something_they_know() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..8 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..2000 {
        simulation.tick();
    }

    let anybody_learned_anything = simulation.population.agents.iter().any(|agent| {
        [
            Undertaking::Hunting,
            Undertaking::Cooking,
            Undertaking::Farming,
            Undertaking::Clothing,
            Undertaking::Foraging,
            Undertaking::Building,
            Undertaking::Crafting,
            Undertaking::Dealing,
        ]
        .iter()
        .any(|undertaking| agent.lessons.attempts(*undertaking) > 0)
    });

    assert!(
        anybody_learned_anything,
        "two thousand ticks of doing things should leave a record of having done them"
    );
}

/// A newborn arrives having just been fed and watered.
///
/// Both survival clocks are kept as a tick the agent last ate or drank on, and
/// both start at zero. For the twelve people a world begins with that is
/// right; for anybody born later it meant arriving having last drunk at the
/// beginning of the world. An infant born after about four thousand ticks was
/// two days past the point where dehydration takes health, lost 1.65 a tick
/// from its first breath, and was dead at sixty-one - which is what a
/// settlement's entire second generation was quietly doing, at full health,
/// beside its mother, being nursed.
#[test]
fn a_newborn_is_not_born_parched() {
    let born_at = 9_000;
    let mother = crate::core::dice::name();

    let mut baby = Agent::with_parents(AgentConfig::default(), vec![mother], born_at);

    assert_eq!(baby.state.ticks_without_water, 0);
    assert_eq!(baby.state.ticks_without_food, 0);
    assert!(!baby.state.is_dehydrated(), "a newborn has just been born, not marooned");

    // And the clocks run from birth rather than from the beginning of time
    baby.state.age_tick_with_modifier(born_at + 50, 1.0);

    assert_eq!(baby.state.ticks_without_water, 50);
    assert!(
        baby.state.health > 99.0,
        "fifty ticks old and it should be in perfect health, not {:.1}",
        baby.state.health
    );
}

/// A settlement's second generation survives its first hour.
#[test]
fn the_children_of_a_settlement_live_past_infancy() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    // Five years, not ten. This asked for twelve thousand ticks when a
    // settlement of that age held under a hundred people; it now holds getting
    // on for twice that, and the cost of a tick rises with the square of who
    // is standing about, so the same claim was taking the best part of an hour
    // to check in a debug build. Six thousand ticks is four full years and is
    // long enough over: a settlement that has not raised a child in four years
    // is not going to.
    for _ in 0..6_000 {
        simulation.tick();
    }

    let born_here = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .filter(|agent| !agent.parent_ids.is_empty())
        .count();

    assert!(
        born_here >= 5,
        "six thousand ticks in, a settlement should hold people born into it, not {born_here}"
    );
}
