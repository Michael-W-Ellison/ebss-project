// src/bevy_gui/ui/panels/timeline.rs
//! Timeline panel showing simulation event history.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Color32, RichText};
use uuid::Uuid;

use crate::bevy_gui::resources::{PanelVisibility, TimelineData, Selection, EntitySelection, MapViewState};
use crate::bevy_gui::events::SelectionChanged;
use crate::gui::events::{EventFilterType, SimulationEvent, SimulationEventExt};

const TILE_SIZE: f32 = 12.0;

pub fn render_timeline_panel(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut timeline: ResMut<TimelineData>,
    mut selection: ResMut<Selection>,
    mut map_view: ResMut<MapViewState>,
    mut selection_events: EventWriter<SelectionChanged>,
) {
    if !panels.timeline {
        return;
    }

    let mut close_requested = false;

    egui::Window::new("Event Timeline")
        .default_size([500.0, 600.0])
        .resizable(true)
        .collapsible(false)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.heading("Event Timeline");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
            ui.separator();

            render_filter_bar(ui, &mut timeline);
            ui.separator();

            render_search_bar(ui, &mut timeline);
            ui.separator();

            render_event_list(ui, &timeline, &mut selection, &mut map_view, &mut selection_events);

            render_pagination(ui, &mut timeline);
        });

    if close_requested {
        panels.timeline = false;
    }
}

fn render_filter_bar(ui: &mut egui::Ui, timeline: &mut TimelineData) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Filters:");

        for filter_type in EventFilterType::all() {
            let is_active = timeline.filter_types.contains(filter_type);
            let (r, g, b) = filter_type.color();
            let color = if is_active {
                Color32::from_rgb(r, g, b)
            } else {
                Color32::from_rgb(r / 2 + 50, g / 2 + 50, b / 2 + 50)
            };

            let text = RichText::new(filter_type.display_name()).color(color);
            if ui.selectable_label(is_active, text).clicked() {
                timeline.toggle_filter(*filter_type);
            }
        }

        ui.separator();

        if ui.small_button("Clear").clicked() {
            timeline.clear_filters();
        }
    });

    ui.horizontal(|ui| {
        ui.label("Sort:");
        if ui.selectable_label(timeline.newest_first, "Newest First").clicked() {
            timeline.newest_first = true;
            timeline.first_page();
        }
        if ui.selectable_label(!timeline.newest_first, "Oldest First").clicked() {
            timeline.newest_first = false;
            timeline.first_page();
        }

        ui.separator();

        ui.checkbox(&mut timeline.auto_scroll, "Auto-scroll");
    });
}

fn render_search_bar(ui: &mut egui::Ui, timeline: &mut TimelineData) {
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.text_edit_singleline(&mut timeline.search_query);
        if response.changed() {
            timeline.first_page();
        }

        if ui.small_button("X").clicked() {
            timeline.search_query.clear();
            timeline.first_page();
        }

        ui.separator();

        let total = timeline.event_log.len();
        let filtered = timeline.filtered_count();
        if filtered == total {
            ui.label(format!("{} events", total));
        } else {
            ui.label(format!("{} / {} events", filtered, total));
        }
    });
}

fn render_event_list(
    ui: &mut egui::Ui,
    timeline: &TimelineData,
    selection: &mut Selection,
    map_view: &mut MapViewState,
    selection_events: &mut EventWriter<SelectionChanged>,
) {
    let events: Vec<SimulationEvent> = timeline.get_page_events()
        .iter()
        .map(|e| (*e).clone())
        .collect();

    let log_empty = timeline.event_log.is_empty();

    if events.is_empty() {
        ui.centered_and_justified(|ui| {
            if log_empty {
                ui.label("No events yet. Events will appear as the simulation runs.");
            } else {
                ui.label("No events match the current filters.");
            }
        });
        return;
    }

    let available_height = ui.available_height() - 40.0;

    egui::ScrollArea::vertical()
        .max_height(available_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for event in &events {
                render_event_row(ui, event, selection, map_view, selection_events);
            }
        });
}

fn render_event_row(
    ui: &mut egui::Ui,
    event: &SimulationEvent,
    selection: &mut Selection,
    map_view: &mut MapViewState,
    selection_events: &mut EventWriter<SelectionChanged>,
) {
    let filter_type = event.filter_type();
    let (r, g, b) = filter_type.color();
    let bg_color = Color32::from_rgba_unmultiplied(r, g, b, 30);
    let border_color = Color32::from_rgb(r, g, b);

    egui::Frame::none()
        .fill(bg_color)
        .stroke(egui::Stroke::new(1.0, border_color))
        .inner_margin(6.0)
        .outer_margin(2.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("T{}", event.tick))
                        .monospace()
                        .color(Color32::GRAY)
                        .size(11.0)
                );

                ui.separator();

                let type_indicator = match filter_type {
                    EventFilterType::Birth => "+",
                    EventFilterType::Death => "X",
                    EventFilterType::Conflict => "!",
                    EventFilterType::Technology => "*",
                    EventFilterType::Pregnancy => "P",
                    EventFilterType::Building => "B",
                    EventFilterType::Emotional => "E",
                    EventFilterType::Health => "H",
                    EventFilterType::Other => "?",
                };
                ui.label(
                    RichText::new(type_indicator)
                        .color(Color32::from_rgb(r, g, b))
                        .strong()
                );

                ui.vertical(|ui| {
                    ui.label(RichText::new(event.short_description()).strong());
                    ui.label(
                        RichText::new(event.detailed_description())
                            .size(11.0)
                            .color(Color32::LIGHT_GRAY)
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(agent_id) = event.primary_agent_id() {
                        if ui.small_button("Select").clicked() {
                            select_agent(agent_id, event.position, selection, map_view, selection_events);
                        }
                    }

                    if let Some(pos) = event.position {
                        if ui.small_button("Go").clicked() {
                            center_on_position(pos.0, pos.1, map_view);
                        }
                    }
                });
            });
        });
}

fn render_pagination(ui: &mut egui::Ui, timeline: &mut TimelineData) {
    let current_page = timeline.current_page;
    let total_pages = timeline.total_pages();

    ui.separator();

    ui.horizontal(|ui| {
        ui.add_enabled_ui(current_page > 0, |ui| {
            if ui.button("|<").clicked() {
                timeline.first_page();
            }
            if ui.button("<").clicked() {
                timeline.prev_page();
            }
        });

        ui.label(format!("Page {} / {}", current_page + 1, total_pages));

        ui.add_enabled_ui(current_page < total_pages.saturating_sub(1), |ui| {
            if ui.button(">").clicked() {
                timeline.next_page();
            }
            if ui.button(">|").clicked() {
                timeline.last_page();
            }
        });

        ui.separator();

        ui.label("Per page:");
        egui::ComboBox::from_id_salt("events_per_page")
            .selected_text(format!("{}", timeline.events_per_page))
            .width(60.0)
            .show_ui(ui, |ui| {
                for &count in &[25, 50, 100, 200] {
                    if ui.selectable_value(
                        &mut timeline.events_per_page,
                        count,
                        format!("{}", count)
                    ).clicked() {
                        timeline.first_page();
                    }
                }
            });
    });
}

fn select_agent(
    agent_id: Uuid,
    position: Option<(i32, i32)>,
    selection: &mut Selection,
    map_view: &mut MapViewState,
    selection_events: &mut EventWriter<SelectionChanged>,
) {
    let old_selection = selection.current.clone();
    selection.current = EntitySelection::Agent(agent_id);
    selection_events.send(SelectionChanged {
        previous: old_selection,
        current: selection.current.clone(),
    });

    if let Some((x, y)) = position {
        center_on_position(x, y, map_view);
    }
}

fn center_on_position(x: i32, y: i32, map_view: &mut MapViewState) {
    let view_size = (400.0, 400.0);
    map_view.offset = (
        -(x as f32 * TILE_SIZE) + view_size.0 / 2.0,
        -(y as f32 * TILE_SIZE) + view_size.1 / 2.0,
    );
}
