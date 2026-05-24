// src/bevy_gui/resources/inspector.rs
//! Inspector panel state resource.

use bevy::prelude::*;

/// Inspector tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorTab {
    #[default]
    Overview,
    Drives,
    Skills,
    Inventory,
    Relationships,
    Goals,
}

impl InspectorTab {
    pub fn all() -> &'static [InspectorTab] {
        &[
            InspectorTab::Overview,
            InspectorTab::Drives,
            InspectorTab::Skills,
            InspectorTab::Inventory,
            InspectorTab::Relationships,
            InspectorTab::Goals,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            InspectorTab::Overview => "Overview",
            InspectorTab::Drives => "Drives",
            InspectorTab::Skills => "Skills",
            InspectorTab::Inventory => "Inventory",
            InspectorTab::Relationships => "Relations",
            InspectorTab::Goals => "Goals",
        }
    }
}

/// Inspector panel state
#[derive(Resource, Default)]
pub struct InspectorState {
    pub active_tab: InspectorTab,
}

impl InspectorState {
    pub fn set_tab(&mut self, tab: InspectorTab) {
        self.active_tab = tab;
    }
}
