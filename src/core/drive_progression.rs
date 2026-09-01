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
            DriveType::Thirst => Self::thirst_tiers(),
            DriveType::Rest => Self::rest_tiers(),
            DriveType::Shelter => Self::shelter_tiers(),
            DriveType::Safety => Self::safety_tiers(),
            // No ladder either. There is no better and worse way to drive a
            // thing off - it goes or it does not - so this is answered
            // outright rather than by degrees.
            DriveType::Aggression => Vec::new(),
            DriveType::Preparedness => Self::preparedness_tiers(),
            DriveType::Industry => Self::industry_tiers(),
            DriveType::Sustenance => Self::sustenance_tiers(),
            DriveType::Curiosity => Self::curiosity_tiers(),
            DriveType::Social => Self::social_tiers(),
            DriveType::Reproduction => Self::reproduction_tiers(),
            DriveType::Luxury => Self::luxury_tiers(),
            DriveType::Utility => Self::utility_tiers(),
            DriveType::Construction => Self::construction_tiers(),
            // Protection has no ladder of its own: it is answered by being
            // where the children are, not by acquiring anything
            DriveType::Protection => Self::safety_tiers(),
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

    /// Thirst drive progression
    fn thirst_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Access to water source".to_string(),
                requirements: vec![
                    Requirement::TownInfrastructure {
                        infrastructure: "water_source".to_string(),
                    },
                ],
                weight: 1.0,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Well or cistern for reliable water".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "well".to_string(),
                    },
                ],
                weight: 0.8,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Personal water storage at home".to_string(),
                requirements: vec![
                    Requirement::PersonalResource {
                        resource: "water".to_string(),
                        amount: 20,
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Abundant clean water supply".to_string(),
                requirements: vec![
                    Requirement::PersonalResource {
                        resource: "water".to_string(),
                        amount: 50,
                    },
                    Requirement::TownInfrastructure {
                        infrastructure: "aqueduct".to_string(),
                    },
                ],
                weight: 0.4,
            },
        ]
    }

    /// Rest drive progression
    fn rest_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Place to sleep exists".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "longhouse".to_string(),
                    },
                ],
                weight: 1.0,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Personal bed in shared dwelling".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "bed".to_string(),
                    },
                ],
                weight: 0.8,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Private sleeping quarters".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "small_house".to_string(),
                    },
                    Requirement::OwnsItem {
                        item: "bed".to_string(),
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Comfortable bedroom with quality furnishings".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "medium_house".to_string(),
                    },
                    Requirement::OwnsItem {
                        item: "quality_bed".to_string(),
                    },
                ],
                weight: 0.4,
            },
        ]
    }

    /// Industry drive progression
    fn industry_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Basic gathering tools available".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "tools".to_string(),
                        amount: 5,
                    },
                ],
                weight: 0.7,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Workshop for processing materials".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "workshop".to_string(),
                    },
                    Requirement::StorehouseResource {
                        resource: "tools".to_string(),
                        amount: 10,
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Smithy for metalworking".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "smithy".to_string(),
                    },
                    Requirement::StorehouseResource {
                        resource: "iron".to_string(),
                        amount: 50,
                    },
                ],
                weight: 0.4,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Advanced crafting infrastructure".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "smithy".to_string(),
                    },
                    Requirement::BuildingExists {
                        building_type: "workshop".to_string(),
                    },
                    Requirement::StorehouseResource {
                        resource: "iron".to_string(),
                        amount: 200,
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Sustenance drive progression (food production)
    fn sustenance_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Hunting and foraging grounds accessible".to_string(),
                requirements: vec![
                    Requirement::TownInfrastructure {
                        infrastructure: "hunting_grounds".to_string(),
                    },
                ],
                weight: 0.8,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Basic agriculture established".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "farm".to_string(),
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Multiple farms and livestock".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "farm".to_string(),
                    },
                    Requirement::BuildingExists {
                        building_type: "animal_pen".to_string(),
                    },
                    Requirement::StorehouseResource {
                        resource: "grain".to_string(),
                        amount: 100,
                    },
                ],
                weight: 0.4,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Food surplus with variety".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "food".to_string(),
                        amount: 500,
                    },
                    Requirement::StorehouseResource {
                        resource: "grain".to_string(),
                        amount: 300,
                    },
                    Requirement::StorehouseResource {
                        resource: "meat".to_string(),
                        amount: 100,
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Curiosity drive progression (exploration and learning)
    fn curiosity_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Explored immediate surroundings".to_string(),
                requirements: vec![
                    Requirement::TownInfrastructure {
                        infrastructure: "explored_area".to_string(),
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Discovered useful resources".to_string(),
                requirements: vec![
                    Requirement::StorehouseResource {
                        resource: "stone".to_string(),
                        amount: 100,
                    },
                    Requirement::StorehouseResource {
                        resource: "iron".to_string(),
                        amount: 20,
                    },
                ],
                weight: 0.4,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Knowledge archive established".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "library".to_string(),
                    },
                ],
                weight: 0.3,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Research and scholarly pursuits".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "library".to_string(),
                    },
                    Requirement::StorehouseResource {
                        resource: "books".to_string(),
                        amount: 20,
                    },
                ],
                weight: 0.2,
            },
        ]
    }

    /// Social drive progression (community bonds)
    fn social_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Part of a community".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "longhouse".to_string(),
                    },
                ],
                weight: 0.7,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Community gathering space".to_string(),
                requirements: vec![
                    Requirement::BuildingExists {
                        building_type: "town_center".to_string(),
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Strong family and friend bonds".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "small_house".to_string(),
                    },
                    Requirement::TownInfrastructure {
                        infrastructure: "meeting_hall".to_string(),
                    },
                ],
                weight: 0.4,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Leadership role in community".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "manor".to_string(),
                    },
                    Requirement::TownInfrastructure {
                        infrastructure: "council_hall".to_string(),
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Reproduction drive progression (family and offspring)
    fn reproduction_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Eligible for partnership".to_string(),
                requirements: vec![
                    Requirement::TownInfrastructure {
                        infrastructure: "community".to_string(),
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Stable partnership formed".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "small_house".to_string(),
                    },
                ],
                weight: 0.5,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Family home with resources for children".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "medium_house".to_string(),
                    },
                    Requirement::PersonalResource {
                        resource: "food".to_string(),
                        amount: 50,
                    },
                ],
                weight: 0.4,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Multi-generational family estate".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "large_house".to_string(),
                    },
                    Requirement::PersonalResource {
                        resource: "food".to_string(),
                        amount: 100,
                    },
                ],
                weight: 0.3,
            },
        ]
    }

    /// Luxury drive progression (comfort and decoration)
    fn luxury_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Basic comfort items".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "furniture".to_string(),
                    },
                ],
                weight: 0.3,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Decorated living space".to_string(),
                requirements: vec![
                    Requirement::PersonalResource {
                        resource: "decorations".to_string(),
                        amount: 5,
                    },
                    Requirement::OwnsItem {
                        item: "quality_furniture".to_string(),
                    },
                ],
                weight: 0.2,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Fine clothing and jewelry".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "fine_clothing".to_string(),
                    },
                    Requirement::OwnsItem {
                        item: "jewelry".to_string(),
                    },
                ],
                weight: 0.15,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Opulent lifestyle with rare items".to_string(),
                requirements: vec![
                    Requirement::PersonalBuilding {
                        building_type: "manor".to_string(),
                    },
                    Requirement::PersonalResource {
                        resource: "rare_goods".to_string(),
                        amount: 10,
                    },
                ],
                weight: 0.1,
            },
        ]
    }

    /// Utility drive progression (tools and equipment)
    fn utility_tiers() -> Vec<DriveTierRequirement> {
        vec![
            DriveTierRequirement {
                tier: DriveTier::Basic,
                description: "Basic tools for work".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "stone_tool".to_string(),
                    },
                ],
                weight: 0.8,
            },
            DriveTierRequirement {
                tier: DriveTier::Intermediate,
                description: "Improved tool set".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "iron_tool".to_string(),
                    },
                    Requirement::StorehouseResource {
                        resource: "tools".to_string(),
                        amount: 10,
                    },
                ],
                weight: 0.6,
            },
            DriveTierRequirement {
                tier: DriveTier::Advanced,
                description: "Specialized equipment for trade".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "specialized_tools".to_string(),
                    },
                    Requirement::BuildingExists {
                        building_type: "workshop".to_string(),
                    },
                ],
                weight: 0.4,
            },
            DriveTierRequirement {
                tier: DriveTier::Luxury,
                description: "Master craftsman equipment".to_string(),
                requirements: vec![
                    Requirement::OwnsItem {
                        item: "master_tools".to_string(),
                    },
                    Requirement::PersonalBuilding {
                        building_type: "workshop".to_string(),
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

    #[test]
    fn test_thirst_tiers() {
        let tiers = DriveProgression::thirst_tiers();
        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].tier, DriveTier::Basic);
        assert!(tiers.iter().any(|t| t.description.contains("water")));
    }

    #[test]
    fn test_rest_tiers() {
        let tiers = DriveProgression::rest_tiers();
        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].tier, DriveTier::Basic);
        assert!(tiers.iter().any(|t| t.description.contains("sleep")));
    }

    #[test]
    fn test_industry_tiers() {
        let tiers = DriveProgression::industry_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("Workshop")));
        assert!(tiers.iter().any(|t| t.description.contains("Smithy")));
    }

    #[test]
    fn test_sustenance_tiers() {
        let tiers = DriveProgression::sustenance_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("agriculture") || t.description.contains("farm")));
    }

    #[test]
    fn test_curiosity_tiers() {
        let tiers = DriveProgression::curiosity_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("Explored")));
    }

    #[test]
    fn test_social_tiers() {
        let tiers = DriveProgression::social_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("community")));
    }

    #[test]
    fn test_reproduction_tiers() {
        let tiers = DriveProgression::reproduction_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("partnership")));
        assert!(tiers.iter().any(|t| t.description.contains("Family")));
    }

    #[test]
    fn test_luxury_tiers() {
        let tiers = DriveProgression::luxury_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("comfort")));
        // Luxury tier should have lowest weight (optional/nice-to-have)
        assert!(tiers.last().unwrap().weight < 0.2);
    }

    #[test]
    fn test_utility_tiers() {
        let tiers = DriveProgression::utility_tiers();
        assert_eq!(tiers.len(), 4);
        assert!(tiers.iter().any(|t| t.description.contains("tools")));
    }

    #[test]
    fn test_all_drives_have_progressions() {
        // All 14 drive types should have progression tiers
        let drives = vec![
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
        ];

        for drive in drives {
            let progression = DriveProgression::new(drive);
            assert!(!progression.tiers.is_empty(), "Drive {:?} should have progression tiers", drive);
            assert!(progression.tiers.len() >= 2, "Drive {:?} should have at least 2 tiers", drive);
        }
    }
}
