// src/bevy_gui/resources/selection.rs
//! Entity selection state resource.

use bevy::prelude::*;
use uuid::Uuid;

use crate::world::Position;

/// Types of entities that can be selected
#[derive(Debug, Clone, PartialEq)]
pub enum EntitySelection {
    None,
    Agent(Uuid),
    Building(Position),
    Resource(Position),
    Terrain(Position),
}

impl Default for EntitySelection {
    fn default() -> Self {
        EntitySelection::None
    }
}

impl EntitySelection {
    pub fn is_none(&self) -> bool {
        matches!(self, EntitySelection::None)
    }

    pub fn is_agent(&self) -> bool {
        matches!(self, EntitySelection::Agent(_))
    }

    pub fn agent_id(&self) -> Option<Uuid> {
        match self {
            EntitySelection::Agent(id) => Some(*id),
            _ => None,
        }
    }
}

/// Current selection state
#[derive(Resource, Default)]
pub struct Selection {
    pub current: EntitySelection,
    pub follow_selected: bool,
}

impl Selection {
    pub fn select(&mut self, selection: EntitySelection) {
        self.current = selection;
    }

    pub fn deselect(&mut self) {
        self.current = EntitySelection::None;
        self.follow_selected = false;
    }

    pub fn toggle_follow(&mut self) {
        if !self.current.is_none() {
            self.follow_selected = !self.follow_selected;
        }
    }
}
