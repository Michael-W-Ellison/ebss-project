// src/analytics/tests/a_roof_and_a_pack_tests.rs
//! Finishing a roof, the store under it, and what a pack is actually for.
//!
//! "Agents should start by carrying some food on them, then by storing some in
//! their tent, and finally by storing extra in the pit. ... Not every agent
//! need be a hunter, but all should be producing things of survival value."
//!
//! Three things were in the way of the middle rung, and all three are the same
//! defect wearing different hats: two places answering one question and never
//! being compared.
//!
//! - Nothing in the decision layer could **finish** a roof. `Build` pushed an
//!   under-construction site and the only caller of
//!   `add_construction_progress` anywhere is in the parallel world action
//!   system this layer does not issue. Every burrow ever dug was still going
//!   up when the last of the diggers died.
//! - A roof already up was read as "there is a building here, so stop", which
//!   was right while nothing could finish one and became exactly wrong the
//!   moment something could.
//! - A **working stock** was twelve items, and a pack holds 17.4 units of
//!   weight. Twelve wood at two units each is twenty-four - more than the
//!   whole pack. Nothing capped hoarding against the pack it went in, so 89.2%
//!   of packs had under five units of room and no material for any tool could
//!   ever be picked up.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::environment::Action;
use crate::world::{BuildingType, Building, Position, World, WorldConfig};

/// One person standing on their own half-dug burrow.
fn somebody_at_a_half_dug_burrow() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (10, 10, 0);

    // A fresh world is seeded with roofs of its own, and this fixture is about
    // one particular roof.
    simulation.world.buildings.clear();
    simulation.world.buildings.push(Building::new_under_construction(
        BuildingType::Burrow,
        Position::new(10, 10),
    ));
    simulation
}

// --------------------------------------------------------------------------
// Finishing it
// --------------------------------------------------------------------------

/// A turn of building puts work into the roof that is already up.
#[test]
fn a_turn_of_work_goes_into_the_roof_that_is_already_going_up() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    let before = simulation.world.buildings[0].construction_progress();

    let result = simulation.execute_action(
        &Action::Build {
            structure_type: "burrow".to_string(),
            position: (10, 10, 0),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert_eq!(
        simulation.world.buildings.len(),
        1,
        "he carried on with the one that was there rather than starting another"
    );
    assert!(
        simulation.world.buildings[0].construction_progress() > before,
        "and it is further up than it was"
    );
}

/// And enough turns of it finish the thing.
#[test]
fn a_roof_gets_finished_in_the_end() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    for _ in 0..12 {
        let _ = simulation.execute_action(
            &Action::Build {
                structure_type: "burrow".to_string(),
                position: (10, 10, 0),
            },
            0,
        );
    }

    assert!(
        simulation.world.buildings[0].is_completed(),
        "twelve turns of digging is a burrow: {:.0}% up",
        simulation.world.buildings[0].construction_progress() * 100.0
    );
}

/// A finished roof has a store under it, and the man who finished it knows.
#[test]
fn a_finished_roof_has_a_store_under_it() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    assert!(
        simulation.world.pits.is_empty(),
        "nothing is under it while it is going up"
    );

    for _ in 0..12 {
        let _ = simulation.execute_action(
            &Action::Build {
                structure_type: "burrow".to_string(),
                position: (10, 10, 0),
            },
            0,
        );
    }

    assert!(
        simulation.world.pits.iter().any(|pit| pit.where_it_is == Position::new(10, 10)),
        "the middle rung of the ladder: a hole under the floor"
    );
    assert!(
        simulation.population.agents[0]
            .memory
            .recall_locations(crate::core::memory::SpatialMemoryType::Storage)
            .iter()
            .any(|place| (place.position.0, place.position.1) == (10, 10)),
        "and the man who dug it knows where it is"
    );
}

/// A roof half up is a reason to go back to it, not a reason to stop.
#[test]
fn a_half_dug_burrow_is_a_reason_to_go_back() {
    let mut simulation = somebody_at_a_half_dug_burrow();

    // Something to dig with, since digging in wants one.
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("handaxe".to_string(), 1));

    let answer = simulation.digging_in(&simulation.population.agents[0], (10, 10, 0));

    assert!(
        matches!(answer, Some(Action::Build { .. })),
        "he is standing on his own half-dug burrow: {answer:?}"
    );
}

/// A roof that is finished is a reason to stop, though.
#[test]
fn a_finished_roof_is_not_dug_twice() {
    let mut simulation = somebody_at_a_half_dug_burrow();
    simulation.world.buildings[0] = Building::new(BuildingType::Burrow, Position::new(10, 10));

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new("handaxe".to_string(), 1));

    assert!(
        simulation
            .digging_in(&simulation.population.agents[0], (10, 10, 0))
            .is_none(),
        "there is already a roof here"
    );
}

// --------------------------------------------------------------------------
// What a pack is for
// --------------------------------------------------------------------------

/// A working stock is a share of the pack, not a count of things.
#[test]
fn a_working_stock_is_something_a_pack_can_hold() {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);

    let agent = &simulation.population.agents[0];
    let stock = crate::analytics::Simulation::what_a_working_stock_weighs(agent);

    assert!(
        stock < agent.inventory.max_weight,
        "a stock that will not go in the pack is not a stock: {stock} against {}",
        agent.inventory.max_weight
    );
    assert!(
        stock > 0.0,
        "and a man may carry something for later: {stock}"
    );
}

/// Iron is not worth carrying to somebody who cannot smelt it.
#[test]
fn nobody_carries_what_they_have_no_use_for() {
    let nobody_knows_anything = |_: &crate::environment::making::Making| false;

    assert!(
        !crate::environment::making::is_this_any_use_to("iron", &nobody_knows_anything),
        "eight units of weight, and not one step he knows takes it"
    );
}

/// And what everybody is born knowing a use for is worth carrying.
#[test]
fn what_there_is_a_use_for_is_worth_carrying() {
    let born_knowing = |step: &crate::environment::making::Making| step.obvious;

    assert!(
        crate::environment::making::is_this_any_use_to("stone", &born_knowing),
        "two stone is a knapped tip, and everybody is born knowing it"
    );
    assert!(
        crate::environment::making::is_this_any_use_to("flax", &born_knowing),
        "and flax is cordage"
    );
}

/// A thing nothing is made of is not refused - the question does not apply.
#[test]
fn the_question_does_not_apply_to_supper() {
    let nobody_knows_anything = |_: &crate::environment::making::Making| false;

    assert!(
        crate::environment::making::is_this_any_use_to("roots", &nobody_knows_anything),
        "a gate that refused everything it had no recipe for would stop a \
         settlement eating"
    );
}

// --------------------------------------------------------------------------
// Enough hole for a winter
// --------------------------------------------------------------------------

/// Digging waited on every pit being full, not on the store being enough.
///
/// A settlement with one pit a third full never dug a second, however far
/// short of the winter it was. Measured at month nine over eight seeded
/// world-years: 6.5 pits a settlement, 78.8% of them full to the brim, a
/// larder capped at 1,950 items - while `does_the_store_still_want_filling`
/// correctly asked for about 7,200.
#[test]
fn a_pit_with_room_in_it_is_not_enough_hole_for_a_winter() {
    use crate::world::Pit;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.world.pits.clear();

    let here = Position::new(25, 25);

    // One empty hole, twenty paces off so it is within reach and not underfoot.
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(30, 25),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    });

    let wanted = crate::analytics::Simulation::what_one_mouth_wants_put_by();
    assert!(
        wanted > Pit::WHAT_A_PIT_TAKES,
        "one mouth's winter is more than one hole holds - {wanted} against {}, \
         which is the whole of why this branch has to exist",
        Pit::WHAT_A_PIT_TAKES
    );

    assert!(
        !simulation.is_there_a_hole_going_spare(here),
        "the empty hole is five paces off, not underfoot"
    );
    assert_eq!(
        simulation.how_much_room_is_left_near(here),
        Pit::WHAT_A_PIT_TAKES,
        "and it is the only room this camp has"
    );
}

/// Nobody digs a second hole beside a half-empty first one.
#[test]
fn nobody_digs_on_top_of_a_hole_that_is_still_going_spare() {
    use crate::world::Pit;

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.world.pits.clear();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    });

    assert!(
        simulation.is_there_a_hole_going_spare(Position::new(25, 25)),
        "he is standing on an empty one"
    );
    assert!(
        simulation.is_there_a_hole_going_spare(Position::new(26, 25)),
        "and a pace off is still on top of it"
    );
    assert!(
        !simulation.is_there_a_hole_going_spare(Position::new(25, 40)),
        "across the camp is somewhere else"
    );
}

// --------------------------------------------------------------------------
// Taking food out of a store
// --------------------------------------------------------------------------

/// One person on a full pit, with a pack that can be made room in.
fn somebody_standing_on_a_full_pit() -> crate::analytics::Simulation {
    use crate::world::{ItemType, Pit};

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.world.pits.clear();

    let mut buried = InventoryItem::new_with_weight("food".to_string(), 60, 0.5);
    buried.food_data = simulation
        .food_database
        .create_food_data(&ItemType::Food, 0);
    let mut pit = Pit {
        where_it_is: Position::new(25, 25),
        holds: Vec::new(),
        covered: true,
        dug: 0,
        belongs: crate::world::Belongs::ToNobody,
    };
    pit.put_in(buried);
    simulation.world.pits.push(pit);
    simulation
}

/// Fill a pack with something nobody would set down for food.
fn a_pack_with_no_room_and_nothing_to_shed(simulation: &mut crate::analytics::Simulation) {
    // A tool is never set down for supper - see `what_i_would_set_down` - so
    // this is a pack that genuinely cannot take another handful.
    while simulation.population.agents[0]
        .inventory
        .weight_capacity_remaining()
        >= 0.5
    {
        if !simulation.population.agents[0]
            .inventory
            .add_item(InventoryItem::new_with_weight("handaxe".to_string(), 1, 0.5))
        {
            break;
        }
    }
}

/// What comes out of the store arrives in the pack, or it stays in the ground.
///
/// It used to do neither. `add_item` returns false when the pack is too heavy
/// and almost every caller ignores it, so eight items left the pit and never
/// arrived: measured directly, a pit of sixty went to fifty-two, the pack
/// stayed at nought, and the action reported "Took 8 food out of the pit".
#[test]
fn what_will_not_go_in_the_pack_stays_in_the_ground() {
    let mut simulation = somebody_standing_on_a_full_pit();
    a_pack_with_no_room_and_nothing_to_shed(&mut simulation);

    let in_the_pit_before = simulation.world.pits[0].how_much_is_in_it();

    let result = simulation.execute_action(
        &Action::PickUp {
            what: "food".to_string(),
        },
        0,
    );

    assert!(!result.success, "there is nowhere to put it: {:?}", result.message);
    assert_eq!(
        simulation.world.pits[0].how_much_is_in_it(),
        in_the_pit_before,
        "and so it is still in the ground"
    );
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("food"),
        0,
        "and nowhere else"
    );
}

/// What does go in is exactly what left the pit.
#[test]
fn what_comes_out_of_the_store_is_what_arrives() {
    let mut simulation = somebody_standing_on_a_full_pit();

    let in_the_pit_before = simulation.world.pits[0].how_much_is_in_it();

    let result = simulation.execute_action(
        &Action::PickUp {
            what: "food".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);

    let taken = in_the_pit_before - simulation.world.pits[0].how_much_is_in_it();
    assert!(taken > 0, "he took something");
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("food"),
        taken,
        "and every one of them is in his pack: nothing stopped existing on the way"
    );
}

/// And the decision does not offer what the executor will refuse.
#[test]
fn nobody_is_offered_a_store_they_cannot_carry_away_from() {
    let mut simulation = somebody_standing_on_a_full_pit();
    simulation.population.agents[0]
        .memory
        .remember_how_much_is_there(
            crate::core::memory::SpatialMemoryType::Storage,
            (25, 25, 0),
            60,
        );
    a_pack_with_no_room_and_nothing_to_shed(&mut simulation);

    let here = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_none(),
        "offering a man his own larder and then refusing him is worse than not \
         offering: this branch sits above every drive there is"
    );
}

// --------------------------------------------------------------------------
// What a tent is actually made of
// --------------------------------------------------------------------------
//
// **`SkinTent` has declared since it was written that it wants eight wood and
// four hides, and two separate comments in `world::buildings` say so.** The
// builder resolved a `ResourceType` to an item name with a `match` of three
// arms - wood, stone, iron - and `continue`d on everything else, in the
// checking pass *and* in the consuming pass. So the hides were neither
// required nor taken, and every tent ever raised in this model was poles and
// air.
//
// It is now asked for by class - poles, a flexible covering, cordage - which
// is what the specification asks for and what stops the same thing happening
// again: a class cannot quietly fail to resolve, because there is no arm to
// fall off the end of.

use crate::environment::tags::{self, Tag};

fn somebody_on_open_ground() -> crate::analytics::Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation =
        crate::analytics::Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (10, 10, 0);
    simulation.world.buildings.clear();
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

fn put_in_the_pack(simulation: &mut crate::analytics::Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(what.to_string(), how_many, 0.5));
}

fn puts_up_a_tent(simulation: &mut crate::analytics::Simulation) -> crate::environment::ActionResult {
    simulation.execute_action(
        &Action::Build {
            structure_type: "tent".to_string(),
            position: (10, 10, 0),
        },
        0,
    )
}

/// **The defect this test exists for.** Poles alone are not a tent.
#[test]
fn a_tent_cannot_be_made_of_poles_and_air() {
    let mut simulation = somebody_on_open_ground();
    put_in_the_pack(&mut simulation, "wood", 20);

    let result = puts_up_a_tent(&mut simulation);

    assert!(
        !result.success,
        "twenty pieces of wood and nothing to stretch over them went up as a \
         tent: {:?}",
        result.message
    );
    assert!(
        result
            .message
            .as_deref()
            .unwrap_or_default()
            .contains(Tag::FlexibleCovering.called()),
        "and the refusal should say which class is short, because that is the \
         next job: {:?}",
        result.message
    );
}

/// With poles, a covering and cordage it goes up, and all three come out of
/// the pack.
#[test]
fn a_tent_takes_what_it_says_it_takes() {
    let mut simulation = somebody_on_open_ground();
    put_in_the_pack(&mut simulation, "wood", 20);
    put_in_the_pack(&mut simulation, "hides", 6);
    put_in_the_pack(&mut simulation, "lashing", 4);

    let result = puts_up_a_tent(&mut simulation);
    assert!(result.success, "{:?}", result.message);

    let left = |what: &str| simulation.population.agents[0].how_many_i_have(what);

    assert!(left("wood") < 20, "the poles went into it");
    assert!(
        left("hides") < 6,
        "and so did the covering, which is the half that was free before"
    );
    assert!(left("lashing") < 4, "and the cordage");
}

/// **What asking by class buys.** A people who scraped their hides into
/// leather can roof with the leather.
///
/// Asked by name it was `ResourceType::Hides` and nothing else, so a
/// settlement that had gone one step further up its own tanning chain had
/// turned its roofing material into something the roof did not recognise.
#[test]
fn leather_will_roof_a_tent_as_well_as_a_raw_hide_will() {
    assert!(tags::is_this_a("leather", Tag::FlexibleCovering));

    let mut simulation = somebody_on_open_ground();
    put_in_the_pack(&mut simulation, "wood", 20);
    put_in_the_pack(&mut simulation, "leather", 6);
    put_in_the_pack(&mut simulation, "lashing", 4);

    let result = puts_up_a_tent(&mut simulation);

    assert!(result.success, "{:?}", result.message);
    assert!(simulation.population.agents[0].how_many_i_have("leather") < 6);
}

/// A burrow still costs a morning and nothing to fetch.
///
/// The whole point of it is that there is nothing to be short of, and a change
/// that made tents dearer had better not have touched the one shelter a people
/// with no timber and no skins can put up. It wants something to dig with -
/// which is the verb matrix doing its job, not a material - and that is all.
#[test]
fn a_burrow_still_wants_nothing_that_has_to_be_gathered() {
    let mut simulation = somebody_on_open_ground();
    put_in_the_pack(&mut simulation, "diggingstick", 1);

    let result = simulation.execute_action(
        &Action::Build {
            structure_type: "burrow".to_string(),
            position: (10, 10, 0),
        },
        0,
    );

    assert!(
        result.success,
        "a man with a stick and nothing else can still dig himself in: {:?}",
        result.message
    );
}
