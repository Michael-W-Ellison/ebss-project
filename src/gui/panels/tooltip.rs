// src/gui/panels/tooltip.rs
//! Tooltip rendering helpers for map entities.

use egui::{Ui, Color32, RichText};

use crate::agents::{LifeStage, Gender};
use crate::core::DriveType;
use crate::world::{TerrainType, ResourceType, BuildingType};
use crate::gui::state::AgentSnapshot;

/// Get color for health/energy values
pub fn vitals_color(value: f32) -> Color32 {
    if value >= 70.0 {
        Color32::from_rgb(100, 200, 100)
    } else if value >= 30.0 {
        Color32::from_rgb(200, 200, 100)
    } else {
        Color32::from_rgb(200, 100, 100)
    }
}

/// Get color for life stage
pub fn life_stage_color(stage: LifeStage) -> Color32 {
    match stage {
        LifeStage::Infant => Color32::from_rgb(255, 220, 100),
        LifeStage::Child => Color32::from_rgb(100, 220, 100),
        LifeStage::Adolescent => Color32::from_rgb(100, 180, 220),
        LifeStage::Adult => Color32::from_rgb(100, 140, 220),
        LifeStage::Elderly => Color32::from_rgb(180, 130, 200),
    }
}

/// Get color for drive type
pub fn drive_color(drive: DriveType) -> Color32 {
    match drive {
        DriveType::Hunger => Color32::from_rgb(255, 150, 100),
        DriveType::Thirst => Color32::from_rgb(100, 180, 255),
        DriveType::Rest => Color32::from_rgb(180, 130, 255),
        DriveType::Safety => Color32::from_rgb(255, 100, 100),
        DriveType::Social => Color32::from_rgb(255, 200, 100),
        DriveType::Shelter => Color32::from_rgb(200, 180, 150),
        DriveType::Preparedness => Color32::from_rgb(150, 180, 200),
        DriveType::Industry => Color32::from_rgb(200, 150, 100),
        DriveType::Sustenance => Color32::from_rgb(150, 220, 150),
        DriveType::Curiosity => Color32::from_rgb(220, 180, 255),
        DriveType::Reproduction => Color32::from_rgb(255, 150, 200),
        DriveType::Luxury => Color32::from_rgb(220, 200, 100),
        DriveType::Utility => Color32::from_rgb(180, 180, 180),
        DriveType::Construction => Color32::from_rgb(180, 140, 100),
        DriveType::Protection => Color32::from_rgb(255, 220, 130),
    }
}

/// Get terrain display name
pub fn terrain_name(terrain: TerrainType) -> &'static str {
    match terrain {
        TerrainType::Plains => "Plains",
        TerrainType::Forest => "Forest",
        TerrainType::Mountain => "Mountain",
        TerrainType::Water => "Water",
        TerrainType::Desert => "Desert",
        TerrainType::Wetland => "Wetland",
        TerrainType::Meadow => "Meadow",
        TerrainType::Hills => "Hills",
        TerrainType::Beach => "Beach",
        TerrainType::Riverbank => "Riverbank",
        TerrainType::Farmland => "Farmland",
    }
}

/// Get resource display name (uses Debug format but could be customized)
pub fn resource_name(resource: ResourceType) -> String {
    format!("{:?}", resource)
}

/// Get building display name (uses Debug format but could be customized)
pub fn building_name(building: BuildingType) -> String {
    format!("{:?}", building)
}

/// Render a section header in tooltip
pub fn tooltip_header(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).strong().color(Color32::WHITE));
}

/// Render a colored stat row
pub fn tooltip_stat(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(Color32::GRAY));
        ui.label(RichText::new(value).color(color));
    });
}

/// Render agent tooltip content
pub fn render_agent_tooltip(ui: &mut Ui, agent: &AgentSnapshot) {
    let stage_color = life_stage_color(agent.life_stage);

    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(8.0, 8.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, stage_color);
        ui.label(RichText::new(format!("{:?}", agent.life_stage)).strong());
        let gender_symbol = match agent.gender {
            Gender::Male => "\u{2642}",
            Gender::Female => "\u{2640}",
        };
        ui.label(RichText::new(gender_symbol).color(Color32::LIGHT_GRAY));
        ui.label(RichText::new(format!("{:.6}...", agent.id)).small().color(Color32::GRAY));
    });

    ui.horizontal(|ui| {
        ui.label("Health:");
        let health_color = vitals_color(agent.health);
        ui.label(RichText::new(format!("{:.0}%", agent.health)).color(health_color));
        ui.label(" | Energy:");
        let energy_color = vitals_color(agent.energy);
        ui.label(RichText::new(format!("{:.0}%", agent.energy)).color(energy_color));
    });

    if agent.is_sleeping {
        ui.label(RichText::new("Status: Sleeping").color(Color32::from_rgb(138, 43, 226)));
    } else {
        let fatigue_text = match agent.fatigue_severity {
            0 => "Well-rested",
            1 => "Slightly tired",
            2 => "Fatigued",
            _ => "Exhausted",
        };
        let fatigue_color = match agent.fatigue_severity {
            0 => Color32::from_rgb(100, 200, 100),
            1 => Color32::from_rgb(200, 200, 100),
            2 => Color32::from_rgb(255, 165, 0),
            _ => Color32::from_rgb(255, 69, 0),
        };
        tooltip_stat(ui, "Status:", fatigue_text, fatigue_color);
    }

    if let Some(drive) = agent.most_urgent_drive {
        let color = drive_color(drive);
        tooltip_stat(ui, "Drive:", &format!("{:?}", drive), color);
    }

    if let Some(activity) = &agent.current_activity {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Activity:").color(Color32::GRAY));
            ui.label(RichText::new(activity).color(Color32::from_rgb(180, 220, 255)));
        });
    }

    if agent.inventory_count > 0 || agent.relationship_count > 0 {
        ui.horizontal(|ui| {
            if agent.inventory_count > 0 {
                ui.label(RichText::new(format!("Items: {}", agent.inventory_count)).small().color(Color32::GRAY));
            }
            if agent.relationship_count > 0 {
                ui.label(RichText::new(format!("Relations: {}", agent.relationship_count)).small().color(Color32::GRAY));
            }
        });
    }
}

/// Render resource tooltip content
pub fn render_resource_tooltip(ui: &mut Ui, resource_type: ResourceType, amount: u32, max_amount: u32) {
    let name = resource_name(resource_type);
    let fill_pct = (amount as f32 / max_amount as f32) * 100.0;
    let fill_color = if fill_pct > 50.0 {
        Color32::from_rgb(100, 200, 100)
    } else if fill_pct > 20.0 {
        Color32::from_rgb(200, 200, 100)
    } else {
        Color32::from_rgb(200, 100, 100)
    };

    tooltip_header(ui, &name);
    ui.horizontal(|ui| {
        ui.label("Amount:");
        ui.label(RichText::new(format!("{}/{}", amount, max_amount)).color(fill_color));
        ui.label(RichText::new(format!("({}%)", fill_pct as u32)).small().color(Color32::GRAY));
    });
}

/// Render building tooltip content
pub fn render_building_tooltip(ui: &mut Ui, building_type: BuildingType, completed: bool, progress: f32) {
    let name = building_name(building_type);
    tooltip_header(ui, &name);

    if completed {
        ui.label(RichText::new("Status: Completed").color(Color32::from_rgb(100, 200, 100)));
    } else {
        ui.horizontal(|ui| {
            ui.label("Progress:");
            ui.label(RichText::new(format!("{:.0}%", progress * 100.0)).color(Color32::from_rgb(255, 200, 100)));
        });
    }
}

/// Render terrain tooltip header
pub fn render_terrain_header(ui: &mut Ui, x: i32, y: i32, terrain: Option<TerrainType>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("({}, {})", x, y)).strong());
        if let Some(t) = terrain {
            ui.label(RichText::new(terrain_name(t)).color(Color32::GRAY));
        }
    });
}
