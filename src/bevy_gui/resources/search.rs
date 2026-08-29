// src/bevy_gui/resources/search.rs
//! Search panel state resource.

use bevy::prelude::*;
use uuid::Uuid;

use crate::agents::LifeStage;
use crate::world::{Position, BuildingType, ResourceType};

/// Type of entity to search for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchType {
    #[default]
    All,
    Agents,
    Buildings,
    Resources,
}

/// Health filter for agent searches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HealthFilter {
    #[default]
    Any,
    /// Health < 25%
    Critical,
    /// Health < 50%
    Low,
    /// Health >= 75%
    Healthy,
}

impl HealthFilter {
    pub fn matches(&self, health: f32) -> bool {
        match self {
            HealthFilter::Any => true,
            HealthFilter::Critical => health < 25.0,
            HealthFilter::Low => health < 50.0,
            HealthFilter::Healthy => health >= 75.0,
        }
    }
}

/// A search result
#[derive(Debug, Clone)]
pub enum SearchResult {
    Agent {
        id: Uuid,
        position: (i32, i32),
        life_stage: LifeStage,
        health: f32,
        energy: f32,
    },
    Building {
        position: Position,
        building_type: BuildingType,
        completed: bool,
    },
    Resource {
        position: Position,
        resource_type: ResourceType,
        amount: u32,
        max_amount: u32,
    },
}

impl SearchResult {
    pub fn position(&self) -> (i32, i32) {
        match self {
            SearchResult::Agent { position, .. } => *position,
            SearchResult::Building { position, .. } => (position.x, position.y),
            SearchResult::Resource { position, .. } => (position.x, position.y),
        }
    }
}

/// Search panel state resource
#[derive(Resource)]
pub struct SearchState {
    /// Search query string
    pub query: String,
    /// Type of entity to search for
    pub search_type: SearchType,
    /// Current search results
    pub results: Vec<SearchResult>,
    /// Currently selected result index
    pub selected_result: Option<usize>,
    /// Life stage filter for agents
    pub life_stage_filter: Option<LifeStage>,
    /// Health filter for agents
    pub health_filter: HealthFilter,
    /// Whether a search needs to be performed
    pub needs_search: bool,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            search_type: SearchType::All,
            results: Vec::new(),
            selected_result: None,
            life_stage_filter: None,
            health_filter: HealthFilter::Any,
            needs_search: false,
        }
    }
}

impl SearchState {
    /// Clear search results and query
    pub fn clear(&mut self) {
        self.query.clear();
        self.results.clear();
        self.selected_result = None;
        self.needs_search = false;
    }

    /// Mark that a search needs to be performed
    pub fn request_search(&mut self) {
        self.needs_search = true;
    }

    /// Select the next result
    pub fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }

        self.selected_result = match self.selected_result {
            None => Some(0),
            Some(idx) => Some((idx + 1).min(self.results.len() - 1)),
        };
    }

    /// Select the previous result
    pub fn select_previous(&mut self) {
        if self.results.is_empty() {
            return;
        }

        self.selected_result = match self.selected_result {
            None => Some(0),
            Some(idx) => Some(idx.saturating_sub(1)),
        };
    }

    /// Get the currently selected result
    pub fn get_selected(&self) -> Option<&SearchResult> {
        self.selected_result.and_then(|idx| self.results.get(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_state_defaults() {
        let state = SearchState::default();
        assert!(state.query.is_empty());
        assert_eq!(state.search_type, SearchType::All);
        assert!(state.results.is_empty());
        assert!(state.selected_result.is_none());
        assert!(state.life_stage_filter.is_none());
        assert_eq!(state.health_filter, HealthFilter::Any);
    }

    #[test]
    fn test_health_filter_matches() {
        assert!(HealthFilter::Any.matches(50.0));
        assert!(HealthFilter::Any.matches(10.0));

        assert!(HealthFilter::Critical.matches(20.0));
        assert!(!HealthFilter::Critical.matches(30.0));

        assert!(HealthFilter::Low.matches(40.0));
        assert!(!HealthFilter::Low.matches(60.0));

        assert!(HealthFilter::Healthy.matches(80.0));
        assert!(!HealthFilter::Healthy.matches(50.0));
    }

    #[test]
    fn test_search_state_navigation() {
        let mut state = SearchState::default();

        // Add some mock results
        state.results.push(SearchResult::Agent {
            id: crate::core::dice::name(),
            position: (0, 0),
            life_stage: LifeStage::Adult,
            health: 100.0,
            energy: 100.0,
        });
        state.results.push(SearchResult::Agent {
            id: crate::core::dice::name(),
            position: (1, 1),
            life_stage: LifeStage::Child,
            health: 80.0,
            energy: 50.0,
        });

        assert!(state.selected_result.is_none());

        state.select_next();
        assert_eq!(state.selected_result, Some(0));

        state.select_next();
        assert_eq!(state.selected_result, Some(1));

        state.select_next(); // Should stay at last
        assert_eq!(state.selected_result, Some(1));

        state.select_previous();
        assert_eq!(state.selected_result, Some(0));

        state.select_previous(); // Should stay at first
        assert_eq!(state.selected_result, Some(0));
    }

    #[test]
    fn test_search_state_clear() {
        let mut state = SearchState::default();
        state.query = "test".to_string();
        state.results.push(SearchResult::Agent {
            id: crate::core::dice::name(),
            position: (0, 0),
            life_stage: LifeStage::Adult,
            health: 100.0,
            energy: 100.0,
        });
        state.selected_result = Some(0);

        state.clear();

        assert!(state.query.is_empty());
        assert!(state.results.is_empty());
        assert!(state.selected_result.is_none());
    }
}
