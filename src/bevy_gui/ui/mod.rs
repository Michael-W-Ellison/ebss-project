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
                if ui.add(egui::Button::new("Quit").shortcut_text("Alt+F4"))
                    .on_hover_text("Exit the application")
                    .clicked()
                {
                    std::process::exit(0);
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.add(egui::Button::new("Search...").shortcut_text("Ctrl+F"))
                    .on_hover_text("Search for agents, buildings, or resources")
                    .clicked()
                {
                    panels.search = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.label(egui::RichText::new("Panels").small().color(egui::Color32::GRAY));
                menu_checkbox_with_tooltip(ui, &mut panels.inspector, "Inspector", "I", "View detailed information about selected entities");
                menu_checkbox_with_tooltip(ui, &mut panels.statistics, "Statistics", "P", "Population graphs and world statistics");
                menu_checkbox_with_tooltip(ui, &mut panels.tech_tree, "Technology Tree", "T", "View discovered and available technologies");
                menu_checkbox_with_tooltip(ui, &mut panels.timeline, "Timeline", "Y", "Historical events and milestones");
                menu_checkbox_with_tooltip(ui, &mut panels.relationship_graph, "Relationships", "R", "Social network visualization");
                menu_checkbox_with_tooltip(ui, &mut panels.legend, "Legend", "L", "Map symbol and color reference");
                ui.separator();
                ui.label(egui::RichText::new("Map").small().color(egui::Color32::GRAY));
                menu_checkbox_with_tooltip(ui, &mut map_view.minimap.enabled, "Minimap", "M", "Show corner minimap for navigation");
                ui.separator();
                menu_checkbox_with_tooltip(ui, &mut panels.keyboard_help, "Keyboard Shortcuts", "H", "Show all keyboard shortcuts");
            });

            ui.menu_button("Map", |ui| {
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
                ui.label(egui::RichText::new("Display").small().color(egui::Color32::GRAY));
                menu_checkbox_with_tooltip(ui, &mut map_view.layers.grid, "Grid Overlay", "G", "Show tile grid lines");
                ui.separator();
                if ui.add(egui::Button::new("Reset View").shortcut_text("Home"))
                    .on_hover_text("Reset zoom and pan to default")
                    .clicked()
                {
                    map_view.reset_view();
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.add(egui::Button::new("Keyboard Shortcuts").shortcut_text("H"))
                    .on_hover_text("Show all available keyboard shortcuts")
                    .clicked()
                {
                    panels.keyboard_help = true;
                    ui.close_menu();
                }
                ui.separator();
                ui.label(egui::RichText::new("About").small().color(egui::Color32::GRAY));
                ui.label("EBSS - Entity-Based Social Simulation");
                ui.label(egui::RichText::new("Bevy GUI v0.1.0").small().color(egui::Color32::GRAY));
            });

            // Status info on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Status indicator with color
                let (status_text, status_color) = match sim_control.state {
                    SimState::Running => ("Running", egui::Color32::from_rgb(100, 200, 100)),
                    SimState::Paused => ("Paused", egui::Color32::from_rgb(200, 200, 100)),
                    SimState::Stepping => ("Step", egui::Color32::from_rgb(100, 180, 255)),
                };

                if let Some(snap) = &snapshot.snapshot {
                    let alive_count = snap.population.agents.iter().filter(|a| a.is_alive).count();
                    let total_count = snap.population.agents.len();

                    // Format time display
                    let days = snap.tick / 1440;
                    let hours = (snap.tick % 1440) / 60;
                    let minutes = snap.tick % 60;

                    ui.label(egui::RichText::new(status_text).color(status_color).strong());
                    ui.separator();
                    ui.label(format!("Day {} {:02}:{:02}", days + 1, hours, minutes))
                        .on_hover_text(format!("Tick: {}", snap.tick));
                    ui.separator();
                    ui.label(format!("{}/{}", alive_count, total_count))
                        .on_hover_text(format!("{} alive of {} total agents", alive_count, total_count));
                    ui.separator();
                    ui.label(format!("{:.1}x", sim_control.speed))
                        .on_hover_text("Simulation speed multiplier");
                } else {
                    ui.label(egui::RichText::new("Connecting...").color(egui::Color32::GRAY));
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
                .clamp_to_range(true)
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
