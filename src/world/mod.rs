// src/world/mod.rs
//! Complete world simulation system with terrain, resources, buildings, and spatial management.

pub mod terrain;
pub mod resources;
pub mod buildings;
pub mod inventory;
pub mod actions;
pub mod grid;
pub mod render;

pub use terrain::{Terrain, TerrainType, Tile};
pub use resources::{Resource, ResourceType, ResourceNode};
pub use buildings::{Building, BuildingType, BuildingState};
pub use inventory::{Inventory, Item, ItemType};
pub use actions::{Action, ActionResult};
pub use grid::{Grid, Position};
pub use render::AsciiRenderer;

use crate::agents::Population;
use serde::{Deserialize, Serialize};

/// Complete world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub grid: Grid,
    pub resources: Vec<ResourceNode>,
    pub buildings: Vec<Building>,
    pub storehouse_inventory: Inventory,
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
    pub size: (u32, u32, u32),
    pub chunk_size: u32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            size: (100, 100, 10),
            chunk_size: 16,
        }
    }
}

pub struct Chunk;
