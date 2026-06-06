// src/environment/heat_source.rs
//! Heat source system for temperature-based smelting and metalworking.
//!
//! Different heat sources produce different temperatures:
//! - Campfire: 600-800°C (can melt lead, tin accidentally)
//! - Smelting Fire (with bellows): 1000-1200°C (copper, bronze)
//! - Bloomery: 1200-1400°C (reduce iron ore to bloom)
//! - Advanced Furnace: 1400-1600°C (melt iron)

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use super::smelting::{SmeltingRegistry, check_smelting, SmeltingCheck};

/// Type of heat source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeatSourceType {
    /// Basic campfire - 600-800°C
    Campfire,
    /// Campfire with air flow (bellows) - 800-1000°C
    BellowsFire,
    /// Dedicated smelting fire with bellows - 1000-1200°C
    SmeltingFire,
    /// Clay-lined pit for smelting - 1100-1300°C
    SmeltingPit,
    /// Stone bloomery for iron reduction - 1200-1400°C
    Bloomery,
    /// Advanced stone furnace - 1300-1500°C
    StoneFurnace,
    /// Clay/brick furnace - 1400-1600°C
    ClayFurnace,
    /// Advanced furnace with optimal airflow - 1500-1700°C
    AdvancedFurnace,
}

impl HeatSourceType {
    /// Get temperature range for this heat source (min, max) in °C
    pub fn temperature_range(&self) -> (f32, f32) {
        match self {
            HeatSourceType::Campfire => (600.0, 800.0),
            HeatSourceType::BellowsFire => (800.0, 1000.0),
            HeatSourceType::SmeltingFire => (1000.0, 1200.0),
            HeatSourceType::SmeltingPit => (1100.0, 1300.0),
            HeatSourceType::Bloomery => (1200.0, 1400.0),
            HeatSourceType::StoneFurnace => (1300.0, 1500.0),
            HeatSourceType::ClayFurnace => (1400.0, 1600.0),
            HeatSourceType::AdvancedFurnace => (1500.0, 1700.0),
        }
    }

    /// Get average temperature
    pub fn average_temperature(&self) -> f32 {
        let (min, max) = self.temperature_range();
        (min + max) / 2.0
    }

    /// Get construction materials required
    pub fn construction_materials(&self) -> Vec<(&'static str, u32)> {
        match self {
            HeatSourceType::Campfire => vec![("wood", 5)],
            HeatSourceType::BellowsFire => vec![("wood", 5), ("leather", 2)],
            HeatSourceType::SmeltingFire => vec![("wood", 10), ("leather", 3), ("stone", 5)],
            HeatSourceType::SmeltingPit => vec![("stone", 20), ("clay", 10)],
            HeatSourceType::Bloomery => vec![("stone", 50), ("clay", 20)],
            HeatSourceType::StoneFurnace => vec![("stone", 100), ("clay", 30)],
            HeatSourceType::ClayFurnace => vec![("clay", 80), ("stone", 40), ("brick", 20)],
            HeatSourceType::AdvancedFurnace => vec![("brick", 100), ("iron_ingot", 10), ("clay", 40)],
        }
    }

    /// Technology ID required to build this
    pub fn required_technology(&self) -> Option<&'static str> {
        match self {
            HeatSourceType::Campfire => None, // Basic knowledge
            HeatSourceType::BellowsFire => Some("bellows"),
            HeatSourceType::SmeltingFire => Some("intentional_smelting"),
            HeatSourceType::SmeltingPit => Some("pit_smelting"),
            HeatSourceType::Bloomery => Some("iron_smelting"),
            HeatSourceType::StoneFurnace => Some("advanced_furnace"),
            HeatSourceType::ClayFurnace => Some("clay_furnace"),
            HeatSourceType::AdvancedFurnace => Some("blast_furnace"),
        }
    }

    /// Fuel consumption rate (units per tick)
    pub fn fuel_consumption_rate(&self) -> f32 {
        match self {
            HeatSourceType::Campfire => 0.1,
            HeatSourceType::BellowsFire => 0.15,
            HeatSourceType::SmeltingFire => 0.2,
            HeatSourceType::SmeltingPit => 0.25,
            HeatSourceType::Bloomery => 0.3,
            HeatSourceType::StoneFurnace => 0.25,
            HeatSourceType::ClayFurnace => 0.2,
            HeatSourceType::AdvancedFurnace => 0.15, // More efficient
        }
    }
}

/// State of fuel in a heat source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelState {
    /// Material ID of fuel
    pub material_id: String,
    /// Amount remaining
    pub amount: f32,
    /// Burn time remaining (ticks)
    pub burn_time: u32,
}

/// Contents being heated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatingContents {
    /// Material ID
    pub material_id: String,
    /// Quantity
    pub quantity: u32,
    /// Time being heated (ticks)
    pub heating_time: u32,
    /// Current temperature reached
    pub current_temp: f32,
}

/// A heat source instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatSource {
    pub id: Uuid,
    pub heat_source_type: HeatSourceType,
    pub position: (i32, i32, i32),

    /// Is the fire currently lit?
    pub is_lit: bool,

    /// Current temperature (°C)
    pub current_temperature: f32,

    /// Fuel currently burning
    pub fuel: Vec<FuelState>,

    /// Materials being heated/smelted
    pub contents: Vec<HeatingContents>,

    /// Who built this
    pub builder: Option<Uuid>,

    /// When it was built
    pub built_at: u64,

    /// Total items smelted (for tracking)
    pub items_processed: u32,
}

impl HeatSource {
    pub fn new(heat_source_type: HeatSourceType, position: (i32, i32, i32), timestamp: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            heat_source_type,
            position,
            is_lit: false,
            current_temperature: 20.0, // Ambient temperature
            fuel: Vec::new(),
            contents: Vec::new(),
            builder: None,
            built_at: timestamp,
            items_processed: 0,
        }
    }

    pub fn with_builder(mut self, builder: Uuid) -> Self {
        self.builder = Some(builder);
        self
    }

    /// Add fuel to the heat source
    pub fn add_fuel(&mut self, material_id: String, amount: f32, burn_time: u32) {
        // Check if we already have this fuel type
        if let Some(existing) = self.fuel.iter_mut().find(|f| f.material_id == material_id) {
            existing.amount += amount;
            existing.burn_time += burn_time;
        } else {
            self.fuel.push(FuelState {
                material_id,
                amount,
                burn_time,
            });
        }
    }

    /// Add material to heat/smelt
    pub fn add_contents(&mut self, material_id: String, quantity: u32) {
        // Check if we already have this material
        if let Some(existing) = self.contents.iter_mut().find(|c| c.material_id == material_id) {
            existing.quantity += quantity;
        } else {
            self.contents.push(HeatingContents {
                material_id,
                quantity,
                heating_time: 0,
                current_temp: self.current_temperature,
            });
        }
    }

    /// Light the fire (requires fuel)
    pub fn light(&mut self) -> bool {
        if !self.fuel.is_empty() {
            self.is_lit = true;
            true
        } else {
            false // No fuel to light
        }
    }

    /// Extinguish the fire
    pub fn extinguish(&mut self) {
        self.is_lit = false;
    }

    /// Update heat source for one tick
    pub fn tick(&mut self, smelting_registry: &SmeltingRegistry) -> Vec<SmeltingResult> {
        let mut results = Vec::new();

        if !self.is_lit {
            // Temperature cools down
            self.current_temperature = (self.current_temperature - 10.0).max(20.0);
            return results;
        }

        // Consume fuel
        let consumption = self.heat_source_type.fuel_consumption_rate();
        let mut total_fuel = 0.0;

        self.fuel.retain_mut(|fuel_state| {
            if fuel_state.amount > consumption {
                fuel_state.amount -= consumption;
                total_fuel += consumption;
                true
            } else {
                total_fuel += fuel_state.amount;
                false // Fuel depleted
            }
        });

        if total_fuel == 0.0 {
            // Out of fuel, fire goes out
            self.is_lit = false;
            self.current_temperature = (self.current_temperature - 50.0).max(20.0);
            return results;
        }

        // Heat up to target temperature
        let (min_temp, max_temp) = self.heat_source_type.temperature_range();
        let target_temp = (min_temp + max_temp) / 2.0;

        if self.current_temperature < target_temp {
            // Heat up quickly at first, then slower as approaching target
            let diff = target_temp - self.current_temperature;
            let heat_rate = (diff * 0.1).max(5.0);
            self.current_temperature = (self.current_temperature + heat_rate).min(max_temp);
        }

        // Heat contents and check for smelting transformations
        let mut completed_smelts = Vec::new();
        for content in &mut self.contents {
            content.heating_time += 1;
            content.current_temp = self.current_temperature;

            // Check if material can be smelted
            match check_smelting(
                smelting_registry,
                &content.material_id,
                content.heating_time,
                content.current_temp,
            ) {
                SmeltingCheck::CanSmelt {
                    recipe_id: _,
                    output_material,
                    output_quantity,
                } => {
                    // Ready to smelt!
                    if let Some(recipe) = smelting_registry.get_by_input(&content.material_id).first() {
                        // Calculate how many we can smelt
                        let batches = content.quantity / recipe.input_quantity;
                        if batches > 0 {
                            let input_consumed = batches * recipe.input_quantity;
                            let output_produced = batches * output_quantity;

                            completed_smelts.push((
                                content.material_id.clone(),
                                input_consumed,
                                output_material.clone(),
                                output_produced,
                            ));

                            self.items_processed += output_produced;
                        }
                    }
                }
                _ => {
                    // Not ready yet, continue heating
                }
            }
        }

        // Process completed smelts
        for (input_mat, input_qty, output_mat, output_qty) in completed_smelts {
            self.remove_contents(&input_mat, input_qty);

            results.push(SmeltingResult {
                input_material: input_mat,
                output_material: output_mat,
                input_quantity: input_qty,
                output_quantity: output_qty,
                heat_source_id: self.id,
            });
        }

        results
    }

    /// Check if can smelt a specific material
    pub fn can_smelt(&self, melting_point: f32) -> bool {
        let (_, max_temp) = self.heat_source_type.temperature_range();
        self.is_lit && max_temp >= melting_point
    }

    /// Remove contents after smelting
    pub fn remove_contents(&mut self, material_id: &str, quantity: u32) -> Option<u32> {
        if let Some(content) = self.contents.iter_mut().find(|c| c.material_id == material_id) {
            if content.quantity >= quantity {
                content.quantity -= quantity;
                if content.quantity == 0 {
                    self.contents.retain(|c| c.material_id != material_id);
                }
                Some(quantity)
            } else {
                let available = content.quantity;
                self.contents.retain(|c| c.material_id != material_id);
                Some(available)
            }
        } else {
            None
        }
    }
}

/// Result of a smelting operation
#[derive(Debug, Clone)]
pub struct SmeltingResult {
    pub input_material: String,
    pub output_material: String,
    pub input_quantity: u32,
    pub output_quantity: u32,
    pub heat_source_id: Uuid,
}

/// Registry of all heat sources in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatSourceRegistry {
    heat_sources: HashMap<Uuid, HeatSource>,
    position_index: HashMap<(i32, i32, i32), Uuid>,
    #[serde(skip, default)]
    smelting_registry: SmeltingRegistry,
}

impl HeatSourceRegistry {
    pub fn new() -> Self {
        Self {
            heat_sources: HashMap::new(),
            position_index: HashMap::new(),
            smelting_registry: SmeltingRegistry::new(),
        }
    }

    /// Re-initialize the smelting registry after deserialization.
    ///
    /// This must be called after loading a saved HeatSourceRegistry to ensure
    /// the smelting registry is properly initialized.
    pub fn initialize_registry(&mut self) {
        self.smelting_registry = SmeltingRegistry::new();
    }

    /// Add a heat source
    pub fn add(&mut self, heat_source: HeatSource) {
        let id = heat_source.id;
        let pos = heat_source.position;

        self.position_index.insert(pos, id);
        self.heat_sources.insert(id, heat_source);
    }

    /// Get a heat source by ID
    pub fn get(&self, id: &Uuid) -> Option<&HeatSource> {
        self.heat_sources.get(id)
    }

    /// Get a mutable heat source by ID
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut HeatSource> {
        self.heat_sources.get_mut(id)
    }

    /// Get heat source at position
    pub fn get_at_position(&self, pos: (i32, i32, i32)) -> Option<&HeatSource> {
        self.position_index
            .get(&pos)
            .and_then(|id| self.heat_sources.get(id))
    }

    /// Get mutable heat source at position
    pub fn get_at_position_mut(&mut self, pos: (i32, i32, i32)) -> Option<&mut HeatSource> {
        if let Some(id) = self.position_index.get(&pos).copied() {
            self.heat_sources.get_mut(&id)
        } else {
            None
        }
    }

    /// Remove a heat source
    pub fn remove(&mut self, id: &Uuid) -> Option<HeatSource> {
        if let Some(heat_source) = self.heat_sources.remove(id) {
            self.position_index.remove(&heat_source.position);
            Some(heat_source)
        } else {
            None
        }
    }

    /// Get all heat sources
    pub fn all(&self) -> Vec<&HeatSource> {
        self.heat_sources.values().collect()
    }

    /// Get all lit heat sources
    pub fn all_lit(&self) -> Vec<&HeatSource> {
        self.heat_sources
            .values()
            .filter(|hs| hs.is_lit)
            .collect()
    }

    /// Find heat sources in range
    pub fn in_range(&self, center: (i32, i32, i32), range: f32) -> Vec<&HeatSource> {
        self.heat_sources
            .values()
            .filter(|hs| {
                let dx = (hs.position.0 - center.0) as f32;
                let dy = (hs.position.1 - center.1) as f32;
                let dz = (hs.position.2 - center.2) as f32;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                distance <= range
            })
            .collect()
    }

    /// Tick all heat sources
    pub fn tick_all(&mut self) -> Vec<SmeltingResult> {
        let mut all_results = Vec::new();

        for heat_source in self.heat_sources.values_mut() {
            let results = heat_source.tick(&self.smelting_registry);
            all_results.extend(results);
        }

        all_results
    }

    /// Get smelting recipes for a material
    pub fn get_smelting_recipes(&self, material_id: &str) -> Vec<&super::smelting::SmeltingRecipe> {
        self.smelting_registry.get_by_input(material_id)
    }

    /// Check if a material can be smelted
    pub fn can_smelt_material(&self, material_id: &str) -> bool {
        self.smelting_registry.can_smelt(material_id)
    }
}

impl Default for HeatSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heat_source_temperatures() {
        assert_eq!(HeatSourceType::Campfire.temperature_range(), (600.0, 800.0));
        assert_eq!(HeatSourceType::Bloomery.temperature_range(), (1200.0, 1400.0));
        assert_eq!(HeatSourceType::Campfire.average_temperature(), 700.0);
    }

    #[test]
    fn test_heat_source_creation() {
        let heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);

        assert_eq!(heat_source.heat_source_type, HeatSourceType::Campfire);
        assert!(!heat_source.is_lit);
        assert_eq!(heat_source.current_temperature, 20.0);
    }

    #[test]
    fn test_add_fuel() {
        let mut heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);

        heat_source.add_fuel("wood".to_string(), 10.0, 100);
        assert_eq!(heat_source.fuel.len(), 1);
        assert_eq!(heat_source.fuel[0].amount, 10.0);

        // Add more of same fuel
        heat_source.add_fuel("wood".to_string(), 5.0, 50);
        assert_eq!(heat_source.fuel.len(), 1);
        assert_eq!(heat_source.fuel[0].amount, 15.0);
    }

    #[test]
    fn test_lighting_fire() {
        let mut heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);

        // Cannot light without fuel
        assert!(!heat_source.light());

        // Add fuel and light
        heat_source.add_fuel("wood".to_string(), 10.0, 100);
        assert!(heat_source.light());
        assert!(heat_source.is_lit);
    }

    #[test]
    fn test_heat_source_heating() {
        let mut heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);
        heat_source.add_fuel("wood".to_string(), 10.0, 100);
        heat_source.light();

        let initial_temp = heat_source.current_temperature;

        // Tick to heat up
        let smelting_registry = crate::environment::smelting::SmeltingRegistry::new();
        heat_source.tick(&smelting_registry);

        assert!(heat_source.current_temperature > initial_temp);
    }

    #[test]
    fn test_can_smelt() {
        let mut heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);
        heat_source.add_fuel("wood".to_string(), 10.0, 100);
        heat_source.light();
        heat_source.current_temperature = 700.0;

        // Campfire can reach 800°C, can melt copper (1085°C)? No
        assert!(!heat_source.can_smelt(1085.0));

        // Can melt lead (327°C)? Yes
        assert!(heat_source.can_smelt(327.0));
    }

    #[test]
    fn test_heat_source_registry() {
        let mut registry = HeatSourceRegistry::new();

        let heat_source = HeatSource::new(HeatSourceType::Campfire, (10, 0, 5), 0);
        let id = heat_source.id;

        registry.add(heat_source);

        assert!(registry.get(&id).is_some());
        assert!(registry.get_at_position((10, 0, 5)).is_some());
    }

    #[test]
    fn test_find_heat_sources_in_range() {
        let mut registry = HeatSourceRegistry::new();

        registry.add(HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0));
        registry.add(HeatSource::new(HeatSourceType::Bloomery, (5, 0, 0), 0));
        registry.add(HeatSource::new(HeatSourceType::SmeltingFire, (100, 0, 0), 0));

        let nearby = registry.in_range((0, 0, 0), 10.0);
        assert_eq!(nearby.len(), 2); // First two are within range
    }

    #[test]
    fn test_fuel_consumption() {
        let mut heat_source = HeatSource::new(HeatSourceType::Campfire, (0, 0, 0), 0);
        heat_source.add_fuel("wood".to_string(), 1.0, 100);
        heat_source.light();

        let consumption_rate = HeatSourceType::Campfire.fuel_consumption_rate();
        let smelting_registry = crate::environment::smelting::SmeltingRegistry::new();

        // Tick several times
        for _ in 0..5 {
            heat_source.tick(&smelting_registry);
        }

        // Fuel should be consumed
        let remaining = heat_source.fuel.first().map(|f| f.amount).unwrap_or(0.0);
        assert!(remaining < 1.0);
        assert!(remaining > 1.0 - (consumption_rate * 6.0)); // ~5-6 ticks worth
    }
}
