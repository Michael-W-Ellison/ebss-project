// src/analytics/inspector.rs
//! Inspector system for examining agents, terrain, and simulation state.

use crate::agents::Agent;
use crate::core::{DriveType, Drive};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Detailed information about an agent for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInspectorData {
    pub id: Uuid,
    pub position: (i32, i32, i32),
    pub health: f32,

    // Drive information
    pub drives: Vec<DriveInspectorData>,
    pub most_urgent_drive: Option<DriveType>,

    // Behavior information
    pub behavior_tree_count: usize,

    // Memory information
    pub memory_summary: MemorySummary,

    // Stats
    pub age: u64, // Ticks alive
}

/// Drive information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInspectorData {
    pub drive_type: DriveType,
    pub name: String,
    pub value: f32,
    pub threshold: f32,
    pub weight: f32,
    pub urgency: f32,
    pub is_active: bool,
    pub satisfaction: String,
}

impl DriveInspectorData {
    pub fn from_drive(drive: &Drive) -> Self {
        Self {
            drive_type: drive.drive_type,
            name: format!("{:?}", drive.drive_type),
            value: drive.value,
            threshold: drive.threshold,
            weight: drive.weight,
            urgency: drive.urgency(),
            is_active: drive.is_active(),
            satisfaction: drive.drive_type.satisfaction_description().to_string(),
        }
    }
}

/// Memory summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySummary {
    pub known_locations: usize,
    pub known_storage: usize,
    pub known_agents: usize,
    pub known_recipes: usize,
}

impl AgentInspectorData {
    pub fn from_agent(agent: &Agent) -> Self {
        let drives: Vec<DriveInspectorData> = agent.drives.drives
            .iter()
            .map(DriveInspectorData::from_drive)
            .collect();

        let most_urgent_drive = agent.drives.most_urgent()
            .map(|d| d.drive_type);

        Self {
            id: agent.id,
            position: agent.state.position,
            health: agent.state.health,
            drives,
            most_urgent_drive,
            behavior_tree_count: agent.behavior_trees.len(),
            memory_summary: MemorySummary {
                known_locations: 0,
                known_storage: 0,
                known_agents: 0,
                known_recipes: 0,
            },
            age: 0, // Will be tracked by simulation
        }
    }

    /// Get drives sorted by urgency (highest first)
    pub fn drives_by_urgency(&self) -> Vec<&DriveInspectorData> {
        let mut drives: Vec<&DriveInspectorData> = self.drives.iter().collect();
        drives.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap());
        drives
    }

    /// Get only active drives
    pub fn active_drives(&self) -> Vec<&DriveInspectorData> {
        self.drives.iter().filter(|d| d.is_active).collect()
    }
}

/// Terrain information at a specific position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainInspectorData {
    pub position: (i32, i32, i32),
    pub material_id: Option<String>,
    pub material_name: Option<String>,
    pub material_category: Option<String>,
    pub hardness: Option<f32>,
    pub is_walkable: bool,
    pub height: i32,
}

/// Selection state for UI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    None,
    Agent(Uuid),
    Terrain((i32, i32, i32)),
}

/// Inspector manages selected entities and provides data
pub struct Inspector {
    selection: Selection,
    agent_data_cache: HashMap<Uuid, AgentInspectorData>,
}

impl Inspector {
    pub fn new() -> Self {
        Self {
            selection: Selection::None,
            agent_data_cache: HashMap::new(),
        }
    }

    /// Select an agent
    pub fn select_agent(&mut self, id: Uuid) {
        self.selection = Selection::Agent(id);
    }

    /// Select terrain at position
    pub fn select_terrain(&mut self, position: (i32, i32, i32)) {
        self.selection = Selection::Terrain(position);
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection = Selection::None;
    }

    /// Get current selection
    pub fn get_selection(&self) -> &Selection {
        &self.selection
    }

    /// Check if an agent is selected
    pub fn is_agent_selected(&self, id: Uuid) -> bool {
        matches!(self.selection, Selection::Agent(selected_id) if selected_id == id)
    }

    /// Cache agent data for quick access
    pub fn cache_agent_data(&mut self, id: Uuid, data: AgentInspectorData) {
        self.agent_data_cache.insert(id, data);
    }

    /// Get cached agent data
    pub fn get_cached_agent_data(&self, id: Uuid) -> Option<&AgentInspectorData> {
        self.agent_data_cache.get(&id)
    }

    /// Update cached data for all agents
    pub fn update_cache(&mut self, agents: &[Agent]) {
        self.agent_data_cache.clear();
        for agent in agents {
            let data = AgentInspectorData::from_agent(agent);
            self.agent_data_cache.insert(agent.id, data);
        }
    }
}

impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentConfig;

    #[test]
    fn test_inspector_creation() {
        let inspector = Inspector::new();
        assert_eq!(inspector.get_selection(), &Selection::None);
    }

    #[test]
    fn test_agent_selection() {
        let mut inspector = Inspector::new();
        let agent_id = Uuid::new_v4();

        inspector.select_agent(agent_id);
        assert_eq!(inspector.get_selection(), &Selection::Agent(agent_id));
        assert!(inspector.is_agent_selected(agent_id));
    }

    #[test]
    fn test_terrain_selection() {
        let mut inspector = Inspector::new();
        let pos = (10, 20, 30);

        inspector.select_terrain(pos);
        assert_eq!(inspector.get_selection(), &Selection::Terrain(pos));
    }

    #[test]
    fn test_clear_selection() {
        let mut inspector = Inspector::new();
        inspector.select_agent(Uuid::new_v4());

        inspector.clear_selection();
        assert_eq!(inspector.get_selection(), &Selection::None);
    }

    #[test]
    fn test_agent_inspector_data() {
        let agent = Agent::new(AgentConfig::default());
        let data = AgentInspectorData::from_agent(&agent);

        assert_eq!(data.id, agent.id);
        assert_eq!(data.drives.len(), 13); // 13 core drives
        assert!(data.health > 0.0);
    }

    #[test]
    fn test_drives_by_urgency() {
        let agent = Agent::new(AgentConfig::default());
        let data = AgentInspectorData::from_agent(&agent);

        let sorted = data.drives_by_urgency();
        assert_eq!(sorted.len(), 13);

        // Verify sorted by urgency (descending)
        for i in 1..sorted.len() {
            assert!(sorted[i-1].urgency >= sorted[i].urgency);
        }
    }

    #[test]
    fn test_cache_agent_data() {
        let mut inspector = Inspector::new();
        let agent = Agent::new(AgentConfig::default());
        let data = AgentInspectorData::from_agent(&agent);
        let agent_id = agent.id;

        inspector.cache_agent_data(agent_id, data);
        assert!(inspector.get_cached_agent_data(agent_id).is_some());
    }
}
