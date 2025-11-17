// src/world/mod.rs
//! Complete world simulation system with terrain, resources, buildings, and spatial management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// World size presets for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    /// Tiny world: 64x64x64 - for quick testing
    Tiny,
    /// Small world: 128x128x96 - for small simulations
    Small,
    /// Medium world: 256x256x128 - default balanced size
    Medium,
    /// Large world: 512x512x160 - for complex simulations
    Large,
    /// Huge world: 1024x1024x192 - for massive simulations
    Huge,
    /// Custom size
    Custom(i32, i32, i32),
}

impl WorldSize {
    /// Get the dimensions (width, depth, height) for this size
    pub fn dimensions(&self) -> (i32, i32, i32) {
        match self {
            WorldSize::Tiny => (64, 64, 64),
            WorldSize::Small => (128, 128, 96),
            WorldSize::Medium => (256, 256, 128),
            WorldSize::Large => (512, 512, 160),
            WorldSize::Huge => (1024, 1024, 192),
            WorldSize::Custom(w, d, h) => (*w, *d, *h),
        }
    }

    /// Validate that dimensions are reasonable
    pub fn is_valid(&self) -> bool {
        let (w, d, h) = self.dimensions();
        w > 0 && d > 0 && h > 0 && w <= 4096 && d <= 4096 && h <= 512
    }

    /// Get estimated memory usage in MB (rough estimate)
    pub fn estimated_memory_mb(&self) -> f32 {
        let (w, d, h) = self.dimensions();
        let blocks = w as f32 * d as f32 * h as f32;
        // Rough estimate: ~100 bytes per block on average (with sparse storage)
        blocks * 100.0 / (1024.0 * 1024.0)
    }
}

// Module declarations
pub mod terrain;
pub mod resources;
pub mod buildings;
pub mod inventory;
pub mod actions;
pub mod grid;
pub mod render;
pub mod production;
pub mod economy;
pub mod technology;
pub mod climate;
pub mod combat;
pub mod crafting;
pub mod spatial_planning;

// Re-exports
pub use terrain::{Terrain, TerrainType, Tile};
pub use resources::{Resource, ResourceType, ResourceNode};
pub use buildings::{Building, BuildingType, BuildingState};
pub use inventory::{Inventory, Item, ItemType};
pub use actions::{Action, ActionResult};
pub use grid::{Grid, Position};
pub use render::AsciiRenderer;
pub use production::{Recipe, Quality, ResourceRequirement, ProductionOutput};
pub use economy::{TradeOffer, Marketplace, MarketData, CompletedTrade, MarketStatistics};
pub use technology::{Technology, TechnologyTree, KnownTechnologies, TechEra, DiscoveryEvent};
pub use climate::{ClimateManager, terrain_to_biome};

use crate::agents::Population;
use crate::environment::{HeatSourceRegistry, AnimalManager, PlantManager};

/// Status of a heat source for smelting
#[derive(Debug, Clone)]
pub struct HeatSourceStatus {
    pub is_lit: bool,
    pub current_temperature: f32,
    pub fuel_remaining: f32,
    pub contents: Vec<(String, u32, u32, f32)>, // (material_id, quantity, heating_time, current_temp)
}

/// Complete world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub grid: Grid,
    pub resources: Vec<ResourceNode>,
    pub buildings: Vec<Building>,
    pub storehouse_inventory: Inventory,
    pub marketplace: Marketplace,
    #[serde(skip)]
    pub tech_tree: TechnologyTree, // Global technology tree (not serialized, recreated)
    pub climate: ClimateManager,
    pub heat_sources: HeatSourceRegistry,
    pub animals: AnimalManager,
    pub plants: PlantManager,
    #[serde(skip)]
    pub combat_manager: combat::CombatManager, // Combat system (not serialized)
    #[serde(skip)]
    pub crafting_manager: crafting::CraftingManager, // Crafting system (not serialized)
    pub tick: u32,
    pub config: WorldConfig, // Store configuration for spatial planning
    pub resource_nodes: std::collections::HashMap<String, Vec<(i32, i32, i32)>>, // Resource locations by type (as tuples)
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

impl WorldConfig {
    /// Set world size
    pub fn with_size(mut self, width: usize, height: usize) -> Self {
        self.size = (width, height);
        self
    }

    /// Set resource configuration
    pub fn with_resources(mut self, resources: ResourceConfig) -> Self {
        self.initial_resources = resources;
        self
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        let (width, height) = self.size;

        // Check for zero dimensions
        if width == 0 {
            return Err("World width must be greater than 0".to_string());
        }
        if height == 0 {
            return Err("World height must be greater than 0".to_string());
        }

        // Check minimum size (must be large enough for agents to move)
        const MIN_SIZE: usize = 10;
        if width < MIN_SIZE || height < MIN_SIZE {
            return Err(format!("World dimensions must be at least {}x{} (minimum playable size)", MIN_SIZE, MIN_SIZE));
        }

        // Check maximum size (prevent memory issues)
        const MAX_SIZE: usize = 2000;
        if width > MAX_SIZE || height > MAX_SIZE {
            return Err(format!("World dimensions must not exceed {}x{} (maximum supported size)", MAX_SIZE, MAX_SIZE));
        }

        // Validate resource counts don't exceed world tiles
        let total_tiles = width * height;
        let total_resources = self.initial_resources.wood_nodes
            + self.initial_resources.stone_nodes
            + self.initial_resources.iron_nodes
            + self.initial_resources.food_nodes;

        if total_resources > total_tiles {
            return Err(format!(
                "Total resource nodes ({}) exceeds world tiles ({})",
                total_resources, total_tiles
            ));
        }

        Ok(())
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
            marketplace: Marketplace::new(),
            tech_tree: TechnologyTree::new(),
            climate: ClimateManager::default(),
            heat_sources: HeatSourceRegistry::new(),
            animals: AnimalManager::new(1000), // Max 1000 animals
            plants: PlantManager::new(5000), // Max 5000 plants
            combat_manager: combat::CombatManager::new(),
            crafting_manager: crafting::CraftingManager::new(),
            tick: 0,
            config: config.clone(),
            resource_nodes: std::collections::HashMap::new(),
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

    // ===== Heat Source Management =====

    /// Build a new heat source at a position
    pub fn build_heat_source(
        &mut self,
        heat_source_type: crate::environment::HeatSourceType,
        position: (i32, i32, i32),
        builder_id: Option<uuid::Uuid>,
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        let (x, y, z) = position;
        if x < 0 || y < 0 || x >= self.grid.width as i32 || y >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        // Check if there's already a heat source at this position
        if self.heat_sources.get_at_position(position).is_some() {
            return Err("Heat source already exists at this position".to_string());
        }

        // Create the heat source
        let mut heat_source = crate::environment::HeatSource::new(
            heat_source_type,
            position,
            self.tick as u64,
        );

        if let Some(builder) = builder_id {
            heat_source = heat_source.with_builder(builder);
        }

        let id = heat_source.id;
        self.heat_sources.add(heat_source);

        Ok(id)
    }

    /// Add fuel to a heat source
    pub fn add_fuel_to_heat_source(
        &mut self,
        heat_source_id: &uuid::Uuid,
        material_id: String,
        amount: f32,
    ) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            // Default burn time based on material (could be expanded)
            let burn_time = match material_id.as_str() {
                "wood" => 100,
                "charcoal" => 200,
                "coal" => 300,
                _ => 50,
            };

            heat_source.add_fuel(material_id, amount, burn_time);
            Ok(())
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Light a heat source
    pub fn light_heat_source(&mut self, heat_source_id: &uuid::Uuid) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            if heat_source.light() {
                Ok(())
            } else {
                Err("Cannot light heat source (no fuel)".to_string())
            }
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Extinguish a heat source
    pub fn extinguish_heat_source(&mut self, heat_source_id: &uuid::Uuid) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            heat_source.extinguish();
            Ok(())
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Add materials to heat/smelt
    pub fn add_to_heat_source(
        &mut self,
        heat_source_id: &uuid::Uuid,
        material_id: String,
        quantity: u32,
    ) -> Result<(), String> {
        if let Some(heat_source) = self.heat_sources.get_mut(heat_source_id) {
            heat_source.add_contents(material_id, quantity);
            Ok(())
        } else {
            Err("Heat source not found".to_string())
        }
    }

    /// Get heat source at position (2D, assumes z=0)
    pub fn get_heat_source_at(&self, x: i32, y: i32) -> Option<&crate::environment::HeatSource> {
        self.heat_sources.get_at_position((x, y, 0))
    }

    /// Get all heat sources within range of a position
    pub fn get_heat_sources_in_range(
        &self,
        position: (i32, i32, i32),
        range: f32,
    ) -> Vec<&crate::environment::HeatSource> {
        self.heat_sources.in_range(position, range)
    }

    /// Get temperature contribution from nearby heat sources
    pub fn environmental_temperature(&self, position: (i32, i32, i32), range: f32) -> f32 {
        let nearby_sources = self.get_heat_sources_in_range(position, range);

        let mut total_heat_contribution = 0.0;

        for source in nearby_sources {
            if source.is_lit {
                let dx = (source.position.0 - position.0) as f32;
                let dy = (source.position.1 - position.1) as f32;
                let dz = (source.position.2 - position.2) as f32;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(1.0);

                // Heat contribution falls off with distance
                let contribution = (source.current_temperature - 20.0) / distance;
                total_heat_contribution += contribution;
            }
        }

        // Base environmental temp + heat contribution
        20.0 + total_heat_contribution
    }

    // ===== Animal Management =====

    /// Spawn a wild animal at a position
    pub fn spawn_animal(
        &mut self,
        species_id: String,
        position: (i32, i32),
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        if position.0 < 0 || position.1 < 0 ||
           position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.animals.spawn_animal(species_id, position)
            .ok_or_else(|| "Failed to spawn animal (max population reached or invalid species)".to_string())
    }

    /// Spawn a group/herd of animals
    pub fn spawn_animal_group(
        &mut self,
        species_id: String,
        center: (i32, i32),
        count: u32,
    ) -> Result<uuid::Uuid, String> {
        // Check if center position is valid
        if center.0 < 0 || center.1 < 0 ||
           center.0 >= self.grid.width as i32 || center.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.animals.spawn_group(species_id, center, count)
            .ok_or_else(|| "Failed to spawn animal group".to_string())
    }

    /// Get animals within radius of a position
    pub fn get_animals_in_radius(
        &self,
        center: (i32, i32),
        radius: f32,
    ) -> Vec<&crate::environment::Animal> {
        self.animals.get_in_radius(center, radius)
    }

    /// Get animals at a specific position
    pub fn get_animals_at(&self, position: (i32, i32)) -> Vec<&crate::environment::Animal> {
        self.animals.get_at_position(position)
    }

    /// Tame an animal (increase tame level)
    pub fn tame_animal(&mut self, animal_id: &uuid::Uuid, amount: f32) -> Result<(), String> {
        if let Some(animal) = self.animals.get_mut(animal_id) {
            animal.tame(amount);
            Ok(())
        } else {
            Err("Animal not found".to_string())
        }
    }

    /// Feed an animal (restores stamina and health)
    pub fn feed_animal(&mut self, animal_id: &uuid::Uuid, amount: f32) -> Result<(), String> {
        if let Some(animal) = self.animals.get_mut(animal_id) {
            animal.stamina = (animal.stamina + amount).min(animal.max_stamina);
            animal.heal(amount * 0.5); // Restore some health too
            Ok(())
        } else {
            Err("Animal not found".to_string())
        }
    }

    /// Damage an animal
    pub fn damage_animal(&mut self, animal_id: &uuid::Uuid, damage: f32) -> Result<bool, String> {
        if let Some(animal) = self.animals.get_mut(animal_id) {
            animal.take_damage(damage);
            let is_dead = !animal.is_alive();
            Ok(is_dead)
        } else {
            Err("Animal not found".to_string())
        }
    }

    /// Get all animals of a specific species
    pub fn get_animals_by_species(&self, species_id: &str) -> Vec<&crate::environment::Animal> {
        self.animals.get_all()
            .iter()
            .filter(|a| a.species_id == species_id)
            .collect()
    }

    /// Get all domesticated animals
    pub fn get_domesticated_animals(&self) -> Vec<&crate::environment::Animal> {
        self.animals.get_all()
            .iter()
            .filter(|a| a.is_domesticated)
            .collect()
    }

    // ===== Plant Management =====

    /// Plant a crop at a position (cultivated)
    pub fn plant_crop(
        &mut self,
        species_id: String,
        position: (i32, i32),
        planter_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        if position.0 < 0 || position.1 < 0 ||
           position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.plants.plant_crop(species_id, position, planter_id)
            .ok_or_else(|| "Failed to plant crop (max population reached or invalid species)".to_string())
    }

    /// Spawn a wild plant at a position
    pub fn spawn_plant(
        &mut self,
        species_id: String,
        position: (i32, i32),
    ) -> Result<uuid::Uuid, String> {
        // Check if position is valid
        if position.0 < 0 || position.1 < 0 ||
           position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return Err("Position out of bounds".to_string());
        }

        self.plants.spawn_plant(species_id, position)
            .ok_or_else(|| "Failed to spawn plant (max population reached or invalid species)".to_string())
    }

    /// Spawn a patch of plants (forest, field, etc.)
    pub fn spawn_plant_patch(
        &mut self,
        species_id: String,
        center: (i32, i32),
        radius: u32,
        density: f32,
    ) -> Vec<uuid::Uuid> {
        self.plants.spawn_patch(species_id, center, radius, density)
    }

    /// Harvest a plant
    pub fn harvest_plant(
        &mut self,
        plant_id: &uuid::Uuid,
    ) -> Result<Vec<crate::environment::PlantDrop>, String> {
        self.plants.harvest_plant(plant_id)
            .ok_or_else(|| "Failed to harvest plant (not found or not harvestable)".to_string())
    }

    /// Get harvestable plants in radius
    pub fn get_harvestable_plants(
        &self,
        center: (i32, i32),
        radius: f32,
    ) -> Vec<&crate::environment::Plant> {
        self.plants.get_harvestable_in_radius(center, radius)
    }

    /// Get all plants in radius
    pub fn get_plants_in_radius(
        &self,
        center: (i32, i32),
        radius: f32,
    ) -> Vec<&crate::environment::Plant> {
        self.plants.get_in_radius(center, radius)
    }

    /// Get plants at a specific position
    pub fn get_plants_at(&self, position: (i32, i32)) -> Vec<&crate::environment::Plant> {
        self.plants.get_at_position(position)
    }

    /// Get all plants of a specific species
    pub fn get_plants_by_species(&self, species_id: &str) -> Vec<&crate::environment::Plant> {
        self.plants.all_plants()
            .iter()
            .filter(|p| p.species_id == species_id)
            .collect()
    }

    /// Get all cultivated plants
    pub fn get_cultivated_plants(&self) -> Vec<&crate::environment::Plant> {
        self.plants.all_plants()
            .iter()
            .filter(|p| p.is_cultivated)
            .collect()
    }

    // ===== Combat System =====

    /// Attack an animal (agent vs animal combat)
    pub fn agent_attack_animal(
        &mut self,
        agent_id: uuid::Uuid,
        agent_weapon_damage: f32,
        agent_mounted_bonus: f32,
        animal_id: &uuid::Uuid,
    ) -> Result<combat::CombatResult, String> {
        // Get animal stats
        let animal = self.animals.get(animal_id)
            .ok_or_else(|| "Animal not found".to_string())?;

        let animal_uuid = animal.id;
        // Use stamina-based defense approximation (higher stamina = better defense)
        let animal_armor = (animal.stamina / animal.max_stamina) * 0.2; // 0-20% defense

        // Create attacker stats (agent)
        let attacker_stats = combat::CombatStats {
            base_damage: 5.0,
            weapon_damage: agent_weapon_damage,
            mounted_bonus: agent_mounted_bonus,
            ..Default::default()
        };

        // Create defender stats (animal)
        let defender_stats = combat::CombatStats {
            base_damage: 0.0, // Not attacking
            armor_rating: animal_armor, // Natural armor
            ..Default::default()
        };

        // Execute combat
        let mut result = self.combat_manager.execute_combat(
            agent_id,
            animal_uuid,
            &attacker_stats,
            &defender_stats,
            Some("weapon".to_string()),
        );

        // Apply damage to animal
        let is_dead = self.damage_animal(&animal_uuid, result.damage_dealt)?;
        result.defender_killed = is_dead;

        Ok(result)
    }

    /// Animal attacks agent
    pub fn animal_attack_agent(
        &mut self,
        animal_id: &uuid::Uuid,
        agent_id: uuid::Uuid,
        agent_armor: f32,
    ) -> Result<combat::CombatResult, String> {
        // Get animal stats
        let animal = self.animals.get(animal_id)
            .ok_or_else(|| "Animal not found".to_string())?;

        // Base attack based on animal max health (larger animals hit harder)
        let animal_damage = (animal.max_health / 20.0).min(20.0); // 5-20 damage range
        let animal_uuid = animal.id;
        let species_name = animal.species_id.clone();

        // Create attacker stats (animal)
        let attacker_stats = combat::CombatStats {
            base_damage: animal_damage,
            weapon_damage: 0.0,
            ..Default::default()
        };

        // Create defender stats (agent)
        let defender_stats = combat::CombatStats {
            base_damage: 0.0,
            armor_rating: agent_armor,
            ..Default::default()
        };

        // Execute combat
        let result = self.combat_manager.execute_combat(
            animal_uuid,
            agent_id,
            &attacker_stats,
            &defender_stats,
            Some(format!("{} attack", species_name)),
        );

        // Note: Damage to agent must be applied by caller
        Ok(result)
    }

    /// Agent vs agent combat
    pub fn agent_attack_agent(
        &mut self,
        attacker_id: uuid::Uuid,
        defender_id: uuid::Uuid,
        attacker_weapon_damage: f32,
        attacker_armor: f32,
        attacker_mounted_bonus: f32,
        defender_weapon_damage: f32,
        defender_armor: f32,
        defender_mounted_bonus: f32,
    ) -> Result<combat::CombatResult, String> {
        // Create attacker stats
        let attacker_stats = combat::CombatStats {
            base_damage: 5.0,
            weapon_damage: attacker_weapon_damage,
            armor_rating: attacker_armor,
            mounted_bonus: attacker_mounted_bonus,
            ..Default::default()
        };

        // Create defender stats
        let defender_stats = combat::CombatStats {
            base_damage: 5.0,
            weapon_damage: defender_weapon_damage,
            armor_rating: defender_armor,
            mounted_bonus: defender_mounted_bonus,
            ..Default::default()
        };

        // Execute combat
        let result = self.combat_manager.execute_combat(
            attacker_id,
            defender_id,
            &attacker_stats,
            &defender_stats,
            Some("weapon".to_string()),
        );

        // Note: Damage must be applied by caller to both agents
        Ok(result)
    }

    /// Animal vs animal combat
    pub fn animal_attack_animal(
        &mut self,
        attacker_id: &uuid::Uuid,
        defender_id: &uuid::Uuid,
    ) -> Result<combat::CombatResult, String> {
        // Get both animals
        let attacker = self.animals.get(attacker_id)
            .ok_or_else(|| "Attacker animal not found".to_string())?;
        let defender = self.animals.get(defender_id)
            .ok_or_else(|| "Defender animal not found".to_string())?;

        let attacker_uuid = attacker.id;
        let attacker_damage = (attacker.max_health / 20.0).min(20.0); // Based on size
        let attacker_defense = (attacker.stamina / attacker.max_stamina) * 0.2;

        let defender_uuid = defender.id;
        let defender_damage = (defender.max_health / 20.0).min(20.0);
        let defender_defense = (defender.stamina / defender.max_stamina) * 0.2;

        // Create stats
        let attacker_stats = combat::CombatStats {
            base_damage: attacker_damage,
            armor_rating: attacker_defense,
            ..Default::default()
        };

        let defender_stats = combat::CombatStats {
            base_damage: defender_damage,
            armor_rating: defender_defense,
            ..Default::default()
        };

        // Execute combat
        let mut result = self.combat_manager.execute_combat(
            attacker_uuid,
            defender_uuid,
            &attacker_stats,
            &defender_stats,
            None,
        );

        // Apply damage to defender
        let is_dead = self.damage_animal(&defender_uuid, result.damage_dealt)?;
        result.defender_killed = is_dead;

        Ok(result)
    }

    /// Get combat statistics for an entity
    pub fn get_combat_stats(&self, entity_id: &uuid::Uuid) -> combat::CombatStatistics {
        self.combat_manager.get_combat_stats(entity_id)
    }

    /// Get recent combat log
    pub fn get_recent_combat(&self, count: usize) -> Vec<&combat::CombatResult> {
        self.combat_manager.get_recent_combat(count)
    }

    // ===== Crafting System =====

    /// Get a crafting recipe
    pub fn get_recipe(&self, recipe_id: &str) -> Option<&crafting::CraftingRecipe> {
        self.crafting_manager.get_recipe(recipe_id)
    }

    /// Get all recipes in a category
    pub fn get_recipes_by_category(&self, category: crafting::CraftingCategory) -> Vec<&crafting::CraftingRecipe> {
        self.crafting_manager.get_recipes_by_category(category)
    }

    /// Get all available recipes
    pub fn get_all_recipes(&self) -> Vec<&crafting::CraftingRecipe> {
        self.crafting_manager.all_recipes()
    }

    /// Attempt to craft an item (checks materials, skills, tools)
    pub fn attempt_craft(
        &mut self,
        recipe_id: &str,
        crafter_id: uuid::Uuid,
        inventory: &mut super::agents::agent::Inventory,
        skills: &HashMap<String, u32>,
        available_tools: &[crafting::ToolRequirement],
    ) -> crafting::CraftingResult {
        // Get inventory materials as HashMap
        let mut inventory_materials = HashMap::new();
        for (item_id, item) in inventory.get_all_items() {
            inventory_materials.insert(item_id.clone(), item.quantity);
        }

        // Check if can craft
        let check_result = self.crafting_manager.can_craft(
            recipe_id,
            &inventory_materials,
            skills,
            available_tools,
        );

        match check_result {
            crafting::CraftingResult::Success { item_id, quantity } => {
                // Get the recipe to consume materials
                if let Some(recipe) = self.crafting_manager.get_recipe(recipe_id) {
                    // Consume materials from inventory
                    for material in &recipe.materials {
                        inventory.remove_item(&material.material_id, material.quantity);
                    }

                    // Start crafting job
                    if let Some(_job_id) = self.crafting_manager.start_crafting(recipe_id.to_string(), crafter_id) {
                        crafting::CraftingResult::Success { item_id, quantity }
                    } else {
                        crafting::CraftingResult::RecipeNotFound
                    }
                } else {
                    crafting::CraftingResult::RecipeNotFound
                }
            }
            other => other,
        }
    }

    /// Check if an agent can craft a recipe (without consuming materials)
    pub fn can_craft_recipe(
        &self,
        recipe_id: &str,
        inventory: &super::agents::agent::Inventory,
        skills: &HashMap<String, u32>,
        available_tools: &[crafting::ToolRequirement],
    ) -> crafting::CraftingResult {
        // Get inventory materials as HashMap
        let mut inventory_materials = HashMap::new();
        for (item_id, item) in inventory.get_all_items() {
            inventory_materials.insert(item_id.clone(), item.quantity);
        }

        self.crafting_manager.can_craft(
            recipe_id,
            &inventory_materials,
            skills,
            available_tools,
        )
    }

    /// Get active crafting jobs for a crafter
    pub fn get_crafter_jobs(&self, crafter_id: &uuid::Uuid) -> Vec<&crafting::CraftingJob> {
        self.crafting_manager.get_crafter_jobs(crafter_id)
    }

    /// Cancel a crafting job
    pub fn cancel_crafting_job(&mut self, job_id: &uuid::Uuid) -> bool {
        self.crafting_manager.cancel_job(job_id)
    }

    // ===== Smelting System =====

    /// Get smelting recipes for a material
    pub fn get_smelting_recipes(&self, material_id: &str) -> Vec<&crate::environment::smelting::SmeltingRecipe> {
        self.heat_sources.get_smelting_recipes(material_id)
    }

    /// Check if a material can be smelted
    pub fn can_smelt_material(&self, material_id: &str) -> bool {
        self.heat_sources.can_smelt_material(material_id)
    }

    /// Get detailed status of smelting in a heat source
    pub fn get_smelting_status(&self, heat_source_id: &uuid::Uuid) -> Option<HeatSourceStatus> {
        if let Some(heat_source) = self.heat_sources.get(heat_source_id) {
            Some(HeatSourceStatus {
                is_lit: heat_source.is_lit,
                current_temperature: heat_source.current_temperature,
                fuel_remaining: heat_source.fuel.iter().map(|f| f.amount).sum(),
                contents: heat_source.contents.iter().map(|c| (
                    c.material_id.clone(),
                    c.quantity,
                    c.heating_time,
                    c.current_temp,
                )).collect(),
            })
        } else {
            None
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;

        // Update climate (weather, seasons, time)
        self.climate.tick();

        // Update buildings
        for building in &mut self.buildings {
            building.tick();
        }

        // Update heat sources (fuel consumption, heating)
        self.heat_sources.tick_all();

        // Update animals (AI, movement, aging)
        self.animals.tick();

        // Update plants (growth, regrowth)
        self.plants.tick();

        // Regenerate resources based on climate conditions (every 10 ticks to reduce overhead)
        if self.tick % 10 == 0 {
            self.regenerate_resources();
        }

        // Update crafting jobs (progress crafting)
        let _completed_crafts = self.crafting_manager.tick();
        // Note: Completed items should be added to crafter inventories by caller

        // Remove depleted resources
        self.remove_depleted_resources();
    }

    /// Regenerate renewable resources based on climate and weather conditions
    fn regenerate_resources(&mut self) {
        let current_season = self.climate.current_season();
        let season_modifier = current_season.plant_growth_modifier();
        let precipitation = self.climate.weather.wetness_per_tick() * 100.0; // Scale to 0-1 range

        for resource in &mut self.resources {
            // Get temperature at resource position
            let terrain_type = self.grid.get_tile(&resource.position)
                .map(|t| t.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            let temperature = self.climate.get_temperature(resource.position, terrain_type);

            // Regenerate the resource
            let _regen_amount = resource.regenerate(temperature, precipitation, season_modifier);

            // Debug log significant regeneration
            // if regen_amount > 0 {
            //     debug!("Resource {:?} at ({}, {}) regenerated {} units",
            //         resource.resource_type, resource.position.x, resource.position.y, regen_amount);
            // }
        }
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
                // New resource types - not yet tracked in stats
                _ => {}
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

    /// Process exploration for an agent at a position
    /// Returns number of new tiles discovered
    pub fn process_exploration(
        &mut self,
        agent_exploration: &mut crate::agents::ExplorationKnowledge,
        agent_position: &Position,
        vision_range: u32,
        current_tick: u32,
    ) -> usize {
        let mut new_discoveries = 0;
        let range = vision_range as i32;

        // Explore all tiles in vision range
        for dx in -range..=range {
            for dy in -range..=range {
                // Check if within circular vision range (not square)
                if (dx * dx + dy * dy) as f32 > (range * range) as f32 {
                    continue;
                }

                let explore_pos = Position::new(
                    agent_position.x + dx,
                    agent_position.y + dy,
                );

                // Check if position is valid
                if !self.grid.is_valid_position(&explore_pos) {
                    continue;
                }

                // Mark tile as explored if new
                if agent_exploration.explore_tile(explore_pos, current_tick) {
                    new_discoveries += 1;

                    // Mark tile as globally explored
                    if let Some(tile) = self.grid.get_tile_mut(&explore_pos) {
                        tile.mark_explored();

                        // Discover terrain type
                        agent_exploration.encounter_terrain(
                            tile.terrain.terrain_type,
                            explore_pos,
                            current_tick,
                        );
                    }

                    // Check for resources at this position
                    for resource in &self.resources {
                        if resource.position == explore_pos && resource.amount > 0 {
                            agent_exploration.discover_resource(
                                explore_pos,
                                resource.resource_type,
                                current_tick,
                            );
                        }
                    }

                    // Check for buildings at this position
                    for building in &self.buildings {
                        if building.position == explore_pos {
                            agent_exploration.discover_building(
                                explore_pos,
                                building.building_type,
                                current_tick,
                            );
                        }
                    }
                }
            }
        }

        // Record milestone discoveries
        if new_discoveries >= 10 {
            agent_exploration.discoveries.push(crate::agents::Discovery {
                discovery_type: crate::agents::DiscoveryType::AreaExplored {
                    tiles_count: new_discoveries,
                },
                tick: current_tick,
                position: *agent_position,
            });
        }

        new_discoveries
    }

    /// Get total number of tiles in the world
    pub fn total_tiles(&self) -> usize {
        self.grid.width * self.grid.height
    }

    // ===== Helper Methods for Spatial Planning and Testing =====

    /// Place a resource node at a specific position (for testing and spatial planning)
    pub fn place_resource_node(&mut self, resource_type: &str, position: (i32, i32, i32)) {
        self.resource_nodes
            .entry(resource_type.to_string())
            .or_insert_with(Vec::new)
            .push(position);
    }

    /// Add a building at a specific position (for testing and spatial planning)
    pub fn add_building_at(&mut self, building_type: BuildingType, position: (i32, i32, i32)) {
        use crate::world::buildings::Building;
        let pos = Position::new(position.0, position.1);
        let building = Building::new(building_type, pos);
        self.buildings.push(building);
    }

    /// Check if terrain at position is passable
    pub fn is_terrain_passable(&self, position: (i32, i32, i32)) -> bool {
        // Check bounds
        if position.0 < 0 || position.1 < 0 {
            return false;
        }
        if position.0 >= self.grid.width as i32 || position.1 >= self.grid.height as i32 {
            return false;
        }

        // For now, all in-bounds terrain is passable
        // In the future, check for water, mountains, etc.
        true
    }

    /// Mark an area as impassable (for testing terrain constraints)
    pub fn set_terrain_impassable(&mut self, center: (i32, i32, i32), _radius: i32) {
        // This would modify terrain in the grid
        // For now, we'll just note this as a placeholder
        // The actual implementation would mark tiles in self.grid
        // as impassable terrain types
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
    /// World dimensions (width, depth, height)
    pub size: (i32, i32, i32),
    /// Size of chunks for spatial optimization
    pub chunk_size: u32,
}

impl GridConfig {
    /// Create a new grid configuration with custom size
    pub fn new(size: (i32, i32, i32)) -> Self {
        Self {
            size,
            chunk_size: 16,
        }
    }

    /// Create from a WorldSize preset
    pub fn from_world_size(world_size: WorldSize) -> Self {
        Self::new(world_size.dimensions())
    }

    /// Validate configuration
    pub fn is_valid(&self) -> bool {
        self.size.0 > 0
            && self.size.1 > 0
            && self.size.2 > 0
            && self.chunk_size > 0
            && self.chunk_size <= 64
    }

    /// Check if a position is within bounds
    pub fn is_in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        let (width, depth, height) = self.size;
        x >= -width/2 && x < width/2
            && z >= -depth/2 && z < depth/2
            && y >= 0 && y < height
    }

    /// Get total world volume
    pub fn volume(&self) -> i64 {
        self.size.0 as i64 * self.size.1 as i64 * self.size.2 as i64
    }
}

impl Default for GridConfig {
    fn default() -> Self {
        Self::from_world_size(WorldSize::Medium)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_size_presets() {
        assert_eq!(WorldSize::Tiny.dimensions(), (64, 64, 64));
        assert_eq!(WorldSize::Small.dimensions(), (128, 128, 96));
        assert_eq!(WorldSize::Medium.dimensions(), (256, 256, 128));
        assert_eq!(WorldSize::Large.dimensions(), (512, 512, 160));
        assert_eq!(WorldSize::Huge.dimensions(), (1024, 1024, 192));
    }

    #[test]
    fn test_custom_world_size() {
        let custom = WorldSize::Custom(100, 200, 300);
        assert_eq!(custom.dimensions(), (100, 200, 300));
    }

    #[test]
    fn test_world_size_validation() {
        assert!(WorldSize::Medium.is_valid());
        assert!(WorldSize::Custom(256, 256, 128).is_valid());
        assert!(!WorldSize::Custom(0, 256, 128).is_valid());
        assert!(!WorldSize::Custom(-10, 256, 128).is_valid());
        assert!(!WorldSize::Custom(10000, 256, 128).is_valid());
    }

    #[test]
    fn test_grid_config_creation() {
        let config = GridConfig::new((256, 256, 128));
        assert_eq!(config.size, (256, 256, 128));
        assert_eq!(config.chunk_size, 16);
    }

    #[test]
    fn test_grid_config_from_world_size() {
        let config = GridConfig::from_world_size(WorldSize::Large);
        assert_eq!(config.size, (512, 512, 160));
    }

    #[test]
    fn test_bounds_checking() {
        let config = GridConfig::new((100, 100, 64));

        // Within bounds (range is [-50, 50) for x and z, [0, 64) for y)
        assert!(config.is_in_bounds(0, 0, 0));
        assert!(config.is_in_bounds(49, 32, 49));
        assert!(config.is_in_bounds(-50, 63, -50)); // -50 is included (lower bound)
        assert!(config.is_in_bounds(-49, 0, -49));

        // Out of bounds
        assert!(!config.is_in_bounds(50, 0, 0));   // 50 is excluded (upper bound)
        assert!(!config.is_in_bounds(-51, 0, 0));  // -51 is out of bounds
        assert!(!config.is_in_bounds(0, -1, 0));   // negative y
        assert!(!config.is_in_bounds(0, 64, 0));   // y at max is excluded
    }

    #[test]
    fn test_world_volume() {
        let config = GridConfig::new((100, 100, 64));
        assert_eq!(config.volume(), 640000);
    }

    // TODO: Rewrite these tests for the current 2D World API
    // The World struct was refactored from 3D to 2D
    // #[test]
    // fn test_world_creation() {
    //     let world = World::new(WorldConfig::default());
    //     // Update assertions for 2D grid
    // }

    // #[test]
    // fn test_world_position_validation() {
    //     let world = World::new(WorldConfig::default());
    //     // Update assertions for 2D positions
    // }

    // #[test]
    // fn test_world_config() {
    //     let config = WorldConfig::default();
    //     assert_eq!(config.size, (50, 50));
    // }

    // #[test]
    // fn test_custom_world_config() {
    //     let config = WorldConfig {
    //         size: (300, 400),
    //         initial_resources: ResourceConfig::default(),
    //     };
    //     assert_eq!(config.size, (300, 400));
    // }

    #[test]
    fn test_position_distance() {
        // Position is now 2D (x, y)
        let p1 = Position::new(0, 0);
        let p2 = Position::new(3, 4);
        // distance_to uses Manhattan distance: |3-0| + |4-0| = 7
        assert_eq!(p1.distance_to(&p2), 7);
        // For Euclidean distance (sqrt(3^2 + 4^2) = 5.0):
        assert_eq!(p1.euclidean_distance_to(&p2), 5.0);
    }

    #[test]
    fn test_estimated_memory() {
        let tiny = WorldSize::Tiny;
        let huge = WorldSize::Huge;

        assert!(tiny.estimated_memory_mb() < huge.estimated_memory_mb());
        assert!(tiny.estimated_memory_mb() < 50.0); // Tiny should be small
    }
}

// External TDD test modules
#[cfg(test)]
mod tdd_tests;
