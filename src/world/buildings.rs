// src/world/buildings.rs
//! Buildings and construction system.

use serde::{Deserialize, Serialize};
use crate::world::{Position, Resource, ResourceType};

/// Types of buildings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingType {
    // Shelter progression
    Longhouse,         // Basic: Shared housing (10 capacity)
    UpgradedLonghouse, // Improved longhouse (15 capacity)
    SmallHouse,        // Personal 1-2 person home
    MediumHouse,       // Family home (4 capacity)
    LargeHouse,        // Large multi-room home (6 capacity)
    Manor,             // Luxury estate (8 capacity)

    // Civic buildings
    TownCenter,        // Administrative center
    TownStorage,       // Large community storage

    // Production buildings
    Workshop,          // Basic crafting tools
    Forge,             // Basic metalworking
    Smithy,            // Advanced metalworking
    Bakery,            // Food processing
    WeaverHut,         // Textile production
    PotteryKiln,       // Pottery production
    Tannery,           // Leather working

    // Resource buildings
    Storehouse,        // Basic resource storage
    Farm,              // Food production
    AnimalPen,         // Animal husbandry

    // Religious buildings
    Shrine,            // Small religious site
    Temple,            // Large religious structure

    // Medical/support
    MedicalBuilding,   // Healing and medicine
}

impl BuildingType {
    /// Get construction requirements
    pub fn requirements(&self) -> Vec<Resource> {
        match self {
            // Housing
            BuildingType::Longhouse => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 50),
            ],
            BuildingType::UpgradedLonghouse => vec![
                Resource::new(ResourceType::Wood, 150),
                Resource::new(ResourceType::Stone, 80),
                Resource::new(ResourceType::Iron, 20),
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
            BuildingType::Manor => vec![
                Resource::new(ResourceType::Wood, 200),
                Resource::new(ResourceType::Stone, 150),
                Resource::new(ResourceType::Iron, 50),
            ],

            // Civic
            BuildingType::TownCenter => vec![
                Resource::new(ResourceType::Wood, 250),
                Resource::new(ResourceType::Stone, 200),
                Resource::new(ResourceType::Iron, 80),
            ],
            BuildingType::TownStorage => vec![
                Resource::new(ResourceType::Wood, 200),
                Resource::new(ResourceType::Stone, 150),
                Resource::new(ResourceType::Iron, 30),
            ],

            // Production
            BuildingType::Workshop => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 60),
            ],
            BuildingType::Forge => vec![
                Resource::new(ResourceType::Wood, 70),
                Resource::new(ResourceType::Stone, 90),
                Resource::new(ResourceType::Iron, 30),
            ],
            BuildingType::Smithy => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 150),
                Resource::new(ResourceType::Iron, 50),
            ],
            BuildingType::Bakery => vec![
                Resource::new(ResourceType::Wood, 60),
                Resource::new(ResourceType::Stone, 80),
            ],
            BuildingType::WeaverHut => vec![
                Resource::new(ResourceType::Wood, 70),
                Resource::new(ResourceType::Stone, 40),
            ],
            BuildingType::PotteryKiln => vec![
                Resource::new(ResourceType::Wood, 50),
                Resource::new(ResourceType::Stone, 100),
            ],
            BuildingType::Tannery => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 60),
            ],

            // Resource
            BuildingType::Storehouse => vec![
                Resource::new(ResourceType::Wood, 150),
                Resource::new(ResourceType::Stone, 100),
            ],
            BuildingType::Farm => vec![
                Resource::new(ResourceType::Wood, 60),
                Resource::new(ResourceType::Stone, 40),
            ],
            BuildingType::AnimalPen => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 30),
            ],

            // Religious
            BuildingType::Shrine => vec![
                Resource::new(ResourceType::Wood, 50),
                Resource::new(ResourceType::Stone, 70),
            ],
            BuildingType::Temple => vec![
                Resource::new(ResourceType::Wood, 150),
                Resource::new(ResourceType::Stone, 200),
                Resource::new(ResourceType::Iron, 40),
            ],

            // Medical
            BuildingType::MedicalBuilding => vec![
                Resource::new(ResourceType::Wood, 90),
                Resource::new(ResourceType::Stone, 70),
            ],
        }
    }

    /// Get construction time (in ticks)
    pub fn construction_time(&self) -> u32 {
        match self {
            // Housing
            BuildingType::Longhouse => 500,
            BuildingType::UpgradedLonghouse => 700,
            BuildingType::SmallHouse => 300,
            BuildingType::MediumHouse => 400,
            BuildingType::LargeHouse => 600,
            BuildingType::Manor => 800,

            // Civic
            BuildingType::TownCenter => 1000,
            BuildingType::TownStorage => 600,

            // Production
            BuildingType::Workshop => 350,
            BuildingType::Forge => 450,
            BuildingType::Smithy => 500,
            BuildingType::Bakery => 400,
            BuildingType::WeaverHut => 350,
            BuildingType::PotteryKiln => 400,
            BuildingType::Tannery => 450,

            // Resource
            BuildingType::Storehouse => 400,
            BuildingType::Farm => 300,
            BuildingType::AnimalPen => 350,

            // Religious
            BuildingType::Shrine => 300,
            BuildingType::Temple => 800,

            // Medical
            BuildingType::MedicalBuilding => 500,
        }
    }

    /// Get building capacity (for housing)
    pub fn capacity(&self) -> usize {
        match self {
            BuildingType::Longhouse => 10,
            BuildingType::UpgradedLonghouse => 15,
            BuildingType::SmallHouse => 2,
            BuildingType::MediumHouse => 4,
            BuildingType::LargeHouse => 6,
            BuildingType::Manor => 8,
            _ => 0,
        }
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            // Housing
            BuildingType::Longhouse => 'L',
            BuildingType::UpgradedLonghouse => 'Ł',
            BuildingType::SmallHouse => 'h',
            BuildingType::MediumHouse => 'H',
            BuildingType::LargeHouse => '#',
            BuildingType::Manor => 'M',

            // Civic
            BuildingType::TownCenter => 'C',
            BuildingType::TownStorage => 'T',

            // Production
            BuildingType::Workshop => 'W',
            BuildingType::Forge => 'f',
            BuildingType::Smithy => 'S',
            BuildingType::Bakery => 'B',
            BuildingType::WeaverHut => 'w',
            BuildingType::PotteryKiln => 'K',
            BuildingType::Tannery => 't',

            // Resource
            BuildingType::Storehouse => 's',
            BuildingType::Farm => 'F',
            BuildingType::AnimalPen => 'A',

            // Religious
            BuildingType::Shrine => '†',
            BuildingType::Temple => '‡',

            // Medical
            BuildingType::MedicalBuilding => '+',
        }
    }

    /// Get color code for terminal rendering
    pub fn color_code(&self) -> &'static str {
        match self {
            // Housing - Magenta
            BuildingType::Longhouse | BuildingType::UpgradedLonghouse |
            BuildingType::SmallHouse | BuildingType::MediumHouse |
            BuildingType::LargeHouse | BuildingType::Manor => "\x1b[35m",

            // Civic - Bright Blue
            BuildingType::TownCenter | BuildingType::TownStorage => "\x1b[94m",

            // Production - Cyan
            BuildingType::Workshop | BuildingType::Forge | BuildingType::Smithy |
            BuildingType::Bakery | BuildingType::WeaverHut | BuildingType::PotteryKiln |
            BuildingType::Tannery => "\x1b[36m",

            // Resource - Yellow/Green
            BuildingType::Storehouse => "\x1b[33m",     // Yellow
            BuildingType::Farm | BuildingType::AnimalPen => "\x1b[32m", // Green

            // Religious - Bright Yellow
            BuildingType::Shrine | BuildingType::Temple => "\x1b[93m",

            // Medical - White
            BuildingType::MedicalBuilding => "\x1b[97m",
        }
    }

    /// Check if this is a production building
    pub fn is_production(&self) -> bool {
        matches!(
            self,
            BuildingType::Workshop | BuildingType::Forge | BuildingType::Smithy |
            BuildingType::Bakery | BuildingType::WeaverHut | BuildingType::PotteryKiln |
            BuildingType::Tannery
        )
    }

    /// Check if this is a resource building
    pub fn is_resource(&self) -> bool {
        matches!(
            self,
            BuildingType::Farm | BuildingType::AnimalPen | BuildingType::Storehouse |
            BuildingType::TownStorage
        )
    }

    /// Check if this is a religious building
    pub fn is_religious(&self) -> bool {
        matches!(self, BuildingType::Shrine | BuildingType::Temple)
    }

    /// Check if this is a civic building
    pub fn is_civic(&self) -> bool {
        matches!(self, BuildingType::TownCenter | BuildingType::TownStorage)
    }

    /// Get the description of what this building enables
    pub fn functionality_description(&self) -> &'static str {
        match self {
            // Housing
            BuildingType::Longhouse => "Basic shared housing for up to 10 agents. Provides shelter and increases well-being.",
            BuildingType::UpgradedLonghouse => "Improved shared housing for up to 15 agents. Better comfort and amenities.",
            BuildingType::SmallHouse => "Personal dwelling for 1-2 agents. Provides privacy and personal space.",
            BuildingType::MediumHouse => "Family home for up to 4 agents. Comfortable living space.",
            BuildingType::LargeHouse => "Spacious home for up to 6 agents. High-quality amenities.",
            BuildingType::Manor => "Luxury estate for up to 8 agents. Premium living conditions.",

            // Civic
            BuildingType::TownCenter => "Administrative hub. Enables advanced planning, coordination, and governance.",
            BuildingType::TownStorage => "Large-scale resource storage. Significantly increases community resource capacity.",

            // Production
            BuildingType::Workshop => "Basic crafting facility. Enables tool creation and basic item production.",
            BuildingType::Forge => "Metalworking facility. Enables basic metal tool and weapon production.",
            BuildingType::Smithy => "Advanced metalworking. Enables complex metal items, armor, and high-quality tools.",
            BuildingType::Bakery => "Food processing facility. Converts raw food into preserved and higher-value food items.",
            BuildingType::WeaverHut => "Textile production. Enables cloth, clothing, and textile goods creation.",
            BuildingType::PotteryKiln => "Pottery production. Enables ceramic containers, storage vessels, and pottery goods.",
            BuildingType::Tannery => "Leather working facility. Processes hides into leather goods and armor.",

            // Resource
            BuildingType::Storehouse => "Basic resource storage. Increases community resource capacity.",
            BuildingType::Farm => "Food production. Generates sustainable food supply for the population.",
            BuildingType::AnimalPen => "Animal husbandry. Provides food, leather, and other animal products.",

            // Religious
            BuildingType::Shrine => "Small religious site. Provides spiritual fulfillment and community gathering space.",
            BuildingType::Temple => "Major religious structure. Significantly boosts spiritual well-being and enables ceremonies.",

            // Medical
            BuildingType::MedicalBuilding => "Healthcare facility. Enables healing, disease treatment, and health recovery.",
        }
    }

    /// Get prerequisite buildings (buildings that should exist before this can be built)
    pub fn prerequisites(&self) -> Vec<BuildingType> {
        match self {
            // Basic buildings have no prerequisites
            BuildingType::Longhouse | BuildingType::SmallHouse | BuildingType::Workshop |
            BuildingType::Storehouse | BuildingType::Farm | BuildingType::Shrine => vec![],

            // Upgraded buildings require base versions or similar
            BuildingType::UpgradedLonghouse => vec![BuildingType::Longhouse],
            BuildingType::MediumHouse => vec![BuildingType::SmallHouse],
            BuildingType::LargeHouse => vec![BuildingType::MediumHouse],
            BuildingType::Manor => vec![BuildingType::LargeHouse],

            // Civic buildings require established settlement
            BuildingType::TownCenter => vec![BuildingType::Longhouse, BuildingType::Storehouse],
            BuildingType::TownStorage => vec![BuildingType::Storehouse],

            // Advanced production requires basic workshop
            BuildingType::Forge => vec![BuildingType::Workshop],
            BuildingType::Smithy => vec![BuildingType::Forge],
            BuildingType::Bakery => vec![BuildingType::Farm],
            BuildingType::PotteryKiln => vec![BuildingType::Workshop],
            BuildingType::Tannery => vec![BuildingType::Workshop],
            BuildingType::WeaverHut => vec![BuildingType::Workshop],

            // Resource buildings
            BuildingType::AnimalPen => vec![BuildingType::Farm],

            // Advanced religious requires basic
            BuildingType::Temple => vec![BuildingType::Shrine],

            // Medical
            BuildingType::MedicalBuilding => vec![BuildingType::Workshop],
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
            BuildingType::Longhouse | BuildingType::UpgradedLonghouse |
            BuildingType::SmallHouse | BuildingType::MediumHouse |
            BuildingType::LargeHouse | BuildingType::Manor
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
