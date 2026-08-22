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
            ClothingTemplate::from_id(recipe.id, Quality::Basic).is_some(),
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
    assert_eq!(first, Quality::Crude, "a first garment should be crude");

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
            .add_item(InventoryItem::new_with_weight("flax".to_string(), 20, 1.0));
        agent.body_temperature.current = 34.0;
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
    simulation.execute_action(&making, 0);

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
        agent.inventory.get_item("flax").map(|f| f.quantity).unwrap_or(0) < 20,
        "the flax should have gone into it"
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
/// Whatever is worn wears down a little each tick, so without a margin the one
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

        let worn = ClothingTemplate::from_id("linen_cloak", Quality::Crude)
            .expect("linen cloak should exist");
        agent.body.equip(worn);

        let mut spare = InventoryItem::new_with_weight("linen_cloak".to_string(), 1, 2.0);
        spare.quality = Some(Quality::Crude);
        agent.inventory.add_item(spare);
    }

    // Wear the worn one down a little, as a tick would
    for _ in 0..50 {
        simulation.population.agents[0].body.tick_equipment_wear();
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

    let cloak = ClothingTemplate::from_id("linen_cloak", Quality::Basic)
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
    simulation.population.agents[0].inventory.max_weight = 500.0;

    // Flax both in the pack and growing next door. A random world does not
    // always let an agent reach the patch it can see - it may be across water
    // - and this test is about what an agent does with material, not about
    // whether it can get to it.
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("flax".to_string(), 20, 1.0));

    // An agent that wants to be warmer than this weather will ever make it.
    // Forcing the current temperature instead would not survive the tick,
    // which recomputes it from the climate before anyone decides anything.
    simulation.population.agents[0].body_temperature.ideal = 45.0;

    for _ in 0..600 {
        simulation.tick();

        if simulation.population.agents[0].body.total_cold_insulation() > 0.0 {
            break;
        }
    }

    let agent = &simulation.population.agents[0];
    assert!(agent.state.is_alive, "the agent should have survived the test");
    assert!(
        agent.body.total_cold_insulation() > 0.0,
        "a cold agent with flax should end up wearing something, carrying {:?}",
        agent.inventory.get_all_items().keys().collect::<Vec<_>>()
    );
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
            ClothingTemplate::from_id("linen_cloak", Quality::Basic)
                .expect("linen cloak should exist"),
        );
    }

    for _ in 0..500 {
        simulation.population.agents[0].body.tick_equipment_wear();
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
