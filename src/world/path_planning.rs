// src/world/path_planning.rs
//! Path planning and road network system for efficient building connectivity
//!
//! This module provides:
//! - A* pathfinding between positions
//! - Road network management
//! - Minimum spanning tree generation for settlements
//! - Road types and upgrades

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;

pub type Position = (i32, i32, i32);

/// Types of roads with different properties
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoadType {
    /// Basic dirt path (1.0x speed)
    DirtPath,
    /// Gravel road (1.3x speed)
    GravelRoad,
    /// Stone road (1.5x speed)
    StoneRoad,
    /// Paved road (2.0x speed)
    PavedRoad,
}

impl RoadType {
    /// Get travel speed multiplier for this road type
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            RoadType::DirtPath => 1.0,
            RoadType::GravelRoad => 1.3,
            RoadType::StoneRoad => 1.5,
            RoadType::PavedRoad => 2.0,
        }
    }

}

/// A node in a path
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathNode {
    pub position: Position,
}

impl PathNode {
    pub fn new(position: Position) -> Self {
        Self { position }
    }
}

/// A road connecting two or more positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    nodes: Vec<PathNode>,
    road_type: RoadType,
}

impl Road {
    pub fn new(nodes: Vec<PathNode>) -> Self {
        Self {
            nodes,
            road_type: RoadType::DirtPath,
        }
    }

    pub fn nodes(&self) -> &[PathNode] {
        &self.nodes
    }

    pub fn road_type(&self) -> RoadType {
        self.road_type
    }

    pub fn upgrade_to(&mut self, new_type: RoadType) {
        self.road_type = new_type;
    }

    pub fn travel_speed_multiplier(&self) -> f32 {
        self.road_type.speed_multiplier()
    }

    pub fn length(&self) -> f32 {
        let mut total = 0.0;
        for i in 0..self.nodes.len().saturating_sub(1) {
            total += distance(self.nodes[i].position, self.nodes[i + 1].position);
        }
        total
    }

    /// Check if this road contains a position
    pub fn contains_position(&self, pos: Position) -> bool {
        self.nodes.iter().any(|node| node.position == pos)
    }
}

/// Network of all roads in the world
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoadNetwork {
    roads: Vec<Road>,
    // Cache of positions that have roads for quick lookup
    road_positions: HashSet<Position>,
}

impl RoadNetwork {
    pub fn new() -> Self {
        Self {
            roads: Vec::new(),
            road_positions: HashSet::new(),
        }
    }

    pub fn add_road(&mut self, road: Road) {
        // Add all positions from this road to the cache
        for node in road.nodes() {
            self.road_positions.insert(node.position);
        }
        self.roads.push(road);
    }

    pub fn get_roads(&self) -> &[Road] {
        &self.roads
    }

    pub fn has_road_at(&self, position: Position) -> bool {
        self.road_positions.contains(&position)
    }

    /// Check if two positions are connected through the road network
    pub fn are_connected(&self, pos1: Position, pos2: Position) -> bool {
        // Simple BFS through road network
        let mut visited = HashSet::new();
        let mut queue = vec![pos1];
        visited.insert(pos1);

        while let Some(current) = queue.pop() {
            if current == pos2 {
                return true;
            }

            // Find all roads that contain current position
            for road in &self.roads {
                if road.contains_position(current) {
                    // Add all positions from this road to queue
                    for node in road.nodes() {
                        if !visited.contains(&node.position) {
                            visited.insert(node.position);
                            queue.push(node.position);
                        }
                    }
                }
            }
        }

        false
    }

    /// Find intersection points between roads
    pub fn find_intersections(&self) -> Vec<Position> {
        let mut intersections = Vec::new();
        let mut position_count: HashMap<Position, usize> = HashMap::new();

        // Count how many roads pass through each position
        for road in &self.roads {
            for node in road.nodes() {
                *position_count.entry(node.position).or_insert(0) += 1;
            }
        }

        // Positions with multiple roads are intersections
        for (pos, count) in position_count {
            if count >= 2 {
                intersections.push(pos);
            }
        }

        intersections
    }

    pub fn clear(&mut self) {
        self.roads.clear();
        self.road_positions.clear();
    }
}

/// Node for A* pathfinding
#[derive(Clone, PartialEq)]
struct AStarNode {
    position: Position,
    g_cost: f32, // Cost from start
    h_cost: f32, // Heuristic cost to goal
    parent: Option<Position>,
}

impl AStarNode {
    fn f_cost(&self) -> f32 {
        self.g_cost + self.h_cost
    }
}

impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_cost().partial_cmp(&self.f_cost()).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Path planner using A* algorithm
pub struct PathPlanner<'a> {
    world: &'a crate::world::World,
}

impl<'a> PathPlanner<'a> {
    pub fn new(world: &'a crate::world::World) -> Self {
        Self { world }
    }

    pub fn is_initialized(&self) -> bool {
        true
    }

    /// Find a path between two positions using A* algorithm
    pub fn find_path(&self, start: Position, goal: Position) -> Option<Vec<PathNode>> {
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<Position, Position> = HashMap::new();
        let mut g_scores: HashMap<Position, f32> = HashMap::new();

        g_scores.insert(start, 0.0);
        open_set.push(AStarNode {
            position: start,
            g_cost: 0.0,
            h_cost: heuristic(start, goal),
            parent: None,
        });

        while let Some(current) = open_set.pop() {
            if current.position == goal {
                // Reconstruct path
                return Some(self.reconstruct_path(&came_from, goal));
            }

            // Check all neighbors
            for neighbor in get_neighbors(current.position) {
                // Skip if impassable
                if !self.world.is_terrain_passable(neighbor) {
                    continue;
                }

                let tentative_g = current.g_cost + distance(current.position, neighbor);

                if tentative_g < *g_scores.get(&neighbor).unwrap_or(&f32::MAX) {
                    came_from.insert(neighbor, current.position);
                    g_scores.insert(neighbor, tentative_g);

                    open_set.push(AStarNode {
                        position: neighbor,
                        g_cost: tentative_g,
                        h_cost: heuristic(neighbor, goal),
                        parent: Some(current.position),
                    });
                }
            }
        }

        None // No path found
    }

    fn reconstruct_path(&self, came_from: &HashMap<Position, Position>, goal: Position) -> Vec<PathNode> {
        let mut path = vec![PathNode::new(goal)];
        let mut current = goal;

        while let Some(&parent) = came_from.get(&current) {
            path.push(PathNode::new(parent));
            current = parent;
        }

        path.reverse();
        path
    }

    /// Connect two buildings with a path
    pub fn connect_buildings(&self, pos1: Position, pos2: Position) -> Option<Vec<PathNode>> {
        self.find_path(pos1, pos2)
    }

    /// Calculate total cost of a path
    pub fn calculate_path_cost(path: &[PathNode]) -> f32 {
        let mut cost = 0.0;
        for i in 0..path.len().saturating_sub(1) {
            cost += distance(path[i].position, path[i + 1].position);
        }
        cost
    }

    /// Create minimum spanning tree to connect all buildings
    pub fn create_minimum_spanning_tree(&self, positions: &[Position]) -> Vec<Vec<PathNode>> {
        if positions.len() < 2 {
            return Vec::new();
        }

        let mut roads = Vec::new();
        let mut connected = HashSet::new();
        connected.insert(positions[0]);

        // Prim's algorithm for MST
        while connected.len() < positions.len() {
            let mut min_cost = f32::MAX;
            let mut best_edge = None;

            // Find minimum cost edge connecting tree to new node
            for &conn_pos in &connected {
                for &other_pos in positions {
                    if !connected.contains(&other_pos) {
                        let cost = distance(conn_pos, other_pos);
                        if cost < min_cost {
                            min_cost = cost;
                            best_edge = Some((conn_pos, other_pos));
                        }
                    }
                }
            }

            if let Some((from, to)) = best_edge {
                connected.insert(to);
                if let Some(path) = self.find_path(from, to) {
                    roads.push(path);
                }
            } else {
                break; // No more connections possible
            }
        }

        roads
    }

    /// Find nearest point on existing road network to connect a new building
    pub fn find_nearest_road_connection(&self, building_pos: Position) -> Option<Position> {
        let mut min_distance = f32::MAX;
        let mut nearest = None;

        for road in self.world.road_network.get_roads() {
            for node in road.nodes() {
                let dist = distance(building_pos, node.position);
                if dist < min_distance {
                    min_distance = dist;
                    nearest = Some(node.position);
                }
            }
        }

        nearest
    }

    /// Smooth a path by removing unnecessary waypoints
    pub fn smooth_path(&self, path: &[PathNode]) -> Vec<PathNode> {
        if path.len() <= 2 {
            return path.to_vec();
        }

        let mut smoothed = vec![path[0].clone()];
        let mut i = 0;

        while i < path.len() - 1 {
            let mut farthest = i + 1;

            // Try to skip as many nodes as possible while maintaining passability
            for j in (i + 2)..path.len() {
                if self.is_line_passable(path[i].position, path[j].position) {
                    farthest = j;
                } else {
                    break;
                }
            }

            smoothed.push(path[farthest].clone());
            i = farthest;
        }

        smoothed
    }

    fn is_line_passable(&self, from: Position, to: Position) -> bool {
        // Simple line-of-sight check
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let steps = dx.abs().max(dy.abs());

        for step in 0..=steps {
            let t = if steps > 0 { step as f32 / steps as f32 } else { 0.0 };
            let x = from.0 + (dx as f32 * t) as i32;
            let y = from.1 + (dy as f32 * t) as i32;
            let z = from.2;

            if !self.world.is_terrain_passable((x, y, z)) {
                return false;
            }
        }

        true
    }
}

/// Calculate Euclidean distance between two positions
fn distance(a: Position, b: Position) -> f32 {
    let dx = (a.0 - b.0) as f32;
    let dy = (a.1 - b.1) as f32;
    let dz = (a.2 - b.2) as f32;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Heuristic function for A* (Euclidean distance)
fn heuristic(a: Position, b: Position) -> f32 {
    distance(a, b)
}

/// Get valid neighbors for a position (8-directional movement)
fn get_neighbors(pos: Position) -> Vec<Position> {
    vec![
        (pos.0 + 1, pos.1, pos.2),     // East
        (pos.0 - 1, pos.1, pos.2),     // West
        (pos.0, pos.1 + 1, pos.2),     // South
        (pos.0, pos.1 - 1, pos.2),     // North
        (pos.0 + 1, pos.1 + 1, pos.2), // SE
        (pos.0 + 1, pos.1 - 1, pos.2), // NE
        (pos.0 - 1, pos.1 + 1, pos.2), // SW
        (pos.0 - 1, pos.1 - 1, pos.2), // NW
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_road_type_speed_multipliers() {
        assert_eq!(RoadType::DirtPath.speed_multiplier(), 1.0);
        assert_eq!(RoadType::GravelRoad.speed_multiplier(), 1.3);
        assert_eq!(RoadType::StoneRoad.speed_multiplier(), 1.5);
        assert_eq!(RoadType::PavedRoad.speed_multiplier(), 2.0);
    }

    #[test]
    fn test_path_node_creation() {
        let node = PathNode::new((10, 20, 0));
        assert_eq!(node.position, (10, 20, 0));
    }

    #[test]
    fn test_road_creation() {
        let road = Road::new(vec![
            PathNode::new((0, 0, 0)),
            PathNode::new((1, 1, 0)),
        ]);
        assert_eq!(road.nodes().len(), 2);
        assert_eq!(road.road_type(), RoadType::DirtPath);
    }

    #[test]
    fn test_road_upgrade() {
        let mut road = Road::new(vec![PathNode::new((0, 0, 0))]);
        road.upgrade_to(RoadType::StoneRoad);
        assert_eq!(road.road_type(), RoadType::StoneRoad);
    }

    #[test]
    fn test_distance_calculation() {
        let dist = distance((0, 0, 0), (3, 4, 0));
        assert!((dist - 5.0).abs() < 0.01); // 3-4-5 triangle
    }

    #[test]
    fn test_get_neighbors() {
        let neighbors = get_neighbors((10, 10, 0));
        assert_eq!(neighbors.len(), 8); // 8-directional
    }
}
