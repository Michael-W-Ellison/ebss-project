// src/gui/panels/inspector.rs
//! Entity inspector panel with detailed views for agents, buildings, and resources.

use egui::{Ui, Color32, ProgressBar, RichText, ScrollArea};
use crate::gui::state::{
    GuiState, EntitySelection, SelectedAgentData, InspectorTab, DriveData,
    SkillData, InventoryItemData, RelationshipData, GoalData,
};

pub fn render_inspector(ui: &mut Ui, state: &mut GuiState) {
    ui.heading("Inspector");
    ui.separator();

    match &state.selected {
        EntitySelection::None => render_empty_state(ui),
        EntitySelection::Agent(_) => render_agent_panel(ui, state),
        EntitySelection::Building(_) => render_building_panel(ui, state),
        EntitySelection::Resource(_) => render_resource_panel(ui, state),
        EntitySelection::Terrain(pos) => render_terrain_panel(ui, state, *pos),
    }
}

fn render_empty_state(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.label(RichText::new("No Selection").size(16.0).color(Color32::GRAY));
        ui.add_space(10.0);
        ui.label("Click on an entity to inspect it:");
        ui.add_space(5.0);
        ui.label("• Agents (colored circles)");
        ui.label("• Buildings (brown squares)");
        ui.label("• Resources (small dots)");
        ui.label("• Terrain (background)");
    });
}

// ============================================================================
// AGENT INSPECTOR
// ============================================================================

fn render_agent_panel(ui: &mut Ui, state: &mut GuiState) {
    let Some(agent) = &state.selected_agent_data else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
            ui.label("Loading agent data...");
        });
        return;
    };

    // Clone data to avoid borrow issues
    let agent = agent.clone();

    // Header with basic info
    render_agent_header(ui, &agent);
    ui.separator();

    // Tab bar
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.inspector_tab, InspectorTab::Overview, "Overview");
        ui.selectable_value(&mut state.inspector_tab, InspectorTab::Drives, "Drives");
        ui.selectable_value(&mut state.inspector_tab, InspectorTab::Skills, "Skills");
        ui.selectable_value(&mut state.inspector_tab, InspectorTab::Inventory, "Items");
        ui.selectable_value(&mut state.inspector_tab, InspectorTab::Relationships, "Social");
        ui.selectable_value(&mut state.inspector_tab, InspectorTab::Goals, "Goals");
    });
    ui.separator();

    // Tab content
    ScrollArea::vertical().show(ui, |ui| {
        match state.inspector_tab {
            InspectorTab::Overview => render_agent_overview(ui, &agent),
            InspectorTab::Drives => render_agent_drives(ui, &agent.drives),
            InspectorTab::Skills => render_agent_skills(ui, &agent.skills),
            InspectorTab::Inventory => render_agent_inventory(ui, &agent.inventory),
            InspectorTab::Relationships => render_agent_relationships(ui, &agent.relationships),
            InspectorTab::Goals => render_agent_goals(ui, &agent),
        }
    });
}

fn render_agent_header(ui: &mut Ui, agent: &SelectedAgentData) {
    ui.horizontal(|ui| {
        // Life stage icon/color
        let stage_color = life_stage_color(agent.life_stage);
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 8.0, stage_color);

        ui.vertical(|ui| {
            ui.label(RichText::new(format!("{:?}", agent.life_stage)).strong());
            ui.label(format!("ID: {}...", &agent.id.to_string()[..8]));
        });
    });

    // Survival warnings
    if agent.survival_status.is_critical {
        ui.add_space(5.0);
        ui.colored_label(Color32::RED, "⚠ CRITICAL CONDITION");
        if agent.survival_status.is_starving {
            ui.colored_label(Color32::YELLOW, format!(
                "  Starving: {} ticks without food",
                agent.survival_status.ticks_without_food
            ));
        }
        if agent.survival_status.is_dehydrated {
            ui.colored_label(Color32::LIGHT_BLUE, format!(
                "  Dehydrated: {} ticks without water",
                agent.survival_status.ticks_without_water
            ));
        }
    }
}

fn render_agent_overview(ui: &mut Ui, agent: &SelectedAgentData) {
    // Vitals section
    ui.heading("Vitals");

    // Health bar
    ui.horizontal(|ui| {
        ui.label("Health:");
        let health_color = vitals_color(agent.health);
        ui.add(ProgressBar::new(agent.health / 100.0)
            .fill(health_color)
            .text(format!("{:.0}%", agent.health)));
    });

    // Energy bar
    ui.horizontal(|ui| {
        ui.label("Energy:");
        let energy_color = vitals_color(agent.energy);
        ui.add(ProgressBar::new(agent.energy / 100.0)
            .fill(energy_color)
            .text(format!("{:.0}%", agent.energy)));
    });

    // Age
    ui.horizontal(|ui| {
        ui.label("Age:");
        let age_pct = agent.age as f32 / agent.max_age.max(1) as f32;
        ui.add(ProgressBar::new(age_pct)
            .fill(Color32::from_rgb(150, 150, 150))
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
                ui.label(RichText::new(format!("• {}", trait_name)).small());
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

    if !agent.parent_ids.is_empty() {
        ui.label(format!("Parents: {}", agent.parent_ids.len()));
    }
}

fn render_emotion_bars(ui: &mut Ui, emotions: &crate::gui::state::EmotionData) {
    let emotions_list = [
        ("Happiness", emotions.happiness, Color32::from_rgb(255, 215, 0)),
        ("Anger", emotions.anger, Color32::from_rgb(255, 69, 0)),
        ("Fear", emotions.fear, Color32::from_rgb(138, 43, 226)),
        ("Sadness", emotions.sadness, Color32::from_rgb(70, 130, 180)),
        ("Curiosity", emotions.curiosity, Color32::from_rgb(50, 205, 50)),
    ];

    for (name, value, color) in emotions_list {
        if value > 0.01 {
            ui.horizontal(|ui| {
                ui.label(format!("{}: ", name));
                ui.add(ProgressBar::new(value)
                    .fill(color)
                    .desired_width(100.0));
            });
        }
    }
}

fn render_agent_drives(ui: &mut Ui, drives: &[DriveData]) {
    ui.heading("Drive States");
    ui.add_space(5.0);

    // Sort by urgency
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
                ui.add(ProgressBar::new(drive.value)
                    .fill(urgency_color(drive.urgency))
                    .desired_width(120.0)
                    .text(format!("{:.0}%", drive.value * 100.0)));
            });

            ui.horizontal(|ui| {
                ui.label("Urgency:");
                ui.add(ProgressBar::new(drive.urgency)
                    .fill(urgency_color(drive.urgency))
                    .desired_width(120.0)
                    .text(format!("{:.0}%", drive.urgency * 100.0)));
            });
        });
        ui.add_space(3.0);
    }
}

fn render_agent_skills(ui: &mut Ui, skills: &std::collections::HashMap<String, SkillData>) {
    ui.heading("Skills");
    ui.add_space(5.0);

    if skills.is_empty() {
        ui.label("No skills developed yet.");
        return;
    }

    // Sort skills by level
    let mut sorted_skills: Vec<_> = skills.values().collect();
    sorted_skills.sort_by(|a, b| b.level.cmp(&a.level));

    for skill in sorted_skills {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let level_color = skill_level_color(skill.level);
                ui.colored_label(level_color, &skill.name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Lv {}", skill.level));
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(&skill.category).small().color(Color32::GRAY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("XP: {}", skill.experience)).small());
                });
            });
        });
    }
}

fn render_agent_inventory(ui: &mut Ui, inventory: &[InventoryItemData]) {
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
                ui.label(RichText::new(&item.item_id).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("x{}", item.quantity));
                });
            });

            if let Some(quality) = &item.quality {
                ui.label(RichText::new(format!("Quality: {}", quality)).small());
            }

            if let Some((current, max)) = item.durability {
                ui.horizontal(|ui| {
                    ui.label("Durability:");
                    let pct = current / max.max(0.01);
                    let color = vitals_color(pct * 100.0);
                    ui.add(ProgressBar::new(pct)
                        .fill(color)
                        .desired_width(80.0)
                        .text(format!("{:.0}/{:.0}", current, max)));
                });
            }

            if let Some((current, max)) = item.fill_level {
                ui.horizontal(|ui| {
                    ui.label("Fill:");
                    let pct = current / max.max(0.01);
                    ui.add(ProgressBar::new(pct)
                        .fill(Color32::from_rgb(0, 150, 255))
                        .desired_width(80.0)
                        .text(format!("{:.1}/{:.1}", current, max)));
                });
            }
        });
    }
}

fn render_agent_relationships(ui: &mut Ui, relationships: &[RelationshipData]) {
    ui.heading("Relationships");
    ui.add_space(5.0);

    if relationships.is_empty() {
        ui.label("No relationships established.");
        return;
    }

    // Sort by bond strength
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
            ui.label(RichText::new(format!("ID: {}...", &rel.other_agent_id.to_string()[..8])).small());
            ui.label(RichText::new(format!("Interactions: {}", rel.total_interactions)).small());
        });
    }
}

fn render_agent_goals(ui: &mut Ui, agent: &SelectedAgentData) {
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

fn render_goal_item(ui: &mut Ui, goal: &GoalData) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            if goal.completed {
                ui.label(RichText::new("✓").color(Color32::GREEN));
            }
            ui.label(&goal.description);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("P: {:.1}", goal.priority));
            });
        });
        if !goal.completed && goal.progress > 0.0 {
            ui.add(ProgressBar::new(goal.progress)
                .fill(Color32::from_rgb(100, 200, 100))
                .text(format!("{:.0}%", goal.progress * 100.0)));
        }
    });
}

// ============================================================================
// BUILDING INSPECTOR
// ============================================================================

fn render_building_panel(ui: &mut Ui, state: &GuiState) {
    let Some(building) = &state.selected_building_data else {
        // Fallback to basic snapshot data
        if let EntitySelection::Building(pos) = &state.selected {
            if let Some(snapshot) = &state.latest_snapshot {
                if let Some(b) = snapshot.world.buildings.iter()
                    .find(|b| b.position.x == pos.x && b.position.y == pos.y)
                {
                    render_building_basic(ui, b);
                    return;
                }
            }
        }
        ui.spinner();
        ui.label("Loading building data...");
        return;
    };

    // Header
    ui.heading(format!("{:?}", building.building_type));
    ui.label(format!("Position: ({}, {})", building.position.x, building.position.y));
    ui.separator();

    ScrollArea::vertical().show(ui, |ui| {
        // Status
        ui.heading("Status");
        if building.completed {
            ui.colored_label(Color32::GREEN, "✓ Completed");
        } else {
            ui.colored_label(Color32::YELLOW, "Under Construction");
            ui.add(ProgressBar::new(building.progress)
                .fill(Color32::from_rgb(100, 200, 100))
                .text(format!("{:.0}%", building.progress * 100.0)));

            // Resources needed
            if !building.resources_needed.is_empty() {
                ui.add_space(5.0);
                ui.label("Resources Required:");
                for (resource, delivered, required) in &building.resources_needed {
                    let pct = *delivered as f32 / (*required).max(1) as f32;
                    let color = if *delivered >= *required {
                        Color32::GREEN
                    } else {
                        Color32::YELLOW
                    };
                    ui.horizontal(|ui| {
                        ui.label(resource);
                        ui.add(ProgressBar::new(pct)
                            .fill(color)
                            .desired_width(80.0)
                            .text(format!("{}/{}", delivered, required)));
                    });
                }
            }

            // Workers
            if !building.worker_ids.is_empty() {
                ui.add_space(5.0);
                ui.label(format!("Workers: {}", building.worker_ids.len()));
            }
        }

        ui.add_space(10.0);

        // Ownership
        ui.heading("Ownership");
        if let Some(owner) = &building.owner_id {
            ui.label(format!("Owner: {}...", &owner.to_string()[..8]));
        } else {
            ui.label("No owner");
        }

        if !building.occupant_ids.is_empty() {
            ui.label(format!("Occupants: {}", building.occupant_ids.len()));
            for occupant in &building.occupant_ids {
                ui.label(RichText::new(format!("  • {}...", &occupant.to_string()[..8])).small());
            }
        }

        ui.add_space(10.0);

        // Description
        ui.heading("Description");
        ui.label(&building.description);

        if !building.benefits.is_empty() {
            ui.add_space(5.0);
            ui.label("Benefits:");
            for benefit in &building.benefits {
                ui.label(format!("• {}", benefit));
            }
        }
    });
}

fn render_building_basic(ui: &mut Ui, building: &crate::gui::state::BuildingSnapshot) {
    ui.heading(format!("{:?}", building.building_type));
    ui.label(format!("Position: ({}, {})", building.position.x, building.position.y));

    if building.completed {
        ui.colored_label(Color32::GREEN, "Status: Completed");
    } else {
        ui.label("Status: Under Construction");
        ui.add(ProgressBar::new(building.progress)
            .text(format!("{:.0}%", building.progress * 100.0)));
    }
}

// ============================================================================
// RESOURCE INSPECTOR
// ============================================================================

fn render_resource_panel(ui: &mut Ui, state: &GuiState) {
    let Some(resource) = &state.selected_resource_data else {
        // Fallback to basic snapshot data
        if let EntitySelection::Resource(pos) = &state.selected {
            if let Some(snapshot) = &state.latest_snapshot {
                if let Some(r) = snapshot.world.resources.iter()
                    .find(|r| r.position.x == pos.x && r.position.y == pos.y)
                {
                    render_resource_basic(ui, r);
                    return;
                }
            }
        }
        ui.spinner();
        ui.label("Loading resource data...");
        return;
    };

    // Header
    ui.heading(format!("{:?}", resource.resource_type));
    ui.label(format!("Position: ({}, {})", resource.position.x, resource.position.y));
    ui.separator();

    ScrollArea::vertical().show(ui, |ui| {
        // Amount
        ui.heading("Amount");
        let fill_color = if resource.is_depleted {
            Color32::RED
        } else if resource.percentage < 25.0 {
            Color32::YELLOW
        } else {
            Color32::GREEN
        };

        ui.add(ProgressBar::new(resource.percentage / 100.0)
            .fill(fill_color)
            .text(format!("{} / {}", resource.amount, resource.max_amount)));

        if resource.is_depleted {
            ui.colored_label(Color32::RED, "DEPLETED");
        } else {
            ui.label(format!("{:.1}% remaining", resource.percentage));
        }

        ui.add_space(10.0);

        // Description
        ui.heading("Description");
        ui.label(&resource.description);

        ui.add_space(10.0);

        // Uses
        ui.heading("Uses");
        for use_case in &resource.uses {
            ui.label(format!("• {}", use_case));
        }
    });
}

fn render_resource_basic(ui: &mut Ui, resource: &crate::gui::state::ResourceSnapshot) {
    ui.heading(format!("{:?}", resource.resource_type));
    ui.label(format!("Position: ({}, {})", resource.position.x, resource.position.y));

    let pct = resource.amount as f32 / resource.max_amount.max(1) as f32;
    ui.add(ProgressBar::new(pct)
        .text(format!("{} / {}", resource.amount, resource.max_amount)));
}

// ============================================================================
// TERRAIN INSPECTOR
// ============================================================================

fn render_terrain_panel(ui: &mut Ui, state: &GuiState, pos: crate::world::Position) {
    ui.heading("Terrain");
    ui.label(format!("Position: ({}, {})", pos.x, pos.y));
    ui.separator();

    if let Some(snapshot) = &state.latest_snapshot {
        if let Some(tile) = snapshot.world.tiles.iter()
            .find(|t| t.x == pos.x && t.y == pos.y)
        {
            ui.label(format!("Type: {:?}", tile.terrain));
            ui.label(format!("Walkable: {}", if tile.walkable { "Yes" } else { "No" }));

            ui.add_space(10.0);
            ui.heading("Description");
            ui.label(terrain_description(tile.terrain));

            // Check for nearby entities
            ui.add_space(10.0);

            let nearby_agents: Vec<_> = snapshot.population.agents.iter()
                .filter(|a| a.is_alive &&
                    (a.position.0 - pos.x).abs() <= 2 &&
                    (a.position.1 - pos.y).abs() <= 2)
                .collect();

            if !nearby_agents.is_empty() {
                ui.heading(format!("Nearby Agents ({})", nearby_agents.len()));
                for agent in nearby_agents.iter().take(5) {
                    ui.label(format!("• {:?} at ({}, {})",
                        agent.life_stage, agent.position.0, agent.position.1));
                }
                if nearby_agents.len() > 5 {
                    ui.label(format!("... and {} more", nearby_agents.len() - 5));
                }
            }
        }
    }
}

fn terrain_description(terrain: crate::world::TerrainType) -> &'static str {
    use crate::world::TerrainType;
    match terrain {
        TerrainType::Plains => "Flat grassland suitable for building and farming.",
        TerrainType::Meadow => "Lush meadow with wildflowers and grazing areas.",
        TerrainType::Forest => "Dense woodland with trees for lumber.",
        TerrainType::Hills => "Rolling hills with varied terrain.",
        TerrainType::Mountain => "Steep rocky terrain, difficult to traverse.",
        TerrainType::Water => "Body of water, not walkable without swimming.",
        TerrainType::Desert => "Arid sandy terrain with sparse vegetation.",
        TerrainType::Wetland => "Marshy area with water and vegetation.",
        TerrainType::Beach => "Sandy shoreline between land and water.",
        TerrainType::Sea => "Salt water. Fish in it, salt in it, and nothing in it to drink.",
        TerrainType::SaltMarsh => "Brackish and boggy, where the sea meets the land.",
        TerrainType::SaltFlat => "Where a shallow sea dried up and left what was in it.",
        TerrainType::Riverbank => "The edge of a river, good for fishing.",
        TerrainType::Farmland => "Ground broken and sown; crops grow here far faster than anything wild.",
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn life_stage_color(stage: crate::agents::LifeStage) -> Color32 {
    use crate::agents::LifeStage;
    match stage {
        LifeStage::Infant => Color32::from_rgb(255, 182, 193),
        LifeStage::Child => Color32::from_rgb(135, 206, 250),
        LifeStage::Adolescent => Color32::from_rgb(144, 238, 144),
        LifeStage::Adult => Color32::WHITE,
        LifeStage::Elderly => Color32::from_rgb(192, 192, 192),
    }
}

fn vitals_color(value: f32) -> Color32 {
    if value > 70.0 {
        Color32::GREEN
    } else if value > 30.0 {
        Color32::YELLOW
    } else {
        Color32::RED
    }
}

fn urgency_color(urgency: f32) -> Color32 {
    if urgency > 0.7 {
        Color32::RED
    } else if urgency > 0.4 {
        Color32::YELLOW
    } else {
        Color32::GREEN
    }
}

fn skill_level_color(level: i32) -> Color32 {
    if level >= 6 {
        Color32::from_rgb(255, 215, 0) // Gold - Master
    } else if level >= 0 {
        Color32::from_rgb(192, 192, 192) // Silver - Journeyman
    } else if level >= -5 {
        Color32::from_rgb(205, 127, 50) // Bronze - Apprentice
    } else {
        Color32::GRAY // Unskilled
    }
}

fn bond_color(strength: f32) -> Color32 {
    if strength >= 0.6 {
        Color32::from_rgb(255, 105, 180) // Pink - Loved one
    } else if strength >= 0.3 {
        Color32::from_rgb(100, 200, 100) // Green - Friend
    } else if strength >= 0.0 {
        Color32::from_rgb(200, 200, 200) // Gray - Acquaintance
    } else if strength >= -0.5 {
        Color32::from_rgb(255, 165, 0) // Orange - Rival
    } else {
        Color32::RED // Enemy
    }
}
