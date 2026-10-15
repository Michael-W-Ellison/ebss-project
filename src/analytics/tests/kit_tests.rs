// src/analytics/tests/kit_tests.rs
//! An edge to work with, and the cord for the next one (ISSUES_FOUND #332).
//!
//! Founders walk in with a stone knife; it wears through in a few weeks of
//! cutting, and nothing made another. These cover:
//! - a struck flake is an edge
//! - somebody with no edge takes one on before anything else in a spare turn
//! - a making short of something goes and gets it, rather than giving up
//! - a full pack is not a reason to leave the makings of an edge on the ground

use crate::agents::{AgentConfig, InventoryItem, Population, SkillType};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

/// One grown person with nothing in the pack, on an empty map.
fn one_person_with_nothing() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.file_the_nodes();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.get_all_items_mut().clear();
    simulation
}

/// Most of what was ever butchered in the stone age was butchered with a
/// flake: better than fingers, worse than a knife with a grip on it.
#[test]
fn a_struck_flake_is_an_edge() {
    let mut simulation = one_person_with_nothing();
    let bare = simulation.population.agents[0].how_much_my_tools_bring_back(SkillType::Leatherworking);

    let flake = simulation.population.agents[0].a_tool_fresh_from_these_hands("knappedtip", 1, 0.5);
    simulation.population.agents[0].inventory.add_item(flake);
    let with_a_flake =
        simulation.population.agents[0].how_much_my_tools_bring_back(SkillType::Leatherworking);

    let knife = simulation.population.agents[0].a_tool_fresh_from_these_hands("stoneknife", 1, 0.5);
    simulation.population.agents[0].inventory.add_item(knife);
    let with_a_knife =
        simulation.population.agents[0].how_much_my_tools_bring_back(SkillType::Leatherworking);

    assert!(with_a_flake > 1.0 && with_a_flake > bare, "a flake is {with_a_flake}, bare hands {bare}");
    assert!(with_a_knife > with_a_flake, "a knife is {with_a_knife}, a flake {with_a_flake}");
}

/// Somebody with no edge at all and two stones has a flake as the next step
/// of the kit - after the pottering, not ahead of it.
///
/// Ahead of it, the settlement lost its first winter's store: the pottering is
/// where bowls come from, and on scattered seed 3 the larder at the year's end
/// was a fifth of what it was with the edge behind it (#333).
#[test]
fn somebody_with_no_edge_knaps_one_after_the_pottering() {
    let mut simulation = one_person_with_nothing();
    for what in ["stone", "flax", "wood"] {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight(what.to_string(), 6, 0.1));
    }
    let here = simulation.population.agents[0].state.position;
    let agent = &simulation.population.agents[0];

    assert_eq!(agent.what_my_kit_is_short_of(), Some("knappedtip"), "the humblest edge is a flake");
    assert_eq!(
        simulation.how_i_would_come_by("knappedtip", agent),
        Some(Action::Craft { item_type: "knappedtip".to_string() })
    );

    let chosen = simulation.what_this_drive_offers(crate::core::DriveType::Utility, agent, here);
    assert!(
        matches!(chosen, Some(Action::Work { .. })),
        "with something to potter at, the pottering comes first: {chosen:?}"
    );
}

/// And with an edge, the cord for the next one is what the kit wants.
#[test]
fn somebody_with_an_edge_keeps_a_cord_for_the_next() {
    let mut simulation = one_person_with_nothing();
    let knife = simulation.population.agents[0].a_tool_fresh_from_these_hands("stoneknife", 1, 0.5);
    simulation.population.agents[0].inventory.add_item(knife);

    assert_eq!(simulation.population.agents[0].what_my_kit_is_short_of(), Some("lashing"));

    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("lashing".to_string(), 2, 0.1));
    assert_eq!(simulation.population.agents[0].what_my_kit_is_short_of(), None);
}

/// A man with a full pack and no edge, standing by a seam of stone: the stone
/// goes in, and what is worth least comes out to make room for it.
///
/// Stone is five to the unit and packs run full, so the fetching was refused
/// before it began and the man stood over the seam with no way to strike a
/// flake, every time (#332).
#[test]
fn a_full_pack_does_not_keep_the_makings_of_an_edge_out() {
    let mut simulation = one_person_with_nothing();
    let here = Position::new(30, 30);
    simulation.world.put_a_node_down(ResourceNode::new(ResourceType::Stone, here, 40));
    let now = simulation.current_turn;
    {
        let agent = &mut simulation.population.agents[0];
        agent.exploration_knowledge.discover_resource(here, ResourceType::Stone, now);
        agent.inventory.max_weight = 20.0;
        // Full of something nobody here has a use for
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("iron".to_string(), 2, 10.0));
    }

    let agent = &simulation.population.agents[0];
    let step = simulation
        .how_i_would_come_by("knappedtip", agent)
        .expect("there is stone underfoot, so there is a way to a flake");
    assert_eq!(step, Action::Gather { resource_type: "stone".to_string() });

    let held = simulation.hold_on_to_the_making(step.clone(), 0);
    assert_eq!(held, step);
    simulation.execute_action(&step, 0);

    assert!(
        simulation.population.agents[0].how_many_i_have("stone") > 0,
        "the stone should be in the pack"
    );
}

/// A making short of something it can fetch goes and fetches it, rather than
/// being given up as stuck.
#[test]
fn a_making_short_of_stone_goes_for_stone() {
    let mut simulation = one_person_with_nothing();
    let here = Position::new(31, 30);
    simulation.world.put_a_node_down(ResourceNode::new(ResourceType::Stone, here, 40));
    let now = simulation.current_turn;
    simulation.population.agents[0]
        .exploration_knowledge
        .discover_resource(here, ResourceType::Stone, now);
    simulation.population.agents[0].errand = Some(crate::agents::Errand {
        going_to: (30, 30, 0),
        set_out_from: (30, 30, 0),
        to_make: Some("knappedtip".to_string()),
        for_drive: crate::core::DriveType::Utility,
        pressed_this_hard: 1.0,
        turns_on_it: 1,
        set_aside: 0,
    });

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.how_i_would_come_by("knappedtip", agent),
        Some(Action::Gather { resource_type: "stone".to_string() })
    );
}


/// Only what the storehouse has a name for is offered to it. Tinder comes three
/// to a scraped stick, so it was the most of anything in the pack of somebody
/// with a flake, and every turn spent putting it by was refused (#332).
#[test]
fn tinder_is_not_offered_to_the_store() {
    let mut simulation = one_person_with_nothing();
    let agent = &mut simulation.population.agents[0];
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight("tinder".to_string(), 30, 0.1));
    agent
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 12, 0.5));

    let spare = agent.what_i_can_spare().map(|(what, _)| what);
    assert_ne!(spare.as_deref(), Some("tinder"));
}

/// A pack full of food has no room to make for a stone, making or not.
#[test]
fn a_pack_full_of_food_is_not_asked_for_stone() {
    let mut simulation = one_person_with_nothing();
    let here = Position::new(30, 30);
    simulation.world.put_a_node_down(ResourceNode::new(ResourceType::Stone, here, 40));
    let now = simulation.current_turn;
    {
        let agent = &mut simulation.population.agents[0];
        agent.exploration_knowledge.discover_resource(here, ResourceType::Stone, now);
        agent.inventory.max_weight = 10.0;
        let mut food = InventoryItem::new_with_weight("dried_legumes".to_string(), 20, 0.5);
        food.food_data = simulation
            .food_database
            .create_food_data(&crate::world::ItemType::Legumes, now);
        agent.inventory.add_item(food);
    }
    let agent = &simulation.population.agents[0];
    assert_eq!(simulation.how_i_would_come_by("knappedtip", agent), None);
}
