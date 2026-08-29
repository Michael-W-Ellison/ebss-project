// src/agents/transport.rs
//! Transport system for carrying additional inventory via bags, packs, carts, and pack animals.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of transport container or vehicle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    // Worn containers
    /// Small bag worn on hip
    Pouch,
    /// Shoulder bag
    Satchel,
    /// Backpack
    Backpack,
    /// Large hiking backpack
    LargeBackpack,

    // Vehicles (pushed/pulled)
    /// Two poles and a hide, dragged
    Travois,
    /// Small handcart
    Handcart,
    /// Larger cart
    Cart,
    /// Full wagon
    Wagon,
    /// Winter sled
    Sled,

    // Pack animals (cargo only)
    /// Donkey with saddlebags
    PackDonkey,
    /// Horse with saddlebags
    PackHorse,
    /// Camel with cargo
    PackCamel,
    /// Mule with cargo
    PackMule,
    /// Ox pulling cart
    OxCart,

    // Rideable mounts
    /// Standard riding horse
    Horse,
    /// Heavy warhorse for combat
    Warhorse,
    /// Light pony for quick travel
    Pony,
    /// Desert camel for hot climates
    RidingCamel,
    /// Riding donkey - slow but reliable
    RidingDonkey,
    /// Riding mule - balanced mount
    RidingMule,
    /// Arctic reindeer for snow
    Reindeer,
    /// Forest elk
    Elk,
}

impl TransportType {
    /// Get the weight capacity bonus this transport provides (in kg)
    pub fn weight_capacity(&self) -> f32 {
        match self {
            // Worn containers
            TransportType::Pouch => 5.0,
            TransportType::Satchel => 15.0,
            TransportType::Backpack => 30.0,
            TransportType::LargeBackpack => 50.0,

            // Vehicles
            TransportType::Travois => 70.0,
            TransportType::Handcart => 140.0,
            TransportType::Cart => 150.0,
            TransportType::Wagon => 500.0,
            TransportType::Sled => 100.0,

            // Pack animals
            TransportType::PackDonkey => 100.0,
            TransportType::PackHorse => 150.0,
            TransportType::PackCamel => 200.0,
            TransportType::PackMule => 120.0,
            TransportType::OxCart => 600.0,

            // Rideable mounts (rider + small cargo)
            TransportType::Horse => 120.0,
            TransportType::Warhorse => 150.0,
            TransportType::Pony => 80.0,
            TransportType::RidingCamel => 140.0,
            TransportType::RidingDonkey => 90.0,
            TransportType::RidingMule => 110.0,
            TransportType::Reindeer => 100.0,
            TransportType::Elk => 110.0,
        }
    }


    /// Check if this transport is worn (backpack, pouch, etc.)
    pub fn is_wearable(&self) -> bool {
        matches!(self,
            TransportType::Pouch |
            TransportType::Satchel |
            TransportType::Backpack |
            TransportType::LargeBackpack
        )
    }

    /// Check if this is a vehicle (cart, wagon, etc.)
    pub fn is_vehicle(&self) -> bool {
        matches!(self,
            TransportType::Travois |
            TransportType::Handcart |
            TransportType::Cart |
            TransportType::Wagon |
            TransportType::Sled
        )
    }

    /// Check if this is a pack animal
    pub fn is_pack_animal(&self) -> bool {
        matches!(self,
            TransportType::PackDonkey |
            TransportType::PackHorse |
            TransportType::PackCamel |
            TransportType::PackMule |
            TransportType::OxCart
        )
    }

    /// Check if this is a rideable mount
    pub fn is_rideable(&self) -> bool {
        matches!(self,
            TransportType::Horse |
            TransportType::Warhorse |
            TransportType::Pony |
            TransportType::RidingCamel |
            TransportType::RidingDonkey |
            TransportType::RidingMule |
            TransportType::Reindeer |
            TransportType::Elk
        )
    }

    /// Get movement speed modifier (1.0 = normal, < 1.0 = slower, > 1.0 = faster for mounts)
    pub fn speed_modifier(&self) -> f32 {
        match self {
            // Worn containers (slight slowdown when full)
            TransportType::Pouch => 0.98,
            TransportType::Satchel => 0.95,
            TransportType::Backpack => 0.90,
            TransportType::LargeBackpack => 0.85,

            // Vehicles (significant slowdown)
            // A travois is dragged rather than rolled, so it costs more of the
            // walking than a cart does and carries less. That is the whole
            // difference between them and the reason one comes first.
            TransportType::Travois => 0.80,
            TransportType::Handcart => 0.70,
            TransportType::Cart => 0.60,
            TransportType::Wagon => 0.50,
            TransportType::Sled => 0.65,

            // Pack animals (moderate slowdown)
            TransportType::PackDonkey => 0.75,
            TransportType::PackHorse => 0.80,
            TransportType::PackCamel => 0.70,
            TransportType::PackMule => 0.75,
            TransportType::OxCart => 0.45,

            // Rideable mounts (speed boost!)
            TransportType::Horse => 1.8,
            TransportType::Warhorse => 1.5,
            TransportType::Pony => 1.6,
            TransportType::RidingCamel => 1.4,
            TransportType::RidingDonkey => 1.2,
            TransportType::RidingMule => 1.4,
            TransportType::Reindeer => 1.7,
            TransportType::Elk => 1.6,
        }
    }

    /// Check if requires animal to operate
    pub fn requires_animal(&self) -> bool {
        self.is_pack_animal() || self.is_rideable()
    }

    /// Get durability (how many uses before breaking, 0 = doesn't break)
    pub fn durability(&self) -> u32 {
        match self {
            // Worn containers
            TransportType::Pouch => 500,
            TransportType::Satchel => 1000,
            TransportType::Backpack => 2000,
            TransportType::LargeBackpack => 2500,

            // Vehicles
            TransportType::Travois => 1200,
            TransportType::Handcart => 5000,
            TransportType::Cart => 10000,
            TransportType::Wagon => 20000,
            TransportType::Sled => 8000,

            // Pack animals (don't break, but might die/flee)
            TransportType::PackDonkey => 0,
            TransportType::PackHorse => 0,
            TransportType::PackCamel => 0,
            TransportType::PackMule => 0,
            TransportType::OxCart => 15000, // Cart can break

            // Rideable mounts (animals don't have durability, they have health/stamina)
            TransportType::Horse => 0,
            TransportType::Warhorse => 0,
            TransportType::Pony => 0,
            TransportType::RidingCamel => 0,
            TransportType::RidingDonkey => 0,
            TransportType::RidingMule => 0,
            TransportType::Reindeer => 0,
            TransportType::Elk => 0,
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            TransportType::Pouch => "Small leather pouch for carrying essentials",
            TransportType::Satchel => "Shoulder bag for moderate loads",
            TransportType::Backpack => "Standard backpack for extended journeys",
            TransportType::LargeBackpack => "Large hiking pack for heavy expeditions",
            TransportType::Travois => "Two poles and a hide, dragged behind",
            TransportType::Handcart => "Small handcart for moving goods",
            TransportType::Cart => "Wheeled cart for transporting materials",
            TransportType::Wagon => "Large wagon for bulk transport",
            TransportType::Sled => "Winter sled for snow travel",
            TransportType::PackDonkey => "Donkey with saddlebags for cargo",
            TransportType::PackHorse => "Horse outfitted for carrying goods",
            TransportType::PackCamel => "Camel loaded for desert transport",
            TransportType::PackMule => "Sturdy mule for mountain cargo",
            TransportType::OxCart => "Ox-drawn cart for heavy loads",
            TransportType::Horse => "Swift riding horse for fast travel",
            TransportType::Warhorse => "Powerful warhorse trained for combat",
            TransportType::Pony => "Nimble pony for light riders",
            TransportType::RidingCamel => "Desert camel for long journeys in heat",
            TransportType::RidingDonkey => "Reliable donkey mount for rough terrain",
            TransportType::RidingMule => "Hardy mule mount for mountain travel",
            TransportType::Reindeer => "Arctic reindeer for snow and tundra",
            TransportType::Elk => "Forest elk for wooded terrain",
        }
    }

    /// Get maximum stamina (only for rideable mounts)
    pub fn max_stamina(&self) -> f32 {
        match self {
            TransportType::Horse => 100.0,
            TransportType::Warhorse => 120.0,
            TransportType::Pony => 80.0,
            TransportType::RidingCamel => 150.0, // Exceptional endurance
            TransportType::RidingDonkey => 90.0,
            TransportType::RidingMule => 110.0,
            TransportType::Reindeer => 130.0,
            TransportType::Elk => 100.0,
            _ => 0.0, // Non-rideable
        }
    }

    /// Get stamina consumption per tick when riding (lower = more efficient)
    pub fn stamina_consumption(&self) -> f32 {
        match self {
            TransportType::Horse => 0.5,
            TransportType::Warhorse => 0.7, // Heavy horse uses more stamina
            TransportType::Pony => 0.6,
            TransportType::RidingCamel => 0.3, // Very efficient
            TransportType::RidingDonkey => 0.4,
            TransportType::RidingMule => 0.4,
            TransportType::Reindeer => 0.3,
            TransportType::Elk => 0.5,
            _ => 0.0,
        }
    }

    /// Get combat effectiveness bonus (for mounted combat)
    pub fn combat_bonus(&self) -> f32 {
        match self {
            TransportType::Horse => 0.2, // 20% combat bonus
            TransportType::Warhorse => 0.5, // 50% combat bonus
            TransportType::Pony => 0.0, // Too small for combat
            TransportType::RidingCamel => 0.1,
            TransportType::RidingDonkey => 0.0,
            TransportType::RidingMule => 0.1,
            TransportType::Reindeer => 0.1,
            TransportType::Elk => 0.3, // Antlers are dangerous
            _ => 0.0,
        }
    }

    /// Get stamina recovery rate per tick when resting
    pub fn stamina_recovery(&self) -> f32 {
        match self {
            TransportType::Horse => 1.0,
            TransportType::Warhorse => 0.8,
            TransportType::Pony => 1.2,
            TransportType::RidingCamel => 0.9,
            TransportType::RidingDonkey => 1.0,
            TransportType::RidingMule => 1.0,
            TransportType::Reindeer => 1.1,
            TransportType::Elk => 1.0,
            _ => 0.0,
        }
    }
}

/// A transport container or vehicle instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transport {
    pub id: Uuid,
    pub transport_type: TransportType,
    /// Current durability (only for vehicles/containers, not animals)
    pub current_durability: u32,
    /// Whether this transport is currently equipped/in use
    pub active: bool,
    /// For animals: health of the animal
    pub animal_health: Option<f32>,
    /// For animals: the animal's UUID if managed separately
    pub animal_id: Option<Uuid>,

    // Mount-specific fields
    /// Current stamina (for rideable mounts)
    pub stamina: Option<f32>,
    /// Training level (0.0 to 1.0) - affects speed and behavior
    pub training_level: Option<f32>,
    /// Whether someone is currently mounted
    pub is_mounted: bool,
    /// Loyalty/bond (0.0 to 1.0) - affects chance of fleeing
    pub loyalty: Option<f32>,
}

impl Transport {
    pub fn new(transport_type: TransportType) -> Self {
        let is_animal = transport_type.is_pack_animal() || transport_type.is_rideable();
        let animal_health = if is_animal { Some(100.0) } else { None };

        let stamina = if transport_type.is_rideable() {
            Some(transport_type.max_stamina())
        } else {
            None
        };

        let training_level = if transport_type.is_rideable() {
            Some(0.0) // Untrained by default
        } else {
            None
        };

        let loyalty = if transport_type.is_rideable() {
            Some(0.0) // No bond by default
        } else {
            None
        };

        Self {
            id: crate::core::dice::name(),
            transport_type,
            current_durability: transport_type.durability(),
            active: false,
            animal_health,
            animal_id: None,
            stamina,
            training_level,
            is_mounted: false,
            loyalty,
        }
    }

    /// Create with specific animal
    pub fn with_animal(transport_type: TransportType, animal_id: Uuid) -> Self {
        let mut transport = Self::new(transport_type);
        transport.animal_id = Some(animal_id);
        transport
    }

    /// Get remaining capacity
    pub fn capacity(&self) -> f32 {
        self.transport_type.weight_capacity()
    }

    /// Check if broken (durability at 0)
    pub fn is_broken(&self) -> bool {
        self.current_durability == 0 && self.transport_type.durability() > 0
    }

    /// Damage the transport
    pub fn damage(&mut self, amount: u32) {
        if self.current_durability > 0 {
            self.current_durability = self.current_durability.saturating_sub(amount);
        }
    }

    /// Repair to full durability
    pub fn repair(&mut self) {
        self.current_durability = self.transport_type.durability();
    }

    /// Check if animal is alive
    pub fn animal_alive(&self) -> bool {
        self.animal_health.map(|h| h > 0.0).unwrap_or(true)
    }

    /// Damage animal
    pub fn damage_animal(&mut self, damage: f32) {
        if let Some(health) = self.animal_health.as_mut() {
            *health = (*health - damage).max(0.0);
        }
    }


    /// Get usability (0.0 to 1.0)
    pub fn usability(&self) -> f32 {
        if !self.animal_alive() {
            return 0.0;
        }

        if self.is_broken() {
            return 0.0;
        }

        let durability_factor = if self.transport_type.durability() > 0 {
            self.current_durability as f32 / self.transport_type.durability() as f32
        } else {
            1.0
        };

        let health_factor = self.animal_health.map(|h| h / 100.0).unwrap_or(1.0);

        durability_factor.min(health_factor)
    }

    // ===== Mount-specific methods =====

    /// Mount this transport (for rideable mounts)
    pub fn mount(&mut self) -> Result<(), String> {
        if !self.transport_type.is_rideable() {
            return Err("This transport is not rideable".to_string());
        }

        if !self.animal_alive() {
            return Err("Mount is dead".to_string());
        }

        if self.is_mounted {
            return Err("Already mounted".to_string());
        }

        if let Some(stamina) = self.stamina {
            if stamina < 10.0 {
                return Err("Mount is too exhausted".to_string());
            }
        }

        self.is_mounted = true;
        self.active = true;
        Ok(())
    }

    /// Dismount from this transport
    pub fn dismount(&mut self) {
        self.is_mounted = false;
    }

    /// Consume stamina while riding
    pub fn consume_stamina(&mut self, multiplier: f32) {
        if let Some(stamina) = self.stamina.as_mut() {
            let consumption = self.transport_type.stamina_consumption() * multiplier;
            *stamina = (*stamina - consumption).max(0.0);
        }
    }

    /// Recover stamina while resting
    pub fn recover_stamina(&mut self) {
        if let Some(stamina) = self.stamina.as_mut() {
            let recovery = self.transport_type.stamina_recovery();
            let max = self.transport_type.max_stamina();
            *stamina = (*stamina + recovery).min(max);
        }
    }

    /// Train the mount (increases training level)
    pub fn train(&mut self, amount: f32) {
        if let Some(training) = self.training_level.as_mut() {
            *training = (*training + amount).min(1.0);
        }
    }

    /// Bond with the mount (increases loyalty)
    pub fn bond(&mut self, amount: f32) {
        if let Some(loyalty) = self.loyalty.as_mut() {
            *loyalty = (*loyalty + amount).min(1.0);
        }
    }

    /// Get effective speed modifier (affected by stamina, training, health)
    pub fn effective_speed(&self) -> f32 {
        let base_speed = self.transport_type.speed_modifier();

        // For rideable mounts, apply modifiers
        if self.transport_type.is_rideable() && self.is_mounted {
            let stamina_factor = self.stamina
                .map(|s| (s / self.transport_type.max_stamina()).max(0.3))
                .unwrap_or(1.0);

            let training_bonus = self.training_level
                .map(|t| 1.0 + (t * 0.2)) // Up to 20% speed bonus when fully trained
                .unwrap_or(1.0);

            let health_factor = self.animal_health
                .map(|h| (h / 100.0).max(0.5))
                .unwrap_or(1.0);

            base_speed * stamina_factor * training_bonus * health_factor
        } else {
            base_speed
        }
    }

    /// Get combat effectiveness (for mounted combat)
    pub fn combat_effectiveness(&self) -> f32 {
        if !self.is_mounted {
            return 0.0;
        }

        let base_bonus = self.transport_type.combat_bonus();
        let training_multiplier = self.training_level.unwrap_or(0.0);

        base_bonus * (0.5 + training_multiplier * 0.5) // Half effectiveness when untrained
    }

    /// Check if mount is exhausted (stamina below threshold)
    pub fn is_exhausted(&self) -> bool {
        self.stamina.map(|s| s < 20.0).unwrap_or(false)
    }

    /// Check if mount might flee (based on loyalty and health)
    pub fn will_flee(&self) -> bool {
        if !self.transport_type.is_rideable() {
            return false;
        }

        let loyalty = self.loyalty.unwrap_or(0.0);
        let health = self.animal_health.unwrap_or(100.0);

        // Low loyalty and low health increases flee chance
        loyalty < 0.3 && health < 30.0
    }

    /// Get stamina percentage
    pub fn stamina_percentage(&self) -> f32 {
        self.stamina
            .map(|s| s / self.transport_type.max_stamina())
            .unwrap_or(0.0)
    }

    /// Get status description for mounts
    pub fn mount_status(&self) -> String {
        if !self.transport_type.is_rideable() {
            return "Not a mount".to_string();
        }

        let health = self.animal_health.unwrap_or(100.0);
        let stamina = self.stamina_percentage() * 100.0;
        let training = self.training_level.unwrap_or(0.0) * 100.0;
        let loyalty = self.loyalty.unwrap_or(0.0) * 100.0;

        format!(
            "Health: {:.0}% | Stamina: {:.0}% | Training: {:.0}% | Loyalty: {:.0}%{}",
            health,
            stamina,
            training,
            loyalty,
            if self.is_mounted { " [MOUNTED]" } else { "" }
        )
    }
}

/// Transport management system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportSystem {
    /// All transports owned
    transports: Vec<Transport>,
    /// Currently active transports
    active_transport_ids: Vec<Uuid>,
    /// Maximum number of transports that can be active at once
    max_active: usize,
}

impl TransportSystem {
    pub fn new() -> Self {
        Self {
            transports: Vec::new(),
            active_transport_ids: Vec::new(),
            max_active: 5, // Can have backpack + cart + pack animal, etc.
        }
    }

    /// Add a transport
    pub fn add_transport(&mut self, transport: Transport) {
        self.transports.push(transport);
    }

    /// Activate a transport (equip it)
    pub fn activate(&mut self, transport_id: &Uuid) -> bool {
        if self.active_transport_ids.len() >= self.max_active {
            return false;
        }

        if let Some(transport) = self.transports.iter_mut().find(|t| t.id == *transport_id) {
            if transport.usability() == 0.0 {
                return false; // Can't use broken/dead transport
            }

            transport.active = true;
            self.active_transport_ids.push(*transport_id);
            true
        } else {
            false
        }
    }

    /// Deactivate a transport
    pub fn deactivate(&mut self, transport_id: &Uuid) {
        if let Some(transport) = self.transports.iter_mut().find(|t| t.id == *transport_id) {
            transport.active = false;
        }
        self.active_transport_ids.retain(|id| id != transport_id);
    }

    /// Get total additional capacity from all active transports
    pub fn total_additional_capacity(&self) -> f32 {
        self.transports.iter()
            .filter(|t| t.active && t.usability() > 0.0)
            .map(|t| t.capacity() * t.usability())
            .sum()
    }


    /// Get movement speed modifier from active transports
    pub fn speed_modifier(&self) -> f32 {
        let modifiers: Vec<f32> = self.transports.iter()
            .filter(|t| t.active)
            .map(|t| t.transport_type.speed_modifier())
            .collect();

        if modifiers.is_empty() {
            1.0
        } else {
            // Use the slowest modifier
            modifiers.into_iter().fold(1.0, |acc, m| acc.min(m))
        }
    }

    /// Get all transports
    pub fn get_all(&self) -> &Vec<Transport> {
        &self.transports
    }

    /// Get active transports
    pub fn get_active(&self) -> Vec<&Transport> {
        self.transports.iter()
            .filter(|t| t.active)
            .collect()
    }

    /// Get a specific transport
    pub fn get(&self, transport_id: &Uuid) -> Option<&Transport> {
        self.transports.iter().find(|t| t.id == *transport_id)
    }

    /// Get mutable transport
    pub fn get_mut(&mut self, transport_id: &Uuid) -> Option<&mut Transport> {
        self.transports.iter_mut().find(|t| t.id == *transport_id)
    }

    /// Remove a transport
    pub fn remove(&mut self, transport_id: &Uuid) -> Option<Transport> {
        if let Some(pos) = self.transports.iter().position(|t| t.id == *transport_id) {
            self.deactivate(transport_id);
            Some(self.transports.remove(pos))
        } else {
            None
        }
    }



    // ===== Mount-specific methods =====

    /// Get currently mounted transport
    pub fn get_mounted(&self) -> Option<&Transport> {
        self.transports.iter().find(|t| t.is_mounted)
    }

    /// Get currently mounted transport (mutable)
    pub fn get_mounted_mut(&mut self) -> Option<&mut Transport> {
        self.transports.iter_mut().find(|t| t.is_mounted)
    }

    /// Mount a specific transport
    pub fn mount_transport(&mut self, transport_id: &Uuid) -> Result<(), String> {
        // Check if already mounted on something
        if self.transports.iter().any(|t| t.is_mounted) {
            return Err("Already mounted on another transport".to_string());
        }

        // Find and mount the transport
        if let Some(transport) = self.transports.iter_mut().find(|t| t.id == *transport_id) {
            transport.mount()
        } else {
            Err("Transport not found".to_string())
        }
    }

    /// Dismount from current mount
    pub fn dismount_current(&mut self) {
        if let Some(transport) = self.transports.iter_mut().find(|t| t.is_mounted) {
            transport.dismount();
        }
    }


    /// Get all available mounts (alive, not exhausted)
    pub fn get_available_mounts(&self) -> Vec<&Transport> {
        self.transports.iter()
            .filter(|t| {
                t.transport_type.is_rideable() &&
                t.animal_alive() &&
                !t.is_exhausted()
            })
            .collect()
    }

    /// Get movement speed (accounting for mounted state)
    pub fn effective_speed_modifier(&self) -> f32 {
        // Check if mounted
        if let Some(mount) = self.get_mounted() {
            return mount.effective_speed();
        }

        // Otherwise use cargo transport speed (slower)
        let modifiers: Vec<f32> = self.transports.iter()
            .filter(|t| t.active && !t.transport_type.is_rideable())
            .map(|t| t.transport_type.speed_modifier())
            .collect();

        if modifiers.is_empty() {
            1.0
        } else {
            modifiers.into_iter().fold(1.0, |acc, m| acc.min(m))
        }
    }

    /// Get combat bonus from mount
    pub fn mounted_combat_bonus(&self) -> f32 {
        self.get_mounted()
            .map(|m| m.combat_effectiveness())
            .unwrap_or(0.0)
    }



    /// Check if currently mounted
    pub fn is_mounted(&self) -> bool {
        self.transports.iter().any(|t| t.is_mounted)
    }

    /// Get summary of all mounts
    pub fn mount_summary(&self) -> Vec<String> {
        self.transports.iter()
            .filter(|t| t.transport_type.is_rideable())
            .map(|t| {
                format!(
                    "{:?}: {}",
                    t.transport_type,
                    t.mount_status()
                )
            })
            .collect()
    }
}

impl Default for TransportSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_properties() {
        assert_eq!(TransportType::Backpack.weight_capacity(), 30.0);
        assert!(TransportType::Backpack.is_wearable());
        assert!(!TransportType::Backpack.is_vehicle());
        assert!(!TransportType::Backpack.is_pack_animal());
    }

    #[test]
    fn test_vehicle_properties() {
        assert_eq!(TransportType::Cart.weight_capacity(), 150.0);
        assert!(!TransportType::Cart.is_wearable());
        assert!(TransportType::Cart.is_vehicle());
        assert!(!TransportType::Cart.is_pack_animal());
    }

    #[test]
    fn test_pack_animal_properties() {
        assert_eq!(TransportType::PackHorse.weight_capacity(), 150.0);
        assert!(!TransportType::PackHorse.is_wearable());
        assert!(!TransportType::PackHorse.is_vehicle());
        assert!(TransportType::PackHorse.is_pack_animal());
        assert!(TransportType::PackHorse.requires_animal());
    }

    #[test]
    fn test_transport_creation() {
        let backpack = Transport::new(TransportType::Backpack);
        assert_eq!(backpack.capacity(), 30.0);
        assert!(!backpack.active);
        assert!(!backpack.is_broken());
        assert_eq!(backpack.usability(), 1.0);
    }

    #[test]
    fn test_transport_durability() {
        let mut cart = Transport::new(TransportType::Cart);
        assert_eq!(cart.current_durability, 10000);

        cart.damage(5000);
        assert_eq!(cart.current_durability, 5000);
        assert!(!cart.is_broken());

        cart.damage(5000);
        assert_eq!(cart.current_durability, 0);
        assert!(cart.is_broken());
        assert_eq!(cart.usability(), 0.0);
    }

    #[test]
    fn test_pack_animal_health() {
        let mut donkey = Transport::new(TransportType::PackDonkey);
        assert!(donkey.animal_alive());
        assert_eq!(donkey.animal_health, Some(100.0));

        donkey.damage_animal(60.0);
        assert_eq!(donkey.animal_health, Some(40.0));
        assert!(donkey.animal_alive());

        donkey.damage_animal(40.0);
        assert_eq!(donkey.animal_health, Some(0.0));
        assert!(!donkey.animal_alive());
        assert_eq!(donkey.usability(), 0.0);
    }

    #[test]
    fn test_transport_system() {
        let mut system = TransportSystem::new();

        let backpack = Transport::new(TransportType::Backpack);
        let backpack_id = backpack.id;

        system.add_transport(backpack);

        assert!(system.activate(&backpack_id));
        assert_eq!(system.total_additional_capacity(), 30.0);

        let cart = Transport::new(TransportType::Cart);
        let cart_id = cart.id;
        system.add_transport(cart);
        assert!(system.activate(&cart_id));

        assert_eq!(system.total_additional_capacity(), 180.0); // 30 + 150
    }

    #[test]
    fn test_speed_modifier() {
        let mut system = TransportSystem::new();

        let backpack = Transport::new(TransportType::Backpack);
        let backpack_id = backpack.id;
        system.add_transport(backpack);
        system.activate(&backpack_id);

        // Backpack alone: 0.90 speed
        assert_eq!(system.speed_modifier(), 0.90);

        let wagon = Transport::new(TransportType::Wagon);
        let wagon_id = wagon.id;
        system.add_transport(wagon);
        system.activate(&wagon_id);

        // With wagon: slowest is 0.50
        assert_eq!(system.speed_modifier(), 0.50);
    }

    #[test]
    fn test_transport_removal() {
        let mut system = TransportSystem::new();

        let backpack = Transport::new(TransportType::Backpack);
        let backpack_id = backpack.id;
        system.add_transport(backpack);
        system.activate(&backpack_id);

        assert_eq!(system.total_additional_capacity(), 30.0);

        let removed = system.remove(&backpack_id);
        assert!(removed.is_some());
        assert_eq!(system.total_additional_capacity(), 0.0);
    }
}
