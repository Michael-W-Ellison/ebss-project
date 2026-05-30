// src/gui/panels/search.rs
//! Search panel for finding entities in the simulation.

use egui::{Ui, Color32, RichText, ScrollArea, Key};
use crate::gui::state::{GuiState, SearchType, SearchResult, HealthFilter};
use crate::agents::LifeStage;

const TILE_SIZE: f32 = 12.0;

pub fn render_search_panel(ui: &mut Ui, state: &mut GuiState) {
    ui.heading("Search");
    ui.separator();

    // Search input
    ui.horizontal(|ui| {
        ui.label("Query:");
        let response = ui.text_edit_singleline(&mut state.search_state.query);
        if response.changed() {
            state.perform_search();
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(idx) = state.search_state.selected_result {
                let view_size = (400.0, 300.0);
                state.select_search_result(idx, TILE_SIZE, view_size);
                state.show_search = false;
            }
        }
    });

    ui.add_space(5.0);

    // Search type filter
    ui.horizontal(|ui| {
        ui.label("Type:");
        ui.selectable_value(&mut state.search_state.search_type, SearchType::All, "All");
        ui.selectable_value(&mut state.search_state.search_type, SearchType::Agents, "Agents");
        ui.selectable_value(&mut state.search_state.search_type, SearchType::Buildings, "Buildings");
        ui.selectable_value(&mut state.search_state.search_type, SearchType::Resources, "Resources");
    });

    // Agent-specific filters (only show when searching agents)
    if matches!(state.search_state.search_type, SearchType::All | SearchType::Agents) {
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Life Stage:");
            if ui.selectable_label(state.search_state.life_stage_filter.is_none(), "Any").clicked() {
                state.search_state.life_stage_filter = None;
                state.perform_search();
            }
            for stage in [LifeStage::Infant, LifeStage::Child, LifeStage::Adolescent, LifeStage::Adult, LifeStage::Elderly] {
                if ui.selectable_label(state.search_state.life_stage_filter == Some(stage), format!("{:?}", stage)).clicked() {
                    state.search_state.life_stage_filter = Some(stage);
                    state.perform_search();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Health:");
            if ui.selectable_label(state.search_state.health_filter == HealthFilter::Any, "Any").clicked() {
                state.search_state.health_filter = HealthFilter::Any;
                state.perform_search();
            }
            if ui.selectable_label(state.search_state.health_filter == HealthFilter::Critical, "Critical").clicked() {
                state.search_state.health_filter = HealthFilter::Critical;
                state.perform_search();
            }
            if ui.selectable_label(state.search_state.health_filter == HealthFilter::Low, "Low").clicked() {
                state.search_state.health_filter = HealthFilter::Low;
                state.perform_search();
            }
            if ui.selectable_label(state.search_state.health_filter == HealthFilter::Healthy, "Healthy").clicked() {
                state.search_state.health_filter = HealthFilter::Healthy;
                state.perform_search();
            }
        });
    }

    ui.separator();

    // Results count
    let result_count = state.search_state.results.len();
    ui.label(RichText::new(format!("{} results", result_count)).small());

    // Results list
    ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            let selected_idx = state.search_state.selected_result;
            let mut clicked_idx = None;
            let mut double_clicked_idx = None;

            for (idx, result) in state.search_state.results.iter().enumerate() {
                let is_selected = selected_idx == Some(idx);

                let label_text = match result {
                    SearchResult::Agent { position, life_stage, health, energy, .. } => {
                        format!(
                            "{:?} at ({}, {}) - HP:{:.0}% E:{:.0}%",
                            life_stage, position.0, position.1, health, energy
                        )
                    }
                    SearchResult::Building { position, building_type, completed } => {
                        let status = if *completed { "Complete" } else { "Building" };
                        format!(
                            "{:?} at ({}, {}) - {}",
                            building_type, position.x, position.y, status
                        )
                    }
                    SearchResult::Resource { position, resource_type, amount, max_amount } => {
                        format!(
                            "{:?} at ({}, {}) - {}/{}",
                            resource_type, position.x, position.y, amount, max_amount
                        )
                    }
                };

                let response = ui.selectable_label(is_selected, &label_text);

                if response.clicked() {
                    clicked_idx = Some(idx);
                }
                if response.double_clicked() {
                    double_clicked_idx = Some(idx);
                }
            }

            if let Some(idx) = double_clicked_idx {
                let view_size = (400.0, 300.0);
                state.select_search_result(idx, TILE_SIZE, view_size);
                state.show_search = false;
                return;
            }

            if let Some(idx) = clicked_idx {
                state.search_state.selected_result = Some(idx);
            }
        });

    ui.separator();

    // Action buttons
    ui.horizontal(|ui| {
        if ui.button("Go to Selected").clicked() {
            if let Some(idx) = state.search_state.selected_result {
                let view_size = (400.0, 300.0);
                state.select_search_result(idx, TILE_SIZE, view_size);
            }
        }
        if ui.button("Clear").clicked() {
            state.search_state.query.clear();
            state.search_state.results.clear();
            state.search_state.selected_result = None;
        }
        if ui.button("Close").clicked() {
            state.show_search = false;
        }
    });

    // Keyboard navigation hint
    ui.add_space(5.0);
    ui.label(RichText::new("Up/Down to navigate, Enter to select, Esc to close").small().color(Color32::GRAY));
}
