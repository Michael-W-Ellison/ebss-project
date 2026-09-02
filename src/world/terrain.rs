// src/world/terrain.rs
//! Terrain types and tile system.

use serde::{Deserialize, Serialize};

/// Types of terrain
/// Ordered, so that a set of terrains is iterated the same way twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TerrainType {
    Plains,   // Walkable, good for farming (grain, cotton) and building
    Forest,   // Walkable, source of wood, herbs, honey
    Mountain, // Slower movement, source of stone and iron
    Water,    // Swimmable with skill, source of fish

    // New terrain types for naturalistic resource distribution
    Desert,   // Hot, dry - source of sand
    Wetland,  // Marshy areas near water - source of clay, flax, reeds (wadeable)
    Meadow,   // Open grassland - herbs, flowers, wild food, grazing
    Hills,    // Between plains and mountains - coal deposits, grazing
    Beach,    // Coastal area - sand, shells, fish access
    Riverbank, // Along rivers - clay, flax, fishing

    /// Salt water. Everything in this world drank out of the same kind of
    /// water until now: a river, a spring and the sea were one terrain and
    /// one drink. The sea is where salt comes from and where a thirsty man
    /// makes his worst mistake.
    Sea,

    /// Where the sea meets the land and neither wins. Brackish, boggy, and
    /// worth boiling for what is in it.
    SaltMarsh,

    /// Where a shallow sea dried up and left what was in it. Rare, walkable,
    /// and the only place salt can simply be picked up off the ground.
    SaltFlat,

    /// Ground broken and sown by an agent - crops grow here far faster than
    /// anything wild, which is how a settlement feeds more people than the
    /// country around it would carry
    Farmland,
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

    /// Check if terrain is walkable (without swimming)
    pub fn is_walkable(&self) -> bool {
        match self.terrain_type {
            TerrainType::Plains | TerrainType::Forest | TerrainType::Mountain |
            TerrainType::Desert | TerrainType::Meadow | TerrainType::Hills |
            TerrainType::Beach | TerrainType::Riverbank | TerrainType::Wetland |
            TerrainType::Farmland | TerrainType::SaltMarsh | TerrainType::SaltFlat => true,
            TerrainType::Water | TerrainType::Sea => false, // Requires swimming
        }
    }

    /// Whether the water here is salt.
    ///
    /// "Agents should know not to drink salt water but if they do so it
    /// should increase their hydration drive more over time even if it seems
    /// to temporarily satiate it."
    pub fn is_the_water_salt(&self) -> bool {
        matches!(
            self.terrain_type,
            TerrainType::Sea | TerrainType::SaltMarsh
        )
    }

    /// Whether salt can be had here at all, by picking it up or by boiling
    /// for it.
    pub fn is_there_salt_here(&self) -> bool {
        matches!(
            self.terrain_type,
            TerrainType::Sea | TerrainType::SaltMarsh | TerrainType::SaltFlat
        )
    }

    /// Whether this ground can be broken into a field.
    ///
    /// Open grass only: nobody ploughs a forest, a mountainside or a marsh.
    pub fn can_be_tilled(&self) -> bool {
        matches!(
            self.terrain_type,
            TerrainType::Plains | TerrainType::Meadow
        )
    }

    /// Whether crops grow here
    pub fn is_cultivated(&self) -> bool {
        matches!(self.terrain_type, TerrainType::Farmland)
    }



    /// Check if this is aquatic terrain (water or wetland)
    pub fn is_aquatic(&self) -> bool {
        matches!(
            self.terrain_type,
            TerrainType::Water
                | TerrainType::Wetland
                | TerrainType::Riverbank
                | TerrainType::Sea
                | TerrainType::SaltMarsh
        )
    }

    /// Get movement cost (for pathfinding)
    pub fn movement_cost(&self) -> u32 {
        match self.terrain_type {
            TerrainType::Plains | TerrainType::Meadow | TerrainType::Beach
            | TerrainType::Farmland => 1,
            TerrainType::Forest | TerrainType::Hills | TerrainType::Riverbank => 2,
            TerrainType::Mountain | TerrainType::Desert => 3, // Desert is slow due to sand
            TerrainType::Wetland | TerrainType::SaltMarsh => 4, // Slow slogging through marsh
            TerrainType::SaltFlat => 2, // Crusted and uneven, but dry
            TerrainType::Water | TerrainType::Sea => u32::MAX, // Requires swimming skill check
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
            TerrainType::Sea => '≈',
            TerrainType::SaltMarsh => ';',
            TerrainType::SaltFlat => '=',
            TerrainType::Riverbank => '=',
            TerrainType::Farmland => '#',
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
            TerrainType::Sea => "\x1b[34m",      // Blue, darker than a river
            TerrainType::SaltMarsh => "\x1b[36m", // Cyan
            TerrainType::SaltFlat => "\x1b[97m", // Bright White
            TerrainType::Riverbank => "\x1b[96m", // Bright Cyan
            TerrainType::Farmland => "\x1b[33m",  // Yellow, like the crop on it
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
    pub explored: bool, // Global exploration state (any agent has seen this)
    pub last_seen_tick: Option<u32>, // When was this tile last observed

    /// The ground itself: what plants can draw on, and what is lying on it
    /// waiting to break down into more of the same
    #[serde(default)]
    pub soil: super::soil::Soil,
}

impl Tile {
    pub fn new(terrain_type: TerrainType) -> Self {
        Self {
            terrain: Terrain::new(terrain_type),
            explored: false, // Tiles start unexplored (fog of war)
            last_seen_tick: None,
            soil: super::soil::Soil::for_terrain(terrain_type),
        }
    }

    /// Mark this tile as explored (globally)
    pub fn mark_explored(&mut self) {
        self.explored = true;
    }



}

/// Visibility states for fog of war rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileVisibility {
    /// Currently visible (in line of sight)
    Visible,
    /// Seen recently but not current (fading)
    RecentlySeen,
    /// Explored but not currently visible (remembered)
    Explored,
    /// Never explored (fog of war)
    Unknown,
}

impl TileVisibility {
    /// Get the brightness multiplier for rendering
    pub fn brightness(&self) -> f32 {
        match self {
            TileVisibility::Visible => 1.0,
            TileVisibility::RecentlySeen => 0.7,
            TileVisibility::Explored => 0.4,
            TileVisibility::Unknown => 0.0,
        }
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
