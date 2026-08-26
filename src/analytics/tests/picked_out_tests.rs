// src/analytics/tests/picked_out_tests.rs
//! Tests for a place running out, and for somebody knowing it has.
//!
//! The map an agent carries knew *what* was at a place and never whether there
//! was any of it left. So somebody would strip a patch, walk home, and walk
//! back to the same bare ground the next morning — and the morning after that,
//! for as long as the drive kept asking.
//!
//! **"Gather: no food sources nearby" was 10,127 refused turns a world**, and
//! "inventory full" another 5,255, and "no generic sources nearby" 2,209.
//! Between them more than half of everything a settlement ever got refused.
//! Two different faults with the same shape: several of the paths that produce
//! a `Gather` cannot see the world at all — `generate_action_for_drive` is a
//! static table that answers Sustenance with "gather food" whether or not
//! there is any food in the county.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::agents::exploration::ExplorationKnowledge;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, World, WorldConfig};

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

// --------------------------------------------------------------------------
// Remembering that a place is bare
// --------------------------------------------------------------------------

/// Nobody starts out believing anywhere is picked out.
#[test]
fn a_new_map_has_no_bare_places_on_it() {
    let simulation = one_person();
    let map = &simulation.population.agents[0].exploration_knowledge;

    assert!(map.where_it_ran_out.is_empty());
    assert!(!map.is_it_picked_out(Position::new(25, 25), 0));
}

/// Going for something and finding none of it puts the place on the map.
#[test]
fn finding_none_of_it_puts_the_place_on_the_map() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.found_none_at(Position::new(30, 30), 100);

    assert!(map.is_it_picked_out(Position::new(30, 30), 100));
    assert!(
        !map.is_it_picked_out(Position::new(40, 40), 100),
        "and nowhere else"
    );
}

/// It fades. A patch picked bare in June is bearing again by September, and a
/// man who writes it off for life is as wrong as the man who goes back every
/// morning.
#[test]
fn a_bare_place_grows_back() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.found_none_at(Position::new(30, 30), 0);

    assert!(map.is_it_picked_out(Position::new(30, 30), 0));
    assert!(
        !map.is_it_picked_out(
            Position::new(30, 30),
            ExplorationKnowledge::HOW_LONG_A_PLACE_STAYS_PICKED_OUT + 1
        ),
        "half a season on, that hedgerow is bearing again"
    );
}

/// And getting something there settles it, whatever this one used to think.
#[test]
fn getting_something_there_settles_it() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    map.found_none_at(Position::new(30, 30), 100);
    map.found_some_at(Position::new(30, 30));

    assert!(!map.is_it_picked_out(Position::new(30, 30), 100));
}

/// Nobody carries an unbounded list of bare ground about with them.
#[test]
fn a_head_is_not_a_filing_cabinet() {
    let mut simulation = one_person();
    let map = &mut simulation.population.agents[0].exploration_knowledge;

    for i in 0..200 {
        map.found_none_at(Position::new(i % 90, i / 90), 100 + i as u32);
    }

    assert!(
        map.where_it_ran_out.len() <= 64,
        "{}",
        map.where_it_ran_out.len()
    );
}

// --------------------------------------------------------------------------
// Where it comes from
// --------------------------------------------------------------------------

/// Stripping the last of something is not a private fact: whoever is near
/// enough watches the ground go bare.
#[test]
fn everybody_standing_there_sees_the_last_of_it_go() {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.resources.clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let here = Position::new(25, 25);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[1].state.position = (25, 25, 0);

    // One stem, so the first armful takes the lot.
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Wood, here, 1));

    let result = simulation.execute_action(
        &Action::Gather {
            resource_type: "wood".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    for who in 0..2 {
        assert!(
            simulation.population.agents[who]
                .exploration_knowledge
                .is_it_picked_out(here, simulation.current_tick),
            "agent {who} was standing on it when the last of it went"
        );
    }
}

/// And somebody who carried an armful home off it knows the place is bearing.
#[test]
fn carrying_something_home_off_it_takes_it_back_off_the_list() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    simulation.world.resources.clear();
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Wood, here, 400));

    simulation.population.agents[0]
        .exploration_knowledge
        .found_none_at(here, 0);

    let result = simulation.execute_action(
        &Action::Gather {
            resource_type: "wood".to_string(),
        },
        0,
    );
    assert!(result.success, "{:?}", result.message);

    assert!(
        !simulation.population.agents[0]
            .exploration_knowledge
            .is_it_picked_out(here, simulation.current_tick),
        "there was plenty there after all"
    );
}

// --------------------------------------------------------------------------
// Acting on it
// --------------------------------------------------------------------------

/// A patch this one has stripped is not where it goes for its dinner.
#[test]
fn nobody_walks_back_to_ground_they_stripped() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    let near = Position::new(here.0 + 3, here.1);
    let far = Position::new(here.0 + 9, here.1);

    simulation.world.resources.clear();
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Food, near, 40));
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Food, far, 40));

    assert_eq!(
        simulation.nearest_edible_this_one_would_go_to(
            &simulation.population.agents[0],
            here,
            30
        ),
        Some(near),
        "with nothing known, the near one wins"
    );

    simulation.population.agents[0]
        .exploration_knowledge
        .found_none_at(near, simulation.current_tick);

    assert_eq!(
        simulation.nearest_edible_this_one_would_go_to(
            &simulation.population.agents[0],
            here,
            30
        ),
        Some(far),
        "having stripped the near one, the far one is where dinner is"
    );
}

/// But a settlement does not starve out of tidiness: the only patch there is
/// gets walked to whatever this one remembers about it.
#[test]
fn the_only_patch_there_is_gets_walked_to_anyway() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;
    let only = Position::new(here.0 + 3, here.1);

    simulation.world.resources.clear();
    simulation
        .world
        .resources
        .push(ResourceNode::new(ResourceType::Food, only, 40));

    simulation.population.agents[0]
        .exploration_knowledge
        .found_none_at(only, simulation.current_tick);

    assert_eq!(
        simulation.nearest_edible_this_one_would_go_to(
            &simulation.population.agents[0],
            here,
            30
        ),
        Some(only),
        "there is nowhere else to eat"
    );
}

// --------------------------------------------------------------------------
// Not asking for what is not there
// --------------------------------------------------------------------------

/// Asking to gather food where there is none is a turn gone.
#[test]
fn asking_for_food_where_there_is_none_is_not_worth_the_turn() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.clear();

    assert!(
        !simulation.could_this_gather_come_to_anything(
            &simulation.population.agents[0],
            here,
            "food"
        ),
        "there is nothing edible in the county"
    );

    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Food,
        Position::new(here.0 + 2, here.1),
        40,
    ));

    assert!(
        simulation.could_this_gather_come_to_anything(
            &simulation.population.agents[0],
            here,
            "food"
        ),
        "and now there is"
    );
}

/// Nor with your arms already full.
#[test]
fn nobody_sets_off_for_another_armful_with_their_arms_full() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.clear();
    simulation.world.resources.push(ResourceNode::new(
        ResourceType::Wood,
        Position::new(here.0 + 2, here.1),
        400,
    ));

    assert!(simulation.could_this_gather_come_to_anything(
        &simulation.population.agents[0],
        here,
        "wood"
    ));

    {
        let agent = &mut simulation.population.agents[0];
        let full = agent.inventory.effective_max_weight();
        agent
            .inventory
            .add_item(InventoryItem::new_with_weight("stone".to_string(), 1, full));
        agent.inventory.recalculate_weight();
    }

    assert!(
        !simulation.could_this_gather_come_to_anything(
            &simulation.population.agents[0],
            here,
            "wood"
        ),
        "there is nowhere to put it"
    );
}

/// A thirsty man with a full waterskin on dry ground is not refused: water is
/// drunk rather than carried off, and that is what carrying one is for.
#[test]
fn a_full_waterskin_is_an_answer_to_thirst_on_dry_ground() {
    let mut simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    simulation.world.resources.clear();

    assert!(
        simulation.could_this_gather_come_to_anything(
            &simulation.population.agents[0],
            here,
            "water"
        ),
        "a waterskin is the whole point of a waterskin"
    );
}

/// And a word this world has never heard is refused without spending the turn
/// finding that out.
#[test]
fn a_word_the_world_does_not_know_is_refused_at_once() {
    let simulation = one_person();
    let here = simulation.population.agents[0].state.position;

    assert!(!simulation.could_this_gather_come_to_anything(
        &simulation.population.agents[0],
        here,
        "luxury"
    ));
}

/// The vocabulary `Gather` answers to is one table, read by the executor and
/// by everything that decides whether a gather is worth asking for. Two
/// tables that drift is how clay came to spawn in every world for a year with
/// nobody able to pick any of it up.
#[test]
fn the_gather_vocabulary_is_one_table() {
    for (word, expected) in [
        ("clay", ResourceType::Clay),
        ("greens", ResourceType::Greens),
        ("roots", ResourceType::Roots),
        ("salt", ResourceType::Salt),
        ("grain", ResourceType::Grain),
    ] {
        assert_eq!(
            Simulation::what_a_gather_asks_for(word),
            Some(expected),
            "{word} is a thing this world has"
        );
    }

    assert_eq!(Simulation::what_a_gather_asks_for("moonbeams"), None);
}
