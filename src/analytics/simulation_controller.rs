// src/analytics/simulation_controller.rs
//! Simulation control for pausing, stepping, and inspecting the simulation.

use crate::world::World;
use crate::agents::Population;

/// State of the simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationState {
    /// Simulation is running
    Running,
    /// Simulation is paused
    Paused,
    /// Simulation is stepping one turn at a time
    Stepping,
}

/// Controls simulation execution and provides inspection capabilities
pub struct SimulationController {
    pub world: World,
    pub population: Population,
    pub state: SimulationState,
    pub current_turn: u64,
    pub turn_rate: f32, // Turns per second when running
}

impl SimulationController {
    pub fn new(world: World, population: Population) -> Self {
        Self {
            world,
            population,
            state: SimulationState::Paused,
            current_turn: 0,
            turn_rate: 10.0,
        }
    }

    /// Pause the simulation
    pub fn pause(&mut self) {
        self.state = SimulationState::Paused;
    }

    /// Resume the simulation
    pub fn play(&mut self) {
        self.state = SimulationState::Running;
    }

    /// Toggle between paused and running
    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            SimulationState::Running => SimulationState::Paused,
            SimulationState::Paused | SimulationState::Stepping => SimulationState::Running,
        };
    }

    /// Step forward one turn
    pub fn step(&mut self) {
        self.state = SimulationState::Stepping;
        self.turn_once();
        self.state = SimulationState::Paused;
    }

    /// Execute one simulation turn
    pub fn turn_once(&mut self) {
        // Update all agent drives
        for agent in &mut self.population.agents {
            agent.drives.take_a_turn();
        }

        self.current_turn += 1;
    }

    /// Update simulation based on delta time
    pub fn update(&mut self, dt: f32) {
        if self.state == SimulationState::Running {
            // Calculate how many turns to run based on turn rate
            let turns_to_run = (dt * self.turn_rate) as u32;
            for _ in 0..turns_to_run.max(1) {
                self.turn_once();
            }
        }
    }

    /// Set the simulation speed (turns per second)
    pub fn set_turn_rate(&mut self, rate: f32) {
        self.turn_rate = rate.max(0.1).min(1000.0);
    }

    /// Get reference to population
    pub fn get_population(&self) -> &Population {
        &self.population
    }




    /// Check if simulation is running
    pub fn is_running(&self) -> bool {
        self.state == SimulationState::Running
    }

    /// Check if simulation is paused
    pub fn is_paused(&self) -> bool {
        self.state == SimulationState::Paused || self.state == SimulationState::Stepping
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldConfig;
    use crate::agents::AgentConfig;

    #[test]
    fn test_controller_creation() {
        let world = World::new(WorldConfig::default());
        let population = Population::new();
        let controller = SimulationController::new(world, population);

        assert_eq!(controller.state, SimulationState::Paused);
        assert_eq!(controller.current_turn, 0);
    }

    #[test]
    fn test_pause_play() {
        let world = World::new(WorldConfig::default());
        let population = Population::new();
        let mut controller = SimulationController::new(world, population);

        controller.play();
        assert_eq!(controller.state, SimulationState::Running);

        controller.pause();
        assert_eq!(controller.state, SimulationState::Paused);
    }

    #[test]
    fn test_toggle_pause() {
        let world = World::new(WorldConfig::default());
        let population = Population::new();
        let mut controller = SimulationController::new(world, population);

        assert_eq!(controller.state, SimulationState::Paused);

        controller.toggle_pause();
        assert_eq!(controller.state, SimulationState::Running);

        controller.toggle_pause();
        assert_eq!(controller.state, SimulationState::Paused);
    }

    #[test]
    fn test_step() {
        let world = World::new(WorldConfig::default());
        let mut population = Population::new();
        population.spawn_agent(AgentConfig::default());

        let mut controller = SimulationController::new(world, population);
        assert_eq!(controller.current_turn, 0);

        controller.step();
        assert_eq!(controller.current_turn, 1);
        assert_eq!(controller.state, SimulationState::Paused);
    }

    #[test]
    fn test_turn_rate() {
        let world = World::new(WorldConfig::default());
        let population = Population::new();
        let mut controller = SimulationController::new(world, population);

        controller.set_turn_rate(20.0);
        assert_eq!(controller.turn_rate, 20.0);

        // Test clamping
        controller.set_turn_rate(10000.0);
        assert_eq!(controller.turn_rate, 1000.0);

        controller.set_turn_rate(0.01);
        assert_eq!(controller.turn_rate, 0.1);
    }
}
