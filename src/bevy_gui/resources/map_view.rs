// src/bevy_gui/resources/map_view.rs
//! Map view state resources.

use bevy::prelude::*;

use crate::agents::{LifeStage, Gender, JobCategory};

/// Map layer visibility toggles
#[derive(Debug, Clone)]
pub struct MapLayers {
    pub terrain: bool,
    pub resources: bool,
    pub buildings: bool,
    pub agents: bool,
    pub grid: bool,
}

impl Default for MapLayers {
    fn default() -> Self {
        Self {
            terrain: true,
            resources: true,
            buildings: true,
            agents: true,
            grid: false,
        }
    }
}

/// Minimap corner position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MinimapPosition {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

/// Minimap display settings
#[derive(Debug, Clone)]
pub struct MinimapSettings {
    pub enabled: bool,
    pub size: f32,
    pub opacity: f32,
    pub position: MinimapPosition,
    pub show_resources: bool,
    pub show_buildings: bool,
    pub show_agents: bool,
}

impl Default for MinimapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            size: 150.0,
            opacity: 0.85,
            position: MinimapPosition::TopRight,
            show_resources: true,
            show_buildings: true,
            show_agents: true,
        }
    }
}

/// Agent filter settings for map display
#[derive(Debug, Clone)]
pub struct AgentMapFilter {
    pub show_infant: bool,
    pub show_child: bool,
    pub show_adolescent: bool,
    pub show_adult: bool,
    pub show_elderly: bool,
    pub show_male: bool,
    pub show_female: bool,
    pub show_sleeping: bool,
    pub show_idle: bool,
    pub show_mining: bool,
    pub show_building: bool,
    pub show_crafting: bool,
    pub show_farming: bool,
    pub show_hunting: bool,
    pub show_fishing: bool,
    pub show_cooking: bool,
    pub show_social: bool,
    pub show_exploring: bool,
    pub show_caretaking: bool,
    pub show_gathering: bool,
    pub show_labor: bool,
}

impl Default for AgentMapFilter {
    fn default() -> Self {
        Self {
            show_infant: true,
            show_child: true,
            show_adolescent: true,
            show_adult: true,
            show_elderly: true,
            show_male: true,
            show_female: true,
            show_sleeping: true,
            show_idle: true,
            show_mining: true,
            show_building: true,
            show_crafting: true,
            show_farming: true,
            show_hunting: true,
            show_fishing: true,
            show_cooking: true,
            show_social: true,
            show_exploring: true,
            show_caretaking: true,
            show_gathering: true,
            show_labor: true,
        }
    }
}

impl AgentMapFilter {
    pub fn is_filtering(&self) -> bool {
        !(self.show_infant && self.show_child && self.show_adolescent
            && self.show_adult && self.show_elderly
            && self.show_male && self.show_female
            && self.show_sleeping && self.show_idle
            && self.show_mining && self.show_building && self.show_crafting
            && self.show_farming && self.show_hunting && self.show_fishing
            && self.show_cooking && self.show_social && self.show_exploring
            && self.show_caretaking && self.show_gathering && self.show_labor)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn show_life_stage(&self, stage: LifeStage) -> bool {
        match stage {
            LifeStage::Infant => self.show_infant,
            LifeStage::Child => self.show_child,
            LifeStage::Adolescent => self.show_adolescent,
            LifeStage::Adult => self.show_adult,
            LifeStage::Elderly => self.show_elderly,
        }
    }

    pub fn show_gender(&self, gender: Gender) -> bool {
        match gender {
            Gender::Male => self.show_male,
            Gender::Female => self.show_female,
        }
    }

    pub fn show_job(&self, job: Option<JobCategory>) -> bool {
        match job {
            None => self.show_idle,
            Some(JobCategory::Mining) => self.show_mining,
            Some(JobCategory::Building) => self.show_building,
            Some(JobCategory::Crafting) => self.show_crafting,
            Some(JobCategory::Farming) => self.show_farming,
            Some(JobCategory::Hunting) => self.show_hunting,
            Some(JobCategory::Fishing) => self.show_fishing,
            Some(JobCategory::Cooking) => self.show_cooking,
            Some(JobCategory::Social) => self.show_social,
            Some(JobCategory::Exploring) => self.show_exploring,
            Some(JobCategory::Caretaking) => self.show_caretaking,
            Some(JobCategory::Gathering) => self.show_gathering,
            Some(JobCategory::Labor) => self.show_labor,
        }
    }
}

/// Map view camera and display state
#[derive(Resource)]
pub struct MapViewState {
    pub zoom: f32,
    pub offset: (f32, f32),
    pub layers: MapLayers,
    pub minimap: MinimapSettings,
    pub agent_filter: AgentMapFilter,
}

impl Default for MapViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: (0.0, 0.0),
            layers: MapLayers::default(),
            minimap: MinimapSettings::default(),
            agent_filter: AgentMapFilter::default(),
        }
    }
}

impl MapViewState {
    pub const MIN_ZOOM: f32 = 0.25;
    pub const MAX_ZOOM: f32 = 4.0;
    pub const TILE_SIZE: f32 = 12.0;

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + 0.25).min(Self::MAX_ZOOM);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom - 0.25).max(Self::MIN_ZOOM);
    }

    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.offset = (0.0, 0.0);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.offset.0 += delta_x;
        self.offset.1 += delta_y;
    }

    /// Center the view on a specific tile coordinate
    pub fn center_on(&mut self, tile_x: i32, tile_y: i32) {
        self.offset.0 = -(tile_x as f32 * Self::TILE_SIZE * self.zoom);
        self.offset.1 = -(tile_y as f32 * Self::TILE_SIZE * self.zoom);
    }
}
