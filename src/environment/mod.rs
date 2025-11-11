// src/environment/mod.rs
//! Environment abstraction layer for different world types.

use serde::{Deserialize, Serialize};
use crate::core::DriveType;

pub struct Environment;
pub struct EnvironmentPlugin;
pub struct Material;
pub struct CraftingTemplate;

/// Actions that agents can perform in the environment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Move to a specific position
    Move { target: (i32, i32, i32) },
    /// Gather a resource
    Gather { resource_type: String },
    /// Consume food
    Eat { food_type: String },
    /// Sleep to restore energy
    Sleep { duration: u32 },
    /// Build a structure
    Build { structure_type: String, position: (i32, i32, i32) },
    /// Craft an item
    Craft { item_type: String },
    /// Store items in inventory or stockpile
    Store { item_type: String, amount: u32 },
    /// Explore a new area
    Explore { direction: (i32, i32, i32) },
    /// Interact with another agent
    Socialize { target_agent_id: uuid::Uuid },
    /// Wait/idle
    Wait,
}

impl Action {
    /// Get which drive this action primarily satisfies
    pub fn primary_drive(&self) -> Option<DriveType> {
        match self {
            Action::Eat { .. } => Some(DriveType::Hunger),
            Action::Sleep { .. } => Some(DriveType::Rest),
            Action::Build { .. } => Some(DriveType::Construction),
            Action::Gather { .. } => Some(DriveType::Industry),
            Action::Craft { .. } => Some(DriveType::Utility),
            Action::Store { .. } => Some(DriveType::Preparedness),
            Action::Explore { .. } => Some(DriveType::Curiosity),
            Action::Socialize { .. } => Some(DriveType::Social),
            Action::Move { .. } => None,
            Action::Wait => None,
        }
    }
}

/// Result of executing an action
#[derive(Debug, Clone, PartialEq)]
pub struct ActionResult {
    pub success: bool,
    pub drive_satisfaction: f32,
    pub message: String,
}

impl ActionResult {
    pub fn success(drive_satisfaction: f32, message: String) -> Self {
        Self {
            success: true,
            drive_satisfaction,
            message,
        }
    }

    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            drive_satisfaction: 0.0,
            message,
        }
    }
}
