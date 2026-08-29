// src/bevy_gui/resources/relationship_graph.rs
//! Relationship graph panel state resource.

use bevy::prelude::*;
use std::collections::BTreeMap;
use uuid::Uuid;
use crate::gui::state::{RelationshipGraphSnapshot, RelationshipFilter, GraphLayoutMode, GraphNodePosition};

/// Relationship graph panel data and UI state
#[derive(Resource)]
pub struct RelationshipGraphData {
    pub snapshot: Option<RelationshipGraphSnapshot>,
    /// Currently selected agent in the graph
    pub selected_agent: Option<Uuid>,
    /// Currently hovered agent
    pub hovered_agent: Option<Uuid>,
    /// Agent to focus on (shows only their connections)
    pub focus_agent: Option<Uuid>,
    /// Current zoom level
    pub zoom: f32,
    /// Pan offset
    pub offset: (f32, f32),
    /// Whether to show labels
    pub show_labels: bool,
    /// Relationship filter settings
    pub filter: RelationshipFilter,
    /// Current layout mode
    pub layout_mode: GraphLayoutMode,
    /// Computed node positions
    pub node_positions: BTreeMap<Uuid, GraphNodePosition>,
    /// Whether layout needs to be recomputed
    pub needs_layout: bool,
    /// Number of force-directed iterations run
    pub layout_iterations: usize,
}

impl Default for RelationshipGraphData {
    fn default() -> Self {
        Self {
            snapshot: None,
            selected_agent: None,
            hovered_agent: None,
            focus_agent: None,
            zoom: 1.0,
            offset: (0.0, 0.0),
            show_labels: true,
            filter: RelationshipFilter::default(),
            layout_mode: GraphLayoutMode::default(),
            node_positions: BTreeMap::new(),
            needs_layout: true,
            layout_iterations: 0,
        }
    }
}

impl RelationshipGraphData {
    /// Reset view to defaults
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.offset = (0.0, 0.0);
    }

    /// Request layout recomputation
    pub fn request_layout(&mut self) {
        self.needs_layout = true;
        self.layout_iterations = 0;
    }
}
