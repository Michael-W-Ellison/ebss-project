// src/analytics/tests/shelter_and_exposure_tests.rs
//! Regression tests for body temperature, exposure and sheltering.
//!
//! These cover the failure that left every agent permanently hypothermic and
//! permanently seeking shelter:
//! - a body holds its temperature in ordinary weather instead of settling at
//!   ambient, and insulation extends how far that holds
//! - shelter reaches the body, so sheltering resolves what sent the agent there
//! - exposure damage recovers once conditions are safe, and stays bounded
//! - agents do not chase shelter that does not exist

use crate::agents::temperature::BodyTemperature;
use crate::agents::{AgentConfig, Population};
use crate::analytics::Simulation;
use crate::environment::exposure::ExposureStatus;
use crate::world::{World, WorldConfig};

fn settled_temperature(ambient: f32, cold_insulation: f32) -> f32 {
    let mut body = BodyTemperature::new();
    for _ in 0..500 {
        body.update(ambient, cold_insulation, 0.0);
    }
    body.current
}

/// A body in ordinary weather holds its temperature rather than drifting to
/// ambient. Metabolic regulation used to be a weak pull toward the ideal that
/// environmental transfer simply overwhelmed, so a 10C day left agents sitting
/// at roughly 15C core and hypothermic for the rest of the run.
#[test]
fn body_holds_temperature_in_temperate_weather() {
    for ambient in [30.0, 20.0, 10.0] {
        let settled = settled_temperature(ambient, 0.0);

        assert!(
            settled > 35.0,
            "an unclothed agent in {ambient}C air should not be hypothermic, settled at {settled:.1}C"
        );
        assert!(
            settled < 39.0,
            "an agent in {ambient}C air should not be overheating, settled at {settled:.1}C"
        );
    }
}

/// Real cold still bites - the model must not simply pin everyone at normal.
#[test]
fn severe_cold_still_causes_hypothermia() {
    let settled = settled_temperature(-20.0, 0.0);

    assert!(
        settled < 35.0,
        "an unclothed agent at -20C should go hypothermic, settled at {settled:.1}C"
    );
}

/// Insulation is what buys survival in the cold.
#[test]
fn insulation_extends_the_survivable_range() {
    let bare = settled_temperature(-20.0, 0.0);
    let clothed = settled_temperature(-20.0, 0.8);

    assert!(
        clothed > bare,
        "insulation should keep an agent warmer: bare {bare:.1}C vs clothed {clothed:.1}C"
    );
    assert!(
        clothed > 35.0,
        "well insulated agents should hold their temperature at -20C, got {clothed:.1}C"
    );
}

/// Shelter has to reach the body, otherwise seeking it never resolves anything.
#[test]
fn shelter_moderates_the_temperature_the_body_feels() {
    use crate::agents::Climate;

    let cold = Climate {
        temperature: -5.0,
        humidity: 0.5,
        wind_speed: 8.0,
    };

    let exposed = cold.effective_temperature();
    let sheltered = cold.sheltered_effective_temperature();

    assert!(
        sheltered > exposed,
        "shelter should be milder than the open: {sheltered:.1}C vs {exposed:.1}C"
    );

    let mut indoors = crate::agents::Agent::new(AgentConfig::default());
    let mut outdoors = crate::agents::Agent::new(AgentConfig::default());

    for _ in 0..200 {
        indoors.update_temperature_with_shelter(&cold, true);
        outdoors.update_temperature_with_shelter(&cold, false);
    }

    assert!(
        indoors.body_temperature.current > outdoors.body_temperature.current,
        "an agent under cover should end up warmer: {:.1}C indoors vs {:.1}C outdoors",
        indoors.body_temperature.current,
        outdoors.body_temperature.current
    );
}

/// Exposure damage recovers once the agent is safe, in shelter or out of it.
///
/// Recovery used to happen only inside the SeekShelter action and only under
/// cover, so an agent that had warmed up still read as exposed indefinitely.
#[test]
fn exposure_damage_recovers_once_conditions_are_safe() {
    use crate::environment::{Weather, WeatherType};

    let calm = Weather::new(WeatherType::Clear);
    let body = BodyTemperature::new(); // comfortable, so nothing is active

    for has_shelter in [true, false] {
        let mut status = ExposureStatus::new();
        status.exposure_damage = 3.0;

        for _ in 0..100 {
            status.update(&body, 20.0, &calm, has_shelter, true, 12.0);
        }

        assert!(
            status.exposure_damage < 3.0,
            "exposure should recover when nothing is harming the agent \
             (shelter: {has_shelter}), stayed at {}",
            status.exposure_damage
        );
    }
}

/// Accumulated damage stays bounded rather than climbing forever.
#[test]
fn exposure_damage_is_bounded() {
    use crate::environment::{Weather, WeatherType};

    let storm = Weather::new(WeatherType::Snow);
    let mut freezing = BodyTemperature::new();
    freezing.current = 28.0; // deeply hypothermic, so damage keeps coming

    let mut status = ExposureStatus::new();

    for _ in 0..5000 {
        status.update(&freezing, -25.0, &storm, false, false, 3.0);
    }

    assert!(
        status.exposure_damage <= ExposureStatus::MAX_EXPOSURE_DAMAGE,
        "exposure damage should be capped, reached {}",
        status.exposure_damage
    );
}

/// Over a long run agents must not end up permanently frozen and permanently
/// sheltering: body temperatures stay near normal and exposure does not pin.
#[test]
fn agents_do_not_stay_frozen_over_a_long_run() {
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();
    for _ in 0..8 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    for _ in 0..3000 {
        simulation.tick();
    }

    let agents = &simulation.population.agents;
    assert!(!agents.is_empty(), "population should not have died out");

    let average_temperature: f32 =
        agents.iter().map(|a| a.body_temperature.current).sum::<f32>() / agents.len() as f32;

    assert!(
        average_temperature > 32.0,
        "agents should not be running at ambient temperature, averaged {average_temperature:.1}C"
    );

    let critical = agents
        .iter()
        .filter(|a| a.exposure_status.is_critical())
        .count();

    assert_eq!(
        critical, 0,
        "no agent should still be in critical exposure after a long run"
    );
}
