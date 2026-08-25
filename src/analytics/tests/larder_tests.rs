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

/// A person keeps a couple of days about them rather than burying the lot.
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
        Agent::ENOUGH_TO_HAND,
        "he walks away from the pit with something to eat on the way home"
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
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(31, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });

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

/// And with a pit under him, buries it.
#[test]
fn a_surplus_and_a_pit_means_burying() {
    let mut simulation = a_digger();
    simulation.execute_action(&Action::Excavate, 0);
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
    simulation.execute_action(&Action::Excavate, 0);
    turn_the_year_to(&mut simulation, Season::Summer);
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, simulation.world.tick));
    simulation.execute_action(
        &Action::Dry {
            what: "food".to_string(),
        },
        0,
    );

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
