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
            // A river is a river along its length, not in five places. At five
            // areas the generator put six or seven reaches of fish on three
            // hundred and seventy-odd water tiles, which is too thin for
            // anybody to build a living on, and two of every three nodes were
            // lost anyway to offsets landing on dry ground.
            fish_areas: 14,

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
            // On the flats where a shallow sea dried up, and in rare seams in
            // the hills for a people with no coast at all
            ResourceType::Salt => vec![
                TerrainType::SaltFlat,
                TerrainType::Mountain,
                TerrainType::Hills,
            ],

            // Agricultural
            ResourceType::Grain => vec![TerrainType::Plains, TerrainType::Meadow],
            ResourceType::Flax => vec![TerrainType::Riverbank, TerrainType::Wetland, TerrainType::Meadow],
            ResourceType::Herbs => vec![TerrainType::Forest, TerrainType::Meadow],
            ResourceType::Cotton => vec![TerrainType::Plains], // Prefers warm, dry areas

            // Animal-derived (from fauna, not terrain-spawned)
            ResourceType::Hides | ResourceType::Wool | ResourceType::Meat | ResourceType::Milk => vec![],

            // Gatherable
            ResourceType::Fish => vec![
                TerrainType::Water,
                TerrainType::Beach,
                TerrainType::Riverbank,
                TerrainType::Sea,
            ],
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

            // A wild hedge, not an orchard.
            //
            // This was (20, 60), and a settlement with the crudest tools in
            // the model buried **four years' eating** out of it - see
            // ISSUES_FOUND #43. Wild berries feed a few people for a few
            // weeks; that is the whole reason a people farms, and a bush that
            // carries sixty is a bush nobody would ever break ground for.
            ResourceType::Food => (8, 24),

            // Minerals
            ResourceType::Clay => (40, 120),
            ResourceType::Sand => (60, 180),
            ResourceType::Coal => (25, 80),

            // Agricultural. Wild grain is thin stuff and the comment beside
            // its regrowth rate already said so; the standing crop did not.
            ResourceType::Grain => (12, 32),
            ResourceType::Flax => (20, 50),
            ResourceType::Herbs => (15, 40),
            ResourceType::Cotton => (25, 60),

            // Gatherable
            ResourceType::Fish => (40, 100),
            ResourceType::Honey => (4, 12),

            // Wild leaf, shoot and the first roots. Thin stuff: a person
            // living on greens has to pick a great many of them, which is
            // why there are more patches of them than there are bushes.
            // Spring is not a lean season, it is the opposite: everything
            // green is putting out leaf at once and there is far more of it
            // than a small band can eat. Thin stuff by weight - greens carry
            // a sixth of the energy of ordinary forage - so it takes a great
            // deal of picking, and that is the cost of living on it rather
            // than scarcity.
            ResourceType::Greens => (40, 110),
            ResourceType::Roots => (30, 80),

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

/// A patch of the size the ground it stands on will carry, on the day the
/// world opens.
///
/// Free-standing, because there are **two** resource spawners in this project
/// and they had two vocabularies. `TerrainResourceMapper::amount_range` covers
/// the clustered minerals and crops; the basic spawner in `World` had its own
/// hard-coded ranges for wood, stone, food, greens and roots. Thinning the
/// hedgerows in one of them and measuring the result was measuring nothing:
/// berries came out of the other one and a world still held 994 units of them
/// against 1,000 before. That is the fourth time this project has been bitten
/// by two copies of one vocabulary. See ISSUES_FOUND #57.
///
/// `today` is the day of the year the world starts on, and it decides whether
/// there is anything on the plant at all. A world used to be made with every
/// bush in full fruit whatever the date: measured over sixteen spring worlds,
/// **216 units of fruit, 254 of grain and 34 of honey** that had no business
/// being there in spring, which then fell off over the first ten days. About
/// a day and a third of food for twelve people, handed to them free in
/// exactly the fortnight half of them die in.
///
/// A patch that *is* in season starts at what its ground will carry, which is
/// what it always did. There is no ramp across the window, and that is worth
/// being explicit about because the two halves behave quite differently:
/// measured by stripping a patch bare and waiting, **a fruit node is back to
/// full in one day, and greens and roots are still short after thirty**. So
/// for fruit the opening amount hardly matters, and for greens and roots it
/// matters a great deal - which is exactly why they are seeded at capacity
/// rather than at some fraction of it. They bear from the first day of the
/// year, so there is no coming-into-season moment for them to ramp through.
pub fn what_this_ground_carries(
    grid: &Grid,
    resource_type: ResourceType,
    pos: Position,
    at_its_best: u32,
    today: u32,
) -> ResourceNode {
    let mut node = ResourceNode::new(resource_type, pos, at_its_best);

    // Out of its season it carries nothing, whatever it is. This is asked
    // before `is_it_grown` because honey is not a growing thing and has a
    // season all the same - a hive worth robbing in autumn is not one in
    // March, and it used to spawn full in March.
    if !resource_type.is_it_bearing(today) {
        node.amount = 0;
        return node;
    }

    if !resource_type.is_it_grown() {
        return node;
    }

    let fertility = grid
        .get_tile(&pos)
        .map(|tile| tile.soil.fertility())
        .unwrap_or(0.5);

    node.amount = node.standing_capacity(fertility).max(1);
    node
}

/// Spawns resources in naturalistic clusters
pub struct NaturalisticSpawner<'a> {
    grid: &'a Grid,
    rng: rand::rngs::StdRng,
    /// The day of the year the world opens on, which decides what is standing
    /// on the plants when anybody first walks past them.
    today: u32,
}

impl<'a> NaturalisticSpawner<'a> {
    pub fn new(grid: &'a Grid, today: u32) -> Self {
        Self {
            grid,
            rng: crate::core::dice::roll(),
            today,
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
                        Some(center_pos.clone())
                    } else {
                        self.somewhere_near(&center_pos, cluster_radius, &preferred_terrains)
                    };

                    let Some(pos) = pos else {
                        continue;
                    };

                    let (min_amount, max_amount) = TerrainResourceMapper::amount_range(resource_type);
                    let amount = self.rng.gen_range(min_amount..=max_amount);
                    nodes.push(self.what_this_ground_carries(resource_type, pos, amount));
                }
            }
        }

        nodes
    }

    /// A patch of the size the ground it stands on will carry.
    ///
    /// The amount range is what the *kind* of thing carries at its best;
    /// what a particular bush carries is that, on the fertility of the tile
    /// it is rooted in. `regenerate_in_ground` has always capped regrowth
    /// this way - see `ResourceNode::how_heavy_a_crop_it_carries` - and the
    /// crop a world *started* with ignored the soil entirely, so a hedge on
    /// exhausted ground came up as heavy as one on a river meadow and then
    /// shrank towards its real capacity over the following season.
    ///
    /// Only growing things. A seam of clay does not care how rich the topsoil
    /// over it is.
    fn what_this_ground_carries(
        &self,
        resource_type: ResourceType,
        pos: Position,
        at_its_best: u32,
    ) -> ResourceNode {
        what_this_ground_carries(self.grid, resource_type, pos, at_its_best, self.today)
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
    /// Somewhere within the cluster radius of a centre that this resource
    /// will actually sit on.
    ///
    /// One throw of the dice was not enough. Clay wants wetland or riverbank,
    /// which is ribbon-shaped terrain a couple of tiles wide - so a single
    /// offset of five in each direction usually lands on dry ground, and the
    /// node was silently dropped. A cluster of three routinely came out as
    /// one lone node: asked for five clusters of three, a world produced
    /// **5.8 nodes**, and a quarter of worlds had no two clay nodes within
    /// twenty paces of each other. Whatever else a cluster is, it is more
    /// than one thing.
    ///
    /// Bounded, and it still gives up: a centre found at the very tip of a
    /// spit may genuinely have nothing else near it, and inventing ground for
    /// it would be worse than a small cluster.
    fn somewhere_near(
        &mut self,
        center: &Position,
        radius: i32,
        preferred_terrains: &[TerrainType],
    ) -> Option<Position> {
        for _ in 0..Self::HOW_MANY_PLACES_ANYBODY_TRIES {
            let pos = self.offset_position(center, radius);
            if self.is_valid_resource_position(&pos, preferred_terrains) {
                return Some(pos);
            }
        }

        None
    }

    /// How many throws before a cluster gives up on its next node.
    const HOW_MANY_PLACES_ANYBODY_TRIES: u32 = 24;

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



}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::seasons::{first_day_of, PartOfSeason, Season};

    fn some_ground() -> Grid {
        let mut grid = Grid::new(40, 40);
        grid.generate_terrain();
        grid.settle_soil();
        grid
    }

    /// A world used to be made with every bush in full fruit whatever the
    /// date. Measured over sixteen spring worlds: 216 units of fruit, 254 of
    /// grain and 34 of honey standing on day nought, all of which fell off
    /// over the following ten days.
    #[test]
    fn nothing_is_standing_on_a_plant_out_of_its_season() {
        let grid = some_ground();
        let pos = Position::new(20, 20);

        let deep_spring = first_day_of(Season::Spring, PartOfSeason::Deep);

        for out_of_season in [
            ResourceType::Food,
            ResourceType::Grain,
            ResourceType::Honey,
        ] {
            let node = what_this_ground_carries(&grid, out_of_season, pos.clone(), 60, deep_spring);
            assert_eq!(
                node.amount, 0,
                "{out_of_season:?} does not bear in spring, so there is nothing on it"
            );
        }
    }

    /// Honey is the one that used to slip past: it is not a *growing* thing,
    /// so the early return for stone and clay caught it, and a hive worth
    /// robbing in autumn spawned full in March.
    #[test]
    fn honey_has_a_season_even_though_it_does_not_grow() {
        let grid = some_ground();
        let pos = Position::new(20, 20);

        assert!(!ResourceType::Honey.is_it_grown());

        let march = first_day_of(Season::Spring, PartOfSeason::Deep);
        let midsummer = first_day_of(Season::Summer, PartOfSeason::Deep);

        assert_eq!(
            what_this_ground_carries(&grid, ResourceType::Honey, pos.clone(), 60, march).amount,
            0
        );
        assert!(
            what_this_ground_carries(&grid, ResourceType::Honey, pos, 60, midsummer).amount > 0,
            "and there is a hive worth robbing at midsummer"
        );
    }

    /// In its own season a patch carries what the ground it stands on will
    /// carry, which is what it always did.
    #[test]
    fn a_patch_in_season_carries_what_its_ground_will_carry() {
        let grid = some_ground();
        let pos = Position::new(20, 20);

        for (what, when) in [
            (ResourceType::Greens, (Season::Spring, PartOfSeason::Early)),
            (ResourceType::Roots, (Season::Spring, PartOfSeason::Early)),
            (ResourceType::Food, (Season::Fall, PartOfSeason::Early)),
            (ResourceType::Grain, (Season::Fall, PartOfSeason::Early)),
        ] {
            let day = first_day_of(when.0, when.1);
            let node = what_this_ground_carries(&grid, what, pos.clone(), 60, day);
            assert!(
                node.amount > 0,
                "{what:?} bears on day {day} and should have something on it"
            );
            assert!(
                node.amount <= node.max_amount,
                "and no more than the ground will carry"
            );
        }
    }

    /// A seam of clay does not care what month it is, and nor does a tree.
    #[test]
    fn what_never_bears_is_the_same_on_every_day_of_the_year() {
        let grid = some_ground();
        let pos = Position::new(20, 20);

        for what in [
            ResourceType::Stone,
            ResourceType::Clay,
            ResourceType::Iron,
            ResourceType::Wood,
        ] {
            let through_the_year: Vec<u32> = [
                (Season::Spring, PartOfSeason::Early),
                (Season::Summer, PartOfSeason::Deep),
                (Season::Fall, PartOfSeason::Late),
                (Season::Winter, PartOfSeason::Deep),
            ]
            .into_iter()
            .map(|(season, part)| {
                what_this_ground_carries(&grid, what, pos.clone(), 60, first_day_of(season, part))
                    .amount
            })
            .collect();

            assert!(
                through_the_year.iter().all(|&n| n > 0),
                "{what:?} does not bear, so it cannot stop: {through_the_year:?}"
            );
            assert!(
                through_the_year.windows(2).all(|w| w[0] == w[1]),
                "{what:?} should be the same all year: {through_the_year:?}"
            );
        }
    }

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
