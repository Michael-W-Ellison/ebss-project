// src/analytics/tests/durability_tests.rs
//! What a worn tool costs, and what it does not.
//!
//! "The intention is for tools to increase task completion speed or enable
//! task completion... The more durable (sharper) the knife, the faster the
//! gathering... Task output amount should depend on quality and technology,
//! as a better quality tool should produce less waste. Durability should only
//! apply to speed, not output amount."
//!
//! That is two channels, and this world used to have one. A single number -
//! technology, workmanship and wear multiplied together - was handed to the
//! harvest to set how much came back, to the butchery to set how much came
//! off the carcass, and to the digging to set what the hole cost, so a blunt
//! flake made a man bring home fewer berries as well as taking longer over
//! them. The two questions are separated here:
//!
//! - `how_fast_my_tools_make_this_go` - technology, workmanship, **wear**,
//!   and whether the thing is in the hand.
//! - `how_much_my_tools_bring_back` - technology and workmanship, and
//!   nothing else.

use crate::agents::skills::Quality;
use crate::agents::{Agent, AgentConfig, Population, SkillType};
use crate::environment::making::AXE_FOR_WOOD;

fn one_person() -> Population {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population
}

/// Wear the agent's axe down to this much of its life.
fn worn_to(agent: &mut Agent, left: f32) {
    let axe = agent.inventory.get_item_mut("handaxe").expect("has an axe");
    let max = axe.max_durability.expect("an axe has a life");
    axe.current_durability = Some(max * left);
}

// --------------------------------------------------------------------------
// Durability is speed, and only speed
// --------------------------------------------------------------------------

/// A blunt axe fells timber more slowly than a sharp one.
#[test]
fn a_worn_edge_makes_the_work_go_slower() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    worn_to(agent, 1.0);
    let sharp = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);

    worn_to(agent, 0.2);
    let blunt = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);

    assert!(
        blunt < sharp,
        "a blunt axe should be slower than a sharp one: {blunt} against {sharp}"
    );
    assert!(
        blunt > 1.0,
        "but a blunt axe is still an axe, and still beats bare hands"
    );
}

/// And brings home exactly as much timber while it does it.
///
/// **The rule this file exists for.** Wear is a tax on the day, not on the
/// load: the same tree yields the same wood whether the axe that felled it
/// was fresh or nearly finished.
#[test]
fn a_worn_edge_brings_back_exactly_as_much() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    worn_to(agent, 1.0);
    let sharp = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    worn_to(agent, 0.05);
    let nearly_done = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert_eq!(
        sharp, nearly_done,
        "durability applies to speed and to nothing else"
    );
}

/// The wear curve is a straight line, not a set of steps.
///
/// "A gradual decrease is preferred over a stepped decrease." A banded rule
/// would make an axe at 76% and one at 100% identical and then drop a quarter
/// of its worth between 76% and 74%. Every equal step of wear costs the same,
/// and no step costs suddenly.
#[test]
fn the_wear_curve_is_gradual_and_has_no_cliff_in_it() {
    let steps: Vec<f32> = (0..=10)
        .map(|tenth| Agent::how_much_edge_is_left(tenth as f32 / 10.0))
        .collect();

    let gaps: Vec<f32> = steps.windows(2).map(|pair| pair[1] - pair[0]).collect();

    for gap in &gaps {
        assert!(*gap > 0.0, "every tenth of wear should cost something");
        assert!(
            (gap - gaps[0]).abs() < 1e-5,
            "and cost the same as every other tenth: {gaps:?}"
        );
    }

    assert_eq!(
        Agent::how_much_edge_is_left(1.0),
        1.0,
        "a fresh edge carries the whole of what the tool is worth"
    );
    assert_eq!(
        Agent::how_much_edge_is_left(0.0),
        Agent::WHAT_A_BLUNT_EDGE_STILL_CARRIES,
        "and a blunt one still carries being the right shape at all"
    );
}

/// Wear is clamped at both ends, so a bad durability figure cannot invert the
/// rule or pay a bonus.
#[test]
fn nonsense_durability_cannot_make_a_tool_better_than_new() {
    assert_eq!(Agent::how_much_edge_is_left(2.0), 1.0);
    assert_eq!(
        Agent::how_much_edge_is_left(-1.0),
        Agent::WHAT_A_BLUNT_EDGE_STILL_CARRIES
    );
}

// --------------------------------------------------------------------------
// Yield is quality and technology
// --------------------------------------------------------------------------

/// A better-made axe wastes less of what it cuts.
#[test]
fn workmanship_decides_how_much_comes_back() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    let made_badly = |agent: &mut Agent, quality: Quality| {
        let axe = agent.inventory.get_item_mut("handaxe").expect("has an axe");
        axe.quality = Some(quality);
    };

    made_badly(agent, Quality::Crude);
    let from_a_crude_one = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    made_badly(agent, Quality::Moderate);
    let from_a_good_one = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert!(
        from_a_good_one > from_a_crude_one,
        "a better-made axe should waste less: {from_a_good_one} against \
         {from_a_crude_one}"
    );

    // And the same ordering on the other channel, because workmanship is the
    // one thing both questions agree to read.
    made_badly(agent, Quality::Crude);
    let slow = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);
    made_badly(agent, Quality::Moderate);
    let quick = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);

    assert!(quick > slow, "and work faster: {quick} against {slow}");
}

/// And a better tool beats a worse one, however each was made.
#[test]
fn technology_decides_how_much_comes_back() {
    let mut population = one_person();
    let agent = &mut population.agents[0];
    agent.inventory.remove_item("handaxe", 1);

    let bare = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);
    assert_eq!(
        bare,
        Agent::what_bare_hands_manage(SkillType::Woodcutting),
        "with nothing in hand, what comes back is what hands manage"
    );

    let axe = agent.a_tool_fresh_from_these_hands("handaxe", 1, 2.0);
    agent.inventory.add_item(axe);

    let with_an_axe = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);
    assert!(with_an_axe > bare, "an axe beats no axe");
    assert!(
        with_an_axe <= AXE_FOR_WOOD.how_much_better,
        "and no axe is worth more than the axe is worth"
    );
}

// --------------------------------------------------------------------------
// The two channels stay apart
// --------------------------------------------------------------------------

/// Quality moves both channels; wear moves only one.
///
/// Stated as one test because it is the whole specification in four numbers,
/// and because a change that quietly reconnects wear to yield would pass
/// every other test in this file.
#[test]
fn quality_tells_on_both_and_wear_tells_on_one() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    worn_to(agent, 1.0);
    let fast_fresh = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);
    let yield_fresh = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    worn_to(agent, 0.1);
    let fast_worn = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);
    let yield_worn = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert!(fast_worn < fast_fresh, "wear slows the work");
    assert_eq!(yield_worn, yield_fresh, "and leaves the load alone");
}

/// Where the axe is decides how fast the work goes, not how much comes home.
///
/// Stopping to dig a tool out of a bag costs part of the day. It does not
/// cost timber, so the pack penalty lives on the speed channel and only
/// there.
#[test]
fn an_axe_in_the_bag_is_slower_but_no_less_fruitful() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    let in_the_bag_speed = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);
    let in_the_bag_yield = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert!(agent.take_in_hand("handaxe"), "the axe can be got out");

    let in_the_hand_speed = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);
    let in_the_hand_yield = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert!(
        in_the_hand_speed > in_the_bag_speed,
        "an axe you have got out should work faster: {in_the_hand_speed} \
         against {in_the_bag_speed}"
    );
    assert_eq!(
        in_the_hand_yield, in_the_bag_yield,
        "and bring back exactly the same"
    );
}

/// A tool that is worn through is no tool on either channel.
#[test]
fn a_tool_worn_through_is_no_tool_at_all() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    worn_to(agent, 0.0);

    assert_eq!(
        agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting),
        Agent::what_bare_hands_manage(SkillType::Woodcutting),
        "a dead axe is no faster than bare hands"
    );
    assert_eq!(
        agent.how_much_my_tools_bring_back(SkillType::Woodcutting),
        Agent::what_bare_hands_manage(SkillType::Woodcutting),
        "nor more fruitful"
    );
}

/// Quality is capped, and the cap is the same on both channels.
///
/// So a Masterwork tool and a merely Good one are not two different models of
/// how work goes - they read one band, and the band is stated once.
#[test]
fn the_two_channels_read_one_opinion_of_workmanship() {
    let best_there_is = Quality::Expert.modifier();
    let worst_there_is = Quality::Pathetic.modifier();

    assert!(
        best_there_is > worst_there_is,
        "the quality ladder runs the way it reads"
    );

    let mut population = one_person();
    let agent = &mut population.agents[0];
    agent.inventory.remove_item("handaxe", 1);

    let axe = agent.a_tool_fresh_from_these_hands("handaxe", 1, 2.0);
    agent.inventory.add_item(axe);
    assert!(agent.take_in_hand("handaxe"));


    // Fresh, in the hand: the wear term and the pack term are both one, so
    // the two channels must agree exactly. Anything else means one of them
    // has picked up an opinion the other has not.
    let fast = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);
    let brought = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert!(
        (fast - brought).abs() < 1e-5,
        "a fresh tool in the hand should read the same both ways: {fast} \
         against {brought}"
    );
}

// --------------------------------------------------------------------------
// What a trip costs
// --------------------------------------------------------------------------

/// A man with nothing in his hands pays what he has always paid.
///
/// **The regression this test exists for.** Charging a gathering trip by
/// dividing its flat cost through by `how_fast_my_tools_make_this_go` looks
/// right and is not: that function answers `what_bare_hands_manage` when
/// there is no tool, which is a quarter for woodcutting, so a barehanded trip
/// silently went from ten to forty. Measured over sixty-four seeded worlds
/// that cost 2.3% of person-days and four more worlds emptied - punishing the
/// bottom of the ladder instead of rewarding the top of it.
#[test]
fn bare_hands_pay_what_bare_hands_have_always_paid() {
    use crate::analytics::Simulation;

    for trade in [
        SkillType::Woodcutting,
        SkillType::Herbalism,
        SkillType::Mining,
        SkillType::Fishing,
        SkillType::Farming,
    ] {
        let bare_hands = Agent::what_bare_hands_manage(trade);
        let share = Simulation::what_the_tool_saves_on_a_trip(trade, bare_hands);

        assert!(
            (share - 1.0).abs() < 1e-5,
            "with nothing in hand a {trade:?} trip should cost the whole of \
             what it always cost, not {share} of it"
        );
    }
}

/// A tool takes work out of the trip, and a blunt one takes less out than a
/// sharp one.
#[test]
fn a_sharper_tool_makes_a_shorter_trip() {
    use crate::analytics::Simulation;

    let mut population = one_person();
    let agent = &mut population.agents[0];

    worn_to(agent, 1.0);
    let sharp = Simulation::what_the_tool_saves_on_a_trip(
        SkillType::Woodcutting,
        agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting),
    );

    worn_to(agent, 0.15);
    let blunt = Simulation::what_the_tool_saves_on_a_trip(
        SkillType::Woodcutting,
        agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting),
    );

    let bare = Simulation::what_the_tool_saves_on_a_trip(
        SkillType::Woodcutting,
        Agent::what_bare_hands_manage(SkillType::Woodcutting),
    );

    assert!(sharp < bare, "an axe should shorten the trip");
    assert!(
        sharp <= blunt,
        "and a sharp axe should shorten it by at least as much as a blunt \
         one: {sharp} against {blunt}"
    );
    assert!(blunt < bare, "a blunt axe still beats no axe");
}

/// No tool ever makes a trip free, because the walk is most of it.
#[test]
fn no_edge_in_the_world_shortens_the_walk() {
    use crate::analytics::Simulation;

    let absurdly_good = Simulation::what_the_tool_saves_on_a_trip(SkillType::Woodcutting, 1_000.0);

    assert_eq!(
        absurdly_good,
        Simulation::WHAT_NO_TOOL_CAN_SAVE_YOU,
        "walking to the patch and back is a cost no edge touches"
    );
}
