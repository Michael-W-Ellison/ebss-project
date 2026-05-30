// src/bevy_gui/resources/inspector.rs
//! Inspector panel state resource.

use bevy::prelude::*;

use super::EntitySelection;

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
            InspectorTab::Inventory => "Items",
            InspectorTab::Relationships => "Social",
            InspectorTab::Goals => "Goals",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            InspectorTab::Overview => "👤",
            InspectorTab::Drives => "🎯",
            InspectorTab::Skills => "⚒",
            InspectorTab::Inventory => "🎒",
            InspectorTab::Relationships => "❤",
            InspectorTab::Goals => "🏁",
        }
    }
}

/// Inspector panel state
#[derive(Resource, Default)]
pub struct InspectorState {
    pub active_tab: InspectorTab,
    pub pinned: bool,
    pub pinned_selection: Option<EntitySelection>,
    pub show_detailed_drives: bool,
    pub show_completed_goals: bool,
    pub skills_sort_by_level: bool,
    pub relationships_sort_by_strength: bool,
}

impl InspectorState {
    pub fn set_tab(&mut self, tab: InspectorTab) {
        self.active_tab = tab;
    }

    pub fn unpin(&mut self) {
        self.pinned = false;
        self.pinned_selection = None;
    }
}
