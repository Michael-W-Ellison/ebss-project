// src/world/terrain.rs
//! Terrain types and tile system.

use serde::{Deserialize, Serialize};

/// Types of terrain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainType {
    Plains,   // Walkable, good for farming and building
    Forest,   // Walkable, source of wood
    Mountain, // Slower movement, source of stone and iron
    Water,    // Not walkable (for now)
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
            TerrainType::Plains | TerrainType::Forest | TerrainType::Mountain => true,
            TerrainType::Water => false,
        }
    }

    /// Get movement cost (for pathfinding)
    pub fn movement_cost(&self) -> u32 {
        match self.terrain_type {
            TerrainType::Plains => 1,
            TerrainType::Forest => 2,
            TerrainType::Mountain => 3,
            TerrainType::Water => u32::MAX, // Impassable
        }
    }

    /// Get ASCII character for rendering
    pub fn ascii_char(&self) -> char {
        match self.terrain_type {
            TerrainType::Plains => '.',
            TerrainType::Forest => 'T',
            TerrainType::Mountain => '^',
            TerrainType::Water => '~',
        }
    }

    /// Get color code for terminal rendering (ANSI)
    pub fn color_code(&self) -> &'static str {
        match self.terrain_type {
            TerrainType::Plains => "\x1b[33m",   // Yellow
            TerrainType::Forest => "\x1b[32m",   // Green
            TerrainType::Mountain => "\x1b[37m", // White
            TerrainType::Water => "\x1b[34m",    // Blue
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
