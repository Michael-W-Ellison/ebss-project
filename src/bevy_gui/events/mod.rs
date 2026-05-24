// src/bevy_gui/events/mod.rs
//! Bevy Events for GUI communication.

use bevy::prelude::*;

use crate::bevy_gui::resources::EntitySelection;

/// Commands sent to the simulation thread
#[derive(Event, Debug, Clone)]
pub enum SimulationCommand {
    Play,
    Pause,
    Step,
    SetSpeed(f32),
    SelectEntity(EntitySelection),
    DeselectAll,
    SaveGame(String),
    LoadGame(String),
}

/// Fired when entity selection changes
#[derive(Event, Debug, Clone)]
pub struct SelectionChanged {
    pub previous: EntitySelection,
    pub current: EntitySelection,
}

/// Fired when map view changes (zoom, pan)
#[derive(Event, Debug, Clone)]
pub struct MapViewChanged;

/// Fired when a panel is toggled
#[derive(Event, Debug, Clone)]
pub struct PanelToggled {
    pub panel: PanelType,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Inspector,
    Statistics,
    Legend,
    TechTree,
    Timeline,
    RelationshipGraph,
    Search,
    KeyboardHelp,
}

/// Request to center the map on a position
#[derive(Event, Debug, Clone)]
pub struct CenterMapRequest {
    pub x: i32,
    pub y: i32,
}
