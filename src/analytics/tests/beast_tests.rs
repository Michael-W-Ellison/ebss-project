// src/analytics/tests/beast_tests.rs
//! Tests for what the beasts make of us.
//!
//! An animal has two drives worth the name — eat, and do not be eaten — and
//! until now it had no opinion about people at all. `AnimalState::Fleeing`
//! and `AnimalState::Attacking` have been in the model since the model had
//! animals and nothing had ever set either of them, so a deer stood placidly
//! in a field while somebody walked up to it with a spear.
//!
//! Temper decides how kindly the odds get read, and a Passive thing never
//! stands its ground however the arithmetic comes out: a rabbit that fights a
//! wolf is not a rabbit.

use crate::agents::{AgentConfig, InventoryItem, Population};
use crate::analytics::Simulation;
use crate::environment::fauna::{AnimalBehavior, AnimalState};
use crate::world::{World, WorldConfig};

fn an_empty_country() -> World {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world
}

/// One person at (30, 30) and whatever the test puts near them.
fn one_person(world: World) -> Simulation {
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[0].state.health = 100.0;
    simulation.population.agents[0]
        .inventory
        .get_all_items_mut()
        .clear();
    simulation.population.agents[0].inventory.recalculate_weight();
    simulation
}

fn arm_them(simulation: &mut Simulation) {
    let mut spear = InventoryItem::new_with_weight("spear".to_string(), 1, 1.0);
    spear.current_durability = Some(25.0);
    spear.max_durability = Some(25.0);
    let _ = simulation.population.agents[0].inventory.add_item(spear);
}

fn how_it_feels(simulation: &mut Simulation) -> AnimalState {
    simulation.what_the_beasts_make_of_us();
    simulation.world.animals.get_all()[0].state.clone()
}

// --------------------------------------------------------------------------
// Running
// --------------------------------------------------------------------------

/// A deer with a man beside it goes.
#[test]
fn a_deer_runs_from_a_man() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (32, 30))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    assert!(
        matches!(how_it_feels(&mut simulation), AnimalState::Fleeing { .. }),
        "it should be away"
    );
}

/// And it runs away from him rather than past him.
#[test]
fn what_runs_puts_ground_between_itself_and_the_thing() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (32, 30))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    simulation.what_the_beasts_make_of_us();
    let before = simulation.world.animals.get_all()[0].position;
    simulation.the_beasts_act_on_it();
    let after = simulation.world.animals.get_all()[0].position;

    assert!(
        after.0 > before.0,
        "the man is to the west, so the deer goes east: {before:?} to {after:?}"
    );
}

/// A rabbit never stands its ground, whatever the odds say. That is what
/// Passive means, and a rabbit that fights a wolf is not a rabbit.
#[test]
fn a_rabbit_never_turns_and_faces_anything() {
    assert_eq!(
        AnimalBehavior::Passive.how_readily_it_stands_its_ground(),
        0.0,
        "there is no arithmetic that makes a rabbit brave"
    );

    let mut world = an_empty_country();
    world
        .spawn_animal("rabbit".to_string(), (31, 30))
        .expect("a rabbit should spawn");
    let mut simulation = one_person(world);

    assert!(
        matches!(how_it_feels(&mut simulation), AnimalState::Fleeing { .. }),
        "it runs"
    );
}

// --------------------------------------------------------------------------
// Standing its ground
// --------------------------------------------------------------------------

/// A bear does not run from one man.
#[test]
fn a_bear_stands_its_ground() {
    let mut world = an_empty_country();
    world
        .spawn_animal("bear".to_string(), (31, 30))
        .expect("a bear should spawn");
    let mut simulation = one_person(world);

    assert!(
        matches!(how_it_feels(&mut simulation), AnimalState::Attacking { .. }),
        "a bear is not afraid of a man"
    );
}

/// And what it turns on is the man it saw.
#[test]
fn what_stands_its_ground_names_what_it_is_facing() {
    let mut world = an_empty_country();
    world
        .spawn_animal("bear".to_string(), (31, 30))
        .expect("a bear should spawn");
    let mut simulation = one_person(world);
    let who = simulation.population.agents[0].id;

    let AnimalState::Attacking { target_id } = how_it_feels(&mut simulation) else {
        panic!("a bear stands its ground");
    };

    assert_eq!(target_id, who, "it is facing the man who walked up to it");
}

/// A thing in the hand changes the arithmetic. The same wolf that would take
/// on an unarmed man goes when he has a spear.
#[test]
fn a_spear_changes_what_a_wolf_thinks_of_you() {
    let with_a_spear = |armed: bool| {
        let mut world = an_empty_country();
        world
            .spawn_animal("wolf".to_string(), (31, 30))
            .expect("a wolf should spawn");
        let mut simulation = one_person(world);
        if armed {
            arm_them(&mut simulation);
        }
        how_it_feels(&mut simulation)
    };

    assert!(
        matches!(with_a_spear(false), AnimalState::Attacking { .. }),
        "a wolf will take on a man with nothing in his hands"
    );
    assert!(
        matches!(with_a_spear(true), AnimalState::Fleeing { .. }),
        "and thinks better of it when he has a spear"
    );
}

/// A wounded thing is worth less in a fight, and knows it.
#[test]
fn a_wounded_beast_reads_the_odds_differently() {
    let sound = Simulation::what_a_beast_is_worth_in_a_fight(100.0, 100.0, 20.0);
    let hurt = Simulation::what_a_beast_is_worth_in_a_fight(10.0, 100.0, 20.0);

    assert!(
        hurt < sound,
        "a bear with one leg is not a bear: {hurt} against {sound}"
    );
}

// --------------------------------------------------------------------------
// Each other
// --------------------------------------------------------------------------

/// A deer runs from a wolf, and not only from us.
#[test]
fn a_deer_runs_from_a_wolf_too() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (10, 10))
        .expect("a deer should spawn");
    world
        .spawn_animal("wolf".to_string(), (12, 10))
        .expect("a wolf should spawn");

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    // The man is right across the map and is nobody's business
    simulation.population.agents[0].state.position = (60, 60, 0);

    simulation.what_the_beasts_make_of_us();

    let deer = simulation
        .world
        .animals
        .get_all()
        .iter()
        .find(|animal| animal.species_id == "deer")
        .expect("the deer is there");

    assert!(
        matches!(deer.state, AnimalState::Fleeing { .. }),
        "there is a wolf two paces off: {:?}",
        deer.state
    );
}

/// And nothing minds a thing right across the country.
#[test]
fn nothing_minds_a_man_across_the_map() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (5, 5))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    let before = simulation.world.animals.get_all()[0].state.clone();
    simulation.what_the_beasts_make_of_us();
    let after = simulation.world.animals.get_all()[0].state.clone();

    assert_eq!(
        before, after,
        "twenty-five paces is somebody else's problem"
    );
}

/// Running costs something. A deer that has been bolting all day is a tired
/// deer, which is what makes a hunt possible at all.
#[test]
fn bolting_costs_a_beast_its_wind() {
    let mut world = an_empty_country();
    world
        .spawn_animal("deer".to_string(), (32, 30))
        .expect("a deer should spawn");
    let mut simulation = one_person(world);

    let before = simulation.world.animals.get_all()[0].stamina;
    simulation.what_the_beasts_make_of_us();
    simulation.the_beasts_act_on_it();
    let after = simulation.world.animals.get_all()[0].stamina;

    assert!(after < before, "{after} against {before}");
}
