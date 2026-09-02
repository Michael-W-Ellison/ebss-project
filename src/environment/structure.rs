// src/environment/structure.rs
//! Structure system for buildings and constructions.
//!
//! Structures are permanent objects built in the world that provide
//! functionality like storage, crafting stations, or shelter.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Types of structures that can be built
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureType {
    /// Water well - provides water access
    Well,
    /// Water cistern - stores large amounts of water
    Cistern,
    /// Water tower - distributes water to area
    WaterTower,
    /// Shelter for agents
    Shelter,
    /// Storage building
    Storage,
    /// Crafting workshop
    Workshop,
    /// Farm plot
    Farm,
    /// Custom structure type
    Custom(u32), // Using u32 instead of String for Copy
}

/// Structure quality/level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StructureLevel {
    Basic = 1,
    Improved = 2,
    Advanced = 3,
    Expert = 4,
    Master = 5,
}

impl StructureLevel {
    /// Get the level as a number
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Get multiplier for capacity/efficiency based on level
    pub fn multiplier(&self) -> f32 {
        match self {
            StructureLevel::Basic => 1.0,
            StructureLevel::Improved => 1.5,
            StructureLevel::Advanced => 2.5,
            StructureLevel::Expert => 4.0,
            StructureLevel::Master => 6.0,
        }
    }

    /// Get the next level, if any
    pub fn next_level(&self) -> Option<StructureLevel> {
        match self {
            StructureLevel::Basic => Some(StructureLevel::Improved),
            StructureLevel::Improved => Some(StructureLevel::Advanced),
            StructureLevel::Advanced => Some(StructureLevel::Expert),
            StructureLevel::Expert => Some(StructureLevel::Master),
            StructureLevel::Master => None,
        }
    }
}

/// A built structure in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structure {
    pub id: String,
    pub name: String,
    pub structure_type: StructureType,
    pub level: StructureLevel,
    pub position: (i32, i32, i32),

    // Capacity (for storage structures)
    pub capacity: f32,
    pub current_fill: f32,

    // Health/durability
    pub health: f32,
    pub max_health: f32,

    // Construction
    pub build_progress: f32, // 0.0 to 1.0
    pub is_complete: bool,

    // Custom properties
    pub properties: BTreeMap<String, String>,
}

impl Structure {
    /// Create a new structure
    pub fn new(
        id: String,
        name: String,
        structure_type: StructureType,
        level: StructureLevel,
        position: (i32, i32, i32),
    ) -> Self {
        let base_capacity = Self::base_capacity(structure_type);
        let capacity = base_capacity * level.multiplier();
        let max_health = 100.0 * level.multiplier();

        Self {
            id,
            name,
            structure_type,
            level,
            position,
            capacity,
            current_fill: 0.0,
            health: max_health,
            max_health,
            build_progress: 0.0,
            is_complete: false,
            properties: BTreeMap::new(),
        }
    }

    /// Get base capacity for structure type
    fn base_capacity(structure_type: StructureType) -> f32 {
        match structure_type {
            StructureType::Well => 500.0,
            StructureType::Cistern => 2000.0,
            StructureType::WaterTower => 5000.0,
            StructureType::Storage => 1000.0,
            _ => 0.0,
        }
    }

    /// Add to structure (water, items, etc.)
    pub fn add(&mut self, amount: f32) -> f32 {
        let space_available = self.capacity - self.current_fill;
        let amount_to_add = amount.min(space_available);
        self.current_fill += amount_to_add;
        amount_to_add
    }

    /// Remove from structure
    pub fn remove(&mut self, amount: f32) -> f32 {
        let amount_to_remove = amount.min(self.current_fill);
        self.current_fill -= amount_to_remove;
        amount_to_remove
    }

    /// Get fill percentage
    pub fn fill_percentage(&self) -> f32 {
        if self.capacity > 0.0 {
            self.current_fill / self.capacity
        } else {
            0.0
        }
    }

    /// Check if structure is water storage
    pub fn is_water_storage(&self) -> bool {
        matches!(
            self.structure_type,
            StructureType::Well | StructureType::Cistern | StructureType::WaterTower
        )
    }

    /// Progress construction
    pub fn build(&mut self, progress_amount: f32) {
        if !self.is_complete {
            self.build_progress = (self.build_progress + progress_amount).min(1.0);
            if self.build_progress >= 1.0 {
                self.is_complete = true;
            }
        }
    }

    /// Upgrade to next level (returns required materials)
    pub fn can_upgrade(&self) -> bool {
        self.is_complete && self.level.next_level().is_some()
    }

    /// Perform upgrade
    pub fn upgrade(&mut self) -> bool {
        if let Some(next_level) = self.level.next_level() {
            self.level = next_level;

            // Recalculate capacity and health
            let base_capacity = Self::base_capacity(self.structure_type);
            let new_capacity = base_capacity * self.level.multiplier();

            // Preserve fill percentage when upgrading
            let fill_percentage = self.fill_percentage();
            self.capacity = new_capacity;
            self.current_fill = new_capacity * fill_percentage;

            // Restore health
            self.max_health = 100.0 * self.level.multiplier();
            self.health = self.max_health;

            true
        } else {
            false
        }
    }

    /// Get required materials for building
    pub fn build_requirements(&self) -> BTreeMap<String, u32> {
        let mut requirements = BTreeMap::new();

        let material_multiplier = self.level.as_u8() as u32;

        match self.structure_type {
            StructureType::Well => {
                requirements.insert("stone".to_string(), 20 * material_multiplier);
                requirements.insert("wood".to_string(), 10 * material_multiplier);
            }
            StructureType::Cistern => {
                requirements.insert("stone".to_string(), 50 * material_multiplier);
                requirements.insert("clay".to_string(), 30 * material_multiplier);
            }
            StructureType::WaterTower => {
                requirements.insert("stone".to_string(), 100 * material_multiplier);
                requirements.insert("iron".to_string(), 50 * material_multiplier);
                requirements.insert("wood".to_string(), 40 * material_multiplier);
            }
            StructureType::Shelter => {
                requirements.insert("wood".to_string(), 30 * material_multiplier);
                requirements.insert("stone".to_string(), 15 * material_multiplier);
            }
            StructureType::Storage => {
                requirements.insert("wood".to_string(), 40 * material_multiplier);
            }
            StructureType::Workshop => {
                requirements.insert("wood".to_string(), 25 * material_multiplier);
                requirements.insert("stone".to_string(), 20 * material_multiplier);
            }
            StructureType::Farm => {
                requirements.insert("wood".to_string(), 15 * material_multiplier);
                requirements.insert("dirt".to_string(), 10 * material_multiplier);
            }
            StructureType::Custom(_) => {}
        }

        requirements
    }

}

/// Manager for all structures in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureRegistry {
    structures: BTreeMap<String, Structure>,
    position_index: BTreeMap<(i32, i32, i32), String>,
}

impl StructureRegistry {
    pub fn new() -> Self {
        Self {
            structures: BTreeMap::new(),
            position_index: BTreeMap::new(),
        }
    }

    /// Add a structure
    pub fn add_structure(&mut self, structure: Structure) -> bool {
        if self.position_index.contains_key(&structure.position) {
            return false; // Position already occupied
        }

        let id = structure.id.clone();
        let position = structure.position;

        self.structures.insert(id.clone(), structure);
        self.position_index.insert(position, id);
        true
    }


    /// Get structure by ID
    pub fn get_structure(&self, id: &str) -> Option<&Structure> {
        self.structures.get(id)
    }


    /// Get structure at position
    pub fn get_structure_at(&self, position: (i32, i32, i32)) -> Option<&Structure> {
        self.position_index.get(&position)
            .and_then(|id| self.structures.get(id))
    }



    /// Get all water storage structures
    pub fn get_water_storage_structures(&self) -> Vec<&Structure> {
        self.structures.values()
            .filter(|s| s.is_water_storage())
            .collect()
    }



    /// Get total water stored in all structures
    pub fn get_total_water(&self) -> f32 {
        self.get_water_storage_structures()
            .iter()
            .map(|s| s.current_fill)
            .sum()
    }
}

impl Default for StructureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structure_creation() {
        let well = Structure::new(
            "well_1".to_string(),
            "Village Well".to_string(),
            StructureType::Well,
            StructureLevel::Basic,
            (0, 64, 0),
        );

        assert_eq!(well.structure_type, StructureType::Well);
        assert_eq!(well.level, StructureLevel::Basic);
        assert_eq!(well.capacity, 500.0);
        assert!(!well.is_complete);
    }

    #[test]
    fn test_structure_building() {
        let mut well = Structure::new(
            "well_1".to_string(),
            "Village Well".to_string(),
            StructureType::Well,
            StructureLevel::Basic,
            (0, 64, 0),
        );

        well.build(0.5);
        assert_eq!(well.build_progress, 0.5);
        assert!(!well.is_complete);

        well.build(0.5);
        assert_eq!(well.build_progress, 1.0);
        assert!(well.is_complete);
    }

    #[test]
    fn test_structure_storage() {
        let mut well = Structure::new(
            "well_1".to_string(),
            "Village Well".to_string(),
            StructureType::Well,
            StructureLevel::Basic,
            (0, 64, 0),
        );

        let added = well.add(100.0);
        assert_eq!(added, 100.0);
        assert_eq!(well.current_fill, 100.0);

        let removed = well.remove(50.0);
        assert_eq!(removed, 50.0);
        assert_eq!(well.current_fill, 50.0);
    }

    #[test]
    fn test_structure_upgrade() {
        let mut well = Structure::new(
            "well_1".to_string(),
            "Village Well".to_string(),
            StructureType::Well,
            StructureLevel::Basic,
            (0, 64, 0),
        );

        well.is_complete = true;
        well.current_fill = 250.0; // 50% full

        assert!(well.can_upgrade());
        assert!(well.upgrade());

        assert_eq!(well.level, StructureLevel::Improved);
        assert_eq!(well.capacity, 500.0 * 1.5);
        // Fill percentage should be preserved
        assert!((well.fill_percentage() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_structure_registry() {
        let mut registry = StructureRegistry::new();

        let well = Structure::new(
            "well_1".to_string(),
            "Village Well".to_string(),
            StructureType::Well,
            StructureLevel::Basic,
            (0, 64, 0),
        );

        assert!(registry.add_structure(well));
        assert!(registry.get_structure("well_1").is_some());
        assert!(registry.get_structure_at((0, 64, 0)).is_some());
    }

    #[test]
    fn test_structure_level_multiplier() {
        assert_eq!(StructureLevel::Basic.multiplier(), 1.0);
        assert_eq!(StructureLevel::Improved.multiplier(), 1.5);
        assert_eq!(StructureLevel::Advanced.multiplier(), 2.5);
        assert_eq!(StructureLevel::Expert.multiplier(), 4.0);
        assert_eq!(StructureLevel::Master.multiplier(), 6.0);
    }

    #[test]
    fn test_build_requirements() {
        let well = Structure::new(
            "well_1".to_string(),
            "Village Well".to_string(),
            StructureType::Well,
            StructureLevel::Basic,
            (0, 64, 0),
        );

        let requirements = well.build_requirements();
        assert_eq!(requirements.get("stone"), Some(&20));
        assert_eq!(requirements.get("wood"), Some(&10));
    }
}
