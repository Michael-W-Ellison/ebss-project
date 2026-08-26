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
        Simulation::WHAT_A_PERSON_KEEPS_ON_THEM < Simulation::ENOUGH_NOT_TO_OPEN_THE_STORE,
        "burying leaves {} and the store shuts at {}: a man who has just \
         filled a pit is locked out of it",
        Simulation::WHAT_A_PERSON_KEEPS_ON_THEM,
        Simulation::ENOUGH_NOT_TO_OPEN_THE_STORE,
    );
}

/// And in the world rather than in the arithmetic: the one meal `Cover` hands
/// back is not a reason to leave the rest of it in the ground.
#[test]
fn one_meal_in_the_pack_does_not_shut_the_store() {
    let mut simulation = a_digger();
    simulation.world.pits.push(Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![supper(40, 0)],
        covered: true,
        dug: 0,
    });
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
        Simulation::ENOUGH_NOT_TO_OPEN_THE_STORE,
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

    let mut simulation = a_digger();
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
        holds: vec![supper(mouths * Simulation::WHAT_ONE_MOUTH_WANTS_PUT_BY, 0)],
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
    let enough = Simulation::WHAT_ONE_MOUTH_WANTS_PUT_BY;

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
        holds: vec![supper(Simulation::WHAT_ONE_MOUTH_WANTS_PUT_BY * 3, 0)],
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
#[test]
fn nobody_buries_into_a_store_that_is_already_a_winter_deep() {
    let mut simulation = a_digger();
    let here = Position::new(25, 25);
    simulation.world.pits.push(Pit {
        where_it_is: here,
        holds: vec![supper(Pit::WHAT_A_PIT_TAKES / 2, 0)],
        covered: true,
        dug: 0,
    });
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(supper(30, 0));

    // Room in the hole, and a load in the pack, and still nothing doing
    assert!(
        simulation.world.pits[0].has_room(),
        "the hole is only half full - room was never the binding question"
    );
    assert!(
        !simulation.does_the_store_still_want_filling(here),
        "but there is already more in the ground than anybody here will eat"
    );
}
