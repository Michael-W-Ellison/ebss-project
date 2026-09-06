// src/analytics/tests/larder_tests.rs
//! Tests for a hole in the ground with food in it.
//!
//! A settlement had nowhere to put anything it ate. `what_i_can_spare`
//! explicitly excluded food, and the only place to put anything was a single
//! global bag of counts with no position that nothing ever spoiled in — so a
//! people stored materials it rarely needed and never once stored a meal.
//! Measured at ten thousand ticks, not one of sixty-five living agents was
//! carrying so much as supper: see ISSUES_FOUND #21.
//!
//! Cold ground with the earth back over it keeps food four times as long as a
//! pack does. That is not a cellar. It is the difference between eating what
//! you found today and eating in February.

use crate::agents::{Agent, AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::{verbs, Action};
use crate::world::nutrition::FoodDatabase;
use crate::environment::seasons::{Season, TICKS_PER_DAY};
use crate::world::{ItemType, Pit, Position, Terrain, TerrainType, World, WorldConfig};

/// Wind the world on until the year reaches the season wanted.
fn turn_the_year_to(simulation: &mut Simulation, wanted: Season) {
    for _ in 0..(TICKS_PER_DAY * 400) {
        if simulation.world.climate.current_season() == wanted {
            return;
        }
        simulation.world.tick();
    }
    panic!("the year never reached {wanted:?}");
}

fn supper(how_many: u32, made_at: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight("food".to_string(), how_many, 0.5);
    meal.food_data = database.create_food_data(&ItemType::Food, made_at);
    meal
}

/// One person standing on ground that can be dug.
/// The same, in the stretch of the year the store exists for.
///
/// A store is opened when the land gives nothing. Every test below that is
/// about the *mechanics* of opening one - walking to it, the meal `Cover`
/// hands back, a carcass not counting as supper - has to be run in that
/// stretch, or the season gate quite correctly refuses before the mechanics
/// are ever reached. They used to run at day nought, in spring, which is when
/// there is leaf on every hedge.
fn a_digger_in_the_lean_season() -> Simulation {
    let mut simulation = a_digger();
    let midwinter = crate::environment::seasons::first_day_of(
        crate::environment::seasons::Season::Winter,
        crate::environment::seasons::PartOfSeason::Deep,
    );
    simulation.world.climate.calendar.day_of_year = midwinter;
    assert!(
        !simulation.are_the_hedgerows_bearing(),
        "midwinter is supposed to be the bare stretch"
    );
    simulation
}

fn a_digger() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    for dx in -3..=3 {
        for dy in -3..=3 {
            let at = Position::new(25 + dx, 25 + dy);
            if let Some(tile) = world.grid.get_tile_mut(&at) {
                tile.terrain = Terrain::new(TerrainType::Plains);
            }
        }
    }

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0].state.energy = 100.0;
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();

    // A mining tool, which is what the matrix says digging wants
    let mut pick = InventoryItem::new_with_weight("handaxe".to_string(), 1, 1.0);
    pick.current_durability = Some(40.0);
    pick.max_durability = Some(40.0);
    let _ = simulation.population.agents[0].inventory.add_item(pick);

    simulation
}

// --------------------------------------------------------------------------
// Digging one
// --------------------------------------------------------------------------

/// A pit gets dug, and something comes out of the hole.
#[test]
fn digging_a_pit_leaves_a_pit_and_a_pile_of_stone() {
    let mut simulation = a_digger();

    let result = simulation.execute_action(&Action::Excavate, 0);

    assert!(result.success, "{:?}", result.message);
    assert_eq!(simulation.world.pits.len(), 1, "there is a hole there now");
    assert!(
        simulation.population.agents[0].how_many_i_have("stone") > 0,
        "and what came out of it is in his hands"
    );
}

/// Nobody digs two pits in one hole.
#[test]
fn nobody_digs_the_same_hole_twice() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);

    let again = simulation.execute_action(&Action::Excavate, 0);

    assert!(!again.success, "there is already a pit there");
    assert_eq!(simulation.world.pits.len(), 1);
}

/// And nobody digs one in a lake.
#[test]
fn nobody_digs_a_pit_in_water() {
    let mut simulation = a_digger();
    if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(25, 25)) {
        tile.terrain = Terrain::new(TerrainType::Water);
    }

    let result = simulation.execute_action(&Action::Excavate, 0);

    assert!(!result.success, "you cannot dig a hole in a lake");
}

/// Digging is the most expensive single thing anybody does, and should be.
#[test]
fn digging_a_pit_is_a_mornings_work() {
    assert!(
        Simulation::WHAT_DIGGING_A_PIT_COSTS > Simulation::WHAT_RUNNING_COSTS,
        "it should cost more than bolting from a wolf"
    );
}

// --------------------------------------------------------------------------
// Filling it
// --------------------------------------------------------------------------

/// Food goes in and the earth goes back over it.
#[test]
fn what_is_put_by_goes_in_and_is_covered() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(20, 0));

    let result = simulation.execute_action(
        &Action::Cover {
            what: "food".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    let pit = &simulation.world.pits[0];
    assert!(pit.covered, "the earth goes back over it");
    assert!(pit.how_much_is_in_it() > 0, "and there is food in it");
}

/// A person standing on their own store keeps a meal about them and buries
/// the rest.
///
/// This used to keep back three days' food, which is nonsense when you are
/// standing on the larder — you can take more out tomorrow, that is what it
/// is for — and it was what stopped anything ever being stored: measured
/// directly, `Cover` was refused 1,513 times out of 1,525 for want of
/// anything to bury, because a settlement living hand to mouth rarely holds
/// more than three of anything.
#[test]
fn nobody_buries_their_whole_supper() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(20, 0));

    simulation.execute_action(
        &Action::Cover {
            what: "food".to_string(),
        },
        0,
    );

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("food"),
        Simulation::WHAT_A_PERSON_KEEPS_ON_THEM,
        "he walks away from the pit with something to eat on the way home"
    );
    assert!(
        simulation.world.pits[0].how_much_is_in_it() > 0,
        "and the rest of it is in the ground"
    );
}

/// Nothing to bury is not a burial.
#[test]
fn nobody_buries_what_they_have_not_got() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);

    let result = simulation.execute_action(
        &Action::Cover {
            what: "food".to_string(),
        },
        0,
    );

    assert!(!result.success, "there is nothing to put by");
}

/// And a hole is wanted first.
#[test]
fn nobody_covers_food_with_no_pit_to_put_it_in() {
    let mut simulation = a_digger();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(20, 0));

    let result = simulation.execute_action(
        &Action::Cover {
            what: "food".to_string(),
        },
        0,
    );

    assert!(!result.success, "there is no pit there");
}

// --------------------------------------------------------------------------
// What the ground keeps
// --------------------------------------------------------------------------

/// The whole point: food under the earth outlasts food in a pack.
#[test]
fn what_is_buried_outlasts_what_is_carried() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(20, 0));
    simulation.execute_action(
        &Action::Cover {
            what: "food".to_string(),
        },
        0,
    );

    // The same meal, out in the weather. Only the world is ticked: a
    // settlement of one starves inside three thousand ticks and takes the
    // comparison with it.
    let mut in_the_pack = supper(20, 0);

    // Berries last three days in a pack, which on this calendar is
    // thirty-six ticks. Thirty is long enough that the difference shows and
    // short enough that there is anything left to compare.
    for _ in 0..30 {
        simulation.world.tick();
    }
    if let Some(food) = in_the_pack.food_data.as_mut() {
        food.update_freshness(simulation.world.tick);
    }

    let buried = simulation
        .world
        .pits
        .first()
        .and_then(|pit| pit.holds.first())
        .and_then(|held| held.food_data.as_ref())
        .map(|food| food.freshness)
        .expect("the pit should still be there with food in it");

    let carried = in_the_pack
        .food_data
        .as_ref()
        .map(|food| food.freshness)
        .unwrap_or(0.0);

    assert!(
        buried > carried,
        "cold ground keeps: {buried:.2} against {carried:.2}"
    );
}

/// Tell everybody about every pit there is.
///
/// A pit is a place an agent has to have *learned* about now - by seeing it,
/// or by having dug or filled it - rather than a fact about the world that
/// every mind has free of charge. See `nearest_pit_i_remember`. These
/// fixtures push a pit straight into the world and then ask a decision about
/// it, so they have to hand over the knowledge that a settlement would have
/// come by in the ordinary way.
fn and_everybody_knows_about_it(simulation: &mut Simulation) {
    let pits: Vec<((i32, i32, i32), u32)> = simulation
        .world
        .pits
        .iter()
        .map(|pit| {
            (
                (pit.where_it_is.x, pit.where_it_is.y, 0),
                pit.how_much_is_in_it().max(1),
            )
        })
        .collect();

    for agent in simulation.population.agents.iter_mut() {
        for (where_it_is, holding) in &pits {
            agent.memory.remember_how_much_is_there(
                crate::core::memory::SpatialMemoryType::Storage,
                *where_it_is,
                *holding,
            );
        }
    }
}

/// An open pit is a hole with food in it, which is much the same as leaving
/// it on the grass.
#[test]
fn an_open_pit_keeps_nothing() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(20, 0)],
        covered: false,
        dug: 0,
    });

    for _ in 0..3000 {
        simulation.world.tick();
    }

    let left = simulation
        .world
        .pits
        .first()
        .and_then(|pit| pit.holds.first())
        .and_then(|held| held.food_data.as_ref())
        .map(|food| food.freshness)
        .unwrap_or(0.0);

    assert!(
        left < 1.0,
        "an open hole is not a larder, and this one kept everything: {left}"
    );
}

// --------------------------------------------------------------------------
// Drawing on it
// --------------------------------------------------------------------------

/// A hungry person standing on a full pit takes something out of it.
#[test]
fn what_is_buried_comes_back_out() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });

    let result = simulation.execute_action(
        &Action::PickUp {
            what: "food".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);
    assert!(
        simulation.population.agents[0].how_many_i_have("food") > 0,
        "he has supper now"
    );
    assert!(
        simulation.world.pits[0].how_much_is_in_it() < 40,
        "and the store is that much lighter"
    );
}

/// Somebody with supper in the pack does not dig it up.
#[test]
fn nobody_raids_the_store_with_a_full_pack() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(10, 0));

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_none(),
        "he has his own"
    );
}

/// And a store a long way off is walked to rather than reached into.
#[test]
fn a_store_across_the_camp_is_walked_to() {
    let mut simulation = a_digger_in_the_lean_season();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(31, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
    and_everybody_knows_about_it(&mut simulation);

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .something_out_of_the_store(&simulation.population.agents[0], here)
        .expect("there is food in the ground six paces off");

    assert!(
        matches!(answer, Action::Move { .. }),
        "he walks over first: {answer:?}"
    );
}

// --------------------------------------------------------------------------
// Deciding to
// --------------------------------------------------------------------------

/// Somebody with more food than they can eat and no pit digs one — once it
/// is dried. Drying comes first: a hole makes a thing keep four times as
/// long and drying makes it keep twenty, and doing both is what a store is
/// for.
#[test]
fn a_surplus_and_no_pit_means_digging() {
    let mut simulation = a_digger();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, 0));

    // Somebody who has already seen what the sun does to food. Without this
    // he lays it out on the ground instead, which is the route into the
    // discovery and comes first for anybody who has not made it yet - see
    // `putting_food_by`.
    simulation.population.agents[0]
        .found_out_how_to(Simulation::THAT_LAYING_IT_OUT_KEEPS_IT);

    simulation.execute_action(
        &Action::Dry {
            what: "food".to_string(),
        },
        0,
    );

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .putting_food_by(&simulation.population.agents[0], here)
        .expect("he has more than he can eat");

    assert!(
        matches!(answer, Action::Excavate),
        "nowhere to put it, so he digs: {answer:?}"
    );
}

/// And with a pit under him, buries it - once it is a thing that will keep.
///
/// The `Dry` here used to be decoration: the digger had never watched anything
/// dry, so the action was refused and the berries went into the ground raw,
/// where they keep twenty-four days against the seventy-five the land gives
/// nothing. Burying was unconditional, so the test passed anyway. It is not
/// unconditional now - see `is_it_worth_burying` and ISSUES_FOUND.md #124 - so
/// the digger has to actually know how to dry a thing.
#[test]
fn a_surplus_and_a_pit_means_burying() {
    let mut simulation = a_digger();
    simulation.population.agents[0]
        .found_out_how_to(crate::agents::Agent::THAT_LAYING_IT_OUT_KEEPS_IT);
    simulation.execute_action(&Action::Excavate, 0);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, 0));
    let dried = simulation.execute_action(
        &Action::Dry {
            what: "food".to_string(),
        },
        0,
    );
    assert!(dried.success, "the drying should take: {:?}", dried.message);

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .putting_food_by(&simulation.population.agents[0], here)
        .expect("he has more than he can eat and a hole to put it in");

    assert!(
        matches!(&answer, Action::Cover { what } if what == "food"),
        "he buries it: {answer:?}"
    );
}

/// An empty store in autumn is the only reason anybody in this model ever
/// gathers food they are not about to eat.
#[test]
fn an_empty_store_in_autumn_sends_somebody_out_for_something_to_fill_it() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Fall);

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .putting_food_by(&simulation.population.agents[0], here)
        .expect("an empty store with the year turning is a reason to go out");

    assert!(
        matches!(&answer, Action::Gather { resource_type } if resource_type == "food"),
        "he goes and gets something to put in it: {answer:?}"
    );
}

/// And nobody puts food by in June.
///
/// The first cut of this ran all year: a settlement dug and foraged for a
/// larder in the middle of summer with berries on every bush, spent 351 trips
/// a world on it, and came out ten people smaller for the effort.
#[test]
fn nobody_forages_for_the_store_in_summer() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Summer);

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .putting_food_by(&simulation.population.agents[0], here)
            .is_none(),
        "there are berries on every bush"
    );
}

/// But a genuine surplus in the hand gets buried whatever month it is.
#[test]
fn a_surplus_in_the_hand_gets_buried_in_any_season() {
    let mut simulation = a_digger();
    simulation.population.agents[0]
        .found_out_how_to(crate::agents::Agent::THAT_LAYING_IT_OUT_KEEPS_IT);
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Summer);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, simulation.world.tick));
    let dried = simulation.execute_action(
        &Action::Dry {
            what: "food".to_string(),
        },
        0,
    );
    assert!(dried.success, "the drying should take: {:?}", dried.message);

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .putting_food_by(&simulation.population.agents[0], here)
        .expect("he is carrying more than he can eat");

    assert!(
        matches!(&answer, Action::Cover { what } if what == "food"),
        "a thing in your hands that will go off is buried whatever month it is: {answer:?}"
    );
}

/// Nobody asks for a pit where a pit will not go.
///
/// The first cut asked for one wherever somebody happened to be standing and
/// the executor refused most of them: 100 attempts a world for 1.7 pits, which
/// is ninety-eight turns spent trying to dig a hole in a lake.
#[test]
fn nobody_asks_for_a_pit_where_one_will_not_go() {
    let mut simulation = a_digger();
    if let Some(tile) = simulation.world.grid.get_tile_mut(&Position::new(25, 25)) {
        tile.terrain = Terrain::new(TerrainType::Water);
    }
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, 0));
    simulation.execute_action(
        &Action::Dry {
            what: "food".to_string(),
        },
        0,
    );

    let here = simulation.population.agents[0].state.position;
    let answer = simulation.putting_food_by(&simulation.population.agents[0], here);

    assert!(
        !matches!(answer, Some(Action::Excavate)),
        "he is standing in a lake: {answer:?}"
    );
}

/// With nothing to spare and nowhere to put anything, autumn is still a
/// reason to dig.
///
/// This was a circle and it took the land actually going bare in winter to
/// show it: digging a pit wanted a surplus in hand, and gathering a surplus
/// for the store wanted a pit to put it in, so neither could ever happen
/// first. The moment food became seasonal the larder stopped being used at
/// all — burials fell from 10.8 a world to 1.8 exactly when a store was
/// worth most.
#[test]
fn autumn_with_nowhere_to_put_anything_is_a_reason_to_dig() {
    let mut simulation = a_digger();
    turn_the_year_to(&mut simulation, Season::Fall);
    let here = simulation.population.agents[0].state.position;

    let answer = simulation
        .putting_food_by(&simulation.population.agents[0], here)
        .expect("the year is turning and there is nowhere to put anything");

    assert!(
        matches!(answer, Action::Excavate),
        "he digs: {answer:?}"
    );
}

/// But out of season, with nothing to spare, there is nothing to be done
/// about tomorrow.
#[test]
fn no_store_and_nothing_spare_in_summer_is_nothing_to_do() {
    let mut simulation = a_digger();
    turn_the_year_to(&mut simulation, Season::Summer);
    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .putting_food_by(&simulation.population.agents[0], here)
            .is_none(),
        "nobody digs a winter store in June"
    );
}

// --------------------------------------------------------------------------
// The matrix
// --------------------------------------------------------------------------

/// The subterranean family is no longer half declaration.
#[test]
fn digging_and_covering_are_live_verbs() {
    for called in ["excavate", "cover", "dig"] {
        let one = verbs::what_that_verb_is(called).expect("in the matrix");
        assert!(one.is_live(), "{called} should be doing something");
        assert_eq!(one.family, verbs::Family::Subterranean);
    }
}

/// And digging wants something to dig with, which the matrix enforces.
#[test]
fn digging_a_store_wants_a_tool() {
    let wanted = verbs::what_this_action_cannot_do_without("excavate");

    assert!(
        wanted.iter().any(|one| matches!(
            one,
            verbs::Wants::AToolFor(crate::agents::SkillType::Mining)
        )),
        "you do not dig a storage pit with your hands: {wanted:?}"
    );
}


// --------------------------------------------------------------------------
// A harvest is not supper
// --------------------------------------------------------------------------

/// The provisioning gap, and the whole reason forty pits a world got dug and
/// none of them ever had anything in it.
///
/// Nothing in this model gathered *for the winter*. It gathered because it
/// was hungry, ate what it picked in the same breath, and put away whatever
/// happened to be left over. Probed directly in autumn, only 108 agent-samples
/// in 3,254 were carrying any food at all — three in a hundred — so there was
/// never a load to carry home.
#[test]
fn a_load_gathered_in_autumn_is_not_eaten_on_the_spot() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Fall);

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(4, simulation.world.tick));

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation.is_this_lot_for_the_store(&simulation.population.agents[0], here),
        "autumn, a store within reach and a man who is not desperate: that is a harvest"
    );
}

/// And in summer it is simply supper.
#[test]
fn the_same_armful_in_summer_is_just_supper() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Summer);

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(4, simulation.world.tick));

    let here = simulation.population.agents[0].state.position;

    assert!(
        !simulation.is_this_lot_for_the_store(&simulation.population.agents[0], here),
        "nobody carries their dinner past their own mouth in June"
    );
}

/// A man who will be dead by morning eats what is in his hand, and the store
/// can wait.
#[test]
fn a_starving_man_eats_the_harvest() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Fall);

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(4, simulation.world.tick));
    simulation.population.agents[0].state.energy = 1.0;
    simulation.population.agents[0].nutrition.energy_reserves = 1.0;

    let here = simulation.population.agents[0].state.position;

    assert!(
        !simulation.is_this_lot_for_the_store(&simulation.population.agents[0], here),
        "the store can wait"
    );
}

/// Once the load is worth carrying, it gets carried — and that beats hunger,
/// because a person filling a store is a person carrying food past their own
/// mouth and Hunger wins every contest it enters.
#[test]
fn a_full_load_gets_taken_to_the_store() {
    let mut simulation = a_digger();
    simulation.population.agents[0]
        .found_out_how_to(crate::agents::Agent::THAT_LAYING_IT_OUT_KEEPS_IT);
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Fall);

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(20, simulation.world.tick));

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .is_the_load_worth_carrying_home(&simulation.population.agents[0], here)
        .expect("that is a load");

    assert!(
        matches!(&answer, Action::Dry { .. } | Action::Cover { .. }),
        "he is standing on the pit, so it goes in: {answer:?}"
    );
}

/// And a load with the store a walk away gets walked.
#[test]
fn a_full_load_with_the_store_across_the_camp_gets_walked_over() {
    use crate::world::Pit;

    let mut simulation = a_digger();
    turn_the_year_to(&mut simulation, Season::Fall);
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(30, 25),
        holds: Vec::new(),
        covered: false,
        dug: 0,
    });

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(20, simulation.world.tick));

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .is_the_load_worth_carrying_home(&simulation.population.agents[0], here)
        .expect("that is a load");

    assert!(
        matches!(answer, Action::Move { .. }),
        "five paces to the store: {answer:?}"
    );
}

/// Putting something by waits on being neither hungry nor parched today, and
/// on nothing else.
///
/// It used to stand behind Sustenance, on the reasoning that a people puts by
/// what it grows. It does not: a people puts by what it finds, and it has
/// been doing that far longer than it has been growing anything. Behind
/// Sustenance, a forager could never store — measured directly, Preparedness
/// sat below its threshold in eight agents out of eight for a whole
/// settlement's life.
#[test]
fn putting_by_waits_on_hunger_and_thirst_and_nothing_else() {
    use crate::core::DriveType;

    let waits_on = DriveType::Preparedness.unlocked_by();

    assert!(waits_on.contains(&DriveType::Hunger));
    assert!(waits_on.contains(&DriveType::Thirst));
    assert!(
        !waits_on.contains(&DriveType::Sustenance),
        "a forager stores food and does not farm it"
    );
}

// --------------------------------------------------------------------------
// Getting back into it
//
// The store was a one-way valve. Measured over ten thousand ticks: nine
// hundred and ninety-one things buried, sixty-eight taken back out, nine
// hundred units of food still in the ground at the end and four hundred and
// seventy rotted where they lay. See ISSUES_FOUND #43.
//
// Two things kept a people out of its own larder, and between them they made
// a circle. `Cover` hands a person one meal back on its way past — the store
// is right there — and drawing on the store asked for a person with *no food
// at all*, so that one meal was exactly enough to lock them out of the pit
// they had just filled. And the branch sat behind the ordinary food branch,
// which answers nearly always, so it rarely got the turn to begin with.
// --------------------------------------------------------------------------

/// The circle, stated as arithmetic: what burying leaves in the pack has to
/// be less than what shuts the store, or nobody who has just filled a pit can
/// ever open it again.
#[test]
fn what_burying_leaves_behind_does_not_shut_the_store() {
    assert!(
        Simulation::WHAT_A_PERSON_KEEPS_ON_THEM < Simulation::enough_not_to_open_the_store(),
        "burying leaves {} and the store shuts at {}: a man who has just \
         filled a pit is locked out of it",
        Simulation::WHAT_A_PERSON_KEEPS_ON_THEM,
        Simulation::enough_not_to_open_the_store(),
    );
}

/// And in the world rather than in the arithmetic: the one meal `Cover` hands
/// back is not a reason to leave the rest of it in the ground.
#[test]
fn one_meal_in_the_pack_does_not_shut_the_store() {
    let mut simulation = a_digger_in_the_lean_season();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
    and_everybody_knows_about_it(&mut simulation);
    let _ = simulation.population.agents[0].inventory.add_item(supper(
        Simulation::WHAT_A_PERSON_KEEPS_ON_THEM,
        0,
    ));

    let here = simulation.population.agents[0].state.position;

    assert!(
        matches!(
            simulation.something_out_of_the_store(&simulation.population.agents[0], here),
            Some(Action::PickUp { .. })
        ),
        "one meal is what he kept back when he buried the other forty"
    );
}

/// Two days' worth is a proper meal, and a proper meal leaves the store shut.
#[test]
fn a_pack_with_two_days_in_it_leaves_the_store_shut() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
    let _ = simulation.population.agents[0].inventory.add_item(supper(
        Simulation::enough_not_to_open_the_store(),
        0,
    ));

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_none(),
        "he has enough about him to be going on with"
    );
}

/// The store stays *behind* the ordinary food branch, and this is here so
/// that nobody moves it again without reading why.
///
/// In front of it, measured at thirty-two worlds a side: the store is drawn
/// on five times as often, the rot in the pits halves — and a settlement eats
/// a fifth less and carries six fewer people. Efficiency did not move at all,
/// which is the whole point of the exercise. A meal out of a hole costs two
/// turns where a berry costs one, and almost everything taken out had been
/// put in by somebody a day earlier. See ISSUES_FOUND #43.
#[test]
fn going_out_for_food_comes_before_digging_up_the_store() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });

    // Something to eat in his hand and a winter's food under his boots
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(1, 0));

    let here = simulation.population.agents[0].state.position;
    let answer = simulation.what_this_drive_offers(
        crate::core::DriveType::Hunger,
        &simulation.population.agents[0],
        here,
    );

    assert!(
        !matches!(answer, Some(Action::PickUp { .. })),
        "he eats what is in his hand, or goes and gets more, before he opens \
         the ground: {answer:?}"
    );
}

// --------------------------------------------------------------------------
// A meal, and a thing that is not one
//
// One settlement in sixteen starved to death standing on its own larder. The
// pit held a haunch nobody had taken a knife to; `something_to_eat` answered
// with it because it was not a basket; the man picked it up, was no better
// fed for it, and picked it up again. Twenty-three thousand turns and every
// one of them a success.
// --------------------------------------------------------------------------

/// An uncut carcass in the pit is not something to eat.
#[test]
fn a_haunch_nobody_has_cut_up_is_not_what_the_store_offers() {
    let database = FoodDatabase::new();
    let mut haunch = InventoryItem::new_with_weight("meat".to_string(), 20, 2.0);
    haunch.food_data = database.create_food_data(&ItemType::Meat, 0);

    let pit = Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![haunch],
        covered: true,
        dug: 0,
    };

    assert!(
        pit.something_to_eat().is_none(),
        "somebody has to take a knife to it first"
    );
}

/// Nor is a stack that has gone over.
#[test]
fn what_has_gone_over_is_not_what_the_store_offers() {
    let mut gone_off = supper(20, 0);
    if let Some(ref mut food) = gone_off.food_data {
        food.freshness = 0.0;
    }

    let pit = Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![gone_off],
        covered: true,
        dug: 0,
    };

    assert!(
        pit.something_to_eat().is_none(),
        "a rotten stack is not supper, whatever else it is"
    );
}

/// And the loop itself: a man standing on a pit of things he cannot eat is
/// not told to pick them up.
#[test]
fn nobody_is_sent_to_dig_up_what_they_cannot_eat() {
    let database = FoodDatabase::new();
    let mut haunch = InventoryItem::new_with_weight("meat".to_string(), 40, 2.0);
    haunch.food_data = database.create_food_data(&ItemType::Meat, 0);

    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![haunch],
        covered: true,
        dug: 0,
    });

    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_none(),
        "there is nothing in that hole he can put in his mouth"
    );
}

/// The pack is counted the same way. An uncut haunch does not read as
/// provisioned, and a man carrying one is not shut out of the store.
#[test]
fn a_pack_full_of_carcass_is_a_pack_with_no_meals_in_it() {
    let database = FoodDatabase::new();
    let mut haunch = InventoryItem::new_with_weight("meat".to_string(), 20, 0.1);
    haunch.food_data = database.create_food_data(&ItemType::Meat, 0);

    let mut simulation = a_digger_in_the_lean_season();
    let _ = simulation.population.agents[0].inventory.add_item(haunch);

    assert_eq!(
        simulation.population.agents[0].how_many_meals_i_have(),
        0,
        "twenty units of food about him and not one of them supper"
    );

    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
    and_everybody_knows_about_it(&mut simulation);
    let here = simulation.population.agents[0].state.position;

    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_some(),
        "so the store stays open to him"
    );
}

/// What is actually edible does count.
#[test]
fn what_can_be_eaten_counts_as_a_meal() {
    let mut simulation = a_digger();
    let _ = simulation.population.agents[0].inventory.add_item(supper(6, 0));

    assert_eq!(
        simulation.population.agents[0].how_many_meals_i_have(),
        6,
        "six meals is six meals"
    );
}

// --------------------------------------------------------------------------
// Enough is enough
//
// A hole takes three hundred and a whole settlement eats about a hundred in a
// winter, so "is there room in the pit" was never once the binding question.
// A people went on burying until the ground held four years' eating, and what
// was in there was almost all *dried* food in *lined* pits — the very best
// this model can do. It was not the wrong food. It was too much of it, sat
// there too long. See ISSUES_FOUND #43.
// --------------------------------------------------------------------------

/// An empty store wants filling.
#[test]
fn an_empty_store_wants_filling() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: Vec::new(),
        covered: false,
        dug: 0,
    });

    assert!(
        simulation.does_the_store_still_want_filling(Position::new(25, 25)),
        "there is nothing in the ground at all"
    );
}

/// A store with a lean season's eating in it for everybody about does not.
#[test]
fn a_store_with_a_winter_in_it_does_not_want_filling() {
    let mut simulation = a_digger();
    let mouths = 1;
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(mouths * Simulation::what_one_mouth_wants_put_by(), 0)],
        covered: true,
        dug: 0,
    });

    assert!(
        !simulation.does_the_store_still_want_filling(Position::new(25, 25)),
        "one man, and a season's eating for him already in the ground"
    );
}

/// It is the whole larder that is counted, not the one hole underfoot. A
/// person can see the pits round their own camp.
#[test]
fn it_is_the_whole_larder_that_is_counted_not_one_hole() {
    let mut simulation = a_digger();
    let enough = Simulation::what_one_mouth_wants_put_by();

    // Two pits a few paces apart, each holding rather less than a season
    for (n, at) in [(25, 25), (28, 25)].iter().enumerate() {
        simulation.world.pits.push(Pit {
            where_it_is: Position::new(at.0, at.1),
            holds: vec![supper(enough - 1 + n as u32, 0)],
            covered: true,
            dug: 0,
        });
    }

    assert!(
        !simulation.does_the_store_still_want_filling(Position::new(25, 25)),
        "neither hole is full on its own and between them they are a winter over"
    );
}

/// And a store a long way off is somebody else's store.
#[test]
fn a_larder_across_the_valley_is_not_this_camps_larder() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(
            25 + Simulation::WORTH_WALKING_TO_THE_STORE as i32 + 5,
            25,
        ),
        holds: vec![supper(300, 0)],
        covered: true,
        dug: 0,
    });

    assert!(
        simulation.does_the_store_still_want_filling(Position::new(25, 25)),
        "a full pit two days' walk off does not feed anybody here"
    );
}

/// More mouths, more wanted. The store is sized to the people it has to see
/// through, which is a thing somebody standing in their own camp can count.
#[test]
fn more_mouths_want_more_put_by() {
    let mut simulation = a_digger();
    let here = Position::new(25, 25);
    simulation.world.pits.push(Pit {
        where_it_is: here,
        holds: vec![supper(Simulation::what_one_mouth_wants_put_by() * 3, 0)],
        covered: true,
        dug: 0,
    });

    assert!(
        !simulation.does_the_store_still_want_filling(here),
        "three seasons' eating and one man to eat it"
    );

    for _ in 0..4 {
        simulation
            .population
            .spawn_agent(crate::agents::AgentConfig::default());
    }
    for agent in simulation.population.agents.iter_mut() {
        agent.state.position = (25, 25, 0);
        agent.state.is_alive = true;
    }

    assert!(
        simulation.does_the_store_still_want_filling(here),
        "five mouths, and what was three winters for one is not one for five"
    );
}

/// Which is the point of the whole thing: a full store stops somebody burying
/// what they are carrying.
///
/// It used to be enough to put half a pit in the ground, because a whole
/// winter for one mouth was seven items. A winter is what a body eats in a
/// day times the days the land gives nothing, and one hole does not hold it -
/// which is the second half of what this entry turned up, and is recorded in
/// `a_pit_does_not_hold_one_persons_winter` below.
#[test]
fn nobody_buries_into_a_store_that_is_already_a_winter_deep() {
    let mut simulation = a_digger_in_the_lean_season();
    let here = Position::new(25, 25);

    let a_winter = Simulation::what_one_mouth_wants_put_by();
    let mut buried = 0;
    let mut where_it_is = here.clone();
    while buried < a_winter {
        let this_one = (a_winter - buried).min(Pit::WHAT_A_PIT_TAKES);
        simulation.world.pits.push(Pit {
            where_it_is: where_it_is.clone(),
            holds: vec![supper(this_one, 0)],
            covered: true,
            dug: 0,
        });
        buried += this_one;
        where_it_is = Position::new(where_it_is.x + 1, where_it_is.y);
    }

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, 0));

    // Room in the ground, and a load in the pack, and still nothing doing
    assert!(
        simulation.world.pits.last().is_some_and(|pit| pit.has_room())
            || simulation.world.pits.len() > 1,
        "there is somewhere left to put it - room was never the binding question"
    );
    assert!(
        !simulation.does_the_store_still_want_filling(here),
        "but there is already more in the ground than anybody here will eat"
    );
}

/// And one hole does not hold one person's winter.
///
/// A pit takes three hundred; a mouth wants what it eats in a day for every
/// day the land gives it nothing, which is eight hundred and sixty-four. So a
/// settlement of twelve wants some thirty-five holes and digs, measured, under
/// three. Room in the ground was never the binding question while the target
/// was seven items a mouth; it is the binding question now, and that is filed
/// rather than fixed here - digging thirty-five holes is a different piece of
/// work from knowing how many you need.
#[test]
fn a_pit_does_not_hold_one_persons_winter() {
    let a_winter = Simulation::what_one_mouth_wants_put_by();

    assert!(
        a_winter > Pit::WHAT_A_PIT_TAKES,
        "a winter for one mouth is {a_winter} items and a hole takes {}",
        Pit::WHAT_A_PIT_TAKES
    );
}

// --------------------------------------------------------------------------
// A store is for the stretch when the land gives nothing
// --------------------------------------------------------------------------

/// The bare stretch is read off the bearing year rather than named, so
/// retuning the year retunes the store with it.
#[test]
fn the_bare_stretch_is_read_off_the_bearing_year() {
    use crate::environment::seasons::{first_day_of, PartOfSeason, Season, DAYS_PER_YEAR};

    let bare = Simulation::how_long_the_hedgerows_give_nothing();

    assert!(bare > 0, "some part of the year has to be bare, or a store is pointless");
    assert!(
        bare < DAYS_PER_YEAR,
        "and some part of it has to bear, or nothing could ever be put by: {bare}"
    );

    // Every day it says is bare should actually be bare, and the run should be
    // the longest one there is
    let mut longest = 0;
    let mut running = 0;
    for day in 0..(DAYS_PER_YEAR * 2) {
        running = if Simulation::are_the_hedgerows_bearing_on(day % DAYS_PER_YEAR) {
            0
        } else {
            running + 1
        };
        if day >= DAYS_PER_YEAR {
            longest = longest.max(running.min(DAYS_PER_YEAR));
        }
    }
    assert_eq!(bare, longest);

    // As the year stands: deep winter is bare and the growing half is not
    assert!(!Simulation::are_the_hedgerows_bearing_on(first_day_of(
        Season::Winter,
        PartOfSeason::Deep
    )));
    for bearing in [
        (Season::Spring, PartOfSeason::Early),
        (Season::Summer, PartOfSeason::Deep),
        (Season::Fall, PartOfSeason::Deep),
    ] {
        assert!(
            Simulation::are_the_hedgerows_bearing_on(first_day_of(bearing.0, bearing.1)),
            "{bearing:?} should have something on the hedges"
        );
    }
}

/// A winter store is what a body eats for as many days as the land gives it
/// nothing - and it was seven items, which is half a day.
#[test]
fn a_winter_store_is_a_winter_of_eating() {
    let a_day = crate::agents::provision::WHAT_A_BODY_EATS_IN_A_DAY;
    let bare = Simulation::how_long_the_hedgerows_give_nothing();
    let want = Simulation::what_one_mouth_wants_put_by();

    assert_eq!(want, (a_day * bare as f32).ceil() as u32);

    assert!(
        want as f32 > a_day * 30.0,
        "a winter store that is under a month of food is not a winter store: \
         {want} items against {:.0} for a month",
        a_day * 30.0
    );
}

/// Nobody opens the winter store in July.
///
/// This is the entry's title. Nothing asked the season, so a pit within reach
/// was simply the nearest food: a settlement drew on its store all year and
/// the pits held between seven and fourteen items from one end of a year to
/// the other, never accumulating.
#[test]
fn the_store_is_not_opened_while_the_hedges_are_bearing() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });

    assert!(
        simulation.are_the_hedgerows_bearing(),
        "the year opens in spring, when there is leaf on every hedge"
    );

    let here = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_none(),
        "there is food growing; the store is for when there is not"
    );
}

/// But a man who is actually starving opens it in any month.
///
/// A rule that let somebody starve beside a full pit would be a worse fault
/// than the one it fixed.
#[test]
fn a_starving_man_opens_the_store_whatever_the_month() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
    and_everybody_knows_about_it(&mut simulation);

    assert!(simulation.are_the_hedgerows_bearing(), "still spring");

    let agent = &mut simulation.population.agents[0];
    agent.state.physiology.reserve = 0.0;
    assert!(agent.state.is_starving(), "and this one is three days in");

    let here = simulation.population.agents[0].state.position;
    assert!(
        simulation
            .something_out_of_the_store(&simulation.population.agents[0], here)
            .is_some(),
        "he does not keep larder discipline on an empty reserve"
    );
}

// --------------------------------------------------------------------------
// Burying what will keep
// --------------------------------------------------------------------------

/// A pit knows how long what goes into it will still be food.
///
/// Bare earth doubles a thing's life and a lined pit quadruples it, and the
/// same number does the ageing and answers the question - see
/// `Pit::how_much_slower_things_age`.
#[test]
fn a_pit_says_how_long_a_thing_will_keep_in_it() {
    use crate::agents::InventoryItem;
    use crate::world::nutrition::{FoodDatabase, PreparationState};
    use crate::world::{ItemType, Pit, Position};

    let database = FoodDatabase::new();
    let mut leaf = InventoryItem::new_with_weight("greens".to_string(), 10, 0.5);
    leaf.food_data = database.create_food_data(&ItemType::Greens, 0);

    let bare = Pit { where_it_is: Position::new(0, 0), holds: Vec::new(), covered: true, dug: 0 };
    let mut lined = Pit { where_it_is: Position::new(1, 0), holds: Vec::new(), covered: true, dug: 0 };
    lined.put_in(InventoryItem::new_with_weight("bowl".to_string(), 1, 1.0));

    let in_bare = bare.how_long_this_would_keep(&leaf, 0).expect("leaf has a clock");
    let in_lined = lined.how_long_this_would_keep(&leaf, 0).expect("leaf has a clock");

    assert!(in_bare > 0.0, "leaf keeps some time in the ground");
    assert!(
        (in_lined - in_bare * 2.0).abs() < 0.01,
        "a bowl between the food and the ground doubles it again: {in_bare} against {in_lined}"
    );

    // And drying it is worth far more than the hole is.
    let mut dried = leaf.clone();
    if let Some(food) = dried.food_data.as_mut() {
        food.preparation = PreparationState::Dried;
    }
    let kept = lined.how_long_this_would_keep(&dried, 0).expect("still has a clock");
    assert!(
        kept > in_lined * 10.0,
        "drying should be worth more than the hole: {in_lined} raw against {kept} dried"
    );
}

/// Leaf will not last the winter in a hole, and nobody should bury it there.
///
/// A settlement buried 512 units a year and ate four of them: 98.4% rotted,
/// and 86% of what went in went in raw. Raw greens keep six days in bare earth
/// against the seventy-five the land gives nothing. See ISSUES_FOUND.md #124.
#[test]
fn nothing_goes_in_the_ground_that_will_not_still_be_food_when_it_is_wanted() {
    use crate::agents::InventoryItem;
    use crate::world::nutrition::{FoodDatabase, PreparationState};
    use crate::world::{ItemType, Pit, Position};

    let database = FoodDatabase::new();
    let bare = Pit { where_it_is: Position::new(0, 0), holds: Vec::new(), covered: true, dug: 0 };
    let bare_stretch =
        crate::agents::provision::how_long_the_land_gives_nothing() as f32;

    let mut raw = InventoryItem::new_with_weight("greens".to_string(), 10, 0.5);
    raw.food_data = database.create_food_data(&ItemType::Greens, 0);
    let raw_days = bare.how_long_this_would_keep(&raw, 0).expect("has a clock");

    assert!(
        raw_days < bare_stretch,
        "raw leaf lasting {raw_days} days would see out a {bare_stretch}-day winter, \
         which is not the world this rule was written for"
    );

    let mut dried = raw.clone();
    if let Some(food) = dried.food_data.as_mut() {
        food.preparation = PreparationState::Dried;
    }
    let dried_days = bare.how_long_this_would_keep(&dried, 0).expect("has a clock");

    assert!(
        dried_days >= bare_stretch,
        "dried leaf should see out the winter: {dried_days} days against {bare_stretch}"
    );
}

/// And a load that will not keep does not go in the ground at all.
///
/// The other half of `a_surplus_and_a_pit_means_burying`: somebody who has
/// never watched anything dry, standing on a hole with an armful of berries
/// that keep twenty-four days against a seventy-five day winter, is not
/// putting anything by by burying them. See ISSUES_FOUND.md #124.
#[test]
fn raw_food_that_will_not_last_the_winter_is_not_buried() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, simulation.world.tick));

    let here = simulation.population.agents[0].state.position;
    let answer = simulation.putting_food_by(&simulation.population.agents[0], here);

    assert!(
        !matches!(&answer, Some(Action::Cover { .. })),
        "burying leaf that goes off in a fortnight is not putting anything by: {answer:?}"
    );
}

/// Everybody is born knowing that food left in the sun keeps.
///
/// It used to have to be watched happening, and the only route to watching it
/// was a branch that fired when somebody happened to put food down on a clear
/// day. That made preserving a thing a settlement stumbled into rather than a
/// thing it did: 86% of what went into the ground went in raw and 98.4% of it
/// rotted. See `Agent::what_anybody_is_born_knowing` and ISSUES_FOUND.md #125.
#[test]
fn drying_is_not_something_anybody_has_to_be_shown() {
    use crate::agents::{Agent, AgentConfig};

    let born = Agent::new(AgentConfig::default());

    assert!(
        born.what_i_found_out()
            .contains(Agent::THAT_LAYING_IT_OUT_KEEPS_IT),
        "a person knows what the sun does to a thing left out in it"
    );

    // And it is not everything: what the making chain calls a discovery still
    // has to be discovered.
    assert_eq!(
        born.what_i_found_out().len(),
        Agent::what_anybody_is_born_knowing().len(),
        "born knowing exactly what everybody is born knowing, and no more"
    );
}

/// So a digger with a hole and an armful can put something by without having
/// had to see it done first.
#[test]
fn somebody_who_has_never_been_shown_can_still_put_food_by() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, simulation.world.tick));

    let dried = simulation.execute_action(
        &Action::Dry { what: "food".to_string() },
        0,
    );

    assert!(
        dried.success,
        "nobody had to teach him this: {:?}",
        dried.message
    );
}

// --------------------------------------------------------------------------
// What nobody can carry
// --------------------------------------------------------------------------

/// A pack cannot hold more than a pack holds, and until now one routinely did.
///
/// `add_item` refuses what will not fit, so a load can never be *put* over the
/// limit. It got there the other way: `max_weight` is worked out fresh every
/// turn from what the body can lift, and a body that goes hungry lifts less
/// than it did. A man loaded up in his strong summer woke in November
/// carrying more than he could hold, and because a pack already over its limit
/// refuses everything, the load was frozen there for the rest of his life. He
/// could never pick up food again. See ISSUES_FOUND.md #126.
#[test]
fn a_body_that_weakens_sets_down_what_it_can_no_longer_carry() {
    let mut simulation = a_digger();

    // A load he can manage today
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 2.0));

    let carried = simulation.population.agents[0].inventory.current_weight;
    assert!(carried > 0.0, "he is carrying something");

    // And a pack that will not take it any more
    simulation.population.agents[0].inventory.max_weight = carried / 2.0;
    assert!(
        simulation.population.agents[0].how_much_too_much_i_am_carrying() > 0.0,
        "he is over his limit"
    );

    simulation.what_nobody_can_carry_any_more();

    assert_eq!(
        simulation.population.agents[0].how_much_too_much_i_am_carrying(),
        0.0,
        "he put down what he could not hold: {:.1} against {:.1}",
        simulation.population.agents[0].inventory.current_weight,
        simulation.population.agents[0].inventory.max_weight,
    );
}

/// And it is set down, not destroyed. Somebody can come back for it.
#[test]
fn what_is_set_down_is_still_there_to_be_picked_up() {
    let mut simulation = a_digger();
    let here = Position::new(25, 25);

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), 8, 2.0));
    simulation.population.agents[0].inventory.max_weight = 1.0;

    simulation.what_nobody_can_carry_any_more();

    let on_the_ground: u32 = simulation
        .world
        .what_is_lying_at(&here)
        .iter()
        .filter(|dropped| dropped.item.item_id == "wood")
        .map(|dropped| dropped.item.quantity)
        .sum();

    assert!(
        on_the_ground > 0,
        "it went on the grass where he stood, not out of the world"
    );
}

/// The pack goes last, and the tool in his hand and his supper before it.
///
/// A man walking under a load he cannot manage puts the stone down, not the
/// axe he is working with and not the food he is going to eat.
#[test]
fn the_axe_and_the_supper_are_the_last_things_anybody_puts_down() {
    let mut simulation = a_digger();

    // `a_digger` already gave him a handaxe
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("stone".to_string(), 6, 5.0));
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(4, 0));

    // Far too much for him
    simulation.population.agents[0].inventory.max_weight = 3.0;

    simulation.what_nobody_can_carry_any_more();

    let agent = &simulation.population.agents[0];
    assert_eq!(
        agent.how_many_i_have("stone"),
        0,
        "the stone is what goes"
    );
    assert!(agent.how_many_i_have("handaxe") > 0, "not the axe he works with");
    assert!(agent.how_many_i_have("food") > 0, "and not his supper");
}

/// What goes down is only as much as the shortfall wants.
///
/// A man who tips his whole bundle of firewood on the grass to carry home a
/// handful of berries has to go and cut more tomorrow, and one who puts three
/// sticks down does not.
#[test]
fn only_as_much_goes_down_as_the_shortfall_wants() {
    let mut simulation = a_digger();

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), 20, 2.0));

    let carried = simulation.population.agents[0].inventory.current_weight;
    // A hair under what he is carrying: one stick's worth of shortfall, plus
    // the day's food the reckoning leaves room for.
    simulation.population.agents[0].inventory.max_weight = carried - 1.0;

    simulation.what_nobody_can_carry_any_more();

    let left = simulation.population.agents[0].how_many_i_have("wood");
    assert!(
        left > 0 && left < 20,
        "he set some of it down and kept the rest: {left} sticks"
    );
}

/// A pack inside its limit is not touched at all.
#[test]
fn nobody_puts_anything_down_who_does_not_have_to() {
    let mut simulation = a_digger();

    let _ = simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight("wood".to_string(), 2, 2.0));
    let before = simulation.population.agents[0].how_many_i_have("wood");

    simulation.what_nobody_can_carry_any_more();

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("wood"),
        before,
        "there was room for it"
    );
}
