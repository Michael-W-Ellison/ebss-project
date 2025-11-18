// src/environment/action.rs
//! Action system for agent-environment interactions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::DriveType;
use super::{Position, ToolType, ToolTier};

/// Types of actions agents can perform in the environment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionType {
    /// Move to a position
    Move,
    /// Harvest/gather a material
    Harvest,
    /// Craft an item
    Craft,
    /// Build a structure
    Build,
    /// Attack an entity
    Attack,
    /// Eat food
    Eat,
    /// Sleep/rest
    Sleep,
    /// Store items
    Store,
    /// Retrieve items
    Retrieve,
    /// Explore an area
    Explore,
    /// Interact with another agent
    Interact,
    /// Custom action (plugin-specific)
    Custom(String),
}

/// Requirements for performing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequirements {
    /// Required items and quantities
    pub required_items: HashMap<String, u32>,
    /// Required tool type
    pub required_tool: Option<ToolType>,
    /// Required tool tier
    pub required_tier: Option<ToolTier>,
    /// Minimum energy level
    pub min_energy: f32,
    /// Required skills or experience
    pub required_skills: HashMap<String, f32>,
    /// Required proximity to position
    pub required_proximity: Option<(Position, f32)>,
}

impl ActionRequirements {
    pub fn none() -> Self {
        Self {
            required_items: HashMap::new(),
            required_tool: None,
            required_tier: None,
            min_energy: 0.0,
            required_skills: HashMap::new(),
            required_proximity: None,
        }
    }

    pub fn with_item(mut self, material_id: String, quantity: u32) -> Self {
        self.required_items.insert(material_id, quantity);
        self
    }

    pub fn with_tool(mut self, tool: ToolType, tier: ToolTier) -> Self {
        self.required_tool = Some(tool);
        self.required_tier = Some(tier);
        self
    }

    pub fn with_min_energy(mut self, energy: f32) -> Self {
        self.min_energy = energy;
        self
    }

    pub fn with_skill(mut self, skill: String, level: f32) -> Self {
        self.required_skills.insert(skill, level);
        self
    }

    pub fn with_proximity(mut self, position: Position, max_distance: f32) -> Self {
        self.required_proximity = Some((position, max_distance));
        self
    }
}

/// Effects of an action on agent drives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEffects {
    /// Drive changes (positive = satisfy, negative = increase)
    pub drive_effects: HashMap<DriveType, f32>,
    /// Energy cost
    pub energy_cost: f32,
    /// Experience gain in various skills
    pub experience_gain: HashMap<String, f32>,
    /// Time required (in ticks)
    pub time_cost: u32,
}

impl ActionEffects {
    pub fn none() -> Self {
        Self {
            drive_effects: HashMap::new(),
            energy_cost: 0.0,
            experience_gain: HashMap::new(),
            time_cost: 0,
        }
    }

    pub fn with_drive_effect(mut self, drive: DriveType, effect: f32) -> Self {
        self.drive_effects.insert(drive, effect);
        self
    }

    pub fn with_energy_cost(mut self, cost: f32) -> Self {
        self.energy_cost = cost;
        self
    }

    pub fn with_experience(mut self, skill: String, exp: f32) -> Self {
        self.experience_gain.insert(skill, exp);
        self
    }

    pub fn with_time_cost(mut self, ticks: u32) -> Self {
        self.time_cost = ticks;
        self
    }
}

/// A complete action definition
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Action type
    pub action_type: ActionType,
    /// Requirements to perform this action
    pub requirements: ActionRequirements,
    /// Effects of performing this action
    pub effects: ActionEffects,
    /// Whether this action can be interrupted
    pub interruptible: bool,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

#[allow(dead_code)]
impl Action {
    pub fn new(id: String, name: String, action_type: ActionType) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            action_type,
            requirements: ActionRequirements::none(),
            effects: ActionEffects::none(),
            interruptible: true,
            properties: HashMap::new(),
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn with_requirements(mut self, req: ActionRequirements) -> Self {
        self.requirements = req;
        self
    }

    pub fn with_effects(mut self, effects: ActionEffects) -> Self {
        self.effects = effects;
        self
    }

    pub fn non_interruptible(mut self) -> Self {
        self.interruptible = false;
        self
    }

    pub fn with_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }
}

/// Context for executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    /// ID of the agent performing the action
    pub agent_id: String,
    /// Position where action is being performed
    pub position: Position,
    /// Target position (for movement, ranged actions)
    pub target_position: Option<Position>,
    /// Target material (for harvesting, crafting)
    pub target_material: Option<String>,
    /// Additional parameters
    pub parameters: HashMap<String, String>,
}

impl ActionContext {
    pub fn new(agent_id: String, position: Position) -> Self {
        Self {
            agent_id,
            position,
            target_position: None,
            target_material: None,
            parameters: HashMap::new(),
        }
    }

    pub fn with_target_position(mut self, pos: Position) -> Self {
        self.target_position = Some(pos);
        self
    }

    pub fn with_target_material(mut self, material: String) -> Self {
        self.target_material = Some(material);
        self
    }

    pub fn with_parameter(mut self, key: String, value: String) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_creation() {
        let action = Action::new(
            "chop_tree".to_string(),
            "Chop Tree".to_string(),
            ActionType::Harvest,
        )
        .with_description("Chop down a tree for wood".to_string())
        .with_requirements(
            ActionRequirements::none()
                .with_tool(ToolType::Axe, ToolTier::Wooden)
                .with_min_energy(10.0)
        )
        .with_effects(
            ActionEffects::none()
                .with_energy_cost(5.0)
                .with_time_cost(100)
                .with_experience("woodcutting".to_string(), 10.0)
        );

        assert_eq!(action.id, "chop_tree");
        assert_eq!(action.requirements.min_energy, 10.0);
        assert_eq!(action.effects.energy_cost, 5.0);
    }

    #[test]
    fn test_action_requirements() {
        let req = ActionRequirements::none()
            .with_item("wood".to_string(), 4)
            .with_tool(ToolType::Axe, ToolTier::Stone)
            .with_skill("crafting".to_string(), 5.0);

        assert_eq!(req.required_items.get("wood"), Some(&4));
        assert_eq!(req.required_tool, Some(ToolType::Axe));
        assert_eq!(req.required_tier, Some(ToolTier::Stone));
    }

    #[test]
    fn test_action_effects() {
        let effects = ActionEffects::none()
            .with_drive_effect(DriveType::Hunger, -0.5)
            .with_energy_cost(10.0)
            .with_experience("farming".to_string(), 15.0);

        assert_eq!(effects.drive_effects.get(&DriveType::Hunger), Some(&-0.5));
        assert_eq!(effects.energy_cost, 10.0);
    }

    #[test]
    fn test_action_context() {
        let ctx = ActionContext::new("agent_123".to_string(), Position::new(0, 0, 0))
            .with_target_position(Position::new(10, 0, 5))
            .with_target_material("stone".to_string())
            .with_parameter("quantity".to_string(), "5".to_string());

        assert_eq!(ctx.agent_id, "agent_123");
        assert_eq!(ctx.target_position, Some(Position::new(10, 0, 5)));
        assert_eq!(ctx.target_material, Some("stone".to_string()));
    }
}
