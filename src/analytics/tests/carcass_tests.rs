// src/analytics/tests/carcass_tests.rs
//! A kill leaves a carcass: what is not cut off it today is still on the bone
//! tomorrow, for the hunter or for whoever he tells (ISSUES_FOUND #331).
//!
//! - the meat is cut as far as a turn takes it, and the rest is left lying
//! - an edge takes more off a carcass in a turn than bare hands
//! - a joint cut off a carcass is as old as the kill
//! - a carcass goes off
//! - a hunter calls people over to a kill, and is thought better of for it

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceType, World, WorldConfig};

/// A hunter with a spear, one throw from killing a deer at his feet.
fn a_deer_about_to_fall() -> (Simulation, uuid::Uuid) {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    let deer = world.spawn_animal("deer".to_string(), (30, 31)).expect("a deer should spawn");
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;
    nothing_to_cut_with(&mut simulation);
    for _ in 0..8 {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight("spear".to_string(), 1, 2.0));
    }
    if let Some(animal) = simulation.world.animals.get_mut(&deer) {
        animal.current_health = 1.0;
    }
    (simulation, deer)
}

/// Founders walk in with a handaxe and a stone knife (`WHAT_THEY_CARRY`);
/// these hands have neither.
fn nothing_to_cut_with(simulation: &mut Simulation) {
    for tool in ["stoneknife", "handaxe", "metalknife"] {
        let held = simulation.population.agents[0]
            .inventory
            .get_item(tool)
            .map(|item| item.quantity)
            .unwrap_or(0);
        simulation.population.agents[0].inventory.remove_item(tool, held);
    }
}

fn bring_it_down(simulation: &mut Simulation, deer: uuid::Uuid) {
    for _ in 0..60 {
        if simulation.world.animals.get(&deer).is_none_or(|a| !a.is_alive()) {
            return;
        }
        simulation.population.agents[0].busy_until = 0;
        simulation.execute_action(&Action::Hunt { animal_id: deer, weapon: None }, 0);
    }
    panic!("the deer never came down");
}

fn the_carcass(simulation: &Simulation) -> Option<&crate::world::ResourceNode> {
    simulation
        .world
        .resources
        .iter()
        .find(|node| node.is_a_carcass() && node.amount > 0)
}

/// A deer brought down with bare hands to cut it is one joint in the pack and
/// the rest on the bone, where the hunter knows it is - not a joint in the pack
/// and eighty-five in a hundred of it never there at all.
#[test]
fn the_rest_of_a_kill_is_left_on_the_bone() {
    let (mut simulation, deer) = a_deer_about_to_fall();
    bring_it_down(&mut simulation, deer);

    let carried = simulation.population.agents[0]
        .inventory
        .get_item("meat")
        .map(|meat| meat.quantity)
        .unwrap_or(0);
    let carcass = the_carcass(&simulation).expect("the rest of the deer should be lying there");
    let left = carcass.amount;
    let at = carcass.position;

    assert_eq!(carried, 1, "bare hands cut one joint in the turn it fell");
    assert!(left >= 4, "a deer is eight to twelve joints a fat one; {left} were left on it");
    assert_eq!(
        simulation.population.agents[0].exploration_knowledge.known_resources.get(&at),
        Some(&ResourceType::Meat),
        "the hunter knows where his kill is"
    );
}

/// How many joints one turn of cutting at a carcass takes, with this in the
/// pack.
fn a_turn_at_the_carcass_with(in_the_pack: Option<&str>) -> u32 {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    let mut simulation = Simulation::new(world, population);
    let here = Position::new(30, 30);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;
    nothing_to_cut_with(&mut simulation);
    if let Some(tool) = in_the_pack {
        simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight(tool.to_string(), 1, 1.0));
    }
    let now = simulation.current_turn;
    simulation.world.a_carcass_lies_here(here, 40, now);
    simulation.population.agents[0]
        .exploration_knowledge
        .discover_resource(here, ResourceType::Meat, now);

    let mut taken = 0;
    for _ in 0..6 {
        let before = simulation.population.agents[0]
            .inventory
            .get_item("meat")
            .map(|meat| meat.quantity)
            .unwrap_or(0);
        simulation.population.agents[0].busy_until = 0;
        simulation.execute_action(&Action::Gather { resource_type: "food".to_string() }, 0);
        let after = simulation.population.agents[0]
            .inventory
            .get_item("meat")
            .map(|meat| meat.quantity)
            .unwrap_or(0);
        taken += after.saturating_sub(before);
    }
    taken
}

/// "Why is meat harvesting not utilizing cutting tools?" A knife takes a
/// carcass apart faster than fingers do; fingers get there in the end.
#[test]
fn a_knife_takes_more_off_a_carcass_than_bare_hands() {
    let bare = a_turn_at_the_carcass_with(None);
    let knife = a_turn_at_the_carcass_with(Some("stoneknife"));

    assert!(bare >= 6, "bare hands still tear a joint off a turn: {bare} in six turns");
    assert!(knife > bare, "six turns with a stone knife took {knife}, with bare hands {bare}");
}

/// A joint cut off a three-day-old carcass is three days old.
#[test]
fn meat_off_a_carcass_is_as_old_as_the_kill() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    let mut simulation = Simulation::new(world, population);
    let here = Position::new(30, 30);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].inventory.max_weight = 500.0;

    let three_days = 3 * crate::environment::seasons::TICKS_PER_DAY;
    simulation.current_turn += three_days;
    simulation.world.turn = simulation.current_turn;
    let killed = simulation.current_turn - three_days;
    simulation.world.a_carcass_lies_here(here, 20, killed);

    simulation.execute_action(&Action::Gather { resource_type: "food".to_string() }, 0);

    let meat = simulation.population.agents[0]
        .inventory
        .get_item("meat")
        .expect("a joint should have come off");
    let food = meat.food_data.as_ref().expect("a joint is food");
    assert_eq!(food.created_turn, killed, "the joint is as old as the kill");
}

/// Left lying, a carcass is carrion inside a week, and goes.
#[test]
fn a_carcass_goes_off() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let now = world.turn;
    world.a_carcass_lies_here(Position::new(30, 30), 20, now);

    let a_week = 7 * crate::environment::seasons::TICKS_PER_DAY;
    while world.turn < now + a_week {
        world.take_a_turn();
    }
    world.remove_depleted_resources();

    assert!(
        !world.resources.iter().any(|node| node.is_a_carcass()),
        "a week-old carcass should be gone"
    );
}

/// A man with more of a kill than he can carry home tells whoever is in
/// earshot where it is, and they think better of him for it.
#[test]
fn a_hunter_calls_people_over_to_his_kill() {
    let mut population = Population::new();
    let mut config = AgentConfig::default();
    config.random_weights = false;
    population.spawn_agent(config.clone());
    population.spawn_agent(config);
    for agent in &mut population.agents {
        agent.traits = crate::core::traits::TraitSet::new();
        agent.state.position = (10, 10, 0);
    }

    let kill = Position::new(14, 12);
    let now = population.current_turn;
    {
        let knows = &mut population.agents[0].exploration_knowledge;
        knows.discover_resource(kill, ResourceType::Meat, now);
        knows.saw_it_again(kill, 30, now);
    }
    let hunter = population.agents[0].id;
    if let Some(social) = population.agents[0].drives.get_mut(crate::core::DriveType::Social) {
        social.value = 0.6;
    }
    let social_before = population.agents[0]
        .drives
        .get(crate::core::DriveType::Social)
        .map(|drive| drive.value)
        .unwrap_or(0.0);

    population.say_it_out_loud();

    let listener = &population.agents[1];
    assert_eq!(
        listener.exploration_knowledge.known_resources.get(&kill),
        Some(&ResourceType::Meat),
        "the man standing next to him should have heard where the kill is"
    );
    let regard = listener
        .relationships
        .get_relationship(&hunter)
        .map(|relationship| relationship.bond_strength)
        .unwrap_or(0.0);
    assert!(regard > 0.0, "being told where the meat is should be taken as a gift");

    let social_after = population.agents[0]
        .drives
        .get(crate::core::DriveType::Social)
        .map(|drive| drive.value)
        .unwrap_or(0.0);
    assert!(
        social_after < social_before,
        "giving the kill away should answer some of the hunter's social drive: {social_before} to {social_after}"
    );
}

/// Taking food off a carcass is knife work, and what makes a man want a knife
/// is the work he is about to do with one. Read off the request alone, it was
/// picking berries (#331).
#[test]
fn food_off_a_carcass_is_knife_work() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.file_the_nodes();
    let mut simulation = Simulation::new(world, population);
    let here = Position::new(30, 30);
    simulation.population.agents[0].state.position = (30, 30, 0);
    let now = simulation.current_turn;
    simulation.world.a_carcass_lies_here(here, 20, now);
    simulation.population.agents[0]
        .exploration_knowledge
        .discover_resource(here, ResourceType::Meat, now);

    let gather = Action::Gather { resource_type: "food".to_string() };
    assert_eq!(
        simulation.what_trade_this_asks_for(&gather, 0),
        Some(crate::agents::skills::SkillType::Leatherworking)
    );
}

/// Somebody told where a kill lies keeps it in mind for as long as it lasts,
/// however full his head is of other places.
///
/// Weighed with everything else he knew, a kill he had only heard of, out in
/// country he did not know, was the first thing to go: forgotten by the next
/// turn and told again, thousands of times over, and never once walked to
/// (#331).
#[test]
fn a_kill_you_were_told_of_is_not_forgotten_by_the_next_turn() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let now = population.current_turn;
    let somebody = uuid::Uuid::from_u128(7);
    let listener = &mut population.agents[0];
    listener.state.position = (10, 10, 0);

    // A head already full of places he has seen for himself
    for i in 0..(crate::agents::Agent::WHAT_A_MAN_CAN_HOLD_IN_MIND as i32 + 20) {
        let place = Position::new(10 + i % 12, 10 + i / 12);
        listener
            .exploration_knowledge
            .discover_resource(place, ResourceType::Food, now);
        listener.exploration_knowledge.saw_it_again(place, 30, now);
    }
    let kill = Position::new(80, 80);
    listener
        .exploration_knowledge
        .take_their_word_for_it(kill, ResourceType::Meat, somebody, now, Some(12), now);

    listener.forget_what_does_not_matter(now + 30);

    assert_eq!(
        listener.exploration_knowledge.known_resources.get(&kill),
        Some(&ResourceType::Meat),
        "the kill he was told of should still be in mind"
    );
}

/// Six joints left on a carcass are eighteen portions of meat, and that is a
/// better walk than a berry bush in full fruit the same distance off.
///
/// A carcass was weighed by the joint at the worth of a portion - a third of
/// what was on it - and the bush won every time, so nobody told of a kill
/// ever went (#331).
#[test]
fn a_carcass_is_worth_more_than_a_bush_the_same_walk_away() {
    let mut world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.file_the_nodes();
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    let now = simulation.current_turn;

    let kill = Position::new(40, 30);
    let bush = Position::new(20, 30);
    simulation.world.a_carcass_lies_here(kill, 6, now);
    simulation
        .world
        .put_a_node_down(crate::world::ResourceNode::new(ResourceType::Food, bush, 14));
    for (place, what) in [(kill, ResourceType::Meat), (bush, ResourceType::Food)] {
        let knows = &mut simulation.population.agents[0].exploration_knowledge;
        knows.discover_resource(place, what, now);
        knows.saw_it_again(place, 14, now);
    }

    let agent = &simulation.population.agents[0];
    let best = simulation.the_best_food_anywhere(agent, agent.state.position);
    assert_eq!(best, Some(kill), "the carcass should be the better walk");
}
