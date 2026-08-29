// src/world/buildings.rs
//! Buildings and construction system.

use serde::{Deserialize, Serialize};
use crate::world::{Position, Resource, ResourceType};

/// Types of buildings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingType {
    // Shelter progression
    /// Hides over a frame of poles. The first thing a people who have just
    /// arrived somewhere can actually put up.
    ///
    /// Every other shelter in this list needs stone, and the cheapest of them
    /// needs thirty of it. Founders start with none, no way to quarry any, and
    /// no skill to build with it - so the Construction drive spent an eighth of
    /// a settlement's whole life restating that it was short of wood and had
    /// no stone at all, and not one building was ever raised. A tent is what
    /// stands between a stone-age people and the weather.
    SkinTent,

    /// A hole in the ground with a roof of turf over it.
    ///
    /// What a people with neither timber nor skins can put up, which turns
    /// out to be most of them: a tent wants eight wood and four hides, hides
    /// come off animals and nothing else, and hunting was unreachable for the
    /// whole life of this project - so `shelters built` was nought in every
    /// arm ever measured. Three deadlocked things, and this is the way out
    /// that does not depend on any of them.
    ///
    /// It costs a morning's digging and nothing else, and it is worse than a
    /// tent in every way except that it can actually be built.
    Burrow,
    Longhouse,         // Basic: Shared housing (10 capacity)
    UpgradedLonghouse, // Improved longhouse (15 capacity)
    SmallHouse,        // Personal 1-2 person home
    MediumHouse,       // Family home (4 capacity)
    LargeHouse,        // Large multi-room home (6 capacity)
    Manor,             // Luxury estate (8 capacity)

    // Civic buildings
    TownCenter,        // Administrative center
    TownStorage,       // Large community storage
    GuardPost,         // Security and defense

    // Production buildings
    Workshop,          // Basic crafting tools
    Forge,             // Basic metalworking
    Smithy,            // Advanced metalworking
    Bakery,            // Food processing
    WeaverHut,         // Textile production
    PotteryKiln,       // Pottery production
    Tannery,           // Leather working
    Mill,              // Grain processing
    Butchery,          // Meat processing
    Brewery,           // Ale/beer production
    Dairy,             // Milk/cheese production
    Glassworks,        // Glass production
    Dyeworks,          // Dye production
    Ropewalk,          // Rope production
    Brickyard,         // Brick production
    PaperMill,         // Paper production
    TailorShop,        // Clothing production
    CobblerShop,       // Shoe production
    BarberShop,        // Grooming services
    Scriptorium,       // Writing/printing

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
            BuildingType::SkinTent => vec![
                Resource::new(ResourceType::Wood, 8),
                Resource::new(ResourceType::Hides, 4),
            ],
            // Earth, and a morning. There is nothing to fetch and nothing to
            // be short of, which is the entire point of it.
            BuildingType::Burrow => vec![],
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
            BuildingType::Mill => vec![
                Resource::new(ResourceType::Wood, 90),
                Resource::new(ResourceType::Stone, 120),
            ],
            BuildingType::Butchery => vec![
                Resource::new(ResourceType::Wood, 70),
                Resource::new(ResourceType::Stone, 50),
            ],
            BuildingType::Brewery => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 80),
                Resource::new(ResourceType::Iron, 20),
            ],
            BuildingType::Dairy => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 60),
            ],
            BuildingType::Glassworks => vec![
                Resource::new(ResourceType::Wood, 60),
                Resource::new(ResourceType::Stone, 140),
                Resource::new(ResourceType::Iron, 30),
            ],
            BuildingType::Dyeworks => vec![
                Resource::new(ResourceType::Wood, 70),
                Resource::new(ResourceType::Stone, 50),
            ],
            BuildingType::Ropewalk => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 40),
            ],
            BuildingType::Brickyard => vec![
                Resource::new(ResourceType::Wood, 80),
                Resource::new(ResourceType::Stone, 100),
            ],
            BuildingType::PaperMill => vec![
                Resource::new(ResourceType::Wood, 120),
                Resource::new(ResourceType::Stone, 80),
            ],
            BuildingType::TailorShop => vec![
                Resource::new(ResourceType::Wood, 70),
                Resource::new(ResourceType::Stone, 50),
            ],
            BuildingType::CobblerShop => vec![
                Resource::new(ResourceType::Wood, 60),
                Resource::new(ResourceType::Stone, 50),
            ],
            BuildingType::BarberShop => vec![
                Resource::new(ResourceType::Wood, 50),
                Resource::new(ResourceType::Stone, 40),
            ],
            BuildingType::Scriptorium => vec![
                Resource::new(ResourceType::Wood, 100),
                Resource::new(ResourceType::Stone, 70),
            ],

            // Civic - GuardPost
            BuildingType::GuardPost => vec![
                Resource::new(ResourceType::Wood, 120),
                Resource::new(ResourceType::Stone, 150),
                Resource::new(ResourceType::Iron, 40),
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
            BuildingType::SkinTent => 40,
            // Longer than a tent: a tent is put up, and a burrow is dug.
            BuildingType::Burrow => 90,
            BuildingType::Longhouse => 500,
            BuildingType::UpgradedLonghouse => 700,
            BuildingType::SmallHouse => 300,
            BuildingType::MediumHouse => 400,
            BuildingType::LargeHouse => 600,
            BuildingType::Manor => 800,

            // Civic
            BuildingType::TownCenter => 1000,
            BuildingType::TownStorage => 600,
            BuildingType::GuardPost => 550,

            // Production
            BuildingType::Workshop => 350,
            BuildingType::Forge => 450,
            BuildingType::Smithy => 500,
            BuildingType::Bakery => 400,
            BuildingType::WeaverHut => 350,
            BuildingType::PotteryKiln => 400,
            BuildingType::Tannery => 450,
            BuildingType::Mill => 500,
            BuildingType::Butchery => 350,
            BuildingType::Brewery => 450,
            BuildingType::Dairy => 400,
            BuildingType::Glassworks => 550,
            BuildingType::Dyeworks => 380,
            BuildingType::Ropewalk => 420,
            BuildingType::Brickyard => 450,
            BuildingType::PaperMill => 500,
            BuildingType::TailorShop => 380,
            BuildingType::CobblerShop => 360,
            BuildingType::BarberShop => 320,
            BuildingType::Scriptorium => 480,

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
            BuildingType::SkinTent => 2,
            BuildingType::Longhouse => 10,
            BuildingType::UpgradedLonghouse => 15,
            BuildingType::SmallHouse => 2,
            BuildingType::MediumHouse => 4,
            BuildingType::LargeHouse => 6,
            BuildingType::Manor => 8,
            _ => 0,
        }
    }

    /// Check if this building type is residential (can house agents)
    pub fn is_residential(&self) -> bool {
        matches!(
            self,
            BuildingType::Longhouse
                | BuildingType::UpgradedLonghouse
                | BuildingType::SmallHouse
                | BuildingType::MediumHouse
                | BuildingType::LargeHouse
                | BuildingType::Manor
        )
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self {
            // Housing
            BuildingType::SkinTent => 't',
            BuildingType::Burrow => 'o',
            BuildingType::Longhouse => 'L',
            BuildingType::UpgradedLonghouse => 'Ł',
            BuildingType::SmallHouse => 'h',
            BuildingType::MediumHouse => 'H',
            BuildingType::LargeHouse => '#',
            BuildingType::Manor => 'M',

            // Civic
            BuildingType::TownCenter => 'C',
            BuildingType::TownStorage => 'T',
            BuildingType::GuardPost => 'G',

            // Production
            BuildingType::Workshop => 'W',
            BuildingType::Forge => 'f',
            BuildingType::Smithy => 'S',
            BuildingType::Bakery => 'B',
            BuildingType::WeaverHut => 'w',
            BuildingType::PotteryKiln => 'K',
            BuildingType::Tannery => 't',
            BuildingType::Mill => 'm',
            BuildingType::Butchery => 'u',
            BuildingType::Brewery => 'b',
            BuildingType::Dairy => 'd',
            BuildingType::Glassworks => 'g',
            BuildingType::Dyeworks => 'y',
            BuildingType::Ropewalk => 'r',
            BuildingType::Brickyard => 'k',
            BuildingType::PaperMill => 'p',
            BuildingType::TailorShop => 'l',
            BuildingType::CobblerShop => 'c',
            BuildingType::BarberShop => 'a',
            BuildingType::Scriptorium => 'q',

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
            BuildingType::SkinTent | BuildingType::Burrow |
            BuildingType::Longhouse | BuildingType::UpgradedLonghouse |
            BuildingType::SmallHouse | BuildingType::MediumHouse |
            BuildingType::LargeHouse | BuildingType::Manor => "\x1b[35m",

            // Civic - Bright Blue
            BuildingType::TownCenter | BuildingType::TownStorage | BuildingType::GuardPost => "\x1b[94m",

            // Production - Cyan
            BuildingType::Workshop | BuildingType::Forge | BuildingType::Smithy |
            BuildingType::Bakery | BuildingType::WeaverHut | BuildingType::PotteryKiln |
            BuildingType::Tannery | BuildingType::Mill | BuildingType::Butchery |
            BuildingType::Brewery | BuildingType::Dairy | BuildingType::Glassworks |
            BuildingType::Dyeworks | BuildingType::Ropewalk | BuildingType::Brickyard |
            BuildingType::PaperMill | BuildingType::TailorShop | BuildingType::CobblerShop |
            BuildingType::BarberShop | BuildingType::Scriptorium => "\x1b[36m",

            // Resource - Yellow/Green
            BuildingType::Storehouse => "\x1b[33m",     // Yellow
            BuildingType::Farm | BuildingType::AnimalPen => "\x1b[32m", // Green

            // Religious - Bright Yellow
            BuildingType::Shrine | BuildingType::Temple => "\x1b[93m",

            // Medical - White
            BuildingType::MedicalBuilding => "\x1b[97m",
        }
    }


    /// Check if this is a resource building
    pub fn is_resource(&self) -> bool {
        matches!(
            self,
            BuildingType::Farm | BuildingType::AnimalPen | BuildingType::Storehouse |
            BuildingType::TownStorage
        )
    }

    /// Get the upgrade path for this building (if any)
    pub fn can_upgrade_to(&self) -> Option<BuildingType> {
        match self {
            BuildingType::SkinTent => Some(BuildingType::Longhouse),
            BuildingType::Longhouse => Some(BuildingType::UpgradedLonghouse),
            BuildingType::SmallHouse => Some(BuildingType::MediumHouse),
            BuildingType::MediumHouse => Some(BuildingType::LargeHouse),
            BuildingType::LargeHouse => Some(BuildingType::Manor),
            BuildingType::Workshop => Some(BuildingType::Smithy),
            BuildingType::Forge => Some(BuildingType::Smithy),
            BuildingType::Shrine => Some(BuildingType::Temple),
            _ => None,
        }
    }

    /// Get upgrade cost (additional resources needed beyond base building)
    pub fn upgrade_cost(&self) -> Vec<Resource> {
        if let Some(upgraded) = self.can_upgrade_to() {
            let base_cost = self.requirements();
            let upgrade_cost = upgraded.requirements();

            // Calculate difference
            let mut additional = Vec::new();
            for upgrade_req in upgrade_cost {
                let base_amount = base_cost
                    .iter()
                    .find(|r| r.resource_type == upgrade_req.resource_type)
                    .map(|r| r.amount)
                    .unwrap_or(0);

                let additional_amount = upgrade_req.amount.saturating_sub(base_amount);
                if additional_amount > 0 {
                    additional.push(Resource::new(upgrade_req.resource_type, additional_amount));
                }
            }
            additional
        } else {
            Vec::new()
        }
    }

    /// Get minimum construction skill recommended for this building
    /// Returns (min_skill, recommended_skill)
    pub fn skill_requirements(&self) -> (i32, i32) {
        match self {
            // Simple buildings - anyone can build
            BuildingType::SmallHouse | BuildingType::Farm | BuildingType::AnimalPen => (0, 2),

            // Basic buildings - some skill helpful
            BuildingType::Longhouse | BuildingType::Storehouse | BuildingType::Workshop => (1, 3),

            // Intermediate buildings - skill important
            BuildingType::MediumHouse | BuildingType::Bakery | BuildingType::WeaverHut |
            BuildingType::PotteryKiln | BuildingType::Tannery | BuildingType::Mill |
            BuildingType::Butchery => (2, 4),

            // Advanced buildings - skilled workers needed
            BuildingType::LargeHouse | BuildingType::UpgradedLonghouse | BuildingType::Forge |
            BuildingType::Brewery | BuildingType::Dairy | BuildingType::Glassworks |
            BuildingType::Dyeworks | BuildingType::Ropewalk | BuildingType::Brickyard => (3, 5),

            // Complex buildings - expert builders required
            BuildingType::Manor | BuildingType::Smithy | BuildingType::TownCenter |
            BuildingType::TownStorage | BuildingType::GuardPost | BuildingType::PaperMill => (4, 6),

            // Master buildings - only skilled masters
            BuildingType::Temple | BuildingType::MedicalBuilding => (5, 8),

            // Specialty buildings
            _ => (2, 4),
        }
    }

    /// Check if this is a religious building
    pub fn is_religious(&self) -> bool {
        matches!(self, BuildingType::Shrine | BuildingType::Temple)
    }


    /// Get the description of what this building enables
    pub fn functionality_description(&self) -> &'static str {
        match self {
            // Housing
            BuildingType::SkinTent => "Hides stretched over poles. Sleeps two, keeps the weather off, and can be put up by people who have only what they can carry.",
            BuildingType::Burrow => "A hole in the ground with turf over it. Cold, dark, damp and cramped, and it can be dug by people who have nothing at all.",
            BuildingType::Longhouse => "Basic shared housing for up to 10 agents. Provides shelter and increases well-being.",
            BuildingType::UpgradedLonghouse => "Improved shared housing for up to 15 agents. Better comfort and amenities.",
            BuildingType::SmallHouse => "Personal dwelling for 1-2 agents. Provides privacy and personal space.",
            BuildingType::MediumHouse => "Family home for up to 4 agents. Comfortable living space.",
            BuildingType::LargeHouse => "Spacious home for up to 6 agents. High-quality amenities.",
            BuildingType::Manor => "Luxury estate for up to 8 agents. Premium living conditions.",

            // Civic
            BuildingType::TownCenter => "Administrative hub. Enables advanced planning, coordination, and governance.",
            BuildingType::TownStorage => "Large-scale resource storage. Significantly increases community resource capacity.",
            BuildingType::GuardPost => "Security station. Provides defense, maintains order, and protects the settlement.",

            // Production
            BuildingType::Workshop => "Basic crafting facility. Enables tool creation and basic item production.",
            BuildingType::Forge => "Metalworking facility. Enables basic metal tool and weapon production.",
            BuildingType::Smithy => "Advanced metalworking. Enables complex metal items, armor, and high-quality tools.",
            BuildingType::Bakery => "Food processing facility. Converts raw food into preserved and higher-value food items.",
            BuildingType::WeaverHut => "Textile production. Enables cloth, clothing, and textile goods creation.",
            BuildingType::PotteryKiln => "Pottery production. Enables ceramic containers, storage vessels, and pottery goods.",
            BuildingType::Tannery => "Leather working facility. Processes hides into leather goods and armor.",
            BuildingType::Mill => "Grain processing. Grinds grain into flour for bread and food production.",
            BuildingType::Butchery => "Meat processing. Prepares meat and animal products for consumption and trade.",
            BuildingType::Brewery => "Beverage production. Brews ale, beer, and other fermented drinks.",
            BuildingType::Dairy => "Milk processing. Produces cheese, butter, and other dairy products.",
            BuildingType::Glassworks => "Glass production. Creates glass, bottles, and decorative glass items.",
            BuildingType::Dyeworks => "Dye production. Processes herbs and materials into dyes for coloring.",
            BuildingType::Ropewalk => "Rope production. Creates rope, cordage, and fiber products.",
            BuildingType::Brickyard => "Brick production. Manufactures bricks for construction.",
            BuildingType::PaperMill => "Paper production. Creates paper, parchment, and writing materials.",
            BuildingType::TailorShop => "Clothing production. Creates garments, clothes, and textile goods.",
            BuildingType::CobblerShop => "Footwear production. Creates shoes, boots, and leather footwear.",
            BuildingType::BarberShop => "Grooming services. Provides haircuts, grooming, and basic medical care.",
            BuildingType::Scriptorium => "Writing and printing. Creates books, documents, and printed materials.",

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
            BuildingType::SkinTent | BuildingType::Burrow |
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
            BuildingType::GuardPost => vec![BuildingType::TownCenter],

            // Advanced production requires basic workshop
            BuildingType::Forge => vec![BuildingType::Workshop],
            BuildingType::Smithy => vec![BuildingType::Forge],
            BuildingType::Bakery => vec![BuildingType::Farm],
            BuildingType::PotteryKiln => vec![BuildingType::Workshop],
            BuildingType::Tannery => vec![BuildingType::Workshop],
            BuildingType::WeaverHut => vec![BuildingType::Workshop],
            BuildingType::Mill => vec![BuildingType::Farm],
            BuildingType::Butchery => vec![BuildingType::AnimalPen],
            BuildingType::Brewery => vec![BuildingType::Farm],
            BuildingType::Dairy => vec![BuildingType::AnimalPen],
            BuildingType::Glassworks => vec![BuildingType::Forge],
            BuildingType::Dyeworks => vec![BuildingType::Farm],
            BuildingType::Ropewalk => vec![BuildingType::Farm],
            BuildingType::Brickyard => vec![BuildingType::Workshop],
            BuildingType::PaperMill => vec![BuildingType::Workshop],
            BuildingType::TailorShop => vec![BuildingType::WeaverHut],
            BuildingType::CobblerShop => vec![BuildingType::Tannery],
            BuildingType::BarberShop => vec![BuildingType::Workshop],
            BuildingType::Scriptorium => vec![BuildingType::PaperMill],

            // Resource buildings
            BuildingType::AnimalPen => vec![BuildingType::Farm],

            // Advanced religious requires basic
            BuildingType::Temple => vec![BuildingType::Shrine],

            // Medical
            BuildingType::MedicalBuilding => vec![BuildingType::Workshop],
        }
    }

    /// Get the production interval in ticks (0 means no production)
    pub fn production_interval(&self) -> u32 {
        match self {
            // Production buildings produce resources
            BuildingType::Farm => 100,
            BuildingType::AnimalPen => 150,
            BuildingType::Mill => 80,
            BuildingType::Bakery => 60,
            BuildingType::Butchery => 100,
            BuildingType::Brewery => 120,
            BuildingType::Dairy => 100,
            BuildingType::WeaverHut => 80,
            BuildingType::PotteryKiln => 100,
            BuildingType::Tannery => 120,
            BuildingType::Forge => 150,
            BuildingType::Smithy => 200,
            BuildingType::Glassworks => 150,
            BuildingType::Dyeworks => 100,
            BuildingType::Ropewalk => 80,
            BuildingType::Brickyard => 120,
            BuildingType::PaperMill => 100,
            BuildingType::TailorShop => 80,
            BuildingType::CobblerShop => 100,
            BuildingType::Scriptorium => 150,
            // Non-production buildings
            _ => 0,
        }
    }

    /// Get resources produced per production cycle
    pub fn production_output(&self) -> Vec<Resource> {
        match self {
            BuildingType::Farm => vec![Resource::new(ResourceType::Food, 10)],
            BuildingType::AnimalPen => vec![Resource::new(ResourceType::Food, 5)],
            BuildingType::Mill => vec![Resource::new(ResourceType::Food, 3)],
            BuildingType::Bakery => vec![Resource::new(ResourceType::Food, 5)],
            BuildingType::Butchery => vec![Resource::new(ResourceType::Food, 8)],
            BuildingType::Brewery => vec![Resource::new(ResourceType::Food, 3)],
            BuildingType::Dairy => vec![Resource::new(ResourceType::Food, 4)],
            BuildingType::WeaverHut => vec![Resource::new(ResourceType::Wood, 2)], // Represents cloth
            BuildingType::PotteryKiln => vec![Resource::new(ResourceType::Stone, 3)], // Represents pottery
            BuildingType::Tannery => vec![Resource::new(ResourceType::Food, 1)], // Represents leather
            BuildingType::Forge => vec![Resource::new(ResourceType::Iron, 2)],
            BuildingType::Smithy => vec![Resource::new(ResourceType::Iron, 3)],
            BuildingType::Glassworks => vec![Resource::new(ResourceType::Stone, 2)], // Represents glass
            BuildingType::Dyeworks => vec![Resource::new(ResourceType::Food, 1)], // Represents dyes
            BuildingType::Ropewalk => vec![Resource::new(ResourceType::Wood, 2)], // Represents rope
            BuildingType::Brickyard => vec![Resource::new(ResourceType::Stone, 5)],
            BuildingType::PaperMill => vec![Resource::new(ResourceType::Wood, 2)], // Represents paper
            BuildingType::TailorShop => vec![Resource::new(ResourceType::Wood, 1)], // Represents clothing
            BuildingType::CobblerShop => vec![Resource::new(ResourceType::Wood, 1)], // Represents shoes
            BuildingType::Scriptorium => vec![Resource::new(ResourceType::Wood, 1)], // Represents books
            _ => Vec::new(),
        }
    }

    /// Get the decay rate per tick (condition lost per tick without maintenance)
    pub fn decay_rate(&self) -> f32 {
        match self {
            // Wooden structures decay faster
            BuildingType::Longhouse | BuildingType::UpgradedLonghouse => 0.0002,
            BuildingType::SmallHouse | BuildingType::MediumHouse => 0.00015,
            BuildingType::LargeHouse | BuildingType::Manor => 0.0001,
            // Stone/civic buildings are more durable
            BuildingType::TownCenter | BuildingType::TownStorage => 0.00005,
            BuildingType::Temple | BuildingType::Shrine => 0.00008,
            // Production buildings need regular maintenance
            BuildingType::Forge | BuildingType::Smithy => 0.0003,
            BuildingType::Farm | BuildingType::AnimalPen => 0.00025,
            // Default decay rate
            _ => 0.0001,
        }
    }


    /// Get the defense bonus this building provides to nearby agents
    /// Returns a multiplier (1.0 = no bonus, 1.2 = 20% defense bonus)
    pub fn defense_bonus(&self) -> f32 {
        match self {
            BuildingType::GuardPost => 1.25, // 25% defense bonus
            BuildingType::TownCenter => 1.1, // 10% defense bonus (administrative coordination)
            _ => 1.0,
        }
    }

    /// Get the effective defense radius of this building (in tiles)
    pub fn defense_radius(&self) -> f32 {
        match self {
            BuildingType::GuardPost => 15.0,
            BuildingType::TownCenter => 20.0,
            _ => 0.0,
        }
    }


    /// Get the healing rate bonus this building provides
    /// Returns a multiplier (1.0 = normal healing, 2.0 = double healing)
    pub fn healing_bonus(&self) -> f32 {
        match self {
            BuildingType::MedicalBuilding => 2.0, // Double healing rate
            BuildingType::BarberShop => 1.3, // 30% healing bonus (basic care)
            _ => 1.0,
        }
    }

    /// Get the effective healing radius of this building (in tiles)
    pub fn healing_radius(&self) -> f32 {
        match self {
            BuildingType::MedicalBuilding => 10.0,
            BuildingType::BarberShop => 5.0,
            _ => 0.0,
        }
    }


    /// Get the morale/happiness bonus for being near this building
    /// Returns happiness amount added per tick when nearby
    pub fn morale_bonus(&self) -> f32 {
        match self {
            // Religious buildings provide passive morale boost
            BuildingType::Temple => 0.02,
            BuildingType::Shrine => 0.01,
            // Civic buildings provide order and security feeling
            BuildingType::TownCenter => 0.015,
            BuildingType::GuardPost => 0.01, // Security feeling
            // Service buildings provide comfort
            BuildingType::BarberShop => 0.008,
            BuildingType::MedicalBuilding => 0.005, // Being near healthcare is reassuring
            // Quality housing provides comfort
            BuildingType::Manor => 0.02,
            BuildingType::LargeHouse => 0.015,
            BuildingType::MediumHouse => 0.01,
            _ => 0.0,
        }
    }

    /// Get the morale bonus radius (in tiles)
    pub fn morale_radius(&self) -> f32 {
        match self {
            BuildingType::Temple => 20.0,
            BuildingType::Shrine => 12.0,
            BuildingType::TownCenter => 25.0,
            BuildingType::GuardPost => 15.0,
            BuildingType::BarberShop => 8.0,
            BuildingType::MedicalBuilding => 10.0,
            BuildingType::Manor | BuildingType::LargeHouse | BuildingType::MediumHouse => 5.0,
            _ => 0.0,
        }
    }

}

/// Building construction state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildingState {
    UnderConstruction {
        progress: u32, // Work progress in ticks
        resources_delivered: Vec<Resource>, // Resources already delivered
        workers: Vec<uuid::Uuid>, // Agents currently working on this building
    },
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
    pub condition: f32, // Building condition 0.0-1.0, decays over time without maintenance
    pub production_timer: u32, // Ticks until next production cycle
    pub pending_production: Vec<Resource>, // Resources produced but not yet collected
}

impl Building {
    pub fn new(building_type: BuildingType, position: Position) -> Self {
        Self {
            building_type,
            position,
            state: BuildingState::Completed, // Start completed for initial buildings
            owner: None,
            occupants: Vec::new(),
            condition: 1.0, // New buildings start in perfect condition
            production_timer: building_type.production_interval(),
            pending_production: Vec::new(),
        }
    }

    pub fn new_under_construction(building_type: BuildingType, position: Position) -> Self {
        Self {
            building_type,
            position,
            state: BuildingState::UnderConstruction {
                progress: 0,
                resources_delivered: Vec::new(),
                workers: Vec::new(),
            },
            owner: None,
            occupants: Vec::new(),
            condition: 1.0,
            production_timer: 0, // Timer starts when building is completed
            pending_production: Vec::new(),
        }
    }

    /// Deliver resources to construction site
    /// Returns true if resource was accepted (needed), false if not needed
    pub fn deliver_resource(&mut self, resource: Resource) -> bool {
        if let BuildingState::UnderConstruction { resources_delivered, .. } = &mut self.state {
            let requirements = self.building_type.requirements();

            // Find which resource this is
            if let Some(req) = requirements.iter().find(|r| r.resource_type == resource.resource_type) {
                // Check how much we've already delivered
                let already_delivered = resources_delivered
                    .iter()
                    .filter(|r| r.resource_type == resource.resource_type)
                    .map(|r| r.amount)
                    .sum::<u32>();

                // Accept only what's needed
                let needed = req.amount.saturating_sub(already_delivered);
                if needed > 0 {
                    let amount_to_accept = resource.amount.min(needed);
                    resources_delivered.push(Resource::new(resource.resource_type, amount_to_accept));
                    return true;
                }
            }
        }
        false
    }

    /// Check if all required resources have been delivered
    pub fn has_all_resources(&self) -> bool {
        if let BuildingState::UnderConstruction { resources_delivered, .. } = &self.state {
            let requirements = self.building_type.requirements();

            for req in requirements {
                let delivered = resources_delivered
                    .iter()
                    .filter(|r| r.resource_type == req.resource_type)
                    .map(|r| r.amount)
                    .sum::<u32>();

                if delivered < req.amount {
                    return false;
                }
            }
            true
        } else {
            true // Completed buildings have all resources
        }
    }

    /// Get missing resources
    pub fn missing_resources(&self) -> Vec<Resource> {
        if let BuildingState::UnderConstruction { resources_delivered, .. } = &self.state {
            let mut missing = Vec::new();
            let requirements = self.building_type.requirements();

            for req in requirements {
                let delivered = resources_delivered
                    .iter()
                    .filter(|r| r.resource_type == req.resource_type)
                    .map(|r| r.amount)
                    .sum::<u32>();

                let remaining = req.amount.saturating_sub(delivered);
                if remaining > 0 {
                    missing.push(Resource::new(req.resource_type, remaining));
                }
            }
            missing
        } else {
            Vec::new()
        }
    }




    /// Advance construction progress (worker performs work)
    /// Returns true if construction completed
    ///
    /// # Arguments
    /// * `work_amount` - Amount of work done (in ticks), modified by worker skill
    /// * `worker_skill` - Construction skill level (0-10+, affects speed)
    pub fn add_construction_progress(&mut self, work_amount: u32, worker_skill: i32) -> bool {
        // Can only work if resources are available
        if !self.has_all_resources() {
            return false;
        }

        if let BuildingState::UnderConstruction { progress, .. } = &mut self.state {
            // Skill multiplier: skill 0 = 1.0x, skill 5 = 1.5x, skill 10 = 2.0x
            let skill_multiplier = 1.0 + (worker_skill as f32 * 0.1);
            let effective_work = (work_amount as f32 * skill_multiplier) as u32;

            *progress += effective_work;
            let required = self.building_type.construction_time();

            if *progress >= required {
                self.state = BuildingState::Completed;
                return true; // Construction completed
            }
        }
        false
    }

    /// Get construction progress (0.0 to 1.0)
    pub fn construction_progress(&self) -> f32 {
        if let BuildingState::UnderConstruction { progress, .. } = &self.state {
            let required = self.building_type.construction_time() as f32;
            (*progress as f32 / required).min(1.0)
        } else {
            1.0
        }
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

    /// Check if this building can be upgraded
    pub fn can_upgrade(&self) -> bool {
        self.is_completed() && self.building_type.can_upgrade_to().is_some()
    }



    /// Process building tick: decay and production
    pub fn tick(&mut self) {
        // Only completed buildings decay and produce
        if !self.is_completed() {
            return;
        }

        // Apply decay based on building type
        let decay_rate = self.building_type.decay_rate();
        self.condition = (self.condition - decay_rate).max(0.0);

        // Production only works if building is in reasonable condition (>25%)
        if self.condition < 0.25 {
            return;
        }

        // Handle production timer
        let production_interval = self.building_type.production_interval();
        if production_interval > 0 {
            if self.production_timer > 0 {
                self.production_timer -= 1;
            } else {
                // Production cycle complete - generate resources
                let output = self.building_type.production_output();

                // Production efficiency based on condition
                let efficiency = self.condition;
                for mut resource in output {
                    // Scale production by building condition
                    resource.amount = (resource.amount as f32 * efficiency).ceil() as u32;
                    if resource.amount > 0 {
                        self.pending_production.push(resource);
                    }
                }

                // Reset timer for next production cycle
                self.production_timer = production_interval;
            }
        }
    }

    /// Collect all pending production from this building
    pub fn collect_production(&mut self) -> Vec<Resource> {
        std::mem::take(&mut self.pending_production)
    }

    /// Perform maintenance on the building (restore condition)
    pub fn maintain(&mut self, repair_amount: f32) {
        self.condition = (self.condition + repair_amount).min(1.0);
    }

    /// Check if building needs maintenance (condition below 50%)
    pub fn needs_maintenance(&self) -> bool {
        self.is_completed() && self.condition < 0.5
    }

    /// Check if building is in critical condition (below 25%)
    pub fn is_critical_condition(&self) -> bool {
        self.is_completed() && self.condition < 0.25
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

        // Deliver required resources (SmallHouse needs 50 Wood + 30 Stone)
        building.deliver_resource(Resource::new(ResourceType::Wood, 50));
        building.deliver_resource(Resource::new(ResourceType::Stone, 30));
        assert!(building.has_all_resources());

        // Add progress (work_amount, worker_skill)
        // Skill 5 gives 1.5x multiplier, so 100 work = 150 effective
        let completed = building.add_construction_progress(100, 5);
        assert!(!completed); // 150 < 300 required, not enough progress

        // Complete construction
        // 300 work * 1.5 = 450 effective, total = 150 + 450 = 600 >= 300
        let completed = building.add_construction_progress(300, 5);
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
