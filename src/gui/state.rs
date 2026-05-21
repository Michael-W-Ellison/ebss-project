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
    pub skills: HashMap<String, SkillData>,
    pub inventory: Vec<InventoryItemData>,
    pub relationships: Vec<RelationshipData>,
    pub emotions: EmotionData,
    pub goals: Vec<GoalData>,
    pub current_activity: Option<String>,
    pub survival_status: SurvivalStatus,
    pub parent_ids: Vec<Uuid>,
}

/// Drive data for display
#[derive(Debug, Clone)]
pub struct DriveData {
    pub drive_type: DriveType,
    pub value: f32,
    pub weight: f32,
    pub urgency: f32,
}

/// Skill data for display
#[derive(Debug, Clone)]
pub struct SkillData {
    pub name: String,
    pub level: i32,
    pub experience: u32,
    pub category: String,
}

/// Inventory item data for display
#[derive(Debug, Clone)]
pub struct InventoryItemData {
    pub item_id: String,
    pub quantity: u32,
    pub quality: Option<String>,
    pub durability: Option<(f32, f32)>, // (current, max)
    pub fill_level: Option<(f32, f32)>, // (current, max) for containers
}

/// Relationship data for display
#[derive(Debug, Clone)]
pub struct RelationshipData {
    pub other_agent_id: Uuid,
    pub relationship_type: String,
    pub bond_strength: f32,
    pub total_interactions: u32,
}

/// Emotion data for display
#[derive(Debug, Clone)]
pub struct EmotionData {
    pub happiness: f32,
    pub anger: f32,
    pub fear: f32,
    pub sadness: f32,
    pub curiosity: f32,
}

/// Goal data for display
#[derive(Debug, Clone)]
pub struct GoalData {
    pub description: String,
    pub priority: f32,
    pub progress: f32,
    pub completed: bool,
}

/// Survival status for display
#[derive(Debug, Clone)]
pub struct SurvivalStatus {
    pub is_starving: bool,
    pub is_dehydrated: bool,
    pub ticks_without_food: u32,
    pub ticks_without_water: u32,
    pub is_critical: bool,
}

/// Building detailed data
#[derive(Debug, Clone)]
pub struct SelectedBuildingData {
    pub building_type: BuildingType,
    pub position: Position,
    pub completed: bool,
    pub progress: f32,
    pub owner_id: Option<Uuid>,
    pub occupant_ids: Vec<Uuid>,
    pub resources_needed: Vec<(String, u32, u32)>, // (resource_type, delivered, required)
    pub worker_ids: Vec<Uuid>,
    pub description: String,
    pub benefits: Vec<String>,
}

/// Resource detailed data
#[derive(Debug, Clone)]
pub struct SelectedResourceData {
    pub resource_type: ResourceType,
    pub position: Position,
    pub amount: u32,
    pub max_amount: u32,
    pub percentage: f32,
    pub is_depleted: bool,
    pub description: String,
    pub uses: Vec<String>,
}

/// Map layer visibility settings
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

/// GUI application state
pub struct GuiState {
    pub simulation_state: SimState,
    pub speed: f32,
    pub selected: EntitySelection,
    pub selected_agent_data: Option<SelectedAgentData>,
    pub selected_building_data: Option<SelectedBuildingData>,
    pub selected_resource_data: Option<SelectedResourceData>,
    pub latest_snapshot: Option<SimulationSnapshot>,

    // Map view state
    pub map_zoom: f32,
    pub map_offset: (f32, f32),
    pub map_layers: MapLayers,
    pub show_minimap: bool,
    pub follow_selected: bool,

    // UI state
    pub show_inspector: bool,
    pub show_statistics: bool,
    pub show_legend: bool,

    // Inspector state
    pub inspector_tab: InspectorTab,
}

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

impl Default for GuiState {
    fn default() -> Self {
        Self {
            simulation_state: SimState::Paused,
            speed: 1.0,
            selected: EntitySelection::None,
            selected_agent_data: None,
            selected_building_data: None,
            selected_resource_data: None,
            latest_snapshot: None,
            map_zoom: 1.0,
            map_offset: (0.0, 0.0),
            map_layers: MapLayers::default(),
            show_minimap: true,
            follow_selected: false,
            show_inspector: true,
            show_statistics: true,
            show_legend: false,
            inspector_tab: InspectorTab::default(),
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

    /// Center the map on a specific world position
    pub fn center_on_position(&mut self, x: i32, y: i32, tile_size: f32, view_size: (f32, f32)) {
        let world_x = x as f32 * tile_size * self.map_zoom;
        let world_y = y as f32 * tile_size * self.map_zoom;
        self.map_offset = (
            world_x - view_size.0 / 2.0,
            world_y - view_size.1 / 2.0,
        );
    }

    /// Center on the currently selected entity
    pub fn center_on_selected(&mut self, tile_size: f32, view_size: (f32, f32)) {
        if let Some(snapshot) = &self.latest_snapshot {
            match &self.selected {
                EntitySelection::Agent(id) => {
                    if let Some(agent) = snapshot.population.agents.iter().find(|a| a.id == *id) {
                        self.center_on_position(agent.position.0, agent.position.1, tile_size, view_size);
                    }
                }
                EntitySelection::Building(pos) => {
                    self.center_on_position(pos.x, pos.y, tile_size, view_size);
                }
                EntitySelection::Resource(pos) => {
                    self.center_on_position(pos.x, pos.y, tile_size, view_size);
                }
                EntitySelection::Terrain(pos) => {
                    self.center_on_position(pos.x, pos.y, tile_size, view_size);
                }
                EntitySelection::None => {}
            }
        }
    }
}
