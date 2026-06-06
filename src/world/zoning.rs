// src/world/zoning.rs
//! Spatial zoning system for organizing settlement development
//!
//! Zones define areas where certain types of buildings are preferred:
//! - Residential: Housing and living spaces
//! - Industrial: Workshops, forges, production facilities
//! - Agricultural: Farms and food production
//! - Commercial: Markets, storehouses, trade facilities

use serde::{Serialize, Deserialize};

/// Types of spatial zones for building placement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneType {
    /// Residential areas for housing
    Residential,
    /// Industrial areas for production and crafting
    Industrial,
    /// Agricultural areas for farming
    Agricultural,
    /// Commercial areas for trade and storage
    Commercial,
    /// Mixed-use areas (no specific preference)
    Mixed,
}

impl ZoneType {
    /// Check if a building type is appropriate for this zone
    pub fn is_appropriate_for_building(&self, building_type: crate::world::BuildingType) -> bool {
        use crate::world::BuildingType;

        match self {
            ZoneType::Residential => matches!(building_type,
                BuildingType::SmallHouse |
                BuildingType::MediumHouse |
                BuildingType::LargeHouse |
                BuildingType::Longhouse |
                BuildingType::UpgradedLonghouse |
                BuildingType::Manor
            ),

            ZoneType::Industrial => matches!(building_type,
                BuildingType::Workshop |
                BuildingType::Forge |
                BuildingType::Smithy |
                BuildingType::PotteryKiln |
                BuildingType::Tannery |
                BuildingType::WeaverHut
            ),

            ZoneType::Agricultural => matches!(building_type,
                BuildingType::Farm |
                BuildingType::Mill |
                BuildingType::Bakery
            ),

            ZoneType::Commercial => matches!(building_type,
                BuildingType::Storehouse |
                BuildingType::TownStorage |
                BuildingType::TownCenter
            ),

            ZoneType::Mixed => true, // All buildings allowed in mixed zones
        }
    }

    /// Get the zone bonus for placing a building in this zone
    /// Returns a multiplier for the location score
    pub fn get_placement_bonus(&self, building_type: crate::world::BuildingType) -> f32 {
        if self.is_appropriate_for_building(building_type) {
            // Strong bonus for placing buildings in appropriate zones
            150.0
        } else {
            // Penalty for placing in wrong zone
            -50.0
        }
    }
}

/// A defined zone with a center point and radius
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub zone_type: ZoneType,
    pub center: (i32, i32, i32),
    pub radius: i32,
}

impl Zone {
    pub fn new(zone_type: ZoneType, center: (i32, i32, i32), radius: i32) -> Self {
        Self {
            zone_type,
            center,
            radius,
        }
    }

    /// Check if a position is within this zone
    pub fn contains(&self, position: (i32, i32, i32)) -> bool {
        let dx = (position.0 - self.center.0) as f32;
        let dy = (position.1 - self.center.1) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        distance <= self.radius as f32
    }

    /// Get the distance from the center of this zone
    pub fn distance_from_center(&self, position: (i32, i32, i32)) -> f32 {
        let dx = (position.0 - self.center.0) as f32;
        let dy = (position.1 - self.center.1) as f32;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Manages all zones in the world
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZoneManager {
    zones: Vec<Zone>,
}

impl ZoneManager {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
        }
    }

    /// Add a new zone to the world
    pub fn add_zone(&mut self, zone_type: ZoneType, center: (i32, i32, i32), radius: i32) {
        self.zones.push(Zone::new(zone_type, center, radius));
    }

    /// Remove all zones
    pub fn clear_zones(&mut self) {
        self.zones.clear();
    }

    /// Get all zones
    pub fn get_zones(&self) -> &[Zone] {
        &self.zones
    }

    /// Get all zones that contain a specific position
    pub fn get_zones_at_position(&self, position: (i32, i32, i32)) -> Vec<ZoneType> {
        self.zones.iter()
            .filter(|zone| zone.contains(position))
            .map(|zone| zone.zone_type)
            .collect()
    }

    /// Check if a position is within any zone of a specific type
    pub fn is_in_zone_type(&self, position: (i32, i32, i32), zone_type: ZoneType) -> bool {
        self.zones.iter()
            .any(|zone| zone.zone_type == zone_type && zone.contains(position))
    }

    /// Get the total zone bonus for placing a building at a position
    /// Considers all zones at that position
    pub fn get_zone_bonus(&self, position: (i32, i32, i32), building_type: crate::world::BuildingType) -> f32 {
        let zones_at_pos = self.get_zones_at_position(position);

        if zones_at_pos.is_empty() {
            // No zone defined - neutral (small penalty to encourage zoning)
            return -10.0;
        }

        // Return the maximum bonus from any applicable zone
        zones_at_pos.iter()
            .map(|zone_type| zone_type.get_placement_bonus(building_type))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// Find the nearest zone of a specific type to a position
    pub fn find_nearest_zone(&self, position: (i32, i32, i32), zone_type: ZoneType) -> Option<&Zone> {
        self.zones.iter()
            .filter(|zone| zone.zone_type == zone_type)
            .min_by(|a, b| {
                let dist_a = a.distance_from_center(position);
                let dist_b = b.distance_from_center(position);
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::BuildingType;

    #[test]
    fn test_zone_creation() {
        let zone = Zone::new(ZoneType::Residential, (10, 10, 0), 5);
        assert_eq!(zone.zone_type, ZoneType::Residential);
        assert_eq!(zone.center, (10, 10, 0));
        assert_eq!(zone.radius, 5);
    }

    #[test]
    fn test_zone_contains() {
        let zone = Zone::new(ZoneType::Residential, (10, 10, 0), 5);

        assert!(zone.contains((10, 10, 0)), "Center should be in zone");
        assert!(zone.contains((12, 12, 0)), "Nearby position should be in zone");
        assert!(!zone.contains((20, 20, 0)), "Far position should not be in zone");
    }

    #[test]
    fn test_residential_zone_appropriateness() {
        let zone_type = ZoneType::Residential;

        assert!(zone_type.is_appropriate_for_building(BuildingType::SmallHouse));
        assert!(zone_type.is_appropriate_for_building(BuildingType::MediumHouse));
        assert!(!zone_type.is_appropriate_for_building(BuildingType::Workshop));
        assert!(!zone_type.is_appropriate_for_building(BuildingType::Farm));
    }

    #[test]
    fn test_industrial_zone_appropriateness() {
        let zone_type = ZoneType::Industrial;

        assert!(zone_type.is_appropriate_for_building(BuildingType::Workshop));
        assert!(zone_type.is_appropriate_for_building(BuildingType::Forge));
        assert!(!zone_type.is_appropriate_for_building(BuildingType::SmallHouse));
    }

    #[test]
    fn test_zone_manager_add_zone() {
        let mut manager = ZoneManager::new();
        assert_eq!(manager.get_zones().len(), 0);

        manager.add_zone(ZoneType::Residential, (10, 10, 0), 5);
        assert_eq!(manager.get_zones().len(), 1);
    }

    #[test]
    fn test_zone_manager_get_zones_at_position() {
        let mut manager = ZoneManager::new();
        manager.add_zone(ZoneType::Residential, (10, 10, 0), 5);
        manager.add_zone(ZoneType::Commercial, (12, 12, 0), 3);

        let zones = manager.get_zones_at_position((11, 11, 0));
        assert!(zones.contains(&ZoneType::Residential));
        // Position (11, 11) might or might not be in commercial zone depending on exact distance
    }
}
