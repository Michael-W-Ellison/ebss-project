// src/analytics/tests/keeping_tests.rs
//! Tests for food going off, and for the three things anybody here can do
//! about it.
//!
//! Every one of the spoilage tables was written as a day-count and stored as
//! ticks at 1440 to the day. The calendar was later put on a scale a life
//! fits inside — `TICKS_PER_DAY` is 12 — and the food tables were not brought
//! with it, so meat written down as lasting a day lasted a hundred and twenty
//! of them and grain written down as ten days lasted twelve and a half years.
//!
//! Nothing in this world spoiled, and everything followed from that: nobody
//! ever went hungry, a larder was insurance against nothing, and six of the
//! nine preparation states had never once been reached, because there was no
//! reason to preserve anything.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::seasons::TICKS_PER_DAY;
use crate::environment::{verbs, Action};
use crate::world::nutrition::{FoodDatabase, PreparationState};
use crate::world::{ItemType, Position, World, WorldConfig};

fn a_meal(of: ItemType, called: &str, how_many: u32, made_at: u32) -> InventoryItem {
    let database = FoodDatabase::new();
    let mut meal = InventoryItem::new_with_weight(called.to_string(), how_many, 0.5);
    meal.food_data = database.create_food_data(&of, made_at);
    meal
}

fn one_person() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

fn how_long_it_lasts(of: ItemType) -> u32 {
    FoodDatabase::new()
        .create_food_data(&of, 0)
        .expect("in the database")
        .base_spoilage_ticks
}

// --------------------------------------------------------------------------
// The clock
// --------------------------------------------------------------------------

/// Meat does not see a season out.
///
/// A first cut of the rescale used the day-counts the tables were written
/// with — meat a day — and that is a different thing on this calendar than it
/// was on the old one: a tick here is an action, not a minute, and walking
/// out to a kill and back is thirty or forty of them. Food that lasts less
/// than the trip that fetches it is not scarcity, it is a broken model, and
/// it cost a settlement a fifth of its people.
#[test]
fn meat_does_not_see_a_season_out() {
    let lasts = how_long_it_lasts(ItemType::Meat);
    let a_season = TICKS_PER_DAY * 24;

    assert!(
        lasts < a_season,
        "meat should be carrion before the season turns, and this lasts {} days",
        lasts / TICKS_PER_DAY
    );
    assert!(
        lasts > TICKS_PER_DAY * 4,
        "and it should outlast the walk home"
    );
}

/// And fish faster than anything else anybody catches.
#[test]
fn fish_rots_faster_than_meat() {
    assert!(how_long_it_lasts(ItemType::Fish) < how_long_it_lasts(ItemType::Meat));
}

/// And berries off the bush do not see a season out either.
#[test]
fn berries_off_the_bush_do_not_keep() {
    let lasts = how_long_it_lasts(ItemType::Food);

    assert!(
        lasts < TICKS_PER_DAY * 24,
        "half a season and they are jam on the inside of the pack, not {} days",
        lasts / TICKS_PER_DAY
    );
}

/// A dry seed keeps a season and more, which is what makes grain worth
/// growing.
#[test]
fn grain_keeps_a_season() {
    let lasts = how_long_it_lasts(ItemType::Grain);

    assert!(
        lasts >= TICKS_PER_DAY * 24,
        "a dry seed should see a settlement through to spring, and this lasts {} days",
        lasts / TICKS_PER_DAY
    );
    assert!(
        lasts <= TICKS_PER_DAY * 96,
        "and not for years on end"
    );
    assert!(
        lasts > how_long_it_lasts(ItemType::Meat),
        "and it should keep far better than a carcass"
    );
}

/// Nothing keeps for a lifetime except honey.
#[test]
fn nothing_but_honey_outlasts_a_person() {
    let a_life = 8000;

    for (what, of) in [
        ("meat", ItemType::Meat),
        ("fish", ItemType::Fish),
        ("berries", ItemType::Food),
        ("grain", ItemType::Grain),
        ("bread", ItemType::Bread),
        ("cheese", ItemType::Cheese),
        ("ale", ItemType::Ale),
    ] {
        assert!(
            how_long_it_lasts(of) < a_life,
            "{what} outlasts the person carrying it"
        );
    }

    assert!(
        how_long_it_lasts(ItemType::Honey) > a_life,
        "honey is the exception and always was"
    );
}

/// Food in a pack actually goes off now, which is the whole point.
#[test]
fn what_is_carried_goes_off() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meat", 4, 0));

    for _ in 0..(TICKS_PER_DAY * 12) {
        simulation.population.agents[0].tick_food_spoilage(simulation.world.tick);
        simulation.world.tick();
    }

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("meat"),
        0,
        "a fortnight on, that is not meat any more"
    );
}

// --------------------------------------------------------------------------
// Drying it
// --------------------------------------------------------------------------

/// Laying food out makes it keep.
#[test]
fn drying_food_makes_it_keep() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meat", 4, 0));

    let result = simulation.execute_action(
        &Action::Dry {
            what: "meat".to_string(),
        },
        0,
    );

    assert!(result.success, "{:?}", result.message);

    let how = simulation.population.agents[0]
        .inventory
        .get_item("meat")
        .and_then(|item| item.food_data.as_ref())
        .map(|food| food.preparation)
        .expect("still in the pack");

    assert_eq!(how, PreparationState::Dried);
    assert!(
        how.spoilage_multiplier() < PreparationState::Raw.spoilage_multiplier(),
        "and that is what makes it worth doing"
    );
}

/// Dried meat outlasts raw meat by a long way.
#[test]
fn dried_meat_outlasts_raw_meat() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meat", 4, 0));
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "venison", 4, 0));

    simulation.execute_action(
        &Action::Dry {
            what: "venison".to_string(),
        },
        0,
    );

    for _ in 0..(TICKS_PER_DAY * 14) {
        simulation.population.agents[0].tick_food_spoilage(simulation.world.tick);
        simulation.world.tick();
    }

    assert_eq!(
        simulation.population.agents[0].how_many_i_have("meat"),
        0,
        "the raw one is gone"
    );
    assert!(
        simulation.population.agents[0].how_many_i_have("venison") > 0,
        "and the dried one is supper"
    );
}

/// Hung in the smoke of a fire it is smoked rather than dried, which is what
/// a people that can make fire does in weather that will not dry anything.
#[test]
fn food_hung_over_a_fire_is_smoked() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meat", 4, 0));

    let fire = simulation
        .world
        .build_heat_source(
            crate::environment::HeatSourceType::Campfire,
            (25, 25, 0),
            None,
        )
        .expect("a hearth goes here");
    let _ = simulation
        .world
        .add_fuel_to_heat_source(&fire, "wood".to_string(), 20.0);
    simulation
        .world
        .light_heat_source(&fire)
        .expect("and it lights");

    simulation.execute_action(
        &Action::Dry {
            what: "meat".to_string(),
        },
        0,
    );

    let how = simulation.population.agents[0]
        .inventory
        .get_item("meat")
        .and_then(|item| item.food_data.as_ref())
        .map(|food| food.preparation)
        .expect("still in the pack");

    assert_eq!(how, PreparationState::Smoked);
}

/// Nobody dries what is already seen to.
#[test]
fn nobody_dries_the_same_thing_twice() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Meat, "meat", 4, 0));

    simulation.execute_action(
        &Action::Dry {
            what: "meat".to_string(),
        },
        0,
    );
    let again = simulation.execute_action(
        &Action::Dry {
            what: "meat".to_string(),
        },
        0,
    );

    assert!(!again.success, "it is already dried");
}

/// And nobody dries carrion. Preserving does not undo what has already
/// happened to a thing.
#[test]
fn nobody_dries_what_has_already_turned() {
    let mut simulation = one_person();
    let mut going_off = a_meal(ItemType::Meat, "meat", 4, 0);
    if let Some(food) = going_off.food_data.as_mut() {
        food.freshness = 0.1;
    }
    let _ = simulation.population.agents[0].inventory.add_item(going_off);

    let result = simulation.execute_action(
        &Action::Dry {
            what: "meat".to_string(),
        },
        0,
    );

    assert!(
        !result.success,
        "all you get from drying carrion is dry carrion"
    );
}

/// Somebody with a surplus dries it before they think about burying it.
#[test]
fn a_surplus_gets_dried_before_it_gets_buried() {
    let mut simulation = one_person();
    let _ = simulation.population.agents[0]
        .inventory
        .add_item(a_meal(ItemType::Food, "food", 30, 0));

    let here = simulation.population.agents[0].state.position;
    let answer = simulation
        .putting_food_by(&simulation.population.agents[0], here)
        .expect("he is carrying more than he can eat");

    assert!(
        matches!(&answer, Action::Dry { what } if what == "food"),
        "a hole makes it keep four times as long and drying makes it keep \
         twenty: {answer:?}"
    );
}

// --------------------------------------------------------------------------
// Leaving it out
// --------------------------------------------------------------------------

/// Food left lying in the weather goes off faster than food in a pack.
#[test]
fn what_is_left_out_goes_off_faster_than_what_is_carried() {
    let mut simulation = one_person();
    let here = Position::new(25, 25);

    simulation
        .world
        .somebody_left_this(a_meal(ItemType::Grain, "grain", 10, 0), here, 0);

    let mut in_the_pack = a_meal(ItemType::Grain, "grain", 10, 0);

    for _ in 0..100 {
        simulation.world.tick();
    }
    if let Some(food) = in_the_pack.food_data.as_mut() {
        food.update_freshness(simulation.world.tick);
    }

    let out_in_the_rain = simulation
        .world
        .what_is_lying_at(&here)
        .first()
        .and_then(|left| left.item.food_data.as_ref())
        .map(|food| food.freshness)
        .unwrap_or(0.0);

    let carried = in_the_pack
        .food_data
        .as_ref()
        .map(|food| food.freshness)
        .unwrap_or(0.0);

    assert!(
        out_in_the_rain < carried,
        "sun, rain and flies: {out_in_the_rain:.2} against {carried:.2}"
    );
}

// --------------------------------------------------------------------------
// A vessel in the ground
// --------------------------------------------------------------------------

/// A pit with a bowl in it keeps better than bare earth.
#[test]
fn a_lined_pit_keeps_better_than_bare_earth() {
    use crate::world::Pit;

    let keeping = |lined: bool| {
        let mut simulation = one_person();
        let mut holds = vec![a_meal(ItemType::Food, "food", 20, 0)];
        if lined {
            holds.push(InventoryItem::new_with_weight("bowl".to_string(), 1, 1.0));
        }

        simulation.world.pits.push(Pit {
            where_it_is: Position::new(25, 25),
            holds,
            covered: true,
            dug: 0,
        });

        for _ in 0..40 {
            simulation.world.tick();
        }

        simulation
            .world
            .pits
            .first()
            .and_then(|pit| pit.holds.iter().find(|held| held.item_id == "food"))
            .and_then(|held| held.food_data.as_ref())
            .map(|food| food.freshness)
            .unwrap_or(0.0)
    };

    let bare = keeping(false);
    let lined = keeping(true);

    assert!(
        lined > bare,
        "a bowl between the food and the damp: {lined:.2} against {bare:.2}"
    );
}

/// And the bowl is not supper. A hungry man who walks to a store and comes
/// back with the vessel has not eaten.
#[test]
fn the_lining_is_not_stores() {
    use crate::world::Pit;

    let pit = Pit {
        where_it_is: Position::new(25, 25),
        holds: vec![InventoryItem::new_with_weight("bowl".to_string(), 1, 1.0)],
        covered: true,
        dug: 0,
    };

    assert!(pit.is_lined(), "there is a bowl in it");
    assert!(!pit.has_food(), "and nothing to eat");
    assert_eq!(pit.how_much_is_in_it(), 0, "the lining is not stores");
}

// --------------------------------------------------------------------------
// The matrix
// --------------------------------------------------------------------------

/// Drying is a verb like any other.
#[test]
fn drying_is_in_the_matrix() {
    let one = verbs::what_that_verb_is("dry").expect("in the matrix");

    assert!(one.is_live(), "something performs it");
    assert_eq!(one.family, verbs::Family::Thermal);
}
