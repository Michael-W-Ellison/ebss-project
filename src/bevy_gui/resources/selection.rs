// src/bevy_gui/resources/selection.rs
//! Entity selection state resource.

use bevy::prelude::*;
use uuid::Uuid;

use crate::world::Position;

/// Types of entities that can be selected
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum EntitySelection {
    #[default]
    None,
    Agent(Uuid),
    Building(Position),
    Resource(Position),
    Terrain(Position),
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

    pub fn select_agent(&mut self, id: Uuid) {
        self.current = EntitySelection::Agent(id);
    }

    pub fn select_building(&mut self, pos: (i32, i32)) {
        self.current = EntitySelection::Building(Position { x: pos.0, y: pos.1 });
    }

    pub fn select_resource(&mut self, pos: (i32, i32)) {
        self.current = EntitySelection::Resource(Position { x: pos.0, y: pos.1 });
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
