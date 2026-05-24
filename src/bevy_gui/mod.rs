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

use bevy::prelude::*;

pub use resources::*;
pub use events::*;
pub use systems::SimulationBridge;

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
            .insert_resource(NotificationQueue::default())
            .insert_resource(SelectedEntityData::default())
            .insert_resource(TechTreeData::default())
            .insert_resource(RelationshipGraphData::default())
            // Events
            .add_event::<SimulationCommand>()
            .add_event::<SelectionChanged>()
            .add_event::<MapViewChanged>()
            .add_event::<PanelToggled>()
            .add_event::<CenterMapRequest>()
            // Add systems in First for bridge communication
            .add_systems(First, systems::receive_snapshots_system)
            // Input handling in PreUpdate
            .add_systems(PreUpdate, systems::keyboard_input_system)
            .add_systems(PreUpdate, systems::map_pan_system)
            // UI rendering in Update
            .add_systems(Update, ui::render_menu_bar)
            .add_systems(Update, ui::render_controls_panel)
            .add_systems(Update, ui::render_map_placeholder)
            .add_systems(Update, ui::render_notifications)
            .add_systems(Update, ui::render_keyboard_help)
            // Command sending in PostUpdate
            .add_systems(PostUpdate, systems::send_commands_system)
            .add_systems(PostUpdate, systems::entity_data_system)
            .add_systems(PostUpdate, systems::tech_tree_data_system);
    }
}
