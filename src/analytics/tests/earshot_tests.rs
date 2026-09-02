//! A voice has a reach.
//!
//! `find_nearest_social_target` returned the nearest person *on the map*, and
//! neither `socialising` nor `sharing_information` looked at where that person
//! was. So a settlement's whole social life was conducted at arbitrary range:
//! two men twelve tiles apart, each alone in a different wood, greeted one
//! another, exchanged news and gave one another presents.
//!
//! Found while tracing why agent-to-agent retaliation never fires - see
//! ISSUES_FOUND.md #102.

use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::world::{World, WorldConfig};

/// Two people, as far apart as the test asks for.
fn two_people(apart: i32) -> Simulation {
    let mut world = World::new(WorldConfig::default());
    world.animals.get_all_mut().clear();
    let mut population = Population::new();
    population.spawn_agent(AgentConfig::default());
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(world, population);
    simulation.population.agents[0].state.position = (30, 30, 0);
    simulation.population.agents[1].state.position = (30 + apart, 30, 0);
    simulation
}

/// Somebody standing beside you can be spoken to.
#[test]
fn a_man_beside_you_can_be_spoken_to() {
    let mut simulation = two_people(1);
    let them = simulation.population.agents[1].id;
    let mut rng = crate::core::dice::roll();

    let result = simulation.socialising(&them, 0, &mut rng, 10);
    assert!(
        result.success || result.message.as_deref() != Some("Too far off to say anything"),
        "a man one tile away is within earshot"
    );
}

/// Somebody across the valley cannot.
#[test]
fn a_man_across_the_valley_cannot() {
    let apart = Simulation::WITHIN_TALKING_DISTANCE + 1;
    let mut simulation = two_people(apart);
    let them = simulation.population.agents[1].id;
    let mut rng = crate::core::dice::roll();

    let result = simulation.socialising(&them, 0, &mut rng, 10);
    assert!(!result.success, "{apart} tiles is out of earshot");
    assert_eq!(
        result.message.as_deref(),
        Some("Too far off to say anything")
    );
}

/// And news does not carry any further than a greeting does.
#[test]
fn news_does_not_carry_further_than_a_greeting() {
    let mut simulation = two_people(Simulation::WITHIN_TALKING_DISTANCE + 1);
    let them = simulation.population.agents[1].id;
    let mut rng = crate::core::dice::roll();

    let result = simulation.sharing_information(&them, 0, &mut rng);
    assert!(!result.success);
    assert_eq!(
        result.message.as_deref(),
        Some("Too far off to say anything")
    );
}

/// Nobody out of earshot is even chosen to be spoken to.
///
/// The verb refusing is the backstop; the point is that the choosing never
/// names somebody it cannot reach, because a turn spent shouting at an empty
/// wood is a turn wasted.
#[test]
fn nobody_out_of_earshot_is_chosen() {
    let me = crate::core::dice::name();
    let them = crate::core::dice::name();
    let here = (30, 30, 0);
    let far = (30 + Simulation::WITHIN_TALKING_DISTANCE + 1, 30, 0);

    assert_eq!(
        Simulation::find_nearest_social_target(me, here, &[(me, here), (them, far)]),
        None,
        "the only other person is out of earshot, so there is nobody to talk to"
    );

    let near = (30 + Simulation::WITHIN_TALKING_DISTANCE, 30, 0);
    assert_eq!(
        Simulation::find_nearest_social_target(me, here, &[(me, here), (them, near)]),
        Some(them),
        "and at the edge of earshot there is"
    );
}
