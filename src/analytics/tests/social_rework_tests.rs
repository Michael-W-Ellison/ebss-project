// src/analytics/tests/social_rework_tests.rs
//! What people are like decides what they are to each other and how they go
//! about things (ISSUES_FOUND #338).
//!
//! "Stop bonds saturating so traits can shape relationships. Mating should
//! only occur between agents in a relationship. Generosity and theft should be
//! tied to traits. Drive demand forces action, but traits influence the type
//! of action taken to satisfy a drive." These cover:
//! - a bond climbs towards a ceiling set by the two natures, and fades if left
//! - a grudge wears off faster for a forgiving man than a vengeful one
//! - only a pair can make a child, and pairs form from warm bonds
//! - a person's traits tilt which way they answer a want

use crate::agents::{AgentConfig, Population, Relationship, RelationshipType, Trait, TraitSet};
use crate::analytics::Simulation;
use crate::core::traits::AWay;
use crate::world::{World, WorldConfig};

fn traits(of: &[Trait]) -> TraitSet {
    let mut set = TraitSet::new();
    for what in of {
        set.add_trait(*what);
    }
    set
}

fn a_bond_between(mine: &TraitSet, theirs: &TraitSet) -> Relationship {
    let mut bond = Relationship::new(uuid::Uuid::from_u128(1), RelationshipType::Acquaintance);
    bond.affinity = Relationship::affinity_between(mine, theirs);
    bond
}

// --------------------------------------------------------------------------
// Bonds
// --------------------------------------------------------------------------

/// Like draws to like, and a clash keeps two people apart.
#[test]
fn what_two_people_are_like_sets_how_close_they_can_be() {
    let kind = traits(&[Trait::Empathic, Trait::Forgiving, Trait::Builder]);
    let same = traits(&[Trait::Empathic, Trait::Forgiving, Trait::Builder]);
    let neither = traits(&[Trait::Glutton, Trait::Goth]);
    let clash = traits(&[Trait::ColdHearted, Trait::Callous, Trait::Atheist]);

    let close = a_bond_between(&kind, &same).how_close_we_could_be();
    let middling = a_bond_between(&kind, &neither).how_close_we_could_be();
    let apart = a_bond_between(&traits(&[Trait::Believer, Trait::Empathic]), &clash)
        .how_close_we_could_be();

    assert!(close > middling, "{close} against {middling}");
    assert!(middling > apart, "{middling} against {apart}");
    assert!(apart >= Relationship::LEAST_CLOSE_PEOPLE_CAN_BE_MADE);
    assert!(close <= Relationship::CLOSEST_FRIENDS_GET);
}

/// However many kindnesses, a bond comes no nearer than the two can be, and
/// each counts for less the nearer it is.
#[test]
fn kindness_fills_the_room_a_bond_has_and_no_more() {
    let mut bond = a_bond_between(&traits(&[Trait::Glutton]), &traits(&[Trait::Goth]));
    let ceiling = bond.how_close_we_could_be();

    let before = bond.bond_strength;
    bond.strengthen(0.05);
    let first = bond.bond_strength - before;

    for _ in 0..500 {
        bond.strengthen(0.05);
    }
    assert!(bond.bond_strength <= ceiling + 1e-6, "{} past {ceiling}", bond.bond_strength);

    let near = bond.bond_strength;
    bond.strengthen(0.05);
    assert!(bond.bond_strength - near < first, "a late kindness counts for less");
}

/// Warmth left untended goes back to a face you know.
#[test]
fn a_friendship_left_alone_fades() {
    let mine = traits(&[Trait::Glutton]);
    let mut bond = a_bond_between(&mine, &traits(&[Trait::Goth]));
    bond.bond_strength = 0.6;
    for _ in 0..180 {
        bond.a_day_untended(&mine);
    }
    assert!(bond.bond_strength < 0.35, "half a year alone: {}", bond.bond_strength);
    assert!(bond.bond_strength >= Relationship::WHERE_AN_UNTENDED_BOND_SETTLES);
}

/// But blood and a pair bond are kept.
#[test]
fn family_and_partners_do_not_fade() {
    let mine = traits(&[Trait::Glutton]);
    for kind in [RelationshipType::Sibling, RelationshipType::Partner] {
        let mut bond = Relationship::new(uuid::Uuid::from_u128(2), kind);
        let was = bond.bond_strength;
        for _ in 0..60 {
            bond.a_day_untended(&mine);
        }
        assert_eq!(bond.bond_strength, was, "{kind:?}");
    }
}

/// A forgiving man lets a grudge go; a vengeful one keeps it.
#[test]
fn a_grudge_wears_off_by_temperament() {
    let after_a_month = |mine: TraitSet| {
        let mut bond = a_bond_between(&mine, &traits(&[Trait::Goth]));
        bond.bond_strength = -0.6;
        for _ in 0..30 {
            bond.a_day_untended(&mine);
        }
        bond.bond_strength
    };
    let forgiving = after_a_month(traits(&[Trait::Forgiving]));
    let ordinary = after_a_month(traits(&[Trait::Glutton]));
    let vengeful = after_a_month(traits(&[Trait::Vengeful]));
    assert!(forgiving > ordinary && ordinary > vengeful, "{forgiving} {ordinary} {vengeful}");
}

// --------------------------------------------------------------------------
// Pairs
// --------------------------------------------------------------------------

fn two_grown_people() -> Population {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    for agent in population.agents.iter_mut() {
        agent.state.position = (10, 10, 0);
    }
    population
}

fn warm_each_way(population: &mut Population, how_warm: f32) {
    let (a, b) = (population.agents[0].id, population.agents[1].id);
    for (me, them) in [(0, b), (1, a)] {
        let bond = population.agents[me].relationships.get_or_create_relationship(them, 0);
        bond.bond_strength = how_warm;
    }
}

/// "Mating should only occur between agents in a relationship."
#[test]
fn only_a_pair_can_make_a_child() {
    use crate::agents::reproduction::{can_mate, MateSelectionCriteria};
    let mut population = two_grown_people();
    let criteria = MateSelectionCriteria::default();
    let (a, b) = (population.agents[0].id, population.agents[1].id);

    assert!(
        !can_mate(&population.agents[0], &population.agents[1], &criteria),
        "two strangers who happen to be standing together"
    );

    population.agents[0].pair_with(b, 0);
    assert!(
        !can_mate(&population.agents[0], &population.agents[1], &criteria),
        "it takes both of them"
    );

    population.agents[1].pair_with(a, 0);
    assert!(population.agents[0].are_we_a_pair(b) && population.agents[1].are_we_a_pair(a));
}

/// Two people warm enough to each other pair off; two who are merely civil
/// do not.
#[test]
fn a_pair_forms_out_of_a_warm_bond() {
    let mut population = two_grown_people();
    warm_each_way(&mut population, 0.3);
    population.pair_off();
    assert!(population.agents[0].my_partner().is_none(), "civil is not enough");

    warm_each_way(&mut population, 0.6);
    population.pair_off();
    let (a, b) = (population.agents[0].id, population.agents[1].id);
    assert_eq!(population.agents[0].my_partner(), Some(b));
    assert_eq!(population.agents[1].my_partner(), Some(a));
}

/// A romantic heart asks less.
#[test]
fn a_romantic_pairs_off_sooner() {
    let mut population = two_grown_people();
    for agent in population.agents.iter_mut() {
        agent.traits = traits(&[Trait::Romantic]);
    }
    warm_each_way(&mut population, 0.45);
    population.pair_off();
    assert!(population.agents[0].my_partner().is_some());
}

/// And a pair whose partner has died is no longer a pair.
#[test]
fn a_widow_is_free_to_pair_again() {
    let mut population = two_grown_people();
    warm_each_way(&mut population, 0.6);
    population.pair_off();
    population.agents[1].state.is_alive = false;
    population.pair_off();
    assert!(population.agents[0].my_partner().is_none());
}

// --------------------------------------------------------------------------
// The way a want is answered
// --------------------------------------------------------------------------

/// A generous nature gives, a greedy one takes, an honest one does not.
#[test]
fn a_nature_tilts_the_way_it_answers_a_want() {
    let altruist = traits(&[Trait::Altruist]);
    let greedy = traits(&[Trait::Greedy]);
    let honest = traits(&[Trait::Honest]);
    let ordinary = traits(&[Trait::Glutton]);

    assert!(altruist.how_readily_i(AWay::Give) > ordinary.how_readily_i(AWay::Give));
    assert!(greedy.how_readily_i(AWay::Give) < ordinary.how_readily_i(AWay::Give));
    assert!(greedy.how_readily_i(AWay::Take) > ordinary.how_readily_i(AWay::Take));
    assert!(honest.how_readily_i(AWay::Take) < 0.5);
    assert_eq!(ordinary.how_readily_i(AWay::Talk), 1.0);
}

/// Somebody company is wanted by: a gossip tells, an extrovert talks, a kind
/// heart gives, and each still sometimes does the others.
#[test]
fn the_same_want_is_answered_differently_by_different_people() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let simulation = Simulation::new(World::new(WorldConfig::default()), population);
    let mut agent = simulation.population.agents[0].clone();

    let ways = [(AWay::Give, 1.5), (AWay::Share, 1.0), (AWay::Talk, 1.0)];
    let share_of = |agent: &crate::agents::Agent, way: AWay| {
        (0..2000)
            .filter(|_| Simulation::the_way_this_one_goes_about_it(agent, &ways) == way)
            .count() as f32
            / 2000.0
    };

    agent.traits = traits(&[Trait::Glutton]);
    let ordinary_share = share_of(&agent, AWay::Share);
    let ordinary_give = share_of(&agent, AWay::Give);

    agent.traits = traits(&[Trait::Gossip]);
    assert!(share_of(&agent, AWay::Share) > ordinary_share + 0.15);

    agent.traits = traits(&[Trait::Altruist]);
    assert!(share_of(&agent, AWay::Give) > ordinary_give + 0.15);

    agent.traits = traits(&[Trait::Cruel]);
    let cruel_give = share_of(&agent, AWay::Give);
    assert!(cruel_give < ordinary_give && cruel_give > 0.0, "{cruel_give}");
}

/// What a theft costs an honest man is far more than what it costs a
/// greedy one, among the same people.
#[test]
fn an_honest_man_counts_a_theft_dearer() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut agent = population.agents[0].clone();

    agent.traits = traits(&[Trait::Honest]);
    let honest = agent.what_taking_it_would_cost_me(2, 0.4);
    agent.traits = traits(&[Trait::Greedy]);
    let greedy = agent.what_taking_it_would_cost_me(2, 0.4);
    assert!(honest > greedy * 10.0, "{honest} against {greedy}");
}

// --------------------------------------------------------------------------
// Talk and company (#339)
// --------------------------------------------------------------------------

/// Talking makes somebody a face you know, and no more, however much is
/// said: talk counts once a day for a pair.
#[test]
fn talk_makes_a_familiar_face_and_no_more() {
    let mut bond = a_bond_between(&traits(&[Trait::Glutton]), &traits(&[Trait::Goth]));
    let a_day = crate::environment::seasons::TICKS_PER_DAY;

    let before = bond.bond_strength;
    for word in 0..50 {
        bond.have_a_word(word);
    }
    assert!(
        bond.bond_strength - before <= Relationship::WHAT_A_DAY_OF_TALK_IS_WORTH + 1e-6,
        "fifty words in a day counted {}",
        bond.bond_strength - before
    );

    for day in 1..400 {
        bond.have_a_word(day * a_day);
        bond.a_day_untended(&traits(&[Trait::Glutton]));
    }
    assert!(bond.bond_strength <= Relationship::A_FAMILIAR_FACE + 1e-6, "{}", bond.bond_strength);
    assert!(bond.bond_strength > 0.25, "talked to every day: {}", bond.bond_strength);
}

/// A good turn a day holds a bond near what the two natures allow, so two of
/// a kind come to stand nearer than two with nothing between them, and two
/// who clash further off; and left alone, any of them slides back.
#[test]
fn keeping_company_brings_a_bond_to_what_the_two_natures_allow() {
    let a_year_of_it = |mine: &[Trait], theirs: &[Trait]| {
        let mine = traits(mine);
        let mut bond = a_bond_between(&mine, &traits(theirs));
        // A good turn every four days, as often as friends sit down
        // together about camp
        for day in 0..365 {
            if day % 4 == 0 {
                bond.positive_interaction(2, day);
            }
            bond.a_day_untended(&mine);
        }
        (bond.bond_strength, bond.how_close_we_could_be(), bond)
    };

    let (kind, kind_ceiling, mut kept) =
        a_year_of_it(&[Trait::Empathic, Trait::Forgiving, Trait::Builder], &[Trait::Empathic, Trait::Forgiving, Trait::Builder]);
    let (plain, plain_ceiling, _) = a_year_of_it(&[Trait::Glutton], &[Trait::Goth]);
    let (clash, _, _) = a_year_of_it(&[Trait::Believer, Trait::Honest], &[Trait::Atheist, Trait::Dishonest]);

    assert!(kind > plain && plain > clash, "{kind} {plain} {clash}");
    assert!(kind > 0.8 * kind_ceiling, "{kind} of {kind_ceiling}");
    assert!(plain > 0.8 * plain_ceiling, "{plain} of {plain_ceiling}");
    assert!(plain >= 0.48, "company every few days makes nearly a friend: {plain}");

    for _ in 0..180 {
        kept.a_day_untended(&traits(&[Trait::Empathic]));
    }
    assert!(kept.bond_strength < 0.5, "half a year apart: {}", kept.bond_strength);
}
