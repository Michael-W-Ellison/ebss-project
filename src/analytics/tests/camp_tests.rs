// src/analytics/tests/camp_tests.rs
//! Who goes when a camp moves.
//!
//! There is no settlement in this model, only people and where each of them
//! lives. A camp moves when somebody decides it should, and the people who
//! live with them go too: as many as the new ground will feed, unless they
//! have a crop standing. That is how one camp becomes two. See ISSUES_FOUND
//! #314.

use crate::agents::{AgentConfig, LifeStage, Population};
use crate::analytics::wanting::camp::CampMove;
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

const HERE: (i32, i32) = (10, 10);
const THERE: (i32, i32) = (60, 20);

/// `how_many` grown people who all live at `HERE`.
fn a_camp(how_many: usize) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();
    let mut population = Population::new();
    for _ in 0..how_many {
        population.spawn_agent(AgentConfig::default());
    }
    let mut simulation = Simulation::new(world, population);
    simulation.current_turn = 1000;
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (HERE.0, HERE.1, 0);
        agent.hearth = Some(HERE);
    }
    simulation
}

fn living_there(simulation: &Simulation) -> usize {
    simulation
        .population
        .agents
        .iter()
        .filter(|agent| agent.hearth == Some(THERE))
        .count()
}

/// A move to water takes everybody who lives there.
#[test]
fn a_camp_moving_for_water_goes_together() {
    let mut simulation = a_camp(5);
    let who = simulation.population.agents[0].id;
    simulation.the_camp_moves(CampMove { who, to: THERE, feeds: None });
    assert_eq!(living_there(&simulation), 5);
    assert!(simulation.population.agents.iter().all(|agent| agent.hearth_moved_at == 1000));
}

/// A move to better ground takes as many as it will feed. The rest stay, and
/// that is a camp become two.
#[test]
fn as_many_go_as_the_new_ground_will_feed() {
    let mut simulation = a_camp(6);
    let who = simulation.population.agents[0].id;
    simulation.the_camp_moves(CampMove { who, to: THERE, feeds: Some(4) });

    assert_eq!(living_there(&simulation), 4);
    assert_eq!(simulation.population.agents[0].hearth, Some(THERE), "the one who decided goes");
    let stayed = simulation.population.agents.iter().filter(|agent| agent.hearth == Some(HERE)).count();
    assert_eq!(stayed, 2);

    // And each lot sleeps at its own camp
    let gone = simulation.population.agents.iter().position(|a| a.hearth == Some(THERE)).unwrap();
    let left = simulation.population.agents.iter().position(|a| a.hearth == Some(HERE)).unwrap();
    assert_eq!(simulation.where_this_one_sleeps(gone), Some(THERE));
    assert_eq!(simulation.where_this_one_sleeps(left), Some(HERE));
}

/// Whoever the one deciding trusts most goes first.
#[test]
fn the_most_trusted_go_first() {
    use crate::agents::emotions::{Relationship, RelationshipType};

    let mut simulation = a_camp(4);
    let trusted = simulation.population.agents[3].id;
    let mut friend = Relationship::new(trusted, RelationshipType::Friend);
    friend.bond_strength = 1.0;
    simulation.population.agents[0].relationships.add_relationship(friend);

    let who = simulation.population.agents[0].id;
    simulation.the_camp_moves(CampMove { who, to: THERE, feeds: Some(2) });
    assert_eq!(simulation.population.agents[3].hearth, Some(THERE));
    assert_eq!(living_there(&simulation), 2);
}

/// Somebody with a crop standing by the camp stays with it.
#[test]
fn a_crop_standing_keeps_its_farmer() {
    use crate::world::{Position, ResourceNode, ResourceType, TerrainType};

    let mut simulation = a_camp(3);
    let field = Position::new(HERE.0 + 3, HERE.1);
    if let Some(tile) = simulation.world.grid.get_tile_mut(&field) {
        tile.terrain.terrain_type = TerrainType::Farmland;
    }
    simulation.world.resources.push(ResourceNode::new(ResourceType::Grain, field, 40));
    // The one who decided goes; nobody else leaves a crop in the ground
    let who = simulation.population.agents[0].id;
    simulation.the_camp_moves(CampMove { who, to: THERE, feeds: None });
    assert_eq!(living_there(&simulation), 1, "only the one who decided");
}

/// A child not yet grown goes with a parent who goes.
#[test]
fn a_child_goes_with_its_parent() {
    let mut simulation = a_camp(2);
    simulation.population.spawn_agent(AgentConfig::default());
    let parent = simulation.population.agents[0].id;
    let child = &mut simulation.population.agents[2];
    child.state.now_this_many_years_old(8);
    child.state.life_stage = LifeStage::Child;
    child.parent_ids = vec![parent];
    child.hearth = Some(HERE);

    simulation.the_camp_moves(CampMove { who: parent, to: THERE, feeds: Some(1) });
    assert_eq!(simulation.population.agents[0].hearth, Some(THERE));
    assert_eq!(simulation.population.agents[1].hearth, Some(HERE), "no room for the other grown one");
    assert_eq!(simulation.population.agents[2].hearth, Some(THERE), "the child goes with its parent");
    assert_eq!(simulation.where_this_one_sleeps(2), Some(THERE));
}

// --------------------------------------------------------------------------
// Farmers and ploughland
// --------------------------------------------------------------------------

/// Lay `kind` down over every tile within `reach` of `at`.
fn ground_of(simulation: &mut Simulation, at: (i32, i32), reach: i32, kind: crate::world::TerrainType) {
    for dx in -reach..=reach {
        for dy in -reach..=reach {
            let tile_at = crate::world::Position::new(at.0 + dx, at.1 + dy);
            if let Some(tile) = simulation.world.grid.get_tile_mut(&tile_at) {
                tile.terrain.terrain_type = kind;
            }
        }
    }
}

/// A camp on ground a plough will hardly take, whose people know of wild grain
/// growing on good meadow at `THERE`. `farmers` of them farm.
fn a_camp_on_poor_ground(farmers: usize) -> Simulation {
    use crate::agents::practices::Practice;
    use crate::world::{Position, ResourceType, TerrainType};

    let mut simulation = a_camp(4);
    ground_of(&mut simulation, HERE, 14, TerrainType::Hills);
    ground_of(&mut simulation, (HERE.0, HERE.1 + 6), 2, TerrainType::Plains);
    ground_of(&mut simulation, THERE, 14, TerrainType::Meadow);
    let grain = Position::new(THERE.0, THERE.1);
    for (index, agent) in simulation.population.agents.iter_mut().enumerate() {
        agent.exploration_knowledge.discover_resource(grain, ResourceType::Grain, 0);
        if index < farmers {
            agent.practices.saw_it_work(Practice::Farming);
            agent.practices.saw_it_work(Practice::Farming);
        }
    }
    simulation
}

/// Set the clock to a half hour when this one thinks about where to live.
fn their_time_to_think(simulation: &mut Simulation, index: usize) {
    for period in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
        simulation.current_turn = 10 * crate::environment::seasons::TICKS_PER_DAY
            + period * crate::environment::seasons::TICKS_BETWEEN_PLANS;
        if simulation.time_to_think_about_where_to_live(&simulation.population.agents[index]) {
            return;
        }
    }
    panic!("nobody has no time of day to think");
}

/// A farmer on poor ground takes the camp to the good ploughland they know of.
#[test]
fn a_farmer_takes_the_camp_to_better_ploughland() {
    let mut simulation = a_camp_on_poor_ground(1);
    their_time_to_think(&mut simulation, 0);

    let position = simulation.population.agents[0].state.position;
    match simulation.moving_to_farmland(&simulation.population.agents[0], position) {
        Some(crate::environment::Action::Move { target }) => assert_eq!((target.0, target.1), THERE),
        other => panic!("a farmer on hill ground who knows of meadow should move: {other:?}"),
    }
    let proposed = simulation.camp_move_proposed.borrow_mut().take().expect("a move of camp");
    assert_eq!(proposed.to, THERE);

    // And the people who live with them go too
    simulation.the_camp_moves(proposed);
    assert_eq!(living_there(&simulation), 4);
}

/// Somebody who has not taken up farming does not move for ploughland.
#[test]
fn a_forager_does_not_move_for_ploughland() {
    let mut simulation = a_camp_on_poor_ground(0);
    their_time_to_think(&mut simulation, 0);
    let position = simulation.population.agents[0].state.position;
    assert!(simulation.moving_to_farmland(&simulation.population.agents[0], position).is_none());
}

/// Nor does a farmer whose ground is as good as what they know of.
#[test]
fn a_farmer_on_good_ground_stays() {
    use crate::world::TerrainType;

    let mut simulation = a_camp_on_poor_ground(1);
    ground_of(&mut simulation, HERE, 14, TerrainType::Meadow);
    their_time_to_think(&mut simulation, 0);
    let position = simulation.population.agents[0].state.position;
    assert!(simulation.moving_to_farmland(&simulation.population.agents[0], position).is_none());
}

/// Nor for good ground nobody has seen.
#[test]
fn nobody_moves_for_ploughland_they_do_not_know_of() {
    let mut simulation = a_camp_on_poor_ground(1);
    simulation.population.agents[0].exploration_knowledge.known_resources.clear();
    their_time_to_think(&mut simulation, 0);
    let position = simulation.population.agents[0].state.position;
    assert!(simulation.moving_to_farmland(&simulation.population.agents[0], position).is_none());
}
