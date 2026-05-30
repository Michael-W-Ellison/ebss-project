// src/gui/panels/save_load.rs
//! Save and Load dialogs for simulation state.

use egui::{Ui, Color32, RichText, ScrollArea};
use crate::gui::state::{GuiState, SaveFileInfo, NotificationType};

pub fn render_save_dialog(ui: &mut Ui, state: &mut GuiState, current_time: f64) {
    ui.heading("Save Simulation");
    ui.separator();

    // Filename input
    ui.horizontal(|ui| {
        ui.label("Filename:");
        ui.text_edit_singleline(&mut state.save_load_state.filename);
        ui.label(".ebss");
    });

    // Directory
    ui.horizontal(|ui| {
        ui.label("Directory:");
        ui.text_edit_singleline(&mut state.save_load_state.save_directory);
    });

    // Current simulation info
    if let Some(snapshot) = &state.latest_snapshot {
        ui.add_space(10.0);
        ui.label(RichText::new("Current State:").strong());
        ui.label(format!("Tick: {}", snapshot.tick));
        ui.label(format!("Agents: {}", snapshot.population.agents.iter().filter(|a| a.is_alive).count()));
        ui.label(format!("Buildings: {}", snapshot.world.buildings.len()));
    }

    // Error/success messages
    if let Some(err) = &state.save_load_state.last_error {
        ui.add_space(5.0);
        ui.colored_label(Color32::RED, err);
    }
    if let Some(success) = &state.save_load_state.last_success {
        ui.add_space(5.0);
        ui.colored_label(Color32::GREEN, success);
    }

    ui.add_space(15.0);
    ui.separator();

    // Action buttons
    let mut save_clicked = false;
    let mut close_clicked = false;

    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            save_clicked = true;
        }
        if ui.button("Cancel").clicked() {
            close_clicked = true;
        }
    });

    if save_clicked {
        if state.save_load_state.filename.is_empty() {
            state.save_load_state.last_error = Some("Please enter a filename".to_string());
        } else {
            state.save_load_state.last_error = None;
            state.save_load_state.last_success = Some(format!(
                "Ready to save as '{}.ebss' (save will be performed by simulation thread)",
                state.save_load_state.filename
            ));
            state.notify("Save requested", NotificationType::Info, current_time);
        }
    }

    if close_clicked {
        state.show_save_dialog = false;
        state.save_load_state.last_error = None;
        state.save_load_state.last_success = None;
    }
}

pub fn render_load_dialog(ui: &mut Ui, state: &mut GuiState, current_time: f64) {
    ui.heading("Load Simulation");
    ui.separator();

    // Directory
    ui.horizontal(|ui| {
        ui.label("Directory:");
        ui.text_edit_singleline(&mut state.save_load_state.save_directory);
        if ui.button("Refresh").clicked() {
            refresh_save_list(state);
        }
    });

    ui.add_space(10.0);

    // Available saves list
    ui.label(RichText::new("Available Saves:").strong());

    let available_height = ui.available_height() - 80.0;

    ScrollArea::vertical()
        .max_height(available_height.max(100.0))
        .show(ui, |ui| {
            if state.save_load_state.available_saves.is_empty() {
                ui.label(RichText::new("No save files found").color(Color32::GRAY));
                ui.label(RichText::new("Click 'Refresh' to scan for saves").small().color(Color32::GRAY));
            } else {
                let selected_idx = state.save_load_state.selected_save;
                let mut clicked_idx = None;

                for (idx, save) in state.save_load_state.available_saves.iter().enumerate() {
                    let is_selected = selected_idx == Some(idx);

                    let label_text = format!(
                        "{} | Tick: {} | Agents: {} | {}",
                        save.filename, save.tick, save.agent_count, save.modified
                    );

                    let response = ui.selectable_label(is_selected, &label_text);

                    if response.clicked() {
                        clicked_idx = Some(idx);
                    }
                }

                if let Some(idx) = clicked_idx {
                    state.save_load_state.selected_save = Some(idx);
                }
            }
        });

    // Error/success messages
    if let Some(err) = &state.save_load_state.last_error {
        ui.add_space(5.0);
        ui.colored_label(Color32::RED, err);
    }

    ui.add_space(10.0);
    ui.separator();

    // Action buttons
    let mut load_clicked = false;
    let mut close_clicked = false;

    ui.horizontal(|ui| {
        let can_load = state.save_load_state.selected_save.is_some();
        if ui.add_enabled(can_load, egui::Button::new("Load")).clicked() {
            load_clicked = true;
        }
        if ui.button("Cancel").clicked() {
            close_clicked = true;
        }
    });

    if load_clicked {
        if let Some(idx) = state.save_load_state.selected_save {
            if let Some(save) = state.save_load_state.available_saves.get(idx) {
                state.notify(
                    format!("Loading '{}' (load will be performed by simulation thread)", save.filename),
                    NotificationType::Info,
                    current_time,
                );
            }
        }
    }

    if close_clicked {
        state.show_load_dialog = false;
        state.save_load_state.last_error = None;
    }
}

fn refresh_save_list(state: &mut GuiState) {
    state.save_load_state.available_saves.clear();

    let dir = if state.save_load_state.save_directory.is_empty() {
        "./saves"
    } else {
        &state.save_load_state.save_directory
    };

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "ebss") {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let modified = entry.metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| {
                            let secs = d.as_secs();
                            let days = secs / 86400;
                            let hours = (secs % 86400) / 3600;
                            let mins = (secs % 3600) / 60;
                            if days > 0 {
                                format!("{}d ago", days)
                            } else if hours > 0 {
                                format!("{}h ago", hours)
                            } else {
                                format!("{}m ago", mins)
                            }
                        })
                        .unwrap_or_else(|| "Unknown".to_string());

                    state.save_load_state.available_saves.push(SaveFileInfo {
                        filename: filename.to_string(),
                        path: path.to_string_lossy().to_string(),
                        tick: 0,
                        agent_count: 0,
                        modified,
                    });
                }
            }
        }
    }

    state.save_load_state.selected_save = None;
}

pub fn render_notifications(ui: &mut Ui, state: &mut GuiState, current_time: f64) {
    state.update_notifications(current_time);

    if state.notifications.is_empty() {
        return;
    }

    let screen_rect = ui.ctx().screen_rect();
    let notification_width = 300.0;
    let notification_height = 40.0;
    let padding = 10.0;

    for (idx, notification) in state.notifications.iter().enumerate() {
        let age = current_time - notification.created_at;
        let fade = if age > notification.duration - 0.5 {
            ((notification.duration - age) / 0.5).clamp(0.0, 1.0) as f32
        } else if age < 0.3 {
            (age / 0.3) as f32
        } else {
            1.0
        };

        let y_offset = idx as f32 * (notification_height + padding);

        let rect = egui::Rect::from_min_size(
            egui::Pos2::new(
                screen_rect.max.x - notification_width - padding,
                screen_rect.min.y + padding + y_offset,
            ),
            egui::Vec2::new(notification_width, notification_height),
        );

        let bg_color = match notification.notification_type {
            NotificationType::Info => Color32::from_rgba_unmultiplied(50, 50, 80, (200.0 * fade) as u8),
            NotificationType::Success => Color32::from_rgba_unmultiplied(30, 80, 30, (200.0 * fade) as u8),
            NotificationType::Warning => Color32::from_rgba_unmultiplied(80, 70, 20, (200.0 * fade) as u8),
            NotificationType::Error => Color32::from_rgba_unmultiplied(100, 30, 30, (200.0 * fade) as u8),
        };

        let border_color = match notification.notification_type {
            NotificationType::Info => Color32::from_rgba_unmultiplied(100, 100, 200, (200.0 * fade) as u8),
            NotificationType::Success => Color32::from_rgba_unmultiplied(100, 200, 100, (200.0 * fade) as u8),
            NotificationType::Warning => Color32::from_rgba_unmultiplied(200, 180, 50, (200.0 * fade) as u8),
            NotificationType::Error => Color32::from_rgba_unmultiplied(200, 100, 100, (200.0 * fade) as u8),
        };

        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, bg_color);
        painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border_color));

        let text_color = Color32::from_rgba_unmultiplied(255, 255, 255, (255.0 * fade) as u8);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            &notification.message,
            egui::FontId::default(),
            text_color,
        );
    }
}
