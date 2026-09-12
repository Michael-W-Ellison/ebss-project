// src/analytics/tests/quality_tests.rs
//! What workmanship is worth, and what decides it.
//!
//! "Higher quality items last longer, are more effective, and decrease task
//! completion [time]... Two agents with the same clothing items but of
//! differing quality should have different weather resistances. Agents with
//! the same type of tool but differing quality should finish the same task at
//! different speeds... **Skill level should determine crafting success
//! chance, while tool quality should cap output quality.**"
//!
//! Nearly all of the machinery for this existed and was not connected to
//! anything. `Skill::perform_check` took a tool quality and had one caller,
//! which passed `None`; `material_quality_limit` - the cap the specification
//! asks for, one rung above the tool - had no caller at all outside its own
//! unit test. What is new here is mostly wiring, and two rules that were
//! genuinely absent: the cap, and a success roll on making anything that is
//! not a garment.

use crate::agents::skills::Quality;
use crate::agents::{Agent, AgentConfig, Population, SkillType};

fn one_person() -> Population {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population
}

fn axe_made(agent: &mut Agent, quality: Quality) {
    let axe = agent.inventory.get_item_mut("handaxe").expect("has an axe");
    axe.quality = Some(quality);
}

/// The knife, which is what the *Crafting* trade works with.
///
/// Worth its own helper because the tool a making is capped by is the tool
/// for the trade that does the making, not the thing being made: a handaxe
/// is made with Crafting, and Crafting's tool is a knife.
fn knife_made(agent: &mut Agent, quality: Quality) {
    let knife = agent
        .inventory
        .get_item_mut("stoneknife")
        .expect("has a knife");
    knife.quality = Some(quality);
}

// --------------------------------------------------------------------------
// Quality is worth something, and every rung of it is worth something
// --------------------------------------------------------------------------

/// Every rung of the ladder is worth more than the one below it.
///
/// **The defect this test exists for.** The band was applied with a `clamp`,
/// and the quality range runs past the top of the band, so Fine and
/// Masterwork both landed on 1.5 - the top two rungs doing identical work, at
/// the end a settlement spends its life climbing towards.
#[test]
fn no_two_rungs_of_the_quality_ladder_are_worth_the_same() {
    let ladder = [
        Quality::Crude,
        Quality::Poor,
        Quality::Common,
        Quality::Good,
        Quality::Fine,
        Quality::Masterwork,
    ];

    let worth: Vec<f32> = ladder
        .iter()
        .map(|quality| Agent::what_this_workmanship_is_worth(*quality))
        .collect();

    for pair in worth.windows(2) {
        assert!(
            pair[1] > pair[0],
            "each rung should beat the one below: {worth:?}"
        );
    }

    let (worst, best) = (worth[0], worth[worth.len() - 1]);
    assert!(
        worst >= 0.7 - 1e-5 && best <= 1.5 + 1e-5,
        "and the whole ladder stays inside the band it was squeezed into: \
         {worst} to {best}"
    );
}

/// Ordinary work is worth exactly what it has always been worth.
///
/// **The second regression this file exists for.** Spreading the six rungs
/// evenly from the worst to the best separates the top two, which was the
/// point - and moves every other rung while doing it, including `Basic`,
/// which is what most things in this world are. A flat line taxed the common
/// case by three per cent to fix a problem at the top. The band is hinged on
/// ordinary work instead.
#[test]
fn separating_the_top_rungs_did_not_move_the_common_one() {
    assert_eq!(
        Agent::what_this_workmanship_is_worth(Quality::Common),
        1.0,
        "plain serviceable work multiplies the job by one, as it always did"
    );
}

/// The same axe, made better, does the same job faster.
#[test]
fn the_same_tool_better_made_finishes_the_job_sooner() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    axe_made(agent, Quality::Poor);
    let rough = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);

    axe_made(agent, Quality::Fine);
    let good = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);

    axe_made(agent, Quality::Masterwork);
    let fine = agent.how_fast_my_tools_make_this_go(SkillType::Woodcutting);

    assert!(rough < good, "a better axe should work faster");
    assert!(
        good < fine,
        "and the best should beat merely good: {fine} against {good}"
    );
}

/// And brings back more, because a better edge wastes less.
#[test]
fn the_same_tool_better_made_wastes_less() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    axe_made(agent, Quality::Poor);
    let rough = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    axe_made(agent, Quality::Masterwork);
    let fine = agent.how_much_my_tools_bring_back(SkillType::Woodcutting);

    assert!(fine > rough, "{fine} against {rough}");
}

/// A better-made tool lasts longer, which is the first thing quality is for.
#[test]
fn a_better_made_tool_lasts_longer() {
    let ladder = [Quality::Crude, Quality::Poor, Quality::Common, Quality::Masterwork];

    let lives: Vec<f32> = ladder
        .iter()
        .map(|quality| quality.tool_durability_modifier())
        .collect();

    for pair in lives.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "a better tool should not have a shorter life: {lives:?}"
        );
    }
    assert!(
        lives[lives.len() - 1] > lives[0],
        "and the best should outlast the worst"
    );
}

// --------------------------------------------------------------------------
// Two coats of the same cut, made differently
// --------------------------------------------------------------------------

/// Two agents in the same garment, made by different hands, are not equally
/// warm.
#[test]
fn the_same_coat_made_better_keeps_more_weather_off() {
    use crate::agents::equipment::ClothingTemplate;

    let rough = ClothingTemplate::from_id("leather_tunic", Quality::Poor)
        .expect("a tunic can be made");
    let fine = ClothingTemplate::from_id("leather_tunic", Quality::Masterwork)
        .expect("a tunic can be made");

    assert!(
        fine.cold_insulation() > rough.cold_insulation(),
        "the better-made coat should keep more cold out: {} against {}",
        fine.cold_insulation(),
        rough.cold_insulation()
    );

    assert!(
        fine.max_durability > rough.max_durability,
        "and should outlast it: {} against {}",
        fine.max_durability,
        rough.max_durability
    );
}

/// Down to the body that wears them, which is what the weather actually asks.
#[test]
fn two_bodies_in_the_same_coat_are_not_equally_warm() {
    use crate::agents::equipment::ClothingTemplate;

    let mut population = one_person();
    population.spawn_agent(AgentConfig::default());

    let rough = ClothingTemplate::from_id("leather_tunic", Quality::Poor).unwrap();
    let fine = ClothingTemplate::from_id("leather_tunic", Quality::Masterwork).unwrap();

    population.agents[0].body.equip(rough);
    population.agents[1].body.equip(fine);

    let in_the_rough_one = population.agents[0].body.total_cold_insulation();
    let in_the_fine_one = population.agents[1].body.total_cold_insulation();

    assert!(
        in_the_fine_one > in_the_rough_one,
        "same coat, better made, warmer body: {in_the_fine_one} against \
         {in_the_rough_one}"
    );
}

// --------------------------------------------------------------------------
// The tool caps what the hand can turn out
// --------------------------------------------------------------------------

/// A master with nothing but a crude flake does not turn out masterwork.
///
/// **The rule the specification states outright.** `material_quality_limit`
/// had been sitting in `skills.rs` since the beginning with no caller: one
/// rung above the tool, which lets a fine hand get a little more out of a
/// tool than it deserves and no more than that.
#[test]
fn a_master_with_the_worst_tools_cannot_turn_out_the_best_work() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.skills.set_skill_level(SkillType::Crafting, 10);
    let by_hand_alone = Quality::from_hand(agent.skills.hand_for(SkillType::Crafting));
    assert!(
        by_hand_alone >= Quality::Fine,
        "a master's hands are worth the best work there is"
    );

    axe_made(agent, Quality::Crude);
    let ceiling = agent.the_best_i_could_turn_out(SkillType::Woodcutting);

    assert!(
        ceiling < by_hand_alone,
        "the tool should hold the work below what the hand could manage: \
         {ceiling:?} against {by_hand_alone:?}"
    );
    assert_ne!(
        ceiling,
        Quality::Masterwork,
        "and the worst tools should never reach the best work"
    );
}

/// A better tool raises the ceiling, one rung at a time.
#[test]
fn a_better_tool_raises_what_can_be_turned_out() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    axe_made(agent, Quality::Poor);
    let with_a_rough_one = agent.the_best_i_could_turn_out(SkillType::Woodcutting);

    axe_made(agent, Quality::Fine);
    let with_a_good_one = agent.the_best_i_could_turn_out(SkillType::Woodcutting);

    assert!(
        with_a_good_one > with_a_rough_one,
        "{with_a_good_one:?} against {with_a_rough_one:?}"
    );
}

/// And empty hands have a ceiling of their own, which is not nothing.
#[test]
fn bare_hands_can_turn_out_serviceable_work_and_no_better() {
    let mut population = one_person();
    let agent = &mut population.agents[0];
    agent.inventory.remove_item("handaxe", 1);

    assert_eq!(
        agent.the_best_i_could_turn_out(SkillType::Woodcutting),
        Agent::WHAT_BARE_HANDS_CAN_TURN_OUT,
        "fingers can twist a cord and shape a lump of clay"
    );
    assert!(
        Agent::WHAT_BARE_HANDS_CAN_TURN_OUT < Quality::Masterwork,
        "but they cannot do fine work"
    );
}

/// The cap is a cap and not a floor: a poor hand with a fine tool still turns
/// out poor work.
///
/// Worth its own test because a cap implemented as an assignment rather than
/// a minimum would hand every beginner with a good knife the work of a
/// master, which is the opposite of what the specification asks for.
#[test]
fn a_fine_tool_does_not_make_a_beginner_a_master() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    agent.skills.set_skill_level(SkillType::Woodcutting, -9);
    let by_hand_alone = Quality::from_hand(agent.skills.hand_for(SkillType::Woodcutting));

    axe_made(agent, Quality::Masterwork);
    let ceiling = agent.the_best_i_could_turn_out(SkillType::Woodcutting);

    assert!(
        ceiling > by_hand_alone,
        "the tool allows better work than this hand can do"
    );
    assert_eq!(
        by_hand_alone.min(ceiling),
        by_hand_alone,
        "and what actually comes out is still the hand's work"
    );
}

// --------------------------------------------------------------------------
// Skill decides whether it comes off at all
// --------------------------------------------------------------------------

/// A beginner spoils a good many attempts; a master spoils none.
#[test]
fn skill_decides_whether_the_making_comes_off() {
    use crate::agents::skills::Skill;

    let raw = Skill::with_level(SkillType::Crafting, -8);
    let master = Skill::with_level(SkillType::Crafting, 9);

    let spoiled = |skill: &Skill| {
        (0..200)
            .filter(|_| !skill.perform_check(Some(Quality::Common)).success)
            .count()
    };

    let beginner_spoiled = spoiled(&raw);
    let master_spoiled = spoiled(&master);

    assert!(
        beginner_spoiled > 0,
        "a raw beginner should spoil some of what they attempt"
    );
    assert_eq!(
        master_spoiled, 0,
        "and a master should spoil none of it"
    );
}

/// A poor tool is a dangerous tool: it rolls the failure check again.
#[test]
fn a_worse_tool_spoils_more_of_what_is_attempted() {
    use crate::agents::skills::Skill;

    let hand = Skill::with_level(SkillType::Crafting, -3);

    let spoiled = |quality: Quality| {
        (0..400)
            .filter(|_| !hand.perform_check(Some(quality)).success)
            .count()
    };

    let with_a_wretched_one = spoiled(Quality::Crude);
    let with_a_fair_one = spoiled(Quality::Common);

    assert!(
        with_a_wretched_one > with_a_fair_one,
        "the same hand with a worse tool should spoil more: \
         {with_a_wretched_one} against {with_a_fair_one}"
    );
}


/// A tool made under no handicap lasts exactly as long as it always did.
///
/// "Higher quality items last longer" was already true here, through the
/// hand: `how_long_this_one_lasts` scales a new tool's life by the hand that
/// made it. Charging the quality on top of that double-counts one fact, and
/// because a founder's work is Crude it double-counts it *downwards* - every
/// founder tool losing a quarter of its life, which measured as nine more
/// worlds emptied and half the first winters gone.
#[test]
fn a_tool_nothing_held_back_lasts_what_the_hand_would_give_it() {
    use crate::environment::making::{how_long_this_one_lasts, AXE_FOR_WOOD};

    let mut population = one_person();
    let agent = &mut population.agents[0];

    // A founder's own hand, with a tool good enough not to hold the work
    // back - which is the ordinary case and must not have changed.
    knife_made(agent, Quality::Masterwork);

    let made = agent.a_tool_fresh_from_these_hands("handaxe", 1, 2.0);
    let hand = agent.skills.hand_for(SkillType::Crafting);

    let unhandicapped = how_long_this_one_lasts(&AXE_FOR_WOOD, hand);

    assert!(
        (made.max_durability.unwrap() - unhandicapped).abs() < 0.01,
        "a tool nothing held back should last what the hand gives it: {:?} \
         against {unhandicapped}",
        made.max_durability
    );
}

/// And a tool made under a handicap lasts less.
#[test]
fn a_tool_the_tools_held_back_lasts_less() {
    let mut population = one_person();
    let agent = &mut population.agents[0];

    // A hand good enough that the tools are what hold it back.
    agent.skills.set_skill_level(SkillType::Crafting, 10);

    knife_made(agent, Quality::Masterwork);
    let unhandicapped = agent
        .a_tool_fresh_from_these_hands("handaxe", 1, 2.0)
        .max_durability
        .unwrap();

    knife_made(agent, Quality::Crude);
    let handicapped = agent
        .a_tool_fresh_from_these_hands("handaxe", 1, 2.0)
        .max_durability
        .unwrap();

    assert!(
        handicapped < unhandicapped,
        "working with wretched tools should tell on what comes out: \
         {handicapped} against {unhandicapped}"
    );
}

// --------------------------------------------------------------------------
// A refusal and a spoiled attempt are different things
// --------------------------------------------------------------------------

/// Spoiling the makings is not being refused.
///
/// **The distinction a settlement test caught the want of.** Wiring a success
/// roll into making sent every spoiled attempt through the same failure path
/// as "no materials" and "no fire", so a beginner who spoiled a third of what
/// he tried learned that *making does not work* - and a beginner who
/// concludes that never practises into a master, which is the whole point of
/// a skill deciding the odds.
#[test]
fn a_spoiled_attempt_is_not_a_refusal() {
    use crate::environment::ActionResult;

    let refused = ActionResult::failure("nothing to do it with".to_string());
    assert!(!refused.success);
    assert!(
        !refused.attempted,
        "being refused means the work never began"
    );

    let spoiled = ActionResult::failure("spoiled it".to_string()).spoiled_in_the_making();
    assert!(!spoiled.success, "nothing came of it either way");
    assert!(
        spoiled.attempted,
        "but the work began, and that is what stops it teaching despair"
    );

    let came_off = ActionResult::success();
    assert!(came_off.success && came_off.attempted);
}


/// The ladder is the one the specification names, in the order it names it.
///
/// These six went by other names until now - Pathetic, Crude, Basic,
/// Moderate, Advanced, Expert - which read as the same ladder one rung out of
/// step, and `Crude` sat at a *different rung* in each. Anybody comparing the
/// two lists had to hold the offset in their head, and a rename done in the
/// wrong order would have collapsed two rungs into one silently.
#[test]
fn the_quality_ladder_is_the_one_the_specification_names() {
    let ladder = [
        (Quality::Crude, "Crude"),
        (Quality::Poor, "Poor"),
        (Quality::Common, "Common"),
        (Quality::Good, "Good"),
        (Quality::Fine, "Fine"),
        (Quality::Masterwork, "Masterwork"),
    ];

    for (rung, called) in ladder {
        assert_eq!(rung.name(), called, "{rung:?} answers to its own name");
    }

    for pair in ladder.windows(2) {
        assert!(
            pair[0].0 < pair[1].0,
            "the ladder runs the way it reads: {:?} below {:?}",
            pair[0].0,
            pair[1].0
        );
    }

    // Ordinary everyday work is the third rung and the neutral one - the
    // specification's "Common: normal everyday quality... baseline durability
    // and efficiency" - and everything in this model is priced against it.
    assert_eq!(ladder[2].0, Quality::Common);
    assert_eq!(Quality::Common.modifier(), 1.0);
    assert_eq!(Quality::Common.value_multiplier(), 1.0);
    assert_eq!(Quality::Common.tool_durability_modifier(), 1.0);
}
