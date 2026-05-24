// src/bevy_gui/systems/bridge.rs
//! Simulation bridge system for thread communication.

use bevy::prelude::*;
use bevy::app::AppExit;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::gui::state::{
    SimulationSnapshot, SimulationCommand as GuiCommand,
    SelectedAgentData, SelectedBuildingData, SelectedResourceData,
    TechTreeSnapshot, RelationshipGraphSnapshot,
    EntitySelection as GuiEntitySelection,
};
use crate::world::Position;
use crate::bevy_gui::resources::{
    CurrentSnapshot, SimulationControl, SimState, StatisticsHistory, HistoryPoint,
    EntitySelection, SelectedEntityData, PanelVisibility, TechTreeData, RelationshipGraphData,
    TimelineData, Selection, SimulationErrors, SimulationError, NotificationQueue, NotificationType,
};
use crate::bevy_gui::events::{SimulationCommand, ShutdownRequested};

/// Error sent from simulation thread to GUI
#[derive(Debug, Clone)]
pub struct BridgeError {
    pub tick: u32,
    pub message: String,
    pub severity: crate::bevy_gui::resources::ErrorSeverity,
    pub context: Option<String>,
}

/// Handles for communication with the simulation thread.
/// Uses Arc<Mutex<>> wrappers to make channels thread-safe for Bevy.
#[derive(Resource)]
pub struct SimulationBridge {
    pub command_tx: Arc<Mutex<Sender<GuiCommand>>>,
    pub snapshot_rx: Arc<Mutex<Receiver<SimulationSnapshot>>>,
    pub error_rx: Arc<Mutex<Receiver<BridgeError>>>,
    pub shutdown_flag: Arc<AtomicBool>,
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

/// System to receive errors from the simulation thread
pub fn receive_errors_system(
    bridge: Res<SimulationBridge>,
    mut errors: ResMut<SimulationErrors>,
    mut notifications: ResMut<NotificationQueue>,
    time: Res<Time>,
) {
    if let Ok(rx) = bridge.error_rx.try_lock() {
        let current_time = time.elapsed_secs_f64();

        while let Ok(bridge_error) = rx.try_recv() {
            let error = SimulationError {
                tick: bridge_error.tick,
                message: bridge_error.message.clone(),
                severity: bridge_error.severity,
                timestamp: current_time,
                context: bridge_error.context.clone(),
            };

            // Add to error log
            errors.push(error);

            // Show notification based on severity
            let notification_msg = if let Some(ctx) = &bridge_error.context {
                format!("[{}] {}: {}", bridge_error.severity.as_str(), ctx, bridge_error.message)
            } else {
                format!("[{}] {}", bridge_error.severity.as_str(), bridge_error.message)
            };

            match bridge_error.severity {
                crate::bevy_gui::resources::ErrorSeverity::Warning => {
                    notifications.warning(&notification_msg, current_time);
                }
                crate::bevy_gui::resources::ErrorSeverity::Error => {
                    notifications.error(&notification_msg, current_time);
                }
                crate::bevy_gui::resources::ErrorSeverity::Fatal => {
                    notifications.error(&format!("FATAL: {}", notification_msg), current_time);
                    log::error!("Fatal simulation error: {}", bridge_error.message);
                }
            }
        }
    }
}

/// System to receive snapshots from the simulation thread
pub fn receive_snapshots_system(
    bridge: Res<SimulationBridge>,
    mut snapshot: ResMut<CurrentSnapshot>,
    mut sim_control: ResMut<SimulationControl>,
    mut stats_history: ResMut<StatisticsHistory>,
    mut timeline: ResMut<TimelineData>,
) {
    if let Ok(rx) = bridge.snapshot_rx.try_lock() {
        while let Ok(new_snapshot) = rx.try_recv() {
            sim_control.state = match new_snapshot.state {
                crate::gui::state::SimState::Running => SimState::Running,
                crate::gui::state::SimState::Paused => SimState::Paused,
                crate::gui::state::SimState::Stepping => SimState::Stepping,
            };
            sim_control.speed = new_snapshot.speed;

            // Update statistics history
            let tick = new_snapshot.tick;
            if stats_history.should_sample(tick) {
                let stats = &new_snapshot.population.stats;
                let point = HistoryPoint {
                    tick,
                    population: stats.total_agents,
                    average_health: stats.average_health,
                    average_energy: stats.average_energy,
                    average_happiness: stats.average_happiness,
                    total_resources: new_snapshot.world.resources.iter()
                        .map(|r| r.amount)
                        .sum(),
                    buildings_completed: new_snapshot.world.buildings.iter()
                        .filter(|b| b.progress >= 1.0)
                        .count(),
                    births: stats.total_births,
                    deaths: stats.total_deaths,
                };
                stats_history.add_point(point);
            }

            // Add events to timeline
            if !new_snapshot.events.is_empty() {
                timeline.add_events(new_snapshot.events.clone());
            }

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
                *request = Some(id.clone());
            }
        }
        EntitySelection::Building(pos) => {
            if let Ok(mut request) = bridge.building_data_request.try_lock() {
                *request = Some(pos.clone());
            }
        }
        EntitySelection::Resource(pos) => {
            if let Ok(mut request) = bridge.resource_data_request.try_lock() {
                *request = Some(pos.clone());
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

/// System to request and receive relationship graph data
pub fn relationship_graph_data_system(
    bridge: Res<SimulationBridge>,
    panels: Res<PanelVisibility>,
    mut graph_data: ResMut<RelationshipGraphData>,
) {
    if !panels.relationship_graph {
        return;
    }

    // Request relationship graph data
    if let Ok(mut request) = bridge.relationship_graph_request.try_lock() {
        *request = true;
    }

    // Check for response
    if let Ok(mut response) = bridge.relationship_graph_response.try_lock() {
        if let Some(data) = Option::<RelationshipGraphSnapshot>::take(&mut response) {
            // Check if the data has changed significantly (new agents)
            let needs_relayout = graph_data.snapshot.as_ref()
                .map(|old| old.nodes.len() != data.nodes.len())
                .unwrap_or(true);

            graph_data.snapshot = Some(data);

            if needs_relayout {
                graph_data.needs_layout = true;
            }
        }
    }
}

/// System to handle shutdown requests (from menu, keyboard shortcut, etc.)
pub fn handle_shutdown_requests(
    bridge: Res<SimulationBridge>,
    mut shutdown_events: EventReader<ShutdownRequested>,
    mut app_exit: EventWriter<AppExit>,
) {
    for _ in shutdown_events.read() {
        log::info!("Shutdown requested, signaling simulation thread...");
        bridge.shutdown_flag.store(true, Ordering::SeqCst);
        app_exit.send(AppExit::Success);
    }
}

/// System to handle app exit events and ensure clean shutdown
pub fn on_app_exit(
    bridge: Res<SimulationBridge>,
    mut exit_events: EventReader<AppExit>,
) {
    for _ in exit_events.read() {
        log::info!("App exit detected, ensuring simulation thread shutdown...");
        bridge.shutdown_flag.store(true, Ordering::SeqCst);
    }
}
