// src/analytics/inspector.rs
//! Inspector system for examining agents, terrain, and simulation state.

use crate::agents::{Agent, BodySummary, SkillCategory, EmotionType, RelationshipType};
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

    // Inventory information
    pub inventory_summary: InventorySummary,

    // Sensory information
    pub sensory_summary: SensorySummary,

    // Body information
    pub body_summary: BodySummary,

    // Skills information
    pub skills_summary: SkillsSummary,

    // Emotion information
    pub emotion_summary: EmotionSummary,

    // Relationship information
    pub relationship_summary: RelationshipSummary,

    // Stats
    pub age: u64, // Ticks alive
}

/// Skills summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsSummary {
    pub total_skills: usize,
    pub highest_skill_level: i32,
    pub highest_skill_name: String,
    pub average_skill_level: f32,
    pub master_skills: usize,
    pub journeyman_skills: usize,
    pub apprentice_skills: usize,
}

/// Sensory system summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorySummary {
    pub vision_range: f32,
    pub vision_acuity: f32,
    pub vision_impaired: bool,
    pub visible_agents_count: usize,
    pub hearing_range: f32,
    pub hearing_sensitivity: f32,
    pub hearing_impaired: bool,
    pub recent_sounds_count: usize,
    pub can_speak: bool,
    pub speech_impaired: bool,
    pub known_languages: Vec<String>,
    pub overall_sensory_health: f32,
}

/// Inventory summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySummary {
    pub total_items: usize,
    pub total_weight: f32,
    pub water_available: f32,
    pub container_count: usize,
    pub slot_usage: String, // e.g., "5/20"
}

/// Emotion summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSummary {
    pub anger: f32,
    pub fear: f32,
    pub sadness: f32,
    pub dominant_emotion: Option<EmotionType>,
    pub is_distressed: bool,
    pub should_flee: bool,
    pub should_attack: bool,
    pub active_sources: usize, // Total active emotion sources
}

/// Relationship summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub total_relationships: usize,
    pub family_count: usize,
    pub loved_ones_count: usize,
    pub friends_count: usize,
    pub enemies_count: usize,
    pub strongest_bond: Option<(Uuid, RelationshipType, f32)>,
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

        // Calculate inventory summary
        let total_items = agent.inventory.get_all_items().len();
        let water_available = agent.inventory.get_total_water();
        let container_count = agent.inventory.get_all_items()
            .values()
            .filter(|item| item.is_container())
            .count();
        let slot_usage = format!("{}/{}", total_items, agent.inventory.max_slots);

        // Calculate sensory summary
        let sensory_summary = SensorySummary {
            vision_range: agent.senses.vision.effective_range(),
            vision_acuity: agent.senses.vision.acuity,
            vision_impaired: agent.senses.vision.impaired,
            visible_agents_count: agent.senses.vision.visible_agents.len(),
            hearing_range: agent.senses.hearing.effective_range(),
            hearing_sensitivity: agent.senses.hearing.sensitivity,
            hearing_impaired: agent.senses.hearing.impaired,
            recent_sounds_count: agent.senses.hearing.heard_sounds.len(),
            can_speak: agent.senses.speech.can_speak,
            speech_impaired: agent.senses.speech.impaired,
            known_languages: agent.senses.speech.known_languages.iter().cloned().collect(),
            overall_sensory_health: agent.senses.overall_health(),
        };

        // Get body summary
        let body_summary = agent.body.summary();

        // Calculate skills summary
        let all_skills = agent.skills.get_all_skills();
        let total_skills = all_skills.len();
        let highest_skill = agent.skills.highest_skill();
        let highest_skill_level = highest_skill.map(|s| s.level).unwrap_or(-10);
        let highest_skill_name = highest_skill
            .map(|s| s.skill_type.name().to_string())
            .unwrap_or_else(|| "None".to_string());
        let average_skill_level = agent.skills.average_skill_level();

        let master_skills = agent.skills.get_skills_by_category(SkillCategory::High).len();
        let journeyman_skills = agent.skills.get_skills_by_category(SkillCategory::Medium).len();
        let apprentice_skills = agent.skills.get_skills_by_category(SkillCategory::Low).len();

        let skills_summary = SkillsSummary {
            total_skills,
            highest_skill_level,
            highest_skill_name,
            average_skill_level,
            master_skills,
            journeyman_skills,
            apprentice_skills,
        };

        // Calculate emotion summary
        let active_sources = agent.emotions.anger_sources.len()
            + agent.emotions.fear_sources.len()
            + agent.emotions.sadness_sources.len();

        let emotion_summary = EmotionSummary {
            anger: agent.emotions.anger,
            fear: agent.emotions.fear,
            sadness: agent.emotions.sadness,
            dominant_emotion: agent.emotions.dominant_emotion(),
            is_distressed: agent.emotions.is_distressed(),
            should_flee: agent.emotions.should_flee(),
            should_attack: agent.emotions.should_attack(),
            active_sources,
        };

        // Calculate relationship summary
        let all_relationships = agent.relationships.get_all();
        let family_count = agent.relationships.get_family().len();
        let loved_ones_count = agent.relationships.get_loved_ones().len();

        let friends_count = all_relationships.values()
            .filter(|r| r.relationship_type == RelationshipType::Friend)
            .count();

        let enemies_count = all_relationships.values()
            .filter(|r| r.relationship_type == RelationshipType::Enemy || r.relationship_type == RelationshipType::Rival)
            .count();

        let strongest_bond = all_relationships.values()
            .max_by(|a, b| a.bond_strength.partial_cmp(&b.bond_strength).unwrap())
            .map(|r| (r.other_agent, r.relationship_type, r.bond_strength));

        let relationship_summary = RelationshipSummary {
            total_relationships: all_relationships.len(),
            family_count,
            loved_ones_count,
            friends_count,
            enemies_count,
            strongest_bond,
        };

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
            inventory_summary: InventorySummary {
                total_items,
                total_weight: agent.inventory.current_weight,
                water_available,
                container_count,
                slot_usage,
            },
            sensory_summary,
            body_summary,
            skills_summary,
            emotion_summary,
            relationship_summary,
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
        assert_eq!(data.drives.len(), 15); // 15 core drives, Thirst and Protection among them
        assert!(data.health > 0.0);
    }

    #[test]
    fn test_drives_by_urgency() {
        let agent = Agent::new(AgentConfig::default());
        let data = AgentInspectorData::from_agent(&agent);

        let sorted = data.drives_by_urgency();
        assert_eq!(sorted.len(), 15);

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
