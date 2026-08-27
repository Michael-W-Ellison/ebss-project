// src/analytics/tests/wondering_tests.rs
//! Tests for curiosity as a question with an answer that arrives later.
//!
//! Curiosity in this model was always the same shape: pick a working nobody
//! here has tried, do it, and get the answer back in the same turn. That is
//! right for "what does this lump of clay do if I press it" and wrong for most
//! of what a stone-age people has to find out, because most of it does not
//! answer for three days and does not answer where you are standing.
//!
//! There was one branch that reached for the later kind — putting food down to
//! see whether the sun keeps it — and it was **gated on the sky being clear**.
//! Which is to say the code already knew the answer and only let anybody run
//! the experiment on the days it comes out well. Finding out that meat left in
//! the rain is ruined is the same discovery as finding out that meat left in
//! the sun keeps, and a people that can only make the second has not found
//! anything out at all.

use crate::agents::wondering::{Watched, Wondering};
use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::{Action, WeatherType};
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 1.0);
    meal.food_data = Some(database.create_food_data(&of, 0).expect("that is food"));
    meal
}

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

// --------------------------------------------------------------------------
// Noticing that something happened
// --------------------------------------------------------------------------

/// A thing exactly as it was left is a real answer, not a failure to get one.
#[test]
fn nothing_happening_is_an_answer() {
    let it = a_meal(ItemType::Meat, "meatstrips", 6);
    let was = Watched::of(&it);
    let is = Watched::of(&it);

    assert_eq!(was.what_became_of_it(&is), None);
}

/// Meat that has gone off has gone off, and somebody coming back can see it.
#[test]
fn going_off_is_something_anybody_can_see() {
    let mut it = a_meal(ItemType::Meat, "meatportions", 6);
    let was = Watched::of(&it);

    if let Some(food) = it.food_data.as_mut() {
        food.freshness = 0.2;
    }
    let is = Watched::of(&it);

    let became = was
        .what_became_of_it(&is)
        .expect("four fifths of it is gone");
    assert!(!became.for_the_better, "{became:?}");
}

/// A thing the sun has dried is a different thing, and it is the sort of
/// change you would do again on purpose.
#[test]
fn drying_out_is_a_change_for_the_better() {
    let mut it = a_meal(ItemType::Fish, "fishstrips", 6);
    let was = Watched::of(&it);

    if let Some(food) = it.food_data.as_mut() {
        food.preparation = PreparationState::Dried;
    }
    let is = Watched::of(&it);

    let became = was.what_became_of_it(&is).expect("it is dried now");
    assert!(became.for_the_better, "{became:?}");
}

/// And a lump of clay that comes out of a fire is not called clay any more,
/// which is the whole of what firing teaches.
#[test]
fn a_thing_that_is_not_what_it_was_is_the_strongest_answer() {
    let clay = InventoryItem::new_with_weight("clay".to_string(), 4, 1.0);
    let fired = InventoryItem::new_with_weight("stoneware".to_string(), 4, 1.0);

    let became = Watched::of(&clay)
        .what_became_of_it(&Watched::of(&fired))
        .expect("it is not clay any more");
    assert!(became.for_the_better);
}

/// A day's ordinary ageing is not a discovery. It happens to everything
/// everywhere and teaches nobody anything about where they left it.
#[test]
fn the_ordinary_passing_of_time_is_not_a_discovery() {
    let mut it = a_meal(ItemType::Meat, "meatportions", 6);
    let was = Watched::of(&it);

    if let Some(food) = it.food_data.as_mut() {
        food.freshness -= Watched::ENOUGH_OF_A_CHANGE_TO_NOTICE / 2.0;
    }

    assert_eq!(was.what_became_of_it(&Watched::of(&it)), None);
}

// --------------------------------------------------------------------------
// Holding the question
// --------------------------------------------------------------------------

/// Somebody with a few fish to spare will leave one somewhere to see.
#[test]
fn somebody_with_food_to_spare_would_leave_some_out() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 6));

    assert_eq!(
        simulation.population.agents[0]
            .what_i_would_leave_out()
            .as_deref(),
        Some("fishstrips")
    );
}

/// Somebody down to their last is not curious about it. An experiment must not
/// cost the experimenter its dinner.
#[test]
fn nobody_experiments_with_their_last_meal() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 1));

    assert_eq!(
        simulation.population.agents[0].what_i_would_leave_out(),
        None
    );
}

/// A lump of flint left in a field is a lump of flint in a field a week later.
#[test]
fn nobody_wonders_what_becomes_of_a_stone() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 20, 1.0));

    assert_eq!(
        simulation.population.agents[0].what_i_would_leave_out(),
        None
    );
}

/// And nobody goes on asking a question they have answered enough times to
/// have a view about.
///
/// Enough times, not once. One answer is one afternoon, and the thing being
/// found out here is that it depends on the afternoon — which cannot be found
/// out at all from a single one.
#[test]
fn somebody_stops_asking_once_they_have_a_view() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 6));

    let agent = &mut simulation.population.agents[0];

    agent.lessons.record_particular("leave:fishstrips", false);
    assert_eq!(
        agent.what_i_would_leave_out().as_deref(),
        Some("fishstrips"),
        "one wet afternoon settles nothing"
    );

    for _ in 0..40 {
        agent.lessons.record_particular("leave:fishstrips", false);
    }

    assert_eq!(agent.what_i_would_leave_out(), None, "he knows now");
}

/// An experiment costs a portion, not the pack. Tipping everything on the
/// grass to see what happens to it cost a settlement an eighth of its people.
#[test]
fn asking_a_question_costs_one_and_not_the_lot() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 6));

    let result = simulation.execute_action(
        &Action::PutDown {
            what: "fishstrips".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("fishstrips"),
        5,
        "he left one out and kept his supper"
    );
}

/// Where somebody putting food down for any other reason puts it all down.
#[test]
fn putting_food_down_is_still_putting_food_down() {
    let mut simulation = one_person();
    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Fish, "fishstrips", 6));

    {
        let agent = &mut simulation.population.agents[0];
        for _ in 0..40 {
            agent.lessons.record_particular("leave:fishstrips", false);
        }
    }

    let result = simulation.execute_action(
        &Action::PutDown {
            what: "fishstrips".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("fishstrips"),
        0,
        "that is not an experiment, that is putting it down"
    );
}

/// Nobody holds more than a few questions open at once.
#[test]
fn a_head_holds_only_so_many_open_questions() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    for i in 0..20 {
        agent.now_i_wonder(Wondering {
            did: "leave".to_string(),
            what: format!("thing{i}"),
            where_it_is: Position::new(25, 25),
            since: 0,
            as_it_was: Watched {
                called: format!("thing{i}"),
                freshness: Some(1.0),
                preparation: None,
            },
            in_this: Vec::new(),
        });
    }

    assert!(agent.wonderings.len() <= 8, "{}", agent.wonderings.len());
    assert!(
        agent.am_i_wondering_about("leave", "thing19"),
        "the newest question is the one still open"
    );
}

// --------------------------------------------------------------------------
// Putting the question
// --------------------------------------------------------------------------

/// Putting a thing down opens the question, and remembers what the sky was
/// doing — which cannot be recovered later, because by the time anybody walks
/// back to look the rain has stopped.
#[test]
fn putting_something_down_opens_the_question() {
    use crate::agents::practices::Circumstance;

    let mut simulation = one_person();
    simulation.world.climate.weather.weather_type = WeatherType::HeavyRain;
    simulation.world.climate.weather.duration_remaining = u32::MAX;

    simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meatportions", 6));

    let result = simulation.execute_action(
        &Action::PutDown {
            what: "meatportions".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    let wondering = simulation.population.agents[0]
        .wonderings
        .first()
        .expect("that is a question");

    assert_eq!(wondering.what, "meatportions");
    assert_eq!(wondering.where_it_is, Position::new(25, 25));
    assert!(
        wondering.in_this.contains(&Circumstance::Raining),
        "what is being found out is what becomes of meat left out *in the rain*: {:?}",
        wondering.in_this
    );
}

/// Putting food by is not an experiment, so a second load of the same thing
/// does not open a second question.
#[test]
fn putting_the_same_thing_down_again_is_not_a_new_question() {
    let mut simulation = one_person();

    for _ in 0..2 {
        simulation.population.agents[0]
            .inventory
            .add_item(a_meal(ItemType::Meat, "meatportions", 6));
        let _ = simulation.execute_action(
            &Action::PutDown {
                what: "meatportions".to_string(),
            },
            0,
        );
    }

    assert_eq!(simulation.population.agents[0].wonderings.len(), 1);
}

// --------------------------------------------------------------------------
// Getting the answer
// --------------------------------------------------------------------------

/// Coming back to find it ruined is the lesson, and it goes down against the
/// weather it was left in rather than the weather it was found in.
#[test]
fn coming_back_to_ruined_meat_teaches_what_ruined_it() {
    use crate::agents::practices::Circumstance;

    let mut simulation = one_person();
    let here = Position::new(25, 25);

    let mut left = a_meal(ItemType::Meat, "meatportions", 6);
    let as_it_was = Watched::of(&left);

    // What is on the ground has turned.
    if let Some(food) = left.food_data.as_mut() {
        food.freshness = 0.1;
    }
    simulation.world.somebody_left_this(left, here, 0);

    simulation.population.agents[0].now_i_wonder(Wondering {
        did: "leave".to_string(),
        what: "meatportions".to_string(),
        where_it_is: here,
        since: 0,
        as_it_was,
        in_this: vec![Circumstance::Raining],
    });

    simulation.who_came_back_to_look();

    let agent = &simulation.population.agents[0];
    assert!(
        agent.wonderings.is_empty(),
        "that question is answered"
    );
    assert_eq!(
        agent.lessons.tried_this("leave:meatportions"),
        1,
        "and the answer is written down"
    );
    assert_eq!(
        agent
            .lessons
            .tried_this_here("leave:meatportions", Circumstance::Raining),
        1,
        "against the rain it was left in"
    );
}

/// Somebody on the other side of the valley finds nothing out. The answer is
/// where the thing is.
#[test]
fn nobody_gets_an_answer_they_did_not_walk_back_for() {
    let mut simulation = one_person();
    let there = Position::new(60, 60);

    let mut left = a_meal(ItemType::Meat, "meatportions", 6);
    let as_it_was = Watched::of(&left);
    if let Some(food) = left.food_data.as_mut() {
        food.freshness = 0.1;
    }
    simulation.world.somebody_left_this(left, there, 0);

    simulation.population.agents[0].now_i_wonder(Wondering {
        did: "leave".to_string(),
        what: "meatportions".to_string(),
        where_it_is: there,
        since: 0,
        as_it_was,
        in_this: Vec::new(),
    });

    simulation.who_came_back_to_look();

    assert_eq!(
        simulation.population.agents[0].wonderings.len(),
        1,
        "he is thirty-five paces away and the question is still open"
    );
}

/// A question nobody ever gets back to is given up on rather than carried for
/// life.
#[test]
fn a_question_nobody_gets_back_to_is_given_up_on() {
    let mut simulation = one_person();
    let there = Position::new(60, 60);

    simulation.population.agents[0].now_i_wonder(Wondering {
        did: "leave".to_string(),
        what: "meatportions".to_string(),
        where_it_is: there,
        since: 0,
        as_it_was: Watched {
            called: "meatportions".to_string(),
            freshness: Some(1.0),
            preparation: Some(PreparationState::Raw),
        },
        in_this: Vec::new(),
    });

    simulation.current_tick = Wondering::HOW_LONG_ANYBODY_WONDERS + 2;
    simulation.who_came_back_to_look();

    assert!(simulation.population.agents[0].wonderings.is_empty());
}

/// Coming back four days on to find it exactly as it was left teaches that
/// leaving that about comes to nothing — which is what stops a man doing it
/// every week for the rest of his life.
#[test]
fn finding_it_exactly_as_you_left_it_is_also_a_lesson() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    let left = a_meal(ItemType::Meat, "meatstrips", 6);
    let as_it_was = Watched::of(&left);
    simulation.world.somebody_left_this(left, here, 0);

    simulation.population.agents[0].now_i_wonder(Wondering {
        did: "leave".to_string(),
        what: "meatstrips".to_string(),
        where_it_is: here,
        since: 0,
        as_it_was,
        in_this: Vec::new(),
    });

    simulation.current_tick = Wondering::HOW_LONG_ANYBODY_WONDERS + 2;
    simulation.who_came_back_to_look();

    let agent = &simulation.population.agents[0];
    assert!(agent.wonderings.is_empty());
    assert_eq!(
        agent.lessons.tried_this("leave:meatstrips"),
        1,
        "nothing came of it, and that is worth knowing"
    );
}

/// Somebody who walked off with it leaves no answer behind, and no lesson
/// either: the experiment was interfered with, not concluded.
#[test]
fn somebody_walking_off_with_it_ends_the_question_and_teaches_nothing() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    simulation.population.agents[0].now_i_wonder(Wondering {
        did: "leave".to_string(),
        what: "meatportions".to_string(),
        where_it_is: here,
        since: 0,
        as_it_was: Watched {
            called: "meatportions".to_string(),
            freshness: Some(1.0),
            preparation: Some(PreparationState::Raw),
        },
        in_this: Vec::new(),
    });

    // Nothing on the ground: it has gone.
    simulation.who_came_back_to_look();

    let agent = &simulation.population.agents[0];
    assert!(agent.wonderings.is_empty());
    assert_eq!(agent.lessons.tried_this("leave:meatportions"), 0);
}

// --------------------------------------------------------------------------
// In the running world
// --------------------------------------------------------------------------

/// The point of all of it: what an agent finds out this way feeds the record
/// it decides on, so leaving meat out in the rain is a thing somebody comes
/// to think better of.
#[test]
fn what_is_found_out_this_way_changes_what_gets_tried() {
    use crate::agents::practices::Circumstance;

    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    for _ in 0..20 {
        agent.lessons.record_particular_here(
            "leave:meatportions",
            false,
            &[Circumstance::Raining],
        );
        agent.lessons.record_particular_here(
            "leave:meatportions",
            true,
            &[Circumstance::ClearSky],
        );
    }

    let in_the_rain = agent
        .lessons
        .how_likely_to_try_this_here("leave:meatportions", &[Circumstance::Raining]);
    let in_the_sun = agent
        .lessons
        .how_likely_to_try_this_here("leave:meatportions", &[Circumstance::ClearSky]);

    assert!(
        in_the_sun > in_the_rain,
        "{in_the_sun} in the sun against {in_the_rain} in the rain"
    );
}

/// And a settlement left to itself asks and answers these questions without
/// anybody arranging it.
#[test]
fn a_settlement_asks_and_answers_questions_on_its_own() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);

    for _ in 0..(crate::environment::seasons::TICKS_PER_DAY * 90) {
        simulation.tick();
        if !simulation.population.agents.iter().any(|a| a.state.is_alive) {
            break;
        }
    }

    let answered: u64 = simulation.what_anybody_found_out.values().sum();

    assert!(
        answered > 0,
        "three months of twelve people living and not one of them ever left \
         anything anywhere and came back to look at it"
    );
}
