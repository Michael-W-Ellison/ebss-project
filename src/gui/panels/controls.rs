// src/gui/panels/controls.rs
//! Simulation control panel (play/pause/step/speed).

use egui::Ui;
use std::sync::mpsc::Sender;
use crate::gui::state::{GuiState, SimState, SimulationCommand};

pub fn render_controls(ui: &mut Ui, state: &GuiState, command_tx: &Sender<SimulationCommand>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        // Play/Pause button
        let (icon, tooltip) = match state.simulation_state {
            SimState::Running => ("\u{23F8}", "Pause simulation"),  // ⏸
            SimState::Paused | SimState::Stepping => ("\u{25B6}", "Start simulation"),  // ▶
        };

        if ui.button(icon).on_hover_text(tooltip).clicked() {
            match state.simulation_state {
                SimState::Running => {
                    let _ = command_tx.send(SimulationCommand::Pause);
                }
                SimState::Paused | SimState::Stepping => {
                    let _ = command_tx.send(SimulationCommand::Play);
                }
            }
        }

        // Step button (only when paused)
        ui.add_enabled_ui(state.simulation_state != SimState::Running, |ui| {
            if ui.button("\u{23ED}").on_hover_text("Step one tick").clicked() {  // ⏭
                let _ = command_tx.send(SimulationCommand::Step);
            }
        });

        ui.separator();

        // Speed slider
        ui.label("Speed:");
        let mut speed = state.speed;
        let speed_slider = egui::Slider::new(&mut speed, 0.1..=10.0)
            .logarithmic(true)
            .suffix("x");

        if ui.add(speed_slider).changed() {
            let _ = command_tx.send(SimulationCommand::SetSpeed(speed));
        }

        // Speed presets
        for (label, preset_speed) in [("0.5x", 0.5), ("1x", 1.0), ("2x", 2.0), ("5x", 5.0)] {
            if ui.small_button(label).clicked() {
                let _ = command_tx.send(SimulationCommand::SetSpeed(preset_speed));
            }
        }

        ui.separator();

        // Tick counter
        if let Some(snapshot) = &state.latest_snapshot {
            ui.label(format!("Tick: {}", snapshot.tick));

            let (days, hours, minutes) =
                crate::environment::seasons::what_the_clock_says(snapshot.tick);
            ui.label(format!("Day {}, {:02}:{:02}", days + 1, hours, minutes));
        }

        ui.separator();

        // Status indicator
        let (status_color, status_text) = match state.simulation_state {
            SimState::Running => (egui::Color32::GREEN, "Running"),
            SimState::Paused => (egui::Color32::YELLOW, "Paused"),
            SimState::Stepping => (egui::Color32::LIGHT_BLUE, "Stepping"),
        };

        ui.colored_label(status_color, format!("\u{25CF} {}", status_text));  // ●
    });
}
