// src/gui/panels/inspector.rs
//! Entity inspector panel.

use egui::{Ui, Color32, ProgressBar};
use crate::gui::state::{GuiState, EntitySelection, SelectedAgentData};

pub fn render_inspector(
    ui: &mut Ui,
    state: &GuiState,
    selected_agent_data: &Option<SelectedAgentData>,
) {
    ui.heading("Inspector");
    ui.separator();

    match &state.selected {
        EntitySelection::None => {
            ui.label("Click on an entity to inspect it.");
            ui.add_space(10.0);
            ui.label("Entities:");
            ui.label("  - Agents (colored circles)");
            ui.label("  - Buildings (brown squares)");
            ui.label("  - Resources (small circles)");
        }

        EntitySelection::Agent(id) => {
            if let Some(agent_data) = selected_agent_data {
                render_agent_inspector(ui, agent_data);
            } else {
                ui.label(format!("Loading agent {:?}...", id));
                ui.spinner();
            }
        }

        EntitySelection::Building(pos) => {
            if let Some(snapshot) = &state.latest_snapshot {
                if let Some(building) = snapshot.world.buildings.iter()
                    .find(|b| b.position.x == pos.x && b.position.y == pos.y)
                {
                    ui.label(format!("Building: {:?}", building.building_type));
                    ui.label(format!("Position: ({}, {})", pos.x, pos.y));

                    if building.completed {
                        ui.colored_label(Color32::GREEN, "Status: Completed");
                    } else {
                        ui.label("Status: Under Construction");
                        ui.add(ProgressBar::new(building.progress).text(format!("{:.0}%", building.progress * 100.0)));
                    }
                } else {
                    ui.label("Building not found.");
                }
            }
        }

        EntitySelection::Resource(pos) => {
            if let Some(snapshot) = &state.latest_snapshot {
                if let Some(resource) = snapshot.world.resources.iter()
                    .find(|r| r.position.x == pos.x && r.position.y == pos.y)
                {
                    ui.label(format!("Resource: {:?}", resource.resource_type));
                    ui.label(format!("Position: ({}, {})", pos.x, pos.y));
                    ui.label(format!("Amount: {} / {}", resource.amount, resource.max_amount));

                    let pct = resource.amount as f32 / resource.max_amount.max(1) as f32;
                    ui.add(ProgressBar::new(pct).text(format!("{:.0}%", pct * 100.0)));
                } else {
                    ui.label("Resource not found.");
                }
            }
        }

        EntitySelection::Terrain(pos) => {
            if let Some(snapshot) = &state.latest_snapshot {
                if let Some(tile) = snapshot.world.tiles.iter()
                    .find(|t| t.x == pos.x && t.y == pos.y)
                {
                    ui.label(format!("Terrain: {:?}", tile.terrain));
                    ui.label(format!("Position: ({}, {})", pos.x, pos.y));
                    ui.label(format!("Walkable: {}", if tile.walkable { "Yes" } else { "No" }));
                } else {
                    ui.label("Terrain not found.");
                }
            }
        }
    }
}

fn render_agent_inspector(ui: &mut Ui, agent: &SelectedAgentData) {
    // Header
    ui.label(format!("Agent: {}", &agent.name[..agent.name.len().min(20)]));
    ui.label(format!("Life Stage: {:?}", agent.life_stage));

    ui.add_space(5.0);

    // Vital stats
    ui.collapsing("Vitals", |ui| {
        // Health bar
        ui.horizontal(|ui| {
            ui.label("Health:");
            let health_color = if agent.health > 70.0 {
                Color32::GREEN
            } else if agent.health > 30.0 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.add(ProgressBar::new(agent.health / 100.0)
                .fill(health_color)
                .text(format!("{:.0}/100", agent.health)));
        });

        // Energy bar
        ui.horizontal(|ui| {
            ui.label("Energy:");
            let energy_color = if agent.energy > 50.0 {
                Color32::from_rgb(0, 200, 255) // Cyan
            } else if agent.energy > 20.0 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.add(ProgressBar::new(agent.energy / 100.0)
                .fill(energy_color)
                .text(format!("{:.0}/100", agent.energy)));
        });

        // Age
        let age_pct = agent.age as f32 / agent.max_age as f32;
        ui.horizontal(|ui| {
            ui.label("Age:");
            ui.add(ProgressBar::new(age_pct)
                .text(format!("{} / {} ({:.0}%)", agent.age, agent.max_age, age_pct * 100.0)));
        });

        ui.label(format!("Position: ({}, {}, {})", agent.position.0, agent.position.1, agent.position.2));
    });

    ui.add_space(5.0);

    // Drives
    ui.collapsing("Drives", |ui| {
        for drive in &agent.drives {
            let urgency_color = if drive.urgency > 0.7 {
                Color32::RED
            } else if drive.urgency > 0.4 {
                Color32::YELLOW
            } else {
                Color32::GREEN
            };

            ui.horizontal(|ui| {
                ui.label(format!("{:?}:", drive.drive_type));
                ui.add(ProgressBar::new(drive.value)
                    .fill(urgency_color)
                    .text(format!("{:.0}%", drive.value * 100.0)));
            });
        }
    });

    ui.add_space(5.0);

    // Traits
    if !agent.traits.is_empty() {
        ui.collapsing("Traits", |ui| {
            for trait_name in &agent.traits {
                ui.label(format!("• {}", trait_name));
            }
        });
    }

    ui.add_space(5.0);

    // Skills
    if !agent.skills.is_empty() {
        ui.collapsing("Skills", |ui| {
            let mut skills: Vec<_> = agent.skills.iter().collect();
            skills.sort_by(|a, b| b.1.cmp(a.1));

            for (skill_name, level) in skills {
                ui.horizontal(|ui| {
                    ui.label(format!("{}: ", skill_name));
                    ui.label(format!("{}", level));
                });
            }
        });
    }

    ui.add_space(5.0);

    // Summary
    ui.collapsing("Summary", |ui| {
        ui.label(format!("Inventory: {} items", agent.inventory_count));
        ui.label(format!("Relationships: {}", agent.relationship_count));
    });
}
