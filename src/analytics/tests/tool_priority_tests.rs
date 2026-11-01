// src/analytics/tests/tool_priority_tests.rs
//! Tools made in the order of the time they save (ISSUES_FOUND #387).
//!
//! "Tool making should be prioritized based on the amount of time it saves."
//!
//! - the record of work at a trade falls by half in a season
//! - somebody with no work recorded keeps the list's order
//! - a gardener whose days go to the garden wants a farming tool first, and a
//!   hunter whose days go to hunting a hunting tool first
//! - a tool that would not repay its making for the work done is not wanted
//! - somebody whose days go to gathering wants a tool for it first
//! - weeding a field is counted as work at farming

use crate::agents::{Agent, AgentConfig, SkillType};
use crate::environment::seasons::{DAYS_PER_SEASON, TICKS_PER_DAY};

fn a_gardener() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.now_this_many_years_old(30);
    agent.inventory.get_all_items_mut().clear();
    agent.found_out_how_to("do:ScatterSeed");
    agent.found_out_how_to("do:TillSoil");
    agent
}

/// What is done at a trade counts for half as much a season on.
#[test]
fn work_at_a_trade_falls_by_half_in_a_season() {
    let mut agent = a_gardener();
    agent.a_turn_at(SkillType::Farming, 0);
    let a_season = DAYS_PER_SEASON * TICKS_PER_DAY;
    let then = agent.how_much_work_goes_to(SkillType::Farming, a_season);
    assert!((then - 0.5).abs() < 0.01, "{then}");
}

/// Nobody with no work on record has any reason to change the list's order.
#[test]
fn somebody_with_no_work_recorded_keeps_the_lists_order() {
    let agent = a_gardener();
    let order = agent.trades_i_want_tools_for();
    assert_eq!(&order[..Agent::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO.len()], &Agent::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO);
    assert_eq!(order.len(), Agent::WHAT_A_PAIR_OF_HANDS_WANTS_TO_DO.len(), "gathering and gardening once done");
}

/// A gardener whose days go to the garden wants a tool for it before a
/// spear; a hunter whose days go to hunting wants the spear first.
#[test]
fn the_trade_the_days_go_to_comes_first() {
    let mut gardener = a_gardener();
    for turn in 0..40 {
        gardener.a_turn_at(SkillType::Farming, turn);
    }
    gardener.a_turn_at(SkillType::Hunting, 40);
    assert_eq!(gardener.trades_i_want_tools_for().first(), Some(&SkillType::Farming));
    assert!(
        !gardener.trades_i_want_tools_for().contains(&SkillType::Hunting),
        "a spear for one turn's hunting would never repay its making"
    );

    let mut hunter = a_gardener();
    for turn in 0..40 {
        hunter.a_turn_at(SkillType::Hunting, turn);
    }
    hunter.a_turn_at(SkillType::Farming, 40);
    assert_eq!(hunter.trades_i_want_tools_for().first(), Some(&SkillType::Hunting));
}

/// Gathering is most of a forager's day, and a digging stick the first tool
/// worth making for it.
#[test]
fn a_forager_wants_a_tool_for_gathering_first() {
    let mut forager = Agent::new(AgentConfig::default());
    forager.state.now_this_many_years_old(30);
    forager.inventory.get_all_items_mut().clear();
    for turn in 0..60 {
        forager.a_turn_at(SkillType::Herbalism, turn);
    }
    assert_eq!(forager.trades_i_want_tools_for().first(), Some(&SkillType::Herbalism));
    assert_eq!(
        forager.what_i_would_rather_have(SkillType::Herbalism).map(|tool| tool.called),
        Some("diggingstick")
    );
}

/// Weeding a field is a turn at farming.
#[test]
fn weeding_a_field_is_counted_as_farming() {
    use crate::agents::Population;
    use crate::analytics::Simulation;
    use crate::environment::Action;
    use crate::world::{Position, Terrain, TerrainType, World, WorldConfig};

    let where_it_is = Position::new(25, 25);
    let mut world = World::new(WorldConfig::default());
    world.resources.retain(|resource| resource.position != where_it_is);
    if let Some(tile) = world.grid.get_tile_mut(&where_it_is) {
        tile.terrain = Terrain::new(TerrainType::Farmland);
        tile.soil.weeds = crate::world::soil::Soil::OVERRUN;
    }
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (where_it_is.x, where_it_is.y, 0);
    assert_eq!(
        simulation.what_trade_this_asks_for(&Action::TendField, 0),
        Some(SkillType::Farming)
    );
}
