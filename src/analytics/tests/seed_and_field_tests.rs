// src/analytics/tests/seed_and_field_tests.rs
//! Stage 3 of the agricultural path: the seasons kept in mind, seed saved,
//! seed chosen, fields hedged (ISSUES_FOUND #395 to #398).
//!
//! - a ripe crop of the camp's seen to fall, twice, is how the seasons come
//!   to mind
//! - somebody who keeps them brings a ripe crop in before its season is out,
//!   if it is worth the walk to them
//! - somebody who saves seed on purpose keeps a season's, not a handful
//! - breaking ground with no seed kept back is how saving it is found out
//! - seed saved off a ripe field carries what the field was bred from into
//!   the next one
//! - choosing the heaviest stand to save from breeds the seed on a step
//! - saving seed off enough harvests is how choosing is found out
//! - a stand of bred seed carries half again what wild seed does
//! - a hedge keeps half the pests off a field
//! - weeding fields inside a hedge and outside is how hedging is found out
//! - the frugal hedge a field the lazy leave open
//! - an edge is laid round the fields and closed

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::soil::Soil;
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
    agent.found_out_how_to("do:ScatterSeed");
    agent.found_out_how_to("do:TillSoil");
    simulation
}

/// A field broken at `at`, with a ripe, full stand of grain bred `bred` on it.
fn a_ripe_field(simulation: &mut Simulation, at: Position, bred: f32) -> usize {
    assert!(simulation.world.grid.break_ground(&at, 0));
    let mut node = ResourceNode::new(ResourceType::Grain, at, Simulation::FIELD_YIELD);
    node.bred = bred;
    node.on_a_field = true;
    let yields = simulation.world.grid.what_it_yields_here_for(&at, ResourceType::Grain);
    node.amount = node.how_heavy_a_crop_it_carries(yields);
    node.it_has_ripened();
    simulation.world.put_a_node_down(node);
    simulation.world.resources.len() - 1
}

/// A handful for anybody who breaks ground; a season's for somebody who
/// saves seed on purpose.
#[test]
fn a_seed_saver_keeps_a_seasons_seed() {
    let mut simulation = a_farmer_at_home();
    let agent = &mut simulation.population.agents[0];
    agent.inventory.add_item(InventoryItem::new("grain".to_string(), 20));
    agent.keep_seed_back();
    assert_eq!(agent.how_many_i_have(Agent::SEED), Agent::A_HANDFUL_OF_SEED);
    agent.found_out_how_to("know:save_seed");
    agent.keep_seed_back();
    assert_eq!(agent.how_many_i_have(Agent::SEED), Agent::A_SEASONS_SEED);
}

/// Breaking ground with no seed kept back, three times, and somebody keeps
/// next year's seed back off the harvest.
#[test]
fn breaking_ground_without_seed_is_how_saving_is_found_out() {
    let mut simulation = a_farmer_at_home();
    for x in 0..Agent::TIMES_SOWN_WITHOUT_SEED_BEFORE_SAVING as i32 {
        assert!(!simulation.population.agents[0].knows_the_way("save_seed"));
        simulation.population.agents[0].state.position = (HOME.0 + x, HOME.1 + 3, 0);
        simulation.population.agents[0].inventory.add_item(InventoryItem::new("grain".to_string(), 1));
        let result = simulation.execute_action(&Action::TillSoil, 0);
        assert!(result.success, "{:?}", result.message);
    }
    assert!(simulation.population.agents[0].knows_the_way("save_seed"));
    assert!(simulation.population.agents[0].did_i_come_to_it_myself("know:save_seed"));
}

/// Seed saved off a ripe field carries what that field was bred from, and a
/// field sown of it carries it on.
#[test]
fn seed_saved_off_a_ripe_field_carries_its_breeding_on() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].found_out_how_to("know:save_seed");
    let here = Position::new(HOME.0, HOME.1);
    a_ripe_field(&mut simulation, here, 0.4);

    let result = simulation.execute_action(&Action::Gather { resource_type: "grain".to_string() }, 0);
    assert!(result.success, "{:?}", result.message);
    let agent = &simulation.population.agents[0];
    assert!(agent.how_many_i_have(Agent::SEED) > 0, "seed kept back off the harvest");
    assert!((agent.seed_line - 0.4).abs() < 1e-6, "{}", agent.seed_line);

    let next = Position::new(HOME.0 + 4, HOME.1);
    simulation.population.agents[0].state.position = (next.x, next.y, 0);
    let result = simulation.execute_action(&Action::TillSoil, 0);
    assert!(result.success, "{:?}", result.message);
    let sown = simulation.world.nodes_on(next).next().expect("a field sown");
    assert!((sown.bred - 0.4).abs() < 1e-6, "{}", sown.bred);
    assert!(sown.sown_on.is_some());
}

/// Choosing the heaviest stand to save from breeds the seed on a step; seed
/// saved off any stand that came does not, and nobody goes back to worse
/// seed than they keep.
#[test]
fn choosing_the_heaviest_breeds_the_seed_on() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.found_out_how_to("know:save_seed");
    agent.inventory.add_item(InventoryItem::new("grain".to_string(), 20));
    agent.save_seed_off(0.2, 5, true);
    assert!((agent.seed_line - 0.2).abs() < 1e-6, "no choosing without knowing to");

    agent.found_out_how_to("know:select_seed");
    agent.save_seed_off(0.2, 5, false);
    assert!((agent.seed_line - 0.2).abs() < 1e-6, "not for a stand that came short");
    agent.save_seed_off(0.2, 5, true);
    assert!((agent.seed_line - (0.2 + Agent::WHAT_CHOOSING_THE_HEAVIEST_DOES)).abs() < 1e-6, "{}", agent.seed_line);
    let line = agent.seed_line;
    agent.save_seed_off(0.2, 5, true);
    assert!((agent.seed_line - line).abs() < 1e-6, "the same field again breeds nothing more: {}", agent.seed_line);
    agent.save_seed_off(0.0, 5, false);
    assert!((agent.seed_line - line).abs() < 1e-6, "wild seed does not undo it");
}

/// Saving seed off enough harvests, and somebody saves it off the heaviest.
#[test]
fn saving_seed_off_harvests_is_how_choosing_is_found_out() {
    let mut agent = Agent::new(AgentConfig::default());
    agent.found_out_how_to("know:save_seed");
    for _ in 0..Agent::HARVESTS_SAVED_FROM_BEFORE_SELECTING {
        assert!(!agent.knows_the_way("select_seed"));
        agent.inventory.add_item(InventoryItem::new("grain".to_string(), 1));
        agent.save_seed_off(0.0, 1, true);
        agent.inventory.remove_item(Agent::SEED, 1);
    }
    assert!(agent.knows_the_way("select_seed"));
}

/// A stand sown of seed bred as far as it goes carries half again what wild
/// seed does.
#[test]
fn a_stand_of_bred_seed_carries_half_again() {
    let at = Position::new(1, 1);
    let wild = ResourceNode::new(ResourceType::Grain, at, Simulation::FIELD_YIELD);
    let mut bred = wild.clone();
    bred.bred = 1.0;
    let (w, b) = (wild.how_heavy_a_crop_it_carries(4.0), bred.how_heavy_a_crop_it_carries(4.0));
    assert_eq!(b * 2, w * 3, "{b} against {w}");
}

/// A hedge keeps the beasts off a field, which are half of what comes for a
/// crop; the weeds come on the same.
#[test]
fn a_hedge_keeps_half_the_pests_off() {
    let mut open = Soil::default();
    let mut hedged = Soil::default();
    for _ in 0..20 {
        open.nobody_weeded_this(1.0, 1.0, false);
        hedged.nobody_weeded_this(1.0, 1.0, true);
    }
    assert_eq!(open.weeds, hedged.weeds);
    assert!((hedged.pests * 2.0 - open.pests).abs() < 1e-5, "{} against {}", hedged.pests, open.pests);
}

/// Somebody who has weeded fields inside a hedge and outside sees which fare
/// better; somebody who has weeded only open ones does not.
#[test]
fn weeding_hedged_and_open_fields_is_how_hedging_is_found_out() {
    let mut agent = Agent::new(AgentConfig::default());
    for _ in 0..10 {
        agent.what_i_saw_of_a_field(false, 0.375);
    }
    assert!(!agent.what_the_fields_have_shown_me(), "nothing to compare them with");
    for _ in 0..10 {
        agent.what_i_saw_of_a_field(true, 0.3);
    }
    assert!(!agent.what_the_fields_have_shown_me(), "not plainly worse");
    for _ in 0..20 {
        agent.what_i_saw_of_a_field(true, 0.1);
    }
    assert!(agent.what_the_fields_have_shown_me());
    assert!(agent.did_i_come_to_it_myself("know:hedge_fields"));
}

/// Whether one open field is worth an edge is a matter of temperament: the
/// frugal hedge it, the lazy leave it; two are worth it to anybody.
#[test]
fn the_frugal_hedge_a_field_the_lazy_leave_open() {
    use crate::core::traits::{Trait, TraitSet};
    let mut simulation = a_farmer_at_home();
    let field = Position::new(HOME.0 + 3, HOME.1);
    assert!(simulation.world.grid.break_ground(&field, 0));
    let with = |t: Option<Trait>| {
        let mut agent = simulation.population.agents[0].clone();
        agent.traits = TraitSet::new();
        if let Some(t) = t {
            agent.traits.add_trait(t);
        }
        agent.found_out_how_to("know:hedge_fields");
        agent
    };
    assert_eq!(simulation.a_field_to_hedge(&with(Some(Trait::Frugal))), Some(field));
    assert_eq!(simulation.a_field_to_hedge(&with(None)), None, "one field is not worth it to most");

    assert!(simulation.world.grid.break_ground(&Position::new(field.x + 1, field.y), 0));
    let with = |t: Option<Trait>| {
        let mut agent = simulation.population.agents[0].clone();
        agent.traits = TraitSet::new();
        if let Some(t) = t {
            agent.traits.add_trait(t);
        }
        agent.found_out_how_to("know:hedge_fields");
        agent
    };
    assert_eq!(simulation.a_field_to_hedge(&with(None)), Some(field), "two are");
    assert_eq!(simulation.a_field_to_hedge(&with(Some(Trait::Lazy))), None);
}

/// Somebody who knows to, standing in an open field with brush in hand, lays
/// an edge round it; when it holds what an edge takes it is closed, and the
/// fields in it are hedged.
#[test]
fn an_edge_is_laid_round_the_fields_and_closed() {
    let mut simulation = a_farmer_at_home();
    let field = Position::new(HOME.0 + 3, HOME.1);
    assert!(simulation.world.grid.break_ground(&field, 0));
    assert!(simulation.world.grid.break_ground(&Position::new(field.x + 1, field.y), 0));
    let agent = &mut simulation.population.agents[0];
    agent.found_out_how_to("know:hedge_fields");
    agent.inventory.add_item(InventoryItem::new("wood".to_string(), 3));

    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.hedging_action(&agent, agent.state.position),
        Some(Action::Move { target: (field.x, field.y, 0) })
    );
    simulation.population.agents[0].state.position = (field.x, field.y, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.hedging_action(&agent, agent.state.position), Some(Action::HedgeAField));

    let result = simulation.execute_action(&Action::HedgeAField, 0);
    assert!(result.success, "{:?}", result.message);
    assert_eq!(simulation.world.fields_hedged, 0, "three armfuls do not close it");

    simulation.population.agents[0].inventory.add_item(InventoryItem::new("wood".to_string(), 10));
    let result = simulation.execute_action(&Action::HedgeAField, 0);
    assert!(result.success, "{:?}", result.message);
    assert_eq!(simulation.world.fields_hedged, 1);
    assert!(simulation.is_this_field_hedged((field.x + 1, field.y)));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_field_to_hedge(&agent), None, "nothing left open");
}

/// The day of the year a grain field's season is `left` days from its end.
fn days_before_the_grain_is_over(left: u32) -> u32 {
    use crate::environment::seasons::{last_day_of, PartOfSeason, Season};
    last_day_of(Season::Fall, PartOfSeason::Deep) - left
}

/// How many days of its season a crop has left, and none outside it.
#[test]
fn a_season_says_how_many_days_it_has_left() {
    let grain = ResourceType::Grain.bearing_window();
    assert_eq!(grain.days_left(days_before_the_grain_is_over(10)), Some(10));
    assert_eq!(grain.days_left(days_before_the_grain_is_over(0)), Some(0));
    assert_eq!(grain.days_left(days_before_the_grain_is_over(0) + 1), None);
}

/// A ripe crop of the camp's seen to fall, twice, and somebody keeps the
/// seasons in mind; somebody living a long way off sees nothing of it.
#[test]
fn a_ripe_crop_seen_to_fall_is_how_the_seasons_come_to_mind() {
    let mut simulation = a_farmer_at_home();
    let far = simulation.population.agents[0].clone();
    simulation.population.agents.push(far);
    simulation.population.agents[1].id = uuid::Uuid::from_u128(0x5ea5_0115);
    simulation.population.agents[1].hearth = Some((HOME.0 + 15, HOME.1 + 15));
    for _ in 0..Agent::CROPS_LOST_BEFORE_THE_SEASONS {
        assert!(!simulation.population.agents[0].knows_the_way("seasons"));
        simulation.world.crops_lost_to_the_season.push(Position::new(HOME.0 + 3, HOME.1));
        simulation.what_the_fields_lost();
    }
    assert!(simulation.population.agents[0].knows_the_way("seasons"));
    assert!(!simulation.population.agents[1].knows_the_way("seasons"), "not their fields");
    assert!(simulation.world.crops_lost_to_the_season.is_empty());
}

/// Field crop that falls is counted by how far it was from the nearest
/// home (#424): within a walk, within two, and farther.
#[test]
fn fallen_crop_is_counted_by_how_far_it_was_from_home() {
    let mut simulation = a_farmer_at_home();
    let walk = Simulation::FIELD_WALK_RADIUS as i32;
    simulation.world.field_crop_fallen_where.push((Position::new(HOME.0 + 3, HOME.1), 5));
    simulation.world.field_crop_fallen_where.push((Position::new(HOME.0, HOME.1 + walk + 1), 7));
    simulation.world.field_crop_fallen_where.push((Position::new(HOME.0 + 2 * walk + 1, HOME.1), 11));
    simulation.where_the_field_crop_fell();
    assert_eq!(simulation.world.field_crop_fallen_by_distance, [5, 7, 11]);
    assert!(simulation.world.field_crop_fallen_where.is_empty());
}

/// Somebody who keeps the seasons brings a ripe crop of the camp's in when
/// its season is nearly out, hungry or not; not while there is time yet, and
/// a lazy man not for a few heads.
#[test]
fn somebody_who_keeps_the_seasons_brings_the_harvest_in() {
    use crate::core::traits::{Trait, TraitSet};
    let mut simulation = a_farmer_at_home();
    let field = Position::new(HOME.0 + 3, HOME.1);
    a_ripe_field(&mut simulation, field, 0.0);
    simulation.world.climate.calendar.day_of_year = days_before_the_grain_is_over(10);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.harvest_action(&agent, agent.state.position), None, "nobody who does not keep them");

    simulation.population.agents[0].found_out_how_to("know:seasons");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.harvest_action(&agent, agent.state.position),
        Some(Action::Move { target: (field.x, field.y, 0) })
    );
    simulation.population.agents[0].state.position = (field.x, field.y, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.harvest_action(&agent, agent.state.position),
        Some(Action::Gather { resource_type: "grain".to_string() })
    );

    simulation.world.climate.calendar.day_of_year = days_before_the_grain_is_over(60);
    assert_eq!(simulation.harvest_action(&agent, agent.state.position), None, "time yet");

    simulation.world.climate.calendar.day_of_year = days_before_the_grain_is_over(10);
    let index = simulation.world.resources.len() - 1;
    let node = &mut simulation.world.resources[index];
    node.taken_since_it_ripened = node.the_harvest() - 3;
    let mut lazy = simulation.population.agents[0].clone();
    lazy.traits = TraitSet::new();
    lazy.traits.add_trait(Trait::Lazy);
    assert_eq!(simulation.harvest_action(&lazy, lazy.state.position), None, "three heads are not worth it to him");
    let mut frugal = lazy.clone();
    frugal.traits = TraitSet::new();
    frugal.traits.add_trait(Trait::Frugal);
    assert!(simulation.harvest_action(&frugal, frugal.state.position).is_some(), "they are to her");
}
