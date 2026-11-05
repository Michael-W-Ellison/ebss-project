// src/analytics/tests/spare_tools_tests.rs
//! A pack full of spare tools (#411).
//!
//! Once skills stopped wearing off in a day (#410), eight of the nine people
//! who starved in forty worlds did it the same way: a pack full of three bows,
//! three nets, three rods or three spears, nothing in it that would go on the
//! grass, and so no room for a handful out of a store. The store branch sent
//! them from one pit to the next and back, two hundred and fifty walks a day,
//! until the reserve ran out. Two of them carried whole fish they could have
//! torn apart and eaten.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::Action;
use crate::world::nutrition::FoodDatabase;
use crate::world::{ItemType, Position, World, WorldConfig};

/// One person on bare ground with an empty pack.
fn somebody_with_an_empty_pack() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].inventory.get_all_items_mut().clear();
    simulation
}

/// A pit of roots `paces` east of where the one person stands, remembered.
fn a_pit_of_roots(simulation: &mut Simulation, paces: i32) -> (i32, i32) {
    let here = simulation.population.agents[0].state.position;
    let at = (here.0 + paces, here.1);
    let each = Simulation::what_one_of_these_weighs(crate::world::ResourceType::Roots);
    let mut buried = InventoryItem::new_with_weight("roots".to_string(), 150, each);
    buried.food_data = simulation.food_database.create_food_data(&ItemType::Roots, 0);
    let mut pit = crate::world::Pit {
        gnawed: 0,
        last_opened: None,
        where_it_is: Position::new(at.0, at.1),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    };
    pit.put_in(buried);
    simulation.world.pits.push(pit);
    simulation.population.agents[0].memory.remember_how_much_is_there(
        crate::core::memory::SpatialMemoryType::Storage,
        (at.0, at.1, 0),
        150,
    );
    at
}

/// Fill what room is left in the pack with `what`, at one unit each.
fn fill_with(simulation: &mut Simulation, what: &str) {
    let agent = &mut simulation.population.agents[0];
    let room = agent.inventory.weight_capacity_remaining();
    let how_many = room.floor() as u32;
    let mut stack = InventoryItem::new_with_weight(what.to_string(), how_many, 1.0);
    stack.current_durability = Some(40.0);
    stack.max_durability = Some(40.0);
    agent.inventory.get_all_items_mut().insert(what.to_string(), stack);
    let left = agent.inventory.weight_capacity_remaining();
    if left > 0.0 {
        let mut rest = InventoryItem::new_with_weight("bow".to_string(), 1, left);
        rest.current_durability = Some(40.0);
        rest.max_durability = Some(40.0);
        agent.inventory.get_all_items_mut().insert("bow".to_string(), rest);
    }
    assert!(agent.inventory.weight_capacity_remaining() < 0.01);
}

/// Somebody who has worked out a net makes one to see it, and not three: a
/// tool is seen once it is in the hand. Cord, a part, is still kept a few of.
#[test]
fn a_tool_is_made_to_see_it_once() {
    let mut simulation = somebody_with_an_empty_pack();
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("fishingnet");
    agent.inventory.add_item(InventoryItem::new("lashing".to_string(), 8));

    assert_eq!(
        agent.what_i_would_try_out().as_deref(),
        Some("fishingnet"),
        "with the cord for it and none made, the net is what he tries"
    );

    agent.inventory.add_item(InventoryItem::new("fishingnet".to_string(), 1));
    assert_ne!(
        agent.what_i_would_try_out().as_deref(),
        Some("fishingnet"),
        "with one in the pack, a second is not made to see it"
    );
}

/// The kit is one of each tool. The second and third net go on the grass to
/// make room for supper; the first stays.
#[test]
fn spare_tools_make_room_and_the_one_in_use_stays() {
    let mut simulation = somebody_with_an_empty_pack();
    fill_with(&mut simulation, "fishingnet");
    let agent = &simulation.population.agents[0];
    let nets = agent.how_many_i_have("fishingnet");
    assert!(nets >= 3, "the fixture is a pack of nets: {nets}");

    assert!(agent.could_i_take_another_handful(1.0), "a spare net makes room for a root");
    let what = agent.what_i_would_set_down();
    assert!(
        matches!(what.as_deref(), Some("fishingnet") | Some("bow")),
        "something spare goes down: {what:?}"
    );
    assert_eq!(
        agent.how_much_of_this_makes_room("fishingnet", 1_000.0),
        nets - 1,
        "every net but the one in use"
    );

    // One of each and nothing else is all kit, and nothing goes.
    let mut simulation = somebody_with_an_empty_pack();
    let agent = &mut simulation.population.agents[0];
    let mut net = InventoryItem::new_with_weight("fishingnet".to_string(), 1, agent.inventory.max_weight);
    net.current_durability = Some(40.0);
    net.max_durability = Some(40.0);
    agent.inventory.get_all_items_mut().insert("fishingnet".to_string(), net);
    assert_eq!(agent.what_i_would_set_down(), None, "the one net is the kit");
    assert_eq!(agent.how_much_of_this_makes_room("fishingnet", 1_000.0), 0);
}

/// Starving beside a pit of roots with a pack full of nets, he puts the
/// spares down and takes the roots out.
#[test]
fn a_pack_of_spare_tools_does_not_keep_him_out_of_the_larder() {
    let mut simulation = somebody_with_an_empty_pack();
    a_pit_of_roots(&mut simulation, 0);
    fill_with(&mut simulation, "fishingnet");
    simulation.population.agents[0].state.physiology.gone_without_food_for(20 * 1_440);

    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;
    let Some(Action::PickUp { what }) = simulation.something_out_of_the_store(&agent, here) else {
        panic!("standing on a pit of roots with spare nets, he was not offered it");
    };
    let result = simulation.execute_action(&Action::PickUp { what }, 0);
    assert!(result.success, "{:?}", result.message);
    let agent = &simulation.population.agents[0];
    assert!(agent.how_many_i_have("roots") > 0, "and roots came in");
    assert!(agent.how_many_i_have("fishingnet") >= 1, "and the net in use stayed");
}

/// With a pack that would not take a unit out of a pit even making room, he
/// is not sent walking to one: the walk does not empty the pack, and the next
/// pit sent him back to the first (#411, as #252 was for rot).
#[test]
fn a_pack_that_cannot_take_a_root_is_not_walked_to_a_pit() {
    let mut simulation = somebody_with_an_empty_pack();
    a_pit_of_roots(&mut simulation, 6);
    // One of a tool, weighing the whole pack: kit, nothing to spare
    let agent = &mut simulation.population.agents[0];
    let mut net = InventoryItem::new_with_weight("fishingnet".to_string(), 1, agent.inventory.max_weight);
    net.current_durability = Some(40.0);
    net.max_durability = Some(40.0);
    agent.inventory.get_all_items_mut().insert("fishingnet".to_string(), net);
    agent.state.physiology.gone_without_food_for(20 * 1_440);

    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;
    assert_eq!(simulation.something_out_of_the_store(&agent, here), None);

    // With room, the same pit is a walk
    simulation.population.agents[0].inventory.get_all_items_mut().clear();
    let agent = simulation.population.agents[0].clone();
    assert!(matches!(
        simulation.something_out_of_the_store(&agent, here),
        Some(Action::Move { .. })
    ));
}

/// Hungry, with whole fish and no knife, he eats: a whole fish comes apart in
/// the hands. The override sent him to the store instead (#411).
#[test]
fn hungry_with_whole_fish_and_no_knife_he_eats() {
    let mut simulation = somebody_with_an_empty_pack();
    a_pit_of_roots(&mut simulation, 6);
    let agent = &mut simulation.population.agents[0];
    let mut whole = InventoryItem::new_with_weight("fish".to_string(), 3, 0.5);
    whole.food_data = FoodDatabase::new().create_food_data(&ItemType::Fish, 0);
    agent.inventory.add_item(whole);
    agent.state.physiology.gone_without_food_for(20 * 1_440);
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = 1.0;
    }
    assert!(agent.find_best_food_to_eat().is_none(), "a whole fish is not supper as it stands");
    assert!(agent.could_pull_apart_by_hand());

    let agent = simulation.population.agents[0].clone();
    let here = agent.state.position;
    let (action, _) = simulation.generate_non_emotional_action(&agent, here);
    assert!(matches!(action, Action::Eat { .. }), "he should eat, and did {action:?}");
}
