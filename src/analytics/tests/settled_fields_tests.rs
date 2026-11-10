// src/analytics/tests/settled_fields_tests.rs
//! Stage 4 of the agricultural path, settled field agriculture (ISSUES_FOUND
//! #417 to #422).
//!
//! - losing crops of one's own is how keeping the fields one has comes to
//!   mind (#417)
//! - somebody who keeps their fields breaks no more while their crops lately
//!   have been lost more often than paid, and does while they pay (#417)
//! - a keeper holds off while their camp's crops are lost, though their
//!   own pay (#423)
//! - where anybody of the camp keeps their fields, the rest go along with
//!   it while the camp's crops are lost, unless they go their own way, and
//!   break no ground a day out from home (#423)
//! - a field kept is broken about home, seed or no seed (#423)
//! - a crop is in its time before its season or early in it, and not late
//!   in it (#418)
//! - somebody who keeps a planting calendar sows only what is in its time,
//!   and breaks no ground while nothing they carry is (#418)
//! - crops of one's own ripening short, seen by somebody who keeps the
//!   seasons, is how the calendar comes to mind (#418)
//! - somebody who manures fields carries muck to the most worn field about
//!   home and spreads it there (#419)
//! - crop taken off worn fields, by somebody who knows muck, is how manuring
//!   comes to mind (#419)
//! - a granary holds four pits' worth and vermin hardly come at it (#420)
//! - the frugal raise a granary over the camp's stores that the lazy would
//!   leave in the ground (#420)
//! - a store of the camp's is raised into a granary, of wood fetched for it
//!   (#420)
//! - lined stores gnawed, opened by a farmer, is how the granary comes to
//!   mind (#420)
//! - two kinds of crop grown long enough, one plainly outpaying the other,
//!   make a staple; one crop is not a comparison (#421)
//! - a staple goes in first (#421)

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::environment::seasons::{Season, TICKS_PER_DAY};
use crate::environment::Action;
use crate::world::soil::SoilGrade;
use crate::world::{Belongs, Pit, Position, ResourceType, Terrain, TerrainType, World, WorldConfig};

const HOME: (i32, i32) = (20, 20);

/// Open plains with nothing on them, and one grown farmer at home who wants
/// to provide and has nothing more pressing.
fn a_farmer_at_home() -> Simulation {
    let mut world = World::new(WorldConfig::default().with_size(40, 40));
    world.animals.get_all_mut().clear();
    world.resources.clear();
    world.thorn_hedges.clear();
    world.pits.clear();
    for x in 0..40 {
        for y in 0..40 {
            if let Some(tile) = world.grid.get_tile_mut(&Position::new(x, y)) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }
    world.climate.calendar.day_of_year = Season::Spring.first_day() + 5;
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
    agent.practices.saw_it_work(crate::agents::practices::Practice::Farming);
    agent.practices.saw_it_work(crate::agents::practices::Practice::Farming);
    if let Some(drive) = agent.drives.get_mut(DriveType::Sustenance) {
        drive.value = 1.0;
    }
    if let Some(drive) = agent.drives.get_mut(DriveType::Hunger) {
        drive.value = 0.0;
    }
    simulation
}

fn grain(how_many: u32) -> InventoryItem {
    let mut grain = InventoryItem::new_with_weight("grain".to_string(), how_many, 0.2);
    grain.food_data = crate::world::nutrition::FoodDatabase::new().create_food_data(&crate::world::ItemType::Grain, 0);
    grain
}

// ---------------------------------------------------------------------------
// Permanent fields (#417)
// ---------------------------------------------------------------------------

/// Three crops of one's own lost is how keeping the fields one has comes to
/// mind, and only to somebody who lost them.
#[test]
fn losing_crops_is_how_keeping_fields_comes_to_mind() {
    let mut agent = a_farmer_at_home().population.agents[0].clone();
    for _ in 0..Agent::CROPS_LOST_BEFORE_KEEPING_FIELDS - 1 {
        agent.what_my_crop_came_to(100.0, 10.0, Some("grain"));
    }
    assert!(!agent.knows_the_way("keep_fields"), "not yet");
    agent.what_my_crop_came_to(100.0, 500.0, Some("grain"));
    assert!(!agent.knows_the_way("keep_fields"), "a crop that paid teaches nothing of it");
    agent.what_my_crop_came_to(100.0, 0.0, Some("grain"));
    assert!(agent.knows_the_way("keep_fields"));
    assert!(agent.did_i_come_to_it_myself("know:keep_fields"));
}

/// Somebody who keeps their fields breaks no new ground while their last
/// crops were lost more often than they paid; while they pay, they break
/// ground as anybody would. Somebody who does not keep fields breaks ground
/// either way.
#[test]
fn a_keeper_breaks_no_more_ground_while_the_crops_do_not_pay() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].inventory.add_item(grain(3));
    for paid in [false, false, true, false] {
        simulation.population.agents[0].crops_lately.push(paid);
    }
    assert!(!simulation.population.agents[0].are_my_crops_paying());
    let at = simulation.population.agents[0].state.position;

    let agent = simulation.population.agents[0].clone();
    assert!(simulation.farming_action(&agent, at).is_some(), "somebody who does not keep fields breaks ground");

    simulation.population.agents[0].found_out_how_to("know:keep_fields");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.farming_action(&agent, at), None, "a keeper whose crops are lost breaks no more");
    assert_eq!(simulation.ground_left_unbroken.get(), 1);

    simulation.population.agents[0].crops_lately = vec![true, true, false, true];
    let agent = simulation.population.agents[0].clone();
    assert!(simulation.farming_action(&agent, at).is_some(), "and one whose crops pay does");
}

/// Somebody else of the camp, grown and living at home, whose crops lately
/// went as given, and who keeps their fields or not.
fn one_of_the_camp(simulation: &mut Simulation, crops: &[bool], keeps_fields: bool) -> usize {
    simulation.population.spawn_agent(AgentConfig::default());
    let index = simulation.population.agents.len() - 1;
    let agent = &mut simulation.population.agents[index];
    agent.state.now_this_many_years_old(30);
    agent.state.position = (HOME.0, HOME.1, 0);
    agent.hearth = Some(HOME);
    agent.traits = crate::core::traits::TraitSet::new();
    agent.crops_lately = crops.to_vec();
    if keeps_fields {
        agent.found_out_how_to("know:keep_fields");
    }
    index
}

/// A keeper whose own crops pay breaks no new ground while the camp's
/// crops lately have been lost more often than paid (#423), and does once
/// the camp's pay.
#[test]
fn a_keeper_holds_off_while_the_camps_crops_are_lost() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].inventory.add_item(grain(3));
    simulation.population.agents[0].crops_lately = vec![true, true, true];
    simulation.population.agents[0].found_out_how_to("know:keep_fields");
    let lost = one_of_the_camp(&mut simulation, &[false, false, false, false, false], false);
    let at = simulation.population.agents[0].state.position;

    let agent = simulation.population.agents[0].clone();
    assert!(agent.are_my_crops_paying(), "their own crops pay");
    assert_eq!(simulation.farming_action(&agent, at), None, "but the camp's do not");
    assert_eq!(simulation.ground_left_unbroken.get(), 1);

    simulation.population.agents[lost].crops_lately = vec![true, true, false];
    let agent = simulation.population.agents[0].clone();
    assert!(simulation.farming_action(&agent, at).is_some(), "and once the camp's pay, they break ground");
}

/// Where anybody grown in the camp keeps their fields and the camp's crops
/// are being lost, somebody who does not keep fields goes along with it and
/// breaks none (#423); a rebel or a stubborn one goes their own way. Where
/// nobody keeps them, it is nobody's way.
#[test]
fn where_the_camp_keeps_its_fields_the_rest_go_along() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].inventory.add_item(grain(3));
    let at = simulation.population.agents[0].state.position;
    let first = one_of_the_camp(&mut simulation, &[false, false, false], false);
    let _second = one_of_the_camp(&mut simulation, &[false, true, false], false);
    let _third = one_of_the_camp(&mut simulation, &[false, false], false);

    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation.farming_action(&agent, at).is_some(),
        "where nobody keeps fields, ground is broken"
    );

    simulation.population.agents[first].found_out_how_to("know:keep_fields");
    let agent = simulation.population.agents[0].clone();
    assert!(!agent.knows_the_way("keep_fields"));
    assert_eq!(simulation.farming_action(&agent, at), None, "one keeper in four makes it the camp's way");
    assert_eq!(simulation.ground_left_by_the_camps_way.get(), 1);

    for own_way in [crate::core::traits::Trait::Rebel, crate::core::traits::Trait::Stubborn] {
        let mut agent = simulation.population.agents[0].clone();
        agent.traits = crate::core::traits::TraitSet::new();
        agent.traits.add_trait(own_way);
        assert!(simulation.farming_action(&agent, at).is_some(), "{:?} goes their own way", own_way);
    }

    simulation.population.agents[first].crops_lately = vec![true, true, true, true, true, true];
    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation.farming_action(&agent, at).is_some(),
        "and while the camp's crops pay, ground is broken"
    );
}

/// Somebody who does not keep fields, carrying seed a day out from home,
/// breaks no ground out there once anybody of their camp keeps fields
/// (#423).
#[test]
fn a_camp_that_keeps_its_fields_breaks_no_ground_a_day_out() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].inventory.add_item(grain(3));
    let away = (HOME.0 + 15, HOME.1, 0);
    simulation.population.agents[0].state.position = away;
    let keeper = one_of_the_camp(&mut simulation, &[true, true, true], false);

    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.farming_action(&agent, away), Some(Action::TillSoil), "nobody keeps fields yet");

    simulation.population.agents[keeper].found_out_how_to("know:keep_fields");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.farming_action(&agent, away), None, "the camp keeps its fields about home");
}

/// Somebody carrying seed breaks ground where they stand, a day out from
/// home; where the camp keeps its fields, they break none out there, and do
/// about home (#423).
#[test]
fn a_kept_field_is_broken_about_home() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].inventory.add_item(grain(3));
    let away = (HOME.0 + 15, HOME.1, 0);
    simulation.population.agents[0].state.position = away;

    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.farming_action(&agent, away),
        Some(Action::TillSoil),
        "somebody who does not keep fields breaks ground where they stand"
    );

    simulation.population.agents[0].found_out_how_to("know:keep_fields");
    simulation.population.agents[0].crops_lately = vec![true, true, true];
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.farming_action(&agent, away), None, "a keeper breaks none a day out");

    simulation.population.agents[0].state.position = (HOME.0, HOME.1, 0);
    let agent = simulation.population.agents[0].clone();
    assert!(
        simulation.farming_action(&agent, (HOME.0, HOME.1, 0)).is_some(),
        "and does about home"
    );
}

// ---------------------------------------------------------------------------
// The planting calendar (#418)
// ---------------------------------------------------------------------------

/// A crop is in its time before its season opens and early in it; not with
/// less than half its season to go.
#[test]
fn a_crop_is_in_its_time_before_its_season_and_early_in_it() {
    let window = ResourceType::Grain.bearing_window();
    let opens = (0..crate::environment::seasons::DAYS_PER_YEAR)
        .find(|day| !window.covers(*day) && window.covers(day + 1))
        .expect("grain has a season")
        + 1;
    let days = window.how_many_days();
    assert!(Simulation::is_it_the_time_to_sow(ResourceType::Grain, opens - 10), "before it");
    assert!(Simulation::is_it_the_time_to_sow(ResourceType::Grain, opens + 2), "early in it");
    assert!(!Simulation::is_it_the_time_to_sow(ResourceType::Grain, opens + days - 5), "late in it");
}

/// Somebody who keeps a planting calendar, late in grain's season with
/// only grain to sow, breaks no ground; somebody who does not, does. Before
/// the season, the calendar keeper sows.
#[test]
fn a_calendar_keeper_sows_only_what_is_in_its_time() {
    let mut simulation = a_farmer_at_home();
    simulation.population.agents[0].inventory.add_item(grain(3));
    let window = ResourceType::Grain.bearing_window();
    let late = (0..crate::environment::seasons::DAYS_PER_YEAR)
        .find(|day| window.days_left(*day) == Some(3))
        .expect("grain's season ends");
    simulation.world.climate.calendar.day_of_year = late;
    let at = simulation.population.agents[0].state.position;

    let agent = simulation.population.agents[0].clone();
    assert!(simulation.farming_action(&agent, at).is_some(), "without a calendar, grain goes in late");

    simulation.population.agents[0].found_out_how_to("know:planting_calendar");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.farming_action(&agent, at), None, "with one, the ground waits");
    assert_eq!(simulation.sowings_put_off.get(), 1);
    let tilled = simulation.execute_action(&Action::TillSoil, 0);
    assert!(!tilled.success, "and nothing is broken for it");
    assert!(simulation.world.grid.field_at(&Position::new(at.0, at.1)).is_none());

    simulation.world.climate.calendar.day_of_year = Season::Spring.first_day() + 5;
    let agent = simulation.population.agents[0].clone();
    assert!(simulation.farming_action(&agent, at).is_some(), "in the spring, grain is in its time");
}

/// Crops one put work into ripening short, three times, is how the calendar
/// comes to somebody who keeps the seasons, and not to somebody who does
/// not.
#[test]
fn short_crops_are_how_the_calendar_comes_to_mind() {
    let mut simulation = a_farmer_at_home();
    let who = simulation.population.agents[0].id;
    for _ in 0..Agent::SHORT_CROPS_BEFORE_A_CALENDAR {
        simulation.world.crops_ripened_short_for.push(who);
    }
    simulation.what_ripened_short();
    assert!(simulation.world.crops_ripened_short_for.is_empty());
    assert!(!simulation.population.agents[0].knows_the_way("planting_calendar"), "not without the seasons");

    simulation.population.agents[0].found_out_how_to("know:seasons");
    simulation.world.crops_ripened_short_for.push(who);
    simulation.what_ripened_short();
    assert!(simulation.population.agents[0].knows_the_way("planting_calendar"));
}

// ---------------------------------------------------------------------------
// Manuring (#419)
// ---------------------------------------------------------------------------

/// Somebody who manures fields, carrying compost, walks to the field about
/// home worn furthest below what its ground was, and spreads it there; it
/// is counted. Somebody who does not leaves it to chance.
#[test]
fn a_manurer_carries_muck_to_the_most_worn_field() {
    let mut simulation = a_farmer_at_home();
    let now = simulation.world.turn;
    let (worn, more_worn, whole) = (
        Position::new(HOME.0 + 2, HOME.1),
        Position::new(HOME.0 + 5, HOME.1),
        Position::new(HOME.0 - 1, HOME.1),
    );
    for at in [worn, more_worn, whole] {
        assert!(simulation.world.grid.break_ground(&at, now));
    }
    simulation.world.grid.the_harvest_is_in(&worn, false, now);
    simulation.world.grid.the_harvest_is_in(&more_worn, false, now);
    simulation.world.grid.the_harvest_is_in(&more_worn, false, now);
    assert_eq!(simulation.world.grid.field_at(&more_worn).map(|f| f.grade), Some(SoilGrade::Exhausted));

    simulation.population.agents[0].found_out_how_to("do:SpreadMuck");
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(Simulation::COMPOST.to_string(), 4, 1.0));
    let at = simulation.population.agents[0].state.position;
    let agent = simulation.population.agents[0].clone();
    assert!(
        !matches!(simulation.muck_action(&agent, at), Some(Action::Move { .. })),
        "nobody walks muck anywhere who does not manure fields"
    );

    simulation.population.agents[0].found_out_how_to("know:manure_fields");
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.muck_action(&agent, at),
        Some(Action::Move { target: (more_worn.x, more_worn.y, 0) }),
        "to the most worn"
    );
    simulation.population.agents[0].state.position = (more_worn.x, more_worn.y, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.muck_action(&agent, agent.state.position), Some(Action::SpreadMuck));
    let spread = simulation.execute_action(&Action::SpreadMuck, 0);
    assert!(spread.success, "{spread:?}");
    assert_eq!(simulation.world.worn_fields_mucked, 1);
}

/// Crop taken off a worn field often enough, by somebody who knows muck
/// does a field good, is how manuring comes to mind.
#[test]
fn crop_off_worn_ground_is_how_manuring_comes_to_mind() {
    let mut simulation = a_farmer_at_home();
    simulation.world.climate.calendar.day_of_year = (0..crate::environment::seasons::DAYS_PER_YEAR)
        .find(|day| ResourceType::Grain.is_it_bearing(*day) && ResourceType::Grain.is_it_bearing(day + 20))
        .expect("grain bears");
    let field = Position::new(HOME.0, HOME.1);
    let now = simulation.world.turn;
    assert!(simulation.world.grid.break_ground(&field, now));
    simulation.world.grid.the_harvest_is_in(&field, false, now);
    let mut crop = crate::world::ResourceNode::new(ResourceType::Grain, field, 10_000);
    crop.amount = 5_000;
    crop.on_a_field = true;
    crop.it_has_ripened();
    simulation.world.put_a_node_down(crop);
    simulation.population.agents[0].found_out_how_to("do:SpreadMuck");
    for _ in 0..Agent::CROPS_OFF_WORN_GROUND_BEFORE_MANURING + 2 {
        simulation.population.agents[0].inventory.get_all_items_mut().clear();
        let gathered = simulation.execute_action(&Action::Gather { resource_type: "grain".to_string() }, 0);
        assert!(gathered.success, "{gathered:?}");
    }
    assert!(simulation.population.agents[0].knows_the_way("manure_fields"));
}

// ---------------------------------------------------------------------------
// Granaries (#420)
// ---------------------------------------------------------------------------

/// A covered store of `held` grain at home, lined with a basket.
fn a_store_at_home(simulation: &mut Simulation, held: u32) {
    a_store_about_home(simulation, held, (0, 0));
}

/// And one a pace or so off.
fn a_store_about_home(simulation: &mut Simulation, held: u32, off: (i32, i32)) {
    simulation.world.pits.push(Pit {
        gnawed: 0,
        last_opened: None,
        where_it_is: Position::new(HOME.0 + off.0, HOME.1 + off.1),
        holds: vec![grain(held), InventoryItem::new_with_weight("basket".to_string(), 1, 1.0)],
        covered: true,
        dug: 0,
        belongs: Belongs::ToNobody,
    });
}

/// A granary holds four pits' worth, vermin take from it as little as from
/// a pot, and it keeps at least as a lined store does.
#[test]
fn a_granary_holds_a_harvest_and_keeps_the_vermin_off() {
    let mut simulation = a_farmer_at_home();
    a_store_at_home(&mut simulation, 300);
    let pit = &mut simulation.world.pits[0];
    assert_eq!(pit.how_it_is_shut(), "lined");
    assert!(!pit.has_room());
    let lined = pit.with_the_lid_on();
    pit.holds.push(InventoryItem::new_with_weight(Pit::GRANARY.to_string(), 1, 0.0));
    assert!(pit.is_a_granary());
    assert_eq!(pit.how_it_is_shut(), "raised");
    assert_eq!(pit.what_it_takes(), Pit::WHAT_A_GRANARY_TAKES);
    assert_eq!(pit.how_much_room_is_left(), Pit::WHAT_A_GRANARY_TAKES - 300);
    assert_eq!(pit.how_much_is_in_it(), 300, "the floor is not food");
    assert_eq!(pit.what_vermin_get_at(), Pit::VERMIN_PAST_A_POT);
    assert!(pit.with_the_lid_on() <= lined);
}

/// Whether the camp's stores are worth raising a granary over is a matter
/// of temperament: the frugal raise one over what the lazy leave in the
/// ground. A single pit's worth is not worth one even to the frugal.
#[test]
fn the_frugal_raise_a_granary_the_lazy_would_not() {
    use crate::core::traits::{Trait, TraitSet};
    let mut simulation = a_farmer_at_home();
    a_store_at_home(&mut simulation, 300);
    let mut frugal = simulation.population.agents[0].clone();
    frugal.traits.add_trait(Trait::Frugal);
    frugal.found_out_how_to("know:granary");
    assert!(simulation.a_store_to_raise(&frugal).is_none(), "one pit's worth");
    a_store_about_home(&mut simulation, 300, (1, 0));
    a_store_about_home(&mut simulation, 300, (0, 1));
    let with = |t: Trait| {
        let mut agent = simulation.population.agents[0].clone();
        agent.traits = TraitSet::new();
        agent.traits.add_trait(t);
        agent.found_out_how_to("know:granary");
        agent
    };
    let (frugal, lazy) = (with(Trait::Frugal), with(Trait::Lazy));
    assert!(simulation.a_store_to_raise(&frugal).is_some());
    assert!(simulation.a_store_to_raise(&lazy).is_none());
}

/// Somebody who knows how fetches the wood for a granary, walks to the
/// camp's store, and raises it; not again once it stands with room in it.
#[test]
fn a_store_of_the_camps_is_raised_into_a_granary() {
    use crate::core::traits::{Trait, TraitSet};
    let mut simulation = a_farmer_at_home();
    a_store_at_home(&mut simulation, 300);
    a_store_about_home(&mut simulation, 300, (1, 0));
    a_store_about_home(&mut simulation, 300, (0, 1));
    {
        let agent = &mut simulation.population.agents[0];
        agent.traits = TraitSet::new();
        agent.traits.add_trait(Trait::Frugal);
        agent.found_out_how_to("know:granary");
        agent.state.position = (HOME.0 + 4, HOME.1, 0);
    }
    simulation.world.put_a_node_down(crate::world::ResourceNode::new(
        ResourceType::Wood,
        Position::new(HOME.0 + 4, HOME.1),
        100,
    ));
    let agent = simulation.population.agents[0].clone();
    assert_eq!(
        simulation.a_granary_to_raise(&agent, agent.state.position),
        Some(Action::Gather { resource_type: "wood".to_string() }),
        "the wood first"
    );
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), Simulation::WOOD_A_GRANARY_TAKES, 0.5));
    let agent = simulation.population.agents[0].clone();
    assert!(matches!(simulation.a_granary_to_raise(&agent, agent.state.position), Some(Action::Move { .. })));
    simulation.population.agents[0].state.position = (HOME.0, HOME.1, 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_granary_to_raise(&agent, agent.state.position), Some(Action::RaiseGranary));
    let raised = simulation.execute_action(&Action::RaiseGranary, 0);
    assert!(raised.success, "{raised:?}");
    assert_eq!(simulation.world.pits.iter().filter(|pit| pit.is_a_granary()).count(), 1);
    assert_eq!(simulation.world.granaries_built, 1);
    assert_eq!(simulation.population.agents[0].how_many_i_have("wood"), 0);
    let agent = simulation.population.agents[0].clone();
    assert_eq!(simulation.a_granary_to_raise(&agent, agent.state.position), None, "one with room is enough");
}

/// Lined stores opened gnawed by somebody who lines stores and breaks
/// ground is how the granary comes to mind.
#[test]
fn lined_stores_gnawed_are_how_a_farmer_thinks_of_a_granary() {
    let mut simulation = a_farmer_at_home();
    a_store_at_home(&mut simulation, 200);
    simulation.population.agents[0].found_out_how_to("know:line_the_store");
    let at = Position::new(HOME.0, HOME.1);
    for time in 1..=Agent::LINED_STORES_GNAWED_BEFORE_A_GRANARY {
        simulation.world.pits[0].gnawed = 3;
        simulation.current_turn = time * 10 * TICKS_PER_DAY;
        simulation.what_opening_the_store_shows(0, at);
    }
    assert!(simulation.population.agents[0].knows_the_way("granary"));
}

// ---------------------------------------------------------------------------
// Staple crops (#421)
// ---------------------------------------------------------------------------

/// Two kinds grown four times each, one paying half again what the other
/// does, make a staple of the one; one kind, however often, is not a
/// comparison, and two that pay alike make none.
#[test]
fn a_crop_that_plainly_pays_best_becomes_a_staple() {
    let mut agent = a_farmer_at_home().population.agents[0].clone();
    for _ in 0..Agent::CROPS_BEFORE_A_STAPLE {
        agent.what_my_crop_came_to(100.0, 400.0, Some("roots"));
    }
    assert_eq!(agent.my_staple_crop(), None, "one kind is no comparison");
    for _ in 0..Agent::CROPS_BEFORE_A_STAPLE {
        agent.what_my_crop_came_to(100.0, 350.0, Some("legumes"));
    }
    assert_eq!(agent.my_staple_crop(), None, "two that pay alike");
    assert!(!agent.knows_the_way("staple"));
    for _ in 0..Agent::CROPS_BEFORE_A_STAPLE * 2 {
        agent.what_my_crop_came_to(100.0, 50.0, Some("legumes"));
    }
    assert_eq!(agent.my_staple_crop().as_deref(), Some("roots"));
    assert!(agent.knows_the_way("staple"));
    assert!(agent.did_i_come_to_it_myself("know:staple"));
}

/// A staple goes in ahead of what this one would otherwise sow.
#[test]
fn a_staple_goes_in_first() {
    let mut agent = a_farmer_at_home().population.agents[0].clone();
    agent.inventory.add_item(grain(3));
    agent.inventory.add_item(InventoryItem::new_with_weight("roots".to_string(), 3, 0.2));
    let before = Simulation::what_this_one_would_sow(&agent, 1.0);
    let other = if before == ResourceType::Roots { "grain" } else { "roots" };
    let other_crop = if before == ResourceType::Roots { ResourceType::Grain } else { ResourceType::Roots };
    let first = if before == ResourceType::Roots { "roots" } else { Agent::SEED };
    for _ in 0..Agent::CROPS_BEFORE_A_STAPLE {
        agent.what_my_crop_came_to(100.0, 50.0, Some(first));
        agent.what_my_crop_came_to(100.0, 400.0, Some(other));
    }
    assert!(agent.knows_the_way("staple"));
    assert_eq!(Simulation::what_this_one_would_sow(&agent, 1.0), other_crop);
}
