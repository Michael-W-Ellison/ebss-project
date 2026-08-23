// src/analytics/tests/personality_tests.rs
//! Tests that people are people rather than copies.
//!
//! The project is for emergent social behaviour out of drives and personality.
//! The drives were live and reading the world; the personality half was not
//! running at all. `Agent::new` set an empty `TraitSet`, and the only
//! `add_trait` on any live path was the congenital infertility roll, so no
//! agent in a running world held one of the sixty-odd defined traits.
//! Inheritance had been written and was already being called - it simply had
//! nothing to inherit, because the founding generation had nothing.
//!
//! Everything downstream had an input that never varied: the trait-to-job
//! affinities, the gossip distortion, the affinity model that decides who gets
//! on with whom, the emotional modifiers, the religious effects. A settlement
//! of eighty people was eighty copies of the same person.

use crate::agents::{Agent, AgentConfig, Population};
use crate::core::traits::{Trait, TraitSet};

/// Somebody entering a world, which is where a personality is drawn.
///
/// `Agent::new` deliberately does not draw one - it builds a body, and several
/// dozen tests of other machinery rely on a bare `Agent::new` being the same
/// agent every time. Who that body turns out to be is settled by
/// `Population::spawn_agent` for founders and by inheritance for everybody
/// born afterwards.
fn a_person() -> Agent {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.agents.pop().expect("spawn_agent adds one")
}

/// Everybody is somebody.
#[test]
fn a_new_person_has_a_personality() {
    for _ in 0..50 {
        let agent = a_person();
        let held = agent.traits.get_traits().len();

        assert!(
            held >= *TraitSet::TRAITS_AT_BIRTH.start(),
            "an agent should be drawn with at least {} traits, not {held}",
            TraitSet::TRAITS_AT_BIRTH.start()
        );
    }
}

/// And not the same somebody as everybody else.
#[test]
fn a_settlement_is_not_one_person_repeated() {
    let mut population = Population::new();
    for _ in 0..40 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut seen: std::collections::HashSet<Trait> = std::collections::HashSet::new();
    for agent in &population.agents {
        for held in agent.traits.get_traits() {
            seen.insert(*held);
        }
    }

    assert!(
        seen.len() > 25,
        "forty people drawing three to five traits each out of sixty-odd should \
         between them hold a good spread; they held {}",
        seen.len()
    );

    // And no two of them should be the same person
    let mut fingerprints: std::collections::HashSet<Vec<String>> =
        std::collections::HashSet::new();
    for agent in &population.agents {
        let mut names: Vec<String> = agent
            .traits
            .get_traits()
            .iter()
            .map(|held| format!("{held:?}"))
            .collect();
        names.sort();
        fingerprints.insert(names);
    }

    assert!(
        fingerprints.len() >= population.agents.len() - 1,
        "forty people should be forty different people; only {} distinct \
         personalities came out",
        fingerprints.len()
    );
}

/// Nobody is a walking contradiction.
#[test]
fn a_person_does_not_hold_two_opposite_traits() {
    for _ in 0..100 {
        let agent = a_person();
        let held = agent.traits.get_traits();

        for (i, one) in held.iter().enumerate() {
            for other in &held[i + 1..] {
                assert!(
                    !one.is_incompatible_with(other),
                    "nobody is both {one:?} and {other:?}"
                );
            }
        }
    }
}

/// Being drawn with a personality is not the same as being drawn with every
/// personality: a set of the wanted size, not a bundle of all of them.
#[test]
fn a_personality_is_a_few_things_not_everything() {
    for _ in 0..50 {
        let agent = a_person();
        let held = agent.traits.get_traits().len();

        // The ordinary draw is bounded; the rare congenital rolls can add up
        // to three more on top of it
        assert!(
            held <= TraitSet::TRAITS_AT_BIRTH.end() + 3,
            "a person is a few tendencies, not sixty; this one had {held}"
        );
    }
}

/// Nobody founds a settlement blind, but people are born blind into one.
///
/// Blindness, deafness and muteness are left out of the founder pool and left
/// in the pool inheritance mutates from, so they arise in a people over
/// generations rather than in the handful who walked into the country. That is
/// where congenital infertility already sat, and it is the same reasoning:
/// these are things somebody is born with, and founders are the one group in
/// the model nobody was born into. It also keeps a settlement recognisable -
/// one founder in fifteen unable to see is not a settlement anybody would know.
#[test]
fn disability_arrives_by_birth_rather_than_by_founding() {
    use crate::agents::reproduction::reproduce;

    const FOUNDERS: usize = 400;
    let unable = (0..FOUNDERS)
        .map(|_| a_person())
        .filter(|agent| {
            agent.traits.has(Trait::Blind)
                || agent.traits.has(Trait::Deaf)
                || agent.traits.has(Trait::Mute)
        })
        .count();

    assert_eq!(
        unable, 0,
        "no founder should be drawn blind, deaf or mute; {unable} of {FOUNDERS} were"
    );

    // But it reaches a settlement's children, through the mutation in
    // inheritance, and rarely
    let mut mother = Agent::new(AgentConfig::default());
    let mut father = Agent::new(AgentConfig::default());
    for held in [Trait::Bookworm, Trait::Explorer, Trait::Builder, Trait::Frugal] {
        mother.traits.add_trait(held);
        father.traits.add_trait(held);
    }

    const CHILDREN: usize = 1_500;
    let born_unable = (0..CHILDREN)
        .map(|_| reproduce(&mother, &father, 1_000))
        .filter(|child| {
            child.traits.has(Trait::Blind)
                || child.traits.has(Trait::Deaf)
                || child.traits.has(Trait::Mute)
        })
        .count();

    assert!(
        born_unable > 0,
        "a settlement's children should sometimes be born unable to see or \
         hear; none of {CHILDREN} were"
    );
    assert!(
        born_unable < CHILDREN / 10,
        "{born_unable} of {CHILDREN} is far too many"
    );
}

/// A blind agent is actually blind: the traits reach the senses at spawn, not
/// only when somebody remembers to call the function afterwards.
#[test]
fn what_a_person_is_born_as_reaches_their_senses() {
    let mut agent = Agent::new(AgentConfig::default());
    let sighted = agent.senses.vision.acuity;

    agent.traits.add_trait(Trait::Blind);
    agent.apply_trait_sensory_modifications();

    assert!(
        agent.senses.vision.acuity < sighted,
        "a blind agent should not see as a sighted one does"
    );
    assert_eq!(agent.senses.vision.acuity, 0.0);

    // And somebody drawn blind at spawn is blind from the start, without
    // anybody having to remember to call that afterwards
    let born_blind = (0..2_000)
        .map(|_| a_person())
        .find(|agent| agent.traits.has(Trait::Blind));

    if let Some(born_blind) = born_blind {
        assert_eq!(
            born_blind.senses.vision.acuity, 0.0,
            "somebody born blind should not see from their first tick"
        );
    }
}

/// A child takes after its parents.
#[test]
fn a_child_takes_after_its_parents() {
    use crate::agents::reproduction::reproduce;

    let mut mother = Agent::new(AgentConfig::default());
    let mut father = Agent::new(AgentConfig::default());

    mother.traits.add_trait(Trait::Bookworm);
    mother.traits.add_trait(Trait::Explorer);
    father.traits.add_trait(Trait::Builder);
    father.traits.add_trait(Trait::Frugal);

    let from_the_parents = [
        Trait::Bookworm,
        Trait::Explorer,
        Trait::Builder,
        Trait::Frugal,
    ];

    // Inheritance is half of each parent's traits with a chance of mutation,
    // so no single child settles this - but across a hundred, most of what
    // turns up should have come from the two of them
    let mut inherited = 0;
    let mut from_elsewhere = 0;

    for _ in 0..100 {
        let child = reproduce(&mother, &father, 1_000);
        for held in child.traits.get_traits() {
            if from_the_parents.contains(held) {
                inherited += 1;
            } else {
                from_elsewhere += 1;
            }
        }
    }

    assert!(
        inherited > 0,
        "a child should be able to take after its parents at all"
    );
    assert!(
        inherited > from_elsewhere,
        "most of what a child is should come from its parents rather than from \
         nowhere: {inherited} inherited against {from_elsewhere} from elsewhere"
    );
}

/// What a child is born with survives what it is born to.
///
/// `give_birth_internal` used to assign straight over the offspring's traits
/// after `with_parents` had already rolled congenital infertility, so the roll
/// was thrown away on every live birth - the one trait anything in the running
/// simulation ever assigned, and it never once survived.
#[test]
fn a_congenital_trait_survives_inheritance() {
    use crate::agents::reproduction::reproduce;

    let mother = Agent::new(AgentConfig::default());
    let father = Agent::new(AgentConfig::default());

    // Neither parent is infertile, so any infertile child got it congenitally
    // rather than by inheritance. At 1.5% a run of four hundred should turn up
    // several.
    let born_infertile = (0..400)
        .filter(|_| {
            reproduce(&mother, &father, 1_000)
                .traits
                .has(Trait::Infertile)
        })
        .count();

    assert!(
        born_infertile > 0,
        "congenital infertility should reach a born child; none of four \
         hundred had it"
    );
}
