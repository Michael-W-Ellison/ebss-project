// src/bevy_gui/systems/bridge.rs
//! Simulation bridge system for thread communication.

use bevy::prelude::*;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::gui::state::{
    SimulationSnapshot, SimulationCommand as GuiCommand,
    SelectedAgentData, SelectedBuildingData, SelectedResourceData,
    TechTreeSnapshot, RelationshipGraphSnapshot,
    EntitySelection as GuiEntitySelection,
};
use crate::world::Position;
use crate::bevy_gui::resources::*;
use crate::bevy_gui::events::SimulationCommand;

/// Handles for communication with the simulation thread.
/// Uses Arc<Mutex<>> wrappers to make channels thread-safe for Bevy.
#[derive(Resource)]
pub struct SimulationBridge {
    pub command_tx: Arc<Mutex<Sender<GuiCommand>>>,
    pub snapshot_rx: Arc<Mutex<Receiver<SimulationSnapshot>>>,
    pub agent_data_request: Arc<Mutex<Option<uuid::Uuid>>>,
    pub agent_data_response: Arc<Mutex<Option<SelectedAgentData>>>,
    pub building_data_request: Arc<Mutex<Option<Position>>>,
    pub building_data_response: Arc<Mutex<Option<SelectedBuildingData>>>,
    pub resource_data_request: Arc<Mutex<Option<Position>>>,
    pub resource_data_response: Arc<Mutex<Option<SelectedResourceData>>>,
    pub tech_tree_request: Arc<Mutex<bool>>,
    pub tech_tree_response: Arc<Mutex<Option<TechTreeSnapshot>>>,
    pub relationship_graph_request: Arc<Mutex<bool>>,
    pub relationship_graph_response: Arc<Mutex<Option<RelationshipGraphSnapshot>>>,
}

/// System to receive snapshots from the simulation thread
pub fn receive_snapshots_system(
    bridge: Res<SimulationBridge>,
    mut snapshot: ResMut<CurrentSnapshot>,
    mut sim_control: ResMut<SimulationControl>,
) {
    if let Ok(rx) = bridge.snapshot_rx.try_lock() {
        while let Ok(new_snapshot) = rx.try_recv() {
            sim_control.state = match new_snapshot.state {
                crate::gui::state::SimState::Running => SimState::Running,
                crate::gui::state::SimState::Paused => SimState::Paused,
                crate::gui::state::SimState::Stepping => SimState::Stepping,
            };
            sim_control.speed = new_snapshot.speed;
            snapshot.update(new_snapshot);
        }
    }
}

/// System to send commands to the simulation thread
pub fn send_commands_system(
    bridge: Res<SimulationBridge>,
    mut commands: EventReader<SimulationCommand>,
) {
    let tx = match bridge.command_tx.try_lock() {
        Ok(tx) => tx,
        Err(_) => return,
    };

    for cmd in commands.read() {
        let gui_cmd = match cmd {
            SimulationCommand::Play => GuiCommand::Play,
            SimulationCommand::Pause => GuiCommand::Pause,
            SimulationCommand::Step => GuiCommand::Step,
            SimulationCommand::SetSpeed(speed) => GuiCommand::SetSpeed(*speed),
            SimulationCommand::SelectEntity(sel) => {
                let gui_sel = match sel {
                    EntitySelection::None => GuiEntitySelection::None,
                    EntitySelection::Agent(id) => GuiEntitySelection::Agent(*id),
                    EntitySelection::Building(pos) => GuiEntitySelection::Building(*pos),
                    EntitySelection::Resource(pos) => GuiEntitySelection::Resource(*pos),
                    EntitySelection::Terrain(pos) => GuiEntitySelection::Terrain(*pos),
                };
                GuiCommand::SelectEntity(gui_sel)
            }
            SimulationCommand::DeselectAll => GuiCommand::DeselectAll,
            SimulationCommand::SaveGame(_path) => continue,
            SimulationCommand::LoadGame(_path) => continue,
        };
        let _ = tx.send(gui_cmd);
    }
}

/// System to request and receive detailed entity data
pub fn entity_data_system(
    bridge: Res<SimulationBridge>,
    selection: Res<Selection>,
    mut entity_data: ResMut<SelectedEntityData>,
    panels: Res<PanelVisibility>,
) {
    if !panels.inspector {
        return;
    }

    // Request data for selected entity
    match &selection.current {
        EntitySelection::Agent(id) => {
            if let Ok(mut request) = bridge.agent_data_request.try_lock() {
                *request = Some(*id);
            }
        }
        EntitySelection::Building(pos) => {
            if let Ok(mut request) = bridge.building_data_request.try_lock() {
                *request = Some(*pos);
            }
        }
        EntitySelection::Resource(pos) => {
            if let Ok(mut request) = bridge.resource_data_request.try_lock() {
                *request = Some(*pos);
            }
        }
        _ => {}
    }

    // Check for agent response
    if let Ok(mut response) = bridge.agent_data_response.try_lock() {
        if let Some(data) = Option::<SelectedAgentData>::take(&mut response) {
            entity_data.set_agent(data);
        }
    }

    // Check for building response
    if let Ok(mut response) = bridge.building_data_response.try_lock() {
        if let Some(data) = Option::<SelectedBuildingData>::take(&mut response) {
            entity_data.set_building(data);
        }
    }

    // Check for resource response
    if let Ok(mut response) = bridge.resource_data_response.try_lock() {
        if let Some(data) = Option::<SelectedResourceData>::take(&mut response) {
            entity_data.set_resource(data);
        }
    }
}

/// System to request and receive tech tree data
pub fn tech_tree_data_system(
    bridge: Res<SimulationBridge>,
    panels: Res<PanelVisibility>,
    mut tech_data: ResMut<TechTreeData>,
) {
    if !panels.tech_tree {
        return;
    }

    // Request tech tree data
    if let Ok(mut request) = bridge.tech_tree_request.try_lock() {
        *request = true;
    }

    // Check for response
    if let Ok(mut response) = bridge.tech_tree_response.try_lock() {
        if let Some(data) = Option::<TechTreeSnapshot>::take(&mut response) {
            tech_data.snapshot = Some(data);
        }
    }
}
