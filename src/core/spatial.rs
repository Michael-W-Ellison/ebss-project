// src/core/spatial.rs
//! Spatial utilities for efficient proximity queries.
//!
//! This module provides spatial data structures optimized for:
//! - Finding nearby entities without O(n²) comparisons
//! - Distance calculations with squared-distance optimization
//! - Grid-based spatial partitioning

use std::collections::HashMap;
use uuid::Uuid;

/// Cell size for spatial grid (agents within same or adjacent cells can interact)
pub const CELL_SIZE: i32 = 10;

/// Squared distance threshold for interaction range (10 tiles)
pub const INTERACTION_RANGE_SQUARED: f32 = 100.0;

/// Squared distance threshold for close range (5 tiles)
pub const CLOSE_RANGE_SQUARED: f32 = 25.0;

/// Spatial grid for efficient proximity queries
///
/// Reduces O(n²) pairwise comparisons to O(n * k) where k is average
/// entities per cell and adjacent cells.
#[derive(Debug, Default)]
pub struct SpatialGrid {
    /// Maps grid cell coordinates to list of entity IDs in that cell
    cells: HashMap<(i32, i32), Vec<Uuid>>,
    /// Maps entity IDs to their current cell for fast lookup
    entity_cells: HashMap<Uuid, (i32, i32)>,
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Clear the grid
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    /// Get the grid cell for a position
    #[inline]
    pub fn get_cell(x: i32, y: i32) -> (i32, i32) {
        (x / CELL_SIZE, y / CELL_SIZE)
    }

    /// Insert an entity at a position
    pub fn insert(&mut self, id: Uuid, x: i32, y: i32) {
        let cell = Self::get_cell(x, y);

        // Remove from old cell if moved
        if let Some(old_cell) = self.entity_cells.get(&id) {
            if *old_cell != cell {
                if let Some(entities) = self.cells.get_mut(old_cell) {
                    entities.retain(|e| *e != id);
                }
            }
        }

        // Add to new cell
        self.cells.entry(cell).or_default().push(id);
        self.entity_cells.insert(id, cell);
    }

    /// Remove an entity from the grid
    pub fn remove(&mut self, id: &Uuid) {
        if let Some(cell) = self.entity_cells.remove(id) {
            if let Some(entities) = self.cells.get_mut(&cell) {
                entities.retain(|e| e != id);
            }
        }
    }

    /// Get all entities in the same cell and adjacent cells
    pub fn get_nearby(&self, x: i32, y: i32) -> Vec<Uuid> {
        let (cx, cy) = Self::get_cell(x, y);
        let mut nearby = Vec::new();

        // Check 3x3 grid of cells (current + 8 neighbors)
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(entities) = self.cells.get(&(cx + dx, cy + dy)) {
                    nearby.extend(entities.iter().cloned());
                }
            }
        }

        nearby
    }

    /// Get all entities in the same cell and adjacent cells (excluding self)
    pub fn get_nearby_excluding(&self, x: i32, y: i32, exclude: &Uuid) -> Vec<Uuid> {
        let mut nearby = self.get_nearby(x, y);
        nearby.retain(|id| id != exclude);
        nearby
    }

    /// Get number of entities in the grid
    pub fn len(&self) -> usize {
        self.entity_cells.len()
    }

    /// Check if grid is empty
    pub fn is_empty(&self) -> bool {
        self.entity_cells.is_empty()
    }
}

/// Calculate squared distance between two positions (faster than sqrt)
#[inline]
pub fn distance_squared(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> f32 {
    let dx = (pos1.0 - pos2.0) as f32;
    let dy = (pos1.1 - pos2.1) as f32;
    dx * dx + dy * dy
}

/// Calculate squared distance for 2D positions
#[inline]
pub fn distance_squared_2d(pos1: (i32, i32), pos2: (i32, i32)) -> f32 {
    let dx = (pos1.0 - pos2.0) as f32;
    let dy = (pos1.1 - pos2.1) as f32;
    dx * dx + dy * dy
}

/// Check if two positions are within interaction range (10 tiles)
#[inline]
pub fn within_interaction_range(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> bool {
    distance_squared(pos1, pos2) <= INTERACTION_RANGE_SQUARED
}

/// Check if two positions are within close range (5 tiles)
#[inline]
pub fn within_close_range(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> bool {
    distance_squared(pos1, pos2) <= CLOSE_RANGE_SQUARED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_grid_insert_and_get_nearby() {
        let mut grid = SpatialGrid::new();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        grid.insert(id1, 5, 5);    // Cell (0, 0)
        grid.insert(id2, 15, 15);  // Cell (1, 1)
        grid.insert(id3, 50, 50);  // Cell (5, 5) - far away

        let nearby = grid.get_nearby(5, 5);
        assert!(nearby.contains(&id1));
        assert!(nearby.contains(&id2)); // Adjacent cell
        assert!(!nearby.contains(&id3)); // Far cell
    }

    #[test]
    fn test_spatial_grid_remove() {
        let mut grid = SpatialGrid::new();

        let id = Uuid::new_v4();
        grid.insert(id, 5, 5);
        assert_eq!(grid.len(), 1);

        grid.remove(&id);
        assert!(grid.is_empty());
    }

    #[test]
    fn test_distance_squared() {
        let pos1 = (0, 0, 0);
        let pos2 = (3, 4, 0);

        // 3² + 4² = 25
        assert_eq!(distance_squared(pos1, pos2), 25.0);
    }

    #[test]
    fn test_within_interaction_range() {
        let pos1 = (0, 0, 0);
        let pos2 = (7, 7, 0);  // Distance ≈ 9.9
        let pos3 = (10, 10, 0); // Distance ≈ 14.1

        assert!(within_interaction_range(pos1, pos2));
        assert!(!within_interaction_range(pos1, pos3));
    }

    #[test]
    fn test_within_close_range() {
        let pos1 = (0, 0, 0);
        let pos2 = (3, 3, 0);  // Distance ≈ 4.2
        let pos3 = (4, 4, 0);  // Distance ≈ 5.6

        assert!(within_close_range(pos1, pos2));
        assert!(!within_close_range(pos1, pos3));
    }

    #[test]
    fn test_get_cell() {
        assert_eq!(SpatialGrid::get_cell(0, 0), (0, 0));
        assert_eq!(SpatialGrid::get_cell(9, 9), (0, 0));
        assert_eq!(SpatialGrid::get_cell(10, 10), (1, 1));
        // Note: Rust integer division truncates toward zero
        assert_eq!(SpatialGrid::get_cell(-5, -5), (0, 0));
        assert_eq!(SpatialGrid::get_cell(-10, -10), (-1, -1));
    }
}
