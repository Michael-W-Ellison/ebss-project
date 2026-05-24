// src/bevy_gui/ui/panels/mod.rs
//! UI panel systems for the Bevy GUI.

mod legend;
mod inspector;
mod statistics;
mod tech_tree;
mod timeline;
mod relationship_graph;
mod save_load;

pub use legend::render_legend_panel;
pub use inspector::render_inspector_panel;
pub use statistics::render_statistics_panel;
pub use tech_tree::render_tech_tree_panel;
pub use timeline::render_timeline_panel;
pub use relationship_graph::render_relationship_graph_panel;
pub use save_load::{render_save_dialog, render_load_dialog};
