// src/bevy_gui/resources/mod.rs
//! Bevy Resources for the EBSS GUI.
//!
//! These resources replace the monolithic GuiState with focused, single-responsibility
//! data containers that leverage Bevy's change detection.

mod simulation_control;
mod snapshot;
mod selection;
mod map_view;
mod panels;
mod statistics;
mod notifications;
mod inspector;
mod relationship_graph;
mod timeline;
mod simulation_errors;
mod save_load;
mod search;

pub use simulation_control::*;
pub use snapshot::*;
pub use selection::*;
pub use map_view::*;
pub use panels::*;
pub use statistics::*;
pub use notifications::*;
pub use inspector::*;
pub use relationship_graph::*;
pub use timeline::*;
pub use simulation_errors::*;
pub use save_load::*;
pub use search::*;
