// src/bevy_gui/ui/panels/search.rs
//! Search panel for finding entities in the simulation.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Color32, RichText};

use crate::agents::LifeStage;
use crate::bevy_gui::resources::{
    PanelVisibility, SearchState, SearchType, HealthFilter, SearchResult,
    CurrentSnapshot, Selection, NotificationQueue,
};
use crate::bevy_gui::events::CenterMapRequest;

/// Render the search panel dialog
pub fn render_search_panel(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut search_state: ResMut<SearchState>,
    mut selection: ResMut<Selection>,
    mut center_request: EventWriter<CenterMapRequest>,
    mut notifications: ResMut<NotificationQueue>,
    time: Res<Time>,
) {
    if !panels.search {
        return;
    }

    let current_time = time.elapsed_secs_f64();
    let mut close_dialog = false;
    let mut go_to_selected = false;

    egui::Window::new("Search")
        .collapsible(false)
        .resizable(true)
        .default_size([400.0, 500.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Search Entities");
            ui.separator();

            // Search query
            ui.horizontal(|ui| {
                ui.label("Query:");
                let response = ui.text_edit_singleline(&mut search_state.query);
                if response.changed() {
                    search_state.request_search();
                }
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    search_state.request_search();
                }
            });

            ui.add_space(5.0);

            // Search type filter
            ui.horizontal(|ui| {
                ui.label("Type:");
                let types = [
                    (SearchType::All, "All"),
                    (SearchType::Agents, "Agents"),
                    (SearchType::Buildings, "Buildings"),
                    (SearchType::Resources, "Resources"),
                ];
                for (search_type, label) in types {
                    if ui.selectable_label(search_state.search_type == search_type, label).clicked() {
                        search_state.search_type = search_type;
                        search_state.request_search();
                    }
                }
            });

            // Agent-specific filters (only show when searching agents)
            if matches!(search_state.search_type, SearchType::All | SearchType::Agents) {
                ui.add_space(5.0);
                ui.collapsing("Agent Filters", |ui| {
                    // Life stage filter
                    ui.horizontal(|ui| {
                        ui.label("Life Stage:");
                        let stages = [
                            (None, "Any"),
                        ];
                        for (stage, label) in stages {
                            if ui.selectable_label(search_state.life_stage_filter == stage, label).clicked() {
                                search_state.life_stage_filter = stage;
                                search_state.request_search();
                            }
                        }
                        let life_stages = [
                            (Some(LifeStage::Infant), "Infant"),
                            (Some(LifeStage::Child), "Child"),
                            (Some(LifeStage::Adolescent), "Adolescent"),
                            (Some(LifeStage::Adult), "Adult"),
                            (Some(LifeStage::Elderly), "Elderly"),
                        ];
                        for (stage, label) in life_stages {
                            if ui.selectable_label(search_state.life_stage_filter == stage, label).clicked() {
                                search_state.life_stage_filter = stage;
                                search_state.request_search();
                            }
                        }
                    });

                    // Health filter
                    ui.horizontal(|ui| {
                        ui.label("Health:");
                        let health_filters = [
                            (HealthFilter::Any, "Any"),
                            (HealthFilter::Critical, "Critical (<25%)"),
                            (HealthFilter::Low, "Low (<50%)"),
                            (HealthFilter::Healthy, "Healthy (>75%)"),
                        ];
                        for (filter, label) in health_filters {
                            if ui.selectable_label(search_state.health_filter == filter, label).clicked() {
                                search_state.health_filter = filter;
                                search_state.request_search();
                            }
                        }
                    });
                });
            }

            ui.add_space(10.0);
            ui.separator();

            // Results header
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Results: {}", search_state.results.len())).strong());
                if !search_state.results.is_empty() {
                    if let Some(idx) = search_state.selected_result {
                        ui.label(format!("(selected: {}/{})", idx + 1, search_state.results.len()));
                    }
                }
            });

            // Results list
            let available_height = ui.available_height() - 50.0;
            egui::ScrollArea::vertical()
                .max_height(available_height.max(100.0))
                .show(ui, |ui| {
                    if search_state.results.is_empty() {
                        ui.label(RichText::new("No results found").color(Color32::GRAY));
                    } else {
                        let mut clicked_idx = None;
                        let mut double_clicked_idx = None;

                        for (idx, result) in search_state.results.iter().enumerate() {
                            let is_selected = search_state.selected_result == Some(idx);
                            let (label, color) = format_search_result(result);

                            let response = ui.selectable_label(
                                is_selected,
                                RichText::new(&label).color(color),
                            );

                            if response.clicked() {
                                clicked_idx = Some(idx);
                            }
                            if response.double_clicked() {
                                double_clicked_idx = Some(idx);
                            }
                        }

                        if let Some(idx) = clicked_idx {
                            search_state.selected_result = Some(idx);
                        }
                        if double_clicked_idx.is_some() {
                            go_to_selected = true;
                        }
                    }
                });

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                let has_selection = search_state.selected_result.is_some();

                // Previous/Next navigation
                if ui.add_enabled(!search_state.results.is_empty(), egui::Button::new("◀ Prev")).clicked() {
                    search_state.select_previous();
                }
                if ui.add_enabled(!search_state.results.is_empty(), egui::Button::new("Next ▶")).clicked() {
                    search_state.select_next();
                }

                ui.separator();

                // Go to selected
                if ui.add_enabled(has_selection, egui::Button::new("Go To")).clicked() {
                    go_to_selected = true;
                }

                // Select entity
                if ui.add_enabled(has_selection, egui::Button::new("Select")).clicked() {
                    if let Some(result) = search_state.get_selected() {
                        match result {
                            SearchResult::Agent { id, .. } => {
                                selection.select_agent(*id);
                                notifications.info("Agent selected", current_time);
                            }
                            SearchResult::Building { position, .. } => {
                                selection.select_building((position.x, position.y));
                                notifications.info("Building selected", current_time);
                            }
                            SearchResult::Resource { position, .. } => {
                                selection.select_resource((position.x, position.y));
                                notifications.info("Resource selected", current_time);
                            }
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_dialog = true;
                    }

                    if ui.button("Clear").clicked() {
                        search_state.clear();
                    }
                });
            });
        });

    // Handle "Go To" action
    if go_to_selected {
        if let Some(result) = search_state.get_selected() {
            let pos = result.position();
            center_request.send(CenterMapRequest { x: pos.0, y: pos.1 });
            notifications.info(&format!("Centered on ({}, {})", pos.0, pos.1), current_time);
        }
    }

    if close_dialog {
        panels.search = false;
    }
}

/// Format a search result for display
fn format_search_result(result: &SearchResult) -> (String, Color32) {
    match result {
        SearchResult::Agent { id, position, life_stage, health, energy } => {
            let id_short = id.to_string()[..8].to_string();
            let health_icon = if *health < 25.0 {
                "!!"
            } else if *health < 50.0 {
                "!"
            } else {
                ""
            };
            (
                format!(
                    "Agent {} {:?} at ({}, {}) H:{:.0}% E:{:.0}% {}",
                    id_short, life_stage, position.0, position.1, health, energy, health_icon
                ),
                Color32::from_rgb(100, 180, 255),
            )
        }
        SearchResult::Building { position, building_type, completed } => {
            let status = if *completed { "" } else { " (building)" };
            (
                format!(
                    "{:?}{} at ({}, {})",
                    building_type, status, position.x, position.y
                ),
                Color32::from_rgb(200, 150, 100),
            )
        }
        SearchResult::Resource { position, resource_type, amount, max_amount } => {
            (
                format!(
                    "{:?} at ({}, {}) - {}/{}",
                    resource_type, position.x, position.y, amount, max_amount
                ),
                Color32::from_rgb(100, 200, 100),
            )
        }
    }
}

/// System that performs searches on the current snapshot
pub fn search_system(
    mut search_state: ResMut<SearchState>,
    snapshot: Res<CurrentSnapshot>,
) {
    if !search_state.needs_search {
        return;
    }

    search_state.needs_search = false;
    search_state.results.clear();
    search_state.selected_result = None;

    let Some(snap) = &snapshot.snapshot else {
        return;
    };

    let query_lower = search_state.query.to_lowercase();

    // Search agents
    if matches!(search_state.search_type, SearchType::All | SearchType::Agents) {
        for agent in &snap.population.agents {
            if !agent.is_alive {
                continue;
            }

            // Apply life stage filter
            if let Some(required_stage) = search_state.life_stage_filter {
                if agent.life_stage != required_stage {
                    continue;
                }
            }

            // Apply health filter
            if !search_state.health_filter.matches(agent.health) {
                continue;
            }

            // Apply query filter (match on ID)
            if !query_lower.is_empty() {
                let id_str = agent.id.to_string().to_lowercase();
                if !id_str.contains(&query_lower) {
                    continue;
                }
            }

            search_state.results.push(SearchResult::Agent {
                id: agent.id,
                position: (agent.position.0, agent.position.1),
                life_stage: agent.life_stage,
                health: agent.health,
                energy: agent.energy,
            });
        }
    }

    // Search buildings
    if matches!(search_state.search_type, SearchType::All | SearchType::Buildings) {
        for building in &snap.world.buildings {
            // Apply query filter (match on building type)
            if !query_lower.is_empty() {
                let type_str = format!("{:?}", building.building_type).to_lowercase();
                if !type_str.contains(&query_lower) {
                    continue;
                }
            }

            search_state.results.push(SearchResult::Building {
                position: building.position,
                building_type: building.building_type,
                completed: building.completed,
            });
        }
    }

    // Search resources
    if matches!(search_state.search_type, SearchType::All | SearchType::Resources) {
        for resource in &snap.world.resources {
            // Apply query filter (match on resource type)
            if !query_lower.is_empty() {
                let type_str = format!("{:?}", resource.resource_type).to_lowercase();
                if !type_str.contains(&query_lower) {
                    continue;
                }
            }

            search_state.results.push(SearchResult::Resource {
                position: resource.position,
                resource_type: resource.resource_type,
                amount: resource.amount,
                max_amount: resource.max_amount,
            });
        }
    }

    // Auto-select first result if any
    if !search_state.results.is_empty() {
        search_state.selected_result = Some(0);
    }
}
