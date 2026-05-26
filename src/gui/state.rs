// src/gui/state.rs
//! State management and communication types for the GUI.

use std::collections::HashMap;
use uuid::Uuid;
use crate::agents::{LifeStage, Gender, JobCategory};
use crate::core::DriveType;
use crate::world::{Position, BuildingType, ResourceType, TerrainType};
use super::events::TimelineState;

/// Commands sent from GUI to simulation thread
#[derive(Debug, Clone)]
pub enum SimulationCommand {
    Play,
    Pause,
    Step,
    SetSpeed(f32),
    SelectEntity(EntitySelection),
    DeselectAll,
    SaveGame(String),
    LoadGame(String),
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
    pub is_sleeping: bool,
    pub fatigue_severity: u8,
    pub relationship_count: usize,
    pub inventory_count: u32,
    pub current_activity: Option<String>,
    pub gender: Gender,
    pub inferred_job: Option<JobCategory>,
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
    /// Events that occurred this tick (for timeline panel)
    pub events: Vec<super::events::SimulationEvent>,
}

/// Selected agent detailed data (only populated when an agent is selected)
#[derive(Debug, Clone)]
pub struct SelectedAgentData {
    pub id: Uuid,
    pub name: String,
    pub gender: Gender,
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

/// History data point for graphs
#[derive(Debug, Clone, Default)]
pub struct HistoryPoint {
    pub tick: u32,
    pub population: usize,
    pub infants: usize,
    pub children: usize,
    pub adolescents: usize,
    pub adults: usize,
    pub elderly: usize,
    pub births: u64,
    pub deaths: u64,
    pub avg_health: f32,
    pub avg_energy: f32,
    pub avg_happiness: f32,
    pub total_resources: u32,
    pub buildings_completed: usize,
    pub buildings_construction: usize,
}

/// Statistics history for graphs
#[derive(Debug, Clone)]
pub struct StatisticsHistory {
    pub points: Vec<HistoryPoint>,
    pub max_points: usize,
    pub sample_interval: u32,
    pub last_sample_tick: u32,
}

impl Default for StatisticsHistory {
    fn default() -> Self {
        Self {
            points: Vec::with_capacity(500),
            max_points: 500,
            sample_interval: 10, // Sample every 10 ticks
            last_sample_tick: 0,
        }
    }
}

impl StatisticsHistory {
    pub fn should_sample(&self, current_tick: u32) -> bool {
        current_tick >= self.last_sample_tick + self.sample_interval
    }

    pub fn add_point(&mut self, point: HistoryPoint) {
        if self.points.len() >= self.max_points {
            self.points.remove(0);
        }
        self.last_sample_tick = point.tick;
        self.points.push(point);
    }

    pub fn population_data(&self) -> Vec<[f64; 2]> {
        self.points.iter()
            .map(|p| [p.tick as f64, p.population as f64])
            .collect()
    }

    pub fn life_stage_data(&self, stage: &str) -> Vec<[f64; 2]> {
        self.points.iter()
            .map(|p| {
                let value = match stage {
                    "infants" => p.infants,
                    "children" => p.children,
                    "adolescents" => p.adolescents,
                    "adults" => p.adults,
                    "elderly" => p.elderly,
                    _ => 0,
                };
                [p.tick as f64, value as f64]
            })
            .collect()
    }

    pub fn health_data(&self) -> Vec<[f64; 2]> {
        self.points.iter()
            .map(|p| [p.tick as f64, p.avg_health as f64])
            .collect()
    }

    pub fn energy_data(&self) -> Vec<[f64; 2]> {
        self.points.iter()
            .map(|p| [p.tick as f64, p.avg_energy as f64])
            .collect()
    }

    pub fn happiness_data(&self) -> Vec<[f64; 2]> {
        self.points.iter()
            .map(|p| [p.tick as f64, p.avg_happiness as f64 * 100.0])
            .collect()
    }

    pub fn births_deaths_data(&self) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let births: Vec<[f64; 2]> = self.points.iter()
            .map(|p| [p.tick as f64, p.births as f64])
            .collect();
        let deaths: Vec<[f64; 2]> = self.points.iter()
            .map(|p| [p.tick as f64, p.deaths as f64])
            .collect();
        (births, deaths)
    }

    pub fn resources_data(&self) -> Vec<[f64; 2]> {
        self.points.iter()
            .map(|p| [p.tick as f64, p.total_resources as f64])
            .collect()
    }

    pub fn buildings_data(&self) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let completed: Vec<[f64; 2]> = self.points.iter()
            .map(|p| [p.tick as f64, p.buildings_completed as f64])
            .collect();
        let construction: Vec<[f64; 2]> = self.points.iter()
            .map(|p| [p.tick as f64, p.buildings_construction as f64])
            .collect();
        (completed, construction)
    }
}

/// Statistics tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatisticsTab {
    #[default]
    Overview,
    Population,
    Vitals,
    Resources,
    Buildings,
}

/// Technology status for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechStatus {
    Unknown,
    Discoverable,
    InProgress,
    Discovered,
}

/// Technology node data for visualization
#[derive(Debug, Clone)]
pub struct TechNodeData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub era: String,
    pub era_index: usize,
    pub status: TechStatus,
    pub discovery_progress: u8,
    pub agents_with_knowledge: usize,
    pub prerequisites: Vec<String>,
    pub unlocks: Vec<String>,
    pub first_discoverer: Option<uuid::Uuid>,
    pub discovery_tick: Option<u32>,
}

/// Technology tree snapshot for GUI
#[derive(Debug, Clone, Default)]
pub struct TechTreeSnapshot {
    pub nodes: Vec<TechNodeData>,
    pub current_era: String,
    pub total_discovered: usize,
    pub total_technologies: usize,
    pub discovery_history: Vec<(u32, String)>, // (tick, tech_id)
}

/// Relationship graph node data for visualization
#[derive(Debug, Clone)]
pub struct RelationshipGraphNode {
    pub agent_id: Uuid,
    pub position: (i32, i32),
    pub life_stage: LifeStage,
    pub health: f32,
    pub is_alive: bool,
    pub relationships: Vec<RelationshipEdge>,
}

/// Relationship edge data for graph
#[derive(Debug, Clone)]
pub struct RelationshipEdge {
    pub target_id: Uuid,
    pub relationship_type: String,
    pub bond_strength: f32,
    pub total_interactions: u32,
}

/// Relationship graph snapshot for GUI
#[derive(Debug, Clone, Default)]
pub struct RelationshipGraphSnapshot {
    pub nodes: Vec<RelationshipGraphNode>,
    pub tick: u32,
}

/// Filter options for relationship graph
#[derive(Debug, Clone)]
pub struct RelationshipFilter {
    pub show_parent: bool,
    pub show_child: bool,
    pub show_sibling: bool,
    pub show_partner: bool,
    pub show_friend: bool,
    pub show_acquaintance: bool,
    pub show_rival: bool,
    pub show_enemy: bool,
    pub min_bond_strength: f32,
}

impl Default for RelationshipFilter {
    fn default() -> Self {
        Self {
            show_parent: true,
            show_child: true,
            show_sibling: true,
            show_partner: true,
            show_friend: true,
            show_acquaintance: false, // Hidden by default (too many)
            show_rival: true,
            show_enemy: true,
            min_bond_strength: -1.0,
        }
    }
}

/// Layout mode for relationship graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphLayoutMode {
    #[default]
    ForceDirected,
    Circular,
    Spatial,
}

/// Computed node position for graph layout
#[derive(Debug, Clone, Default)]
pub struct GraphNodePosition {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

/// Relationship graph panel state
#[derive(Debug, Clone)]
pub struct RelationshipGraphState {
    pub zoom: f32,
    pub offset: (f32, f32),
    pub selected_agent: Option<Uuid>,
    pub hovered_agent: Option<Uuid>,
    pub filter: RelationshipFilter,
    pub layout_mode: GraphLayoutMode,
    pub show_labels: bool,
    pub focus_agent: Option<Uuid>,
    pub node_positions: HashMap<Uuid, GraphNodePosition>,
    pub layout_iterations: u32,
    pub needs_layout: bool,
}

impl Default for RelationshipGraphState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: (0.0, 0.0),
            selected_agent: None,
            hovered_agent: None,
            filter: RelationshipFilter::default(),
            layout_mode: GraphLayoutMode::default(),
            show_labels: true,
            focus_agent: None,
            node_positions: HashMap::new(),
            layout_iterations: 0,
            needs_layout: true,
        }
    }
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

/// Filter options for agent display on the map
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
    pub agent_filter: AgentMapFilter,
    pub show_minimap: bool,
    pub follow_selected: bool,
    pub minimap_settings: MinimapSettings,

    // UI state
    pub show_inspector: bool,
    pub show_statistics: bool,
    pub show_legend: bool,
    pub show_keyboard_help: bool,
    pub show_search: bool,
    pub show_save_dialog: bool,
    pub show_load_dialog: bool,

    // Inspector state
    pub inspector_tab: InspectorTab,

    // Statistics state
    pub statistics_tab: StatisticsTab,
    pub statistics_history: StatisticsHistory,

    // Tech tree state
    pub show_tech_tree: bool,
    pub tech_tree_snapshot: Option<TechTreeSnapshot>,
    pub selected_tech: Option<String>,

    // Relationship graph state
    pub show_relationship_graph: bool,
    pub relationship_graph_snapshot: Option<RelationshipGraphSnapshot>,
    pub relationship_graph_state: RelationshipGraphState,

    // Search state
    pub search_state: SearchState,

    // Save/Load state
    pub save_load_state: SaveLoadState,

    // Timeline state
    pub show_timeline: bool,
    pub timeline_state: TimelineState,

    // Notifications
    pub notifications: Vec<Notification>,
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
    pub size: f32,
    pub show_resources: bool,
    pub show_buildings: bool,
    pub show_agents: bool,
    pub opacity: f32,
    pub position: MinimapPosition,
}

impl Default for MinimapSettings {
    fn default() -> Self {
        Self {
            size: 150.0,
            show_resources: true,
            show_buildings: true,
            show_agents: true,
            opacity: 0.85,
            position: MinimapPosition::default(),
        }
    }
}

/// Search filter and results
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub search_type: SearchType,
    pub results: Vec<SearchResult>,
    pub selected_result: Option<usize>,
    pub life_stage_filter: Option<LifeStage>,
    pub health_filter: HealthFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchType {
    #[default]
    All,
    Agents,
    Buildings,
    Resources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HealthFilter {
    #[default]
    Any,
    Critical,
    Low,
    Healthy,
}

#[derive(Debug, Clone)]
pub enum SearchResult {
    Agent {
        id: Uuid,
        position: (i32, i32),
        life_stage: LifeStage,
        health: f32,
        energy: f32,
    },
    Building {
        position: Position,
        building_type: BuildingType,
        completed: bool,
    },
    Resource {
        position: Position,
        resource_type: ResourceType,
        amount: u32,
        max_amount: u32,
    },
}

/// Save/Load dialog state
#[derive(Debug, Clone, Default)]
pub struct SaveLoadState {
    pub filename: String,
    pub save_directory: String,
    pub available_saves: Vec<SaveFileInfo>,
    pub selected_save: Option<usize>,
    pub last_error: Option<String>,
    pub last_success: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SaveFileInfo {
    pub filename: String,
    pub path: String,
    pub tick: u32,
    pub agent_count: usize,
    pub modified: String,
}

/// Notification/toast message
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub created_at: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
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
            agent_filter: AgentMapFilter::default(),
            show_minimap: true,
            follow_selected: false,
            minimap_settings: MinimapSettings::default(),
            show_inspector: true,
            show_statistics: true,
            show_legend: false,
            show_keyboard_help: false,
            show_search: false,
            show_save_dialog: false,
            show_load_dialog: false,
            inspector_tab: InspectorTab::default(),
            statistics_tab: StatisticsTab::default(),
            statistics_history: StatisticsHistory::default(),
            show_tech_tree: false,
            tech_tree_snapshot: None,
            selected_tech: None,
            show_relationship_graph: false,
            relationship_graph_snapshot: None,
            relationship_graph_state: RelationshipGraphState::default(),
            search_state: SearchState::default(),
            save_load_state: SaveLoadState::default(),
            show_timeline: false,
            timeline_state: TimelineState::default(),
            notifications: Vec::new(),
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

        // Process events from snapshot and add to timeline
        for event in &snapshot.events {
            self.timeline_state.add_event(event.clone());
        }

        // Record history point if interval elapsed
        if self.statistics_history.should_sample(snapshot.tick) {
            let stats = &snapshot.population.stats;
            let world = &snapshot.world;

            let total_resources: u32 = world.resources.iter()
                .map(|r| r.amount)
                .sum();

            let buildings_completed = world.buildings.iter()
                .filter(|b| b.completed)
                .count();

            let point = HistoryPoint {
                tick: snapshot.tick,
                population: stats.total_agents,
                infants: stats.infants,
                children: stats.children,
                adolescents: stats.adolescents,
                adults: stats.adults,
                elderly: stats.elderly,
                births: stats.total_births,
                deaths: stats.total_deaths,
                avg_health: stats.average_health,
                avg_energy: stats.average_energy,
                avg_happiness: stats.average_happiness,
                total_resources,
                buildings_completed,
                buildings_construction: world.buildings.len() - buildings_completed,
            };

            self.statistics_history.add_point(point);
        }

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

    /// Add a notification message
    pub fn notify(&mut self, message: impl Into<String>, notification_type: NotificationType, current_time: f64) {
        self.notifications.push(Notification {
            message: message.into(),
            notification_type,
            created_at: current_time,
            duration: 3.0,
        });
    }

    /// Update notifications, removing expired ones
    pub fn update_notifications(&mut self, current_time: f64) {
        self.notifications.retain(|n| current_time - n.created_at < n.duration);
    }

    /// Perform search based on current search state
    pub fn perform_search(&mut self) {
        self.search_state.results.clear();

        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };

        let query_lower = self.search_state.query.to_lowercase();

        // Search agents
        if matches!(self.search_state.search_type, SearchType::All | SearchType::Agents) {
            for agent in &snapshot.population.agents {
                if !agent.is_alive {
                    continue;
                }

                // Filter by life stage
                if let Some(stage) = self.search_state.life_stage_filter {
                    if agent.life_stage != stage {
                        continue;
                    }
                }

                // Filter by health
                match self.search_state.health_filter {
                    HealthFilter::Critical if agent.health >= 25.0 => continue,
                    HealthFilter::Low if agent.health >= 50.0 || agent.health < 25.0 => continue,
                    HealthFilter::Healthy if agent.health < 50.0 => continue,
                    _ => {}
                }

                // Match query against ID or life stage name
                let id_str = format!("{:?}", agent.id).to_lowercase();
                let stage_str = format!("{:?}", agent.life_stage).to_lowercase();

                if query_lower.is_empty() || id_str.contains(&query_lower) || stage_str.contains(&query_lower) {
                    self.search_state.results.push(SearchResult::Agent {
                        id: agent.id,
                        position: (agent.position.0, agent.position.1),
                        life_stage: agent.life_stage,
                        health: agent.health,
                        energy: agent.energy,
                    });
                }
            }
        }

        // Search buildings
        if matches!(self.search_state.search_type, SearchType::All | SearchType::Buildings) {
            for building in &snapshot.world.buildings {
                let type_str = format!("{:?}", building.building_type).to_lowercase();

                if query_lower.is_empty() || type_str.contains(&query_lower) {
                    self.search_state.results.push(SearchResult::Building {
                        position: building.position,
                        building_type: building.building_type,
                        completed: building.completed,
                    });
                }
            }
        }

        // Search resources
        if matches!(self.search_state.search_type, SearchType::All | SearchType::Resources) {
            for resource in &snapshot.world.resources {
                let type_str = format!("{:?}", resource.resource_type).to_lowercase();

                if query_lower.is_empty() || type_str.contains(&query_lower) {
                    self.search_state.results.push(SearchResult::Resource {
                        position: resource.position,
                        resource_type: resource.resource_type,
                        amount: resource.amount,
                        max_amount: resource.max_amount,
                    });
                }
            }
        }

        // Sort results by relevance (agents first, then by position)
        self.search_state.results.sort_by(|a, b| {
            let type_order = |r: &SearchResult| match r {
                SearchResult::Agent { .. } => 0,
                SearchResult::Building { .. } => 1,
                SearchResult::Resource { .. } => 2,
            };
            type_order(a).cmp(&type_order(b))
        });

        self.search_state.selected_result = if self.search_state.results.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Select and center on a search result
    pub fn select_search_result(&mut self, index: usize, tile_size: f32, view_size: (f32, f32)) {
        if let Some(result) = self.search_state.results.get(index) {
            match result {
                SearchResult::Agent { id, position, .. } => {
                    self.selected = EntitySelection::Agent(*id);
                    self.center_on_position(position.0, position.1, tile_size, view_size);
                }
                SearchResult::Building { position, .. } => {
                    self.selected = EntitySelection::Building(*position);
                    self.center_on_position(position.x, position.y, tile_size, view_size);
                }
                SearchResult::Resource { position, .. } => {
                    self.selected = EntitySelection::Resource(*position);
                    self.center_on_position(position.x, position.y, tile_size, view_size);
                }
            }
            self.search_state.selected_result = Some(index);
        }
    }

    /// Cycle to next entity of same type as currently selected
    pub fn select_next_entity(&mut self) {
        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };

        match &self.selected {
            EntitySelection::Agent(current_id) => {
                let agents: Vec<_> = snapshot.population.agents.iter()
                    .filter(|a| a.is_alive)
                    .collect();
                if let Some(idx) = agents.iter().position(|a| a.id == *current_id) {
                    let next_idx = (idx + 1) % agents.len();
                    self.selected = EntitySelection::Agent(agents[next_idx].id);
                }
            }
            EntitySelection::Building(current_pos) => {
                let buildings = &snapshot.world.buildings;
                if let Some(idx) = buildings.iter().position(|b| b.position == *current_pos) {
                    let next_idx = (idx + 1) % buildings.len();
                    self.selected = EntitySelection::Building(buildings[next_idx].position);
                }
            }
            EntitySelection::Resource(current_pos) => {
                let resources = &snapshot.world.resources;
                if let Some(idx) = resources.iter().position(|r| r.position == *current_pos) {
                    let next_idx = (idx + 1) % resources.len();
                    self.selected = EntitySelection::Resource(resources[next_idx].position);
                }
            }
            EntitySelection::None | EntitySelection::Terrain(_) => {
                // Select first alive agent
                if let Some(agent) = snapshot.population.agents.iter().find(|a| a.is_alive) {
                    self.selected = EntitySelection::Agent(agent.id);
                }
            }
        }
    }

    /// Cycle to previous entity of same type
    pub fn select_previous_entity(&mut self) {
        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };

        match &self.selected {
            EntitySelection::Agent(current_id) => {
                let agents: Vec<_> = snapshot.population.agents.iter()
                    .filter(|a| a.is_alive)
                    .collect();
                if let Some(idx) = agents.iter().position(|a| a.id == *current_id) {
                    let prev_idx = if idx == 0 { agents.len() - 1 } else { idx - 1 };
                    self.selected = EntitySelection::Agent(agents[prev_idx].id);
                }
            }
            EntitySelection::Building(current_pos) => {
                let buildings = &snapshot.world.buildings;
                if let Some(idx) = buildings.iter().position(|b| b.position == *current_pos) {
                    let prev_idx = if idx == 0 { buildings.len() - 1 } else { idx - 1 };
                    self.selected = EntitySelection::Building(buildings[prev_idx].position);
                }
            }
            EntitySelection::Resource(current_pos) => {
                let resources = &snapshot.world.resources;
                if let Some(idx) = resources.iter().position(|r| r.position == *current_pos) {
                    let prev_idx = if idx == 0 { resources.len() - 1 } else { idx - 1 };
                    self.selected = EntitySelection::Resource(resources[prev_idx].position);
                }
            }
            EntitySelection::None | EntitySelection::Terrain(_) => {
                // Select last alive agent
                if let Some(agent) = snapshot.population.agents.iter().filter(|a| a.is_alive).last() {
                    self.selected = EntitySelection::Agent(agent.id);
                }
            }
        }
    }
}
