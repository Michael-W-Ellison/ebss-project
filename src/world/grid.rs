// src/world/grid.rs
//! Spatial grid system for the world.

use serde::{Deserialize, Serialize};
use crate::world::Tile;

/// 2D Position in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Calculate Manhattan distance to another position
    pub fn distance_to(&self, other: &Position) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u32
    }

    /// Calculate Euclidean distance to another position
    pub fn euclidean_distance_to(&self, other: &Position) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get neighboring positions (4-directional)
    pub fn neighbors(&self) -> Vec<Position> {
        vec![
            Position::new(self.x + 1, self.y),
            Position::new(self.x - 1, self.y),
            Position::new(self.x, self.y + 1),
            Position::new(self.x, self.y - 1),
        ]
    }

    /// Get all 8 neighboring positions (including diagonals)
    pub fn neighbors_8(&self) -> Vec<Position> {
        vec![
            Position::new(self.x + 1, self.y),
            Position::new(self.x - 1, self.y),
            Position::new(self.x, self.y + 1),
            Position::new(self.x, self.y - 1),
            Position::new(self.x + 1, self.y + 1),
            Position::new(self.x + 1, self.y - 1),
            Position::new(self.x - 1, self.y + 1),
            Position::new(self.x - 1, self.y - 1),
        ]
    }
}

/// 2D Grid containing tiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<Tile>>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        let tiles = vec![vec![Tile::default(); width]; height];

        Self {
            width,
            height,
            tiles,
        }
    }

    /// Generate procedural terrain
    pub fn generate_terrain(&mut self) {
        use rand::Rng;
        use crate::world::TerrainType;
        let mut rng = rand::thread_rng();

        // Simple noise-based terrain generation
        for y in 0..self.height {
            for x in 0..self.width {
                let noise = self.simple_noise(x as f32 * 0.1, y as f32 * 0.1);

                let terrain_type = if noise < 0.2 {
                    TerrainType::Water
                } else if noise < 0.4 {
                    TerrainType::Plains
                } else if noise < 0.7 {
                    TerrainType::Forest
                } else {
                    TerrainType::Mountain
                };

                self.tiles[y][x].terrain.terrain_type = terrain_type;
            }
        }
    }

    // Simple noise function for terrain generation
    fn simple_noise(&self, x: f32, y: f32) -> f32 {
        let value = (x.sin() * 43758.5453 + y.cos() * 12345.6789).sin();
        (value + 1.0) / 2.0 // Normalize to 0-1
    }

    pub fn get_tile(&self, pos: &Position) -> Option<&Tile> {
        if self.is_valid_position(pos) {
            Some(&self.tiles[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_tile_mut(&mut self, pos: &Position) -> Option<&mut Tile> {
        if self.is_valid_position(pos) {
            Some(&mut self.tiles[pos.y as usize][pos.x as usize])
        } else {
            None
        }
    }

    pub fn is_valid_position(&self, pos: &Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && (pos.x as usize) < self.width && (pos.y as usize) < self.height
    }

    /// Find path from start to end (simple breadth-first search)
    pub fn find_path(&self, start: &Position, end: &Position) -> Option<Vec<Position>> {
        use std::collections::{HashMap, VecDeque};

        if !self.is_valid_position(start) || !self.is_valid_position(end) {
            return None;
        }

        if start == end {
            return Some(vec![*start]);
        }

        let mut queue = VecDeque::new();
        let mut came_from: HashMap<Position, Position> = HashMap::new();
        let mut visited = HashMap::new();

        queue.push_back(*start);
        visited.insert(*start, true);

        while let Some(current) = queue.pop_front() {
            if current == *end {
                // Reconstruct path
                let mut path = vec![current];
                let mut current = current;

                while let Some(&prev) = came_from.get(&current) {
                    path.push(prev);
                    current = prev;
                }

                path.reverse();
                return Some(path);
            }

            for neighbor in current.neighbors() {
                if !self.is_valid_position(&neighbor) {
                    continue;
                }

                // Check if tile is walkable
                if let Some(tile) = self.get_tile(&neighbor) {
                    if !tile.terrain.is_walkable() {
                        continue;
                    }
                }

                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, true);
                    came_from.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        None // No path found
    }

    /// Find path avoiding both terrain obstacles and occupied positions
    /// Returns the next position to move to (first step of path), not the full path
    pub fn find_path_with_agents(&self, start: &Position, end: &Position, occupied_positions: &[Position]) -> Option<Position> {
        use std::collections::{HashMap, VecDeque};

        if !self.is_valid_position(start) || !self.is_valid_position(end) {
            return None;
        }

        if start == end {
            return None; // Already at destination
        }

        // Check if destination is walkable
        if let Some(tile) = self.get_tile(end) {
            if !tile.terrain.is_walkable() {
                return None;
            }
        }

        let mut queue = VecDeque::new();
        let mut came_from: HashMap<Position, Position> = HashMap::new();
        let mut visited = HashMap::new();

        queue.push_back(*start);
        visited.insert(*start, true);

        while let Some(current) = queue.pop_front() {
            if current == *end {
                // Reconstruct path and return first step
                let mut path_node = current;

                while let Some(&prev) = came_from.get(&path_node) {
                    if prev == *start {
                        // path_node is the first step from start
                        return Some(path_node);
                    }
                    path_node = prev;
                }

                return Some(current); // Shouldn't happen, but fallback
            }

            for neighbor in current.neighbors() {
                if !self.is_valid_position(&neighbor) {
                    continue;
                }

                // Skip if occupied by another agent
                if occupied_positions.contains(&neighbor) {
                    continue;
                }

                // Check if tile is walkable
                if let Some(tile) = self.get_tile(&neighbor) {
                    if !tile.terrain.is_walkable() {
                        continue;
                    }
                }

                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, true);
                    came_from.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        None // No path found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0, 0);
        let p2 = Position::new(3, 4);

        assert_eq!(p1.distance_to(&p2), 7); // Manhattan distance
        assert!((p1.euclidean_distance_to(&p2) - 5.0).abs() < 0.001); // Euclidean distance
    }

    #[test]
    fn test_position_neighbors() {
        let pos = Position::new(5, 5);
        let neighbors = pos.neighbors();

        assert_eq!(neighbors.len(), 4);
        assert!(neighbors.contains(&Position::new(6, 5)));
        assert!(neighbors.contains(&Position::new(4, 5)));
        assert!(neighbors.contains(&Position::new(5, 6)));
        assert!(neighbors.contains(&Position::new(5, 4)));
    }

    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(10, 10);
        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 10);
        assert_eq!(grid.tiles.len(), 10);
        assert_eq!(grid.tiles[0].len(), 10);
    }

    #[test]
    fn test_grid_valid_position() {
        let grid = Grid::new(10, 10);

        assert!(grid.is_valid_position(&Position::new(0, 0)));
        assert!(grid.is_valid_position(&Position::new(9, 9)));
        assert!(!grid.is_valid_position(&Position::new(-1, 0)));
        assert!(!grid.is_valid_position(&Position::new(0, -1)));
        assert!(!grid.is_valid_position(&Position::new(10, 0)));
        assert!(!grid.is_valid_position(&Position::new(0, 10)));
    }

    #[test]
    fn test_grid_get_tile() {
        let grid = Grid::new(10, 10);
        let pos = Position::new(5, 5);

        assert!(grid.get_tile(&pos).is_some());
        assert!(grid.get_tile(&Position::new(-1, 0)).is_none());
    }
}
