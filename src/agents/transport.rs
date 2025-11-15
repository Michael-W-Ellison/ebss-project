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
    /// Small handcart
    Handcart,
    /// Larger cart
    Cart,
    /// Full wagon
    Wagon,
    /// Winter sled
    Sled,

    // Pack animals
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
            TransportType::Handcart => 75.0,
            TransportType::Cart => 150.0,
            TransportType::Wagon => 500.0,
            TransportType::Sled => 100.0,

            // Pack animals
            TransportType::PackDonkey => 100.0,
            TransportType::PackHorse => 150.0,
            TransportType::PackCamel => 200.0,
            TransportType::PackMule => 120.0,
            TransportType::OxCart => 600.0,
        }
    }

    /// Get the weight of the transport itself (empty)
    pub fn self_weight(&self) -> f32 {
        match self {
            // Worn containers
            TransportType::Pouch => 0.3,
            TransportType::Satchel => 1.0,
            TransportType::Backpack => 2.0,
            TransportType::LargeBackpack => 3.5,

            // Vehicles
            TransportType::Handcart => 15.0,
            TransportType::Cart => 50.0,
            TransportType::Wagon => 200.0,
            TransportType::Sled => 25.0,

            // Pack animals (animal weight, not cargo)
            TransportType::PackDonkey => 200.0,
            TransportType::PackHorse => 400.0,
            TransportType::PackCamel => 600.0,
            TransportType::PackMule => 350.0,
            TransportType::OxCart => 800.0, // Ox + cart
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

    /// Get movement speed modifier (1.0 = normal, < 1.0 = slower)
    pub fn speed_modifier(&self) -> f32 {
        match self {
            // Worn containers (slight slowdown when full)
            TransportType::Pouch => 0.98,
            TransportType::Satchel => 0.95,
            TransportType::Backpack => 0.90,
            TransportType::LargeBackpack => 0.85,

            // Vehicles (significant slowdown)
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
        }
    }

    /// Check if requires animal to operate
    pub fn requires_animal(&self) -> bool {
        self.is_pack_animal()
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
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            TransportType::Pouch => "Small leather pouch for carrying essentials",
            TransportType::Satchel => "Shoulder bag for moderate loads",
            TransportType::Backpack => "Standard backpack for extended journeys",
            TransportType::LargeBackpack => "Large hiking pack for heavy expeditions",
            TransportType::Handcart => "Small handcart for moving goods",
            TransportType::Cart => "Wheeled cart for transporting materials",
            TransportType::Wagon => "Large wagon for bulk transport",
            TransportType::Sled => "Winter sled for snow travel",
            TransportType::PackDonkey => "Donkey with saddlebags for cargo",
            TransportType::PackHorse => "Horse outfitted for carrying goods",
            TransportType::PackCamel => "Camel loaded for desert transport",
            TransportType::PackMule => "Sturdy mule for mountain cargo",
            TransportType::OxCart => "Ox-drawn cart for heavy loads",
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
    /// For pack animals: health of the animal
    pub animal_health: Option<f32>,
    /// For pack animals: the animal's UUID if managed separately
    pub animal_id: Option<Uuid>,
}

impl Transport {
    pub fn new(transport_type: TransportType) -> Self {
        let animal_health = if transport_type.is_pack_animal() {
            Some(100.0)
        } else {
            None
        };

        Self {
            id: Uuid::new_v4(),
            transport_type,
            current_durability: transport_type.durability(),
            active: false,
            animal_health,
            animal_id: None,
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

    /// Heal animal
    pub fn heal_animal(&mut self, amount: f32) {
        if let Some(health) = self.animal_health.as_mut() {
            *health = (*health + amount).min(100.0);
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

    /// Get total weight of all active transports themselves
    pub fn total_transport_weight(&self) -> f32 {
        self.transports.iter()
            .filter(|t| t.active)
            .map(|t| t.transport_type.self_weight())
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

    /// Count transports of a specific type
    pub fn count_type(&self, transport_type: TransportType) -> usize {
        self.transports.iter()
            .filter(|t| t.transport_type == transport_type)
            .count()
    }

    /// Check if has any pack animals
    pub fn has_pack_animal(&self) -> bool {
        self.transports.iter()
            .any(|t| t.transport_type.is_pack_animal() && t.animal_alive())
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
