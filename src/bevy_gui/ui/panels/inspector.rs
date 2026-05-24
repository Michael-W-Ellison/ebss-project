// src/bevy_gui/ui/panels/inspector.rs
//! Entity inspector panel with detailed views for agents, buildings, and resources.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::bevy_gui::resources::{
    PanelVisibility, Selection, EntitySelection, SelectedEntityData, InspectorTab, InspectorState,
};
use crate::gui::state::{
    SelectedAgentData, SelectedBuildingData, SelectedResourceData,
    DriveData, SkillData, InventoryItemData, RelationshipData, GoalData,
};

pub fn render_inspector_panel(
    mut egui_ctx: EguiContexts,
    panels: Res<PanelVisibility>,
    selection: Res<Selection>,
    entity_data: Res<SelectedEntityData>,
    mut inspector_state: ResMut<InspectorState>,
) {
    if !panels.inspector {
        return;
    }

    egui::SidePanel::left("inspector_panel")
        .default_width(280.0)
        .resizable(true)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Inspector");
            ui.separator();

            match &selection.current {
                EntitySelection::None => render_empty_state(ui),
                EntitySelection::Agent(_) => {
                    if let Some(agent) = &entity_data.agent {
                        render_agent_panel(ui, agent, &mut inspector_state);
                    } else {
                        render_loading(ui, "agent");
                    }
                }
                EntitySelection::Building(_) => {
                    if let Some(building) = &entity_data.building {
                        render_building_panel(ui, building);
                    } else {
                        render_loading(ui, "building");
                    }
                }
                EntitySelection::Resource(_) => {
                    if let Some(resource) = &entity_data.resource {
                        render_resource_panel(ui, resource);
                    } else {
                        render_loading(ui, "resource");
                    }
                }
                EntitySelection::Terrain(pos) => {
                    render_terrain_panel(ui, *pos);
                }
            }
        });
}

fn render_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(egui::RichText::new("No Selection").size(16.0).color(egui::Color32::GRAY));
        ui.add_space(10.0);
        ui.label("Click on an entity to inspect it:");
        ui.add_space(5.0);
        ui.label("• Agents (colored circles)");
        ui.label("• Buildings (brown squares)");
        ui.label("• Resources (small dots)");
        ui.label("• Terrain (background)");
    });
}

fn render_loading(ui: &mut egui::Ui, entity_type: &str) {
    ui.centered_and_justified(|ui| {
        ui.spinner();
        ui.label(format!("Loading {} data...", entity_type));
    });
}

// ============================================================================
// AGENT INSPECTOR
// ============================================================================

fn render_agent_panel(ui: &mut egui::Ui, agent: &SelectedAgentData, state: &mut InspectorState) {
    render_agent_header(ui, agent);
    ui.separator();

    // Tab bar
    ui.horizontal(|ui| {
        for tab in InspectorTab::all() {
            if ui.selectable_label(state.active_tab == *tab, tab.name()).clicked() {
                state.active_tab = *tab;
            }
        }
    });
    ui.separator();

    // Tab content
    egui::ScrollArea::vertical().show(ui, |ui| {
        match state.active_tab {
            InspectorTab::Overview => render_agent_overview(ui, agent),
            InspectorTab::Drives => render_agent_drives(ui, &agent.drives),
            InspectorTab::Skills => render_agent_skills(ui, &agent.skills),
            InspectorTab::Inventory => render_agent_inventory(ui, &agent.inventory),
            InspectorTab::Relationships => render_agent_relationships(ui, &agent.relationships),
            InspectorTab::Goals => render_agent_goals(ui, agent),
        }
    });
}

fn render_agent_header(ui: &mut egui::Ui, agent: &SelectedAgentData) {
    ui.horizontal(|ui| {
        let stage_color = life_stage_color(agent.life_stage);
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 8.0, stage_color);

        ui.vertical(|ui| {
            ui.label(egui::RichText::new(format!("{:?}", agent.life_stage)).strong());
            ui.label(format!("ID: {}...", &agent.id.to_string()[..8]));
        });
    });

    if agent.survival_status.is_critical {
        ui.add_space(5.0);
        ui.colored_label(egui::Color32::RED, "⚠ CRITICAL CONDITION");
        if agent.survival_status.is_starving {
            ui.colored_label(egui::Color32::YELLOW, format!(
                "  Starving: {} ticks without food",
                agent.survival_status.ticks_without_food
            ));
        }
        if agent.survival_status.is_dehydrated {
            ui.colored_label(egui::Color32::LIGHT_BLUE, format!(
                "  Dehydrated: {} ticks without water",
                agent.survival_status.ticks_without_water
            ));
        }
    }
}

fn render_agent_overview(ui: &mut egui::Ui, agent: &SelectedAgentData) {
    ui.heading("Vitals");

    // Health bar
    ui.horizontal(|ui| {
        ui.label("Health:");
        let health_color = vitals_color(agent.health);
        ui.add(egui::ProgressBar::new(agent.health / 100.0)
            .fill(health_color)
            .text(format!("{:.0}%", agent.health)));
    });

    // Energy bar
    ui.horizontal(|ui| {
        ui.label("Energy:");
        let energy_color = vitals_color(agent.energy);
        ui.add(egui::ProgressBar::new(agent.energy / 100.0)
            .fill(energy_color)
            .text(format!("{:.0}%", agent.energy)));
    });

    // Age
    ui.horizontal(|ui| {
        ui.label("Age:");
        let age_pct = agent.age as f32 / agent.max_age.max(1) as f32;
        ui.add(egui::ProgressBar::new(age_pct)
            .fill(egui::Color32::from_rgb(150, 150, 150))
            .text(format!("{} / {}", agent.age, agent.max_age)));
    });

    ui.add_space(10.0);

    // Position
    ui.heading("Location");
    ui.label(format!("Position: ({}, {}, {})",
        agent.position.0, agent.position.1, agent.position.2));

    if let Some(activity) = &agent.current_activity {
        ui.label(format!("Activity: {}", activity));
    }

    ui.add_space(10.0);

    // Emotions
    ui.heading("Emotions");
    render_emotion_bars(ui, &agent.emotions);

    ui.add_space(10.0);

    // Traits
    if !agent.traits.is_empty() {
        ui.heading("Traits");
        ui.horizontal_wrapped(|ui| {
            for trait_name in &agent.traits {
                ui.label(egui::RichText::new(format!("• {}", trait_name)).small());
            }
        });
    }

    ui.add_space(10.0);

    // Summary stats
    ui.heading("Summary");
    ui.horizontal(|ui| {
        ui.label(format!("Items: {}", agent.inventory.len()));
        ui.separator();
        ui.label(format!("Relationships: {}", agent.relationships.len()));
        ui.separator();
        ui.label(format!("Goals: {}", agent.goals.len()));
    });
}

fn render_emotion_bars(ui: &mut egui::Ui, emotions: &crate::gui::state::EmotionData) {
    let emotions_list = [
        ("Happiness", emotions.happiness, egui::Color32::from_rgb(255, 215, 0)),
        ("Anger", emotions.anger, egui::Color32::from_rgb(255, 69, 0)),
        ("Fear", emotions.fear, egui::Color32::from_rgb(138, 43, 226)),
        ("Sadness", emotions.sadness, egui::Color32::from_rgb(70, 130, 180)),
        ("Curiosity", emotions.curiosity, egui::Color32::from_rgb(50, 205, 50)),
    ];

    for (name, value, color) in emotions_list {
        if value > 0.01 {
            ui.horizontal(|ui| {
                ui.label(format!("{}: ", name));
                ui.add(egui::ProgressBar::new(value)
                    .fill(color)
                    .desired_width(100.0));
            });
        }
    }
}

fn render_agent_drives(ui: &mut egui::Ui, drives: &[DriveData]) {
    ui.heading("Drive States");
    ui.add_space(5.0);

    let mut sorted_drives = drives.to_vec();
    sorted_drives.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));

    for drive in sorted_drives {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let urgency_color = urgency_color(drive.urgency);
                ui.colored_label(urgency_color, format!("{:?}", drive.drive_type));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("W: {:.1}", drive.weight));
                });
            });

            ui.horizontal(|ui| {
                ui.label("Value:");
                ui.add(egui::ProgressBar::new(drive.value)
                    .fill(urgency_color(drive.urgency))
                    .desired_width(120.0)
                    .text(format!("{:.0}%", drive.value * 100.0)));
            });

            ui.horizontal(|ui| {
                ui.label("Urgency:");
                ui.add(egui::ProgressBar::new(drive.urgency)
                    .fill(urgency_color(drive.urgency))
                    .desired_width(120.0)
                    .text(format!("{:.0}%", drive.urgency * 100.0)));
            });
        });
        ui.add_space(3.0);
    }
}

fn render_agent_skills(ui: &mut egui::Ui, skills: &std::collections::HashMap<String, SkillData>) {
    ui.heading("Skills");
    ui.add_space(5.0);

    if skills.is_empty() {
        ui.label("No skills developed yet.");
        return;
    }

    let mut sorted_skills: Vec<_> = skills.values().collect();
    sorted_skills.sort_by(|a, b| b.level.cmp(&a.level));

    for skill in sorted_skills {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let level_color = skill_level_color(skill.level.max(0) as u32);
                ui.colored_label(level_color, &skill.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Lv {}", skill.level));
                });
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&skill.category).small().color(egui::Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("XP: {}", skill.experience)).small());
                });
            });
        });
    }
}

fn render_agent_inventory(ui: &mut egui::Ui, inventory: &[InventoryItemData]) {
    ui.heading("Inventory");
    ui.add_space(5.0);

    if inventory.is_empty() {
        ui.label("Inventory is empty.");
        return;
    }

    ui.label(format!("Items: {}", inventory.len()));
    ui.separator();

    for item in inventory {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&item.item_id).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("x{}", item.quantity));
                });
            });

            if let Some(quality) = &item.quality {
                ui.label(egui::RichText::new(format!("Quality: {}", quality)).small());
            }

            if let Some((current, max)) = item.durability {
                ui.horizontal(|ui| {
                    ui.label("Durability:");
                    let pct = current / max.max(0.01);
                    let color = vitals_color(pct * 100.0);
                    ui.add(egui::ProgressBar::new(pct)
                        .fill(color)
                        .desired_width(80.0)
                        .text(format!("{:.0}/{:.0}", current, max)));
                });
            }
        });
    }
}

fn render_agent_relationships(ui: &mut egui::Ui, relationships: &[RelationshipData]) {
    ui.heading("Relationships");
    ui.add_space(5.0);

    if relationships.is_empty() {
        ui.label("No relationships established.");
        return;
    }

    let mut sorted_rels = relationships.to_vec();
    sorted_rels.sort_by(|a, b| b.bond_strength.partial_cmp(&a.bond_strength).unwrap_or(std::cmp::Ordering::Equal));

    for rel in sorted_rels {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let bond_color = bond_color(rel.bond_strength);
                ui.colored_label(bond_color, &rel.relationship_type);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{:.0}%", rel.bond_strength * 100.0));
                });
            });
            ui.label(egui::RichText::new(format!("ID: {}...", &rel.other_agent_id.to_string()[..8])).small());
            ui.label(egui::RichText::new(format!("Interactions: {}", rel.total_interactions)).small());
        });
    }
}

fn render_agent_goals(ui: &mut egui::Ui, agent: &SelectedAgentData) {
    ui.heading("Active Goals");
    ui.add_space(5.0);

    let active_goals: Vec<_> = agent.goals.iter().filter(|g| !g.completed).collect();

    if active_goals.is_empty() {
        ui.label("No active goals.");
    } else {
        for goal in active_goals {
            render_goal_item(ui, goal);
        }
    }

    ui.add_space(10.0);

    let completed_goals: Vec<_> = agent.goals.iter().filter(|g| g.completed).collect();
    if !completed_goals.is_empty() {
        ui.collapsing(format!("Completed Goals ({})", completed_goals.len()), |ui| {
            for goal in completed_goals {
                render_goal_item(ui, goal);
            }
        });
    }

    if let Some(activity) = &agent.current_activity {
        ui.add_space(10.0);
        ui.heading("Current Plan");
        ui.label(activity);
    }
}

fn render_goal_item(ui: &mut egui::Ui, goal: &GoalData) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            if goal.completed {
                ui.label(egui::RichText::new("✓").color(egui::Color32::GREEN));
            }
            ui.label(&goal.description);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("P: {:.1}", goal.priority));
            });
        });
        if !goal.completed && goal.progress > 0.0 {
            ui.add(egui::ProgressBar::new(goal.progress)
                .fill(egui::Color32::from_rgb(100, 200, 100))
                .text(format!("{:.0}%", goal.progress * 100.0)));
        }
    });
}

// ============================================================================
// BUILDING INSPECTOR
// ============================================================================

fn render_building_panel(ui: &mut egui::Ui, building: &SelectedBuildingData) {
    ui.heading(format!("{:?}", building.building_type));
    ui.label(format!("Position: ({}, {})", building.position.x, building.position.y));
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Status");
        if building.completed {
            ui.colored_label(egui::Color32::GREEN, "✓ Completed");
        } else {
            ui.colored_label(egui::Color32::YELLOW, "Under Construction");
            ui.add(egui::ProgressBar::new(building.progress)
                .fill(egui::Color32::from_rgb(100, 200, 100))
                .text(format!("{:.0}%", building.progress * 100.0)));

            if !building.resources_needed.is_empty() {
                ui.add_space(5.0);
                ui.label("Resources Required:");
                for (resource, delivered, required) in &building.resources_needed {
                    let pct = *delivered as f32 / (*required).max(1) as f32;
                    let color = if *delivered >= *required {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::YELLOW
                    };
                    ui.horizontal(|ui| {
                        ui.label(resource);
                        ui.add(egui::ProgressBar::new(pct)
                            .fill(color)
                            .desired_width(80.0)
                            .text(format!("{}/{}", delivered, required)));
                    });
                }
            }

            if !building.worker_ids.is_empty() {
                ui.add_space(5.0);
                ui.label(format!("Workers: {}", building.worker_ids.len()));
            }
        }

        ui.add_space(10.0);
        ui.heading("Ownership");
        if let Some(owner) = &building.owner_id {
            ui.label(format!("Owner: {}...", &owner.to_string()[..8]));
        } else {
            ui.label("No owner");
        }

        if !building.occupant_ids.is_empty() {
            ui.label(format!("Occupants: {}", building.occupant_ids.len()));
        }

        ui.add_space(10.0);
        ui.heading("Description");
        ui.label(&building.description);
    });
}

// ============================================================================
// RESOURCE INSPECTOR
// ============================================================================

fn render_resource_panel(ui: &mut egui::Ui, resource: &SelectedResourceData) {
    ui.heading(format!("{:?}", resource.resource_type));
    ui.label(format!("Position: ({}, {})", resource.position.x, resource.position.y));
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Quantity");
        ui.label(format!("Amount: {}", resource.amount));

        if resource.max_amount > 0 {
            let pct = resource.amount as f32 / resource.max_amount as f32;
            let color = if pct > 0.5 {
                egui::Color32::GREEN
            } else if pct > 0.25 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            };
            ui.add(egui::ProgressBar::new(pct)
                .fill(color)
                .text(format!("{} / {}", resource.amount, resource.max_amount)));
        }

        ui.add_space(5.0);
        ui.label(format!("Fill: {:.0}%", resource.percentage));

        if resource.is_depleted {
            ui.add_space(5.0);
            ui.colored_label(egui::Color32::RED, "✗ Depleted");
        }

        ui.add_space(10.0);
        ui.heading("Description");
        ui.label(&resource.description);

        if !resource.uses.is_empty() {
            ui.add_space(5.0);
            ui.heading("Uses");
            for use_str in &resource.uses {
                ui.label(format!("• {}", use_str));
            }
        }
    });
}

// ============================================================================
// TERRAIN INSPECTOR
// ============================================================================

fn render_terrain_panel(ui: &mut egui::Ui, pos: crate::world::Position) {
    ui.heading("Terrain");
    ui.label(format!("Position: ({}, {})", pos.x, pos.y));
    ui.separator();

    ui.label("Click on an agent, building, or resource for more details.");
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn life_stage_color(stage: crate::agents::LifeStage) -> egui::Color32 {
    match stage {
        crate::agents::LifeStage::Infant => egui::Color32::from_rgb(255, 182, 193),
        crate::agents::LifeStage::Child => egui::Color32::from_rgb(135, 206, 250),
        crate::agents::LifeStage::Adolescent => egui::Color32::from_rgb(144, 238, 144),
        crate::agents::LifeStage::Adult => egui::Color32::WHITE,
        crate::agents::LifeStage::Elderly => egui::Color32::from_rgb(192, 192, 192),
    }
}

fn vitals_color(value: f32) -> egui::Color32 {
    if value >= 75.0 {
        egui::Color32::from_rgb(100, 200, 100)
    } else if value >= 50.0 {
        egui::Color32::from_rgb(200, 200, 100)
    } else if value >= 25.0 {
        egui::Color32::from_rgb(200, 150, 100)
    } else {
        egui::Color32::from_rgb(200, 100, 100)
    }
}

fn urgency_color(urgency: f32) -> egui::Color32 {
    if urgency >= 0.8 {
        egui::Color32::from_rgb(255, 100, 100)
    } else if urgency >= 0.6 {
        egui::Color32::from_rgb(255, 180, 100)
    } else if urgency >= 0.4 {
        egui::Color32::from_rgb(255, 255, 100)
    } else {
        egui::Color32::from_rgb(150, 200, 150)
    }
}

fn skill_level_color(level: u32) -> egui::Color32 {
    match level {
        0..=2 => egui::Color32::WHITE,
        3..=5 => egui::Color32::from_rgb(100, 200, 100),
        6..=8 => egui::Color32::from_rgb(100, 150, 255),
        9..=10 => egui::Color32::from_rgb(255, 200, 100),
        _ => egui::Color32::from_rgb(255, 100, 255),
    }
}

fn bond_color(strength: f32) -> egui::Color32 {
    if strength >= 0.8 {
        egui::Color32::from_rgb(255, 200, 100)
    } else if strength >= 0.5 {
        egui::Color32::from_rgb(100, 200, 100)
    } else if strength >= 0.2 {
        egui::Color32::from_rgb(150, 150, 150)
    } else {
        egui::Color32::from_rgb(100, 100, 100)
    }
}
