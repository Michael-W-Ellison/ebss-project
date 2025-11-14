// src/world/mod.rs
//! Complete world simulation system with terrain, resources, buildings, and spatial management.

use serde::{Deserialize, Serialize};

/// World size presets for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    /// Tiny world: 64x64x64 - for quick testing
    Tiny,
    /// Small world: 128x128x96 - for small simulations
    Small,
    /// Medium world: 256x256x128 - default balanced size
    Medium,
    /// Large world: 512x512x160 - for complex simulations
    Large,
    /// Huge world: 1024x1024x192 - for massive simulations
    Huge,
    /// Custom size
    Custom(i32, i32, i32),
}

impl WorldSize {
    /// Get the dimensions (width, depth, height) for this size
    pub fn dimensions(&self) -> (i32, i32, i32) {
        match self {
            WorldSize::Tiny => (64, 64, 64),
            WorldSize::Small => (128, 128, 96),
            WorldSize::Medium => (256, 256, 128),
            WorldSize::Large => (512, 512, 160),
            WorldSize::Huge => (1024, 1024, 192),
            WorldSize::Custom(w, d, h) => (*w, *d, *h),
        }
    }

    /// Validate that dimensions are reasonable
    pub fn is_valid(&self) -> bool {
        let (w, d, h) = self.dimensions();
        w > 0 && d > 0 && h > 0 && w <= 4096 && d <= 4096 && h <= 512
    }

    /// Get estimated memory usage in MB (rough estimate)
    pub fn estimated_memory_mb(&self) -> f32 {
        let (w, d, h) = self.dimensions();
        let blocks = w as f32 * d as f32 * h as f32;
        // Rough estimate: ~100 bytes per block on average (with sparse storage)
        blocks * 100.0 / (1024.0 * 1024.0)
    }
}

/// Configuration for the world grid system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub mod terrain;
pub mod resources;
pub mod buildings;
pub mod inventory;
pub mod actions;
pub mod grid;
pub mod render;
pub mod production;
pub mod economy;
pub mod technology;

pub use terrain::{Terrain, TerrainType, Tile};
pub use resources::{Resource, ResourceType, ResourceNode};
pub use buildings::{Building, BuildingType, BuildingState};
pub use inventory::{Inventory, Item, ItemType};
pub use actions::{Action, ActionResult};
pub use grid::{Grid, Position};
pub use render::AsciiRenderer;
pub use production::{Recipe, Quality, ResourceRequirement, ProductionOutput, get_job_recipes, get_primary_recipe};
pub use economy::{TradeOffer, Marketplace, MarketData, CompletedTrade, MarketStatistics};
pub use technology::{Technology, TechnologyTree, KnownTechnologies, TechEra, DiscoveryEvent};

use crate::agents::Population;
use serde::{Deserialize, Serialize};

/// Complete world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub grid: Grid,
    pub resources: Vec<ResourceNode>,
    pub buildings: Vec<Building>,
    pub storehouse_inventory: Inventory,
    pub marketplace: Marketplace,
    #[serde(skip)]
    pub tech_tree: TechnologyTree, // Global technology tree (not serialized, recreated)
    pub tick: u32,
}

/// World configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub size: (usize, usize), // Width, Height (no Z for simplicity)
    pub initial_resources: ResourceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub wood_nodes: usize,
    pub stone_nodes: usize,
    pub iron_nodes: usize,
    pub food_nodes: usize,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            size: (50, 50),
            initial_resources: ResourceConfig {
                wood_nodes: 20,
                stone_nodes: 15,
                iron_nodes: 8,
                food_nodes: 25,
            },
        }
    }
}

impl World {
    pub fn new(config: WorldConfig) -> Self {
        let mut grid = Grid::new(config.size.0, config.size.1);
        grid.generate_terrain();

        let mut world = Self {
            grid,
            resources: Vec::new(),
            buildings: Vec::new(),
            storehouse_inventory: Inventory::new(10000), // Large capacity
            marketplace: Marketplace::new(),
            tech_tree: TechnologyTree::new(),
            tick: 0,
        };

        // Place initial resources
        world.generate_resources(&config.initial_resources);

        // Build initial longhouse at center
        let center = (config.size.0 / 2, config.size.1 / 2);
        world.add_building(Building::new(
            BuildingType::Longhouse,
            Position::new(center.0 as i32, center.1 as i32),
        ));

        world
    }

    fn generate_resources(&mut self, config: &ResourceConfig) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Generate wood nodes (in forest areas)
        for _ in 0..config.wood_nodes {
            let pos = self.find_random_terrain_position(TerrainType::Forest);
            self.resources.push(ResourceNode::new(
                ResourceType::Wood,
                pos,
                rng.gen_range(50..150),
            ));
        }

        // Generate stone nodes (in mountain areas)
        for _ in 0..config.stone_nodes {
            let pos = self.find_random_terrain_position(TerrainType::Mountain);
            self.resources.push(ResourceNode::new(
                ResourceType::Stone,
                pos,
                rng.gen_range(80..200),
            ));
        }

        // Generate iron nodes (rare, in mountains)
        for _ in 0..config.iron_nodes {
            let pos = self.find_random_terrain_position(TerrainType::Mountain);
            self.resources.push(ResourceNode::new(
                ResourceType::Iron,
                pos,
                rng.gen_range(30..100),
            ));
        }

        // Generate food nodes (in plains)
        for _ in 0..config.food_nodes {
            let pos = self.find_random_terrain_position(TerrainType::Plains);
            self.resources.push(ResourceNode::new(
                ResourceType::Food,
                pos,
                rng.gen_range(20..60),
            ));
        }
    }

    fn find_random_terrain_position(&self, terrain_type: TerrainType) -> Position {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Try up to 100 times to find matching terrain
        for _ in 0..100 {
            let x = rng.gen_range(0..self.grid.width) as i32;
            let y = rng.gen_range(0..self.grid.height) as i32;
            let pos = Position::new(x, y);

            if let Some(tile) = self.grid.get_tile(&pos) {
                if tile.terrain.terrain_type == terrain_type {
                    // Check if position is not occupied
                    if !self.is_position_occupied(&pos) {
                        return pos;
                    }
                }
            }
        }

        // Fallback: return random position
        Position::new(
            rng.gen_range(0..self.grid.width) as i32,
            rng.gen_range(0..self.grid.height) as i32,
        )
    }

    pub fn is_position_occupied(&self, pos: &Position) -> bool {
        // Check buildings
        if self.buildings.iter().any(|b| &b.position == pos) {
            return true;
        }

        // Check resources
        if self.resources.iter().any(|r| &r.position == pos) {
            return true;
        }

        false
    }

    pub fn add_building(&mut self, building: Building) {
        self.buildings.push(building);
    }

    pub fn get_resource_at(&self, pos: &Position) -> Option<&ResourceNode> {
        self.resources.iter().find(|r| &r.position == pos)
    }

    pub fn get_resource_at_mut(&mut self, pos: &Position) -> Option<&mut ResourceNode> {
        self.resources.iter_mut().find(|r| &r.position == pos)
    }

    pub fn get_building_at(&self, pos: &Position) -> Option<&Building> {
        self.buildings.iter().find(|b| &b.position == pos)
    }

    pub fn remove_depleted_resources(&mut self) {
        self.resources.retain(|r| r.amount > 0);
    }

    pub fn tick(&mut self) {
        self.tick += 1;

        // Update buildings
        for building in &mut self.buildings {
            building.tick();
        }

        // Remove depleted resources
        self.remove_depleted_resources();
    }

    /// Get statistics about the world
    pub fn stats(&self) -> WorldStats {
        let mut stats = WorldStats::default();

        stats.total_resources = self.resources.len();
        stats.total_buildings = self.buildings.len();

        for resource in &self.resources {
            match resource.resource_type {
                ResourceType::Wood => stats.wood_available += resource.amount,
                ResourceType::Stone => stats.stone_available += resource.amount,
                ResourceType::Iron => stats.iron_available += resource.amount,
                ResourceType::Food => stats.food_available += resource.amount,
                // New resource types - not yet tracked in stats
                _ => {}
            }
        }

        // Count storehouse inventory
        stats.wood_stored = self.storehouse_inventory.count_item(&ItemType::Wood);
        stats.stone_stored = self.storehouse_inventory.count_item(&ItemType::Stone);
        stats.iron_stored = self.storehouse_inventory.count_item(&ItemType::Iron);
        stats.food_stored = self.storehouse_inventory.count_item(&ItemType::Food);

        // Count buildings by type
        for building in &self.buildings {
            match building.building_type {
                BuildingType::Longhouse => stats.longhouses += 1,
                BuildingType::UpgradedLonghouse => stats.longhouses += 1, // Count as longhouse
                BuildingType::SmallHouse => stats.small_houses += 1,
                BuildingType::MediumHouse => stats.medium_houses += 1,
                BuildingType::LargeHouse => stats.large_houses += 1,
                BuildingType::Manor => stats.large_houses += 1, // Count as large house
                BuildingType::Storehouse | BuildingType::TownStorage => stats.storehouses += 1,
                BuildingType::Workshop => stats.workshops += 1,
                BuildingType::Smithy | BuildingType::Forge => stats.smithies += 1,
                BuildingType::Farm => stats.farms += 1,
                // All other building types are tracked in total_buildings but not individually
                _ => {}
            }
        }

        stats
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStats {
    pub total_resources: usize,
    pub total_buildings: usize,
    pub wood_available: u32,
    pub stone_available: u32,
    pub iron_available: u32,
    pub food_available: u32,
    pub wood_stored: u32,
    pub stone_stored: u32,
    pub iron_stored: u32,
    pub food_stored: u32,
    pub longhouses: usize,
    pub small_houses: usize,
    pub medium_houses: usize,
    pub large_houses: usize,
    pub storehouses: usize,
    pub workshops: usize,
    pub smithies: usize,
    pub farms: usize,
}

// Legacy types for compatibility
pub struct GridConfig {
    /// World dimensions (width, depth, height)
    pub size: (i32, i32, i32),
    /// Size of chunks for spatial optimization
    pub chunk_size: u32,
}

impl GridConfig {
    /// Create a new grid configuration with custom size
    pub fn new(size: (i32, i32, i32)) -> Self {
        Self {
            size,
            chunk_size: 16,
        }
    }

    /// Create from a WorldSize preset
    pub fn from_world_size(world_size: WorldSize) -> Self {
        Self::new(world_size.dimensions())
    }

    /// Validate configuration
    pub fn is_valid(&self) -> bool {
        self.size.0 > 0
            && self.size.1 > 0
            && self.size.2 > 0
            && self.chunk_size > 0
            && self.chunk_size <= 64
    }

    /// Check if a position is within bounds
    pub fn is_in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        let (width, depth, height) = self.size;
        x >= -width/2 && x < width/2
            && z >= -depth/2 && z < depth/2
            && y >= 0 && y < height
    }

    /// Get total world volume
    pub fn volume(&self) -> i64 {
        self.size.0 as i64 * self.size.1 as i64 * self.size.2 as i64
    }
}

impl Default for GridConfig {
    fn default() -> Self {
        Self::from_world_size(WorldSize::Medium)
    }
}

/// World configuration including grid and generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// Grid configuration
    pub grid: GridConfig,
    /// World generation seed
    pub seed: u64,
    /// Difficulty level (0.0 to 1.0)
    pub difficulty: f32,
}

impl WorldConfig {
    /// Create a new world configuration
    pub fn new(seed: u64, world_size: WorldSize) -> Self {
        Self {
            grid: GridConfig::from_world_size(world_size),
            seed,
            difficulty: 0.5,
        }
    }

    /// Create with custom dimensions
    pub fn with_custom_size(seed: u64, width: i32, depth: i32, height: i32) -> Self {
        Self {
            grid: GridConfig::new((width, depth, height)),
            seed,
            difficulty: 0.5,
        }
    }
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self::new(0, WorldSize::Medium)
    }
}

/// 3D position in world space
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Chunk for spatial partitioning
pub struct Chunk {
    pub position: (i32, i32, i32),
}

/// World simulation state
pub struct World {
    config: GridConfig,
}

impl World {
    /// Create a new world with the given configuration
    pub fn new(config: GridConfig) -> Self {
        assert!(config.is_valid(), "Invalid grid configuration");
        Self { config }
    }

    /// Create a world from a preset size
    pub fn with_size(world_size: WorldSize) -> Self {
        Self::new(GridConfig::from_world_size(world_size))
    }

    /// Get the world configuration
    pub fn config(&self) -> &GridConfig {
        &self.config
    }

    /// Check if a position is valid in this world
    pub fn is_valid_position(&self, x: i32, y: i32, z: i32) -> bool {
        self.config.is_in_bounds(x, y, z)
    }

    /// Get world dimensions
    pub fn dimensions(&self) -> (i32, i32, i32) {
        self.config.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_size_presets() {
        assert_eq!(WorldSize::Tiny.dimensions(), (64, 64, 64));
        assert_eq!(WorldSize::Small.dimensions(), (128, 128, 96));
        assert_eq!(WorldSize::Medium.dimensions(), (256, 256, 128));
        assert_eq!(WorldSize::Large.dimensions(), (512, 512, 160));
        assert_eq!(WorldSize::Huge.dimensions(), (1024, 1024, 192));
    }

    #[test]
    fn test_custom_world_size() {
        let custom = WorldSize::Custom(100, 200, 300);
        assert_eq!(custom.dimensions(), (100, 200, 300));
    }

    #[test]
    fn test_world_size_validation() {
        assert!(WorldSize::Medium.is_valid());
        assert!(WorldSize::Custom(256, 256, 128).is_valid());
        assert!(!WorldSize::Custom(0, 256, 128).is_valid());
        assert!(!WorldSize::Custom(-10, 256, 128).is_valid());
        assert!(!WorldSize::Custom(10000, 256, 128).is_valid());
    }

    #[test]
    fn test_grid_config_creation() {
        let config = GridConfig::new((256, 256, 128));
        assert_eq!(config.size, (256, 256, 128));
        assert_eq!(config.chunk_size, 16);
    }

    #[test]
    fn test_grid_config_from_world_size() {
        let config = GridConfig::from_world_size(WorldSize::Large);
        assert_eq!(config.size, (512, 512, 160));
    }

    #[test]
    fn test_bounds_checking() {
        let config = GridConfig::new((100, 100, 64));

        // Within bounds (range is [-50, 50) for x and z, [0, 64) for y)
        assert!(config.is_in_bounds(0, 0, 0));
        assert!(config.is_in_bounds(49, 32, 49));
        assert!(config.is_in_bounds(-50, 63, -50)); // -50 is included (lower bound)
        assert!(config.is_in_bounds(-49, 0, -49));

        // Out of bounds
        assert!(!config.is_in_bounds(50, 0, 0));   // 50 is excluded (upper bound)
        assert!(!config.is_in_bounds(-51, 0, 0));  // -51 is out of bounds
        assert!(!config.is_in_bounds(0, -1, 0));   // negative y
        assert!(!config.is_in_bounds(0, 64, 0));   // y at max is excluded
    }

    #[test]
    fn test_world_volume() {
        let config = GridConfig::new((100, 100, 64));
        assert_eq!(config.volume(), 640000);
    }

    #[test]
    fn test_world_creation() {
        let world = World::with_size(WorldSize::Medium);
        assert_eq!(world.dimensions(), (256, 256, 128));
    }

    #[test]
    fn test_world_position_validation() {
        let world = World::with_size(WorldSize::Small);

        assert!(world.is_valid_position(0, 0, 0));
        assert!(world.is_valid_position(63, 95, 63));
        assert!(!world.is_valid_position(64, 0, 0));
        assert!(!world.is_valid_position(0, 96, 0));
    }

    #[test]
    fn test_world_config() {
        let config = WorldConfig::new(12345, WorldSize::Large);
        assert_eq!(config.seed, 12345);
        assert_eq!(config.grid.size, (512, 512, 160));
    }

    #[test]
    fn test_custom_world_config() {
        let config = WorldConfig::with_custom_size(99999, 300, 400, 150);
        assert_eq!(config.grid.size, (300, 400, 150));
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0, 0, 0);
        let p2 = Position::new(3, 4, 0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }

    #[test]
    fn test_estimated_memory() {
        let tiny = WorldSize::Tiny;
        let huge = WorldSize::Huge;

        assert!(tiny.estimated_memory_mb() < huge.estimated_memory_mb());
        assert!(tiny.estimated_memory_mb() < 50.0); // Tiny should be small
    }
}
pub struct Chunk;
