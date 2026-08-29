// src/analytics/tests/deposit_state_tests.rs
//! Tests that a report carries how much was there, not only where it is.
//!
//! Everything one agent could tell another was a place, a resource type and a
//! date. A listener already weighed the age of the claim — "a seam I passed
//! last week" against "a seam I passed this morning" — and had no way at all
//! to weigh either against "the last handful of a worked-out one". The two
//! sound identical and are worth walking to on completely different terms.
//!
//! Three things read the amount now: what a man keeps in his head, where he
//! goes when he leaves camp, and whether bare ground makes him a liar.

use crate::agents::exploration::{ExplorationKnowledge, Hearsay};
use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::Simulation;
use crate::core::SpatialMemoryType;
use crate::world::{Position, ResourceType, World, WorldConfig};

// --------------------------------------------------------------------------
// Carrying it
// --------------------------------------------------------------------------

/// Walking past a place is finding out how much is on it.
#[test]
fn seeing_a_place_again_is_seeing_how_much_is_left_of_it() {
    let mut map = ExplorationKnowledge::new();
    let seam = Position::new(20, 20);

    map.discover_resource(seam, ResourceType::Clay, 0);
    assert_eq!(
        map.how_much_was_there_then(&seam),
        None,
        "nothing is known about the amount until somebody looks"
    );

    map.saw_it_again(seam, 18, 100);
    assert_eq!(map.how_much_was_there_then(&seam), Some(18));

    // And the seam is worked down
    map.saw_it_again(seam, 2, 400);
    assert_eq!(
        map.how_much_was_there_then(&seam),
        Some(2),
        "the last thing he saw is what he remembers"
    );
}

/// And taking somebody's word for it takes their word about the amount.
#[test]
fn a_reported_place_is_remembered_at_the_size_it_was_reported() {
    let mut map = ExplorationKnowledge::new();
    let told_me = uuid::Uuid::new_v4();
    let seam = Position::new(30, 30);

    map.take_their_word_for_it(seam, ResourceType::Clay, told_me, 90, Some(2), 100);

    assert_eq!(map.how_much_was_there_then(&seam), Some(2));
    assert_eq!(
        map.who_told_me.get(&seam).and_then(|said| said.how_much_they_said),
        Some(2),
        "and it is remembered as something he said, not something seen"
    );
}

// --------------------------------------------------------------------------
// Whether bare ground makes him a liar
// --------------------------------------------------------------------------

fn said(how_much: Option<u32>) -> Hearsay {
    Hearsay {
        who: uuid::Uuid::new_v4(),
        they_saw_it_on: 1000,
        told_me_on: 1000,
        how_much_they_said: how_much,
    }
}

/// A man who says he saw the last handful of a seam this morning, and is found
/// to have told the truth about the last handful of a seam, is not a liar.
#[test]
fn reporting_a_nearly_worked_out_seam_honestly_is_not_lying() {
    let honest = said(Some(2));

    assert!(
        honest.was_he_answerable_for_it(1004),
        "his news is fresh enough to be held to, which is the point"
    );
    assert!(
        !honest.does_bare_ground_convict_him(1004, false),
        "but somebody took the handful, and that is what he said would happen"
    );
}

/// A man who says there is a seam there, and there is nothing, still is.
#[test]
fn reporting_a_rich_seam_that_is_not_there_still_convicts() {
    let claiming_plenty = said(Some(20));

    assert!(
        claiming_plenty.does_bare_ground_convict_him(1004, false),
        "nobody strips twenty units of a seam between the telling and the walk"
    );
}

/// And the excuse cannot be claimed by somebody who invented the place: a lie
/// in this model is about where, and a liar claims a place worth walking to.
#[test]
fn a_liar_cannot_hide_behind_the_excuse_made_for_an_honest_man() {
    let liar = said(Some(Population::WHAT_A_LIAR_SAYS_IS_THERE));

    assert!(
        !liar.he_did_say_it_was_nearly_gone(),
        "what a liar claims is well clear of what counts as the last of it"
    );
    assert!(liar.does_bare_ground_convict_him(1004, false));
}

/// Somebody who said nothing about the amount is judged the way he always was.
#[test]
fn saying_nothing_about_how_much_leaves_the_old_test_standing() {
    let old_news = said(None);

    assert!(old_news.does_bare_ground_convict_him(1004, false));
    assert!(
        !old_news.does_bare_ground_convict_him(9999, false),
        "and stale news still excuses him"
    );
}

// --------------------------------------------------------------------------
// What a head holds
// --------------------------------------------------------------------------

/// Two places wanted equally and known equally well: the richer one stays.
#[test]
fn a_worked_out_place_is_the_one_let_go_of() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    let rich = Position::new(10, 10);
    let bare = Position::new(11, 11);

    // Everything else about the two is the same
    for (where_it_is, how_much) in [(rich, 20), (bare, 1)] {
        agent
            .exploration_knowledge
            .discover_resource(where_it_is, ResourceType::Clay, 50);
        agent.exploration_knowledge.saw_it_again(where_it_is, how_much, 50);
    }

    // And enough other clutter to force a choice
    for filler in 0..200i32 {
        let somewhere = Position::new(40 + filler % 8, 40 + filler / 8);
        agent
            .exploration_knowledge
            .discover_resource(somewhere, ResourceType::Stone, 50);
        agent.exploration_knowledge.saw_it_again(somewhere, 10, 50);
    }

    agent.forget_what_does_not_matter(60);

    let kept = |where_it_is: &Position| {
        agent
            .exploration_knowledge
            .known_resources
            .contains_key(where_it_is)
    };

    assert!(
        kept(&rich) || !kept(&bare),
        "a head that has to drop one of them should not drop the seam and \
         keep the last handful"
    );
}

/// And forgetting a place forgets what was on it, so the book stays bounded.
#[test]
fn forgetting_a_place_forgets_how_much_was_on_it() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    for filler in 0..300i32 {
        let somewhere = Position::new(filler % 40, filler / 40);
        agent
            .exploration_knowledge
            .discover_resource(somewhere, ResourceType::Stone, 50);
        agent.exploration_knowledge.saw_it_again(somewhere, 10, 50);
    }

    agent.forget_what_does_not_matter(60);

    assert_eq!(
        agent.exploration_knowledge.how_much_was_there.len(),
        agent.exploration_knowledge.known_resources.len(),
        "the amounts book should hold exactly the places still known"
    );
}

// --------------------------------------------------------------------------
// Where he walks when he leaves — the half that is NOT shipping
// --------------------------------------------------------------------------
//
// Choosing the richest remembered place rather than the furthest is the
// obvious use for the amount and it is not here. Three arms of thirty-two
// worlds each produced one world that refused for want of water 3,092, 851 and
// 13,004 times against a baseline worst case of seven, and weighing the amount
// by how stale the memory was — the obvious guess — made the worst of them
// worse. It wants its own investigation and its own arm. See ISSUES_FOUND #68.
//
// What is tested here is that the memory now carries the amount, so that the
// investigation has something to work with.

/// A remembered place knows how much was on it.
#[test]
fn a_remembered_place_carries_how_much_was_standing_there() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    let puddle = (25, 49, 0);
    let spring = (25, 46, 0);

    agent
        .memory
        .remember_how_much_is_there(SpatialMemoryType::Water, puddle, 1);
    agent
        .memory
        .remember_how_much_is_there(SpatialMemoryType::Water, spring, 60);

    let worth = |at: (i32, i32, i32)| {
        agent
            .memory
            .spatial_memories
            .iter()
            .find(|m| m.position == at)
            .map(|m| m.value)
    };

    assert_eq!(worth(puddle), Some(1.0), "a puddle is a puddle");
    assert_eq!(
        worth(spring),
        Some(60.0),
        "and `SpatialMemory::value` was 1.0 for everything before this"
    );
}

/// And seeing it again at a different size updates it.
#[test]
fn walking_past_it_again_updates_what_is_remembered_of_it() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let agent = &mut population.agents[0];

    let waterhole = (25, 46, 0);

    agent
        .memory
        .remember_how_much_is_there(SpatialMemoryType::Water, waterhole, 60);
    agent
        .memory
        .remember_how_much_is_there(SpatialMemoryType::Water, waterhole, 4);

    let remembered: Vec<f32> = agent
        .memory
        .spatial_memories
        .iter()
        .filter(|m| m.position == waterhole)
        .map(|m| m.value)
        .collect();

    assert_eq!(
        remembered,
        vec![4.0],
        "one memory of the place, at the size he last saw it"
    );
}
