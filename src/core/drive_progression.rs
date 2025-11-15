// src/core/drive_progression.rs
//! Hierarchical drive progression system.
//!
//! Drives evolve as basic needs are met:
//! - Food: Storehouse -> Home stores
//! - Shelter: Longhouse -> Personal house -> Upgraded house
//! - Security: Basic gear -> Town wall -> Better gear
//!
//! This creates a dynamic civilization that progresses naturally.

use serde::{Deserialize, Serialize};
use crate::core::DriveType;

/// Tier of drive satisfaction (from basic to advanced)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DriveTier {
    /// Basic survival needs (e.g., any food, any shelter)
    Basic,
    /// Intermediate needs (e.g., personal stores, shared house)
    Intermediate,
    /// Advanced needs (e.g., several days of food, own house)
    Advanced,
    /// Luxury/optimization needs (e.g., upgraded house, abundant supplies)
    Luxury,
}

/// A drive progression defines how a drive evolves through tiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveProgression {
    pub drive_type: DriveType,
    pub current_tier: DriveTier,
    pub tiers: Vec<DriveTierRequirement>,
}

/// Requirements for a specific tier of drive satisfaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveTierRequirement {
    pub tier: DriveTier,
    pub description: String,
    pub requirements: Vec<Requirement>,
    /// Weight/priority of this tier (higher = more urgent)
    pub weight: f32,
}

/// A specific requirement that must be met
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Requirement {
    /// Storehouse must have minimum amount of resource
    StorehouseResource { resource: String, amount: u32 },
    /// Agent's personal home must have minimum amount
    PersonalResource { resource: String, amount: u32 },
    /// A building of a type must exist
    BuildingExists { building_type: String },
    /// Agent must have personal access to building type
    PersonalBuilding { building_type: String },
    /// Agent must own item
    OwnsItem { item: String },
    /// Town must have specific infrastructure
    TownInfrastructure { infrastructure: String },
}

impl DriveProgression {
    /// Create default progression for a drive type
    pub fn new(drive_type: DriveType) -> Self {
        let tiers = match drive_type {
            DriveType::Hunger => Self::hunger_tiers(),
            DriveType::Shelter => Self::shelter_tiers(),
            DriveType::Safety => Self::safety_tiers(),
            DriveType::Preparedness => Self::preparedness_tiers(),
            DriveType::Construction => Self::construction_tiers(),
            _ => vec![], // Other drives use basic tier only
        };

        Self {
            drive_type,
            current_tier: DriveTier::Basic,
            tiers,
        }
    }

    /// Hunger drive progression
    fn hunger_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Storehouse has adequate food for everyone".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "food".to_string(),
                        amount: 100, // Base threshold
                    },
                ],
                weight: 1.0,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Personal home has food supply".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "food".to_string(),
                        amount: 200,
                    },
                    Requirement::PersonalResource {
                        resource: "food".to_string(),
                        amount: 10,
                    },
                ],
                weight: 0.8,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Several days of food at home".to_string(),
                requirements: vec![
                    Requirement::PersonalResource {
                        resource: "food".to_string(),
                        amount: 30, // 3+ days
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Abundant food variety in home".to_string(),
                requirements: vec![
                    Requirement::PersonalResource {
                        resource: "food".to_string(),
                        amount: 100,
                    },
                ],
                weight: 0.4,
            },
        ]
    }

    /// Shelter drive progression
    fn shelter_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Longhouse exists with beds for all".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "longhouse".to_string(),
                    },
                ],
                weight: 1.0,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Personal house for 2-3 agents".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "small_house".to_string(),
                    },
                ],
                weight: 0.7,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Upgraded personal house".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "medium_house".to_string(),
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Large upgraded house with furnishings".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "large_house".to_string(),
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Safety drive progression
    fn safety_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Wooden weapons and leather armor available".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "wooden_weapon".to_string(),
                        amount: 5,
                    },
                    Requirement::StorehouseResource {
                        resource: "leather_armor".to_string(),
                        amount: 5,
                    },
                ],
                weight: 0.9,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Own basic protective gear".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "weapon".to_string(),
                    },
                    Requirement::OwnsItem {
                        item: "armor".to_string(),
                    },
                ],
                weight: 0.7,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Town wall for protection".to_string(),
                requirements: vec![
                    Requirement::TownInfrastructure {
                        infrastructure: "wall".to_string(),
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Iron/steel weapons and metal armor".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "iron_weapon".to_string(),
                    },
                    Requirement::OwnsItem {
                        item: "metal_armor".to_string(),
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Preparedness drive progression
    fn preparedness_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Basic resource stockpiles".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "wood".to_string(),
                        amount: 100,
                    },
                    Requirement::StorehouseResource {
                        resource: "stone".to_string(),
                        amount: 50,
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Diverse material stockpiles".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "wood".to_string(),
                        amount: 500,
                    },
                    Requirement::StorehouseResource {
                        resource: "stone".to_string(),
                        amount: 300,
                    },
                    Requirement::StorehouseResource {
                        resource: "iron".to_string(),
                        amount: 100,
                    },
                ],
                weight: 0.4,
            },
        ]
    }

    /// Construction drive progression
    fn construction_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Basic structures built".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "workshop".to_string(),
                    },
                    Requirement::BuildingExists {
                        building_type: "storehouse".to_string(),
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Multiple specialized buildings".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "smithy".to_string(),
                    },
                    Requirement::BuildingExists {
                        building_type: "farm".to_string(),
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Get the current tier requirements
    pub fn current_requirements(&self) -> Option<&DriveTierRequirement> {
        self.tiers.iter().find(|t| t.tier == self.current_tier)
    }

    /// Check if current tier is satisfied and progress to next if so
    pub fn check_progression(&mut self, satisfaction: f32) -> bool {
        // If current tier is well satisfied (> 0.9), try to progress
        if satisfaction > 0.9 {
            let current_tier_index = self.tiers.iter()
                .position(|t| t.tier == self.current_tier);

            if let Some(idx) = current_tier_index {
                if idx + 1 < self.tiers.len() {
                    // Progress to next tier
                    self.current_tier = self.tiers[idx + 1].tier;
                    return true;
                }
            }
        }

        // If current tier is not satisfied (< 0.5), regress to previous if possible
        if satisfaction < 0.5 && self.current_tier > DriveTier::Basic {
            let current_tier_index = self.tiers.iter()
                .position(|t| t.tier == self.current_tier);

            if let Some(idx) = current_tier_index {
                if idx > 0 {
                    // Regress to previous tier
                    self.current_tier = self.tiers[idx - 1].tier;
                    return true;
                }
            }
        }

        false
    }

    /// Get weight/importance of current tier
    pub fn current_weight(&self) -> f32 {
        self.current_requirements()
            .map(|r| r.weight)
            .unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drive_progression_creation() {
        let progression = DriveProgression::new(DriveType::Hunger);
        assert_eq!(progression.current_tier, DriveTier::Basic);
        assert!(!progression.tiers.is_empty());
    }

    #[test]
    fn test_tier_ordering() {
        assert!(DriveTier::Basic < DriveTier::Intermediate);
        assert!(DriveTier::Intermediate < DriveTier::Advanced);
        assert!(DriveTier::Advanced < DriveTier::Luxury);
    }

    #[test]
    fn test_progression_advance() {
        let mut progression = DriveProgression::new(DriveType::Hunger);
        assert_eq!(progression.current_tier, DriveTier::Basic);

        // High satisfaction should trigger progression
        let progressed = progression.check_progression(0.95);
        assert!(progressed);
        assert_eq!(progression.current_tier, DriveTier::Intermediate);
    }

    #[test]
    fn test_progression_regress() {
        let mut progression = DriveProgression::new(DriveType::Hunger);
        progression.current_tier = DriveTier::Intermediate;

        // Low satisfaction should trigger regression
        let regressed = progression.check_progression(0.3);
        assert!(regressed);
        assert_eq!(progression.current_tier, DriveTier::Basic);
    }

    #[test]
    fn test_hunger_tiers() {
        let tiers = DriveProgression::hunger_tiers();
        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].tier, DriveTier::Basic);
        assert_eq!(tiers[3].tier, DriveTier::Luxury);
    }

    #[test]
    fn test_shelter_tiers() {
        let tiers = DriveProgression::shelter_tiers();
        assert!(tiers.len() >= 3);
        // Should progress from longhouse to personal houses
        assert!(tiers.iter().any(|t| t.description.contains("Longhouse")));
        assert!(tiers.iter().any(|t| t.description.contains("Personal house")));
    }
}
