// src/bevy_gui/ui/panels/mod.rs
//! UI panel systems for the Bevy GUI.

mod legend;
mod inspector;
mod statistics;

pub use legend::render_legend_panel;
pub use inspector::render_inspector_panel;
pub use statistics::render_statistics_panel;
