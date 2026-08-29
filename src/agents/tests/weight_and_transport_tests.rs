// src/agents/tests/weight_and_transport_tests.rs
//! Integration tests for inventory weight system and transport

use crate::agents::{
    Agent, AgentConfig, InventoryItem, Transport, TransportType,
};

#[test]
fn test_inventory_weight_enforcement() {
    let mut agent = Agent::new(AgentConfig::default());

    // An agent with nothing to carry things in holds what two hands hold
    let room = agent.inventory.max_weight;
    assert_eq!(room, Agent::WHAT_TWO_HANDS_HOLD);
    assert_eq!(agent.inventory.current_weight, 0.0);

    // A stone in each hand goes in
    let a_couple = (room / 4.0).floor() as u32;
    let light_item = InventoryItem::new_with_weight("stone".to_string(), a_couple, 1.0);
    assert!(agent.inventory.add_item(light_item));
    assert_eq!(agent.inventory.current_weight, a_couple as f32);

    // And so does an armful on top of it
    let an_armful = ((room - a_couple as f32) / 1.0).floor() as u32;
    let medium_item =
        InventoryItem::new_with_weight("iron_block".to_string(), an_armful, 1.0);
    assert!(agent.inventory.add_item(medium_item));

    // But nothing past that
    let heavy_item = InventoryItem::new_with_weight("wood".to_string(), 20, 1.0);
    assert!(
        !agent.inventory.add_item(heavy_item),
        "Should not add item over weight limit"
    );
}

#[test]
fn test_inventory_weight_with_containers() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add empty waterskin (0.5kg empty)
    let mut waterskin = InventoryItem::new_container("waterskin".to_string(), 1, 5.0);
    waterskin.weight_per_unit = 0.5;
    agent.inventory.add_item(waterskin);

    assert_eq!(agent.inventory.current_weight, 0.5);

    // Fill with 5L of water (5kg)
    agent.inventory.fill_containers(5.0);

    // Total weight should be 5.5kg (0.5 container + 5.0 water)
    assert_eq!(agent.inventory.current_weight, 5.5);

    // Drink 2L of water
    agent.inventory.drink_water(2.0);

    // Weight should be 3.5kg (0.5 container + 3.0 water)
    assert_eq!(agent.inventory.current_weight, 3.5);
}

#[test]
fn test_backpack_increases_capacity() {
    let mut agent = Agent::new(AgentConfig::default());

    // Base capacity is 100kg
    // What two bare hands hold, which is an armful rather than a backpack.
    // Asked of the constant rather than written down here: the base was a
    // hundred, and a hundred in two bare hands made every container in the
    // model decorative. See ISSUES #88.
    let bare_hands = Agent::WHAT_TWO_HANDS_HOLD;
    assert_eq!(agent.total_carrying_capacity(), bare_hands);

    // Add a backpack
    let backpack = Transport::new(TransportType::Backpack);
    let backpack_id = backpack.id;
    agent.add_transport(backpack);

    // Capacity shouldn't change until equipped
    assert_eq!(agent.total_carrying_capacity(), bare_hands);

    // Equip backpack
    assert!(agent.equip_transport(&backpack_id));

    // Capacity should increase by what a backpack holds
    assert_eq!(
        agent.total_carrying_capacity(),
        bare_hands + TransportType::Backpack.weight_capacity()
    );
}

#[test]
fn test_multiple_transports() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add backpack (30kg)
    let backpack = Transport::new(TransportType::Backpack);
    let backpack_id = backpack.id;
    agent.add_transport(backpack);
    agent.equip_transport(&backpack_id);

    // Add cart (150kg)
    let cart = Transport::new(TransportType::Cart);
    let cart_id = cart.id;
    agent.add_transport(cart);
    agent.equip_transport(&cart_id);

    // Two hands, a backpack and a cart
    assert_eq!(
        agent.total_carrying_capacity(),
        Agent::WHAT_TWO_HANDS_HOLD
            + TransportType::Backpack.weight_capacity()
            + TransportType::Cart.weight_capacity()
    );
}

#[test]
fn test_pack_animal() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add pack donkey (100kg capacity)
    let donkey = Transport::new(TransportType::PackDonkey);
    let donkey_id = donkey.id;
    agent.add_transport(donkey);
    agent.equip_transport(&donkey_id);

    // Capacity should increase
    assert_eq!(
        agent.total_carrying_capacity(),
        Agent::WHAT_TWO_HANDS_HOLD + TransportType::PackDonkey.weight_capacity()
    );
}

#[test]
fn test_movement_speed_with_weight() {
    let agent = Agent::new(AgentConfig::default());

    // Empty agent should have base speed (1.0 from body)
    let base_speed = agent.movement_speed();
    assert!(base_speed > 0.9); // Should be close to 1.0

    // Add some weight
    let mut loaded_agent = Agent::new(AgentConfig::default());
    let half_a_load = (loaded_agent.inventory.max_weight / 2.0).floor() as u32;
    let heavy_item =
        InventoryItem::new_with_weight("stone".to_string(), half_a_load, 1.0);
    loaded_agent.inventory.add_item(heavy_item);

    // 50% loaded should have slight speed penalty
    let loaded_speed = loaded_agent.movement_speed();
    assert!(loaded_speed < base_speed);
    assert!(loaded_speed > 0.8); // Not too slow
}

#[test]
fn test_movement_speed_with_cart() {
    let mut agent = Agent::new(AgentConfig::default());

    // Base speed
    let base_speed = agent.movement_speed();

    // Add cart
    let cart = Transport::new(TransportType::Cart);
    let cart_id = cart.id;
    agent.add_transport(cart);
    agent.equip_transport(&cart_id);

    // Cart reduces speed to 0.60
    let cart_speed = agent.movement_speed();
    assert_eq!(cart_speed, 0.60);
    assert!(cart_speed < base_speed);
}

#[test]
fn test_overweight_penalty() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add items over capacity
    let overweight_item = InventoryItem::new_with_weight("stone".to_string(), 150, 1.0);
    agent.inventory.current_weight = 150.0; // Force overweight

    assert!(agent.inventory.is_overweight());

    // Overweight gives 50% speed penalty
    let speed = agent.movement_speed();
    assert_eq!(speed, 0.5);
}

#[test]
fn test_transport_durability() {
    let mut transport = Transport::new(TransportType::Handcart);

    // Start at full durability
    assert!(!transport.is_broken());
    assert_eq!(transport.usability(), 1.0);

    // Damage halfway
    transport.damage(2500);
    assert!(!transport.is_broken());
    assert_eq!(transport.usability(), 0.5);

    // Damage to breaking
    transport.damage(2500);
    assert!(transport.is_broken());
    assert_eq!(transport.usability(), 0.0);
}

#[test]
fn test_pack_animal_health() {
    let mut donkey = Transport::new(TransportType::PackDonkey);

    // Healthy at start
    assert!(donkey.animal_alive());
    assert_eq!(donkey.animal_health, Some(100.0));
    assert_eq!(donkey.usability(), 1.0);

    // Damage animal
    donkey.damage_animal(70.0);
    assert!(donkey.animal_alive());
    assert_eq!(donkey.animal_health, Some(30.0));
    assert_eq!(donkey.usability(), 0.3); // 30% health = 30% usability

    // Kill animal
    donkey.damage_animal(30.0);
    assert!(!donkey.animal_alive());
    assert_eq!(donkey.usability(), 0.0);
}

#[test]
fn test_can_carry_check() {
    let mut agent = Agent::new(AgentConfig::default());

    // Can carry what two hands hold, and no more
    let room = agent.inventory.max_weight;
    assert!(agent.can_carry(room / 2.0));
    assert!(agent.can_carry(room));
    assert!(!agent.can_carry(room + 1.0));

    // Add some weight
    let a_few = (room / 3.0).floor() as u32;
    let item = InventoryItem::new_with_weight("stone".to_string(), a_few, 1.0);
    assert!(agent.inventory.add_item(item));

    // Can now carry the rest of it and no more
    let left = room - a_few as f32;
    assert!(agent.can_carry(left));
    assert!(!agent.can_carry(left + 1.0));
}

#[test]
fn test_remove_item_updates_weight() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add an armful of stone - as much as two hands hold
    let an_armful = agent.inventory.max_weight.floor() as u32;
    let item = InventoryItem::new_with_weight("stone".to_string(), an_armful, 1.0);
    assert!(agent.inventory.add_item(item));
    assert_eq!(agent.inventory.current_weight, an_armful as f32);

    // Remove some
    let put_down = an_armful / 2;
    agent.inventory.remove_item("stone", put_down);
    assert_eq!(
        agent.inventory.current_weight,
        (an_armful - put_down) as f32
    );

    // Remove rest
    agent.inventory.remove_item("stone", an_armful - put_down);
    assert_eq!(agent.inventory.current_weight, 0.0);
}

#[test]
fn test_recalculate_weight() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add items
    let item1 = InventoryItem::new_with_weight("stone".to_string(), 4, 2.0);
    let item2 = InventoryItem::new_with_weight("wood".to_string(), 2, 1.0);

    assert!(agent.inventory.add_item(item1));
    assert!(agent.inventory.add_item(item2));
    let actually_carrying = agent.inventory.current_weight;

    // Manually corrupt weight
    agent.inventory.current_weight = 0.0;

    // Recalculate
    agent.inventory.recalculate_weight();

    assert_eq!(agent.inventory.current_weight, actually_carrying);
}

#[test]
fn test_wagon_for_bulk_transport() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add wagon
    let wagon = Transport::new(TransportType::Wagon);
    let wagon_id = wagon.id;
    agent.add_transport(wagon);
    agent.equip_transport(&wagon_id);

    // Wagon adds 500kg capacity
    assert_eq!(
        agent.total_carrying_capacity(),
        Agent::WHAT_TWO_HANDS_HOLD + TransportType::Wagon.weight_capacity()
    );

    // Can carry massive loads
    let huge_item = InventoryItem::new_with_weight("ore".to_string(), 400, 1.0);
    assert!(agent.inventory.add_item(huge_item));
}

#[test]
fn test_unequip_transport_reduces_capacity() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add and equip backpack
    let backpack = Transport::new(TransportType::Backpack);
    let backpack_id = backpack.id;
    agent.add_transport(backpack);
    agent.equip_transport(&backpack_id);

    assert_eq!(
        agent.total_carrying_capacity(),
        Agent::WHAT_TWO_HANDS_HOLD + TransportType::Backpack.weight_capacity()
    );

    // Unequip
    agent.unequip_transport(&backpack_id);

    // Back to what two hands hold
    assert_eq!(agent.total_carrying_capacity(), Agent::WHAT_TWO_HANDS_HOLD);
}

#[test]
fn test_large_backpack() {
    let mut agent = Agent::new(AgentConfig::default());

    // Add large backpack (50kg capacity)
    let backpack = Transport::new(TransportType::LargeBackpack);
    let backpack_id = backpack.id;
    agent.add_transport(backpack);
    agent.equip_transport(&backpack_id);

    assert_eq!(
        agent.total_carrying_capacity(),
        Agent::WHAT_TWO_HANDS_HOLD + TransportType::LargeBackpack.weight_capacity()
    );

    // But slower speed
    let speed = agent.movement_speed();
    assert_eq!(speed, 0.85); // Large backpack speed modifier
}

#[test]
fn test_transport_types() {
    // Wearable
    assert!(TransportType::Backpack.is_wearable());
    assert!(!TransportType::Cart.is_wearable());

    // Vehicle
    assert!(TransportType::Cart.is_vehicle());
    assert!(!TransportType::Backpack.is_vehicle());

    // Pack animal
    assert!(TransportType::PackHorse.is_pack_animal());
    assert!(!TransportType::Backpack.is_pack_animal());
}
