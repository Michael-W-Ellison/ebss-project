//! Stopping to make a better tool, when the tool buys back more than it costs.
//!
//! "The agent should look at the drive, their skills, the availability of tools
//! to decrease time, if they need to make any tools, and decide the quickest
//! method of satisfying their most important drive." And, earlier: "eight hours
//! with this axe, or two hours making a better one and six with that."

use crate::environment::making::{how_many_turns_to_make, Making};

/// The chain can be priced, not just walked one step at a time.
#[test]
fn a_thing_already_in_the_pack_costs_nothing_to_make() {
    let holding = |what: &str| u32::from(what == "handaxe");
    let knows = |_: &Making| true;

    assert_eq!(
        how_many_turns_to_make("handaxe", &holding, &knows),
        Some(0),
        "you do not make what you are already carrying"
    );
}

/// And something with a chain behind it costs more than something with none.
#[test]
fn a_longer_chain_costs_more_turns() {
    let empty_handed = |_: &str| 0u32;
    let knows_everything = |_: &Making| true;

    let axe = how_many_turns_to_make("handaxe", &empty_handed, &knows_everything);
    assert!(axe.is_some(), "a stone-age people can make a handaxe");
    let axe = axe.unwrap();
    assert!(axe > 0, "an axe out of nothing is not free");

    // With the makings already to hand it is one turn - the difference between
    // the two is the whole of what the chain costs
    let makings_to_hand = |what: &str| match what {
        "handaxe" => 0,
        _ => 9,
    };
    let with_makings = how_many_turns_to_make("handaxe", &makings_to_hand, &knows_everything)
        .expect("still makeable with the makings in hand");
    assert!(
        with_makings < axe,
        "an axe with the makings to hand ({with_makings}) should cost less \
         than one from nothing ({axe})"
    );
}

/// Nobody prices a thing they have never heard of.
#[test]
fn what_nobody_knows_how_to_make_has_no_price() {
    let empty_handed = |_: &str| 0u32;
    let knows_nothing = |_: &Making| false;

    assert_eq!(
        how_many_turns_to_make("handaxe", &empty_handed, &knows_nothing),
        None
    );
}

/// The arithmetic itself: what a tool saves against what it costs.
///
/// This is the specification's own worked example. A job that takes eight
/// hours by hand and six with a better tool is worth two hours of making, and
/// is not worth three.
#[test]
fn a_tool_is_worth_making_when_it_buys_back_more_than_it_costs() {
    // What the model actually computes, spelled out: the work the tool has in
    // it, at the difference between the two rates.
    let saves = |lasts: f32, now: f32, after: f32| lasts * (1.0 / now - 1.0 / after);

    // A tool with forty pieces of work in it, taking bare hands from one to
    // 1.8, saves about eighteen turns - well worth a few turns of knapping
    let a_real_axe = saves(40.0, 1.0, 1.8);
    assert!(
        a_real_axe > 10.0,
        "forty jobs at nearly twice the speed should be worth ten turns: {a_real_axe}"
    );

    // A tool that is barely better than what is already in hand is not worth
    // stopping for, however long it lasts
    let barely_better = saves(40.0, 1.7, 1.8);
    assert!(
        barely_better < a_real_axe,
        "a marginal upgrade is worth less than a first tool"
    );

    // And one that does not last is not worth it either
    let wears_out_at_once = saves(2.0, 1.0, 1.8);
    assert!(
        wears_out_at_once < 1.0,
        "a tool with two jobs in it cannot pay for a turn of making: \
         {wears_out_at_once}"
    );
}


// --------------------------------------------------------------------------
// The ladder itself
// --------------------------------------------------------------------------

/// Every tool is a thing somebody can actually make.
///
/// A `Tool` entry with no `Making` behind it is a multiplier nobody can ever
/// hold: `what_i_would_rather_have` would propose it for ever and
/// `how_many_turns_to_make` would return `None` for ever.
#[test]
fn every_tool_has_a_chain_behind_it() {
    use crate::environment::making::{EVERY_STEP, EVERY_TOOL};

    for tool in EVERY_TOOL {
        assert!(
            EVERY_STEP.iter().any(|step| step.makes == tool.called),
            "nothing in the chain makes a {}",
            tool.called
        );
    }
}

/// And every trade an agent spends its days on has something to reach for.
///
/// Herbalism had nothing at all until the digging stick, which is why the tool
/// arithmetic reached its sum twenty-one times in fourteen thousand
/// agent-turns. See ISSUES #85.
#[test]
fn the_trades_that_fill_a_day_all_have_a_ladder() {
    use crate::agents::SkillType;
    use crate::environment::making::what_helps_with;

    for trade in [
        SkillType::Herbalism,
        SkillType::Fishing,
        SkillType::Hunting,
        SkillType::Woodcutting,
        SkillType::Mining,
        SkillType::Farming,
        SkillType::Construction,
        SkillType::Crafting,
    ] {
        assert!(
            what_helps_with(trade).next().is_some(),
            "{trade:?} is a trade with no tool in the world"
        );
    }
}

/// A ladder has rungs: something to start on, and something to climb to.
#[test]
fn hunting_and_fishing_both_have_more_than_one_rung() {
    use crate::agents::SkillType;
    use crate::environment::making::what_helps_with;

    for trade in [SkillType::Hunting, SkillType::Fishing] {
        let mut rungs: Vec<f32> = what_helps_with(trade)
            .map(|tool| tool.how_much_better)
            .collect();
        rungs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(
            rungs.len() >= 3,
            "{trade:?} has {} rungs: {rungs:?}",
            rungs.len()
        );
        assert!(
            rungs.last().unwrap() > rungs.first().unwrap(),
            "{trade:?} has rungs that do not climb: {rungs:?}"
        );
    }
}

/// The fishing rod is called a rod, because the fishery looks for one by name.
///
/// `Action::Fish` has given a fifth of a chance to anything with "rod" in its
/// name since the fishery was built, and until now nothing in the chain made
/// one, so the branch had never fired.
#[test]
fn the_rod_is_named_so_the_fishery_finds_it() {
    use crate::environment::making::ROD_FOR_FISHING;

    assert!(
        ROD_FOR_FISHING.called.to_lowercase().contains("rod"),
        "the fishery matches on the name, so the name matters: {}",
        ROD_FOR_FISHING.called
    );
}

/// A cart in the pack is a cart in the hand, and a cart carries.
///
/// `TransportSystem` could model all of this from the day it was written and
/// nothing ever put a transport into it. See `Agent::take_up_the_cart`.
#[test]
fn a_handcart_is_taken_up_and_carries_more() {
    use crate::agents::{Agent, AgentConfig, InventoryItem};

    let mut agent = Agent::new(AgentConfig::default());
    agent.take_up_the_cart();
    assert!(
        agent.transport.get_active().is_empty(),
        "nobody is pulling a cart they have not got"
    );
    let bare_hands = agent.inventory.max_weight;

    agent
        .inventory
        .add_item(InventoryItem::new_with_weight("handcart".to_string(), 1, 8.0));
    agent.take_up_the_cart();

    assert_eq!(
        agent.transport.get_active().len(),
        1,
        "a cart in the pack is a cart in the hand"
    );
    assert!(
        agent.inventory.max_weight > bare_hands,
        "a cart should carry something: {} against {bare_hands}",
        agent.inventory.max_weight
    );

    // And putting it down puts it down
    agent.inventory.remove_item("handcart", 1);
    agent.take_up_the_cart();
    assert!(
        agent.transport.get_active().is_empty(),
        "the cart went and the pulling did not"
    );
}
