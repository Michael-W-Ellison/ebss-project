// src/analytics/tests/crop_investment_tests.rs
//! A crop is an investment (#416): time and food put in against a hoped-for
//! return, and what it came to weighs on the people who put it in.
//!
//! - breaking ground, the seed sown and weeding are put into the crop
//! - a crop that falls with nothing brought in is settled against them
//! - a crop brought in is settled too, and pays
//! - a failed crop saddens its farmer, more as more of it was lost, and
//!   shakes their belief in farming and in that crop
//! - a temperament sets how hard it lands

use crate::agents::{Agent, AgentConfig, EmotionSource, InventoryItem, Population};
use crate::agents::practices::Practice;
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Position, ResourceNode, ResourceType, Terrain, TerrainType, World, WorldConfig};

const HOME: (i32, i32) = (20, 20);

/// Open plains with nothing on them, and one grown farmer at home.
fn a_farmer_at_home() -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(40, 40));
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.thorn_hedges.clear();
    for x in 0..40 {
        for y in 0..40 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    let agent = &mut simulation.population.agents[0];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some(HOME);
    agent.inventory.get_all_items_mut().clear();
    agent.inventory.max_weight = 500.0;
    agent.traits = crate::core::traits::TraitSet::new();
    agent.found_out_how_to("do:ScatterSeed");
    agent.found_out_how_to("do:TillSoil");
    simulation
}

/// A ripe field of grain at `at`, in its season, with `put_in` of the
/// farmer's energy in it.
fn a_ripe_field_with_work_in_it(simulation: &mut Simulation, at: Position, put_in: f32) -> usize {
    let in_season = (0..crate::environment::seasons::DAYS_PER_YEAR)
        .find(|day| ResourceType::Grain.is_it_bearing(*day) && ResourceType::Grain.is_it_bearing(*day + 5))
        .expect("grain bears some time in the year");
    simulation.world.climate.calendar.day_of_year = in_season;
    assert!(simulation.world.grid.break_ground(&at, 0));
    let mut node = ResourceNode::new(ResourceType::Grain, at, Simulation::FIELD_YIELD);
    node.on_a_field = true;
    let yields = simulation.world.grid.what_it_yields_here_for(&at, ResourceType::Grain);
    node.amount = node.how_heavy_a_crop_it_carries(yields);
    node.it_has_ripened();
    node.somebody_put_into_the_crop(simulation.population.agents[0].id, put_in);
    simulation.world.put_a_node_down(node);
    simulation.world.resources.len() - 1
}

fn a_day_goes_by(simulation: &mut Simulation) {
    for _ in 0..crate::environment::seasons::PLANNING_PERIODS_PER_DAY {
        simulation.world.take_a_turn();
    }
}

fn sadness_over_a_failed_crop(agent: &Agent) -> f32 {
    agent
        .emotions
        .sadness_sources
        .get(&EmotionSource::Event(Agent::A_CROP_THAT_FAILED.to_string()))
        .copied()
        .unwrap_or(0.0)
}

fn happiness_over_a_crop_that_paid(agent: &Agent) -> f32 {
    agent
        .emotions
        .happiness_sources
        .get(&EmotionSource::Event(Agent::A_CROP_THAT_PAID.to_string()))
        .copied()
        .unwrap_or(0.0)
}

/// The turn breaking ground and the handful of grain sown are put into the
/// crop, and so is every turn weeding it.
#[test]
fn sowing_and_weeding_are_put_into_the_crop() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("grain".to_string(), 3, 0.2));
    let who = simulation.population.agents[0].id;
    let here = Position::new(HOME.0, HOME.1);

    let tilled = simulation.execute_action(&Action::TillSoil, 0);
    assert!(tilled.success, "{:?}", tilled.message);
    let crop = simulation.world.node_numbers_on(here).first().copied().expect("a crop sown");
    let sowing = Simulation::what_a_turn_of_this_work_costs(Simulation::BREAKING_GROUND_COSTS)
        + simulation.what_an_item_of_this_crop_is_worth(ResourceType::Grain);
    assert_eq!(simulation.world.resources[crop].put_into_this_crop, vec![(who, sowing)]);

    if let Some(tile) = simulation.world.grid.get_tile_mut(&here) {
        for _ in 0..400 {
            tile.soil.nobody_weeded_this(1.0, 1.0, false);
        }
    }
    let weeded = simulation.execute_action(&Action::TendField, 0);
    assert!(weeded.success, "{:?}", weeded.message);
    let weeding = Simulation::what_a_turn_of_this_work_costs(Simulation::WEEDING_COSTS);
    let (_, put_in) = simulation.world.resources[crop].put_into_this_crop[0];
    assert!((put_in - (sowing + weeding)).abs() < 1e-3, "{put_in} put in");
}

/// A ripe crop that falls with nothing brought in is the work and the seed
/// thrown away: it is settled, and its farmer is saddened and trusts farming
/// and grain less.
#[test]
fn a_crop_that_falls_unharvested_weighs_on_its_farmer() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].practices.record_outcome(Practice::Farming, true);
    simulation.population.agents[0].practices.record_outcome(Practice::Farming, true);
    let believed = simulation.population.agents[0].practices.confidence(Practice::Farming);
    // A grain field's lesson is kept under the name kept seed goes by
    let grain = format!("sow:{}", Agent::SEED);
    // And a fair record of it, without which one crop says nothing yet
    for _ in 0..12 {
        simulation.population.agents[0].lessons.record_particular(&grain, true);
    }
    let thought_of_grain = simulation.population.agents[0].lessons.how_likely_to_try_this(&grain);

    let crop = a_ripe_field_with_work_in_it(&mut simulation, Position::new(HOME.0 + 3, HOME.1), 300.0);
    // It fell, or was eaten off, before anybody brought it in
    simulation.world.resources[crop].amount = 0;
    a_day_goes_by(&mut simulation);

    assert_eq!(simulation.world.crops_settled.len(), 1, "the crop is over");
    assert!(simulation.world.resources[crop].put_into_this_crop.is_empty());
    simulation.what_the_crops_came_to();
    assert!(simulation.world.crops_settled.is_empty());
    assert_eq!((simulation.world.crops_that_failed, simulation.world.crops_that_paid), (1, 0));

    let agent = &simulation.population.agents[0];
    assert!(
        (sadness_over_a_failed_crop(agent) - Agent::WHAT_A_FAILED_CROP_WEIGHS).abs() < 1e-4,
        "all of it lost: {}",
        sadness_over_a_failed_crop(agent)
    );
    assert!(agent.practices.confidence(Practice::Farming) < believed, "farming trusted less");
    assert!(agent.lessons.how_likely_to_try_this(&grain) < thought_of_grain, "and grain");
    assert_eq!(happiness_over_a_crop_that_paid(agent), 0.0);
}

/// A crop brought in gives back many times what went into it, and that is a
/// satisfaction, not a sorrow.
#[test]
fn a_crop_brought_in_pays_its_farmer() {
    let mut simulation = a_farmer_at_home();
    let crop = a_ripe_field_with_work_in_it(&mut simulation, Position::new(HOME.0 + 3, HOME.1), 300.0);
    let stand = simulation.world.resources[crop].ripe_stand;
    let took = simulation.world.resources[crop].harvest(stand);
    assert!(took > 0);
    a_day_goes_by(&mut simulation);

    assert_eq!(simulation.world.crops_settled.len(), 1, "the harvest is in");
    assert_eq!(simulation.world.crops_settled[0].2, took, "and what came off it is counted");
    simulation.what_the_crops_came_to();
    assert_eq!((simulation.world.crops_that_failed, simulation.world.crops_that_paid), (0, 1));

    let agent = &simulation.population.agents[0];
    assert_eq!(sadness_over_a_failed_crop(agent), 0.0);
    assert!(
        (happiness_over_a_crop_that_paid(agent) - Agent::WHAT_A_CROP_THAT_PAID_CHEERS).abs() < 1e-4,
        "{}",
        happiness_over_a_crop_that_paid(agent)
    );
}

/// What grows on after a crop is over had nothing put into it, and is
/// nobody's loss.
#[test]
fn a_crop_nobody_worked_is_nobodys_loss() {
    let mut simulation = a_farmer_at_home();
    let crop = a_ripe_field_with_work_in_it(&mut simulation, Position::new(HOME.0 + 3, HOME.1), 0.0);
    assert!(simulation.world.resources[crop].put_into_this_crop.is_empty());
    simulation.world.resources[crop].amount = 0;
    a_day_goes_by(&mut simulation);
    assert!(simulation.world.crops_settled.is_empty());
}

/// Half of it back is half a loss; all of it back is none. Each of two who
/// put work in is paid out of the crop as they paid in.
#[test]
fn a_failed_crop_weighs_as_much_as_was_lost() {
    let mut simulation = a_farmer_at_home();
    let other = simulation.population.agents[0].clone();
    simulation.population.agents.push(other);
    simulation.population.agents[1].id = uuid::Uuid::from_u128(0x0416);
    let (one, two) = (simulation.population.agents[0].id, simulation.population.agents[1].id);

    // A hundred and two hundred put in; back, what is worth a hundred and fifty
    let worth = simulation.what_an_item_of_this_crop_is_worth(ResourceType::Grain);
    let handfuls = (150.0 / worth).round() as u32;
    let came_back = handfuls as f32 * worth;
    simulation
        .world
        .crops_settled
        .push((ResourceType::Grain, vec![(one, 100.0), (two, 200.0)], handfuls));
    simulation.what_the_crops_came_to();

    for (index, theirs) in [(0, 100.0f32), (1, 200.0f32)] {
        let back = came_back * theirs / 300.0;
        let expected = Agent::WHAT_A_FAILED_CROP_WEIGHS * (1.0 - back / theirs);
        let felt = sadness_over_a_failed_crop(&simulation.population.agents[index]);
        assert!((felt - expected).abs() < 1e-4, "agent {index}: {felt} against {expected}");
    }
}

/// How hard a failed crop lands is a matter of temperament: harder on the
/// empathic, lighter on the cold-hearted.
#[test]
fn a_temperament_sets_how_hard_a_failed_crop_lands() {
    use crate::core::traits::{Trait, TraitSet};
    let felt = |t: Option<Trait>| {
        let mut agent = a_farmer_at_home().population.agents[0].clone();
        agent.traits = TraitSet::new();
        if let Some(t) = t {
            agent.traits.add_trait(t);
        }
        agent.what_my_crop_came_to(300.0, 0.0, Some("grain"));
        sadness_over_a_failed_crop(&agent)
    };
    let (plain, empathic, cold) = (felt(None), felt(Some(Trait::Empathic)), felt(Some(Trait::ColdHearted)));
    assert!(empathic > plain, "{empathic} against {plain}");
    assert!(cold < plain, "{cold} against {plain}");
}
