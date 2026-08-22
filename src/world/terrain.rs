// src/world/terrain.rs
//! Terrain types and tile system.

use serde::{Deserialize, Serialize};

/// Types of terrain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            TerrainType::Farmland => true,
            TerrainType::Water => false, // Requires swimming
        }
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

    /// Check if terrain requires swimming
    pub fn requires_swimming(&self) -> bool {
        matches!(self.terrain_type, TerrainType::Water)
    }

    /// Check if terrain is passable with swimming capability
    pub fn is_passable_with_swimming(&self) -> bool {
        // All terrain is passable if you can swim
        true
    }

    /// Check if this is aquatic terrain (water or wetland)
    pub fn is_aquatic(&self) -> bool {
        matches!(self.terrain_type, TerrainType::Water | TerrainType::Wetland | TerrainType::Riverbank)
    }

    /// Get movement cost (for pathfinding)
    pub fn movement_cost(&self) -> u32 {
        match self.terrain_type {
            TerrainType::Plains | TerrainType::Meadow | TerrainType::Beach
            | TerrainType::Farmland => 1,
            TerrainType::Forest | TerrainType::Hills | TerrainType::Riverbank => 2,
            TerrainType::Mountain | TerrainType::Desert => 3, // Desert is slow due to sand
            TerrainType::Wetland => 4, // Slow slogging through marsh
            TerrainType::Water => u32::MAX, // Requires swimming skill check
        }
    }

    /// Get movement cost for an agent with swimming skill
    /// swimming_skill: 0.0 (can't swim) to 1.0 (expert swimmer)
    pub fn movement_cost_with_swimming(&self, swimming_skill: f32) -> u32 {
        match self.terrain_type {
            TerrainType::Water => {
                if swimming_skill <= 0.0 {
                    u32::MAX // Can't swim at all
                } else {
                    // Base cost of 5, reduced by swimming skill (min 3)
                    let skill_reduction = (swimming_skill * 2.0) as u32;
                    (5 - skill_reduction.min(2)).max(3)
                }
            }
            TerrainType::Wetland => {
                // Wetland is easier with swimming skill
                if swimming_skill > 0.3 {
                    3 // Skilled swimmer moves faster through marsh
                } else {
                    4 // Normal slog
                }
            }
            _ => self.movement_cost(),
        }
    }

    /// Check if an agent can enter this terrain
    /// swimming_skill: 0.0 to 1.0, None means no swimming check needed
    pub fn can_enter(&self, swimming_skill: Option<f32>) -> bool {
        match self.terrain_type {
            TerrainType::Water => {
                // Need at least minimal swimming skill (0.1) to enter water
                swimming_skill.map(|s| s >= 0.1).unwrap_or(false)
            }
            _ => self.is_walkable(),
        }
    }

    /// Get stamina cost multiplier for this terrain
    pub fn stamina_multiplier(&self) -> f32 {
        match self.terrain_type {
            TerrainType::Plains | TerrainType::Meadow | TerrainType::Beach => 1.0,
            TerrainType::Farmland => 1.2, // Worked ground is heavier going
            TerrainType::Forest | TerrainType::Hills | TerrainType::Riverbank => 1.3,
            TerrainType::Mountain => 2.0,
            TerrainType::Desert => 1.8, // Heat makes it tiring
            TerrainType::Wetland => 1.5, // Slogging is tiring
            TerrainType::Water => 3.0, // Swimming is very tiring
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
}

impl Tile {
    pub fn new(terrain_type: TerrainType) -> Self {
        Self {
            terrain: Terrain::new(terrain_type),
            explored: false, // Tiles start unexplored (fog of war)
            last_seen_tick: None,
        }
    }

    /// Mark this tile as explored (globally)
    pub fn mark_explored(&mut self) {
        self.explored = true;
    }

    /// Mark this tile as seen at a specific tick
    pub fn mark_seen(&mut self, tick: u32) {
        self.explored = true;
        self.last_seen_tick = Some(tick);
    }

    /// Check if tile is currently visible (seen recently)
    pub fn is_currently_visible(&self, current_tick: u32, visibility_duration: u32) -> bool {
        if let Some(last_seen) = self.last_seen_tick {
            current_tick.saturating_sub(last_seen) <= visibility_duration
        } else {
            false
        }
    }

    /// Get visibility state for rendering
    pub fn visibility_state(&self, current_tick: u32) -> TileVisibility {
        if let Some(last_seen) = self.last_seen_tick {
            let age = current_tick.saturating_sub(last_seen);
            if age == 0 {
                TileVisibility::Visible
            } else if age <= 100 {
                TileVisibility::RecentlySeen
            } else {
                TileVisibility::Explored
            }
        } else if self.explored {
            TileVisibility::Explored
        } else {
            TileVisibility::Unknown
        }
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

    /// Get ANSI color modifier for this visibility state
    pub fn color_modifier(&self) -> &'static str {
        match self {
            TileVisibility::Visible => "",           // Full color
            TileVisibility::RecentlySeen => "\x1b[2m", // Dim
            TileVisibility::Explored => "\x1b[90m",    // Gray
            TileVisibility::Unknown => "\x1b[30m",     // Black (hidden)
        }
    }

    /// Should entities on this tile be rendered?
    pub fn shows_entities(&self) -> bool {
        matches!(self, TileVisibility::Visible | TileVisibility::RecentlySeen)
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
