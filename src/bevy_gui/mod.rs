// src/bevy_gui/mod.rs
//! Bevy-based GUI for EBSS using bevy_egui.
//!
//! This module provides an ECS-based GUI implementation that leverages Bevy's
//! system scheduling, change detection, and event systems.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin ebss_bevy_gui --features bevy_gui
//! ```

pub mod resources;
pub mod events;
pub mod systems;
pub mod ui;

#[cfg(test)]
mod tests;

use bevy::prelude::*;

pub use resources::*;
pub use events::*;
pub use systems::{SimulationBridge, BridgeError};

/// Plugin that sets up the EBSS GUI systems and resources.
///
/// Note: The SimulationBridge resource must be inserted separately before
/// adding this plugin, as it contains channels that can't be cloned.
pub struct EbssGuiPlugin;

impl Plugin for EbssGuiPlugin {
    fn build(&self, app: &mut App) {
        app
            // Insert default resources
            .insert_resource(SimulationControl::default())
            .insert_resource(CurrentSnapshot::default())
            .insert_resource(Selection::default())
            .insert_resource(MapViewState::default())
            .insert_resource(PanelVisibility::default())
            .insert_resource(StatisticsData::default())
            .insert_resource(StatisticsHistory::default())
            .insert_resource(NotificationQueue::default())
            .insert_resource(SelectedEntityData::default())
            .insert_resource(TechTreeData::default())
            .insert_resource(RelationshipGraphData::default())
            .insert_resource(InspectorState::default())
            .insert_resource(TimelineData::default())
            .insert_resource(SimulationErrors::default())
            .insert_resource(SaveLoadState::default())
            .insert_resource(SearchState::default())
            // Events
            .add_event::<SimulationCommand>()
            .add_event::<SelectionChanged>()
            .add_event::<MapViewChanged>()
            .add_event::<PanelToggled>()
            .add_event::<CenterMapRequest>()
            .add_event::<ShutdownRequested>()
            // Add systems in First for bridge communication
            .add_systems(First, systems::receive_snapshots_system)
            .add_systems(First, systems::receive_errors_system)
            // Input handling in PreUpdate
            .add_systems(PreUpdate, systems::keyboard_input_system)
            .add_systems(PreUpdate, systems::map_pan_system)
            // UI rendering in Update
            .add_systems(Update, ui::render_menu_bar)
            .add_systems(Update, ui::render_controls_panel)
            .add_systems(Update, ui::render_map_placeholder)
            .add_systems(Update, ui::render_notifications)
            .add_systems(Update, ui::render_keyboard_help)
            // Panel systems
            .add_systems(Update, ui::render_inspector_panel)
            .add_systems(Update, ui::render_legend_panel)
            .add_systems(Update, ui::render_statistics_panel)
            .add_systems(Update, ui::render_tech_tree_panel)
            .add_systems(Update, ui::render_timeline_panel)
            .add_systems(Update, ui::render_relationship_graph_panel)
            // Dialog systems
            .add_systems(Update, ui::render_save_dialog)
            .add_systems(Update, ui::render_load_dialog)
            .add_systems(Update, ui::render_search_panel)
            // Search system
            .add_systems(Update, ui::search_system)
            // Command sending in PostUpdate
            .add_systems(PostUpdate, systems::send_commands_system)
            .add_systems(PostUpdate, systems::entity_data_system)
            .add_systems(PostUpdate, systems::tech_tree_data_system)
            .add_systems(PostUpdate, systems::relationship_graph_data_system)
            .add_systems(PostUpdate, systems::selection_sync_system)
            // Shutdown handling
            .add_systems(PostUpdate, systems::handle_shutdown_requests)
            .add_systems(Last, systems::on_app_exit);
    }
}
