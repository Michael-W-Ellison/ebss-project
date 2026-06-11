// src/bevy_gui/resources/timeline.rs
//! Timeline panel state resource.

use bevy::prelude::*;
use std::collections::HashSet;
use crate::gui::events::{EventFilterType, SimulationEvent, SimulationEventExt};

/// Timeline panel data and UI state
#[derive(Resource)]
pub struct TimelineData {
    /// Event log storing recent simulation events
    pub event_log: Vec<SimulationEvent>,
    /// Maximum number of events to store
    pub max_events: usize,
    /// Active filter types (empty = show all)
    pub filter_types: HashSet<EventFilterType>,
    /// Search query for filtering events
    pub search_query: String,
    /// Sort order (true = newest first)
    pub newest_first: bool,
    /// Number of events to show per page
    pub events_per_page: usize,
    /// Current page (0-indexed)
    pub current_page: usize,
    /// Whether to auto-scroll to new events
    pub auto_scroll: bool,
}

impl Default for TimelineData {
    fn default() -> Self {
        Self {
            event_log: Vec::with_capacity(1000),
            max_events: 1000,
            filter_types: HashSet::new(),
            search_query: String::new(),
            newest_first: true,
            events_per_page: 50,
            current_page: 0,
            auto_scroll: true,
        }
    }
}

impl TimelineData {
    /// Add events from a snapshot
    pub fn add_events(&mut self, events: Vec<SimulationEvent>) {
        for event in events {
            if self.event_log.len() >= self.max_events {
                self.event_log.remove(0);
            }
            self.event_log.push(event);
        }
        if self.auto_scroll && self.newest_first {
            self.current_page = 0;
        }
    }

    /// Toggle a filter type
    pub fn toggle_filter(&mut self, filter_type: EventFilterType) {
        if self.filter_types.contains(&filter_type) {
            self.filter_types.remove(&filter_type);
        } else {
            self.filter_types.insert(filter_type);
        }
        self.current_page = 0;
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filter_types.clear();
        self.current_page = 0;
    }

    /// Get filtered events
    pub fn filtered_events(&self) -> Vec<&SimulationEvent> {
        let query_lower = self.search_query.to_lowercase();

        let mut filtered: Vec<&SimulationEvent> = self.event_log.iter()
            .filter(|event| {
                if !self.filter_types.is_empty() && !self.filter_types.contains(&event.filter_type()) {
                    return false;
                }

                if !query_lower.is_empty() {
                    let description = event.short_description().to_lowercase();
                    let detailed = event.detailed_description().to_lowercase();
                    if !description.contains(&query_lower) && !detailed.contains(&query_lower) {
                        return false;
                    }
                }

                true
            })
            .collect();

        if self.newest_first {
            filtered.sort_by(|a, b| b.tick.cmp(&a.tick));
        } else {
            filtered.sort_by(|a, b| a.tick.cmp(&b.tick));
        }

        filtered
    }

    /// Get paginated events
    pub fn get_page_events(&self) -> Vec<&SimulationEvent> {
        let all_filtered = self.filtered_events();
        let start = self.current_page * self.events_per_page;
        let end = (start + self.events_per_page).min(all_filtered.len());

        if start >= all_filtered.len() {
            Vec::new()
        } else {
            all_filtered[start..end].to_vec()
        }
    }

    /// Get total number of pages
    pub fn total_pages(&self) -> usize {
        let filtered_count = self.filtered_events().len();
        if filtered_count == 0 {
            1
        } else {
            filtered_count.div_ceil(self.events_per_page)
        }
    }

    /// Get total filtered event count
    pub fn filtered_count(&self) -> usize {
        self.filtered_events().len()
    }

    /// Go to next page
    pub fn next_page(&mut self) {
        let max_page = self.total_pages().saturating_sub(1);
        self.current_page = (self.current_page + 1).min(max_page);
    }

    /// Go to previous page
    pub fn prev_page(&mut self) {
        self.current_page = self.current_page.saturating_sub(1);
    }

    /// Go to first page
    pub fn first_page(&mut self) {
        self.current_page = 0;
    }

    /// Go to last page
    pub fn last_page(&mut self) {
        self.current_page = self.total_pages().saturating_sub(1);
    }
}
