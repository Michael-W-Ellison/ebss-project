// src/world/terrain.rs
//! Terrain types and tile system.

use serde::{Deserialize, Serialize};

/// Types of terrain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainType {
    Plains,   // Walkable, good for farming (grain, cotton) and building
    Forest,   // Walkable, source of wood, herbs, honey
    Mountain, // Slower movement, source of stone and iron
    Water,    // Not walkable (for now), source of fish

    // New terrain types for naturalistic resource distribution
    Desert,   // Hot, dry - source of sand
    Wetland,  // Marshy areas near water - source of clay, flax, reeds
    Meadow,   // Open grassland - herbs, flowers, wild food, grazing
    Hills,    // Between plains and mountains - coal deposits, grazing
    Beach,    // Coastal area - sand, shells, fish access
    Riverbank, // Along rivers - clay, flax, fishing
}

/// Terrain with properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Terrain {
    pub terrain_type: TerrainType,
}

impl Terrain {
    pub fn new(terrain_type: TerrainType) -> Self {
        Self { terrain_type }
    }

    /// Check if terrain is walkable
    pub fn is_walkable(&self) -> bool {
        match self.terrain_type {
            TerrainType::Plains | TerrainType::Forest | TerrainType::Mountain |
            TerrainType::Desert | TerrainType::Meadow | TerrainType::Hills |
            TerrainType::Beach | TerrainType::Riverbank => true,
            TerrainType::Water | TerrainType::Wetland => false, // Wetland is marshy, slow/impassable
        }
    }

    /// Get movement cost (for pathfinding)
    pub fn movement_cost(&self) -> u32 {
        match self.terrain_type {
            TerrainType::Plains | TerrainType::Meadow | TerrainType::Beach => 1,
            TerrainType::Forest | TerrainType::Hills | TerrainType::Riverbank => 2,
            TerrainType::Mountain | TerrainType::Desert => 3, // Desert is slow due to sand
            TerrainType::Water | TerrainType::Wetland => u32::MAX, // Impassable
        }
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self.terrain_type {
            TerrainType::Plains => '.',
            TerrainType::Forest => 'T',
            TerrainType::Mountain => '^',
            TerrainType::Water => '~',
            TerrainType::Desert => ':',
            TerrainType::Wetland => '%',
            TerrainType::Meadow => ',',
            TerrainType::Hills => 'n',
            TerrainType::Beach => '_',
            TerrainType::Riverbank => '=',
        }
    }

    /// Get color code for terminal rendering (ANSI)
    pub fn color_code(&self) -> &'static str {
        match self.terrain_type {
            TerrainType::Plains => "\x1b[33m",   // Yellow
            TerrainType::Forest => "\x1b[32m",   // Green
            TerrainType::Mountain => "\x1b[37m", // White
            TerrainType::Water => "\x1b[34m",    // Blue
            TerrainType::Desert => "\x1b[93m",   // Bright Yellow
            TerrainType::Wetland => "\x1b[36m",  // Cyan
            TerrainType::Meadow => "\x1b[92m",   // Bright Green
            TerrainType::Hills => "\x1b[90m",    // Dark Gray
            TerrainType::Beach => "\x1b[97m",    // Bright White
            TerrainType::Riverbank => "\x1b[96m", // Bright Cyan
        }
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new(TerrainType::Plains)
    }
}

/// A tile in the world grid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub terrain: Terrain,
    pub explored: bool, // For fog of war (future feature)
}

impl Tile {
    pub fn new(terrain_type: TerrainType) -> Self {
        Self {
            terrain: Terrain::new(terrain_type),
            explored: false, // Tiles start unexplored (fog of war)
        }
    }

    /// Mark this tile as explored (globally)
    pub fn mark_explored(&mut self) {
        self.explored = true;
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self::new(TerrainType::Plains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_walkable() {
        assert!(Terrain::new(TerrainType::Plains).is_walkable());
        assert!(Terrain::new(TerrainType::Forest).is_walkable());
        assert!(Terrain::new(TerrainType::Mountain).is_walkable());
        assert!(!Terrain::new(TerrainType::Water).is_walkable());
    }

    #[test]
    fn test_terrain_movement_cost() {
        assert_eq!(Terrain::new(TerrainType::Plains).movement_cost(), 1);
        assert_eq!(Terrain::new(TerrainType::Forest).movement_cost(), 2);
        assert_eq!(Terrain::new(TerrainType::Mountain).movement_cost(), 3);
        assert_eq!(Terrain::new(TerrainType::Water).movement_cost(), u32::MAX);
    }

    #[test]
    fn test_tile_creation() {
        let tile = Tile::new(TerrainType::Forest);
        assert_eq!(tile.terrain.terrain_type, TerrainType::Forest);
        assert!(!tile.explored); // Tiles start unexplored (fog of war)

        // Test marking as explored
        let mut tile = tile;
        tile.mark_explored();
        assert!(tile.explored);
    }
}
