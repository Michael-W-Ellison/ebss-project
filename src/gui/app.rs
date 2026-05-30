// src/gui/app.rs
//! Main GUI application implementing eframe::App.

use eframe::egui::{self, Key};
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

    /// Tech tree snapshot data (updated periodically)
    pub tech_tree_request: Arc<Mutex<bool>>,
    pub tech_tree_response: Arc<Mutex<Option<TechTreeSnapshot>>>,

    /// Relationship graph snapshot data (updated periodically)
    pub relationship_graph_request: Arc<Mutex<bool>>,
    pub relationship_graph_response: Arc<Mutex<Option<RelationshipGraphSnapshot>>>,
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
        tech_tree_request: Arc<Mutex<bool>>,
        tech_tree_response: Arc<Mutex<Option<TechTreeSnapshot>>>,
        relationship_graph_request: Arc<Mutex<bool>>,
        relationship_graph_response: Arc<Mutex<Option<RelationshipGraphSnapshot>>>,
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
            tech_tree_request,
            tech_tree_response,
            relationship_graph_request,
            relationship_graph_response,
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

        // Check tech tree data
        if let Ok(mut response) = self.tech_tree_response.try_lock() {
            if response.is_some() {
                self.state.tech_tree_snapshot = response.take();
            }
        }

        // Check relationship graph data
        if let Ok(mut response) = self.relationship_graph_response.try_lock() {
            if response.is_some() {
                self.state.relationship_graph_snapshot = response.take();
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

        // Request tech tree data when tech tree panel is visible
        if self.state.show_tech_tree {
            if let Ok(mut request) = self.tech_tree_request.try_lock() {
                *request = true;
            }
        }

        // Request relationship graph data when relationship graph panel is visible
        if self.state.show_relationship_graph {
            if let Ok(mut request) = self.relationship_graph_request.try_lock() {
                *request = true;
            }
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

        let current_time = ctx.input(|i| i.time);

        // Handle global keyboard shortcuts
        self.handle_global_shortcuts(ctx, current_time);

        // Top panel with menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save... (Ctrl+S)").clicked() {
                        self.state.show_save_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("Load... (Ctrl+O)").clicked() {
                        self.state.show_load_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Search... (Ctrl+F)").clicked() {
                        self.state.show_search = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.state.show_inspector, "Inspector Panel (I)");
                    ui.checkbox(&mut self.state.show_statistics, "Statistics Panel (P)");
                    ui.checkbox(&mut self.state.show_tech_tree, "Technology Tree (T)");
                    ui.checkbox(&mut self.state.show_timeline, "Timeline (Y)");
                    ui.checkbox(&mut self.state.show_relationship_graph, "Relationship Graph (R)");
                    ui.checkbox(&mut self.state.show_legend, "Legend (L)");
                    ui.checkbox(&mut self.state.show_minimap, "Minimap (M)");
                    ui.separator();
                    ui.checkbox(&mut self.state.show_keyboard_help, "Keyboard Shortcuts (H)");
                });
                ui.menu_button("Map", |ui| {
                    ui.checkbox(&mut self.state.map_layers.terrain, "Show Terrain");
                    ui.checkbox(&mut self.state.map_layers.resources, "Show Resources");
                    ui.checkbox(&mut self.state.map_layers.buildings, "Show Buildings");
                    ui.checkbox(&mut self.state.map_layers.agents, "Show Agents");
                    ui.checkbox(&mut self.state.map_layers.grid, "Show Grid (G)");
                    ui.separator();
                    if ui.button("Reset View (Home)").clicked() {
                        self.state.map_zoom = 1.0;
                        self.state.map_offset = (0.0, 0.0);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts (H)").clicked() {
                        self.state.show_keyboard_help = true;
                        ui.close_menu();
                    }
                });

                // Status info on the right side
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(snapshot) = &self.state.latest_snapshot {
                        let status = match self.state.simulation_state {
                            SimState::Running => "Running",
                            SimState::Paused => "Paused",
                            SimState::Stepping => "Stepping",
                        };
                        ui.label(format!(
                            "Tick: {} | Agents: {} | {}",
                            snapshot.tick,
                            snapshot.population.agents.iter().filter(|a| a.is_alive).count(),
                            status
                        ));
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
                .default_width(280.0)
                .min_width(250.0)
                .show(ctx, |ui| {
                    panels::statistics::render_statistics(ui, &mut self.state);
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

            // Render notifications overlay
            panels::save_load::render_notifications(ui, &mut self.state, current_time);
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

        // Tech tree window (if enabled)
        if self.state.show_tech_tree {
            egui::Window::new("Technology Tree")
                .collapsible(true)
                .resizable(true)
                .default_size([900.0, 600.0])
                .show(ctx, |ui| {
                    panels::tech_tree::render_tech_tree(ui, &mut self.state);
                });
        }

        // Timeline window (if enabled)
        if self.state.show_timeline {
            egui::Window::new("Event Timeline")
                .collapsible(true)
                .resizable(true)
                .default_size([450.0, 550.0])
                .show(ctx, |ui| {
                    panels::timeline::render_timeline(ui, &mut self.state);
                });
        }

        // Relationship graph window (if enabled)
        if self.state.show_relationship_graph {
            egui::Window::new("Relationship Graph")
                .collapsible(true)
                .resizable(true)
                .default_size([700.0, 550.0])
                .show(ctx, |ui| {
                    panels::relationship_graph::render_relationship_graph(ui, &mut self.state);
                });
        }

        // Search window (if enabled)
        if self.state.show_search {
            egui::Window::new("Search")
                .collapsible(false)
                .resizable(true)
                .default_size([400.0, 450.0])
                .show(ctx, |ui| {
                    panels::search::render_search_panel(ui, &mut self.state);
                });
        }

        // Keyboard help window (if enabled)
        if self.state.show_keyboard_help {
            egui::Window::new("Keyboard Shortcuts")
                .collapsible(false)
                .resizable(false)
                .default_size([350.0, 500.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    panels::keyboard_help::render_keyboard_help(ui);
                });
        }

        // Save dialog (if enabled)
        if self.state.show_save_dialog {
            egui::Window::new("Save Simulation")
                .collapsible(false)
                .resizable(false)
                .default_size([400.0, 300.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    panels::save_load::render_save_dialog(ui, &mut self.state, current_time);
                });
        }

        // Load dialog (if enabled)
        if self.state.show_load_dialog {
            egui::Window::new("Load Simulation")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 400.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    panels::save_load::render_load_dialog(ui, &mut self.state, current_time);
                });
        }
    }
}

impl EbssApp {
    fn handle_global_shortcuts(&mut self, ctx: &egui::Context, current_time: f64) {
        ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            let shift = i.modifiers.shift;

            // Escape - close dialogs or deselect
            if i.key_pressed(Key::Escape) {
                if self.state.show_keyboard_help {
                    self.state.show_keyboard_help = false;
                } else if self.state.show_search {
                    self.state.show_search = false;
                } else if self.state.show_save_dialog {
                    self.state.show_save_dialog = false;
                } else if self.state.show_load_dialog {
                    self.state.show_load_dialog = false;
                } else if self.state.show_tech_tree {
                    self.state.show_tech_tree = false;
                } else if self.state.show_timeline {
                    self.state.show_timeline = false;
                } else if self.state.show_relationship_graph {
                    self.state.show_relationship_graph = false;
                } else {
                    self.state.selected = EntitySelection::None;
                    self.state.follow_selected = false;
                }
            }

            // Simulation controls
            if i.key_pressed(Key::Space) && !ctrl {
                match self.state.simulation_state {
                    SimState::Running => {
                        let _ = self.command_tx.send(SimulationCommand::Pause);
                        self.state.notify("Paused", NotificationType::Info, current_time);
                    }
                    SimState::Paused | SimState::Stepping => {
                        let _ = self.command_tx.send(SimulationCommand::Play);
                        self.state.notify("Playing", NotificationType::Info, current_time);
                    }
                }
            }

            if i.key_pressed(Key::N) && !ctrl {
                let _ = self.command_tx.send(SimulationCommand::Step);
            }

            // Speed controls (1-5 for 1x-5x, 0 for 10x)
            for (key, speed) in [
                (Key::Num1, 1.0),
                (Key::Num2, 2.0),
                (Key::Num3, 3.0),
                (Key::Num4, 4.0),
                (Key::Num5, 5.0),
                (Key::Num0, 10.0),
            ] {
                if i.key_pressed(key) && !ctrl {
                    let _ = self.command_tx.send(SimulationCommand::SetSpeed(speed));
                    self.state.notify(format!("Speed: {}x", speed), NotificationType::Info, current_time);
                }
            }

            // Panel toggles
            if i.key_pressed(Key::H) && !ctrl {
                self.state.show_keyboard_help = !self.state.show_keyboard_help;
            }
            if i.key_pressed(Key::I) && !ctrl {
                self.state.show_inspector = !self.state.show_inspector;
            }
            if i.key_pressed(Key::P) && !ctrl {
                self.state.show_statistics = !self.state.show_statistics;
            }
            if i.key_pressed(Key::T) && !ctrl {
                self.state.show_tech_tree = !self.state.show_tech_tree;
            }
            if i.key_pressed(Key::L) && !ctrl {
                self.state.show_legend = !self.state.show_legend;
            }
            if i.key_pressed(Key::M) && !ctrl {
                self.state.show_minimap = !self.state.show_minimap;
            }
            if i.key_pressed(Key::Y) && !ctrl {
                self.state.show_timeline = !self.state.show_timeline;
            }
            if i.key_pressed(Key::R) && !ctrl {
                self.state.show_relationship_graph = !self.state.show_relationship_graph;
            }

            // Ctrl shortcuts
            if ctrl {
                if i.key_pressed(Key::F) {
                    self.state.show_search = true;
                    self.state.perform_search();
                }
                if i.key_pressed(Key::S) {
                    self.state.show_save_dialog = true;
                }
                if i.key_pressed(Key::O) {
                    self.state.show_load_dialog = true;
                }
            }

            // Entity cycling with Tab
            if i.key_pressed(Key::Tab) {
                if shift {
                    self.state.select_previous_entity();
                } else {
                    self.state.select_next_entity();
                }

                // Center on new selection
                let view_size = (400.0, 300.0);
                self.state.center_on_selected(12.0, view_size);
            }
        });
    }
}
