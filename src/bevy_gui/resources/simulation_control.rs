// src/bevy_gui/resources/simulation_control.rs
//! Simulation control state resource.

use bevy::prelude::*;

/// Current simulation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimState {
    Running,
    #[default]
    Paused,
    Stepping,
}

/// Controls simulation playback
#[derive(Resource)]
pub struct SimulationControl {
    pub state: SimState,
    pub speed: f32,
}

impl Default for SimulationControl {
    fn default() -> Self {
        Self {
            state: SimState::Paused,
            speed: 1.0,
        }
    }
}

impl SimulationControl {
    pub fn is_running(&self) -> bool {
        self.state == SimState::Running
    }

    pub fn is_paused(&self) -> bool {
        self.state == SimState::Paused
    }

    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            SimState::Running => SimState::Paused,
            SimState::Paused | SimState::Stepping => SimState::Running,
        };
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.1, 10.0);
    }
}
