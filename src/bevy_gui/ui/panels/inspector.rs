// src/bevy_gui/ui/panels/inspector.rs
//! Entity inspector panel with detailed views for agents, buildings, and resources.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::bevy_gui::resources::{
    PanelVisibility, Selection, EntitySelection, SelectedEntityData, InspectorTab, InspectorState,
    CurrentSnapshot, NotificationQueue,
};
use crate::bevy_gui::events::{SimulationCommand, CenterMapRequest};
use crate::gui::state::{
    SelectedAgentData, SelectedBuildingData, SelectedResourceData,
    DriveData, SkillData, InventoryItemData, GoalData,
};
use crate::environment::TICKS_PER_YEAR;

/// How many ticks make a year, in the shape the age fields want.
fn ebss_years() -> u32 {
    TICKS_PER_YEAR.max(1)
}

pub fn render_inspector_panel(
    mut egui_ctx: EguiContexts,
    panels: Res<PanelVisibility>,
    mut selection: ResMut<Selection>,
    entity_data: Res<SelectedEntityData>,
    mut inspector_state: ResMut<InspectorState>,
    snapshot: Res<CurrentSnapshot>,
    mut sim_commands: EventWriter<SimulationCommand>,
    mut center_request: EventWriter<CenterMapRequest>,
    mut notifications: ResMut<NotificationQueue>,
    time: Res<Time>,
) {
    if !panels.inspector {
        return;
    }

    let current_time = time.elapsed_secs_f64();

    egui::SidePanel::left("inspector_panel")
        .default_width(300.0)
        .min_width(250.0)
        .resizable(true)
        .show(egui_ctx.ctx_mut(), |ui| {
            // Panel header with tools
            ui.horizontal(|ui| {
                ui.heading("Inspector");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pin_text = if inspector_state.pinned {
                        egui::RichText::new("📌").color(egui::Color32::from_rgb(255, 200, 100))
                    } else {
                        egui::RichText::new("📌").color(egui::Color32::GRAY)
                    };
                    if ui.button(pin_text)
                        .on_hover_text(if inspector_state.pinned {
                            "Unpin selection (inspector will follow selection)"
                        } else {
                            "Pin selection (inspector stays on this entity)"
                        })
                        .clicked()
                    {
                        inspector_state.pinned = !inspector_state.pinned;
                        if inspector_state.pinned {
                            inspector_state.pinned_selection = Some(selection.current.clone());
                            notifications.info("Selection pinned", current_time);
                        } else {
                            inspector_state.pinned_selection = None;
                            notifications.info("Selection unpinned", current_time);
                        }
                    }
                });
            });
            ui.separator();

            // Determine which selection to show (pinned or current)
            let display_selection = if inspector_state.pinned {
                inspector_state.pinned_selection.as_ref().unwrap_or(&selection.current)
            } else {
                &selection.current
            };

            match display_selection {
                EntitySelection::None => render_empty_state(ui),
                EntitySelection::Agent(_) => {
                    if let Some(agent) = &entity_data.agent {
                        render_agent_panel(ui, agent, &mut inspector_state, &snapshot, &mut selection, &mut sim_commands, &mut center_request, &mut notifications, current_time);
                    } else {
                        render_loading(ui, "agent");
                    }
                }
                EntitySelection::Building(_) => {
                    if let Some(building) = &entity_data.building {
                        render_building_panel(ui, building, &snapshot, &mut selection, &mut sim_commands, &mut center_request, &mut notifications, current_time);
                    } else {
                        render_loading(ui, "building");
                    }
                }
                EntitySelection::Resource(_) => {
                    if let Some(resource) = &entity_data.resource {
                        render_resource_panel(ui, resource, &snapshot);
                    } else {
                        render_loading(ui, "resource");
                    }
                }
                EntitySelection::Terrain(pos) => {
                    render_terrain_panel(ui, *pos, &snapshot);
                }
            }
        });
}

fn render_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(egui::RichText::new("No Selection").size(18.0).color(egui::Color32::GRAY));
        ui.add_space(15.0);
        ui.label(egui::RichText::new("Click on the map to inspect:").color(egui::Color32::LIGHT_GRAY));
        ui.add_space(10.0);

        let hints = [
            ("○", "Agents", "colored circles on map"),
            ("□", "Buildings", "brown squares"),
            ("·", "Resources", "small colored dots"),
            ("▢", "Terrain", "background tiles"),
        ];

        for (icon, name, desc) in hints {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).monospace().color(egui::Color32::from_rgb(150, 150, 150)));
                ui.label(egui::RichText::new(name).strong());
                ui.label(egui::RichText::new(format!("- {}", desc)).small().color(egui::Color32::GRAY));
            });
        }

        ui.add_space(20.0);
        ui.label(egui::RichText::new("Tip: Press Tab to cycle through agents").small().color(egui::Color32::DARK_GRAY));
    });
}

fn render_loading(ui: &mut egui::Ui, entity_type: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(30.0);
        ui.spinner();
        ui.add_space(10.0);
        ui.label(format!("Loading {} data...", entity_type));
    });
}

// ============================================================================
// AGENT INSPECTOR
// ============================================================================

fn render_agent_panel(
    ui: &mut egui::Ui,
    agent: &SelectedAgentData,
    state: &mut InspectorState,
    snapshot: &Res<CurrentSnapshot>,
    selection: &mut ResMut<Selection>,
    sim_commands: &mut EventWriter<SimulationCommand>,
    center_request: &mut EventWriter<CenterMapRequest>,
    notifications: &mut ResMut<NotificationQueue>,
    current_time: f64,
) {
    render_agent_header(ui, agent, notifications, current_time);
    ui.separator();

    // Tab bar with icons
    ui.horizontal_wrapped(|ui| {
        for tab in InspectorTab::all() {
            let label = format!("{} {}", tab.icon(), tab.name());
            let response = ui.selectable_label(state.active_tab == *tab, label);
            if response.clicked() {
                state.active_tab = *tab;
            }
        }
    });
    ui.separator();

    // Tab content
    egui::ScrollArea::vertical().show(ui, |ui| {
        match state.active_tab {
            InspectorTab::Overview => render_agent_overview(ui, agent),
            InspectorTab::Drives => render_agent_drives(ui, &agent.drives, state),
            InspectorTab::Skills => render_agent_skills(ui, &agent.skills, state),
            InspectorTab::Inventory => render_agent_inventory(ui, &agent.inventory),
            InspectorTab::Relationships => render_agent_relationships(
                ui, agent, selection, sim_commands, center_request, notifications, current_time, state
            ),
            InspectorTab::Goals => render_agent_goals(ui, agent, state),
        }
    });
}

fn render_agent_header(ui: &mut egui::Ui, agent: &SelectedAgentData, notifications: &mut ResMut<NotificationQueue>, current_time: f64) {
    // Main header row
    ui.horizontal(|ui| {
        // Life stage indicator circle
        let stage_color = life_stage_color(agent.life_stage);
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(24.0, 24.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 10.0, stage_color);

        // There is no gender in this model - "agents are gender neutral; there
        // are no male/female agents, merely child and adult agents" - so what
        // used to be a male or female symbol beside somebody grown is now the
        // only distinction there is: whether they are grown.
        let gender_symbol = match agent.life_stage {
            crate::agents::LifeStage::Infant | crate::agents::LifeStage::Child => "",
            _ => "\u{25CF}",
        };

        ui.vertical(|ui| {
            // Name or life stage
            let display_name = if agent.name.is_empty() {
                format!("{:?}", agent.life_stage)
            } else {
                agent.name.clone()
            };
            ui.label(egui::RichText::new(display_name).strong().size(14.0));

            // ID with copy button
            ui.horizontal(|ui| {
                let short_id = &agent.id.to_string()[..8];
                ui.label(egui::RichText::new(format!("ID: {}...", short_id)).small().color(egui::Color32::GRAY));
                if ui.small_button("📋")
                    .on_hover_text("Copy full ID to clipboard")
                    .clicked()
                {
                    ui.output_mut(|o| o.copied_text = agent.id.to_string());
                    notifications.info("ID copied to clipboard", current_time);
                }
            });
        });

        // Quick stats on right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Age display
            let age_years = agent.age / ebss_years();
            ui.label(egui::RichText::new(format!("{}y", age_years)).small());
        });
    });

    // Vitals summary bar
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        // Health mini-bar
        let health_color = vitals_color(agent.health);
        ui.label(egui::RichText::new("♥").color(health_color));
        ui.add(egui::ProgressBar::new(agent.health / 100.0)
            .fill(health_color)
            .desired_width(60.0));

        ui.separator();

        // Energy mini-bar
        let energy_color = vitals_color(agent.energy);
        ui.label(egui::RichText::new("⚡").color(energy_color));
        ui.add(egui::ProgressBar::new(agent.energy / 100.0)
            .fill(energy_color)
            .desired_width(60.0));
    });

    // Critical status alerts
    if agent.survival_status.is_critical {
        ui.add_space(5.0);
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚠").color(egui::Color32::RED).size(16.0));
                ui.label(egui::RichText::new("CRITICAL").color(egui::Color32::RED).strong());
            });

            if agent.survival_status.is_starving {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("  🍖").color(egui::Color32::YELLOW));
                    ui.label(egui::RichText::new(format!(
                        "Starving ({} ticks)",
                        agent.survival_status.ticks_without_food
                    )).color(egui::Color32::YELLOW).small());
                });
            }

            if agent.survival_status.is_dehydrated {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("  💧").color(egui::Color32::LIGHT_BLUE));
                    ui.label(egui::RichText::new(format!(
                        "Dehydrated ({} ticks)",
                        agent.survival_status.ticks_without_water
                    )).color(egui::Color32::LIGHT_BLUE).small());
                });
            }
        });
    }
}

fn render_agent_overview(ui: &mut egui::Ui, agent: &SelectedAgentData) {
    // Vitals section
    ui.collapsing(egui::RichText::new("♥ Vitals").strong(), |ui| {
        // Health bar with detailed tooltip
        ui.horizontal(|ui| {
            ui.label("Health:");
            let health_color = vitals_color(agent.health);
            ui.add(egui::ProgressBar::new(agent.health / 100.0)
                .fill(health_color)
                .text(format!("{:.0}%", agent.health)))
                .on_hover_text("Current health level. Below 25% is critical.");
        });

        // Energy bar
        ui.horizontal(|ui| {
            ui.label("Energy:");
            let energy_color = vitals_color(agent.energy);
            ui.add(egui::ProgressBar::new(agent.energy / 100.0)
                .fill(energy_color)
                .text(format!("{:.0}%", agent.energy)))
                .on_hover_text("Current energy level. Low energy causes fatigue.");
        });

        // Age progress
        ui.horizontal(|ui| {
            ui.label("Age:");
            let age_years = agent.age / ebss_years();
            let max_years = agent.max_age / ebss_years();
            let age_pct = agent.age as f32 / agent.max_age.max(1) as f32;
            let age_color = if age_pct > 0.8 {
                egui::Color32::from_rgb(200, 150, 100)
            } else {
                egui::Color32::from_rgb(150, 150, 150)
            };
            ui.add(egui::ProgressBar::new(age_pct)
                .fill(age_color)
                .text(format!("{} / {} years", age_years, max_years)))
                .on_hover_text(format!("{} / {} ticks", agent.age, agent.max_age));
        });
    }).header_response.on_hover_text("Health, energy, and age information");

    ui.add_space(5.0);

    // Location & Activity section
    ui.collapsing(egui::RichText::new("📍 Location & Activity").strong(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Position:");
            ui.label(egui::RichText::new(format!("({}, {}, {})",
                agent.position.0, agent.position.1, agent.position.2)).monospace());
        });

        if let Some(activity) = &agent.current_activity {
            ui.horizontal(|ui| {
                ui.label("Activity:");
                ui.label(egui::RichText::new(activity).color(egui::Color32::from_rgb(100, 180, 255)));
            });
        } else {
            ui.horizontal(|ui| {
                ui.label("Activity:");
                ui.label(egui::RichText::new("Idle").color(egui::Color32::GRAY));
            });
        }
    });

    ui.add_space(5.0);

    // Emotions section
    ui.collapsing(egui::RichText::new("😊 Emotions").strong(), |ui| {
        render_emotion_bars(ui, &agent.emotions);
    });

    ui.add_space(5.0);

    // Traits section
    if !agent.traits.is_empty() {
        ui.collapsing(egui::RichText::new(format!("✨ Traits ({})", agent.traits.len())).strong(), |ui| {
            ui.horizontal_wrapped(|ui| {
                for trait_name in &agent.traits {
                    ui.label(
                        egui::RichText::new(trait_name)
                            .background_color(egui::Color32::from_rgb(50, 50, 70))
                            .color(egui::Color32::from_rgb(200, 200, 255))
                    );
                }
            });
        });
    }

    ui.add_space(5.0);

    // Family section
    if !agent.parent_ids.is_empty() {
        ui.collapsing(egui::RichText::new(format!("👨‍👩‍👧 Family")).strong(), |ui| {
            ui.label(egui::RichText::new("Parents:").small().color(egui::Color32::GRAY));
            for parent_id in &agent.parent_ids {
                ui.horizontal(|ui| {
                    ui.label(format!("  {}...", &parent_id.to_string()[..8]));
                });
            }
        });
    }

    ui.add_space(5.0);

    // Summary stats
    ui.group(|ui| {
        ui.horizontal(|ui| {
            stat_badge(ui, "🎒", agent.inventory.len(), "Items");
            ui.separator();
            stat_badge(ui, "❤", agent.relationships.len(), "Relations");
            ui.separator();
            stat_badge(ui, "����", agent.goals.iter().filter(|g| !g.completed).count(), "Active Goals");
        });
    });
}

fn stat_badge(ui: &mut egui::Ui, icon: &str, count: usize, tooltip: &str) {
    ui.label(egui::RichText::new(format!("{} {}", icon, count)))
        .on_hover_text(tooltip);
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

fn render_agent_drives(ui: &mut egui::Ui, drives: &[DriveData], state: &mut InspectorState) {
    // Header with sort option
    ui.horizontal(|ui| {
        ui.heading("Drive States");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut state.show_detailed_drives, "Details")
                .on_hover_text("Show detailed drive information");
        });
    });
    ui.add_space(5.0);

    if drives.is_empty() {
        ui.label(egui::RichText::new("No drive data available").color(egui::Color32::GRAY));
        return;
    }

    let mut sorted_drives = drives.to_vec();
    sorted_drives.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));

    // Most urgent drive highlight
    if let Some(top_drive) = sorted_drives.first() {
        if top_drive.urgency > 0.5 {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🎯 Priority:").strong());
                    let color = urgency_color(top_drive.urgency);
                    ui.colored_label(color, format!("{:?}", top_drive.drive_type));
                    ui.label(egui::RichText::new(format!("({:.0}%)", top_drive.urgency * 100.0)).color(color));
                });
            });
            ui.add_space(5.0);
        }
    }

    for drive in sorted_drives {
        let urgency_col = urgency_color(drive.urgency);
        let drive_icon = drive_icon(drive.drive_type);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(drive_icon));
                ui.colored_label(urgency_col, format!("{:?}", drive.drive_type));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Urgency indicator
                    let urgency_label = if drive.urgency >= 0.8 {
                        "Critical"
                    } else if drive.urgency >= 0.6 {
                        "High"
                    } else if drive.urgency >= 0.4 {
                        "Medium"
                    } else {
                        "Low"
                    };
                    ui.label(egui::RichText::new(urgency_label).small().color(urgency_col));
                });
            });

            // Compact or detailed view
            if state.show_detailed_drives {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Value:").small());
                    ui.add(egui::ProgressBar::new(drive.value)
                        .fill(urgency_col)
                        .desired_width(100.0)
                        .text(format!("{:.0}%", drive.value * 100.0)));
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Urgency:").small());
                    ui.add(egui::ProgressBar::new(drive.urgency)
                        .fill(urgency_col)
                        .desired_width(100.0)
                        .text(format!("{:.0}%", drive.urgency * 100.0)));
                });

                ui.label(egui::RichText::new(format!("Weight: {:.2}", drive.weight)).small().color(egui::Color32::GRAY));
            } else {
                // Compact: just urgency bar
                ui.add(egui::ProgressBar::new(drive.urgency)
                    .fill(urgency_col)
                    .text(format!("{:.0}%", drive.urgency * 100.0)));
            }
        });
        ui.add_space(2.0);
    }
}

fn drive_icon(drive_type: crate::core::DriveType) -> &'static str {
    use crate::core::DriveType;
    match drive_type {
        DriveType::Hunger => "🍖",
        DriveType::Thirst => "💧",
        DriveType::Rest => "💤",
        DriveType::Safety => "🛡",
        DriveType::Social => "👥",
        DriveType::Shelter => "🏠",
        DriveType::Curiosity => "❓",
        DriveType::Preparedness => "📦",
        DriveType::Industry => "⚒",
        DriveType::Sustenance => "🌾",
        DriveType::Reproduction => "💕",
        DriveType::Luxury => "💎",
        DriveType::Utility => "🔧",
        DriveType::Construction => "🏗",
        DriveType::Protection => "🧒",
    }
}

fn render_agent_skills(ui: &mut egui::Ui, skills: &std::collections::BTreeMap<String, SkillData>, state: &mut InspectorState) {
    ui.horizontal(|ui| {
        ui.heading("Skills");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.selectable_label(state.skills_sort_by_level, "By Level")
                .on_hover_text("Sort skills by level")
                .clicked()
            {
                state.skills_sort_by_level = true;
            }
            if ui.selectable_label(!state.skills_sort_by_level, "By Name")
                .on_hover_text("Sort skills alphabetically")
                .clicked()
            {
                state.skills_sort_by_level = false;
            }
        });
    });
    ui.add_space(5.0);

    if skills.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("No skills developed yet").color(egui::Color32::GRAY));
            ui.label(egui::RichText::new("Skills are gained through activities").small().color(egui::Color32::DARK_GRAY));
        });
        return;
    }

    // Summary
    let total_levels: i32 = skills.values().map(|s| s.level).sum();
    let max_level = skills.values().map(|s| s.level).max().unwrap_or(0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{} skills", skills.len())).small());
        ui.separator();
        ui.label(egui::RichText::new(format!("Total: {} levels", total_levels)).small());
        ui.separator();
        ui.label(egui::RichText::new(format!("Max: Lv {}", max_level)).small().color(skill_level_color(max_level as u32)));
    });
    ui.separator();

    let mut sorted_skills: Vec<_> = skills.values().collect();
    if state.skills_sort_by_level {
        sorted_skills.sort_by(|a, b| b.level.cmp(&a.level));
    } else {
        sorted_skills.sort_by(|a, b| a.name.cmp(&b.name));
    }

    // Group by category
    let mut categories: std::collections::BTreeMap<&str, Vec<&SkillData>> = std::collections::BTreeMap::new();
    for skill in &sorted_skills {
        categories.entry(&skill.category).or_default().push(skill);
    }

    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
    sorted_categories.sort_by(|a, b| a.0.cmp(b.0));

    for (category, cat_skills) in sorted_categories {
        ui.collapsing(egui::RichText::new(format!("{} ({})", category, cat_skills.len())).strong(), |ui| {
            for skill in cat_skills {
                render_skill_item(ui, skill);
            }
        });
    }
}

fn render_skill_item(ui: &mut egui::Ui, skill: &SkillData) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            let level_color = skill_level_color(skill.level.max(0) as u32);

            // Level badge
            ui.label(
                egui::RichText::new(format!("Lv{}", skill.level))
                    .color(level_color)
                    .strong()
                    .background_color(egui::Color32::from_rgb(40, 40, 50))
            );

            ui.label(&skill.name);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{} XP", skill.experience)).small().color(egui::Color32::GRAY));
            });
        });

        // XP progress to next level (assuming 100 XP per level)
        let xp_for_next = ((skill.level + 1) * 100) as u32;
        let xp_progress = (skill.experience % 100) as f32 / 100.0;
        ui.add(egui::ProgressBar::new(xp_progress)
            .fill(egui::Color32::from_rgb(80, 120, 180))
            .desired_width(ui.available_width() - 10.0)
            .text(egui::RichText::new(format!("{}/{} to Lv{}", skill.experience % 100, 100, skill.level + 1)).small()));
    });
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

fn render_agent_relationships(
    ui: &mut egui::Ui,
    agent: &SelectedAgentData,
    selection: &mut ResMut<Selection>,
    sim_commands: &mut EventWriter<SimulationCommand>,
    center_request: &mut EventWriter<CenterMapRequest>,
    notifications: &mut ResMut<NotificationQueue>,
    current_time: f64,
    state: &mut InspectorState,
) {
    let relationships = &agent.relationships;

    ui.horizontal(|ui| {
        ui.heading("Relationships");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.selectable_label(state.relationships_sort_by_strength, "Strength")
                .on_hover_text("Sort by bond strength")
                .clicked()
            {
                state.relationships_sort_by_strength = true;
            }
            if ui.selectable_label(!state.relationships_sort_by_strength, "Type")
                .on_hover_text("Sort by relationship type")
                .clicked()
            {
                state.relationships_sort_by_strength = false;
            }
        });
    });
    ui.add_space(5.0);

    // Family section (parents)
    if !agent.parent_ids.is_empty() {
        ui.collapsing(egui::RichText::new(format!("👨‍👩‍👧 Family ({})", agent.parent_ids.len())).strong(), |ui| {
            for parent_id in &agent.parent_ids {
                render_clickable_agent(ui, *parent_id, "Parent", selection, sim_commands, center_request, notifications, current_time);
            }
        });
        ui.add_space(5.0);
    }

    if relationships.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("No social relationships yet").color(egui::Color32::GRAY));
            ui.label(egui::RichText::new("Relationships form through interactions").small().color(egui::Color32::DARK_GRAY));
        });
        return;
    }

    // Summary stats
    let strong_bonds = relationships.iter().filter(|r| r.bond_strength >= 0.7).count();
    let total_interactions: u32 = relationships.iter().map(|r| r.total_interactions).sum();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{} relationships", relationships.len())).small());
        ui.separator();
        ui.label(egui::RichText::new(format!("{} strong bonds", strong_bonds)).small().color(egui::Color32::from_rgb(255, 200, 100)));
    });
    ui.separator();

    let mut sorted_rels = relationships.to_vec();
    if state.relationships_sort_by_strength {
        sorted_rels.sort_by(|a, b| b.bond_strength.partial_cmp(&a.bond_strength).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        sorted_rels.sort_by(|a, b| a.relationship_type.cmp(&b.relationship_type));
    }

    for rel in sorted_rels {
        let bond_col = bond_color(rel.bond_strength);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                // Relationship type with icon
                let rel_icon = relationship_icon(&rel.relationship_type);
                ui.label(egui::RichText::new(rel_icon));
                ui.colored_label(bond_col, &rel.relationship_type);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Bond strength
                    ui.add(egui::ProgressBar::new(rel.bond_strength)
                        .fill(bond_col)
                        .desired_width(50.0));
                });
            });

            // Clickable agent ID
            ui.horizontal(|ui| {
                let short_id = &rel.other_agent_id.to_string()[..8];
                if ui.link(format!("→ {}...", short_id))
                    .on_hover_text("Click to select this agent")
                    .clicked()
                {
                    selection.select_agent(rel.other_agent_id);
                    sim_commands.send(SimulationCommand::SelectEntity(EntitySelection::Agent(rel.other_agent_id)));
                    notifications.info("Selected related agent", current_time);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{} talks", rel.total_interactions)).small().color(egui::Color32::GRAY));
                });
            });
        });
        ui.add_space(2.0);
    }
}

fn render_clickable_agent(
    ui: &mut egui::Ui,
    agent_id: uuid::Uuid,
    label: &str,
    selection: &mut ResMut<Selection>,
    sim_commands: &mut EventWriter<SimulationCommand>,
    center_request: &mut EventWriter<CenterMapRequest>,
    notifications: &mut ResMut<NotificationQueue>,
    current_time: f64,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).small().color(egui::Color32::GRAY));
        let short_id = &agent_id.to_string()[..8];
        if ui.link(format!("{}...", short_id))
            .on_hover_text("Click to select this agent")
            .clicked()
        {
            selection.select_agent(agent_id);
            sim_commands.send(SimulationCommand::SelectEntity(EntitySelection::Agent(agent_id)));
            notifications.info(format!("Selected {}", label.to_lowercase()), current_time);
        }
    });
}

fn relationship_icon(rel_type: &str) -> &'static str {
    let lower = rel_type.to_lowercase();
    if lower.contains("friend") {
        "👫"
    } else if lower.contains("parent") || lower.contains("mother") || lower.contains("father") {
        "👨‍👩‍👧"
    } else if lower.contains("child") || lower.contains("son") || lower.contains("daughter") {
        "👶"
    } else if lower.contains("spouse") || lower.contains("partner") || lower.contains("mate") {
        "💑"
    } else if lower.contains("sibling") || lower.contains("brother") || lower.contains("sister") {
        "👫"
    } else if lower.contains("enemy") || lower.contains("rival") {
        "⚔"
    } else {
        "👤"
    }
}

fn render_agent_goals(ui: &mut egui::Ui, agent: &SelectedAgentData, state: &mut InspectorState) {
    let active_goals: Vec<_> = agent.goals.iter().filter(|g| !g.completed).collect();
    let completed_goals: Vec<_> = agent.goals.iter().filter(|g| g.completed).collect();

    ui.horizontal(|ui| {
        ui.heading("Goals");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(&mut state.show_completed_goals, "Completed")
                .on_hover_text("Show completed goals");
        });
    });
    ui.add_space(5.0);

    // Current activity highlight
    if let Some(activity) = &agent.current_activity {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🎬 Current:").strong());
                ui.label(egui::RichText::new(activity).color(egui::Color32::from_rgb(100, 200, 255)));
            });
        });
        ui.add_space(5.0);
    }

    // Summary
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{} active", active_goals.len())).small());
        ui.separator();
        ui.label(egui::RichText::new(format!("{} completed", completed_goals.len())).small().color(egui::Color32::GREEN));
    });
    ui.separator();

    // Active goals
    if active_goals.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("No active goals").color(egui::Color32::GRAY));
            ui.label(egui::RichText::new("Agent is awaiting new objectives").small().color(egui::Color32::DARK_GRAY));
        });
    } else {
        ui.label(egui::RichText::new("Active Goals").strong());
        let mut sorted_active = active_goals.clone();
        sorted_active.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));

        for goal in sorted_active {
            render_goal_item(ui, goal, false);
        }
    }

    // Completed goals (collapsible or shown based on state)
    if !completed_goals.is_empty() && state.show_completed_goals {
        ui.add_space(10.0);
        ui.collapsing(egui::RichText::new(format!("✓ Completed ({})", completed_goals.len())).color(egui::Color32::GREEN), |ui| {
            for goal in completed_goals {
                render_goal_item(ui, goal, true);
            }
        });
    }
}

fn render_goal_item(ui: &mut egui::Ui, goal: &GoalData, completed: bool) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            // Status indicator
            if completed {
                ui.label(egui::RichText::new("✓").color(egui::Color32::GREEN));
            } else if goal.progress > 0.0 {
                ui.label(egui::RichText::new("▶").color(egui::Color32::from_rgb(100, 180, 255)));
            } else {
                ui.label(egui::RichText::new("○").color(egui::Color32::GRAY));
            }

            // Description (truncated if too long)
            let desc = if goal.description.len() > 40 {
                format!("{}...", &goal.description[..37])
            } else {
                goal.description.clone()
            };
            ui.label(&desc).on_hover_text(&goal.description);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Priority badge
                let priority_color = if goal.priority >= 0.8 {
                    egui::Color32::from_rgb(255, 100, 100)
                } else if goal.priority >= 0.5 {
                    egui::Color32::from_rgb(255, 200, 100)
                } else {
                    egui::Color32::GRAY
                };
                ui.label(egui::RichText::new(format!("P{:.0}", goal.priority * 10.0)).small().color(priority_color));
            });
        });

        // Progress bar for in-progress goals
        if !completed && goal.progress > 0.0 {
            ui.add(egui::ProgressBar::new(goal.progress)
                .fill(egui::Color32::from_rgb(100, 200, 100))
                .desired_width(ui.available_width() - 10.0)
                .text(format!("{:.0}%", goal.progress * 100.0)));
        }
    });
}

// ============================================================================
// BUILDING INSPECTOR
// ============================================================================

fn render_building_panel(
    ui: &mut egui::Ui,
    building: &SelectedBuildingData,
    snapshot: &Res<CurrentSnapshot>,
    selection: &mut ResMut<Selection>,
    sim_commands: &mut EventWriter<SimulationCommand>,
    center_request: &mut EventWriter<CenterMapRequest>,
    notifications: &mut ResMut<NotificationQueue>,
    current_time: f64,
) {
    // Header
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🏠").size(20.0));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(format!("{:?}", building.building_type)).strong().size(14.0));
            ui.label(egui::RichText::new(format!("({}, {})", building.position.x, building.position.y)).small().color(egui::Color32::GRAY));
        });
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Status section
        ui.collapsing(egui::RichText::new("📊 Status").strong(), |ui| {
            if building.completed {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("✓ Completed").color(egui::Color32::GREEN).strong());
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔨 Under Construction").color(egui::Color32::YELLOW));
                });

                ui.add(egui::ProgressBar::new(building.progress)
                    .fill(egui::Color32::from_rgb(100, 200, 100))
                    .text(format!("{:.0}%", building.progress * 100.0)));

                // Resources needed
                if !building.resources_needed.is_empty() {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("Resources Required:").small());

                    for (resource, delivered, required) in &building.resources_needed {
                        let pct = *delivered as f32 / (*required).max(1) as f32;
                        let (color, status) = if *delivered >= *required {
                            (egui::Color32::GREEN, "✓")
                        } else {
                            (egui::Color32::YELLOW, "○")
                        };

                        ui.horizontal(|ui| {
                            ui.label(status);
                            ui.label(resource);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add(egui::ProgressBar::new(pct)
                                    .fill(color)
                                    .desired_width(60.0)
                                    .text(format!("{}/{}", delivered, required)));
                            });
                        });
                    }
                }

                // Active workers
                if !building.worker_ids.is_empty() {
                    ui.add_space(5.0);
                    ui.collapsing(format!("👷 Workers ({})", building.worker_ids.len()), |ui| {
                        for worker_id in &building.worker_ids {
                            render_clickable_agent(ui, *worker_id, "Worker", selection, sim_commands, center_request, notifications, current_time);
                        }
                    });
                }
            }
        });

        ui.add_space(5.0);

        // Ownership section
        ui.collapsing(egui::RichText::new("👤 Ownership").strong(), |ui| {
            if let Some(owner) = &building.owner_id {
                render_clickable_agent(ui, *owner, "Owner", selection, sim_commands, center_request, notifications, current_time);
            } else {
                ui.label(egui::RichText::new("No owner (public)").color(egui::Color32::GRAY));
            }

            if !building.occupant_ids.is_empty() {
                ui.add_space(5.0);
                ui.collapsing(format!("🏠 Occupants ({})", building.occupant_ids.len()), |ui| {
                    for occupant_id in &building.occupant_ids {
                        render_clickable_agent(ui, *occupant_id, "Occupant", selection, sim_commands, center_request, notifications, current_time);
                    }
                });
            }
        });

        ui.add_space(5.0);

        // Benefits section
        if !building.benefits.is_empty() {
            ui.collapsing(egui::RichText::new(format!("✨ Benefits ({})", building.benefits.len())).strong(), |ui| {
                for benefit in &building.benefits {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("•").color(egui::Color32::GREEN));
                        ui.label(benefit);
                    });
                }
            });
            ui.add_space(5.0);
        }

        // Description
        ui.collapsing(egui::RichText::new("📝 Description").strong(), |ui| {
            ui.label(&building.description);
        });
    });
}

// ============================================================================
// RESOURCE INSPECTOR
// ============================================================================

fn render_resource_panel(ui: &mut egui::Ui, resource: &SelectedResourceData, snapshot: &Res<CurrentSnapshot>) {
    // Header with icon
    let resource_icon = resource_type_icon(resource.resource_type);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(resource_icon).size(20.0));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(format!("{:?}", resource.resource_type)).strong().size(14.0));
            ui.label(egui::RichText::new(format!("({}, {})", resource.position.x, resource.position.y)).small().color(egui::Color32::GRAY));
        });
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Quantity section
        ui.collapsing(egui::RichText::new("📊 Quantity").strong(), |ui| {
            let pct = if resource.max_amount > 0 {
                resource.amount as f32 / resource.max_amount as f32
            } else {
                1.0
            };

            let (color, status) = if resource.is_depleted {
                (egui::Color32::RED, "Depleted")
            } else if pct > 0.5 {
                (egui::Color32::GREEN, "Abundant")
            } else if pct > 0.25 {
                (egui::Color32::YELLOW, "Moderate")
            } else {
                (egui::Color32::from_rgb(255, 150, 100), "Scarce")
            };

            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(color, status);
            });

            if resource.max_amount > 0 {
                ui.horizontal(|ui| {
                    ui.label("Amount:");
                    ui.label(egui::RichText::new(format!("{} / {}", resource.amount, resource.max_amount)).strong());
                });

                ui.add(egui::ProgressBar::new(pct)
                    .fill(color)
                    .text(format!("{:.0}%", resource.percentage)));
            } else {
                ui.horizontal(|ui| {
                    ui.label("Amount:");
                    ui.label(egui::RichText::new(format!("{}", resource.amount)).strong());
                });
            }

            if resource.is_depleted {
                ui.add_space(5.0);
                ui.colored_label(egui::Color32::RED, "⚠ This resource is depleted and cannot be harvested");
            }
        });

        ui.add_space(5.0);

        // Uses section
        if !resource.uses.is_empty() {
            ui.collapsing(egui::RichText::new(format!("🔧 Uses ({})", resource.uses.len())).strong(), |ui| {
                for use_str in &resource.uses {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("•").color(egui::Color32::from_rgb(100, 180, 255)));
                        ui.label(use_str);
                    });
                }
            });
            ui.add_space(5.0);
        }

        // Description
        ui.collapsing(egui::RichText::new("📝 Description").strong(), |ui| {
            ui.label(&resource.description);
        });
    });
}

fn resource_type_icon(resource_type: crate::world::ResourceType) -> &'static str {
    use crate::world::ResourceType;
    match resource_type {
        ResourceType::Wood => "🪵",
        ResourceType::Stone => "🪨",
        ResourceType::Iron => "⛏",
        ResourceType::Food => "🍖",
        ResourceType::Water => "💧",
        ResourceType::Grain => "🌾",
        ResourceType::Fish => "🐟",
        ResourceType::Meat => "🥩",
        ResourceType::Herbs => "🌿",
        _ => "📦",
    }
}

// ============================================================================
// TERRAIN INSPECTOR
// ============================================================================

fn render_terrain_panel(ui: &mut egui::Ui, pos: crate::world::Position, snapshot: &Res<CurrentSnapshot>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("🗺").size(20.0));
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Terrain").strong().size(14.0));
            ui.label(egui::RichText::new(format!("({}, {})", pos.x, pos.y)).small().color(egui::Color32::GRAY));
        });
    });
    ui.separator();

    // Get tile info from snapshot
    if let Some(snap) = &snapshot.snapshot {
        // Find terrain type
        if let Some(tile) = snap.world.tiles.iter().find(|t| t.x == pos.x && t.y == pos.y) {
            ui.collapsing(egui::RichText::new("🏔 Terrain Type").strong(), |ui| {
                let terrain_name = format!("{:?}", tile.terrain);
                let terrain_color = terrain_type_color(tile.terrain);

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, terrain_color);
                    ui.label(egui::RichText::new(&terrain_name).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("Walkable:");
                    if tile.walkable {
                        ui.colored_label(egui::Color32::GREEN, "Yes");
                    } else {
                        ui.colored_label(egui::Color32::RED, "No");
                    }
                });
            });
        }

        ui.add_space(5.0);

        // Entities on this tile
        let agents_here: Vec<_> = snap.population.agents.iter()
            .filter(|a| a.position.0 == pos.x && a.position.1 == pos.y && a.is_alive)
            .collect();

        let resources_here: Vec<_> = snap.world.resources.iter()
            .filter(|r| r.position.x == pos.x && r.position.y == pos.y)
            .collect();

        let buildings_here: Vec<_> = snap.world.buildings.iter()
            .filter(|b| b.position.x == pos.x && b.position.y == pos.y)
            .collect();

        // Store counts before moving into closures
        let agent_count = agents_here.len();
        let resource_count = resources_here.len();
        let building_count = buildings_here.len();

        // Agents
        if agent_count > 0 {
            ui.collapsing(egui::RichText::new(format!("👥 Agents ({})", agent_count)).strong(), |ui| {
                for agent in &agents_here {
                    ui.horizontal(|ui| {
                        let stage_color = life_stage_color(agent.life_stage);
                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(10.0, 10.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 4.0, stage_color);
                        ui.label(format!("{:?} - {}...", agent.life_stage, &agent.id.to_string()[..8]));
                    });
                }
            });
        }

        // Resources
        if resource_count > 0 {
            ui.collapsing(egui::RichText::new(format!("📦 Resources ({})", resource_count)).strong(), |ui| {
                for resource in &resources_here {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:?}: {}/{}", resource.resource_type, resource.amount, resource.max_amount));
                    });
                }
            });
        }

        // Buildings
        if building_count > 0 {
            ui.collapsing(egui::RichText::new(format!("🏠 Buildings ({})", building_count)).strong(), |ui| {
                for building in &buildings_here {
                    ui.horizontal(|ui| {
                        let status = if building.completed { "✓" } else { "🔨" };
                        ui.label(format!("{} {:?}", status, building.building_type));
                    });
                }
            });
        }

        // If nothing on tile
        if agent_count == 0 && resource_count == 0 && building_count == 0 {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("No entities on this tile").color(egui::Color32::GRAY));
        }
    } else {
        ui.label(egui::RichText::new("Waiting for world data...").color(egui::Color32::GRAY));
    }

    ui.add_space(10.0);
    ui.label(egui::RichText::new("💡 Tip: Click on an entity for details").small().color(egui::Color32::DARK_GRAY));
}

fn terrain_type_color(terrain: crate::world::TerrainType) -> egui::Color32 {
    use crate::world::TerrainType;
    match terrain {
        TerrainType::Plains => egui::Color32::from_rgb(144, 238, 144),
        TerrainType::Meadow => egui::Color32::from_rgb(124, 252, 0),
        TerrainType::Forest => egui::Color32::from_rgb(34, 139, 34),
        TerrainType::Hills => egui::Color32::from_rgb(139, 137, 112),
        TerrainType::Mountain => egui::Color32::from_rgb(128, 128, 128),
        TerrainType::Water => egui::Color32::from_rgb(65, 105, 225),
        TerrainType::Desert => egui::Color32::from_rgb(238, 203, 173),
        TerrainType::Wetland => egui::Color32::from_rgb(85, 107, 47),
        TerrainType::Beach => egui::Color32::from_rgb(238, 214, 175),
        TerrainType::Sea => egui::Color32::from_rgb(20, 60, 120),
        TerrainType::SaltMarsh => egui::Color32::from_rgb(96, 128, 116),
        TerrainType::SaltFlat => egui::Color32::from_rgb(232, 232, 224),
        TerrainType::Riverbank => egui::Color32::from_rgb(107, 142, 35),
        TerrainType::Farmland => egui::Color32::from_rgb(205, 170, 90),
    }
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
