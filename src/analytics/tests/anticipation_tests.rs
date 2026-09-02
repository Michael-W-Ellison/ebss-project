//! Looking one job ahead: what an agent knows about a need before it arrives.
//!
//! "The planner should attempt to anticipate drive demand increase so that
//! actions can be efficiently executed, reducing the odds of tasks being
//! dropped mid-completion. Each agent should be slightly different due to
//! varying drive demands and personality traits. It should also allow for the
//! proper preparation of actions such as hunting requiring a weapon."
//!
//! Three things, and this file covers all three: the clock that says how long
//! there is (`how_long_before_this_asks`), the rule that reads it before
//! setting out (`what_will_not_wait_for`, `what_this_will_not_outlast`), and
//! the preparation that gets a spear into a hand before the hunt rather than
//! after it (`what_a_hunt_wants_first`).

use crate::agents::{AgentConfig, InventoryItem, LifeStage, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::{World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.life_stage = LifeStage::Adult;
    simulation
}

/// A need already over its threshold is not something to plan around: it is
/// asking now.
#[test]
fn a_need_already_asking_has_no_time_left() {
    let mut simulation = one_person();
    let agent = &mut simulation.population.agents[0];

    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 1.0;
    }

    assert_eq!(
        simulation.population.agents[0].how_long_before_this_asks(DriveType::Hunger),
        Some(0),
        "a need at full is asking now"
    );
}

/// And a need further from its threshold is further off in time. This is the
/// whole of the anticipation: the drive's own arithmetic, read forwards.
#[test]
fn a_need_further_down_is_further_off() {
    let mut simulation = one_person();

    let threshold = simulation.population.agents[0]
        .drives
        .get(DriveType::Hunger)
        .map(|drive| drive.threshold)
        .expect("hunger exists");

    let mut when = |value: f32| {
        if let Some(hunger) = simulation.population.agents[0]
            .drives
            .get_mut(DriveType::Hunger)
        {
            hunger.value = value;
        }
        simulation.population.agents[0]
            .how_long_before_this_asks(DriveType::Hunger)
            .expect("hunger can always ask")
    };

    let nearly = when(threshold * 0.9);
    let halfway = when(threshold * 0.5);
    let empty = when(0.0);

    assert!(
        empty > halfway && halfway > nearly,
        "the further below the threshold, the longer there is: {empty} > {halfway} > {nearly}"
    );
}

/// Two people in the same field get two different answers, because the clock
/// is read off each one's own drive. Here it is the weight of having been
/// ignored - a need that has been denied for days builds faster.
#[test]
fn two_people_do_not_get_the_same_answer() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let threshold = simulation.population.agents[0]
        .drives
        .get(DriveType::Hunger)
        .map(|drive| drive.threshold)
        .expect("hunger exists");

    for who in 0..2 {
        if let Some(hunger) = simulation.population.agents[who]
            .drives
            .get_mut(DriveType::Hunger)
        {
            hunger.value = threshold * 0.5;
            hunger.denied_ticks = 0;
        }
    }

    // One of them has been going short for two days.
    if let Some(hunger) = simulation.population.agents[1]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.denied_ticks = 24;
    }

    let easy = simulation.population.agents[0]
        .how_long_before_this_asks(DriveType::Hunger)
        .expect("hunger can ask");
    let pressed = simulation.population.agents[1]
        .how_long_before_this_asks(DriveType::Hunger)
        .expect("hunger can ask");

    assert!(
        pressed < easy,
        "the one who has been going short gets there sooner: {pressed} against {easy}"
    );
}

/// A job the body's own clock will not last out is one to think again about,
/// and a job that finishes with time in hand is not.
///
/// The rule is deliberately the death clock rather than the threshold. Written
/// against the threshold it fired on nearly everything - hunger is a few turns
/// off asking most of the time and outranks every secondary need - and a
/// settlement that defers every job longer than three turns does nothing but
/// eat. See `what_will_not_wait_for`.
#[test]
fn a_job_the_body_will_not_last_out_is_one_to_think_again_about() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];

    let clocks: Vec<f32> = [DriveType::Hunger, DriveType::Thirst, DriveType::Rest]
        .into_iter()
        .filter_map(|drive| agent.state.ticks_before_this_kills_me(drive))
        .filter(|left| left.is_finite())
        .collect();

    let soonest = clocks
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    assert!(soonest.is_finite(), "a body has at least one clock running");

    let comfortable = (soonest as u32).saturating_sub(1);
    assert_eq!(
        agent.what_will_not_wait_for(DriveType::Luxury, comfortable),
        None,
        "a job that finishes with time in hand is not interrupted"
    );

    let far_too_long = soonest as u32 + 10;
    let gives_way = agent.what_will_not_wait_for(DriveType::Luxury, far_too_long);
    assert!(
        gives_way.is_some_and(|drive| drive.rank() == crate::core::DriveRank::Primary),
        "a job longer than the body has left gives way to what kills: {gives_way:?}"
    );
}

/// It does not give way to a need of its own band. Two secondary needs trading
/// places is the ordinary business of a day, and turning round for it is how
/// an agent gets nothing done.
#[test]
fn a_need_of_the_same_standing_does_not_take_the_turn() {
    let mut simulation = one_person();

    for drive_type in [DriveType::Sustenance, DriveType::Shelter] {
        let threshold = simulation.population.agents[0]
            .drives
            .get(drive_type)
            .map(|drive| drive.threshold)
            .expect("the drive exists");

        if let Some(drive) = simulation.population.agents[0].drives.get_mut(drive_type) {
            drive.value = threshold;
        }
    }

    let agent = &simulation.population.agents[0];

    assert_ne!(
        agent.what_will_not_wait_for(DriveType::Sustenance, 100_000),
        Some(DriveType::Shelter),
        "one secondary need does not interrupt another"
    );
}

/// And nothing at all interrupts a job that is over before anything could.
#[test]
fn nothing_interrupts_a_job_that_takes_one_turn() {
    let mut simulation = one_person();

    if let Some(hunger) = simulation.population.agents[0]
        .drives
        .get_mut(DriveType::Hunger)
    {
        hunger.value = 1.0;
    }

    let agent = &simulation.population.agents[0];

    assert_eq!(
        agent.what_will_not_wait_for(DriveType::Luxury, 0),
        None,
        "there is no room to be interrupted inside a single turn"
    );
}

/// A walk is as long as the walk; a one-turn job is one turn.
#[test]
fn how_long_a_job_is_is_the_length_of_the_walk() {
    let simulation = one_person();
    let agent = &simulation.population.agents[0];
    let here = agent.state.position;

    assert_eq!(
        Simulation::how_long_this_would_take(
            agent,
            here,
            &Action::Move { target: (here.0 + 14, here.1, here.2) }
        ),
        14
    );

    assert_eq!(
        Simulation::how_long_this_would_take(
            agent,
            here,
            &Action::Eat { food_type: "generic".to_string() }
        ),
        1
    );
}

/// Somebody who means to hunt and has nothing to hunt with goes and gets
/// something first, rather than walking to the animal and being refused.
#[test]
fn a_hunter_with_nothing_in_hand_goes_and_gets_a_spear() {
    let mut simulation = one_person();

    // The makings of the humblest thing there is, and the knife that shapes it
    for (what, how_many) in [("wood", 4u32), ("stoneknife", 1)] {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.5));
    }

    let agent = &simulation.population.agents[0];
    let getting_one = simulation.what_a_hunt_wants_first(agent);

    assert!(
        matches!(getting_one, Some(Action::Craft { .. }) | Some(Action::Gather { .. })),
        "an empty-handed hunter should be getting hold of something, not hunting: {getting_one:?}"
    );
}

/// And somebody who already has one gets on with the hunt.
#[test]
fn a_hunter_with_a_spear_is_not_sent_to_make_another() {
    let mut simulation = one_person();

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));

    let agent = &simulation.population.agents[0];

    assert_eq!(
        simulation.what_a_hunt_wants_first(agent),
        None,
        "there is nothing to prepare when the spear is already in the pack"
    );
}

/// The decision layer and the executor ask one question about whether a kill
/// is possible, so that nobody walks to an animal it cannot bring down.
///
/// This was two questions and two answers: `worth_hunting` read the equipment
/// slot, which nothing ever fills, and the executor read the pack. Measured,
/// 589 hunts in 599 were decided, walked to and then refused for want of a
/// spear.
#[test]
fn the_two_layers_agree_about_what_can_be_brought_down() {
    let mut simulation = one_person();

    let Some(deer) = simulation.world.animals.get_species("deer").cloned() else {
        return;
    };

    let empty_handed = &simulation.population.agents[0];
    assert!(
        !Simulation::could_bring_it_down(empty_handed, &deer),
        "a deer is bigger than a thrown stone will kill"
    );

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));

    let armed = &simulation.population.agents[0];
    assert!(
        Simulation::could_bring_it_down(armed, &deer),
        "with a spear in the pack it can"
    );
}

/// No verb states its precondition by naming one rung of a tool ladder.
///
/// This is the guard for the defect that cost 589 hunts. `HUNT` and `THROW`
/// both wanted `ThisInHand("spear")`, so a man with a sharpened stick, a sling
/// or a bow was refused before the executor was reached - and so was a man
/// after a rabbit, which needs nothing at all. A tool has siblings; a verb
/// that names one of them by hand has to be kept in step with the tool table
/// by somebody remembering to, and nobody did.
///
/// `AToolFor(trade)` is the way to say it, and where the requirement depends
/// on something this table cannot see - what is being hunted - the honest
/// answer is `BareHands` and a single owner elsewhere.
#[test]
fn no_verb_asks_for_one_rung_of_a_tool_ladder_by_name() {
    use crate::environment::making::EVERY_TOOL;
    use crate::environment::verbs::{Wants, EVERY_VERB};

    for verb in EVERY_VERB {
        let Wants::ThisInHand(what) = verb.wants else {
            continue;
        };

        assert!(
            EVERY_TOOL.iter().all(|tool| tool.called != what),
            "the verb {:?} asks for {what} by name, and {what} is one rung of a tool ladder - \
             ask for AToolFor(trade), or let whoever can see the job decide",
            verb.called
        );
    }
}

/// And a hunt, gathered across every verb that makes one up, asks for nothing
/// this table can be wrong about.
///
/// `what_this_action_cannot_do_without` collects the wants of *every* verb
/// with the same `done_by`, so a hunt asks both `HUNT` and `THROW`. Fixing one
/// of them changed nothing measurable, which is what a requirement written
/// twice looks like from the outside.
#[test]
fn a_hunt_asks_for_nothing_this_table_cannot_see() {
    let wanted = crate::environment::verbs::what_this_action_cannot_do_without("hunt");

    assert!(
        wanted.is_empty(),
        "what a hunt needs depends on the quarry, which this table cannot see - \
         `Simulation::could_bring_it_down` owns it. Still asked for: {wanted:?}"
    );
}

/// Nothing that grows back is deleted off the map when it is emptied.
///
/// `World::remove_depleted_resources` keeps a node at nought only if
/// `is_renewable` says so, and `is_renewable` used to keep its own hand-written
/// list of what grows. Greens and Roots were in neither list, so a patch picked
/// bare ceased to exist. The comment on that function states the case against
/// exactly what it was doing: "deleting it would make berry patches and fish
/// runs single-use and drain the world of food permanently."
#[test]
fn a_patch_picked_bare_is_still_there_to_grow_back() {
    use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

    let growing: Vec<ResourceType> = ResourceType::all()
        .into_iter()
        .filter(|kind| kind.how_fast_it_comes_back() > 0.0)
        .collect();

    assert!(!growing.is_empty(), "something in this world grows");

    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    for (which, kind) in growing.iter().enumerate() {
        let mut node = ResourceNode::new(*kind, Position::new(which as i32, 0), 30);
        node.amount = 0;
        world.resources.push(node);
    }

    world.remove_depleted_resources();

    for kind in growing {
        assert!(
            world.resources.iter().any(|node| node.resource_type == kind),
            "a patch of {kind:?} was picked bare and the world deleted it, so it \
             can never grow back"
        );
    }
}
