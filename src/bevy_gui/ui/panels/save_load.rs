// src/bevy_gui/ui/panels/save_load.rs
//! Save and Load dialog panels for simulation state.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Color32, RichText};

use crate::bevy_gui::resources::{
    PanelVisibility, SaveLoadState, CurrentSnapshot, NotificationQueue,
};
use crate::bevy_gui::events::SimulationCommand;

/// Render the save dialog window
pub fn render_save_dialog(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut save_state: ResMut<SaveLoadState>,
    snapshot: Res<CurrentSnapshot>,
    mut notifications: ResMut<NotificationQueue>,
    mut sim_commands: EventWriter<SimulationCommand>,
    time: Res<Time>,
) {
    if !panels.save_dialog {
        return;
    }

    let current_time = time.elapsed_secs_f64();
    let mut close_dialog = false;

    egui::Window::new("Save Simulation")
        .collapsible(false)
        .resizable(false)
        .default_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Save Simulation");
            ui.separator();

            // Filename input
            ui.horizontal(|ui| {
                ui.label("Filename:");
                ui.text_edit_singleline(&mut save_state.filename);
                ui.label(".ebss");
            });

            // Directory
            ui.horizontal(|ui| {
                ui.label("Directory:");
                ui.text_edit_singleline(&mut save_state.save_directory);
            });

            // Current simulation info
            if let Some(snap) = &snapshot.snapshot {
                ui.add_space(10.0);
                ui.label(RichText::new("Current State:").strong());
                ui.label(format!("Tick: {}", snap.tick));
                ui.label(format!(
                    "Agents: {}",
                    snap.population.agents.iter().filter(|a| a.is_alive).count()
                ));
                ui.label(format!("Buildings: {}", snap.world.buildings.len()));
            }

            // Error/success messages
            if let Some(err) = &save_state.last_error {
                ui.add_space(5.0);
                ui.colored_label(Color32::RED, err);
            }
            if let Some(success) = &save_state.last_success {
                ui.add_space(5.0);
                ui.colored_label(Color32::GREEN, success);
            }

            ui.add_space(15.0);
            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if save_state.filename.is_empty() {
                        save_state.set_error("Please enter a filename");
                    } else {
                        let path = save_state.get_save_path();

                        // Ensure directory exists
                        let dir = if save_state.save_directory.is_empty() {
                            "./saves"
                        } else {
                            &save_state.save_directory
                        };
                        if let Err(e) = std::fs::create_dir_all(dir) {
                            save_state.set_error(format!("Failed to create directory: {}", e));
                        } else {
                            sim_commands.send(SimulationCommand::SaveGame(path.clone()));
                            notifications.success(
                                &format!("Saving to {}", path),
                                current_time,
                            );
                            save_state.set_success(format!("Save requested: {}", path));
                            close_dialog = true;
                        }
                    }
                }

                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        panels.save_dialog = false;
        save_state.clear_messages();
    }
}

/// Render the load dialog window
pub fn render_load_dialog(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut save_state: ResMut<SaveLoadState>,
    mut notifications: ResMut<NotificationQueue>,
    mut sim_commands: EventWriter<SimulationCommand>,
    time: Res<Time>,
) {
    if !panels.load_dialog {
        return;
    }

    let current_time = time.elapsed_secs_f64();
    let mut close_dialog = false;

    // Refresh saves list when dialog opens (check if empty)
    if save_state.available_saves.is_empty() && save_state.last_error.is_none() {
        save_state.refresh_saves();
    }

    egui::Window::new("Load Simulation")
        .collapsible(false)
        .resizable(true)
        .default_size([450.0, 400.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Load Simulation");
            ui.separator();

            // Directory with refresh button
            ui.horizontal(|ui| {
                ui.label("Directory:");
                let dir_changed = ui.text_edit_singleline(&mut save_state.save_directory).changed();
                if ui.button("Refresh").clicked() || dir_changed {
                    save_state.refresh_saves();
                }
            });

            ui.add_space(10.0);
            ui.label(RichText::new("Available Saves:").strong());

            // Available saves list
            let available_height = ui.available_height() - 80.0;

            egui::ScrollArea::vertical()
                .max_height(available_height.max(100.0))
                .show(ui, |ui| {
                    if save_state.available_saves.is_empty() {
                        ui.label(RichText::new("No save files found").color(Color32::GRAY));
                        ui.label(
                            RichText::new("Click 'Refresh' to scan for saves")
                                .small()
                                .color(Color32::GRAY),
                        );
                    } else {
                        let mut clicked_idx = None;

                        for (idx, save) in save_state.available_saves.iter().enumerate() {
                            let is_selected = save_state.selected_save == Some(idx);

                            let label_text = format!(
                                "{} | {}",
                                save.filename, save.modified
                            );

                            let response = ui.selectable_label(is_selected, &label_text);

                            if response.clicked() {
                                clicked_idx = Some(idx);
                            }
                        }

                        if let Some(idx) = clicked_idx {
                            save_state.selected_save = Some(idx);
                        }
                    }
                });

            // Error messages
            if let Some(err) = &save_state.last_error {
                ui.add_space(5.0);
                ui.colored_label(Color32::RED, err);
            }

            ui.add_space(10.0);
            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                let can_load = save_state.selected_save.is_some();

                if ui.add_enabled(can_load, egui::Button::new("Load")).clicked() {
                    if let Some(save) = save_state.get_selected_save() {
                        let path = save.path.clone();
                        sim_commands.send(SimulationCommand::LoadGame(path.clone()));
                        notifications.info(
                            &format!("Loading from {}", save.filename),
                            current_time,
                        );
                        close_dialog = true;
                    }
                }

                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        panels.load_dialog = false;
        save_state.clear_messages();
    }
}
