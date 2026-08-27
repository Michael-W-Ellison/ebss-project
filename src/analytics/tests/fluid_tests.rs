// src/analytics/tests/fluid_tests.rs
//! Tests for the family that was entirely declaration until there was
//! something to hold water in.
//!
//! Soaking, fermenting and boiling all want a vessel with something in it, and
//! nobody in this world could carry water until somebody worked out how to
//! hollow out a block of wood. That is the order these things have to come in
//! and it is why the fluid family sat idle in the matrix while every other
//! family got built.
//!
//! What each of them buys is different and each of them is real. Flax left in
//! water lets go of its fibre and gives three times the cordage. Fruit and
//! water left alone turn into something that keeps a fortnight where berries
//! keep hours. And a pot of grain over a fire is the only way to cook grain at
//! all, because a fire on its own ruins it.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::verbs::{self, Wants};
use crate::environment::{making, Action};
use crate::world::{World, WorldConfig};

fn a_person() -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.position = (25, 25, 0);

    let everything: Vec<(String, u32)> = simulation.population.agents[0]
        .inventory
        .get_all_items()
        .values()
        .map(|item| (item.item_id.clone(), item.quantity))
        .collect();

    for (what, how_many) in everything {
        for _ in 0..how_many {
            simulation.population.agents[0]
                .inventory
                .remove_item(&what, 1);
        }
    }

    simulation
}

fn give(simulation: &mut Simulation, what: &str, how_many: u32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_with_weight(
            what.to_string(),
            how_many,
            1.0,
        ));
}

/// A bowl with water in it
fn give_a_full_bowl(simulation: &mut Simulation, holding: f32) {
    simulation.population.agents[0]
        .inventory
        .add_item(InventoryItem::new_container("bowl".to_string(), 1, 10.0));
    simulation.population.agents[0]
        .inventory
        .fill_containers(holding);
}

fn work(verb: &str, to: &str) -> Action {
    Action::Work {
        verb: verb.to_string(),
        to: to.to_string(),
    }
}

// --------------------------------------------------------------------------
// A vessel is what the family wants
// --------------------------------------------------------------------------

/// Every fluid verb the matrix says is live wants a vessel.
#[test]
fn the_fluid_verbs_want_something_to_hold_water_in() {
    for one in verbs::EVERY_VERB
        .iter()
        .filter(|verb| verb.family == verbs::Family::Fluid)
        .filter(|verb| verb.is_live())
    {
        assert_eq!(
            one.wants,
            Wants::AVessel,
            "{} is done with water and water has to be carried",
            one.called
        );
    }
}

/// A vessel means a full one. An empty bowl is a bowl, not a means.
#[test]
fn an_empty_bowl_will_not_do() {
    let nothing_held = |_: &str| 0;
    let no_tools = |_| false;

    assert!(!Wants::AVessel.satisfied_by_hands(&nothing_held, &no_tools, true, 0.0));
    assert!(Wants::AVessel.satisfied_by_hands(&nothing_held, &no_tools, true, 3.0));
}

/// And the executor refuses without one, from the matrix and nowhere else.
#[test]
fn nobody_soaks_anything_without_water() {
    assert_eq!(
        verbs::what_this_action_cannot_do_without("soak"),
        vec![Wants::AVessel],
    );

    let mut simulation = a_person();
    give(&mut simulation, "flax", 6);
    simulation.population.agents[0].found_out_how_to("rettedflax");

    let dry = simulation.execute_action(&work("soak", "flax"), 0);
    assert!(
        !dry.success,
        "you cannot ret flax without water: {:?}",
        dry.message
    );
    assert_eq!(
        simulation.population.agents[0].how_many_i_have("flax"),
        6,
        "and nothing is spent trying"
    );
}

// --------------------------------------------------------------------------
// Retting
// --------------------------------------------------------------------------

/// Flax left in water gives three times the cordage.
#[test]
fn retted_flax_gives_more_cordage_than_raw_flax() {
    let raw = making::every_way_to_make("lashing")
        .find(|step| step.needs.iter().any(|(what, _)| *what == "flax"))
        .expect("cordage comes off raw flax");
    let retted = making::every_way_to_make("lashing")
        .find(|step| step.needs.iter().any(|(what, _)| *what == "rettedflax"))
        .expect("and off retted flax");

    let per_stem = |step: &making::Making| {
        step.how_many as f32
            / step.needs.iter().map(|(_, how_many)| *how_many).sum::<u32>() as f32
    };

    assert!(
        per_stem(retted) > per_stem(raw),
        "retting is what it is for: {:.1} against {:.1}",
        per_stem(retted),
        per_stem(raw)
    );
}

/// And soaking it takes the water out of the bowl.
#[test]
fn soaking_flax_uses_up_the_water() {
    let mut simulation = a_person();
    give(&mut simulation, "flax", 6);
    give_a_full_bowl(&mut simulation, 8.0);
    simulation.population.agents[0].found_out_how_to("rettedflax");

    let before = simulation.population.agents[0].how_much_water_i_carry();
    assert!(before > 0.0, "the bowl starts full");

    let result = simulation.execute_action(&work("soak", "flax"), 0);
    assert!(result.success, "flax rets: {:?}", result.message);

    assert!(
        simulation.population.agents[0].how_many_i_have("rettedflax") > 0,
        "and there is retted flax in the pack"
    );
    assert!(
        simulation.population.agents[0].how_much_water_i_carry() < before,
        "and less water in the bowl than there was"
    );
}

// --------------------------------------------------------------------------
// Fermenting
// --------------------------------------------------------------------------

/// What fruit and water turn into keeps a fortnight where berries keep hours.
#[test]
fn what_ferments_keeps_far_longer_than_what_went_in() {
    let simulation = a_person();

    let keeps = |what: crate::world::ItemType| {
        simulation
            .food_database
            .create_food_data(&what, 0)
            .expect("food")
            .base_spoilage_ticks
    };

    assert!(
        keeps(crate::world::ItemType::Ale) > keeps(crate::world::ItemType::Food) * 4,
        "this is the storing the specification asked for: {} against {}",
        keeps(crate::world::ItemType::Ale),
        keeps(crate::world::ItemType::Food)
    );
}

/// And it comes out as food, not as a lump of stuff with a name on it.
#[test]
fn what_ferments_is_something_you_can_live_on() {
    let mut simulation = a_person();
    give(&mut simulation, "food", 8);
    give_a_full_bowl(&mut simulation, 8.0);
    simulation.population.agents[0].found_out_how_to("ale");

    let result = simulation.execute_action(&work("ferment", "food"), 0);
    assert!(result.success, "berries ferment: {:?}", result.message);

    let made = simulation.population.agents[0]
        .inventory
        .get_item("ale")
        .expect("there is something in the bowl now");

    assert!(
        made.food_data.is_some(),
        "and it is a thing a person can live on"
    );
}

// --------------------------------------------------------------------------
// Boiling
// --------------------------------------------------------------------------

/// A fire on its own ruins flour, which is why a pot matters.
///
/// Whole grain improves on a fire and ground grain does not, which is a
/// distinction the food tables already drew and nothing had ever used.
#[test]
fn a_fire_on_its_own_ruins_flour() {
    use crate::world::nutrition::CookingOutcome;
    use crate::world::ItemType;

    assert_eq!(
        ItemType::Grain.cooking_outcome(),
        CookingOutcome::Improves,
        "a handful of grain in the embers is better for it"
    );
    assert_eq!(
        ItemType::Flour.cooking_outcome(),
        CookingOutcome::Ruins,
        "a handful of flour in the embers is a handful of ash"
    );

    // And there is a way to cook it that is not a fire on its own
    assert!(
        making::how_to_work("boil", "flour").is_some(),
        "a pot of flour and water is the answer"
    );
}

/// The whole chain from a seed: grain, crushed, boiled, is bread.
#[test]
fn a_seed_becomes_bread_in_three_steps() {
    let crushing = making::how_to_work("crush", "grain").expect("grain grinds");
    assert_eq!(crushing.makes, "flour");

    let boiling = making::how_to_work("boil", &crushing.makes.to_string())
        .expect("and flour boils");
    assert_eq!(boiling.makes, "bread");

    // And what comes out at the end is worth having
    let simulation = a_person();
    let worth = |what: crate::world::ItemType| {
        simulation
            .food_database
            .create_food_data(&what, 0)
            .expect("food")
    };

    assert_eq!(
        worth(crate::world::ItemType::Bread).preparation,
        crate::world::nutrition::PreparationState::Cooked,
        "bread comes out of the pot already cooked"
    );
}

/// Boiling wants a fire as well as a vessel.
#[test]
fn boiling_wants_a_fire_as_well_as_a_pot() {
    let step = making::how_to_work("boil", "flour").expect("in the table");
    assert!(step.over_a_fire, "you cannot boil anything cold");
    assert!(step.wants_water > 0.0, "or dry");

    let mut simulation = a_person();
    give(&mut simulation, "flour", 6);
    give_a_full_bowl(&mut simulation, 8.0);
    simulation.population.agents[0].found_out_how_to("bread");

    let cold = simulation.execute_action(&work("boil", "flour"), 0);
    assert!(
        !cold.success,
        "a pot and no fire is a pot: {:?}",
        cold.message
    );
    assert!(
        cold.message
            .as_deref()
            .is_some_and(|said| said.contains("fire")),
        "and it should say so: {:?}",
        cold.message
    );
}

/// With a fire, it is a meal.
#[test]
fn a_pot_over_a_fire_makes_a_meal_of_flour() {
    let mut simulation = a_person();
    give(&mut simulation, "flour", 6);
    give_a_full_bowl(&mut simulation, 8.0);
    simulation.population.agents[0].found_out_how_to("bread");

    // Wood enough for a hearth, and light it
    give(&mut simulation, "wood", 12);
    let lit = simulation.execute_action(&Action::LightFire, 0);
    assert!(lit.success, "a fire goes up: {:?}", lit.message);

    let result = simulation.execute_action(&work("boil", "flour"), 0);
    assert!(result.success, "and the pot goes on it: {:?}", result.message);

    let made = simulation.population.agents[0]
        .inventory
        .get_item("bread")
        .expect("there is a meal in the pack");

    assert!(made.food_data.is_some(), "and it is food");
    assert!(
        simulation.population.agents[0].how_many_i_have("flour") < 6,
        "and the flour went into it"
    );
}

// --------------------------------------------------------------------------
// Nobody sets out to do what they cannot
// --------------------------------------------------------------------------

/// An agent with no water does not propose to soak anything.
#[test]
fn nobody_proposes_a_fluid_working_with_a_dry_pack() {
    let mut simulation = a_person();
    give(&mut simulation, "flax", 6);
    simulation.population.agents[0].found_out_how_to("rettedflax");

    let would = (0..60)
        .filter_map(|_| simulation.population.agents[0].what_i_would_work_on())
        .any(|(verb, _)| verb == "soak");

    assert!(!would, "no water, no retting, and no plan to");

    give_a_full_bowl(&mut simulation, 8.0);

    let would = (0..60)
        .filter_map(|_| simulation.population.agents[0].what_i_would_work_on())
        .any(|(verb, _)| verb == "soak");

    assert!(would, "with a full bowl it is a thing he would do");
}

/// Water in a vessel is water an agent is carrying.
#[test]
fn a_vessel_is_how_water_travels() {
    let mut simulation = a_person();

    assert_eq!(
        simulation.population.agents[0].how_much_water_i_carry(),
        0.0,
        "empty-handed, none"
    );

    give_a_full_bowl(&mut simulation, 6.0);

    assert!(
        simulation.population.agents[0].how_much_water_i_carry() > 0.0,
        "with a bowl, some"
    );

    let drawn = simulation.population.agents[0].draw_from_what_i_carry(2.0);
    assert!(drawn > 0.0, "and it can be drawn out again");
    assert!(
        simulation.population.agents[0].how_much_water_i_carry() < 6.0,
        "leaving less than there was"
    );
}
