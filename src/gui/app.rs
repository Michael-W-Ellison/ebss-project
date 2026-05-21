// src/gui/app.rs
//! Main GUI application implementing eframe::App.

use eframe::egui;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

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

    /// Shared agent data for fetching selected agent details
    pub agent_data_request: Arc<Mutex<Option<uuid::Uuid>>>,
    pub agent_data_response: Arc<Mutex<Option<SelectedAgentData>>>,
}

impl EbssApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        command_tx: Sender<SimulationCommand>,
        snapshot_rx: Receiver<SimulationSnapshot>,
        agent_data_request: Arc<Mutex<Option<uuid::Uuid>>>,
        agent_data_response: Arc<Mutex<Option<SelectedAgentData>>>,
    ) -> Self {
        Self {
            state: GuiState::new(),
            command_tx,
            snapshot_rx,
            selected_agent_data: None,
            agent_data_request,
            agent_data_response,
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
    fn check_agent_data(&mut self) {
        if let Ok(mut response) = self.agent_data_response.try_lock() {
            if response.is_some() {
                self.selected_agent_data = response.take();
            }
        }
    }
}

impl eframe::App for EbssApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process incoming snapshots
        self.process_snapshots();

        // Check for agent data response
        self.check_agent_data();

        // Request repaint for animation
        ctx.request_repaint();

        // Top panel with controls
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
                .default_width(280.0)
                .show(ctx, |ui| {
                    panels::inspector::render_inspector(
                        ui,
                        &self.state,
                        &self.selected_agent_data,
                    );
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
