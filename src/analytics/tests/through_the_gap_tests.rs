// src/analytics/tests/through_the_gap_tests.rs
//! What stood between a starving settlement and its own larder.
//!
//! Every settlement starved in months ten to twelve with thousands of items
//! still in its pits. Traced person by person, besides the rot in their packs
//! (`full_pack_tests::a_pack_full_of_rot_makes_room_for_the_larder`), two
//! things kept a body from the food: the shelter override took the turns of a
//! hungry man with supper in his pack, and a half-built burrow was a wall. See
//! ISSUES_FOUND #252.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::exposure::ExposureType;
use crate::environment::Action;
use crate::world::{Building, BuildingType, Position, World, WorldConfig};

/// One person, cold, with a roof beside them and a day's roots in the pack.
fn cold_with_supper_and_a_roof(hungry: bool) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.resources.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let here = simulation.population.agents[0].state.position;
    simulation
        .world
        .buildings
        .push(Building::new(BuildingType::Burrow, Position::new(here.0 + 1, here.1)));

    let mut roots = InventoryItem::new_with_weight("roots".to_string(), 12, 1.0);
    roots.food_data = simulation
        .food_database
        .create_food_data(&crate::world::ItemType::Roots, 0);
    let agent = &mut simulation.population.agents[0];
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.get_all_items_mut().insert("roots".to_string(), roots);
    agent
        .exposure_status
        .active_exposures
        .push(ExposureType::Hypothermia);
    if let Some(hunger) = agent.drives.get_mut(DriveType::Hunger) {
        hunger.value = if hungry { 0.9 } else { 0.0 };
    }

    simulation
}

/// Somebody hungry eats before they huddle.
///
/// The shelter override fires on being cold at all, and through the hungry
/// gap everybody is cold every turn - so it took every turn a body had,
/// including from somebody with supper in the pack, until the last quarter of
/// the reserve. Eating is one turn, and it can be done under a roof.
#[test]
fn somebody_cold_and_hungry_eats_what_they_carry_first() {
    let simulation = cold_with_supper_and_a_roof(true);
    let agent = simulation.population.agents[0].clone();
    assert!(agent.needs_shelter(), "the fixture wants somebody cold");
    assert!(
        simulation.nearest_shelter_from(agent.state.position).is_some(),
        "and a roof within reach"
    );

    let (action, _) = simulation.generate_non_emotional_action(&agent, agent.state.position);
    assert!(
        matches!(action, Action::Eat { .. }),
        "cold and hungry with supper in the pack, they were sent to {action:?}"
    );
}

/// And somebody cold and fed still goes in out of the cold: this does not
/// narrow the shelter rule, it puts a meal in front of it.
#[test]
fn somebody_cold_and_fed_still_goes_to_the_roof() {
    let simulation = cold_with_supper_and_a_roof(false);
    let agent = simulation.population.agents[0].clone();

    let (action, _) = simulation.generate_non_emotional_action(&agent, agent.state.position);
    assert!(
        matches!(action, Action::SeekShelter),
        "cold and fed, they were sent to {action:?} rather than the roof"
    );
}

/// A half-built burrow is a hole you can climb across.
///
/// A site used to block, as scaffolding. Traced through a winter: somebody on
/// a hillside between three burrows begun and never finished, and a river on
/// the fourth side, refused a step 14,560 times towards a larder eight paces
/// off.
#[test]
fn a_building_site_is_not_a_wall() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    let here = (10, 10, 0);
    simulation.population.agents[0].state.position = here;
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let at = Position::new(here.0 + dx, here.1 + dy);
        if let Some(tile) = simulation.world.grid.get_tile_mut(&at) {
            tile.terrain = crate::world::Terrain::new(crate::world::TerrainType::Meadow);
        }
        simulation
            .world
            .buildings
            .push(Building::new_under_construction(BuildingType::Burrow, at));
    }

    assert!(simulation.is_passable_tile(here.0 + 1, here.1));

    let result = simulation.execute_action(
        &Action::Move {
            target: (here.0 + 5, here.1, 0),
        },
        0,
    );
    assert!(result.success, "boxed in by building sites: {:?}", result.message);
    assert_ne!(simulation.population.agents[0].state.position, here, "and they did not move");
}

/// Nobody tastes a strange plant who could not survive the worst of one.
///
/// Eighteen people in twelve settlement-years died of a mouthful, because
/// nothing asked what condition the taster was in and a winter leaves
/// everybody in poor condition.
#[test]
fn the_worst_plant_leaves_a_taster_standing() {
    let (_, worst) = Simulation::WHAT_A_BAD_PLANT_DOES;
    assert!(
        Simulation::WELL_ENOUGH_TO_RISK_IT - worst > 0.0,
        "a taster at {} health is killed by a plant doing {worst}",
        Simulation::WELL_ENOUGH_TO_RISK_IT
    );

    let mut simulation = cold_with_supper_and_a_roof(false);
    simulation.population.agents[0].state.health = Simulation::WELL_ENOUGH_TO_RISK_IT - 1.0;
    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation.tasting_action(&agent, agent.state.position).is_none(),
        "somebody under the line was let near a strange plant"
    );
}

/// A walk out of a pocket goes out of the pocket.
///
/// Somebody in a bay of water that opens behind them, heading for somewhere
/// past its far wall. The direct step is clear until the wall and the way
/// round lies back the way they came, so taking the direct step whenever it
/// was clear and asking the search only when it was not had them step to the
/// wall, be sent back a pace, step to the wall again - for as long as they
/// lived. Traced through a winter, two days of it on the way to a pit nine
/// paces off. See ISSUES_FOUND #254.
#[test]
fn a_walk_does_not_bounce_off_the_end_of_a_bay() {
    use crate::world::{Terrain, TerrainType};

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);

    // Open ground, then a bay of water: walls along y = 3 and y = 9 and across
    // x = 25, open to the east.
    for x in 10..45 {
        for y in 0..15 {
            let ground = if (y == 3 || y == 9) && (25..=35).contains(&x) || (x == 25 && (3..=9).contains(&y)) {
                TerrainType::Water
            } else {
                TerrainType::Meadow
            };
            if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain = Terrain::new(ground);
            }
        }
    }
    simulation.world.buildings.clear();

    let target = (19, 6, 0);
    simulation.population.agents[0].state.position = (28, 6, 0);
    simulation.population.agents[0].stepped_from = None;

    let mut got_there = false;
    for _ in 0..80 {
        simulation.execute_action(&Action::Move { target }, 0);
        let at = simulation.population.agents[0].state.position;
        if (at.0, at.1) == (target.0, target.1) {
            got_there = true;
            break;
        }
    }

    let at = simulation.population.agents[0].state.position;
    assert!(got_there, "eighty steps and still at {at:?}, short of {target:?}");
}

/// A parent with a small child at their feet is not sent to where they are
/// already standing because there is a wolf about.
///
/// The walk to a child in danger was a `Move` to the child's tile, which is
/// the parent's own when the child is held, and the walker books that as
/// done. The branch sits above eating, so for as long as a wolf hung about
/// the parent did nothing at all: traced, eight days of it standing on a
/// pit. See ISSUES_FOUND #255.
#[test]
fn a_parent_holding_a_child_is_not_sent_to_where_they_stand() {
    use crate::agents::Agent;

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("wolf".to_string(), (33, 30))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let parent = simulation.population.agents[0].id;
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.now_this_many_years_old(30);

    let mut child = Agent::with_parents(AgentConfig::default(), vec![parent], simulation.current_turn);
    child.state.position = (30, 30, 0);
    simulation.population.agents.push(child);

    let agent = simulation.population.agents[0].clone();
    let answer = simulation.protective_action(&agent, agent.state.position);
    assert!(
        !matches!(answer, Some(Action::Move { target }) if (target.0, target.1) == (30, 30)),
        "a parent holding their child was sent to the tile they stand on: {answer:?}"
    );

    // And one whose child is old enough to wander, and a few paces off with a
    // wolf beside it, still goes to it. A small child is never a few paces
    // off: it is carried - see `a_parent_is_not_sent_back_for_the_child_in_their_arms`.
    simulation.population.agents[1].state.now_this_many_years_old(8);
    simulation.population.agents[1].update_life_stage();
    simulation.population.agents[1].state.position = (32, 30, 0);
    let agent = simulation.population.agents[0].clone();
    assert!(
        matches!(simulation.protective_action(&agent, agent.state.position), Some(Action::Move { target }) if (target.0, target.1) == (32, 30)),
        "a child with a wolf beside it and its parent away should bring the parent"
    );
}

/// Somebody a little under half their reserve and eating is not dying.
///
/// Wasting took five health a day at any depth, so a body that fell just past
/// the line and was climbing back died three weeks later whatever it ate.
/// See ISSUES_FOUND #255.
#[test]
fn a_body_just_past_the_line_is_not_on_a_clock() {
    use crate::agents::Agent;
    use crate::environment::seasons::{PLANNING_PERIODS_PER_DAY, TICKS_BETWEEN_PLANS};

    let mut just_past = Agent::new(AgentConfig::default());
    let mut near_empty = Agent::new(AgentConfig::default());
    for (agent, share) in [(&mut just_past, 0.48), (&mut near_empty, 0.05)] {
        agent.state.now_this_many_years_old(30);
        let capacity = agent.state.physiology.reserve_capacity;
        agent.state.physiology.reserve = capacity * share;
    }

    let a_day = PLANNING_PERIODS_PER_DAY;
    for turn in 0..a_day {
        for agent in [&mut just_past, &mut near_empty] {
            // Kept where they are: this is about the damage, not the eating.
            let held = agent.state.physiology.reserve;
            agent.state.physiology.hydration = 1.0;
            agent.process_survival_turn(turn * TICKS_BETWEEN_PLANS);
            agent.state.physiology.reserve = held;
        }
    }

    let lost_just_past = 100.0 - just_past.state.health;
    let lost_near_empty = 100.0 - near_empty.state.health;
    assert!(
        lost_just_past < 0.5,
        "a day at 0.48 of the reserve cost {lost_just_past:.2} health"
    );
    assert!(
        lost_near_empty > 3.0,
        "and a day at the bottom of it should still hurt: {lost_near_empty:.2}"
    );
}

/// A small child is with one parent, and the other is not sent after it.
///
/// A child under six is kept on the first of its parents still living, so to
/// the other it was always wherever that one was - past the leash, or near a
/// wolf - and the other walked after it: a quarter of every parent's turns.
/// See ISSUES_FOUND #256.
#[test]
fn the_parent_not_holding_a_small_child_is_not_sent_after_it() {
    use crate::agents::Agent;

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("wolf".to_string(), (33, 30))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    for (i, at) in [(0usize, (50, 30, 0)), (1, (30, 30, 0))] {
        simulation.population.agents[i].state.position = at;
        simulation.population.agents[i].state.now_this_many_years_old(30);
    }
    let (holding, other) = (simulation.population.agents[0].id, simulation.population.agents[1].id);

    let mut child = Agent::with_parents(AgentConfig::default(), vec![holding, other], simulation.current_turn);
    child.state.position = (50, 30, 0);
    simulation.population.agents.push(child);
    assert_eq!(
        simulation.who_a_small_child_is_kept_with(&simulation.population.agents[2]),
        Some(holding)
    );

    let the_other = simulation.population.agents[1].clone();
    let answer = simulation.protective_action(&the_other, the_other.state.position);
    assert!(
        !matches!(answer, Some(Action::Move { .. })),
        "the parent twenty paces off was sent after a child in the other's arms: {answer:?}"
    );
}

/// Somebody who knows a plant is poison says so.
///
/// Only an onlooker close enough to watch a taster fall ill used to learn it,
/// so every person found each bad plant for themselves. See ISSUES_FOUND #258.
#[test]
fn a_poison_plant_is_passed_on_in_talk() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (20, 20, 0);
    simulation.population.agents[1].state.position = (20, 20, 0);
    simulation.population.agents[0].now_i_know_that_plant(3, false);
    simulation.population.agents[0].now_i_know_that_plant(5, true);

    assert!(!simulation.population.agents[1].have_i_tried_that_plant(3));
    let listener = simulation.population.agents[1].id;
    let said = simulation.execute_action(&Action::ShareInformation { target_agent_id: listener }, 0);
    assert!(said.success, "{:?}", said.message);

    let heard = &simulation.population.agents[1];
    assert!(heard.have_i_tried_that_plant(3) && !heard.is_that_plant_food(3), "the warning did not reach them");
    assert!(!heard.have_i_tried_that_plant(5), "and a good one is still theirs to find out");
}

/// A parent is not sent back for the child in their arms.
///
/// A carried child catches up with its carrier at the start of the next turn,
/// so a step left it a pace behind, and with a wolf about the parent went
/// back for it - a step there and a step back, turn after turn. See
/// ISSUES_FOUND #259.
#[test]
fn a_parent_is_not_sent_back_for_the_child_in_their_arms() {
    use crate::agents::Agent;

    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
        .spawn_animal("wolf".to_string(), (33, 30))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let parent = simulation.population.agents[0].id;
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.now_this_many_years_old(30);

    // A pace behind, the way a carried child is after one step.
    let mut child = Agent::with_parents(AgentConfig::default(), vec![parent], simulation.current_turn);
    child.state.position = (31, 30, 0);
    simulation.population.agents.push(child);

    let agent = simulation.population.agents[0].clone();
    let answer = simulation.protective_action(&agent, agent.state.position);
    assert!(
        !matches!(answer, Some(Action::Move { .. })),
        "a parent was sent back a pace for a child they are carrying: {answer:?}"
    );
}

/// Parents hand a small child between them, and whoever is carrying it feeds
/// it.
///
/// It stayed six years with the first of its parents still living, who ate
/// for two and never caught up while the other ate for one. See
/// ISSUES_FOUND #259.
#[test]
fn a_small_child_is_handed_to_the_better_fed_parent() {
    use crate::agents::Agent;

    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    for (i, spare) in [(0usize, 0.55), (1, 0.9)] {
        let agent = &mut simulation.population.agents[i];
        agent.state.now_this_many_years_old(30);
        agent.state.position = (20, 20, 0);
        agent.state.physiology.reserve = agent.state.physiology.reserve_capacity * spare;
    }
    let (worn, fresh) = (simulation.population.agents[0].id, simulation.population.agents[1].id);
    let mut child = Agent::with_parents(AgentConfig::default(), vec![worn, fresh], simulation.current_turn);
    child.state.position = (20, 20, 0);
    simulation.population.agents.push(child);

    assert_eq!(simulation.who_a_small_child_is_kept_with(&simulation.population.agents[2]), Some(worn));

    // Apart, nothing can be handed over.
    simulation.population.agents[1].state.position = (40, 20, 0);
    simulation.hand_the_small_ones_over();
    assert_eq!(simulation.population.agents[2].carried_by, None, "handed across twenty paces");

    // Together, it goes to the one who has more to give.
    simulation.population.agents[1].state.position = (21, 20, 0);
    simulation.hand_the_small_ones_over();
    assert_eq!(simulation.who_a_small_child_is_kept_with(&simulation.population.agents[2]), Some(fresh));

    simulation.the_small_stay_with_their_people();
    assert_eq!(simulation.population.agents[2].state.position, (21, 20, 0), "and goes with them");

    simulation.feed_the_small_children();
    assert!(simulation.population.agents[1].state.physiology.also_feeding > 0.0, "and they feed it");
    assert_eq!(simulation.population.agents[0].state.physiology.also_feeding, 0.0);

    // And it does not go straight back over a small difference.
    simulation.population.agents[1].state.physiology.reserve =
        simulation.population.agents[1].state.physiology.reserve_capacity * 0.6;
    simulation.hand_the_small_ones_over();
    assert_eq!(simulation.who_a_small_child_is_kept_with(&simulation.population.agents[2]), Some(fresh));
}
