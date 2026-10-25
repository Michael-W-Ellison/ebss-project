// src/analytics/tests/stamina_tests.rs
//! Stamina is tiredness (#70): work takes it, sleep and rest give it back,
//! and a meal does not. A meal of five used to refill the whole pool, and
//! nobody in this model was ever tired.

use crate::agents::fatigue::SleepQualityFactors;
use crate::agents::{Agent, AgentConfig, AgentState, InventoryItem};
use crate::core::DriveType;

fn a_grown_man() -> Agent {
    let mut agent = Agent::new(AgentConfig::default());
    agent.state.now_this_many_years_old(30);
    agent
}

/// A meal fills the body and leaves stamina where it was.
#[test]
fn a_meal_does_not_give_back_stamina() {
    let mut agent = a_grown_man();
    agent.state.energy = 40.0;
    agent.state.eat(10);
    assert_eq!(agent.state.energy, 40.0);

    agent.inventory.add_item(InventoryItem::new("food".to_string(), 3));
    assert!(agent.eat_food(1));
    assert_eq!(agent.state.energy, 40.0, "not the old flat twenty a mouthful either");
}

/// Hard work tires a man more than idling does.
#[test]
fn work_takes_stamina() {
    let idle = {
        let mut agent = a_grown_man();
        agent.process_survival_turn(1);
        100.0 - agent.state.energy
    };
    let digging = {
        let mut agent = a_grown_man();
        agent.state.effort_this_turn = 20.0;
        agent.process_survival_turn(1);
        100.0 - agent.state.energy
    };
    assert!(idle > 0.0 && idle < 0.5, "a turn of living costs a little: {idle}");
    assert!(
        digging > idle + 2.0,
        "a turn of digging costs a good deal more: {digging} against {idle}"
    );
}

/// A night's sleep gives back a day's ordinary work.
#[test]
fn a_nights_sleep_gives_back_a_days_work() {
    let mut agent = a_grown_man();
    let a_day = 32.0
        * AgentState::WHAT_AN_ORDINARY_TURN_OF_WORK_COSTS
        * AgentState::WHAT_A_UNIT_OF_WORK_TIRES;
    agent.state.energy = 100.0 - a_day;
    let fair = SleepQualityFactors {
        has_shelter: true,
        has_bed: true,
        safety: 1.0,
        health: 1.0,
        hunger: 0.2,
        comfort: 0.5,
    };
    for turn in 0..16 {
        agent.sleep_turn(turn, &fair);
    }
    assert!(
        agent.state.energy >= 99.0,
        "a night under a roof should put back a day of ordinary work: {}",
        agent.state.energy
    );
}

/// A man running out of stamina wants rest before he is empty.
#[test]
fn a_tired_man_wants_rest() {
    let mut agent = a_grown_man();
    agent.state.energy = 30.0;
    agent.process_survival_turn(1);
    let rest = agent.drives.get(DriveType::Rest).map(|drive| drive.value).unwrap_or(0.0);
    assert!(
        rest >= DriveType::Rest.default_threshold() - 0.01,
        "at thirty the Rest drive should be at its threshold: {rest}"
    );

    let mut fresh = a_grown_man();
    fresh.state.energy = 95.0;
    fresh.process_survival_turn(1);
    let rest = fresh.drives.get(DriveType::Rest).map(|drive| drive.value).unwrap_or(0.0);
    assert!(rest < DriveType::Rest.default_threshold(), "a rested man does not: {rest}");
}

/// A body with nothing left rests, whatever it wanted to do, unless it is
/// running for its life or doing something that costs it nothing.
#[test]
fn a_spent_body_rests() {
    use crate::analytics::Simulation;
    use crate::environment::Action;
    use crate::world::{World, WorldConfig};

    let mut population = crate::agents::Population::new();
    population.spawn_agent(AgentConfig::default());
    let mut simulation = Simulation::new(World::new(WorldConfig::default()), population);
    simulation.population.agents[0].state.energy = 0.0;

    assert_eq!(simulation.what_a_spent_body_does(0, Action::Excavate, false), Action::Wait);
    assert_eq!(
        simulation.what_a_spent_body_does(0, Action::Excavate, true),
        Action::Excavate,
        "running for it is let through"
    );
    let supper = Action::Eat { food_type: "generic".to_string() };
    assert_eq!(simulation.what_a_spent_body_does(0, supper.clone(), false), supper);

    simulation.population.agents[0].state.energy = 50.0;
    assert_eq!(simulation.what_a_spent_body_does(0, Action::Excavate, false), Action::Excavate);
}

/// Huddling under cover warms a cold body faster than standing there, so
/// the turn spent on it can end the cold that chose it (#233).
#[test]
fn a_huddle_under_cover_warms_a_cold_body() {
    use crate::analytics::Simulation;
    use crate::environment::Action;
    use crate::world::{Position, TerrainType, World, WorldConfig};

    let cold_body = |huddle: bool| {
        let mut world = World::new(WorldConfig::default());
        world.animals.get_all_mut().clear();
        if let Some(tile) = world.grid.get_tile_mut(&Position::new(25, 25)) {
            tile.terrain.terrain_type = TerrainType::Forest;
        }
        let mut population = crate::agents::Population::new();
        population.spawn_agent(AgentConfig::default());
        let mut simulation = Simulation::new(world, population);
        // Midwinter, in a blizzard: cold enough that a body under the trees
        // loses heat faster than it makes it, which is when a huddle tells.
        simulation.world.climate.calendar.day_of_year =
            crate::environment::seasons::Season::Winter.first_day() + 30;
        simulation.world.climate.weather =
            crate::environment::Weather::new(crate::environment::WeatherType::Blizzard);
        let agent = &mut simulation.population.agents[0];
        agent.state.now_this_many_years_old(30);
        agent.state.position = (25, 25, 0);
        agent.body_temperature.current = 34.0;
        if huddle {
            let result = simulation.execute_action(&Action::SeekShelter, 0);
            assert!(result.success, "{:?}", result.message);
            assert!(simulation.population.agents[0].state.huddled);
        }
        simulation.update_agent_exposure();
        assert!(!simulation.population.agents[0].state.huddled, "the huddle is spent");
        simulation.population.agents[0].body_temperature.current
    };

    let stood = cold_body(false);
    let huddled = cold_body(true);
    assert!(huddled > stood, "huddled {huddled} against standing {stood}");
}
