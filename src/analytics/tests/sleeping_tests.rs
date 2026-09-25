// src/analytics/tests/sleeping_tests.rs
//! Ground nobody is near sleeps, and wakes exactly where it would have been.
//!
//! A resource node further from every living person than anybody looks stops
//! being brought up to date each day, and when somebody comes near it lives
//! its missed days over from a log of each day's weather. The claim is not
//! that this is close: it is that it is the same, to the bit, as never having
//! slept. See `world::sleeping` and ISSUES_FOUND #249.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// Everything about every node that a day of regrowth can touch.
fn every_node(world: &World) -> Vec<(i32, i32, u32, u32, u32, u32, u32, bool, u32)> {
    world
        .resources
        .iter()
        .map(|r| {
            (
                r.position.x,
                r.position.y,
                r.amount,
                r.max_amount,
                r.inflow_carried.to_bits(),
                r.flow.to_bits(),
                r.ripe_stand,
                r.on_a_field,
                r.taken_since_it_ripened,
            )
        })
        .collect()
}

/// And everything about every person.
fn everybody(simulation: &Simulation) -> Vec<(uuid::Uuid, bool, (i32, i32, i32), u32, u32)> {
    simulation
        .population
        .agents
        .iter()
        .map(|a| {
            (
                a.id,
                a.state.is_alive,
                a.state.position,
                a.state.health.to_bits(),
                a.state.physiology.reserve.to_bits(),
            )
        })
        .collect()
}

fn a_settlement_on_a_big_map(seed: u64, sleeping: bool) -> Simulation {
    crate::core::dice::seed(seed);
    let mut world = World::new(WorldConfig::default().with_size(320, 320));
    world.nobody_sleeps = !sleeping;
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }
    Simulation::new(world, population)
}

/// A settlement on a map mostly far from it: the far ground sleeps, and woken
/// at the end the world is the world it would have been.
#[test]
fn a_world_whose_far_ground_slept_is_the_same_world() {
    const TURNS: usize = 60 * crate::environment::seasons::PLANNING_PERIODS_PER_DAY as usize;

    let mut awake = a_settlement_on_a_big_map(7, false);
    let rolled_before = crate::core::dice::draws_taken();
    for _ in 0..TURNS {
        awake.take_a_turn();
    }
    let rolled_awake = crate::core::dice::draws_taken() - rolled_before;

    let mut slept = a_settlement_on_a_big_map(7, true);
    let rolled_before = crate::core::dice::draws_taken();
    let mut most_asleep = 0;
    for _ in 0..TURNS {
        slept.take_a_turn();
        most_asleep = most_asleep.max(slept.world.how_many_are_asleep());
    }
    let rolled_slept = crate::core::dice::draws_taken() - rolled_before;

    assert!(
        most_asleep > slept.world.resources.len() / 2,
        "most of a 320-cell map is far from a settlement of twelve, and \
         {most_asleep} of {} nodes slept",
        slept.world.resources.len()
    );

    assert_eq!(rolled_slept, rolled_awake, "the sleeping world rolled differently");
    assert_eq!(everybody(&slept), everybody(&awake), "and its people are not the same people");

    slept.world.wake_everything();
    assert_eq!(slept.world.how_many_are_asleep(), 0);
    assert_eq!(
        every_node(&slept.world),
        every_node(&awake.world),
        "woken, every node should be where it would have been"
    );
}

/// A world nobody has told where its people are keeps everything awake: a
/// world run on its own is not an empty world.
#[test]
fn a_world_nobody_has_told_about_keeps_everything_awake() {
    let mut world = World::new(WorldConfig::default().with_size(200, 200));
    for _ in 0..(3 * crate::environment::seasons::PLANNING_PERIODS_PER_DAY) {
        world.take_a_turn();
    }
    assert_eq!(world.how_many_are_asleep(), 0);
}

/// An empty world sleeps whole, and wakes the same as one that never slept -
/// over a whole year, through every season's growing and falling off.
#[test]
fn an_empty_world_sleeps_a_year_and_wakes_the_same() {
    let a_year = crate::environment::seasons::PLANNING_PERIODS_PER_YEAR as usize;

    // One after the other, not turn about: both draw on the one seeded dice,
    // and interleaved they would each get the other's weather.
    let a_year_of = |sleeping: bool| {
        crate::core::dice::seed(11);
        let mut world = World::new(WorldConfig::default().with_size(120, 120));
        world.nobody_sleeps = !sleeping;
        for _ in 0..a_year {
            world.people_are_at(Vec::new());
            world.take_a_turn();
        }
        world
    };
    let awake = a_year_of(false);
    let mut slept = a_year_of(true);
    assert_eq!(slept.how_many_are_asleep(), slept.resources.len(), "nobody is near anything");

    slept.wake_everything();
    assert_eq!(every_node(&slept), every_node(&awake));
}

/// Somebody standing somewhere keeps the ground round them awake.
#[test]
fn the_ground_near_somebody_stays_awake() {
    let mut world = World::new(WorldConfig::default().with_size(400, 400));
    for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
        world.people_are_at(vec![(200, 200)]);
        world.take_a_turn();
    }

    for node in &world.resources {
        let near = (node.position.x - 200).abs() <= 60 && (node.position.y - 200).abs() <= 60;
        if near {
            assert!(
                node.asleep_since.is_none(),
                "a node at {:?} is within sixty of somebody and asleep",
                node.position
            );
        }
    }
    assert!(world.how_many_are_asleep() > 0, "and the far corners sleep");
}

/// Everybody remembers food in far country - further off than anything stays
/// awake - so the search for the best food anywhere reads nodes that are
/// asleep. A node is brought up to date before anybody decides with it in
/// mind, so the settlement decides exactly as it would have with nothing
/// asleep. See ISSUES_FOUND #251.
#[test]
fn remembering_far_country_decides_the_same_as_if_it_never_slept() {
    const TURNS: usize = 60 * crate::environment::seasons::PLANNING_PERIODS_PER_DAY as usize;

    let remembering_far_country = |sleeping: bool| {
        let mut simulation = a_settlement_on_a_big_map(7, sleeping);
        let far_food: Vec<(crate::world::Position, crate::world::ResourceType)> = simulation
            .world
            .resources
            .iter()
            .filter(|node| node.resource_type.is_edible())
            .map(|node| (node.position, node.resource_type))
            .collect();
        for agent in &mut simulation.population.agents {
            let (x, y, _) = agent.state.position;
            for (at, what) in &far_food {
                let far = (at.x - x).abs() > crate::world::sleeping::FAR_ENOUGH_TO_SLEEP
                    || (at.y - y).abs() > crate::world::sleeping::FAR_ENOUGH_TO_SLEEP;
                if far {
                    agent.exploration_knowledge.discover_resource(*at, *what, 0);
                }
            }
        }
        simulation
    };

    let mut awake = remembering_far_country(false);
    let rolled_before = crate::core::dice::draws_taken();
    for _ in 0..TURNS {
        awake.take_a_turn();
    }
    let rolled_awake = crate::core::dice::draws_taken() - rolled_before;

    let mut slept = remembering_far_country(true);
    let rolled_before = crate::core::dice::draws_taken();
    for _ in 0..TURNS {
        slept.take_a_turn();
    }
    let rolled_slept = crate::core::dice::draws_taken() - rolled_before;

    assert_eq!(rolled_slept, rolled_awake, "the sleeping world rolled differently");
    assert_eq!(everybody(&slept), everybody(&awake), "and its people are not the same people");

    slept.world.wake_everything();
    assert_eq!(every_node(&slept.world), every_node(&awake.world));
}

/// One patch in far country, stripped bare, and a man two hundred cells off
/// who remembers it. While nobody is near, it sleeps through the weeks it
/// would have borne again, and read as it stands it is still bare: he would
/// not set out for it. Woken before he makes up his mind, it is what it would
/// have been in a world where nothing slept, and he sets out for it as he
/// would have there.
#[test]
fn a_far_patch_he_remembers_is_read_as_it_stands_today() {
    use crate::world::{Position, ResourceNode, ResourceType};

    const HERE: (i32, i32) = (10, 10);
    let far_patch = Position::new(290, 290);
    let days = 40;

    let the_country = |sleeping: bool| {
        crate::core::dice::seed(3);
        let mut world = World::new(WorldConfig::default().with_size(320, 320));
        world.nobody_sleeps = !sleeping;
        world.resources.clear();
        let mut patch = ResourceNode::new(ResourceType::Food, far_patch, 60);
        patch.amount = 0;
        world.resources.push(patch);
        world.file_the_nodes();
        // From the first day the berries come on, so that there is something
        // for it to have borne while it slept.
        let the_berries_come_on = (0..crate::environment::seasons::DAYS_PER_YEAR)
            .find(|&day| {
                ResourceType::Food.is_it_bearing(day)
                    && !ResourceType::Food.is_it_bearing(
                        (day + crate::environment::seasons::DAYS_PER_YEAR - 1)
                            % crate::environment::seasons::DAYS_PER_YEAR,
                    )
            })
            .expect("berries bear some time in the year");
        world.climate.calendar.day_of_year = the_berries_come_on;
        for _ in 0..(days * crate::environment::seasons::PLANNING_PERIODS_PER_DAY) {
            world.people_are_at(vec![HERE]);
            world.take_a_turn();
        }

        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());
        let mut simulation = Simulation::new(world, population);
        let agent = &mut simulation.population.agents[0];
        agent.state.position = (HERE.0, HERE.1, 0);
        agent.exploration_knowledge.known_resources.clear();
        agent
            .exploration_knowledge
            .discover_resource(far_patch, ResourceType::Food, 0);
        simulation
    };

    let awake = the_country(false);
    let mut slept = the_country(true);
    let here = (HERE.0, HERE.1, 0);

    // The fixture is only worth anything if sleeping left it behind.
    assert!(slept.world.resources[0].asleep_since.is_some(), "the far patch should be asleep");
    assert_eq!(slept.world.resources[0].amount, 0, "asleep, it is as bare as it fell asleep");
    assert!(
        awake.world.resources[0].amount > 0,
        "and awake it has borne again in {days} days - pick a better season"
    );
    let he_would = awake.the_best_food_anywhere(&awake.population.agents[0].clone(), here);
    assert_eq!(he_would, Some(far_patch), "with nothing asleep he sets out for it");
    assert_eq!(
        slept.the_best_food_anywhere(&slept.population.agents[0].clone(), here),
        None,
        "read asleep, it is bare"
    );

    slept.wake_what_this_one_remembers(0);
    assert_eq!(slept.world.resources[0].amount, awake.world.resources[0].amount);
    assert_eq!(slept.the_best_food_anywhere(&slept.population.agents[0].clone(), here), he_would);
}
