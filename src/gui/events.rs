// src/gui/events.rs
//! GUI-specific event logging and timeline display components.
//!
//! Re-exports core event types and provides GUI-specific functionality
//! like event filtering, logging, and timeline state management.

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

// Re-export core event types for backward compatibility
pub use crate::core::events::{SimulationEvent, SimulationEventType, DeathCause};

/// Maximum number of events to store in the event log
pub const MAX_EVENTS: usize = 1000;

/// Filter types for the timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventFilterType {
    Birth,
    Death,
    Conflict,
    Technology,
    Pregnancy,
    Building,
    Emotional,
    Health,
    Other,
}

impl EventFilterType {
    /// Get all filter types
    pub fn all() -> &'static [EventFilterType] {
        &[
            EventFilterType::Birth,
            EventFilterType::Death,
            EventFilterType::Conflict,
            EventFilterType::Technology,
            EventFilterType::Pregnancy,
            EventFilterType::Building,
            EventFilterType::Emotional,
            EventFilterType::Health,
            EventFilterType::Other,
        ]
    }

    /// Get display name for the filter type
    pub fn display_name(&self) -> &'static str {
        match self {
            EventFilterType::Birth => "Births",
            EventFilterType::Death => "Deaths",
            EventFilterType::Conflict => "Conflicts",
            EventFilterType::Technology => "Technology",
            EventFilterType::Pregnancy => "Pregnancies",
            EventFilterType::Building => "Buildings",
            EventFilterType::Emotional => "Emotional",
            EventFilterType::Health => "Health",
            EventFilterType::Other => "Other",
        }
    }

    /// Get color for the event type (RGB tuple)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            EventFilterType::Birth => (100, 200, 100),      // Green
            EventFilterType::Death => (200, 80, 80),        // Red
            EventFilterType::Conflict => (220, 120, 50),    // Orange
            EventFilterType::Technology => (100, 150, 220), // Blue
            EventFilterType::Pregnancy => (200, 150, 200),  // Pink
            EventFilterType::Building => (180, 160, 100),   // Brown/tan
            EventFilterType::Emotional => (180, 100, 180),  // Purple
            EventFilterType::Health => (200, 200, 80),      // Yellow
            EventFilterType::Other => (150, 150, 150),      // Gray
        }
    }

    /// Get the filter type for a simulation event
    pub fn from_event(event: &SimulationEvent) -> Self {
        match &event.event_type {
            SimulationEventType::Birth { .. } => EventFilterType::Birth,
            SimulationEventType::Death { .. } => EventFilterType::Death,
            SimulationEventType::Conflict { .. } => EventFilterType::Conflict,
            SimulationEventType::TechnologyDiscovered { .. } => EventFilterType::Technology,
            SimulationEventType::Pregnancy { .. } => EventFilterType::Pregnancy,
            SimulationEventType::BuildingStarted { .. } |
            SimulationEventType::BuildingCompleted { .. } => EventFilterType::Building,
            SimulationEventType::MajorEmotionalEvent { .. } => EventFilterType::Emotional,
            SimulationEventType::Collapse { .. } => EventFilterType::Health,
            SimulationEventType::Abandonment { .. } => EventFilterType::Other,
            SimulationEventType::StorehouseDeposit { .. } => EventFilterType::Other,
        }
    }
}

/// Extension trait to add GUI-specific methods to SimulationEvent
pub trait SimulationEventExt {
    /// Get the event filter type for this event
    fn filter_type(&self) -> EventFilterType;
}

impl SimulationEventExt for SimulationEvent {
    fn filter_type(&self) -> EventFilterType {
        EventFilterType::from_event(self)
    }
}

/// Event log storing recent simulation events
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    /// Events stored in chronological order (oldest first)
    events: VecDeque<SimulationEvent>,
    /// Maximum number of events to store
    max_events: usize,
}

impl EventLog {
    /// Create a new event log with default capacity
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_EVENTS),
            max_events: MAX_EVENTS,
        }
    }

    /// Create an event log with custom capacity
    pub fn with_capacity(max_events: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(max_events),
            max_events,
        }
    }

    /// Add an event to the log
    pub fn push(&mut self, event: SimulationEvent) {
        // Remove oldest events if at capacity
        while self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Get all events
    pub fn events(&self) -> &VecDeque<SimulationEvent> {
        &self.events
    }

    /// Get the number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the log is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get events filtered by type and search query
    pub fn filtered_events(
        &self,
        filter_types: &HashSet<EventFilterType>,
        search_query: &str,
        newest_first: bool,
    ) -> Vec<&SimulationEvent> {
        let query_lower = search_query.to_lowercase();

        let mut filtered: Vec<&SimulationEvent> = self.events.iter()
            .filter(|event| {
                // Filter by event type
                if !filter_types.is_empty() && !filter_types.contains(&event.filter_type()) {
                    return false;
                }

                // Filter by search query
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

        // Sort by tick
        if newest_first {
            filtered.sort_by(|a, b| b.tick.cmp(&a.tick));
        } else {
            filtered.sort_by(|a, b| a.tick.cmp(&b.tick));
        }

        filtered
    }

    /// Get the most recent event tick
    pub fn latest_tick(&self) -> Option<u32> {
        self.events.back().map(|e| e.tick)
    }
}

/// State for the timeline panel
#[derive(Debug, Clone)]
pub struct TimelineState {
    /// Event log
    pub event_log: EventLog,
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

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            event_log: EventLog::new(),
            filter_types: HashSet::new(),
            search_query: String::new(),
            newest_first: true,
            events_per_page: 50,
            current_page: 0,
            auto_scroll: true,
        }
    }
}

impl TimelineState {
    /// Create a new timeline state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event to the timeline
    pub fn add_event(&mut self, event: SimulationEvent) {
        self.event_log.push(event);
        // Reset to first page when new events arrive (if auto-scroll is on)
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
        self.current_page = 0; // Reset pagination when filter changes
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filter_types.clear();
        self.current_page = 0;
    }

    /// Get filtered and paginated events
    pub fn get_page_events(&self) -> Vec<&SimulationEvent> {
        let all_filtered = self.event_log.filtered_events(
            &self.filter_types,
            &self.search_query,
            self.newest_first,
        );

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
        let filtered_count = self.event_log.filtered_events(
            &self.filter_types,
            &self.search_query,
            self.newest_first,
        ).len();

        if filtered_count == 0 {
            1
        } else {
            (filtered_count + self.events_per_page - 1) / self.events_per_page
        }
    }

    /// Get total filtered event count
    pub fn filtered_count(&self) -> usize {
        self.event_log.filtered_events(
            &self.filter_types,
            &self.search_query,
            self.newest_first,
        ).len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_event_log_capacity() {
        let mut log = EventLog::with_capacity(5);

        for i in 0..10 {
            log.push(SimulationEvent::new(
                i,
                SimulationEventType::Birth {
                    mother_id: Uuid::new_v4(),
                    child_id: Uuid::new_v4(),
                    father_id: None,
                },
                None,
            ));
        }

        assert_eq!(log.len(), 5);
        assert_eq!(log.events().front().unwrap().tick, 5);
        assert_eq!(log.events().back().unwrap().tick, 9);
    }

    #[test]
    fn test_event_filtering() {
        let mut log = EventLog::new();

        log.push(SimulationEvent::new(
            1,
            SimulationEventType::Birth {
                mother_id: Uuid::new_v4(),
                child_id: Uuid::new_v4(),
                father_id: None,
            },
            None,
        ));

        log.push(SimulationEvent::new(
            2,
            SimulationEventType::Death {
                agent_id: Uuid::new_v4(),
                cause: DeathCause::OldAge,
            },
            None,
        ));

        let mut filter = HashSet::new();
        filter.insert(EventFilterType::Birth);

        let filtered = log.filtered_events(&filter, "", true);
        assert_eq!(filtered.len(), 1);
        assert!(matches!(filtered[0].event_type, SimulationEventType::Birth { .. }));
    }

    #[test]
    fn test_timeline_pagination() {
        let mut state = TimelineState::new();
        state.events_per_page = 3;

        for i in 0..10 {
            state.add_event(SimulationEvent::new(
                i,
                SimulationEventType::Birth {
                    mother_id: Uuid::new_v4(),
                    child_id: Uuid::new_v4(),
                    father_id: None,
                },
                None,
            ));
        }

        assert_eq!(state.total_pages(), 4); // 10 events / 3 per page = 4 pages
        assert_eq!(state.get_page_events().len(), 3);

        state.next_page();
        assert_eq!(state.current_page, 1);
        assert_eq!(state.get_page_events().len(), 3);

        state.last_page();
        assert_eq!(state.current_page, 3);
        assert_eq!(state.get_page_events().len(), 1); // Last page has 1 event
    }

    #[test]
    fn test_filter_type_from_event() {
        let birth_event = SimulationEvent::new(
            1,
            SimulationEventType::Birth {
                mother_id: Uuid::new_v4(),
                child_id: Uuid::new_v4(),
                father_id: None,
            },
            None,
        );
        assert_eq!(EventFilterType::from_event(&birth_event), EventFilterType::Birth);

        let death_event = SimulationEvent::new(
            2,
            SimulationEventType::Death {
                agent_id: Uuid::new_v4(),
                cause: DeathCause::Starvation,
            },
            None,
        );
        assert_eq!(EventFilterType::from_event(&death_event), EventFilterType::Death);
    }
}
