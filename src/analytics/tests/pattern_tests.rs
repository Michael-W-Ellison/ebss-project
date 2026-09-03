// src/analytics/tests/pattern_tests.rs
//! Tests that an agent joins what it did to the need that got answered.
//!
//! "When an agent satisfies drive demand, it links its previous actions taken
//! to the drive satisfaction to form a pattern. (e.g., travel to + specific
//! location = water)."

use crate::agents::patterns::{Bearing, Element, Patterns};
use crate::agents::{Agent, AgentConfig, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use crate::world::{ResourceNode, ResourceType, World, WorldConfig};

fn a_lone_agent() -> Population {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population
}

/// A drink, taken at a named place.
fn a_drink() -> ActionResult {
    ActionResult::success().with_drive_change(DriveType::Thirst, -0.5)
}

// --- what gets written down -------------------------------------------------

/// Answering a need writes the need, the doing, and the ground down together.
#[test]
fn answering_a_need_is_written_down_against_the_place() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];
    let bank = (14, 9, 0);

    agent.link_what_worked(
        &Action::Gather { resource_type: "water".to_string() },
        &a_drink(),
        DriveType::Thirst,
        bank,
        100,
    );

    // The verb and the thing it was done to are separate elements, so each
    // can be believed to a different degree
    assert_eq!(agent.patterns.how_often(DriveType::Thirst, "gather"), 1);
    let (what, trail) = agent
        .patterns
        .what_answers(DriveType::Thirst)
        .expect("something answers thirst now");
    assert_eq!(what, "gather");
    assert_eq!(trail.last_worked, 100);

    // And the ground is written down beside them, as its own element
    let ground = agent
        .patterns
        .trail(DriveType::Thirst, &Element::At(bank))
        .expect("the bank was written down");
    assert_eq!(ground.last_worked, 100);
    assert!(ground.strength > 0.0);

    // As is what it was done to
    assert!(agent.patterns.strength(DriveType::Thirst, &Element::On("water".to_string())) > 0.0);
}

/// A drive that barely moved is not evidence of anything.
#[test]
fn a_need_that_barely_moved_teaches_nothing() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];

    let barely = ActionResult::success()
        .with_drive_change(DriveType::Thirst, -(Patterns::ENOUGH_TO_NOTICE / 2.0));

    agent.link_what_worked(
        &Action::Wait,
        &barely,
        DriveType::Thirst,
        (1, 1, 0),
        10,
    );

    assert!(
        agent.patterns.what_answers(DriveType::Thirst).is_none(),
        "joining a drive's own drift to whatever the agent happened to be \
         doing is how a superstition gets made"
    );
}

/// An action aimed at a need that does not answer it counts against.
#[test]
fn a_place_that_stops_working_stops_being_worth_the_walk() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];
    let bank = (14, 9, 0);
    let drinking = Action::Gather { resource_type: "water".to_string() };

    for tick in 0..4 {
        agent.link_what_worked(&drinking, &a_drink(), DriveType::Thirst, bank, tick);
    }
    assert_eq!(
        agent.patterns.where_it_worked(DriveType::Thirst, 4),
        Some(bank),
        "four drinks in the same place is a place worth going back to"
    );

    // Now the river is dry.
    for _ in 0..4 {
        agent.link_what_worked(
            &drinking,
            &ActionResult::failure("No water sources nearby".to_string()),
            DriveType::Thirst,
            bank,
            5,
        );
    }

    assert_eq!(
        agent.patterns.where_it_worked(DriveType::Thirst, 5),
        None,
        "and four dry trips is not"
    );
}

/// Twice is a coincidence; a habit takes more.
#[test]
fn one_lucky_drink_is_not_a_place_worth_walking_to() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];
    let bank = (20, 20, 0);
    let drinking = Action::Gather { resource_type: "water".to_string() };

    for tick in 0..(Patterns::A_HABIT_BY_NOW - 1) {
        agent.link_what_worked(&drinking, &a_drink(), DriveType::Thirst, bank, tick);
        assert_eq!(
            agent.patterns.where_it_worked(DriveType::Thirst, tick),
            None,
            "still a coincidence after {} times",
            tick + 1
        );
    }

    agent.link_what_worked(&drinking, &a_drink(), DriveType::Thirst, bank, 9);
    assert_eq!(
        agent.patterns.where_it_worked(DriveType::Thirst, 9),
        Some(bank),
        "and a habit after {}",
        Patterns::A_HABIT_BY_NOW
    );
}

/// A place remembered from last year is not a place to walk to today.
#[test]
fn a_place_goes_stale() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];
    let patch = (30, 30, 0);

    for tick in 0..6 {
        agent.link_what_worked(
            &Action::Gather { resource_type: "food".to_string() },
            &ActionResult::success().with_drive_change(DriveType::Hunger, -0.4),
            DriveType::Hunger,
            patch,
            tick,
        );
    }

    assert_eq!(
        agent.patterns.where_it_worked(DriveType::Hunger, 6),
        Some(patch)
    );
    assert_eq!(
        agent
            .patterns
            .where_it_worked(DriveType::Hunger, 6 + Patterns::STILL_WORTH_THE_WALK + 1),
        None,
        "a patch picked bare in the spring is not worth the walk in the autumn"
    );
}

/// One action can answer more than one need, and both get written down.
#[test]
fn one_doing_can_answer_two_needs() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];

    let a_meal_by_the_river = ActionResult::success()
        .with_drive_change(DriveType::Hunger, -0.4)
        .with_drive_change(DriveType::Thirst, -0.3);

    agent.link_what_worked(
        &Action::Fish,
        &a_meal_by_the_river,
        DriveType::Hunger,
        (5, 5, 0),
        1,
    );

    assert_eq!(agent.patterns.how_often(DriveType::Hunger, "fish"), 1);
    assert_eq!(agent.patterns.how_often(DriveType::Thirst, "fish"), 1);
}

/// The ground under your feet is not somewhere to walk to.
#[test]
fn the_place_you_are_standing_is_not_a_destination() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];
    let here = (7, 7, 0);

    for tick in 0..6 {
        agent.link_what_worked(
            &Action::Gather { resource_type: "water".to_string() },
            &a_drink(),
            DriveType::Thirst,
            here,
            tick,
        );
    }

    assert_eq!(
        agent.somewhere_that_answered(DriveType::Thirst, here, 6),
        None,
        "\"where do I go\" is not answered by \"here\""
    );
    assert_eq!(
        agent.somewhere_that_answered(DriveType::Thirst, (40, 40, 0), 6),
        Some(here),
        "but from across the map it is"
    );
}

// --- what it changes --------------------------------------------------------

/// A thirsty agent with nowhere in reach walks back to where it drank.
#[test]
fn a_thirsty_agent_walks_back_to_the_bank_it_drank_from() {
    let population = a_lone_agent();
    let mut world = World::new(WorldConfig::default());

    // Strip every drop of water out of the world, so that nothing but the
    // remembered pattern can produce an answer.
    world
        .resources
        .retain(|resource| resource.resource_type != ResourceType::Water);

    let mut simulation = Simulation::new(world, population);
    let here = simulation.population.agents[0].state.position;
    let bank = (here.0 + 30, here.1 + 30, here.2);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.drink_water(1000.0);
        if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
            thirst.value = 0.9;
        }
        for tick in 0..6 {
            agent.link_what_worked(
                &Action::Gather { resource_type: "water".to_string() },
                &a_drink(),
                DriveType::Thirst,
                bank,
                tick,
            );
        }
    }

    let action = {
        let agent = &simulation.population.agents[0];
        simulation.what_this_drive_offers(DriveType::Thirst, agent, here)
    };

    assert_eq!(
        action,
        Some(Action::Move { target: bank }),
        "with no water anywhere in reach, the remembered bank is the answer"
    );
}

/// And with water right there it does not walk anywhere.
#[test]
fn a_remembered_bank_does_not_beat_the_stream_at_your_feet() {
    let population = a_lone_agent();
    let mut world = World::new(WorldConfig::default());
    let here = population.agents[0].state.position;

    world.resources.push(ResourceNode::new(
        ResourceType::Water,
        crate::world::Position::new(here.0, here.1),
        1000,
    ));

    let mut simulation = Simulation::new(world, population);
    let far_off = (here.0 + 40, here.1 + 40, here.2);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.drink_water(1000.0);
        for tick in 0..6 {
            agent.link_what_worked(
                &Action::Gather { resource_type: "water".to_string() },
                &a_drink(),
                DriveType::Thirst,
                far_off,
                tick,
            );
        }
    }

    let action = {
        let agent = &simulation.population.agents[0];
        simulation.what_this_drive_offers(DriveType::Thirst, agent, here)
    };

    assert_eq!(
        action,
        Some(Action::Gather { resource_type: "water".to_string() }),
        "nobody walks forty tiles past a stream"
    );
}

/// A settlement works patterns out for itself, by living.
#[test]
fn a_settlement_works_out_what_answers_what() {
    let mut population = Population::new();
    for _ in 0..8 {
        population.spawn_agent(AgentConfig::default());
    }
    let world = World::new(WorldConfig::default());
    let mut simulation = Simulation::new(world, population);

    for agent in simulation.population.agents.iter() {
        assert!(
            agent.patterns.is_empty(),
            "nobody arrives having worked anything out"
        );
    }

    // Put water where the people are. What is under test is whether a
    // settlement joins the drinking to the drink, not whether a random world
    // happens to put a spring inside walking distance of where it dropped
    // them - which it does not always, and this failed about once in fifteen
    // runs before the water was put there on purpose.
    let here = simulation.population.agents[0].state.position;
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Water,
        crate::world::Position::new(here.0, here.1),
        100_000,
    ));

    for _ in 0..600 {
        simulation.tick();
    }

    let now = simulation.current_tick;
    let worked_out: usize = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .map(|agent| agent.patterns.how_much_i_have_worked_out())
        .sum();

    assert!(
        worked_out > 0,
        "fifty days of living should join at least one doing to one need"
    );

    let anybody_knows_where_water_is = simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.state.is_alive)
        .any(|agent| agent.patterns.where_it_worked(DriveType::Thirst, now).is_some());

    assert!(
        anybody_knows_where_water_is,
        "and somebody should have worked out where the water is"
    );
}

/// What one agent works out is that agent's own.
#[test]
fn a_pattern_belongs_to_the_man_who_noticed_it() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    population.agents[0].link_what_worked(
        &Action::Gather { resource_type: "water".to_string() },
        &a_drink(),
        DriveType::Thirst,
        (3, 3, 0),
        1,
    );

    assert_eq!(
        population.agents[0].patterns.how_often(DriveType::Thirst, "gather"),
        1
    );
    assert_eq!(
        population.agents[1].patterns.how_often(DriveType::Thirst, "gather"),
        0
    );
}

/// A founder is born having worked nothing out.
#[test]
fn nobody_arrives_knowing_what_answers_what() {
    let population = a_lone_agent();
    let founder: &Agent = &population.agents[0];

    assert!(founder.patterns.is_empty());
    assert_eq!(founder.patterns.how_much_i_have_worked_out(), 0);
    assert_eq!(
        founder.somewhere_that_answered(DriveType::Thirst, (0, 0, 0), 0),
        None
    );
}

/// The whole of the generalising: what two successes share outgrows what they
/// differ in.
///
/// "I went hunting out east and got meat which satisfied my hunger drive" and
/// "I went hunting out west and got meat which satisfied my hunger drive" share
/// every element except the direction. So hunting should end up believed in
/// twice as strongly as east does, without anybody having written down that
/// the hunting is the part that matters.
#[test]
fn hunting_east_and_west_teaches_hunting_rather_than_east() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];

    let east = (40, 0, 0);
    let west = (-40, 0, 0);
    let camp = (0, 0, 0);

    // Both trips set out from the same camp, so both have a bearing
    agent.errand = Some(crate::agents::Errand {
        going_to: east,
        set_out_from: camp,
        to_make: None,
        for_drive: DriveType::Hunger,
        pressed_this_hard: 1.0,
        turns_on_it: 1,
        set_aside: 0,
    });
    agent.link_what_worked(&a_hunt(), &a_meal(), DriveType::Hunger, east, 10);

    agent.errand = Some(crate::agents::Errand {
        going_to: west,
        set_out_from: camp,
        to_make: None,
        for_drive: DriveType::Hunger,
        pressed_this_hard: 1.0,
        turns_on_it: 1,
        set_aside: 0,
    });
    agent.link_what_worked(&a_hunt(), &a_meal(), DriveType::Hunger, west, 20);

    let hunting = agent
        .patterns
        .strength(DriveType::Hunger, &Element::Did("hunt".to_string()));
    let out_east = agent
        .patterns
        .strength(DriveType::Hunger, &Element::Toward(Bearing::East));
    let out_west = agent
        .patterns
        .strength(DriveType::Hunger, &Element::Toward(Bearing::West));

    assert!(hunting > 0.0, "hunting answered hunger twice and is believed");
    assert!(out_east > 0.0, "so did going east, once");
    assert!(
        hunting > out_east && hunting > out_west,
        "the doing is shared by both trips and the direction is not, so the \
         doing should be the better worn of the two: hunting {hunting}, \
         east {out_east}, west {out_west}"
    );
    assert!(
        (out_east - out_west).abs() < 1e-6,
        "and neither direction has anything the other has not"
    );
}

/// A path nobody walks grows over.
#[test]
fn a_trail_nobody_walks_grows_over() {
    let mut population = a_lone_agent();
    let agent = &mut population.agents[0];
    let bank = (14, 9, 0);

    for tick in 0..3 {
        agent.link_what_worked(
            &Action::Gather { resource_type: "water".to_string() },
            &a_drink(),
            DriveType::Thirst,
            bank,
            tick,
        );
    }

    let when_fresh = agent
        .patterns
        .strength(DriveType::Thirst, &Element::At(bank));
    assert!(when_fresh > 0.0);

    // A season of not going back
    let a_season = Patterns::STILL_WORTH_THE_WALK;
    agent.patterns.fade(a_season);

    let after_a_season = agent
        .patterns
        .strength(DriveType::Thirst, &Element::At(bank));
    assert!(
        after_a_season < when_fresh,
        "the trail should have faded: {when_fresh} -> {after_a_season}"
    );

    // And a year of it takes the place off the map altogether, which is what
    // lets an agent hold a corner of the world instead of all of it
    agent.patterns.fade(a_season * 4);
    assert_eq!(
        agent
            .patterns
            .strength(DriveType::Thirst, &Element::At(bank)),
        0.0,
        "a place nobody has been to in a year is forgotten"
    );
}

/// A need answered in one turn is worth more than the same need answered in ten.
#[test]
fn a_quick_answer_lays_a_stronger_trail_than_a_slow_one() {
    let mut population = a_lone_agent();

    let near = (1, 0, 0);
    let far = (40, 0, 0);

    let quick = {
        let agent = &mut population.agents[0];
        agent.errand = Some(crate::agents::Errand {
            going_to: near,
            set_out_from: (0, 0, 0),
            to_make: None,
            for_drive: DriveType::Thirst,
            pressed_this_hard: 1.0,
            turns_on_it: 1,
            set_aside: 0,
        });
        agent.link_what_worked(
            &Action::Gather { resource_type: "water".to_string() },
            &a_drink(),
            DriveType::Thirst,
            near,
            10,
        );
        agent.patterns.strength(DriveType::Thirst, &Element::At(near))
    };

    let slow = {
        let mut second = a_lone_agent();
        let agent = &mut second.agents[0];
        agent.errand = Some(crate::agents::Errand {
            going_to: far,
            set_out_from: (0, 0, 0),
            to_make: None,
            for_drive: DriveType::Thirst,
            pressed_this_hard: 1.0,
            turns_on_it: 10,
            set_aside: 0,
        });
        agent.link_what_worked(
            &Action::Gather { resource_type: "water".to_string() },
            &a_drink(),
            DriveType::Thirst,
            far,
            10,
        );
        agent.patterns.strength(DriveType::Thirst, &Element::At(far))
    };

    assert!(
        quick > slow,
        "the same drink for a tenth of the walking should be worth more: \
         one turn {quick}, ten turns {slow}"
    );
}

/// Elements are map keys, and JSON has only string keys.
#[test]
fn every_element_survives_being_written_down_and_read_back() {
    let all = [
        Element::Did("gather".to_string()),
        Element::On("Berries".to_string()),
        Element::At((-14, 9, 3)),
        Element::Toward(Bearing::SouthWest),
        Element::When(crate::environment::seasons::Season::Winter),
    ];

    for element in all {
        let written = element.to_string();
        let read_back = Element::try_from(written.clone())
            .unwrap_or_else(|why| panic!("{written} did not come back: {why}"));
        assert_eq!(element, read_back, "{written}");
    }

    // And through the format the map keys actually go through
    let mut patterns = Patterns::default();
    patterns.it_worked(
        DriveType::Hunger,
        &[Element::At((3, 4, 0)), Element::Did("fish".to_string())],
        0.5,
        1,
    );
    let written = serde_json::to_string(&patterns).expect("patterns write");
    let read_back: Patterns = serde_json::from_str(&written).expect("patterns read");
    assert_eq!(
        read_back.strength(DriveType::Hunger, &Element::At((3, 4, 0))),
        patterns.strength(DriveType::Hunger, &Element::At((3, 4, 0))),
    );
}

/// A bearing is taken from where the walk began, and a step is not a bearing.
#[test]
fn a_bearing_is_taken_from_where_the_walk_began() {
    let camp = (0, 0, 0);

    assert_eq!(Bearing::from_home(camp, (40, 1, 0)), Some(Bearing::East));
    assert_eq!(Bearing::from_home(camp, (-40, 1, 0)), Some(Bearing::West));
    assert_eq!(Bearing::from_home(camp, (1, 40, 0)), Some(Bearing::North));
    assert_eq!(Bearing::from_home(camp, (30, 30, 0)), Some(Bearing::NorthEast));
    assert_eq!(Bearing::from_home(camp, (-30, -30, 0)), Some(Bearing::SouthWest));

    // Standing on the spot, or one pace off it, has no direction in it
    assert_eq!(Bearing::from_home(camp, camp), None);
    assert_eq!(Bearing::from_home(camp, (1, 1, 0)), None);
}

/// A hunt, and what a hunt that fed somebody looks like.
fn a_hunt() -> Action {
    Action::Hunt {
        animal_id: uuid::Uuid::nil(),
        weapon: None,
    }
}

fn a_meal() -> ActionResult {
    ActionResult::success().with_drive_change(DriveType::Hunger, -0.5)
}
