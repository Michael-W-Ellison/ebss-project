// src/analytics/tests/news_tests.rs
//! Tests that news carries an age, a room, and a shelf life.
//!
//! A claim used to record who made it and nothing else, so a man who honestly
//! reported a patch he saw last season was called a liar the moment somebody
//! found it picked. Telling was strictly two-handed - one speaker, one
//! listener, and nobody else heard a word of it however many people were
//! standing round - and a liar weighed only the man in front of him. And what
//! an agent remembered was capped by nothing at all, so a settlement that
//! talks carried the whole map in every head.

use crate::agents::exploration::Hearsay;
use crate::agents::{Agent, AgentConfig, LifeStage, Population};
use crate::core::{DriveType, Trait};
use crate::world::{Position, ResourceType, World, WorldConfig};

fn somebody() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.traits = crate::core::traits::TraitSet::new();
    agent.state.life_stage = LifeStage::Adult;
    agent.state.health = 100.0;
    agent
}

/// A week-old sighting is not a lie.
#[test]
fn a_week_old_sighting_found_empty_is_not_a_lie() {
    let a_week_ago = Hearsay {
        who: crate::core::dice::name(),
        they_saw_it_on: 100,
        told_me_on: 110,
        how_much_they_said: Some(20),
    };

    // A week is eighty-four ticks
    assert!(
        !a_week_ago.was_he_answerable_for_it(100 + 84),
        "a man who says he saw a patch a week back cannot be held to what is \
         there now"
    );
}

/// A sighting from this morning is.
#[test]
fn a_patch_just_passed_and_found_empty_is_a_lie() {
    let this_morning = Hearsay {
        who: crate::core::dice::name(),
        they_saw_it_on: 1000,
        told_me_on: 1000,
        how_much_they_said: Some(20),
    };

    assert!(
        this_morning.was_he_answerable_for_it(1004),
        "a man who says he walked past it an hour ago can be held to it"
    );
}

/// And the same claim ages out of being answerable.
#[test]
fn a_fresh_claim_stops_being_answerable_once_it_is_old() {
    let said = Hearsay {
        who: crate::core::dice::name(),
        they_saw_it_on: 1000,
        told_me_on: 1000,
        how_much_they_said: Some(20),
    };

    assert!(said.was_he_answerable_for_it(1010));
    assert!(
        !said.was_he_answerable_for_it(1000 + Hearsay::STILL_ANSWERABLE_FOR + 1),
        "a man is not answerable for ever; the ground changes under him"
    );
}

/// Being out of date costs a man some standing and no anger at all.
#[test]
fn being_out_of_date_is_not_the_same_as_lying() {
    let stale_teller = crate::core::dice::name();
    let liar = crate::core::dice::name();

    let mut heard_stale_news = somebody();
    let mut was_lied_to = somebody();

    heard_stale_news.found_out_they_were_out_of_date(stale_teller);
    was_lied_to.found_out_i_was_lied_to(liar, "food", 100);

    assert_eq!(
        heard_stale_news.emotions.anger, 0.0,
        "nobody is angry at a man whose news simply keeps badly"
    );
    assert!(
        was_lied_to.emotions.anger > 0.0,
        "and somebody is angry at a man who invented a place"
    );

    let stale_credit = heard_stale_news
        .knowledge
        .trust_ratings
        .get(&stale_teller)
        .expect("it should be on the record")
        .trust;
    let liar_credit = was_lied_to
        .knowledge
        .trust_ratings
        .get(&liar)
        .expect("so should this")
        .trust;

    assert!(
        stale_credit > liar_credit,
        "out of date should cost less than lying: {stale_credit:.3} against \
         {liar_credit:.3}"
    );
    assert_eq!(
        was_lied_to
            .knowledge
            .trust_ratings
            .get(&liar)
            .unwrap()
            .wrong_count,
        1,
        "a lie is a wrong answer"
    );
    assert_eq!(
        heard_stale_news
            .knowledge
            .trust_ratings
            .get(&stale_teller)
            .unwrap()
            .wrong_count,
        0,
        "and stale news is not one"
    );
}

/// A liar counts the people who can hear him.
#[test]
fn a_crowd_makes_a_man_think_twice() {
    let mut liar = somebody();
    liar.traits.add_trait(Trait::Dishonest);
    liar.traits.add_trait(Trait::Manipulative);

    let room: Vec<uuid::Uuid> = (0..8).map(|_| crate::core::dice::name()).collect();

    let alone_with_one = (0..400)
        .filter(|_| liar.would_lie_to_this_room(&room[..1], 0))
        .count();
    let in_front_of_eight = (0..400)
        .filter(|_| liar.would_lie_to_this_room(&room, 0))
        .count();

    assert!(
        in_front_of_eight < alone_with_one,
        "a lie told to eight people is eight people who may go and look: \
         {alone_with_one} against {in_front_of_eight} out of four hundred"
    );
    assert!(
        alone_with_one > 0,
        "and a private word should still be worth trying"
    );
}

/// An empty room is nobody to lie to.
#[test]
fn nobody_lies_to_an_empty_room() {
    let mut liar = somebody();
    liar.traits.add_trait(Trait::Dishonest);
    assert!(!liar.would_lie_to_this_room(&[], 0));
}

/// What an agent keeps is what it has some use for.
#[test]
fn a_thirsty_man_keeps_the_waterholes_and_lets_the_flax_go() {
    let mut agent = somebody();

    // Parched, and unbothered about anything else
    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
        thirst.value = 0.95;
        thirst.denied_ticks = 400;
    }
    agent.state.gone_without_water_for(3_000);

    // More places than anybody can hold in mind, half of them water and half
    // of them flax
    for n in 0..Agent::WHAT_A_MAN_CAN_HOLD_IN_MIND {
        agent.exploration_knowledge.discover_resource(
            Position::new(n as i32, 0),
            ResourceType::Water,
            10,
        );
        agent.exploration_knowledge.discover_resource(
            Position::new(n as i32, 1),
            ResourceType::Flax,
            10,
        );
    }

    agent.forget_what_does_not_matter(20);

    let (mut water, mut flax) = (0, 0);
    for what in agent.exploration_knowledge.known_resources.values() {
        match what {
            ResourceType::Water => water += 1,
            ResourceType::Flax => flax += 1,
            _ => {}
        }
    }

    assert_eq!(
        agent.exploration_knowledge.known_resources.len(),
        Agent::WHAT_A_MAN_CAN_HOLD_IN_MIND,
        "he can only hold so much in mind at once"
    );
    assert!(
        water > flax,
        "and what he holds on to is what he has a use for: {water} waterholes \
         against {flax} flax patches"
    );
}

/// And nothing is forgotten while there is room for it.
#[test]
fn nothing_is_forgotten_while_there_is_room_for_it() {
    let mut agent = somebody();
    for n in 0..10 {
        agent.exploration_knowledge.discover_resource(
            Position::new(n, 0),
            ResourceType::Flax,
            10,
        );
    }

    agent.forget_what_does_not_matter(5_000);

    assert_eq!(
        agent.exploration_knowledge.known_resources.len(),
        10,
        "a man with ten things on his mind forgets none of them"
    );
}

/// Walking past a thing again is seeing it again.
#[test]
fn walking_past_a_thing_again_is_seeing_it_again() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents[0].state.position = (30, 30, 0);

    let here = Position::new(31, 30);
    world.resources.push(crate::world::ResourceNode::new(
        ResourceType::Food,
        here,
        200,
    ));

    let mut simulation = crate::analytics::Simulation::new(world, population);
    for _ in 0..60 {
        simulation.tick();
    }

    let seen_on = simulation.population.agents[0]
        .exploration_knowledge
        .when_i_saw_it(&here)
        .expect("he is standing next to it");

    assert!(
        simulation.current_tick.saturating_sub(seen_on) <= Hearsay::STILL_ANSWERABLE_FOR,
        "a man who has been beside a patch for sixty ticks can say he just \
         passed it; his last sighting stood at {seen_on} on tick {}",
        simulation.current_tick
    );
}

/// News reaches more than one person at a time.
#[test]
fn news_reaches_everybody_within_earshot() {
    // **A seed block, not a seed.**
    //
    // Whether twelve people who wander at random fall within earshot of each
    // other is a draw, and one seed only fixes that draw until something else
    // moves what happens after it: 4,101 held for a while and stopped holding
    // the moment the country's animals were placed differently, which has
    // nothing whatever to do with talking. A claim about whether telling is
    // two-handed is a claim about the ordinary settlement, so it is asked of
    // four of them. See ISSUES_FOUND.md #132.
    let worlds = 4;
    let heard_by_more_than_one = (0..worlds)
        .filter(|world_number| widest_a_teller_reached(4_101 + world_number) > 1)
        .count();

    assert!(
        heard_by_more_than_one + 1 >= worlds as usize,
        "somebody saying where the food is should be heard by more than one \
         person in the ordinary settlement: it happened in \
         {heard_by_more_than_one} of {worlds}"
    );
}

/// The most people any one speaker was believed by, in a settlement of twelve
/// left to itself for two thousand ticks.
fn widest_a_teller_reached(seed: u64) -> usize {
    crate::core::dice::seed(seed);

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..12 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = crate::analytics::Simulation::new(world, population);
    for agent in simulation.population.agents.iter_mut() {
        agent.state.life_stage = LifeStage::Adult;
        agent.traits = crate::core::traits::TraitSet::new();
    }

    for _ in 0..2000 {
        simulation.tick();
    }

    // How many people each speaker has been believed by. Telling used to be
    // strictly two-handed, so the most any one telling could reach was one.
    let mut listeners_per_speaker: std::collections::BTreeMap<uuid::Uuid, usize> =
        std::collections::BTreeMap::new();
    for agent in simulation
        .population
        .agents
        .iter()
        .filter(|a| a.state.is_alive)
    {
        let mut heard_from: std::collections::BTreeSet<uuid::Uuid> =
            std::collections::BTreeSet::new();
        for said in agent.exploration_knowledge.who_told_me.values() {
            heard_from.insert(said.who);
        }
        for speaker in heard_from {
            *listeners_per_speaker.entry(speaker).or_insert(0) += 1;
        }
    }

    listeners_per_speaker.values().copied().max().unwrap_or(0)
}

/// An honest settlement does not end up full of accused liars.
#[test]
fn honest_agents_do_not_end_up_accused() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..25 {
        population.spawn_agent(AgentConfig::default());
    }
    // Nobody here would dream of it
    for agent in population.agents.iter_mut() {
        agent.traits.add_trait(Trait::Honest);
    }

    let mut simulation = crate::analytics::Simulation::new(world, population);
    for _ in 0..4000 {
        simulation.tick();
    }

    // Everybody who was ever accused of lying. Children born during the run
    // draw their own traits and are not necessarily honest, so the claim
    // being tested is that an *honest man* is never called a liar, not that
    // nobody in the settlement ever is.
    let honest: std::collections::BTreeSet<uuid::Uuid> = simulation
        .population
        .agents
        .iter()
        .filter(|a| a.traits.has(Trait::Honest))
        .map(|a| a.id)
        .collect();

    let wrongly_accused: Vec<uuid::Uuid> = simulation
        .population
        .agents
        .iter()
        .filter(|a| a.state.is_alive)
        .flat_map(|a| {
            a.knowledge
                .trust_ratings
                .iter()
                .filter(|(_, record)| record.wrong_count > 0)
                .map(|(who, _)| *who)
        })
        .filter(|who| honest.contains(who))
        .collect();

    assert!(
        wrongly_accused.is_empty(),
        "a man who never lies should never be called a liar, and {} of them \
         were",
        wrongly_accused.len()
    );
}

/// What an agent knows stays inside its head.
#[test]
fn nobody_carries_the_whole_map_in_their_head() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..25 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = crate::analytics::Simulation::new(world, population);
    for _ in 0..4000 {
        simulation.tick();
    }

    for agent in simulation
        .population
        .agents
        .iter()
        .filter(|a| a.state.is_alive)
    {
        assert!(
            agent.exploration_knowledge.known_resources.len()
                <= Agent::WHAT_A_MAN_CAN_HOLD_IN_MIND,
            "an agent held {} places in mind",
            agent.exploration_knowledge.known_resources.len()
        );
    }
}
