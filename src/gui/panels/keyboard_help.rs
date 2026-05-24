// src/gui/panels/keyboard_help.rs
//! Keyboard shortcuts help overlay.

use egui::{Ui, Color32, RichText};

pub fn render_keyboard_help(ui: &mut Ui) {
    ui.heading("Keyboard Shortcuts");
    ui.separator();

    render_section(ui, "Simulation Control", &[
        ("Space", "Play/Pause simulation"),
        ("N", "Step one tick"),
        ("1-5", "Set simulation speed (1x-5x)"),
        ("0", "Set speed to 10x"),
    ]);

    ui.add_space(10.0);

    render_section(ui, "Map Navigation", &[
        ("W/Up", "Pan up"),
        ("S/Down", "Pan down"),
        ("A/Left", "Pan left"),
        ("D/Right", "Pan right"),
        ("Shift+WASD", "Pan faster"),
        ("+/=", "Zoom in"),
        ("-", "Zoom out"),
        ("Home", "Reset view"),
        ("Scroll", "Zoom at cursor"),
    ]);

    ui.add_space(10.0);

    render_section(ui, "Selection", &[
        ("Click", "Select entity"),
        ("C", "Center on selection"),
        ("F", "Toggle follow mode"),
        ("Tab", "Next entity"),
        ("Shift+Tab", "Previous entity"),
        ("Escape", "Deselect / Close dialogs"),
    ]);

    ui.add_space(10.0);

    render_section(ui, "Panels & Dialogs", &[
        ("H", "Toggle this help"),
        ("I", "Toggle inspector panel"),
        ("P", "Toggle statistics panel"),
        ("T", "Toggle tech tree"),
        ("Y", "Toggle timeline"),
        ("L", "Toggle legend"),
        ("M", "Toggle minimap"),
        ("G", "Toggle grid overlay"),
        ("Ctrl+F", "Open search"),
        ("Ctrl+S", "Save simulation"),
        ("Ctrl+O", "Load simulation"),
    ]);

    ui.add_space(15.0);
    ui.separator();
    ui.label(RichText::new("Press H or Escape to close this help").small().color(Color32::GRAY));
}

fn render_section(ui: &mut Ui, title: &str, shortcuts: &[(&str, &str)]) {
    ui.label(RichText::new(title).strong().color(Color32::from_rgb(100, 180, 255)));

    egui::Grid::new(title)
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            for (key, description) in shortcuts {
                ui.label(RichText::new(*key).monospace().color(Color32::from_rgb(255, 200, 100)));
                ui.label(*description);
                ui.end_row();
            }
        });
}
