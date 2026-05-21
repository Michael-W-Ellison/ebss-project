// src/gui/state.rs
//! State management and communication types for the GUI.

use std::collections::HashMap;
use uuid::Uuid;
use crate::agents::LifeStage;
use crate::core::DriveType;
use crate::world::{Position, BuildingType, ResourceType, TerrainType};

/// Commands sent from GUI to simulation thread
#[derive(Debug, Clone)]
pub enum SimulationCommand {
    Play,
    Pause,
    Step,
    SetSpeed(f32),
    SelectEntity(EntitySelection),
    DeselectAll,
}

/// Entity selection types
#[derive(Debug, Clone, PartialEq)]
pub enum EntitySelection {
    None,
    Agent(Uuid),
    Building(Position),
    Resource(Position),
    Terrain(Position),
}

impl Default for EntitySelection {
    fn default() -> Self {
        EntitySelection::None
    }
}

/// Current simulation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    Running,
    Paused,
    Stepping,
}

impl Default for SimState {
    fn default() -> Self {
        SimState::Paused
    }
}

/// Lightweight snapshot of world state for GUI rendering
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileSnapshot>,
    pub resources: Vec<ResourceSnapshot>,
    pub buildings: Vec<BuildingSnapshot>,
    pub tick: u32,
}

/// Single tile data
#[derive(Debug, Clone)]
pub struct TileSnapshot {
    pub x: i32,
    pub y: i32,
    pub terrain: TerrainType,
    pub walkable: bool,
}

/// Resource node data
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub position: Position,
    pub resource_type: ResourceType,
    pub amount: u32,
    pub max_amount: u32,
}

/// Building data
#[derive(Debug, Clone)]
pub struct BuildingSnapshot {
    pub position: Position,
    pub building_type: BuildingType,
    pub completed: bool,
    pub progress: f32,
}

/// Lightweight agent data for map rendering
#[derive(Debug, Clone)]
pub struct AgentSnapshot {
    pub id: Uuid,
    pub position: (i32, i32, i32),
    pub health: f32,
    pub energy: f32,
    pub life_stage: LifeStage,
    pub is_alive: bool,
    pub most_urgent_drive: Option<DriveType>,
}

/// Population statistics snapshot
#[derive(Debug, Clone, Default)]
pub struct PopulationStatsSnapshot {
    pub total_agents: usize,
    pub infants: usize,
    pub children: usize,
    pub adolescents: usize,
    pub adults: usize,
    pub elderly: usize,
    pub total_births: u64,
    pub total_deaths: u64,
    pub average_health: f32,
    pub average_energy: f32,
    pub average_happiness: f32,
}

/// Population snapshot
#[derive(Debug, Clone)]
pub struct PopulationSnapshot {
    pub agents: Vec<AgentSnapshot>,
    pub stats: PopulationStatsSnapshot,
}

/// Complete simulation snapshot sent to GUI each frame
#[derive(Debug, Clone)]
pub struct SimulationSnapshot {
    pub tick: u32,
    pub state: SimState,
    pub speed: f32,
    pub world: WorldSnapshot,
    pub population: PopulationSnapshot,
    pub selected: EntitySelection,
}

/// Selected agent detailed data (only populated when an agent is selected)
#[derive(Debug, Clone)]
pub struct SelectedAgentData {
    pub id: Uuid,
    pub name: String,
    pub position: (i32, i32, i32),
    pub health: f32,
    pub energy: f32,
    pub age: u32,
    pub max_age: u32,
    pub life_stage: LifeStage,
    pub drives: Vec<DriveData>,
    pub traits: Vec<String>,
    pub skills: HashMap<String, i32>,
    pub inventory_count: usize,
    pub relationship_count: usize,
}

/// Drive data for display
#[derive(Debug, Clone)]
pub struct DriveData {
    pub drive_type: DriveType,
    pub value: f32,
    pub weight: f32,
    pub urgency: f32,
}

/// GUI application state
pub struct GuiState {
    pub simulation_state: SimState,
    pub speed: f32,
    pub selected: EntitySelection,
    pub selected_agent_data: Option<SelectedAgentData>,
    pub latest_snapshot: Option<SimulationSnapshot>,

    // Map view state
    pub map_zoom: f32,
    pub map_offset: (f32, f32),

    // UI state
    pub show_inspector: bool,
    pub show_statistics: bool,
    pub show_legend: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            simulation_state: SimState::Paused,
            speed: 1.0,
            selected: EntitySelection::None,
            selected_agent_data: None,
            latest_snapshot: None,
            map_zoom: 1.0,
            map_offset: (0.0, 0.0),
            show_inspector: true,
            show_statistics: true,
            show_legend: false,
        }
    }
}

impl GuiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_from_snapshot(&mut self, snapshot: SimulationSnapshot) {
        self.simulation_state = snapshot.state;
        self.speed = snapshot.speed;
        self.latest_snapshot = Some(snapshot);
    }
}
