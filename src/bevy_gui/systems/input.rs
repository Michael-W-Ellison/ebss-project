// src/bevy_gui/systems/input.rs
//! Input handling systems.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::bevy_gui::resources::*;
use crate::bevy_gui::events::SimulationCommand;

/// Handle global keyboard shortcuts
pub fn keyboard_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut egui_ctx: EguiContexts,
    mut sim_commands: EventWriter<SimulationCommand>,
    mut sim_control: ResMut<SimulationControl>,
    mut panels: ResMut<PanelVisibility>,
    mut map_view: ResMut<MapViewState>,
    mut selection: ResMut<Selection>,
    mut notifications: ResMut<NotificationQueue>,
    time: Res<Time>,
) {
    // Don't process shortcuts if egui wants keyboard input
    let ctx = egui_ctx.ctx_mut();
    if ctx.wants_keyboard_input() {
        return;
    }

    let current_time = time.elapsed_secs_f64();
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    // Escape - close dialogs or deselect
    if keys.just_pressed(KeyCode::Escape) {
        if panels.has_modal_open() {
            panels.close_dialogs();
        } else if panels.tech_tree {
            panels.tech_tree = false;
        } else if panels.timeline {
            panels.timeline = false;
        } else if panels.relationship_graph {
            panels.relationship_graph = false;
        } else {
            selection.deselect();
            sim_commands.send(SimulationCommand::DeselectAll);
        }
    }

    // Simulation controls
    if keys.just_pressed(KeyCode::Space) && !ctrl {
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

    if keys.just_pressed(KeyCode::KeyN) && !ctrl {
        sim_commands.send(SimulationCommand::Step);
    }

    // Speed controls
    let speed_keys = [
        (KeyCode::Digit1, 1.0),
        (KeyCode::Digit2, 2.0),
        (KeyCode::Digit3, 3.0),
        (KeyCode::Digit4, 4.0),
        (KeyCode::Digit5, 5.0),
        (KeyCode::Digit0, 10.0),
    ];
    for (key, speed) in speed_keys {
        if keys.just_pressed(key) && !ctrl {
            sim_control.set_speed(speed);
            sim_commands.send(SimulationCommand::SetSpeed(speed));
            notifications.info(format!("Speed: {}x", speed), current_time);
        }
    }

    // Panel toggles
    if keys.just_pressed(KeyCode::KeyH) && !ctrl {
        panels.toggle_keyboard_help();
    }
    if keys.just_pressed(KeyCode::KeyI) && !ctrl {
        panels.toggle_inspector();
    }
    if keys.just_pressed(KeyCode::KeyP) && !ctrl {
        panels.toggle_statistics();
    }
    if keys.just_pressed(KeyCode::KeyT) && !ctrl {
        panels.toggle_tech_tree();
    }
    if keys.just_pressed(KeyCode::KeyL) && !ctrl {
        panels.toggle_legend();
    }
    if keys.just_pressed(KeyCode::KeyM) && !ctrl {
        map_view.minimap.enabled = !map_view.minimap.enabled;
    }
    if keys.just_pressed(KeyCode::KeyY) && !ctrl {
        panels.toggle_timeline();
    }
    if keys.just_pressed(KeyCode::KeyR) && !ctrl {
        panels.toggle_relationship_graph();
    }

    // Map controls
    if keys.just_pressed(KeyCode::KeyG) && !ctrl {
        map_view.layers.grid = !map_view.layers.grid;
    }
    if keys.just_pressed(KeyCode::Home) {
        map_view.reset_view();
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        map_view.zoom_in();
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        map_view.zoom_out();
    }

    // Follow mode
    if keys.just_pressed(KeyCode::KeyF) && !ctrl {
        selection.toggle_follow();
    }

    // Ctrl shortcuts
    if ctrl {
        if keys.just_pressed(KeyCode::KeyF) {
            panels.search = true;
        }
        if keys.just_pressed(KeyCode::KeyS) {
            panels.save_dialog = true;
        }
        if keys.just_pressed(KeyCode::KeyO) {
            panels.load_dialog = true;
        }
    }
}

/// Handle map panning with arrow keys
pub fn map_pan_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut egui_ctx: EguiContexts,
    mut map_view: ResMut<MapViewState>,
) {
    let ctx = egui_ctx.ctx_mut();
    if ctx.wants_keyboard_input() {
        return;
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let speed = if shift { 40.0 } else { 20.0 };

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        map_view.pan(0.0, -speed);
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        map_view.pan(0.0, speed);
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        map_view.pan(-speed, 0.0);
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        map_view.pan(speed, 0.0);
    }
}
