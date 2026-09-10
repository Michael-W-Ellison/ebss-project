// src/analytics/tests/working_together_tests.rs
//! **Nobody in this model has ever joined a job somebody else started.**
//!
//! The absence showed from three sides and they were all the same absence.
//! `workers` has sat on `BuildingState::UnderConstruction` since buildings
//! were written with nothing to write it. `SpatialMemoryType::Shelter` has sat
//! in the memory with no writer either. And the only lookup for a half-built
//! roof was "is there one within two paces of where I am standing" - so two
//! men three paces apart dug two burrows and finished neither.
//!
//! There is no settlement object here and nothing agrees to anything. What
//! coordinates them is the half-built roof: one man starts it, another sees
//! it, and both go to the one that is furthest along. See
//! `the_job_this_camp_has_going`.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::Action;
use crate::world::{Building, BuildingType, Position, World, WorldConfig};

fn a_camp_of_two() -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    world.buildings.clear();

    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());

    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (25, 25, 0);
    simulation.population.agents[1].state.position = (25, 25, 0);
    simulation
}

/// A site somebody has put `how_far` of the way up, at `at`.
fn a_roof_going_up(simulation: &mut Simulation, at: Position, how_far: u32) {
    let mut roof = Building::new_under_construction(BuildingType::Burrow, at);
    roof.add_construction_progress(how_far, 0);
    simulation.world.add_building(roof);
}

fn he_knows_of_the_roof(simulation: &mut Simulation, who: usize, at: Position) {
    simulation.population.agents[who]
        .memory
        .remember_location(crate::core::memory::SpatialMemoryType::Shelter, (at.x, at.y, 0));
}

/// The slot that had nothing to write it.
#[test]
fn a_job_can_say_how_many_hands_are_on_it() {
    let simulation = a_camp_of_two();
    let first = simulation.population.agents[0].id;
    let second = simulation.population.agents[1].id;

    let mut roof = Building::new_under_construction(BuildingType::Burrow, Position::new(25, 25));
    assert_eq!(roof.how_many_hands(), 0, "a fresh site has nobody on it");

    roof.a_hand_on_it(first);
    roof.a_hand_on_it(first);
    assert_eq!(roof.how_many_hands(), 1, "the same man twice is one man");

    roof.a_hand_on_it(second);
    assert_eq!(roof.how_many_hands(), 2);

    // And a finished job is not a job.
    roof.add_construction_progress(BuildingType::Burrow.construction_time(), 0);
    assert!(roof.is_completed());
    assert_eq!(roof.how_many_hands(), 0);
}

/// **The one furthest along, not the one nearest.**
///
/// This is the whole of the coordination. A camp that always takes the nearest
/// job finishes nothing; a camp that always takes the one nearest done
/// finishes one roof, and then the next.
#[test]
fn the_job_nearest_finished_wins_over_the_job_nearest_to_hand() {
    let mut simulation = a_camp_of_two();

    let barely_begun = Position::new(26, 25);
    let nearly_up = Position::new(33, 25);

    a_roof_going_up(&mut simulation, barely_begun, 5);
    a_roof_going_up(&mut simulation, nearly_up, BuildingType::Burrow.construction_time() - 5);

    he_knows_of_the_roof(&mut simulation, 0, barely_begun);
    he_knows_of_the_roof(&mut simulation, 0, nearly_up);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.the_job_this_camp_has_going(agent, agent.state.position),
        Some(nearly_up),
        "he went to the hole beside him instead of the one that wanted an \
         afternoon's work to be a roof"
    );
}

/// A finished roof is not a job, however well he remembers it.
#[test]
fn a_finished_roof_is_not_a_job() {
    let mut simulation = a_camp_of_two();

    let up = Position::new(28, 25);
    simulation
        .world
        .add_building(Building::new(BuildingType::Burrow, up));
    he_knows_of_the_roof(&mut simulation, 0, up);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.the_job_this_camp_has_going(agent, agent.state.position),
        None
    );
}

/// And a job the far side of the map is not a job, it is a morning gone.
///
/// The walk was measured: sending somebody back to a roof unconditionally cost
/// 98,079 person-days against 99,396 over 32 seeded worlds. A camp is what you
/// can see the smoke of.
#[test]
fn a_job_across_the_map_is_not_this_camps_job() {
    let mut simulation = a_camp_of_two();

    let far_off = Position::new(25 + Simulation::HOW_FAR_TO_LEND_A_HAND + 5, 25);
    a_roof_going_up(&mut simulation, far_off, 5);
    he_knows_of_the_roof(&mut simulation, 0, far_off);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.the_job_this_camp_has_going(agent, agent.state.position),
        None
    );
}

/// He will not join a job on a roof he has no business touching.
///
/// In a live world this is nearly always yes - everything the decision layer
/// can raise comes out as the camp's, because `is_residential` names only the
/// grand houses it cannot build. It is here so that it stops being yes the day
/// somebody builds a house of their own. See `world::belonging`.
#[test]
fn he_does_not_lend_a_hand_on_a_strangers_house() {
    let mut simulation = a_camp_of_two();
    let his = simulation.population.agents[1].id;

    let at = Position::new(28, 25);
    let mut roof = Building::new_under_construction(BuildingType::SmallHouse, at);
    roof.now_belongs_to(crate::world::Belongs::To(his));
    roof.add_construction_progress(5, 0);
    simulation.world.add_building(roof);

    he_knows_of_the_roof(&mut simulation, 0, at);

    let agent = &simulation.population.agents[0];
    assert_eq!(
        simulation.the_job_this_camp_has_going(agent, agent.state.position),
        None
    );
}

/// **Two men, one roof.**
///
/// The end-to-end statement, and the one the module is named after: a site the
/// second man never touched and cannot see from where he stands is a job he
/// walks to, because he saw it earlier and remembers it.
#[test]
fn a_second_man_walks_to_a_roof_the_first_one_started() {
    let mut simulation = a_camp_of_two();

    let going_up = Position::new(31, 25);
    a_roof_going_up(&mut simulation, going_up, 20);
    he_knows_of_the_roof(&mut simulation, 1, going_up);

    // He has supper about him, so the walk is not being weighed against an
    // empty pack - that gate is measured and separate.
    simulation.population.agents[1].state.what_the_larder_says = None;

    let agent = &simulation.population.agents[1];
    let doing = simulation.digging_in(agent, agent.state.position);

    assert_eq!(
        doing,
        Some(Action::Move {
            target: (going_up.x, going_up.y, 0)
        }),
        "he started a hole of his own beside a job that was already going"
    );
}

/// And standing on it, he works on it rather than walking to it.
#[test]
fn standing_on_the_job_he_works_on_it() {
    let mut simulation = a_camp_of_two();

    let going_up = Position::new(31, 25);
    a_roof_going_up(&mut simulation, going_up, 20);
    he_knows_of_the_roof(&mut simulation, 1, going_up);
    simulation.population.agents[1].state.position = (going_up.x, going_up.y, 0);
    simulation.population.agents[1].state.what_the_larder_says = None;

    let agent = &simulation.population.agents[1];
    match simulation.digging_in(agent, agent.state.position) {
        Some(Action::Build { position, .. }) => {
            assert_eq!(position, (going_up.x, going_up.y, 0));
        }
        other => panic!("standing on a half-dug burrow he chose {other:?}"),
    }
}

/// Seeing a roof going up is how he learns there is a job on.
///
/// The writer `SpatialMemoryType::Shelter` never had. Without it a man could
/// only ever go back to a roof he had touched himself, and coordination
/// between two people was impossible however good the rest of it was.
#[test]
fn a_roof_going_up_is_something_a_man_notices() {
    let mut simulation = a_camp_of_two();

    let going_up = Position::new(26, 25);
    a_roof_going_up(&mut simulation, going_up, 5);

    let mut world = std::mem::replace(&mut simulation.world, World::new(WorldConfig::default()));
    simulation.population.process_exploration_with_world(&mut world);
    simulation.world = world;

    let remembered = simulation.population.agents[1]
        .memory
        .recall_locations(crate::core::memory::SpatialMemoryType::Shelter);

    assert!(
        remembered
            .iter()
            .any(|place| place.position == (going_up.x, going_up.y, 0)),
        "he stood a pace from a half-built roof and never noticed it"
    );
}
