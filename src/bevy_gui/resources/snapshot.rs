// src/bevy_gui/resources/snapshot.rs
//! Simulation snapshot resource for GUI rendering.

use bevy::prelude::*;

use crate::gui::state::{
    SimulationSnapshot, WorldSnapshot, PopulationSnapshot,
    SelectedAgentData, SelectedBuildingData, SelectedResourceData,
    TechTreeSnapshot,
};
use crate::gui::events::SimulationEvent;

/// Current simulation snapshot for rendering
#[derive(Resource, Default)]
pub struct CurrentSnapshot {
    pub snapshot: Option<SimulationSnapshot>,
    pub tick: u32,
}

impl CurrentSnapshot {
    pub fn update(&mut self, snapshot: SimulationSnapshot) {
        self.tick = snapshot.tick;
        self.snapshot = Some(snapshot);
    }

    pub fn world(&self) -> Option<&WorldSnapshot> {
        self.snapshot.as_ref().map(|s| &s.world)
    }

    pub fn population(&self) -> Option<&PopulationSnapshot> {
        self.snapshot.as_ref().map(|s| &s.population)
    }

    pub fn events(&self) -> &[SimulationEvent] {
        self.snapshot.as_ref()
            .map(|s| s.events.as_slice())
            .unwrap_or(&[])
    }
}

/// Detailed entity data fetched on demand
#[derive(Resource, Default)]
pub struct SelectedEntityData {
    pub agent: Option<SelectedAgentData>,
    pub building: Option<SelectedBuildingData>,
    pub resource: Option<SelectedResourceData>,
}

impl SelectedEntityData {
    pub fn clear(&mut self) {
        self.agent = None;
        self.building = None;
        self.resource = None;
    }

    pub fn set_agent(&mut self, data: SelectedAgentData) {
        self.clear();
        self.agent = Some(data);
    }

    pub fn set_building(&mut self, data: SelectedBuildingData) {
        self.clear();
        self.building = Some(data);
    }

    pub fn set_resource(&mut self, data: SelectedResourceData) {
        self.clear();
        self.resource = Some(data);
    }
}

/// Tech tree data
#[derive(Resource, Default)]
pub struct TechTreeData {
    pub snapshot: Option<TechTreeSnapshot>,
    pub selected_tech: Option<String>,
}
