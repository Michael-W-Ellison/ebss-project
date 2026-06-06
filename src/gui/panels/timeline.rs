// src/gui/panels/timeline.rs
//! Timeline panel showing simulation event history.

use egui::{Ui, Color32, RichText, ScrollArea};
use uuid::Uuid;

use crate::gui::state::{GuiState, EntitySelection};
use crate::gui::events::{EventFilterType, SimulationEvent, SimulationEventExt};

const TILE_SIZE: f32 = 12.0;

/// Render the timeline panel
pub fn render_timeline(ui: &mut Ui, state: &mut GuiState) {
    ui.heading("Event Timeline");
    ui.separator();

    // Filter controls
    render_filter_bar(ui, state);
    ui.separator();

    // Search bar
    render_search_bar(ui, state);
    ui.separator();

    // Event list
    render_event_list(ui, state);

    // Pagination controls
    render_pagination(ui, state);
}

/// Render the filter checkboxes
fn render_filter_bar(ui: &mut Ui, state: &mut GuiState) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Filters:");

        for filter_type in EventFilterType::all() {
            let is_active = state.timeline_state.filter_types.contains(filter_type);
            let (r, g, b) = filter_type.color();
            let color = if is_active {
                Color32::from_rgb(r, g, b)
            } else {
                Color32::from_rgb(r / 2 + 50, g / 2 + 50, b / 2 + 50)
            };

            let text = RichText::new(filter_type.display_name()).color(color);
            if ui.selectable_label(is_active, text).clicked() {
                state.timeline_state.toggle_filter(*filter_type);
            }
        }

        ui.separator();

        if ui.small_button("Clear").clicked() {
            state.timeline_state.clear_filters();
        }
    });

    // Sort order toggle
    ui.horizontal(|ui| {
        ui.label("Sort:");
        if ui.selectable_label(state.timeline_state.newest_first, "Newest First").clicked() {
            state.timeline_state.newest_first = true;
            state.timeline_state.first_page();
        }
        if ui.selectable_label(!state.timeline_state.newest_first, "Oldest First").clicked() {
            state.timeline_state.newest_first = false;
            state.timeline_state.first_page();
        }

        ui.separator();

        ui.checkbox(&mut state.timeline_state.auto_scroll, "Auto-scroll");
    });
}

/// Render the search bar
fn render_search_bar(ui: &mut Ui, state: &mut GuiState) {
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.text_edit_singleline(&mut state.timeline_state.search_query);
        if response.changed() {
            state.timeline_state.first_page();
        }

        if ui.small_button("X").clicked() {
            state.timeline_state.search_query.clear();
            state.timeline_state.first_page();
        }

        ui.separator();

        // Event count
        let total = state.timeline_state.event_log.len();
        let filtered = state.timeline_state.filtered_count();
        if filtered == total {
            ui.label(format!("{} events", total));
        } else {
            ui.label(format!("{} / {} events", filtered, total));
        }
    });
}

/// Render the list of events
fn render_event_list(ui: &mut Ui, state: &mut GuiState) {
    // Clone events to avoid borrow conflict with state
    let events: Vec<SimulationEvent> = state.timeline_state.get_page_events()
        .iter()
        .map(|e| (*e).clone())
        .collect();

    let log_empty = state.timeline_state.event_log.is_empty();

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

    let available_height = ui.available_height() - 40.0; // Leave room for pagination

    ScrollArea::vertical()
        .max_height(available_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for event in &events {
                render_event_row(ui, state, event);
            }
        });
}

/// Render a single event row
fn render_event_row(ui: &mut Ui, state: &mut GuiState, event: &SimulationEvent) {
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
                // Tick number
                ui.label(
                    RichText::new(format!("T{}", event.tick))
                        .monospace()
                        .color(Color32::GRAY)
                        .size(11.0)
                );

                ui.separator();

                // Event icon/type indicator
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

                // Event description
                ui.vertical(|ui| {
                    ui.label(RichText::new(event.short_description()).strong());
                    ui.label(
                        RichText::new(event.detailed_description())
                            .size(11.0)
                            .color(Color32::LIGHT_GRAY)
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Action buttons
                    if let Some(agent_id) = event.primary_agent_id() {
                        if ui.small_button("Select").clicked() {
                            select_agent(state, agent_id, event.position);
                        }
                    }

                    if let Some(pos) = event.position {
                        if ui.small_button("Go").clicked() {
                            center_on_position(state, pos.0, pos.1);
                        }
                    }
                });
            });
        });
}

/// Render pagination controls
fn render_pagination(ui: &mut Ui, state: &mut GuiState) {
    let current_page = state.timeline_state.current_page;
    let total_pages = state.timeline_state.total_pages();

    ui.separator();

    ui.horizontal(|ui| {
        ui.add_enabled_ui(current_page > 0, |ui| {
            if ui.button("|<").clicked() {
                state.timeline_state.first_page();
            }
            if ui.button("<").clicked() {
                state.timeline_state.prev_page();
            }
        });

        ui.label(format!("Page {} / {}", current_page + 1, total_pages));

        ui.add_enabled_ui(current_page < total_pages.saturating_sub(1), |ui| {
            if ui.button(">").clicked() {
                state.timeline_state.next_page();
            }
            if ui.button(">|").clicked() {
                state.timeline_state.last_page();
            }
        });

        ui.separator();

        // Events per page selector
        ui.label("Per page:");
        egui::ComboBox::from_id_salt("events_per_page")
            .selected_text(format!("{}", state.timeline_state.events_per_page))
            .width(60.0)
            .show_ui(ui, |ui| {
                for &count in &[25, 50, 100, 200] {
                    if ui.selectable_value(
                        &mut state.timeline_state.events_per_page,
                        count,
                        format!("{}", count)
                    ).clicked() {
                        state.timeline_state.first_page();
                    }
                }
            });
    });
}

/// Select an agent and optionally center on their position
fn select_agent(state: &mut GuiState, agent_id: Uuid, position: Option<(i32, i32)>) {
    state.selected = EntitySelection::Agent(agent_id);

    if let Some((x, y)) = position {
        center_on_position(state, x, y);
    } else if let Some(snapshot) = &state.latest_snapshot {
        // Try to find agent position from snapshot
        if let Some(agent) = snapshot.population.agents.iter().find(|a| a.id == agent_id) {
            center_on_position(state, agent.position.0, agent.position.1);
        }
    }
}

/// Center the map on a position
fn center_on_position(state: &mut GuiState, x: i32, y: i32) {
    // Estimate view size (this will be approximate without the actual ui dimensions)
    let view_size = (400.0, 400.0);
    state.center_on_position(x, y, TILE_SIZE, view_size);
}

/// Format tick as time string (e.g., "Day 5, 14:30")
#[allow(dead_code)]
pub fn format_tick_as_time(tick: u32) -> String {
    const TICKS_PER_HOUR: u32 = 60;
    const TICKS_PER_DAY: u32 = TICKS_PER_HOUR * 24;

    let day = tick / TICKS_PER_DAY + 1;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    let minute = (tick % TICKS_PER_HOUR) * 60 / TICKS_PER_HOUR;

    format!("Day {}, {:02}:{:02}", day, hour, minute)
}
