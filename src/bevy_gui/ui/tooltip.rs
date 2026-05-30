// src/bevy_gui/ui/tooltip.rs
//! Tooltip rendering helpers for map entities.

use bevy_egui::egui::{self, Color32, RichText, Ui};

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

/// Get color for life stage (map version - different palette)
pub fn life_stage_map_color(stage: LifeStage) -> Color32 {
    match stage {
        LifeStage::Infant => Color32::from_rgb(255, 182, 193),
        LifeStage::Child => Color32::from_rgb(135, 206, 250),
        LifeStage::Adolescent => Color32::from_rgb(144, 238, 144),
        LifeStage::Adult => Color32::from_rgb(255, 255, 255),
        LifeStage::Elderly => Color32::from_rgb(192, 192, 192),
    }
}

/// Get color for drive type
pub fn drive_color(drive: DriveType) -> Color32 {
    match drive {
        DriveType::Hunger => Color32::from_rgb(255, 140, 0),
        DriveType::Thirst => Color32::from_rgb(0, 191, 255),
        DriveType::Rest => Color32::from_rgb(138, 43, 226),
        DriveType::Safety => Color32::from_rgb(255, 0, 0),
        DriveType::Social => Color32::from_rgb(255, 105, 180),
        DriveType::Shelter => Color32::from_rgb(139, 90, 43),
        DriveType::Curiosity => Color32::from_rgb(255, 255, 0),
        DriveType::Preparedness => Color32::from_rgb(218, 165, 32),
        DriveType::Industry => Color32::from_rgb(169, 169, 169),
        DriveType::Sustenance => Color32::from_rgb(144, 238, 144),
        DriveType::Reproduction => Color32::from_rgb(255, 182, 193),
        DriveType::Luxury => Color32::from_rgb(230, 230, 250),
        DriveType::Utility => Color32::from_rgb(192, 192, 192),
        DriveType::Construction => Color32::from_rgb(139, 69, 19),
    }
}

/// Get terrain color for map rendering
pub fn terrain_color(terrain: TerrainType) -> Color32 {
    match terrain {
        TerrainType::Plains => Color32::from_rgb(144, 238, 144),
        TerrainType::Meadow => Color32::from_rgb(124, 252, 0),
        TerrainType::Forest => Color32::from_rgb(34, 139, 34),
        TerrainType::Hills => Color32::from_rgb(139, 137, 112),
        TerrainType::Mountain => Color32::from_rgb(128, 128, 128),
        TerrainType::Water => Color32::from_rgb(65, 105, 225),
        TerrainType::Desert => Color32::from_rgb(238, 203, 173),
        TerrainType::Wetland => Color32::from_rgb(85, 107, 47),
        TerrainType::Beach => Color32::from_rgb(238, 214, 175),
        TerrainType::Riverbank => Color32::from_rgb(107, 142, 35),
    }
}

/// Get resource color for map rendering
pub fn resource_color(resource_type: ResourceType) -> Color32 {
    match resource_type {
        ResourceType::Wood => Color32::from_rgb(139, 69, 19),
        ResourceType::Stone => Color32::from_rgb(169, 169, 169),
        ResourceType::Iron => Color32::from_rgb(112, 128, 144),
        ResourceType::Food => Color32::from_rgb(255, 99, 71),
        ResourceType::Water => Color32::from_rgb(0, 191, 255),
        ResourceType::Grain => Color32::from_rgb(255, 215, 0),
        ResourceType::Flax => Color32::from_rgb(245, 245, 220),
        ResourceType::Herbs => Color32::from_rgb(0, 128, 0),
        ResourceType::Cotton => Color32::from_rgb(255, 250, 250),
        ResourceType::Hides => Color32::from_rgb(139, 90, 43),
        ResourceType::Wool => Color32::from_rgb(255, 250, 240),
        ResourceType::Meat => Color32::from_rgb(205, 92, 92),
        ResourceType::Milk => Color32::from_rgb(255, 255, 240),
        ResourceType::Fish => Color32::from_rgb(70, 130, 180),
        ResourceType::Honey => Color32::from_rgb(255, 185, 15),
        ResourceType::Clay => Color32::from_rgb(205, 133, 63),
        ResourceType::Sand => Color32::from_rgb(238, 214, 175),
        ResourceType::Coal => Color32::from_rgb(47, 79, 79),
        ResourceType::Flour => Color32::from_rgb(255, 248, 220),
        ResourceType::Leather => Color32::from_rgb(139, 69, 19),
        ResourceType::Cloth => Color32::from_rgb(186, 85, 211),
        ResourceType::Linen => Color32::from_rgb(245, 245, 220),
        ResourceType::Glass => Color32::from_rgb(200, 225, 255),
        ResourceType::Bricks => Color32::from_rgb(178, 34, 34),
        ResourceType::Charcoal => Color32::from_rgb(54, 54, 54),
        ResourceType::Rope => Color32::from_rgb(193, 154, 107),
        ResourceType::Paper => Color32::from_rgb(255, 255, 240),
        ResourceType::Dye => Color32::from_rgb(148, 0, 211),
        ResourceType::Bread => Color32::from_rgb(222, 184, 135),
        ResourceType::Ale => Color32::from_rgb(210, 105, 30),
        ResourceType::Cheese => Color32::from_rgb(255, 215, 0),
        ResourceType::Clothing => Color32::from_rgb(147, 112, 219),
        ResourceType::Shoes => Color32::from_rgb(101, 67, 33),
        ResourceType::Tools => Color32::from_rgb(169, 169, 169),
        ResourceType::Weapons => Color32::from_rgb(192, 192, 192),
        ResourceType::Armor => Color32::from_rgb(128, 128, 128),
        ResourceType::Pottery => Color32::from_rgb(205, 92, 0),
        ResourceType::Furniture => Color32::from_rgb(160, 82, 45),
        ResourceType::Jewelry => Color32::from_rgb(255, 215, 0),
    }
}

/// Get building color for map rendering
pub fn building_color(building_type: BuildingType, completed: bool) -> Color32 {
    if !completed {
        return Color32::from_rgb(180, 150, 100);
    }
    match building_type {
        BuildingType::Longhouse => Color32::from_rgb(139, 90, 43),
        BuildingType::UpgradedLonghouse => Color32::from_rgb(160, 82, 45),
        BuildingType::SmallHouse => Color32::from_rgb(160, 82, 45),
        BuildingType::MediumHouse => Color32::from_rgb(139, 90, 43),
        BuildingType::LargeHouse => Color32::from_rgb(139, 69, 19),
        BuildingType::Manor => Color32::from_rgb(218, 165, 32),
        BuildingType::TownCenter => Color32::from_rgb(169, 169, 169),
        BuildingType::TownStorage => Color32::from_rgb(139, 119, 101),
        BuildingType::GuardPost => Color32::from_rgb(139, 0, 0),
        BuildingType::Workshop => Color32::from_rgb(105, 105, 105),
        BuildingType::Forge => Color32::from_rgb(255, 140, 0),
        BuildingType::Smithy => Color32::from_rgb(112, 128, 144),
        BuildingType::Bakery => Color32::from_rgb(245, 222, 179),
        BuildingType::WeaverHut => Color32::from_rgb(186, 85, 211),
        BuildingType::PotteryKiln => Color32::from_rgb(205, 92, 0),
        BuildingType::Tannery => Color32::from_rgb(139, 90, 43),
        BuildingType::Mill => Color32::from_rgb(222, 184, 135),
        BuildingType::Butchery => Color32::from_rgb(205, 92, 92),
        BuildingType::Brewery => Color32::from_rgb(139, 69, 19),
        BuildingType::Dairy => Color32::from_rgb(255, 255, 240),
        BuildingType::Glassworks => Color32::from_rgb(200, 225, 255),
        BuildingType::Dyeworks => Color32::from_rgb(148, 0, 211),
        BuildingType::Ropewalk => Color32::from_rgb(193, 154, 107),
        BuildingType::Brickyard => Color32::from_rgb(178, 34, 34),
        BuildingType::PaperMill => Color32::from_rgb(255, 255, 240),
        BuildingType::TailorShop => Color32::from_rgb(147, 112, 219),
        BuildingType::CobblerShop => Color32::from_rgb(101, 67, 33),
        BuildingType::BarberShop => Color32::from_rgb(255, 182, 193),
        BuildingType::Scriptorium => Color32::from_rgb(139, 90, 43),
        BuildingType::Storehouse => Color32::from_rgb(139, 119, 101),
        BuildingType::Farm => Color32::from_rgb(34, 139, 34),
        BuildingType::AnimalPen => Color32::from_rgb(139, 119, 101),
        BuildingType::Shrine => Color32::from_rgb(255, 250, 205),
        BuildingType::Temple => Color32::from_rgb(255, 215, 0),
        BuildingType::MedicalBuilding => Color32::from_rgb(255, 255, 255),
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
    }
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
    let name = format!("{:?}", resource_type);
    let fill_pct = (amount as f32 / max_amount.max(1) as f32) * 100.0;
    let fill_color = if fill_pct > 50.0 {
        Color32::from_rgb(100, 200, 100)
    } else if fill_pct > 20.0 {
        Color32::from_rgb(200, 200, 100)
    } else {
        Color32::from_rgb(200, 100, 100)
    };

    ui.label(RichText::new(&name).strong().color(Color32::WHITE));
    ui.horizontal(|ui| {
        ui.label("Amount:");
        ui.label(RichText::new(format!("{}/{}", amount, max_amount)).color(fill_color));
        ui.label(RichText::new(format!("({}%)", fill_pct as u32)).small().color(Color32::GRAY));
    });
}

/// Render building tooltip content
pub fn render_building_tooltip(ui: &mut Ui, building_type: BuildingType, completed: bool, progress: f32) {
    let name = format!("{:?}", building_type);
    ui.label(RichText::new(&name).strong().color(Color32::WHITE));

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
