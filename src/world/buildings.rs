// src/world/buildings.rs
//! Buildings and construction system.

use serde::{Deserialize, Serialize};
use crate::world::{Position, Resource, ResourceType};

/// Types of buildings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingType {
    // Shelter progression
    Longhouse,    // Basic: Shared housing
    SmallHouse,   // Intermediate: Personal 1-2 person home
    MediumHouse,  // Advanced: Family home
    LargeHouse,   // Luxury: Large multi-room home

    // Infrastructure
    Storehouse,   // Resource storage
    Workshop,     // Crafting tools
    Smithy,       // Advanced metalworking
    Farm,         // Food production
}

impl BuildingType {
    /// Get construction requirements
    pub fn requirements(&self) -> Vec<Resource> {
        match self {
            BuildingType::Longhouse => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 50),
            ],
            BuildingType::SmallHouse => vec![
                Resource::new(ResourceType::Wood, 50),
                Resource::new(ResourceType::Stone, 30),
            ],
            BuildingType::MediumHouse => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 50),
                Resource::new(ResourceType::Iron, 10),
            ],
            BuildingType::LargeHouse => vec![
                Resource::new(ResourceType::Wood, 120),
                Resource::new(ResourceType::Stone, 80),
                Resource::new(ResourceType::Iron, 30),
            ],
            BuildingType::Storehouse => vec![
                Resource::new(ResourceType::Wood, 150),
                Resource::new(ResourceType::Stone, 100),
            ],
            BuildingType::Workshop => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 60),
            ],
            BuildingType::Smithy => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 150),
                Resource::new(ResourceType::Iron, 50),
            ],
            BuildingType::Farm => vec![
                Resource::new(ResourceType::Wood, 60),
                Resource::new(ResourceType::Stone, 40),
            ],
        }
    }

    /// Get construction time (in ticks)
    pub fn construction_time(&self) -> u32 {
        match self {
            BuildingType::Longhouse => 500,
            BuildingType::SmallHouse => 300,
            BuildingType::MediumHouse => 400,
            BuildingType::LargeHouse => 600,
            BuildingType::Storehouse => 400,
            BuildingType::Workshop => 350,
            BuildingType::Smithy => 500,
            BuildingType::Farm => 300,
        }
    }

    /// Get building capacity (for housing)
    pub fn capacity(&self) -> usize {
        match self {
            BuildingType::Longhouse => 10,
            BuildingType::SmallHouse => 2,
            BuildingType::MediumHouse => 4,
            BuildingType::LargeHouse => 6,
            _ => 0,
        }
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            BuildingType::Longhouse => 'L',
            BuildingType::SmallHouse => 'h',
            BuildingType::MediumHouse => 'H',
            BuildingType::LargeHouse => '#',
            BuildingType::Storehouse => 'S',
            BuildingType::Workshop => 'W',
            BuildingType::Smithy => 'M',
            BuildingType::Farm => 'F',
        }
    }

    /// Get color code for terminal rendering
    pub fn color_code(&self) -> &'static str {
        match self {
            BuildingType::Longhouse | BuildingType::SmallHouse |
            BuildingType::MediumHouse | BuildingType::LargeHouse => "\x1b[35m", // Magenta
            BuildingType::Storehouse => "\x1b[33m",   // Yellow
            BuildingType::Workshop => "\x1b[36m",     // Cyan
            BuildingType::Smithy => "\x1b[31m",       // Red
            BuildingType::Farm => "\x1b[32m",         // Green
        }
    }
}

/// Building construction state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingState {
    UnderConstruction { progress: u32 }, // Progress in ticks
    Completed,
}

/// A building in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub building_type: BuildingType,
    pub position: Position,
    pub state: BuildingState,
    pub owner: Option<uuid::Uuid>, // Optional owner (for houses)
    pub occupants: Vec<uuid::Uuid>, // Agents currently living here
}

impl Building {
    pub fn new(building_type: BuildingType, position: Position) -> Self {
        Self {
            building_type,
            position,
            state: BuildingState::Completed, // Start completed for initial buildings
            owner: None,
            occupants: Vec::new(),
        }
    }

    pub fn new_under_construction(building_type: BuildingType, position: Position) -> Self {
        Self {
            building_type,
            position,
            state: BuildingState::UnderConstruction { progress: 0 },
            owner: None,
            occupants: Vec::new(),
        }
    }

    /// Advance construction progress
    pub fn add_construction_progress(&mut self, ticks: u32) -> bool {
        if let BuildingState::UnderConstruction { progress } = &mut self.state {
            *progress += ticks;
            let required = self.building_type.construction_time();

            if *progress >= required {
                self.state = BuildingState::Completed;
                return true; // Construction completed
            }
        }
        false
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.state, BuildingState::Completed)
    }

    pub fn is_housing(&self) -> bool {
        matches!(
            self.building_type,
            BuildingType::Longhouse | BuildingType::SmallHouse |
            BuildingType::MediumHouse | BuildingType::LargeHouse
        )
    }

    pub fn can_house_agent(&self) -> bool {
        self.is_completed()
            && self.is_housing()
            && self.occupants.len() < self.building_type.capacity()
    }

    pub fn add_occupant(&mut self, agent_id: uuid::Uuid) -> bool {
        if self.can_house_agent() {
            self.occupants.push(agent_id);
            true
        } else {
            false
        }
    }

    pub fn remove_occupant(&mut self, agent_id: uuid::Uuid) {
        self.occupants.retain(|id| id != &agent_id);
    }

    pub fn tick(&mut self) {
        // Buildings could decay, produce resources, etc. in the future
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_building_requirements() {
        let reqs = BuildingType::SmallHouse.requirements();
        assert!(reqs.len() >= 2);

        // Should have wood and stone
        assert!(reqs.iter().any(|r| r.resource_type == ResourceType::Wood));
        assert!(reqs.iter().any(|r| r.resource_type == ResourceType::Stone));
    }

    #[test]
    fn test_building_construction() {
        let pos = Position::new(10, 10);
        let mut building = Building::new_under_construction(BuildingType::SmallHouse, pos);

        assert!(!building.is_completed());

        // Add progress
        let completed = building.add_construction_progress(100);
        assert!(!completed); // Not enough progress

        // Complete construction
        let completed = building.add_construction_progress(300);
        assert!(completed);
        assert!(building.is_completed());
    }

    #[test]
    fn test_building_occupancy() {
        let pos = Position::new(10, 10);
        let mut building = Building::new(BuildingType::SmallHouse, pos);

        assert!(building.can_house_agent());
        assert_eq!(building.building_type.capacity(), 2);

        let agent1 = uuid::Uuid::new_v4();
        let agent2 = uuid::Uuid::new_v4();
        let agent3 = uuid::Uuid::new_v4();

        assert!(building.add_occupant(agent1));
        assert!(building.add_occupant(agent2));
        assert!(!building.add_occupant(agent3)); // Full

        building.remove_occupant(agent1);
        assert!(building.add_occupant(agent3)); // Now there's room
    }
}
