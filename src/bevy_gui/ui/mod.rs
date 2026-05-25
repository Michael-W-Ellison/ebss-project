// src/bevy_gui/ui/mod.rs
//! UI rendering systems using bevy_egui.

pub mod map;
pub mod panels;
pub mod tooltip;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::bevy_gui::resources::*;
use crate::bevy_gui::events::SimulationCommand;

pub use panels::{
    render_legend_panel, render_inspector_panel, render_statistics_panel,
    render_tech_tree_panel, render_timeline_panel, render_relationship_graph_panel,
    render_save_dialog, render_load_dialog, render_search_panel, search_system,
};
pub use map::render_map;

/// Render the top menu bar
pub fn render_menu_bar(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut map_view: ResMut<MapViewState>,
    snapshot: Res<CurrentSnapshot>,
    sim_control: Res<SimulationControl>,
) {
    egui::TopBottomPanel::top("top_menu").show(egui_ctx.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save... (Ctrl+S)").clicked() {
                    panels.save_dialog = true;
                    ui.close_menu();
                }
                if ui.button("Load... (Ctrl+O)").clicked() {
                    panels.load_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Search... (Ctrl+F)").clicked() {
                    panels.search = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut panels.inspector, "Inspector Panel (I)");
                ui.checkbox(&mut panels.statistics, "Statistics Panel (P)");
                ui.checkbox(&mut panels.tech_tree, "Technology Tree (T)");
                ui.checkbox(&mut panels.timeline, "Timeline (Y)");
                ui.checkbox(&mut panels.relationship_graph, "Relationship Graph (R)");
                ui.checkbox(&mut panels.legend, "Legend (L)");
                ui.checkbox(&mut map_view.minimap.enabled, "Minimap (M)");
                ui.separator();
                ui.checkbox(&mut panels.keyboard_help, "Keyboard Shortcuts (H)");
            });

            ui.menu_button("Map", |ui| {
                ui.checkbox(&mut map_view.layers.terrain, "Show Terrain");
                ui.checkbox(&mut map_view.layers.resources, "Show Resources");
                ui.checkbox(&mut map_view.layers.buildings, "Show Buildings");
                ui.checkbox(&mut map_view.layers.agents, "Show Agents");
                ui.checkbox(&mut map_view.layers.grid, "Show Grid (G)");
                ui.separator();
                if ui.button("Reset View (Home)").clicked() {
                    map_view.reset_view();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard Shortcuts (H)").clicked() {
                    panels.keyboard_help = true;
                    ui.close_menu();
                }
            });

            // Status info on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status = match sim_control.state {
                    SimState::Running => "Running",
                    SimState::Paused => "Paused",
                    SimState::Stepping => "Stepping",
                };

                if let Some(snap) = &snapshot.snapshot {
                    let agent_count = snap.population.agents.iter().filter(|a| a.is_alive).count();
                    ui.label(format!(
                        "Tick: {} | Agents: {} | {}",
                        snap.tick, agent_count, status
                    ));
                } else {
                    ui.label(format!("Waiting... | {}", status));
                }
            });
        });
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
            // Play/Pause
            let play_text = if sim_control.is_running() { "⏸ Pause" } else { "▶ Play" };
            if ui.button(play_text).clicked() {
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

            // Step
            if ui.button("⏭ Step").clicked() {
                sim_commands.send(SimulationCommand::Step);
            }

            ui.separator();

            // Speed control
            ui.label("Speed:");
            let speeds = [0.5, 1.0, 2.0, 5.0, 10.0];
            for speed in speeds {
                let label = format!("{}x", speed);
                if ui.selectable_label((sim_control.speed - speed).abs() < 0.01, &label).clicked() {
                    sim_control.set_speed(speed);
                    sim_commands.send(SimulationCommand::SetSpeed(speed));
                }
            }

            ui.separator();

            ui.label(format!("Current: {:.1}x", sim_control.speed));
        });
    });
}

/// Render placeholder for map panel (will be expanded in Phase 3)
pub fn render_map_placeholder(
    mut egui_ctx: EguiContexts,
    snapshot: Res<CurrentSnapshot>,
) {
    egui::CentralPanel::default().show(egui_ctx.ctx_mut(), |ui| {
        if snapshot.snapshot.is_some() {
            ui.centered_and_justified(|ui| {
                ui.heading("Map View - Coming in Phase 3");
                ui.label("Simulation is running. Use the controls below.");
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.heading("Waiting for simulation data...");
            });
        }
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

            render_shortcut_section(ui, "Simulation Control", &[
                ("Space", "Play/Pause simulation"),
                ("N", "Step one tick"),
                ("1-5", "Set simulation speed (1x-5x)"),
                ("0", "Set speed to 10x"),
            ]);

            ui.add_space(10.0);

            render_shortcut_section(ui, "Map Navigation", &[
                ("W/Up", "Pan up"),
                ("S/Down", "Pan down"),
                ("A/Left", "Pan left"),
                ("D/Right", "Pan right"),
                ("Shift+WASD", "Pan faster"),
                ("+/=", "Zoom in"),
                ("-", "Zoom out"),
                ("Home", "Reset view"),
            ]);

            ui.add_space(10.0);

            render_shortcut_section(ui, "Panels", &[
                ("H", "Toggle this help"),
                ("I", "Toggle inspector"),
                ("P", "Toggle statistics"),
                ("T", "Toggle tech tree"),
                ("Y", "Toggle timeline"),
                ("R", "Toggle relationship graph"),
                ("L", "Toggle legend"),
                ("M", "Toggle minimap"),
                ("G", "Toggle grid"),
            ]);

            ui.add_space(10.0);

            render_shortcut_section(ui, "Other", &[
                ("F", "Toggle follow mode"),
                ("Escape", "Close dialog/deselect"),
                ("Ctrl+S", "Save"),
                ("Ctrl+O", "Load"),
                ("Ctrl+F", "Search"),
            ]);

            ui.separator();
            ui.label(egui::RichText::new("Press H or Escape to close").small().color(egui::Color32::GRAY));
        });
}

fn render_shortcut_section(ui: &mut egui::Ui, title: &str, shortcuts: &[(&str, &str)]) {
    ui.label(egui::RichText::new(title).strong().color(egui::Color32::from_rgb(100, 180, 255)));

    egui::Grid::new(title)
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            for (key, description) in shortcuts {
                ui.label(egui::RichText::new(*key).monospace().color(egui::Color32::from_rgb(255, 200, 100)));
                ui.label(*description);
                ui.end_row();
            }
        });
}
