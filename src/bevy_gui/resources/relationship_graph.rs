// src/bevy_gui/resources/relationship_graph.rs
//! Relationship graph panel state resource.

use bevy::prelude::*;
use crate::gui::state::RelationshipGraphSnapshot;

/// Relationship graph panel data
#[derive(Resource, Default)]
pub struct RelationshipGraphData {
    pub snapshot: Option<RelationshipGraphSnapshot>,
}
