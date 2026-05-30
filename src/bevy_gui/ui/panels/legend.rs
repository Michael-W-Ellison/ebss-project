// src/bevy_gui/ui/panels/legend.rs
//! Map legend panel showing symbol meanings.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::bevy_gui::resources::PanelVisibility;

pub fn render_legend_panel(
    mut egui_ctx: EguiContexts,
    panels: Res<PanelVisibility>,
) {
    if !panels.legend {
        return;
    }

    egui::SidePanel::right("legend_panel")
        .default_width(200.0)
        .resizable(true)
        .show(egui_ctx.ctx_mut(), |ui| {
            render_legend_content(ui);
        });
}

fn render_legend_content(ui: &mut egui::Ui) {
    ui.heading("Map Legend");
    ui.separator();

    ui.collapsing("Agents (by Life Stage)", |ui| {
        legend_item(ui, egui::Color32::from_rgb(255, 182, 193), "Infant");
        legend_item(ui, egui::Color32::from_rgb(135, 206, 250), "Child");
        legend_item(ui, egui::Color32::from_rgb(144, 238, 144), "Adolescent");
        legend_item(ui, egui::Color32::WHITE, "Adult");
        legend_item(ui, egui::Color32::from_rgb(192, 192, 192), "Elderly");
    });

    ui.add_space(5.0);

    ui.collapsing("Agent Status", |ui| {
        ui.label("Health Indicators:");
        ui.label("  - Red ring: Critical (<25%)");
        ui.label("  - Yellow ring: Low (<50%)");
        ui.label("  - No ring: Healthy (>50%)");
        ui.add_space(5.0);
        ui.label("Sleep Indicators:");
        ui.label("  - Zzz icon: Agent is sleeping");
        ui.label("  - Blue glow: Fatigued");
        ui.add_space(5.0);
        ui.label("Selection:");
        ui.label("  - White ring: Selected");
    });

    ui.add_space(5.0);

    ui.collapsing("Terrain", |ui| {
        legend_item(ui, egui::Color32::from_rgb(144, 238, 144), "Plains");
        legend_item(ui, egui::Color32::from_rgb(124, 252, 0), "Meadow");
        legend_item(ui, egui::Color32::from_rgb(34, 139, 34), "Forest");
        legend_item(ui, egui::Color32::from_rgb(139, 137, 112), "Hills");
        legend_item(ui, egui::Color32::from_rgb(128, 128, 128), "Mountain");
        legend_item(ui, egui::Color32::from_rgb(65, 105, 225), "Water");
        legend_item(ui, egui::Color32::from_rgb(238, 203, 173), "Desert");
        legend_item(ui, egui::Color32::from_rgb(85, 107, 47), "Wetland");
        legend_item(ui, egui::Color32::from_rgb(238, 214, 175), "Beach");
        legend_item(ui, egui::Color32::from_rgb(107, 142, 35), "Riverbank");
    });

    ui.add_space(5.0);

    ui.collapsing("Resources", |ui| {
        legend_item(ui, egui::Color32::from_rgb(139, 69, 19), "Wood");
        legend_item(ui, egui::Color32::from_rgb(169, 169, 169), "Stone");
        legend_item(ui, egui::Color32::from_rgb(112, 128, 144), "Iron");
        legend_item(ui, egui::Color32::from_rgb(255, 99, 71), "Food");
        legend_item(ui, egui::Color32::from_rgb(0, 191, 255), "Water");
        legend_item(ui, egui::Color32::from_rgb(47, 79, 79), "Coal");
        legend_item(ui, egui::Color32::from_rgb(255, 215, 0), "Grain");
        legend_item(ui, egui::Color32::from_rgb(0, 128, 0), "Herbs");
    });

    ui.add_space(5.0);

    ui.collapsing("Buildings", |ui| {
        legend_item(ui, egui::Color32::from_rgb(139, 90, 43), "Completed");
        legend_item(ui, egui::Color32::from_rgb(180, 150, 100), "Under Construction");
    });

    ui.add_space(10.0);

    ui.collapsing("Controls", |ui| {
        ui.label("Mouse:");
        ui.label("  - Click: Select entity");
        ui.label("  - Scroll: Zoom in/out");
        ui.label("  - Drag: Pan map");
        ui.label("  - Click minimap: Jump to location");
        ui.add_space(5.0);
        ui.label("Keyboard:");
        ui.label("  - Space: Play/Pause");
        ui.label("  - N: Step one tick");
        ui.label("  - Arrows/WASD: Pan map");
        ui.label("  - +/-: Zoom in/out");
        ui.label("  - F: Toggle follow mode");
        ui.label("  - G: Toggle grid");
        ui.label("  - Home: Reset view");
        ui.label("  - Escape: Deselect");
    });
}

fn legend_item(ui: &mut egui::Ui, color: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, color);
        ui.label(label);
    });
}
