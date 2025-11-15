// src/core/drives.rs
//! Drive system for agent motivation.
//!
//! Drives represent internal motivations that accumulate over time and
//! trigger goal-seeking behavior. Each drive has:
//! - A current value (0.0 to 1.0)
//! - A threshold for activation
//! - A weight (agent personality)
//! - Increase/decrease conditions

use serde::{Deserialize, Serialize};

/// The 14 core drives that motivate agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveType {
    /// Need for food
    Hunger,
    /// Need for water
    Thirst,
    /// Need for sleep
    Rest,
    /// Need for protective structure
    Shelter,
    /// Need for safety from threats
    Safety,
    /// Need for resource stockpiles
    Preparedness,
    /// Need to gather and process materials
    Industry,
    /// Need to produce food
    Sustenance,
    /// Need to explore and learn
    Curiosity,
    /// Need for proximity to others
    Social,
    /// Need to produce offspring
    Reproduction,
    /// Need for rare or decorative items
    Luxury,
    /// Need for tools and equipment
    Utility,
    /// Need to build structures
    Construction,
}

impl DriveType {
    /// Get all drive types
    pub fn all() -> [DriveType; 14] {
        [
            DriveType::Hunger,
            DriveType::Thirst,
            DriveType::Rest,
            DriveType::Shelter,
            DriveType::Safety,
            DriveType::Preparedness,
            DriveType::Industry,
            DriveType::Sustenance,
            DriveType::Curiosity,
            DriveType::Social,
            DriveType::Reproduction,
            DriveType::Luxury,
            DriveType::Utility,
            DriveType::Construction,
        ]
    }

    /// Get the default threshold for this drive type
    pub fn default_threshold(&self) -> f32 {
        match self {
            DriveType::Hunger => 0.7,
            DriveType::Thirst => 0.75,
            DriveType::Rest => 0.6,
            DriveType::Shelter => 0.5,
            DriveType::Safety => 0.8,
            DriveType::Preparedness => 0.4,
            DriveType::Industry => 0.3,
            DriveType::Sustenance => 0.3,
            DriveType::Curiosity => 0.2,
            DriveType::Social => 0.5,
            DriveType::Reproduction => 0.6,
            DriveType::Luxury => 0.1,
            DriveType::Utility => 0.4,
            DriveType::Construction => 0.3,
        }
    }

    /// Get the base accumulation rate per tick
    pub fn base_accumulation_rate(&self) -> f32 {
        match self {
            DriveType::Hunger => 0.01,
            DriveType::Thirst => 0.012,  // Slightly faster than hunger
            DriveType::Rest => 0.008,
            DriveType::Shelter => 0.005,
            DriveType::Safety => 0.02,  // Spikes with threats
            DriveType::Preparedness => 0.002,
            DriveType::Industry => 0.003,
            DriveType::Sustenance => 0.003,
            DriveType::Curiosity => 0.004,
            DriveType::Social => 0.006,
            DriveType::Reproduction => 0.001,
            DriveType::Luxury => 0.001,
            DriveType::Utility => 0.002,
            DriveType::Construction => 0.002,
        }
    }

    /// Get a description of what satisfies this drive
    pub fn satisfaction_description(&self) -> &'static str {
        match self {
            DriveType::Hunger => "Consuming food",
            DriveType::Thirst => "Drinking water",
            DriveType::Rest => "Sleeping in bed",
            DriveType::Shelter => "Being inside shelter structure",
            DriveType::Safety => "Being in shelter, possessing weapons",
            DriveType::Preparedness => "Stockpiling resources and tools",
            DriveType::Industry => "Mining, smelting, processing materials",
            DriveType::Sustenance => "Farming, harvesting, producing food",
            DriveType::Curiosity => "Exploring, learning, discovering recipes",
            DriveType::Social => "Being near other agents",
            DriveType::Reproduction => "Producing offspring",
            DriveType::Luxury => "Acquiring rare or decorative items",
            DriveType::Utility => "Crafting and maintaining tools",
            DriveType::Construction => "Building structures",
        }
    }
}

/// The state of a single drive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drive {
    pub drive_type: DriveType,
    /// Current value (0.0 to 1.0)
    pub value: f32,
    /// Threshold for activation
    pub threshold: f32,
    /// Personal weight/importance for this agent
    pub weight: f32,
}

impl Drive {
    /// Create a new drive with default values
    pub fn new(drive_type: DriveType) -> Self {
        Self {
            drive_type,
            value: 0.0,
            threshold: drive_type.default_threshold(),
            weight: 1.0,
        }
    }

    /// Create a new drive with custom weight
    pub fn with_weight(drive_type: DriveType, weight: f32) -> Self {
        Self {
            drive_type,
            value: 0.0,
            threshold: drive_type.default_threshold(),
            weight,
        }
    }

    /// Increase the drive value
    pub fn increase(&mut self, amount: f32) {
        self.value = (self.value + amount).min(1.0);
    }

    /// Decrease the drive value
    pub fn decrease(&mut self, amount: f32) {
        self.value = (self.value - amount).max(0.0);
    }

    /// Check if the drive is above threshold
    pub fn is_active(&self) -> bool {
        self.value >= self.threshold
    }

    /// Get the effective urgency (value * weight)
    pub fn urgency(&self) -> f32 {
        self.value * self.weight
    }

    /// Update the drive for one tick
    pub fn tick(&mut self) {
        let rate = self.drive_type.base_accumulation_rate();
        self.increase(rate);
    }

    /// Fully satisfy this drive
    pub fn satisfy(&mut self) {
        self.value = 0.0;
    }

    /// Partially satisfy this drive
    pub fn partial_satisfy(&mut self, amount: f32) {
        self.decrease(amount);
    }
}

/// Complete drive state for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveState {
    pub drives: Vec<Drive>,
}

impl DriveState {
    /// Create a new drive state with default values
    pub fn new() -> Self {
        Self {
            drives: DriveType::all()
                .iter()
                .map(|&dt| Drive::new(dt))
                .collect(),
        }
    }

    /// Create a new drive state with randomized weights
    /// Ensures survival drives (Hunger, Rest, Safety, Shelter) have higher minimum weights
    pub fn with_random_weights() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        Self {
            drives: DriveType::all()
                .iter()
                .map(|&dt| {
                    // Survival-critical drives get higher base weights
                    let weight = match dt {
                        DriveType::Hunger | DriveType::Rest => {
                            // Tier 1 survival: 1.5-2.5 weight range
                            rng.gen_range(1.5..2.5)
                        }
                        DriveType::Safety | DriveType::Shelter => {
                            // Tier 2 survival: 1.0-2.0 weight range
                            rng.gen_range(1.0..2.0)
                        }
                        _ => {
                            // Other drives: 0.5-1.5 weight range (lower than survival)
                            rng.gen_range(0.5..1.5)
                        }
                    };
                    Drive::with_weight(dt, weight)
                })
                .collect(),
        }
    }

    /// Get a drive by type
    pub fn get(&self, drive_type: DriveType) -> Option<&Drive> {
        self.drives.iter().find(|d| d.drive_type == drive_type)
    }

    /// Get a mutable drive by type
    pub fn get_mut(&mut self, drive_type: DriveType) -> Option<&mut Drive> {
        self.drives.iter_mut().find(|d| d.drive_type == drive_type)
    }

    /// Get the most urgent active drive
    pub fn most_urgent(&self) -> Option<&Drive> {
        self.drives
            .iter()
            .filter(|d| d.is_active())
            .max_by(|a, b| a.urgency().partial_cmp(&b.urgency()).unwrap())
    }

    /// Update all drives for one tick
    pub fn tick(&mut self) {
        for drive in &mut self.drives {
            drive.tick();
        }
    }

    /// Get all active drives sorted by urgency
    pub fn active_drives(&self) -> Vec<&Drive> {
        let mut active: Vec<&Drive> = self.drives
            .iter()
            .filter(|d| d.is_active())
            .collect();
        
        active.sort_by(|a, b| b.urgency().partial_cmp(&a.urgency()).unwrap());
        active
    }
}

impl Default for DriveState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_creation() {
        let drive = Drive::new(DriveType::Hunger);
        assert_eq!(drive.value, 0.0);
        assert_eq!(drive.weight, 1.0);
        assert!(!drive.is_active());
    }

    #[test]
    fn test_drive_increase_decrease() {
        let mut drive = Drive::new(DriveType::Hunger);
        
        drive.increase(0.5);
        assert_eq!(drive.value, 0.5);
        
        drive.decrease(0.2);
        assert_eq!(drive.value, 0.3);
    }

    #[test]
    fn test_drive_clamping() {
        let mut drive = Drive::new(DriveType::Hunger);
        
        drive.increase(2.0);
        assert_eq!(drive.value, 1.0);
        
        drive.decrease(2.0);
        assert_eq!(drive.value, 0.0);
    }

    #[test]
    fn test_drive_activation() {
        let mut drive = Drive::new(DriveType::Hunger);
        assert!(!drive.is_active());
        
        drive.value = 0.8;
        assert!(drive.is_active());
    }

    #[test]
    fn test_drive_state_creation() {
        let state = DriveState::new();
        assert_eq!(state.drives.len(), 14);
    }

    #[test]
    fn test_drive_state_get() {
        let state = DriveState::new();
        let hunger = state.get(DriveType::Hunger).unwrap();
        assert_eq!(hunger.drive_type, DriveType::Hunger);
    }

    #[test]
    fn test_most_urgent() {
        let mut state = DriveState::new();
        
        state.get_mut(DriveType::Hunger).unwrap().value = 0.8;
        state.get_mut(DriveType::Safety).unwrap().value = 0.9;
        
        let most_urgent = state.most_urgent().unwrap();
        assert_eq!(most_urgent.drive_type, DriveType::Safety);
    }

    #[test]
    fn test_tick_accumulation() {
        let mut drive = Drive::new(DriveType::Hunger);
        let initial = drive.value;
        
        drive.tick();
        
        assert!(drive.value > initial);
    }
}
