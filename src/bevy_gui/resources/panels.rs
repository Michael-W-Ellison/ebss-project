// src/bevy_gui/resources/panels.rs
//! Panel visibility state resource.

use bevy::prelude::*;

/// Panel visibility toggles
#[derive(Resource)]
pub struct PanelVisibility {
    pub inspector: bool,
    pub statistics: bool,
    pub legend: bool,
    pub tech_tree: bool,
    pub timeline: bool,
    pub relationship_graph: bool,
    pub search: bool,
    pub keyboard_help: bool,
    pub save_dialog: bool,
    pub load_dialog: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            inspector: true,
            statistics: true,
            legend: false,
            tech_tree: false,
            timeline: false,
            relationship_graph: false,
            search: false,
            keyboard_help: false,
            save_dialog: false,
            load_dialog: false,
        }
    }
}

impl PanelVisibility {
    pub fn toggle_inspector(&mut self) {
        self.inspector = !self.inspector;
    }

    pub fn toggle_statistics(&mut self) {
        self.statistics = !self.statistics;
    }

    pub fn toggle_legend(&mut self) {
        self.legend = !self.legend;
    }

    pub fn toggle_tech_tree(&mut self) {
        self.tech_tree = !self.tech_tree;
    }

    pub fn toggle_timeline(&mut self) {
        self.timeline = !self.timeline;
    }

    pub fn toggle_relationship_graph(&mut self) {
        self.relationship_graph = !self.relationship_graph;
    }

    pub fn toggle_keyboard_help(&mut self) {
        self.keyboard_help = !self.keyboard_help;
    }

    pub fn close_dialogs(&mut self) {
        self.keyboard_help = false;
        self.search = false;
        self.save_dialog = false;
        self.load_dialog = false;
    }

    pub fn has_modal_open(&self) -> bool {
        self.keyboard_help || self.search || self.save_dialog || self.load_dialog
    }
}
