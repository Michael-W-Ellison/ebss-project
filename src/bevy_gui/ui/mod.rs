// src/bevy_gui/ui/mod.rs
//! UI rendering systems using bevy_egui.

pub mod map;
pub mod panels;
pub mod tooltip;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::bevy_gui::resources::*;
use crate::bevy_gui::events::{SimulationCommand, CenterMapRequest};
use crate::gui::events::SimulationEventExt;

pub use panels::{
    render_legend_panel, render_inspector_panel, render_statistics_panel,
    render_tech_tree_panel, render_timeline_panel, render_relationship_graph_panel,
    render_save_dialog, render_load_dialog, render_search_panel, search_system,
};
pub use map::render_map;

/// Render the top menu bar
// Bevy system: parameters are injected by the ECS scheduler
#[allow(clippy::too_many_arguments)]
pub fn render_menu_bar(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut map_view: ResMut<MapViewState>,
    mut selection: ResMut<Selection>,
    mut sim_control: ResMut<SimulationControl>,
    mut sim_commands: EventWriter<SimulationCommand>,
    mut center_request: EventWriter<CenterMapRequest>,
    mut notifications: ResMut<NotificationQueue>,
    snapshot: Res<CurrentSnapshot>,
    stats_history: Res<StatisticsHistory>,
    timeline: Res<TimelineData>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    egui::TopBottomPanel::top("top_menu").show(egui_ctx.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            // ===== FILE MENU =====
            ui.menu_button("File", |ui| {
                ui.set_min_width(180.0);

                if ui.add(egui::Button::new("Save...").shortcut_text("Ctrl+S"))
                    .on_hover_text("Save the current simulation state to a file")
                    .clicked()
                {
                    panels.save_dialog = true;
                    ui.close_menu();
                }

                if ui.add(egui::Button::new("Load...").shortcut_text("Ctrl+O"))
                    .on_hover_text("Load a previously saved simulation")
                    .clicked()
                {
                    panels.load_dialog = true;
                    ui.close_menu();
                }

                ui.separator();

                ui.label(egui::RichText::new("Export").small().color(egui::Color32::GRAY));

                if ui.add(egui::Button::new("Export Statistics..."))
                    .on_hover_text("Export population statistics to CSV file")
                    .clicked()
                {
                    match export_statistics_csv(&stats_history) {
                        Ok(path) => notifications.success(format!("Statistics exported to {}", path), current_time),
                        Err(e) => notifications.error(format!("Export failed: {}", e), current_time),
                    }
                    ui.close_menu();
                }

                if ui.add(egui::Button::new("Export Timeline..."))
                    .on_hover_text("Export event timeline to CSV file")
                    .clicked()
                {
                    match export_timeline_csv(&timeline) {
                        Ok(path) => notifications.success(format!("Timeline exported to {}", path), current_time),
                        Err(e) => notifications.error(format!("Export failed: {}", e), current_time),
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui.add(egui::Button::new("Quit").shortcut_text("Alt+F4"))
                    .on_hover_text("Exit the application")
                    .clicked()
                {
                    std::process::exit(0);
                }
            });

            // ===== EDIT MENU =====
            ui.menu_button("Edit", |ui| {
                ui.set_min_width(200.0);

                if ui.add(egui::Button::new("Search...").shortcut_text("Ctrl+F"))
                    .on_hover_text("Search for agents, buildings, or resources")
                    .clicked()
                {
                    panels.search = true;
                    ui.close_menu();
                }

                ui.separator();

                ui.label(egui::RichText::new("Selection").small().color(egui::Color32::GRAY));

                let has_selection = !matches!(selection.current, EntitySelection::None);

                if ui.add_enabled(has_selection, egui::Button::new("Deselect All").shortcut_text("Esc"))
                    .on_hover_text("Clear the current selection")
                    .clicked()
                {
                    selection.deselect();
                    sim_commands.send(SimulationCommand::DeselectAll);
                    notifications.info("Selection cleared", current_time);
                    ui.close_menu();
                }

                if ui.add(egui::Button::new("Select Next Agent").shortcut_text("Tab"))
                    .on_hover_text("Cycle to the next agent")
                    .clicked()
                {
                    if let Some(snap) = &snapshot.snapshot {
                        select_next_agent(&mut selection, &mut sim_commands, &mut center_request, &mut notifications, snap, current_time, false);
                    }
                    ui.close_menu();
                }

                if ui.add(egui::Button::new("Select Previous Agent").shortcut_text("Shift+Tab"))
                    .on_hover_text("Cycle to the previous agent")
                    .clicked()
                {
                    if let Some(snap) = &snapshot.snapshot {
                        select_next_agent(&mut selection, &mut sim_commands, &mut center_request, &mut notifications, snap, current_time, true);
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui.add_enabled(has_selection, egui::Button::new("Center on Selection").shortcut_text("C"))
                    .on_hover_text("Center the map view on the selected entity")
                    .clicked()
                {
                    center_on_current_selection(&selection, &snapshot, &mut center_request, &mut notifications, current_time);
                    ui.close_menu();
                }

                if ui.add_enabled(
                    matches!(selection.current, EntitySelection::Agent(_)),
                    egui::Button::new("Follow Selection").shortcut_text("F")
                )
                    .on_hover_text("Auto-center on selected agent as it moves")
                    .clicked()
                {
                    selection.toggle_follow();
                    if selection.follow_selected {
                        notifications.info("Follow mode enabled", current_time);
                    } else {
                        notifications.info("Follow mode disabled", current_time);
                    }
                    ui.close_menu();
                }
            });

            // ===== SIMULATION MENU =====
            ui.menu_button("Simulation", |ui| {
                ui.set_min_width(180.0);

                let is_running = sim_control.is_running();

                if is_running {
                    if ui.add(egui::Button::new("⏸ Pause").shortcut_text("Space"))
                        .on_hover_text("Pause the simulation")
                        .clicked()
                    {
                        sim_control.state = SimState::Paused;
                        sim_commands.send(SimulationCommand::Pause);
                        notifications.info("Paused", current_time);
                        ui.close_menu();
                    }
                } else {
                    if ui.add(egui::Button::new("▶ Play").shortcut_text("Space"))
                        .on_hover_text("Resume the simulation")
                        .clicked()
                    {
                        sim_control.state = SimState::Running;
                        sim_commands.send(SimulationCommand::Play);
                        notifications.info("Playing", current_time);
                        ui.close_menu();
                    }
                }

                if ui.add(egui::Button::new("⏭ Step Forward").shortcut_text("N"))
                    .on_hover_text("Advance simulation by one tick")
                    .clicked()
                {
                    sim_commands.send(SimulationCommand::Step);
                    notifications.info("Stepped", current_time);
                    ui.close_menu();
                }

                ui.separator();

                ui.label(egui::RichText::new("Speed").small().color(egui::Color32::GRAY));

                let speed_options = [
                    (0.5, "½× Slow", None),
                    (1.0, "1× Normal", Some("1")),
                    (2.0, "2× Fast", Some("2")),
                    (5.0, "5× Faster", Some("5")),
                    (10.0, "10× Maximum", Some("0")),
                ];

                for (speed, label, shortcut) in speed_options {
                    let is_selected = (sim_control.speed - speed).abs() < 0.01;
                    let button = if let Some(key) = shortcut {
                        egui::Button::new(label).shortcut_text(key)
                    } else {
                        egui::Button::new(label)
                    };

                    let mut response = ui.add(button);
                    if is_selected {
                        response = response.highlight();
                    }
                    if response.clicked() {
                        sim_control.set_speed(speed);
                        sim_commands.send(SimulationCommand::SetSpeed(speed));
                        notifications.info(format!("Speed: {}x", speed), current_time);
                        ui.close_menu();
                    }
                }

                ui.separator();

                // Current speed display
                ui.horizontal(|ui| {
                    ui.label("Current:");
                    ui.label(egui::RichText::new(format!("{:.1}×", sim_control.speed)).strong());
                });
            });

            // ===== VIEW MENU =====
            ui.menu_button("View", |ui| {
                ui.set_min_width(200.0);

                ui.label(egui::RichText::new("Panels").small().color(egui::Color32::GRAY));
                menu_checkbox_with_tooltip(ui, &mut panels.inspector, "Inspector", "I", "View detailed information about selected entities");
                menu_checkbox_with_tooltip(ui, &mut panels.statistics, "Statistics", "P", "Population graphs and world statistics");
                menu_checkbox_with_tooltip(ui, &mut panels.tech_tree, "Technology Tree", "T", "View discovered and available technologies");
                menu_checkbox_with_tooltip(ui, &mut panels.timeline, "Timeline", "Y", "Historical events and milestones");
                menu_checkbox_with_tooltip(ui, &mut panels.relationship_graph, "Relationships", "R", "Social network visualization");
                menu_checkbox_with_tooltip(ui, &mut panels.legend, "Legend", "L", "Map symbol and color reference");

                ui.separator();

                ui.label(egui::RichText::new("Map Display").small().color(egui::Color32::GRAY));
                menu_checkbox_with_tooltip(ui, &mut map_view.minimap.enabled, "Minimap", "M", "Show corner minimap for navigation");
                menu_checkbox_with_tooltip(ui, &mut map_view.layers.grid, "Grid Overlay", "G", "Show tile grid lines");

                ui.separator();

                ui.label(egui::RichText::new("Zoom").small().color(egui::Color32::GRAY));

                ui.horizontal(|ui| {
                    if ui.button("-").on_hover_text("Zoom out").clicked() {
                        map_view.zoom_out();
                    }
                    ui.label(format!("{:.0}%", map_view.zoom * 100.0));
                    if ui.button("+").on_hover_text("Zoom in").clicked() {
                        map_view.zoom_in();
                    }
                });

                let zoom_presets = [
                    (0.5, "50%"),
                    (1.0, "100%"),
                    (2.0, "200%"),
                    (4.0, "400%"),
                ];

                ui.horizontal(|ui| {
                    for (zoom, label) in zoom_presets {
                        if ui.selectable_label((map_view.zoom - zoom).abs() < 0.01, label).clicked() {
                            map_view.zoom = zoom;
                        }
                    }
                });

                ui.separator();

                if ui.add(egui::Button::new("Reset View").shortcut_text("Home"))
                    .on_hover_text("Reset zoom and pan to default")
                    .clicked()
                {
                    map_view.reset_view();
                    notifications.info("View reset", current_time);
                    ui.close_menu();
                }

                ui.separator();

                menu_checkbox_with_tooltip(ui, &mut panels.keyboard_help, "Keyboard Shortcuts", "H", "Show all keyboard shortcuts");
            });

            // ===== MAP MENU =====
            ui.menu_button("Map", |ui| {
                ui.set_min_width(180.0);

                ui.label(egui::RichText::new("Layers").small().color(egui::Color32::GRAY));
                ui.checkbox(&mut map_view.layers.terrain, "Terrain")
                    .on_hover_text("Show terrain types (plains, forest, water, etc.)");
                ui.checkbox(&mut map_view.layers.resources, "Resources")
                    .on_hover_text("Show resource deposits on the map");
                ui.checkbox(&mut map_view.layers.buildings, "Buildings")
                    .on_hover_text("Show constructed and in-progress buildings");
                ui.checkbox(&mut map_view.layers.agents, "Agents")
                    .on_hover_text("Show population members");

                ui.separator();

                ui.label(egui::RichText::new("Quick Toggle").small().color(egui::Color32::GRAY));

                ui.horizontal(|ui| {
                    if ui.button("All On").on_hover_text("Show all layers").clicked() {
                        map_view.layers.terrain = true;
                        map_view.layers.resources = true;
                        map_view.layers.buildings = true;
                        map_view.layers.agents = true;
                    }
                    if ui.button("All Off").on_hover_text("Hide all layers").clicked() {
                        map_view.layers.terrain = false;
                        map_view.layers.resources = false;
                        map_view.layers.buildings = false;
                        map_view.layers.agents = false;
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Terrain Only").clicked() {
                        map_view.layers.terrain = true;
                        map_view.layers.resources = false;
                        map_view.layers.buildings = false;
                        map_view.layers.agents = false;
                    }
                    if ui.button("Agents Only").clicked() {
                        map_view.layers.terrain = true;
                        map_view.layers.resources = false;
                        map_view.layers.buildings = false;
                        map_view.layers.agents = true;
                    }
                });

                ui.separator();

                ui.label(egui::RichText::new("Agent Filters").small().color(egui::Color32::GRAY));

                if map_view.agent_filter.is_filtering() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Active filters").color(egui::Color32::from_rgb(255, 200, 100)));
                        if ui.button("Clear").clicked() {
                            map_view.agent_filter.reset();
                            notifications.info("Filters cleared", current_time);
                        }
                    });
                } else {
                    ui.label("No filters active");
                }

                ui.menu_button("Configure Filters...", |ui| {
                    render_agent_filter_submenu(ui, &mut map_view.agent_filter);
                });
            });

            // ===== WINDOW MENU =====
            ui.menu_button("Window", |ui| {
                ui.set_min_width(180.0);

                ui.label(egui::RichText::new("Layout").small().color(egui::Color32::GRAY));

                if ui.button("Close All Panels")
                    .on_hover_text("Close all side panels")
                    .clicked()
                {
                    panels.inspector = false;
                    panels.statistics = false;
                    panels.tech_tree = false;
                    panels.timeline = false;
                    panels.relationship_graph = false;
                    panels.legend = false;
                    panels.keyboard_help = false;
                    notifications.info("All panels closed", current_time);
                    ui.close_menu();
                }

                if ui.button("Default Layout")
                    .on_hover_text("Reset to default panel layout")
                    .clicked()
                {
                    panels.inspector = true;
                    panels.statistics = false;
                    panels.tech_tree = false;
                    panels.timeline = false;
                    panels.relationship_graph = false;
                    panels.legend = false;
                    panels.keyboard_help = false;
                    map_view.minimap.enabled = true;
                    notifications.info("Layout reset to default", current_time);
                    ui.close_menu();
                }

                if ui.button("Analysis Layout")
                    .on_hover_text("Open statistics and timeline panels")
                    .clicked()
                {
                    panels.inspector = false;
                    panels.statistics = true;
                    panels.tech_tree = false;
                    panels.timeline = true;
                    panels.relationship_graph = false;
                    panels.legend = false;
                    notifications.info("Analysis layout applied", current_time);
                    ui.close_menu();
                }

                if ui.button("Social Layout")
                    .on_hover_text("Open inspector and relationship graph")
                    .clicked()
                {
                    panels.inspector = true;
                    panels.statistics = false;
                    panels.tech_tree = false;
                    panels.timeline = false;
                    panels.relationship_graph = true;
                    panels.legend = false;
                    notifications.info("Social layout applied", current_time);
                    ui.close_menu();
                }

                ui.separator();

                ui.label(egui::RichText::new("Minimap").small().color(egui::Color32::GRAY));

                ui.checkbox(&mut map_view.minimap.enabled, "Show Minimap");

                ui.horizontal(|ui| {
                    ui.label("Position:");
                    let positions = [
                        (MinimapPosition::TopLeft, "TL"),
                        (MinimapPosition::TopRight, "TR"),
                        (MinimapPosition::BottomLeft, "BL"),
                        (MinimapPosition::BottomRight, "BR"),
                    ];
                    for (pos, label) in positions {
                        if ui.selectable_label(map_view.minimap.position == pos, label).clicked() {
                            map_view.minimap.position = pos;
                        }
                    }
                });
            });

            // ===== HELP MENU =====
            ui.menu_button("Help", |ui| {
                ui.set_min_width(220.0);

                if ui.add(egui::Button::new("Keyboard Shortcuts").shortcut_text("H"))
                    .on_hover_text("Show all available keyboard shortcuts")
                    .clicked()
                {
                    panels.keyboard_help = true;
                    ui.close_menu();
                }

                ui.separator();

                ui.label(egui::RichText::new("Quick Tips").small().color(egui::Color32::GRAY));

                ui.label("• Click on map to select entities");
                ui.label("• Tab/Shift+Tab cycles through agents");
                ui.label("• Scroll wheel zooms toward cursor");
                ui.label("• Drag to pan the map view");
                ui.label("• Press F to follow selected agent");

                ui.separator();

                ui.label(egui::RichText::new("About").small().color(egui::Color32::GRAY));
                ui.label(egui::RichText::new("EBSS").strong());
                ui.label("Entity-Based Social Simulation");
                ui.label(egui::RichText::new("Bevy GUI v0.1.0").small().color(egui::Color32::GRAY));

                if let Some(snap) = &snapshot.snapshot {
                    ui.separator();
                    ui.label(egui::RichText::new("Session Info").small().color(egui::Color32::GRAY));
                    ui.label(format!("World: {}×{}", snap.world.width, snap.world.height));
                    ui.label(format!("Total Ticks: {}", snap.tick));
                    let days = snap.tick / 1440;
                    ui.label(format!("Simulated Days: {}", days));
                }
            });

            // ===== STATUS BAR (right side) =====
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (status_text, status_color) = match sim_control.state {
                    SimState::Running => ("▶ Running", egui::Color32::from_rgb(100, 200, 100)),
                    SimState::Paused => ("⏸ Paused", egui::Color32::from_rgb(200, 200, 100)),
                    SimState::Stepping => ("⏭ Step", egui::Color32::from_rgb(100, 180, 255)),
                };

                if let Some(snap) = &snapshot.snapshot {
                    let alive_count = snap.population.agents.iter().filter(|a| a.is_alive).count();
                    let total_count = snap.population.agents.len();

                    let days = snap.tick / 1440;
                    let hours = (snap.tick % 1440) / 60;
                    let minutes = snap.tick % 60;

                    // Status with icon
                    ui.label(egui::RichText::new(status_text).color(status_color).strong())
                        .on_hover_text(format!("Simulation state\nSpeed: {:.1}×", sim_control.speed));

                    ui.separator();

                    // Time display
                    ui.label(format!("Day {} {:02}:{:02}", days + 1, hours, minutes))
                        .on_hover_text(format!(
                            "Simulation time\nTick: {}\n1 day = 1440 ticks",
                            snap.tick
                        ));

                    ui.separator();

                    // Population with health indicator
                    let avg_health = snap.population.stats.average_health;
                    let health_color = if avg_health >= 70.0 {
                        egui::Color32::from_rgb(100, 200, 100)
                    } else if avg_health >= 40.0 {
                        egui::Color32::from_rgb(200, 200, 100)
                    } else {
                        egui::Color32::from_rgb(200, 100, 100)
                    };

                    ui.label(egui::RichText::new(format!("♥ {}", alive_count)).color(health_color))
                        .on_hover_text(format!(
                            "{} alive / {} total\nAvg Health: {:.0}%\nAvg Energy: {:.0}%",
                            alive_count,
                            total_count,
                            snap.population.stats.average_health,
                            snap.population.stats.average_energy
                        ));

                    ui.separator();

                    // Speed indicator
                    ui.label(format!("{:.1}×", sim_control.speed))
                        .on_hover_text("Simulation speed\nPress 1-5 or 0 to change");
                } else {
                    ui.label(egui::RichText::new("⏳ Connecting...").color(egui::Color32::GRAY));
                }
            });
        });
    });
}

fn menu_checkbox_with_tooltip(ui: &mut egui::Ui, value: &mut bool, label: &str, shortcut: &str, tooltip: &str) {
    ui.horizontal(|ui| {
        ui.checkbox(value, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(shortcut).small().color(egui::Color32::GRAY));
        });
    }).response.on_hover_text(tooltip);
}

fn select_next_agent(
    selection: &mut Selection,
    sim_commands: &mut EventWriter<SimulationCommand>,
    center_request: &mut EventWriter<crate::bevy_gui::events::CenterMapRequest>,
    notifications: &mut NotificationQueue,
    snapshot: &crate::gui::state::SimulationSnapshot,
    current_time: f64,
    reverse: bool,
) {
    let alive_agents: Vec<_> = snapshot.population.agents.iter()
        .filter(|a| a.is_alive)
        .collect();

    if alive_agents.is_empty() {
        notifications.info("No agents available", current_time);
        return;
    }

    let current_idx = selection.current.agent_id()
        .and_then(|id| alive_agents.iter().position(|a| a.id == id));

    let next_idx = if reverse {
        match current_idx {
            None => alive_agents.len() - 1,
            Some(0) => alive_agents.len() - 1,
            Some(idx) => idx - 1,
        }
    } else {
        match current_idx {
            None => 0,
            Some(idx) => (idx + 1) % alive_agents.len(),
        }
    };

    let agent = alive_agents[next_idx];
    selection.select_agent(agent.id);
    sim_commands.send(SimulationCommand::SelectEntity(EntitySelection::Agent(agent.id)));

    center_request.send(crate::bevy_gui::events::CenterMapRequest {
        x: agent.position.0,
        y: agent.position.1,
    });

    notifications.info(
        format!("Agent {}/{}", next_idx + 1, alive_agents.len()),
        current_time,
    );
}

fn center_on_current_selection(
    selection: &Selection,
    snapshot: &Res<CurrentSnapshot>,
    center_request: &mut EventWriter<crate::bevy_gui::events::CenterMapRequest>,
    notifications: &mut NotificationQueue,
    current_time: f64,
) {
    match &selection.current {
        EntitySelection::Agent(id) => {
            if let Some(snap) = &snapshot.snapshot {
                if let Some(agent) = snap.population.agents.iter().find(|a| a.id == *id && a.is_alive) {
                    center_request.send(crate::bevy_gui::events::CenterMapRequest {
                        x: agent.position.0,
                        y: agent.position.1,
                    });
                    notifications.info("Centered on agent", current_time);
                }
            }
        }
        EntitySelection::Building(pos) | EntitySelection::Resource(pos) | EntitySelection::Terrain(pos) => {
            center_request.send(crate::bevy_gui::events::CenterMapRequest { x: pos.x, y: pos.y });
            notifications.info("Centered on selection", current_time);
        }
        EntitySelection::None => {
            notifications.info("Nothing selected", current_time);
        }
    }
}

fn render_agent_filter_submenu(ui: &mut egui::Ui, filter: &mut crate::bevy_gui::resources::AgentMapFilter) {
    ui.set_min_width(200.0);

    ui.horizontal(|ui| {
        if ui.button("Show All").clicked() {
            filter.reset();
        }
        if ui.button("Hide All").clicked() {
            filter.show_infant = false;
            filter.show_child = false;
            filter.show_adolescent = false;
            filter.show_adult = false;
            filter.show_elderly = false;
        }
    });

    ui.separator();

    ui.label(egui::RichText::new("Life Stage").small().color(egui::Color32::GRAY));
    ui.horizontal(|ui| {
        ui.checkbox(&mut filter.show_infant, "Infant");
        ui.checkbox(&mut filter.show_child, "Child");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut filter.show_adolescent, "Teen");
        ui.checkbox(&mut filter.show_adult, "Adult");
        ui.checkbox(&mut filter.show_elderly, "Elder");
    });

    ui.separator();

    ui.label(egui::RichText::new("Gender").small().color(egui::Color32::GRAY));
    ui.horizontal(|ui| {
        ui.checkbox(&mut filter.show_male, "Male");
        ui.checkbox(&mut filter.show_female, "Female");
    });

    ui.separator();

    ui.label(egui::RichText::new("Status").small().color(egui::Color32::GRAY));
    ui.horizontal(|ui| {
        ui.checkbox(&mut filter.show_sleeping, "Sleeping");
        ui.checkbox(&mut filter.show_idle, "Idle");
    });
}

/// Render the bottom controls panel
pub fn render_controls_panel(
    mut egui_ctx: EguiContexts,
    mut sim_control: ResMut<SimulationControl>,
    mut sim_commands: EventWriter<SimulationCommand>,
    mut notifications: ResMut<NotificationQueue>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    egui::TopBottomPanel::bottom("controls_panel").show(egui_ctx.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            // Play/Pause button with visual feedback
            let (play_text, play_tooltip) = if sim_control.is_running() {
                ("⏸ Pause", "Pause the simulation (Space)")
            } else {
                ("▶ Play", "Resume the simulation (Space)")
            };

            let play_button = egui::Button::new(play_text)
                .min_size(egui::vec2(70.0, 0.0));

            if ui.add(play_button)
                .on_hover_text(play_tooltip)
                .clicked()
            {
                sim_control.toggle_pause();
                let cmd = if sim_control.is_running() {
                    notifications.info("Playing", current_time);
                    SimulationCommand::Play
                } else {
                    notifications.info("Paused", current_time);
                    SimulationCommand::Pause
                };
                sim_commands.send(cmd);
            }

            // Step button
            let step_button = egui::Button::new("⏭ Step");
            if ui.add(step_button)
                .on_hover_text("Advance simulation by one tick (N)")
                .clicked()
            {
                sim_commands.send(SimulationCommand::Step);
                notifications.info("Stepped", current_time);
            }

            ui.separator();

            // Speed control with shortcut hints
            ui.label("Speed:");

            let speed_configs = [
                (0.5, "½x", None, "Half speed - detailed observation"),
                (1.0, "1x", Some("1"), "Normal speed"),
                (2.0, "2x", Some("2"), "Double speed"),
                (5.0, "5x", Some("5"), "Fast forward"),
                (10.0, "10x", Some("0"), "Maximum speed"),
            ];

            for (speed, label, shortcut, tooltip) in speed_configs {
                let is_selected = (sim_control.speed - speed).abs() < 0.01;
                let button_text = if is_selected {
                    egui::RichText::new(label).strong()
                } else {
                    egui::RichText::new(label)
                };

                let full_tooltip = if let Some(key) = shortcut {
                    format!("{} ({})", tooltip, key)
                } else {
                    tooltip.to_string()
                };

                if ui.selectable_label(is_selected, button_text)
                    .on_hover_text(full_tooltip)
                    .clicked()
                {
                    sim_control.set_speed(speed);
                    sim_commands.send(SimulationCommand::SetSpeed(speed));
                    notifications.info(format!("Speed: {}x", speed), current_time);
                }
            }

            ui.separator();

            // Speed slider for fine control
            ui.label("Fine:");
            let mut speed = sim_control.speed;
            let slider = egui::Slider::new(&mut speed, 0.1..=20.0)
                .logarithmic(true)
                .clamping(egui::SliderClamping::Always)
                .max_decimals(1)
                .suffix("x");

            if ui.add(slider)
                .on_hover_text("Drag for fine speed control")
                .changed()
            {
                sim_control.set_speed(speed);
                sim_commands.send(SimulationCommand::SetSpeed(speed));
            }

            // Quick tips on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("H: Help").small().color(egui::Color32::GRAY))
                    .on_hover_text("Press H for keyboard shortcuts");
            });
        });
    });
}

/// Render notifications
pub fn render_notifications(
    mut egui_ctx: EguiContexts,
    mut notifications: ResMut<NotificationQueue>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();
    notifications.cleanup_expired(current_time);

    if notifications.notifications.is_empty() {
        return;
    }

    let ctx = egui_ctx.ctx_mut();
    let screen_rect = ctx.screen_rect();
    let notification_width = 250.0;
    let notification_height = 40.0;
    let margin = 10.0;

    for (i, notification) in notifications.notifications.iter().enumerate() {
        let y_offset = margin + (i as f32 * (notification_height + 5.0));

        let pos = egui::pos2(
            screen_rect.max.x - notification_width - margin,
            screen_rect.max.y - y_offset - notification_height - 50.0,
        );

        let remaining = notification.remaining_time(current_time);
        let alpha = (remaining.min(1.0) * 255.0) as u8;

        let bg_color = match notification.notification_type {
            NotificationType::Info => egui::Color32::from_rgba_unmultiplied(50, 50, 50, alpha),
            NotificationType::Success => egui::Color32::from_rgba_unmultiplied(50, 100, 50, alpha),
            NotificationType::Warning => egui::Color32::from_rgba_unmultiplied(150, 100, 50, alpha),
            NotificationType::Error => egui::Color32::from_rgba_unmultiplied(150, 50, 50, alpha),
        };

        egui::Area::new(egui::Id::new(format!("notification_{}", i)))
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_min_width(notification_width - 16.0);
                        ui.label(
                            egui::RichText::new(&notification.message)
                                .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
                        );
                    });
            });
    }
}

/// Render keyboard help overlay
pub fn render_keyboard_help(
    mut egui_ctx: EguiContexts,
    panels: Res<PanelVisibility>,
) {
    if !panels.keyboard_help {
        return;
    }

    egui::Window::new("Keyboard Shortcuts")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Keyboard Shortcuts");
            ui.separator();

            ui.columns(2, |columns| {
                // Left column
                render_shortcut_section(&mut columns[0], "Simulation", &[
                    ("Space", "Play/Pause"),
                    ("N", "Step one tick"),
                    ("1-5", "Speed 1x-5x"),
                    ("0", "Speed 10x"),
                ]);

                columns[0].add_space(8.0);

                render_shortcut_section(&mut columns[0], "Map Navigation", &[
                    ("W/A/S/D", "Pan map"),
                    ("Arrow keys", "Pan map"),
                    ("Shift+Pan", "Pan faster"),
                    ("+/=", "Zoom in"),
                    ("-", "Zoom out"),
                    ("Home", "Reset view"),
                    ("Drag", "Pan (mouse)"),
                    ("Scroll", "Zoom (mouse)"),
                ]);

                columns[0].add_space(8.0);

                render_shortcut_section(&mut columns[0], "Selection", &[
                    ("Click", "Select entity"),
                    ("Tab", "Next agent"),
                    ("Shift+Tab", "Previous agent"),
                    ("C", "Center on selection"),
                    ("F", "Follow selected"),
                    ("Escape", "Deselect"),
                ]);

                // Right column
                render_shortcut_section(&mut columns[1], "Panels", &[
                    ("H", "Keyboard shortcuts"),
                    ("I", "Inspector"),
                    ("P", "Statistics"),
                    ("T", "Tech tree"),
                    ("Y", "Timeline"),
                    ("R", "Relationships"),
                    ("L", "Legend"),
                ]);

                columns[1].add_space(8.0);

                render_shortcut_section(&mut columns[1], "Map Display", &[
                    ("M", "Toggle minimap"),
                    ("G", "Toggle grid"),
                ]);

                columns[1].add_space(8.0);

                render_shortcut_section(&mut columns[1], "File Operations", &[
                    ("Ctrl+S", "Save game"),
                    ("Ctrl+O", "Load game"),
                    ("Ctrl+F", "Search"),
                ]);

                columns[1].add_space(8.0);

                render_shortcut_section(&mut columns[1], "Mouse Controls", &[
                    ("Left click", "Select"),
                    ("Drag", "Pan view"),
                    ("Scroll", "Zoom to cursor"),
                    ("Hover", "Show tooltip"),
                    ("Minimap click", "Navigate"),
                ]);
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Press H or Escape to close").small().color(egui::Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("EBSS Bevy GUI").small().color(egui::Color32::from_rgb(100, 100, 100)));
                });
            });
        });
}

fn render_shortcut_section(ui: &mut egui::Ui, title: &str, shortcuts: &[(&str, &str)]) {
    ui.label(egui::RichText::new(title).strong().color(egui::Color32::from_rgb(100, 180, 255)));

    egui::Grid::new(format!("shortcuts_{}", title))
        .num_columns(2)
        .spacing([12.0, 2.0])
        .show(ui, |ui| {
            for (key, description) in shortcuts {
                ui.label(egui::RichText::new(*key).monospace().color(egui::Color32::from_rgb(255, 200, 100)));
                ui.label(egui::RichText::new(*description).small());
                ui.end_row();
            }
        });
}

fn export_statistics_csv(stats_history: &StatisticsHistory) -> Result<String, std::io::Error> {
    use std::io::Write;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("ebss_statistics_{}.csv", timestamp);

    let mut file = std::fs::File::create(&filename)?;

    writeln!(file, "tick,population,infants,children,adolescents,adults,elderly,births,deaths,avg_health,avg_energy,avg_happiness,total_resources,buildings_completed,buildings_construction")?;

    for point in &stats_history.points {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{:.2},{:.2},{:.2},{},{},{}",
            point.tick,
            point.population,
            point.infants,
            point.children,
            point.adolescents,
            point.adults,
            point.elderly,
            point.births,
            point.deaths,
            point.average_health,
            point.average_energy,
            point.average_happiness,
            point.total_resources,
            point.buildings_completed,
            point.buildings_construction,
        )?;
    }

    Ok(filename)
}

fn export_timeline_csv(timeline: &TimelineData) -> Result<String, std::io::Error> {
    use std::io::Write;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("ebss_timeline_{}.csv", timestamp);

    let mut file = std::fs::File::create(&filename)?;

    writeln!(file, "tick,event_type,description,position_x,position_y")?;

    for event in &timeline.event_log {
        let description = event.short_description();
        let escaped_description = description.replace('"', "\"\"");
        let (pos_x, pos_y) = event.position.map(|(x, y)| (x.to_string(), y.to_string()))
            .unwrap_or(("".to_string(), "".to_string()));

        writeln!(
            file,
            "{},{:?},\"{}\",{},{}",
            event.tick,
            event.filter_type(),
            escaped_description,
            pos_x,
            pos_y,
        )?;
    }

    Ok(filename)
}
