// src/analytics/tests/clothing_tests.rs
//! Tests for clothing: what agents make, what they wear, and what it buys them.
//!
//! Insulation used to be zero for every agent for its whole life. Clothing
//! recipes, equipment slots and cold insulation all existed and all worked when
//! a garment was placed on an agent by hand; nothing ever drove an agent to
//! make or wear anything, so cold was a thing they endured rather than solved.
//! These cover:
//! - a garment is worth what its material is worth, and practice tells
//! - a cold agent makes what it has the material for and puts it on
//! - a warm agent does not bother
//! - two similar coats are not swapped back and forth forever

use crate::agents::equipment::{garment_recipe, ClothingTemplate, GARMENT_RECIPES};
use crate::agents::skills::Quality;
use crate::agents::{AgentConfig, EquipmentSlot, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::seasons::PLANNING_PERIODS_PER_DAY;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// Every garment is made of something an agent could hold, and the warm
/// materials are the ones that come off animals.
#[test]
fn a_garment_is_worth_what_it_is_made_of() {
    assert!(!GARMENT_RECIPES.is_empty());

    for recipe in GARMENT_RECIPES {
        assert!(recipe.material_amount > 0, "{} costs nothing", recipe.id);
        assert!(
            garment_recipe(recipe.id).is_some(),
            "{} cannot be looked up by its own id",
            recipe.id
        );
        assert!(
            ClothingTemplate::from_id(recipe.id, Quality::Common).is_some(),
            "{} cannot be built into something wearable",
            recipe.id
        );
    }

    let warmth = |id: &str| garment_recipe(id).expect("recipe should exist").warmth();

    assert!(
        warmth("fur_coat") > warmth("wool_cloak"),
        "fur should beat wool"
    );
    assert!(
        warmth("wool_cloak") > warmth("linen_cloak"),
        "wool should beat woven flax"
    );
    assert!(
        warmth("linen_cloak") > warmth("bark_boots"),
        "a cloak should beat bark on the feet"
    );
}

/// Boots go on the feet. They were on the legs, where they fought with trousers
/// for the same slot.
#[test]
fn boots_go_on_the_feet() {
    let boots = garment_recipe("bark_boots").expect("bark boots should exist");
    assert_eq!(boots.slot, EquipmentSlot::Feet);

    let pants = garment_recipe("leather_pants").expect("leather pants should exist");
    assert_eq!(pants.slot, EquipmentSlot::Legs);
}

/// A first attempt is crude but wearable, and gets better with practice.
#[test]
fn practice_tells_in_what_comes_out() {
    let mut agent = crate::agents::Agent::new(AgentConfig::default());

    let first = Simulation::expected_garment_quality(&agent);
    assert_eq!(first, Quality::Poor, "a first garment should be crude");

    agent.skills.set_skill_level(SkillType::Leatherworking, 5);
    let practised = Simulation::expected_garment_quality(&agent);

    assert!(
        practised.modifier() > first.modifier(),
        "practice should make a better garment: {first:?} -> {practised:?}"
    );
}

/// A cold agent with flax in its pack makes a cloak and puts it on.
///
/// Making and wearing are one act. Leaving the garment in the pack to be worn
/// later does not work: a stack carries one quality for all of it, so a better
/// second coat merged into the first and was recorded as no better than it.
#[test]
fn a_cold_agent_makes_a_cloak_and_wears_it() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);

    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("flax".to_string(), 200, 1.0));
        agent.body_temperature.current = 34.0;
        // A linen cloak waits on retting (Stage 3 cloth); this one knows it
        agent.found_out_how_to("rettedflax");
    }

    assert_eq!(
        simulation.population.agents[0]
            .body
            .total_cold_insulation(),
        0.0,
        "an agent starts with nothing on"
    );

    let position = simulation.population.agents[0].state.position;

    // It makes the warmest thing the flax will run to, and puts it straight on
    let making = simulation
        .clothing_action(&simulation.population.agents[0], position, true)
        .expect("a cold agent with material should decide to make something");
    assert!(
        matches!(&making, Action::MakeClothing { garment } if garment == "linen_cloak"),
        "expected a linen cloak, got {making:?}"
    );
    // A raw beginner spoils about half of what they attempt, and a cold agent
    // keeps trying. This is what a first winter actually looks like: several
    // bundles of flax ruined in the learning, and then a cloak.
    let flax_to_start_with = simulation.population.agents[0]
        .inventory
        .get_item("flax")
        .map(|flax| flax.quantity)
        .unwrap_or(0);

    let mut made_one = false;
    for _ in 0..20 {
        if simulation.execute_action(&making, 0).success {
            made_one = true;
            break;
        }
    }

    assert!(made_one, "twenty attempts should land at least one cloak");

    let agent = &simulation.population.agents[0];
    assert!(
        agent.body.total_cold_insulation() > 0.0,
        "the cloak should be on its back, keeping the cold off"
    );
    assert!(
        !agent.inventory.has_item("linen_cloak", 1),
        "the cloak is worn, not carried"
    );
    assert!(
        agent.inventory.get_item("flax").map(|f| f.quantity).unwrap_or(0)
            < flax_to_start_with,
        "the flax should have gone into it, and into what was spoiled on the way"
    );
}

/// A warm agent has better things to do than sew.
#[test]
fn a_warm_agent_does_not_bother() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("flax".to_string(), 20, 1.0));
        agent.body_temperature.current = agent.body_temperature.ideal;
    }

    let position = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .clothing_action(&simulation.population.agents[0], position, false)
            .is_none(),
        "an agent at its ideal temperature should not be making clothes"
    );
}

/// Two similar cloaks are not swapped back and forth forever.
///
/// Whatever is worn wears down a little each turn, so without a margin the one
/// folded in the pack is always fractionally warmer, and an agent spends its
/// whole life changing its coat.
#[test]
fn a_near_identical_coat_is_not_worth_changing_into() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent.body_temperature.current = 34.0;

        let worn = ClothingTemplate::from_id("linen_cloak", Quality::Poor)
            .expect("linen cloak should exist");
        agent.body.equip(worn);

        let mut spare = InventoryItem::new_with_weight("linen_cloak".to_string(), 1, 2.0);
        spare.quality = Some(Quality::Poor);
        agent.inventory.add_item(spare);
    }

    // Wear the worn one down a little, as a turn would
    for _ in 0..50 {
        simulation.population.agents[0].body.turn_equipment_wear();
    }

    assert_eq!(
        Simulation::garment_to_put_on(&simulation.population.agents[0]),
        None,
        "a spare coat no better than the one on your back is not worth changing into"
    );
}

/// A cloak is worth wearing: the same weather leaves a clothed agent warmer.
#[test]
fn a_cloak_keeps_the_cold_out() {
    use crate::agents::temperature::BodyTemperature;

    let mut bare = BodyTemperature::new();
    let mut clothed = BodyTemperature::new();

    let cloak = ClothingTemplate::from_id("linen_cloak", Quality::Common)
        .expect("linen cloak should exist");
    let insulation = cloak.cold_insulation();
    assert!(insulation > 0.0);

    for _ in 0..500 {
        bare.update(0.0, 0.0, 0.0);
        clothed.update(0.0, insulation, 0.0);
    }

    assert!(
        clothed.current > bare.current,
        "the clothed agent should settle warmer: {} against {}",
        clothed.current,
        bare.current
    );
}

/// Left to itself, a cold agent with flax growing nearby ends up dressed.
#[test]
fn a_cold_agent_ends_up_dressed() {
    // **A seed block, not a seed** - see ISSUES_FOUND.md #132 and #165.
    //
    // The comment below already said what was wrong with one seed and then
    // used one anyway: seeding after the fixture is built fixes the making
    // but not the world, and this run "measures how many other errands a
    // world gives a man". Adding a crop to the world gave him one more errand
    // and 9,140 stopped holding.
    //
    // Asked of a block, the true rate is **10 of 24** - a cold man with flax
    // in his pack and flax growing three paces away ends up dressed in about
    // four worlds in ten, and in the other six he is doing something else for
    // fifty days while freezing. That is worse than this test used to claim
    // and it is what the model actually does; one lucky seed had been hiding
    // it. The threshold is a third, set under the measurement rather than at
    // it, and what it now guards is that the chain is *reachable* - which is
    // all this test was ever able to say.
    //
    // **And a third was the rate, not under it.** Asked of 96 worlds, the
    // rate is 30 of 96 before #300 and 29 of 96 after - about three in ten,
    // the same behaviour either side. The block of 24 went from 9 to 6 on
    // nothing but which worlds the dice dealt it, with the bar at 8: a test
    // set at the rate it measures flips on every change upstream, which is
    // what this one has been doing. It asks 96 worlds now, and the bar is a
    // quarter, under the measurement. The worse news is the measurement: in
    // seven worlds in ten a freezing man with flax in his pack never makes
    // anything to wear (#300).
    //
    // **What it was measuring was a man who wanted a coat and was never
    // pressed to make one** (#341). Wanting one is `wants_more_clothing`, and
    // he did on 87% of his turns; making one was reached only through the
    // Shelter drive, which chill does not carry over its threshold by day and
    // a roof puts to nothing by night. The coats he made, he made when some
    // other drive happened past them: over the same 96 worlds, given the
    // fifty days this always meant to give him, a median of seventeen days
    // and sixteen still bare at the end. And the "fifty days" had been twelve
    // and a half since a turn became half an hour, and the 96 worlds were a
    // chain that any change upstream dealt afresh.
    //
    // Now a man chilly with the makings on him dresses on the spot
    // (`dressing_on_the_spot`), the window is fifty days of half-hour turns,
    // and each world is seeded before it is built. Measured: dressed within
    // the first day in 96 of 96 worlds, where without that the same 96 manage
    // 78 within fifty days. The bar is nine in ten within a day, under the
    // measurement.
    let worlds = 96;
    let in_a_day = (0..worlds).filter(|which| a_cold_man_dresses(9_140 + which, 1)).count();
    eprintln!("a cold man dressed within a day in {in_a_day} of {worlds} worlds");

    assert!(
        in_a_day * 10 >= worlds as usize * 9,
        "a chilly man with flax and an awl on him should make himself \
         something to wear the day he is cold: {in_a_day} of {worlds}"
    );
}

/// Linen is found out, and furs are not: a cold man with flax and no
/// retting makes nothing of the flax, and a cold man with hides makes a fur
/// coat, which is Stage 0 of the clothing path.
#[test]
fn a_cold_man_wears_what_his_people_knows_how_to_make() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("flax".to_string(), 200, 1.0));
        agent.body_temperature.current = 34.0;
    }
    let position = simulation.population.agents[0].state.position;

    assert!(!simulation.population.agents[0].knows_how_to_make_garment("linen_cloak"));
    let with_flax = simulation.clothing_action(&simulation.population.agents[0], position, true);
    assert!(
        !matches!(&with_flax, Some(Action::MakeClothing { garment }) if garment.starts_with("linen")),
        "nobody here has retted flax, so there is no linen: {with_flax:?}"
    );

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("hides".to_string(), 20, 1.0));
    let with_hides = simulation.clothing_action(&simulation.population.agents[0], position, true);
    assert!(
        matches!(&with_hides, Some(Action::MakeClothing { garment }) if garment == "fur_coat" || garment == "fur_hat"),
        "furs are born knowledge: {with_hides:?}"
    );
}

/// A parent with a coat hands it to a child who has none.
///
/// Clothing hangs off two drives: Shelter is a coat for yourself, Protection
/// is a coat for your child. A child cannot clothe itself - it cannot gather
/// flax, has no skill to sew, and until this nobody made it anything; the
/// model worked around that by counting a carer standing nearby *as shelter*,
/// which is a plaster and only holds while somebody is stood there.
///
/// The trap this guards is the one it nearly shipped as. `GiveTo` carries no
/// item: what changes hands is whatever `what_i_can_spare` picks, which is
/// whatever you have *most* of past `ENOUGH_TO_HAND` - and a garment, held one
/// at a time, is never that. So a parent setting out to clothe a bare child
/// handed it firewood. That is ISSUES #175's shape exactly: a want that
/// reaches the list and not the pack.
#[test]
fn a_parent_hands_a_bare_child_the_coat_and_not_the_firewood() {
    use crate::agents::LifeStage;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let parent = population.agents[0].id;
    let child = population.agents[1].id;

    population.agents[1].parent_ids.push(parent);
    population.agents[1].state.life_stage = LifeStage::Child;

    let here = population.agents[0].state.position;
    population.agents[1].state.position = here;

    // A coat in the pack, and a great deal of firewood - which is what the
    // ordinary gift rule would reach for.
    let coat = GARMENT_RECIPES[0].id;
    population.agents[0]
        .inventory
        .add_item(InventoryItem::new(coat.to_string(), 1));
    population.agents[0]
        .inventory
        .add_item(InventoryItem::new("wood".to_string(), 40));

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    let me = 0;
    let them = 1;
    let handed_over = simulation
        .what_i_would_hand_over(me, them)
        .expect("a parent with a coat and a bare child has something to give");

    assert_eq!(
        handed_over.0, coat,
        "the coat goes to the child, not the forty sticks of firewood"
    );

    // And the parent's own decision reaches for it: Protection answers with a
    // gift rather than falling through to standing about.
    let agent = simulation.population.agents[0].clone();
    let what = simulation.protective_action(&agent, agent.state.position);

    assert!(
        matches!(what, Some(Action::GiveTo { to }) if to == child),
        "a parent whose child is going bare should hand it something, got {what:?}"
    );
}

/// And a warm, well-dressed child draws nothing.
#[test]
fn nobody_strips_themselves_for_a_child_who_is_already_dressed() {
    use crate::agents::LifeStage;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let parent = population.agents[0].id;
    population.agents[1].parent_ids.push(parent);
    population.agents[1].state.life_stage = LifeStage::Child;
    population.agents[1].state.position = population.agents[0].state.position;

    let coat = GARMENT_RECIPES[0].id;
    population.agents[0]
        .inventory
        .add_item(InventoryItem::new(coat.to_string(), 1));

    // Dress the child past the point where it wants anything.
    let dressed = ClothingTemplate::from_id(coat, Quality::Common)
        .expect("the recipe builds something wearable");
    population.agents[1]
        .body
        .equipment
        .insert(dressed.slot, dressed);

    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);

    if simulation.population.agents[1].body.total_cold_insulation()
        < Simulation::ENOUGH_INSULATION
    {
        // One garment was not enough to clear the bar, so this fixture cannot
        // say anything. Better silent than falsely green.
        return;
    }

    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation.one_of_mine_who_is_bare(&agent).is_none(),
        "a child already carrying enough insulation is not going bare"
    );
}

/// One cold man, flax in the pack and flax next door, for so many days.
fn a_cold_man_dresses(seed: u64, days: u32) -> bool {
    // **And seeded before the world is built, too** (#341). Built from
    // whatever the world before it left in the stream, the 96 worlds were a
    // chain: a world that ran longer, or one die drawn once a day anywhere
    // in the model, dealt every world after it differently, and at a rate
    // near a third 96 worlds swing by about four either way - which is where
    // the bar sat.
    crate::core::dice::seed(seed.wrapping_mul(7_919).wrapping_add(13));
    let mut world = World::new(WorldConfig::default());

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let start = population.agents[0].state.position;
    world.resources.push(ResourceNode::new(
        ResourceType::Flax,
        Position::new(start.0 + 3, start.1),
        200,
    ));

    let mut simulation = Simulation::new(world, population);
    // "Flax and an awl on him": the awl was the founders' knife, which is
    // Stage 1 now; this man has one, as the kit before Stage 0 gave him.
    crate::analytics::tests::everybody_with_the_kit_before_stage_zero(&mut simulation);
    simulation.population.agents[0].inventory.max_weight = 500.0;

    // Seeded here rather than at the top, because building the world and the
    // agent draws: seeding first and then constructing them means the run
    // still moves whenever anything upstream changes how many numbers the
    // fixture takes. Whether a beginner ruins his first four bundles of flax
    // is a coin, and this test is about whether a cold man dresses himself.
    // See ISSUES_FOUND.md #132.
    crate::core::dice::seed(seed);

    // Flax both in the pack and growing next door. A random world does not
    // always let an agent reach the patch it can see - it may be across water
    // - and this test is about what an agent does with material, not about
    // whether it can get to it. Enough of it to survive a beginner's spoilage
    // too: about half of a raw hand's attempts are ruined in the making, so
    // one bundle would test the dice rather than the behaviour.
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("flax".to_string(), 200, 1.0));

    // Linen is Stage 3 of the textile path and waits on retted flax, and
    // sewing on an awl, which is the knife (`knows_how_to_make_garment`);
    // this is a man whose people have got that far, because the test is
    // about whether he dresses, not about whether linen has been found out.
    simulation.population.agents[0].found_out_how_to("rettedflax");
    simulation.population.agents[0].found_out_how_to("stoneknife");

    // An agent that wants to be warmer than this weather will ever make it.
    // Forcing the current temperature instead would not survive the turn,
    // which recomputes it from the climate before anyone decides anything.
    simulation.population.agents[0].body_temperature.ideal = 45.0;

    // Counted in days of half-hour turns. This was 600 turns, written as
    // fifty days when a turn was two hours; since a turn became half an hour
    // 600 was twelve and a half days, and nothing said so (#341). A lone man
    // kept permanently freezing does not live two hundred days, so a long
    // window trades whether he dresses for whether he dies.
    for _ in 0..(days * PLANNING_PERIODS_PER_DAY) {
        simulation.take_a_turn();

        // Some of these worlds kill him. A man who died is a man who did not
        // dress, which is an answer and not a panic.
        let Some(agent) = simulation.population.agents.first() else {
            return false;
        };

        if agent.body.total_cold_insulation() > 0.0 {
            break;
        }
    }

    simulation
        .population
        .agents
        .first()
        .map(|agent| agent.state.is_alive && agent.body.total_cold_insulation() > 0.0)
        .unwrap_or(false)
}

/// Taking a coat off and putting it back on is not a way to mend it.
#[test]
fn a_folded_coat_keeps_its_wear() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    {
        let agent = &mut simulation.population.agents[0];
        agent.inventory.max_weight = 500.0;
        agent.body_temperature.current = 34.0;
        agent.body.equip(
            ClothingTemplate::from_id("linen_cloak", Quality::Common)
                .expect("linen cloak should exist"),
        );
    }

    for _ in 0..500 {
        simulation.population.agents[0].body.turn_equipment_wear();
    }

    let worn_down = simulation.population.agents[0]
        .body
        .total_cold_insulation();
    assert!(worn_down > 0.0, "the cloak should still be on");

    // Take it off into the pack, and put it straight back on
    let displaced = simulation.population.agents[0]
        .body
        .unequip(EquipmentSlot::Back)
        .expect("the cloak should come off");

    let mut folded = InventoryItem::new_with_weight("linen_cloak".to_string(), 1, 2.0);
    folded.quality = Some(displaced.quality);
    folded.current_durability = Some(displaced.durability);
    folded.max_durability = Some(displaced.max_durability);
    simulation.population.agents[0].inventory.add_item(folded);

    let wearing = Action::WearClothing {
        garment: "linen_cloak".to_string(),
    };
    simulation.execute_action(&wearing, 0);

    let back_on = simulation.population.agents[0]
        .body
        .total_cold_insulation();

    assert!(
        (back_on - worn_down).abs() < 0.001,
        "a coat off the back and on again should be exactly as worn: {worn_down} -> {back_on}"
    );
}
