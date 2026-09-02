// src/analytics/tests/asking_tests.rs
//! Tests for the other verbs as questions, for asking somebody about a thing
//! you have never seen, and for counting what all of it is actually worth.
//!
//! Three things that belong together.
//!
//! Burying and salting are questions like leaving a thing out is a question,
//! and **the verb has to decide what counts as a good answer**. A thing left
//! on the grass that is exactly as it was left a week later teaches nothing:
//! nothing came of leaving it there. A thing *buried* that is exactly as it
//! was left a week later is the entire point of burying it. Getting that
//! backwards would have taught a settlement that its larder was useless.
//!
//! Nothing anywhere let a man who had worked something out *tell* anybody.
//! Everything in this model is found out first-hand or watched being done, so
//! a settlement of forty could work the same thing out forty times over and be
//! no further on than the first man who worked it out.
//!
//! And none of it could be judged, because nothing had ever counted the waste.
//! The point of preserving anything is that the time spent getting it was not
//! wasted: if half the meat rots before it is eaten then half the hunt was
//! wasted, and the hours are gone either way.

use crate::agents::wondering::{Kept, Watched, Wondering};
use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 1.0);
    meal.food_data = Some(database.create_food_data(&of, 0).expect("that is food"));
    meal
}

/// A making nobody is born knowing: grain between two stones. See
/// `making::CRUSH_GRAIN`, which is `obvious: false`, and so is the kind of
/// thing that is still worth asking a neighbour about.
const THE_MAKING: &str = "flour";

fn a_making() -> InventoryItem {
    InventoryItem::new_with_weight(THE_MAKING.to_string(), 3, 0.5)
}

fn a_settlement(how_many: usize) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    for who in 0..how_many {
        simulation.population.agents[who].state.position = (25, 25, 0);
        simulation.population.agents[who]
            .inventory
            .get_all_items_mut()
            .clear();
        simulation.population.agents[who].inventory.recalculate_weight();
    }
    simulation
}

fn wondering(did: &str, what: &str, was: Watched, where_it_is: Position) -> Wondering {
    Wondering {
        did: did.to_string(),
        what: what.to_string(),
        where_it_is,
        since: 0,
        as_it_was: was,
        in_this: Vec::new(),
    }
}

// --------------------------------------------------------------------------
// The verb decides what a good answer is
// --------------------------------------------------------------------------

/// A thing buried that has not changed is the whole point of burying it.
#[test]
fn buried_food_that_has_not_changed_is_the_answer_yes() {
    let it = a_meal(ItemType::Meat, "meatportions", 6);
    let question = wondering(
        Wondering::BURYING_IT,
        "meatportions",
        Watched::of(&it),
        Position::new(25, 25),
    );

    let became = question
        .what_it_means(&Watched::of(&it), true)
        .expect("a week in the ground and still good");
    assert!(became.for_the_better, "{became:?}");
}

/// Where a thing left on the grass that has not changed is nothing at all.
#[test]
fn food_left_on_the_grass_that_has_not_changed_is_the_answer_no() {
    let it = a_meal(ItemType::Meat, "meatstrips", 6);
    let question = wondering(
        "leave",
        "meatstrips",
        Watched::of(&it),
        Position::new(25, 25),
    );

    let became = question
        .what_it_means(&Watched::of(&it), true)
        .expect("a week on the grass");
    assert!(
        !became.for_the_better,
        "nothing came of leaving it there: {became:?}"
    );
}

/// Neither answer is given before there has been time for one.
#[test]
fn no_answer_is_given_before_there_has_been_time_for_one() {
    let it = a_meal(ItemType::Meat, "meatportions", 6);

    for did in [Wondering::BURYING_IT, Wondering::SALTING_IT, "leave"] {
        let question = wondering(did, "meatportions", Watched::of(&it), Position::new(25, 25));
        assert_eq!(
            question.what_it_means(&Watched::of(&it), false),
            None,
            "{did} answered itself on the first afternoon"
        );
    }
}

/// And food that went off in the hole is the answer no, whatever the verb.
#[test]
fn food_that_went_off_in_the_hole_is_the_answer_no() {
    let it = a_meal(ItemType::Meat, "meatportions", 6);
    let question = wondering(
        Wondering::BURYING_IT,
        "meatportions",
        Watched::of(&it),
        Position::new(25, 25),
    );

    let mut gone = it.clone();
    if let Some(food) = gone.food_data.as_mut() {
        food.freshness = 0.1;
    }

    let became = question
        .what_it_means(&Watched::of(&gone), true)
        .expect("it is ruined");
    assert!(!became.for_the_better, "{became:?}");
}

/// Each verb knows where to go and look. Burying puts a thing in a hole and
/// salting leaves it in the pack; only leaving it out puts it on the grass.
#[test]
fn each_verb_knows_where_to_look() {
    let it = a_meal(ItemType::Meat, "meatportions", 6);
    let here = Position::new(25, 25);

    assert_eq!(
        wondering("leave", "meatportions", Watched::of(&it), here).where_to_look(),
        Kept::OnTheGround
    );
    assert_eq!(
        wondering(Wondering::BURYING_IT, "meatportions", Watched::of(&it), here).where_to_look(),
        Kept::InThePit
    );
    assert_eq!(
        wondering(Wondering::SALTING_IT, "meatportions", Watched::of(&it), here).where_to_look(),
        Kept::InMyPack
    );
}

// --------------------------------------------------------------------------
// Putting those questions
// --------------------------------------------------------------------------

/// Burying something opens the question.
#[test]
fn burying_something_opens_the_question() {
    let mut simulation = a_settlement(1);
    let here = Position::new(25, 25);

    simulation.world.pits.push(crate::world::Pit {
        where_it_is: here,
        holds: Vec::new(),
        covered: false,
        dug: 0,
    });
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 8));

    let result = simulation.execute_action(
        &Action::Cover {
            what: "meatportions".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    assert!(
        simulation.population.agents[0]
            .am_i_wondering_about(Wondering::BURYING_IT, "meatportions"),
        "{:?}",
        simulation.population.agents[0].wonderings
    );
}

/// Salting something opens the question, and it stays in the pack where its
/// owner can see it — so this one never wants a walk back.
#[test]
fn salting_something_opens_a_question_that_travels_with_you() {
    let mut simulation = a_settlement(1);

    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 6));
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("salt".to_string(), 4, 0.1));

    let result = simulation.execute_action(
        &Action::Salt {
            what: "meatportions".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    let question = simulation.population.agents[0]
        .wonderings
        .iter()
        .find(|w| w.did == Wondering::SALTING_IT)
        .expect("that is a question");

    assert_eq!(question.where_to_look(), Kept::InMyPack);
    assert_eq!(
        question.as_it_was.preparation,
        Some(PreparationState::Salted),
        "what is being watched is the salted meat, not the meat it was"
    );
}

/// And salted meat that is still sound a week later is the answer yes.
#[test]
fn salted_meat_still_sound_a_week_later_is_the_answer_yes() {
    let mut simulation = a_settlement(1);

    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 6));
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("salt".to_string(), 4, 0.1));

    let _ = simulation.execute_action(
        &Action::Salt {
            what: "meatportions".to_string(),
        },
        0,
    );

    simulation.current_tick = Wondering::HOW_LONG_ANYBODY_WONDERS + 2;
    simulation.who_came_back_to_look();

    let agent = &simulation.population.agents[0];
    assert!(
        !agent.am_i_wondering_about(Wondering::SALTING_IT, "meatportions"),
        "that question is answered"
    );
    assert!(
        agent.lessons.how_likely_to_try_this("salt:meatportions") > 0.0,
        "and it is written down"
    );
}

// --------------------------------------------------------------------------
// Clay in the fire
// --------------------------------------------------------------------------

/// A lump of clay left at a lit fire is not a lump of clay in the morning.
#[test]
fn clay_left_at_a_fire_comes_out_hard() {
    let mut simulation = a_settlement(1);
    let here = (25, 25, 0);

    let fire = simulation
        .world
        .build_heat_source(crate::environment::HeatSourceType::Campfire, here, None)
        .expect("a fire can be built here");
    let _ = simulation
        .world
        .add_fuel_to_heat_source(&fire, "wood".to_string(), 400.0);
    let _ = simulation.world.light_heat_source(&fire);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_weight("clay".to_string(), 3, 1.0),
        Position::new(25, 25),
        0,
    );

    simulation.current_tick = crate::environment::seasons::TICKS_PER_DAY * 2;
    simulation.what_the_fire_hardened();

    let lying = simulation.world.what_is_lying_at(&Position::new(25, 25));
    assert!(
        lying.iter().any(|left| left.item.item_id == "stoneware"),
        "{:?}",
        lying.iter().map(|l| &l.item.item_id).collect::<Vec<_>>()
    );
    assert!(
        simulation.population.agents[0]
            .what_i_found_out()
            .contains(Simulation::THAT_FIRE_HARDENS_CLAY),
        "and whoever was sitting there saw it"
    );
}

/// Clay lying in a field, with no fire anywhere near, is clay.
#[test]
fn clay_in_a_cold_field_stays_clay() {
    let mut simulation = a_settlement(1);

    simulation.world.somebody_left_this(
        InventoryItem::new_with_weight("clay".to_string(), 3, 1.0),
        Position::new(40, 40),
        0,
    );

    simulation.current_tick = crate::environment::seasons::TICKS_PER_DAY * 2;
    simulation.what_the_fire_hardened();

    assert!(
        simulation
            .world
            .what_is_lying_at(&Position::new(40, 40))
            .iter()
            .all(|left| left.item.item_id == "clay")
    );
}

/// Clay is worth leaving somewhere to see, where a flint is not.
#[test]
fn clay_is_worth_watching_and_a_flint_is_not() {
    let mut simulation = a_settlement(1);
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 20, 1.0));

    assert_eq!(
        simulation.population.agents[0].what_i_would_leave_out(),
        None
    );

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("clay".to_string(), 6, 1.0));

    assert_eq!(
        simulation.population.agents[0]
            .what_i_would_leave_out()
            .as_deref(),
        Some("clay")
    );
}

// --------------------------------------------------------------------------
// Asking somebody
// --------------------------------------------------------------------------

/// A dried strip has nothing left to tell anybody.
///
/// It used to be worth asking about: laying food out to keep it had to be
/// watched before somebody would do it on purpose, and a neighbour eating
/// dried meat was one of the two ways to see it. Everybody is born knowing it
/// now - see `Agent::what_anybody_is_born_knowing` - so a meal teaches
/// nothing, and the branch that asked about one is gone. What is still worth
/// asking about is a *making* nobody has worked out. See ISSUES_FOUND.md #125.
#[test]
fn a_meal_is_no_longer_worth_asking_about() {
    let mut simulation = a_settlement(2);

    {
        let mut dried = a_meal(ItemType::Meat, "meatstrips", 6);
        if let Some(food) = dried.food_data.as_mut() {
            food.preparation = PreparationState::Dried;
        }
        simulation.population.agents[1].inventory.add_item(dried);
    }

    let here = simulation.population.agents[0].state.position;
    let asked = simulation
        .somebody_to_ask_about_something(&simulation.population.agents[0], here);

    assert!(
        !matches!(asked, Some((_, ref what)) if what == "meatstrips"),
        "there is nothing a dried strip can teach: {asked:?}"
    );
}

/// A man who has never crushed grain cannot tell you how.
#[test]
fn nobody_can_explain_a_thing_they_do_not_understand() {
    let mut simulation = a_settlement(2);

    simulation.population.agents[1].inventory.add_item(a_making());
    // and agent 1 has found nothing out at all

    let here = simulation.population.agents[0].state.position;
    assert_eq!(
        simulation
            .somebody_to_ask_about_something(&simulation.population.agents[0], here),
        None,
        "he is holding it and has no idea how it happened"
    );
}

/// Nobody asks after a stick.
#[test]
fn nobody_asks_after_a_stick() {
    let mut simulation = a_settlement(2);
    simulation.population.agents[1]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), 10, 1.0));

    let here = simulation.population.agents[0].state.position;
    assert_eq!(
        simulation
            .somebody_to_ask_about_something(&simulation.population.agents[0], here),
        None
    );
}

/// Nor after a thing they already know about.
#[test]
fn nobody_asks_after_a_thing_they_already_know() {
    let mut simulation = a_settlement(2);

    simulation.population.agents[1].inventory.add_item(a_making());
    simulation.population.agents[1].found_out_how_to(THE_MAKING);
    simulation.population.agents[0].found_out_how_to(THE_MAKING);

    let here = simulation.population.agents[0].state.position;
    assert_eq!(
        simulation
            .somebody_to_ask_about_something(&simulation.population.agents[0], here),
        None
    );
}

/// Nor somebody across the valley.
#[test]
fn nobody_shouts_across_a_valley() {
    let mut simulation = a_settlement(2);

    simulation.population.agents[1].inventory.add_item(a_making());
    simulation.population.agents[1].found_out_how_to(THE_MAKING);
    simulation.population.agents[1].state.position = (60, 60, 0);

    let here = simulation.population.agents[0].state.position;
    assert_eq!(
        simulation
            .somebody_to_ask_about_something(&simulation.population.agents[0], here),
        None
    );
}

/// Asking passes the discovery, and what passes is the *name* of it — so the
/// hearer can go and try it, not so that they believe it.
#[test]
fn being_told_lets_you_try_it_rather_than_making_you_believe_it() {
    let mut simulation = a_settlement(2);

    simulation.population.agents[1].inventory.add_item(a_making());
    simulation.population.agents[1].found_out_how_to(THE_MAKING);

    let them = simulation.population.agents[1].id;
    let before = simulation.population.agents[0].lessons.tried_this(THE_MAKING);

    let result = simulation.execute_action(
        &Action::AskAbout {
            who: them,
            what: THE_MAKING.to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    assert!(
        simulation.population.agents[0]
            .what_i_found_out()
            .contains(THE_MAKING),
        "he knows it can be done now"
    );
    assert_eq!(
        simulation.population.agents[0].lessons.tried_this(THE_MAKING),
        before,
        "and he has still never crushed a handful of grain, which is the \
         difference between being told a thing works and finding out"
    );
}

/// Asking somebody who cannot explain it is refused rather than quietly
/// teaching nothing.
#[test]
fn asking_somebody_who_cannot_explain_is_refused() {
    let mut simulation = a_settlement(2);
    simulation.population.agents[1]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), 10, 1.0));

    let them = simulation.population.agents[1].id;
    let result = simulation.execute_action(
        &Action::AskAbout {
            who: them,
            what: "wood".to_string(),
        },
        0,
    );

    assert!(!result.success, "{:?}", result.message);
}

/// And asking yourself teaches nothing.
#[test]
fn asking_yourself_teaches_nothing() {
    let mut simulation = a_settlement(1);
    let me = simulation.population.agents[0].id;

    let result = simulation.execute_action(
        &Action::AskAbout {
            who: me,
            what: THE_MAKING.to_string(),
        },
        0,
    );

    assert!(!result.success);
}

// --------------------------------------------------------------------------
// What the waste actually is
// --------------------------------------------------------------------------

/// Eating is counted.
#[test]
fn what_gets_eaten_is_counted() {
    let mut simulation = a_settlement(1);
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Food, "berries", 6));

    assert_eq!(simulation.population.agents[0].food_i_ate, 0);

    let _ = simulation.population.agents[0].eat_food_item("berries", 0);

    assert_eq!(simulation.population.agents[0].food_i_ate, 1);
}

/// And so is what goes off in somebody's own pack, which is the waste nobody
/// ever counted.
#[test]
fn what_goes_off_in_the_pack_is_counted() {
    let mut simulation = a_settlement(1);
    let agent = &mut simulation.population.agents[0];

    assert_eq!(agent.food_that_rotted_on_me, 0);
    agent.watched_food_go_off("meatportions", 4);
    assert_eq!(agent.food_that_rotted_on_me, 4);
}

/// Food that rots where it lies is counted against the hunt that got it.
#[test]
fn what_rots_on_the_ground_is_counted() {
    let mut simulation = a_settlement(1);

    let mut going = a_meal(ItemType::Meat, "meatportions", 5);
    if let Some(food) = going.food_data.as_mut() {
        food.freshness = 0.01;
        food.base_spoilage_ticks = 1;
    }
    simulation
        .world
        .somebody_left_this(going, Position::new(40, 40), 0);

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 3) {
        simulation.world.tick();
    }

    assert!(
        simulation.world.food_that_rotted_where_it_lay > 0,
        "five portions went off in a field and nothing counted it"
    );
}
