// src/world/resource_spawning.rs
//! Naturalistic resource spawning system.
//!
//! This module handles spawning resources in a way that mimics natural distribution:
//! - Resources appear in terrain-appropriate locations
//! - Resources cluster together (veins, patches, groves)
//! - Edge effects: some resources prefer terrain transitions
//! - Flora/fauna integration for renewable resources

use rand::Rng;

use crate::world::{Position, ResourceNode, ResourceType, TerrainType, Grid};

/// Configuration for naturalistic resource spawning
#[derive(Debug, Clone)]
pub struct NaturalisticResourceConfig {
    // === Mineral Resources ===
    /// Number of clay deposit clusters
    pub clay_clusters: usize,
    /// Number of sand deposit clusters
    pub sand_clusters: usize,
    /// Number of coal vein clusters
    pub coal_clusters: usize,

    // === Agricultural Resources ===
    /// Number of wild grain patches
    pub grain_patches: usize,
    /// Number of flax patches
    pub flax_patches: usize,
    /// Number of herb patches
    pub herb_patches: usize,
    /// Number of cotton patches
    pub cotton_patches: usize,

    // === Gatherable Resources ===
    /// Number of honey/beehive locations
    pub honey_locations: usize,
    /// Number of fish spawning areas
    pub fish_areas: usize,

    // === Cluster settings ===
    /// Average nodes per cluster
    pub nodes_per_cluster: usize,
    /// Maximum cluster spread radius
    pub cluster_radius: i32,
}

impl Default for NaturalisticResourceConfig {
    fn default() -> Self {
        Self {
            // Minerals - less common, clustered in deposits
            clay_clusters: 4,
            sand_clusters: 3,
            coal_clusters: 3,

            // Agricultural - scattered patches
            grain_patches: 5,
            flax_patches: 3,
            herb_patches: 6,
            cotton_patches: 2,

            // Gatherable
            honey_locations: 4,
            fish_areas: 5,

            // Clustering
            nodes_per_cluster: 3,
            cluster_radius: 5,
        }
    }
}

/// Maps resource types to their preferred terrain types
pub struct TerrainResourceMapper;

impl TerrainResourceMapper {
    /// Get preferred terrain types for a resource
    pub fn preferred_terrains(resource: ResourceType) -> Vec<TerrainType> {
        match resource {
            // Basic resources (existing behavior)
            ResourceType::Wood => vec![TerrainType::Forest],
            ResourceType::Stone => vec![TerrainType::Mountain, TerrainType::Hills],
            ResourceType::Iron => vec![TerrainType::Mountain],
            ResourceType::Food => vec![TerrainType::Plains, TerrainType::Meadow, TerrainType::Forest],

            // Minerals
            ResourceType::Clay => vec![TerrainType::Wetland, TerrainType::Riverbank],
            ResourceType::Sand => vec![TerrainType::Desert, TerrainType::Beach],
            ResourceType::Coal => vec![TerrainType::Hills, TerrainType::Mountain],

            // Agricultural
            ResourceType::Grain => vec![TerrainType::Plains, TerrainType::Meadow],
            ResourceType::Flax => vec![TerrainType::Riverbank, TerrainType::Wetland, TerrainType::Meadow],
            ResourceType::Herbs => vec![TerrainType::Forest, TerrainType::Meadow],
            ResourceType::Cotton => vec![TerrainType::Plains], // Prefers warm, dry areas

            // Animal-derived (from fauna, not terrain-spawned)
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk => vec![],

            // Gatherable
            ResourceType::Fish => vec![TerrainType::Water, TerrainType::Beach, TerrainType::Riverbank],
            ResourceType::Honey => vec![TerrainType::Forest, TerrainType::Meadow],

            // Processed/finished goods don't spawn naturally
            _ => vec![],
        }
    }

    /// Get amount range for a resource type
    pub fn amount_range(resource: ResourceType) -> (u32, u32) {
        match resource {
            // Basic resources
            ResourceType::Wood => (50, 150),
            ResourceType::Stone => (80, 200),
            ResourceType::Iron => (30, 100),
            ResourceType::Food => (20, 60),

            // Minerals
            ResourceType::Clay => (40, 120),
            ResourceType::Sand => (60, 180),
            ResourceType::Coal => (25, 80),

            // Agricultural
            ResourceType::Grain => (30, 80),
            ResourceType::Flax => (20, 50),
            ResourceType::Herbs => (15, 40),
            ResourceType::Cotton => (25, 60),

            // Gatherable
            ResourceType::Fish => (40, 100),
            ResourceType::Honey => (10, 30),

            // Default for others
            _ => (10, 30),
        }
    }

    /// Check if a resource can spawn at terrain transitions (edge effects)
    pub fn prefers_edges(resource: ResourceType) -> bool {
        matches!(
            resource,
            ResourceType::Clay |      // Found at water edges
            ResourceType::Flax |      // Grows near water
            ResourceType::Herbs |     // Forest edges
            ResourceType::Fish        // Water edges
        )
    }
}

/// Spawns resources in naturalistic clusters
pub struct NaturalisticSpawner<'a> {
    grid: &'a Grid,
    rng: rand::rngs::ThreadRng,
}

impl<'a> NaturalisticSpawner<'a> {
    pub fn new(grid: &'a Grid) -> Self {
        Self {
            grid,
            rng: rand::thread_rng(),
        }
    }

    /// Spawn all resources according to configuration
    pub fn spawn_all(&mut self, config: &NaturalisticResourceConfig) -> Vec<ResourceNode> {
        let mut resources = Vec::new();

        // Spawn mineral clusters
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Clay,
            config.clay_clusters,
            config.nodes_per_cluster,
            config.cluster_radius,
        ));
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Sand,
            config.sand_clusters,
            config.nodes_per_cluster,
            config.cluster_radius,
        ));
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Coal,
            config.coal_clusters,
            config.nodes_per_cluster,
            config.cluster_radius,
        ));

        // Spawn agricultural patches
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Grain,
            config.grain_patches,
            config.nodes_per_cluster + 1, // Patches are slightly larger
            config.cluster_radius + 2,
        ));
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Flax,
            config.flax_patches,
            config.nodes_per_cluster,
            config.cluster_radius,
        ));
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Herbs,
            config.herb_patches,
            config.nodes_per_cluster,
            config.cluster_radius + 1,
        ));
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Cotton,
            config.cotton_patches,
            config.nodes_per_cluster,
            config.cluster_radius,
        ));

        // Spawn gatherable resources
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Honey,
            config.honey_locations,
            1, // Single nodes (beehives)
            0,
        ));
        resources.extend(self.spawn_resource_clusters(
            ResourceType::Fish,
            config.fish_areas,
            config.nodes_per_cluster,
            config.cluster_radius,
        ));

        resources
    }

    /// Spawn clusters of a specific resource type
    fn spawn_resource_clusters(
        &mut self,
        resource_type: ResourceType,
        num_clusters: usize,
        nodes_per_cluster: usize,
        cluster_radius: i32,
    ) -> Vec<ResourceNode> {
        let mut nodes = Vec::new();
        let preferred_terrains = TerrainResourceMapper::preferred_terrains(resource_type);

        if preferred_terrains.is_empty() {
            return nodes; // Resource doesn't spawn naturally
        }

        let prefers_edges = TerrainResourceMapper::prefers_edges(resource_type);

        for _ in 0..num_clusters {
            // Find a suitable center for this cluster
            let center = if prefers_edges {
                self.find_edge_position(&preferred_terrains)
            } else {
                self.find_terrain_position(&preferred_terrains)
            };

            if let Some(center_pos) = center {
                // Spawn nodes in cluster around center
                for i in 0..nodes_per_cluster {
                    let pos = if i == 0 {
                        center_pos.clone()
                    } else {
                        self.offset_position(&center_pos, cluster_radius)
                    };

                    // Verify position is valid terrain
                    if self.is_valid_resource_position(&pos, &preferred_terrains) {
                        let (min_amount, max_amount) = TerrainResourceMapper::amount_range(resource_type);
                        let amount = self.rng.gen_range(min_amount..=max_amount);
                        nodes.push(ResourceNode::new(resource_type, pos, amount));
                    }
                }
            }
        }

        nodes
    }

    /// Find a random position in one of the preferred terrain types
    fn find_terrain_position(&mut self, preferred_terrains: &[TerrainType]) -> Option<Position> {
        for _ in 0..100 {
            let x = self.rng.gen_range(0..self.grid.width) as i32;
            let y = self.rng.gen_range(0..self.grid.height) as i32;
            let pos = Position::new(x, y);

            if let Some(tile) = self.grid.get_tile(&pos) {
                if preferred_terrains.contains(&tile.terrain.terrain_type) {
                    return Some(pos);
                }
            }
        }
        None
    }

    /// Find a position at terrain edges (transitions between terrain types)
    fn find_edge_position(&mut self, preferred_terrains: &[TerrainType]) -> Option<Position> {
        for _ in 0..100 {
            let x = self.rng.gen_range(1..self.grid.width - 1) as i32;
            let y = self.rng.gen_range(1..self.grid.height - 1) as i32;
            let pos = Position::new(x, y);

            if let Some(tile) = self.grid.get_tile(&pos) {
                if preferred_terrains.contains(&tile.terrain.terrain_type) {
                    // Check if this is an edge (adjacent to different terrain)
                    if self.is_terrain_edge(&pos) {
                        return Some(pos);
                    }
                }
            }
        }

        // Fallback to non-edge position
        self.find_terrain_position(preferred_terrains)
    }

    /// Check if position is at a terrain edge
    fn is_terrain_edge(&self, pos: &Position) -> bool {
        if let Some(center_tile) = self.grid.get_tile(pos) {
            let center_terrain = center_tile.terrain.terrain_type;

            // Check 4-connected neighbors
            let neighbors = [
                Position::new(pos.x - 1, pos.y),
                Position::new(pos.x + 1, pos.y),
                Position::new(pos.x, pos.y - 1),
                Position::new(pos.x, pos.y + 1),
            ];

            for neighbor_pos in &neighbors {
                if let Some(neighbor_tile) = self.grid.get_tile(neighbor_pos) {
                    if neighbor_tile.terrain.terrain_type != center_terrain {
                        return true; // Found different terrain = edge
                    }
                }
            }
        }
        false
    }

    /// Offset a position randomly within a radius
    fn offset_position(&mut self, center: &Position, radius: i32) -> Position {
        let dx = self.rng.gen_range(-radius..=radius);
        let dy = self.rng.gen_range(-radius..=radius);
        Position::new(
            (center.x + dx).clamp(0, self.grid.width as i32 - 1),
            (center.y + dy).clamp(0, self.grid.height as i32 - 1),
        )
    }

    /// Check if position is valid for placing a resource
    fn is_valid_resource_position(&self, pos: &Position, preferred_terrains: &[TerrainType]) -> bool {
        if let Some(tile) = self.grid.get_tile(pos) {
            return preferred_terrains.contains(&tile.terrain.terrain_type);
        }
        false
    }
}

/// Animal-based resource generation configuration
#[derive(Debug, Clone)]
pub struct AnimalResourceConfig {
    /// Chance that hunting produces hides (0.0-1.0)
    pub hide_drop_chance: f32,
    /// Chance that hunting produces meat
    pub meat_drop_chance: f32,
    /// Base wool production per tick for sheep
    pub wool_production_rate: f32,
    /// Base milk production per tick for cattle
    pub milk_production_rate: f32,
}

impl Default for AnimalResourceConfig {
    fn default() -> Self {
        Self {
            hide_drop_chance: 0.8,
            meat_drop_chance: 0.9,
            wool_production_rate: 0.05,
            milk_production_rate: 0.1,
        }
    }
}

/// Maps animal species to resources they can produce
pub struct AnimalResourceMapper;

impl AnimalResourceMapper {
    /// Get resources produced when an animal is hunted/killed
    pub fn hunting_products(species: &str) -> Vec<(ResourceType, u32)> {
        match species.to_lowercase().as_str() {
            "deer" | "elk" | "boar" => vec![
                (ResourceType::Meat, 3),
                (ResourceType::Hides, 1),
            ],
            "rabbit" | "hare" => vec![
                (ResourceType::Meat, 1),
                (ResourceType::Hides, 1),
            ],
            "wolf" | "bear" => vec![
                (ResourceType::Meat, 2),
                (ResourceType::Hides, 2),
            ],
            "fish" | "salmon" | "trout" => vec![
                (ResourceType::Fish, 1),
            ],
            "cow" | "cattle" | "ox" => vec![
                (ResourceType::Meat, 5),
                (ResourceType::Hides, 2),
            ],
            "sheep" => vec![
                (ResourceType::Meat, 2),
                (ResourceType::Wool, 2),
            ],
            "pig" => vec![
                (ResourceType::Meat, 4),
            ],
            "chicken" | "goose" | "duck" => vec![
                (ResourceType::Meat, 1),
            ],
            _ => vec![
                (ResourceType::Meat, 1),
            ],
        }
    }

    /// Get resources produced by living animals (husbandry)
    pub fn husbandry_products(species: &str) -> Vec<(ResourceType, f32)> {
        match species.to_lowercase().as_str() {
            "sheep" => vec![
                (ResourceType::Wool, 0.1), // Per tick production rate
            ],
            "cow" | "cattle" => vec![
                (ResourceType::Milk, 0.15),
            ],
            "goat" => vec![
                (ResourceType::Milk, 0.1),
            ],
            "chicken" => vec![
                (ResourceType::Food, 0.2), // Eggs as food
            ],
            "bee" | "beehive" => vec![
                (ResourceType::Honey, 0.05),
            ],
            _ => vec![],
        }
    }
}

/// Terrain generation helper for creating naturalistic worlds
pub struct TerrainGenerator;

impl TerrainGenerator {
    /// Generate adjacent terrain types around water to create natural transitions
    pub fn generate_water_adjacent_terrain() -> Vec<(TerrainType, f32)> {
        vec![
            (TerrainType::Beach, 0.3),      // Beaches along coasts
            (TerrainType::Riverbank, 0.4),  // Riverbanks along rivers
            (TerrainType::Wetland, 0.2),    // Wetlands in low areas
            (TerrainType::Plains, 0.1),     // Some plains
        ]
    }

    /// Generate terrain types around mountains
    pub fn generate_mountain_adjacent_terrain() -> Vec<(TerrainType, f32)> {
        vec![
            (TerrainType::Hills, 0.5),      // Hills are common around mountains
            (TerrainType::Forest, 0.3),     // Forests on lower slopes
            (TerrainType::Plains, 0.2),     // Valleys
        ]
    }

    /// Generate terrain types for arid regions
    pub fn generate_arid_terrain() -> Vec<(TerrainType, f32)> {
        vec![
            (TerrainType::Desert, 0.6),
            (TerrainType::Plains, 0.3),
            (TerrainType::Hills, 0.1),
        ]
    }

    /// Generate terrain types for temperate regions
    pub fn generate_temperate_terrain() -> Vec<(TerrainType, f32)> {
        vec![
            (TerrainType::Plains, 0.3),
            (TerrainType::Forest, 0.3),
            (TerrainType::Meadow, 0.25),
            (TerrainType::Hills, 0.15),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_resource_mapping() {
        // Clay should spawn in wetlands and riverbanks
        let clay_terrains = TerrainResourceMapper::preferred_terrains(ResourceType::Clay);
        assert!(clay_terrains.contains(&TerrainType::Wetland));
        assert!(clay_terrains.contains(&TerrainType::Riverbank));

        // Sand should spawn in desert and beach
        let sand_terrains = TerrainResourceMapper::preferred_terrains(ResourceType::Sand);
        assert!(sand_terrains.contains(&TerrainType::Desert));
        assert!(sand_terrains.contains(&TerrainType::Beach));

        // Coal should spawn in hills and mountains
        let coal_terrains = TerrainResourceMapper::preferred_terrains(ResourceType::Coal);
        assert!(coal_terrains.contains(&TerrainType::Hills));
        assert!(coal_terrains.contains(&TerrainType::Mountain));
    }

    #[test]
    fn test_animal_resource_mapping() {
        // Deer should produce meat and hides
        let deer_products = AnimalResourceMapper::hunting_products("deer");
        assert!(deer_products.iter().any(|(r, _)| *r == ResourceType::Meat));
        assert!(deer_products.iter().any(|(r, _)| *r == ResourceType::Hides));

        // Sheep should produce wool when alive
        let sheep_husbandry = AnimalResourceMapper::husbandry_products("sheep");
        assert!(sheep_husbandry.iter().any(|(r, _)| *r == ResourceType::Wool));
    }

    #[test]
    fn test_edge_preference() {
        // Clay prefers edges (water/land interface)
        assert!(TerrainResourceMapper::prefers_edges(ResourceType::Clay));
        assert!(TerrainResourceMapper::prefers_edges(ResourceType::Flax));

        // Stone doesn't prefer edges
        assert!(!TerrainResourceMapper::prefers_edges(ResourceType::Stone));
    }

    #[test]
    fn test_amount_ranges() {
        // Check that ranges are valid (min < max)
        let resources = vec![
            ResourceType::Clay, ResourceType::Sand, ResourceType::Coal,
            ResourceType::Grain, ResourceType::Flax, ResourceType::Herbs,
        ];

        for resource in resources {
            let (min, max) = TerrainResourceMapper::amount_range(resource);
            assert!(min < max, "Invalid range for {:?}", resource);
            assert!(min > 0, "Min should be positive for {:?}", resource);
        }
    }
}
