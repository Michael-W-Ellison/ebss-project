// src/gui/app.rs
//! Main GUI application implementing eframe::App.

use eframe::egui;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::world::Position;
use super::state::*;
use super::panels;

/// Main GUI application
pub struct EbssApp {
    /// GUI state
    pub state: GuiState,

    /// Channel to send commands to simulation thread
    pub command_tx: Sender<SimulationCommand>,

    /// Channel to receive snapshots from simulation thread
    pub snapshot_rx: Receiver<SimulationSnapshot>,

    /// Selected agent detailed data (fetched on demand)
    pub selected_agent_data: Option<SelectedAgentData>,

    /// Selected building detailed data
    pub selected_building_data: Option<SelectedBuildingData>,

    /// Selected resource detailed data
    pub selected_resource_data: Option<SelectedResourceData>,

    /// Shared agent data for fetching selected agent details
    pub agent_data_request: Arc<Mutex<Option<uuid::Uuid>>>,
    pub agent_data_response: Arc<Mutex<Option<SelectedAgentData>>>,

    /// Shared building data for fetching selected building details
    pub building_data_request: Arc<Mutex<Option<Position>>>,
    pub building_data_response: Arc<Mutex<Option<SelectedBuildingData>>>,

    /// Shared resource data for fetching selected resource details
    pub resource_data_request: Arc<Mutex<Option<Position>>>,
    pub resource_data_response: Arc<Mutex<Option<SelectedResourceData>>>,
}

impl EbssApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        command_tx: Sender<SimulationCommand>,
        snapshot_rx: Receiver<SimulationSnapshot>,
        agent_data_request: Arc<Mutex<Option<uuid::Uuid>>>,
        agent_data_response: Arc<Mutex<Option<SelectedAgentData>>>,
        building_data_request: Arc<Mutex<Option<Position>>>,
        building_data_response: Arc<Mutex<Option<SelectedBuildingData>>>,
        resource_data_request: Arc<Mutex<Option<Position>>>,
        resource_data_response: Arc<Mutex<Option<SelectedResourceData>>>,
    ) -> Self {
        Self {
            state: GuiState::new(),
            command_tx,
            snapshot_rx,
            selected_agent_data: None,
            selected_building_data: None,
            selected_resource_data: None,
            agent_data_request,
            agent_data_response,
            building_data_request,
            building_data_response,
            resource_data_request,
            resource_data_response,
        }
    }

    /// Send a command to the simulation thread
    pub fn send_command(&self, cmd: SimulationCommand) {
        let _ = self.command_tx.send(cmd);
    }

    /// Process any pending snapshots from simulation
    fn process_snapshots(&mut self) {
        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            self.state.update_from_snapshot(snapshot);
        }
    }

    /// Check for selected agent data response
    fn check_entity_data(&mut self) {
        // Check agent data
        if let Ok(mut response) = self.agent_data_response.try_lock() {
            if response.is_some() {
                self.selected_agent_data = response.take();
                self.state.selected_agent_data = self.selected_agent_data.clone();
            }
        }

        // Check building data
        if let Ok(mut response) = self.building_data_response.try_lock() {
            if response.is_some() {
                self.selected_building_data = response.take();
                self.state.selected_building_data = self.selected_building_data.clone();
            }
        }

        // Check resource data
        if let Ok(mut response) = self.resource_data_response.try_lock() {
            if response.is_some() {
                self.selected_resource_data = response.take();
                self.state.selected_resource_data = self.selected_resource_data.clone();
            }
        }
    }

    /// Request data for currently selected entity
    fn request_selected_entity_data(&mut self) {
        match &self.state.selected {
            EntitySelection::Agent(id) => {
                if let Ok(mut request) = self.agent_data_request.try_lock() {
                    *request = Some(*id);
                }
            }
            EntitySelection::Building(pos) => {
                if let Ok(mut request) = self.building_data_request.try_lock() {
                    *request = Some(*pos);
                }
            }
            EntitySelection::Resource(pos) => {
                if let Ok(mut request) = self.resource_data_request.try_lock() {
                    *request = Some(*pos);
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for EbssApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process incoming snapshots
        self.process_snapshots();

        // Check for entity data responses
        self.check_entity_data();

        // Request data for selected entity periodically
        self.request_selected_entity_data();

        // Request repaint for animation
        ctx.request_repaint();

        // Top panel with menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.state.show_inspector, "Inspector Panel");
                    ui.checkbox(&mut self.state.show_statistics, "Statistics Panel");
                    ui.checkbox(&mut self.state.show_legend, "Legend");
                    ui.checkbox(&mut self.state.show_minimap, "Minimap");
                });
                ui.menu_button("Map", |ui| {
                    ui.checkbox(&mut self.state.map_layers.terrain, "Show Terrain");
                    ui.checkbox(&mut self.state.map_layers.resources, "Show Resources");
                    ui.checkbox(&mut self.state.map_layers.buildings, "Show Buildings");
                    ui.checkbox(&mut self.state.map_layers.agents, "Show Agents");
                    ui.checkbox(&mut self.state.map_layers.grid, "Show Grid");
                    ui.separator();
                    if ui.button("Reset View").clicked() {
                        self.state.map_zoom = 1.0;
                        self.state.map_offset = (0.0, 0.0);
                    }
                });
            });
        });

        // Bottom panel with simulation controls
        egui::TopBottomPanel::bottom("controls_panel").show(ctx, |ui| {
            panels::controls::render_controls(ui, &self.state, &self.command_tx);
        });

        // Right panel with inspector (if enabled)
        if self.state.show_inspector {
            egui::SidePanel::right("inspector_panel")
                .default_width(300.0)
                .min_width(250.0)
                .show(ctx, |ui| {
                    panels::inspector::render_inspector(ui, &mut self.state);
                });
        }

        // Left panel with statistics (if enabled)
        if self.state.show_statistics {
            egui::SidePanel::left("statistics_panel")
                .default_width(220.0)
                .show(ctx, |ui| {
                    panels::statistics::render_statistics(ui, &self.state);
                });
        }

        // Central panel with map view
        egui::CentralPanel::default().show(ctx, |ui| {
            panels::map_view::render_map(
                ui,
                &mut self.state,
                &self.command_tx,
                &self.agent_data_request,
            );
        });

        // Legend window (if enabled)
        if self.state.show_legend {
            egui::Window::new("Legend")
                .collapsible(true)
                .resizable(true)
                .show(ctx, |ui| {
                    panels::legend::render_legend(ui);
                });
        }
    }
}
