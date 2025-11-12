// src/agents/agent.rs
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::core::{BehaviorTree, DriveState, Memory};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub random_weights: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { random_weights: true }
    }
}

/// Item stored in inventory with quantity and optional fill level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub item_id: String,
    pub quantity: u32,
    /// For containers: current fill level (e.g., water in waterskin)
    pub fill_level: Option<f32>,
    /// For containers: maximum capacity
    pub max_capacity: Option<f32>,
}

impl InventoryItem {
    pub fn new(item_id: String, quantity: u32) -> Self {
        Self {
            item_id,
            quantity,
            fill_level: None,
            max_capacity: None,
        }
    }

    pub fn new_container(item_id: String, quantity: u32, capacity: f32) -> Self {
        Self {
            item_id,
            quantity,
            fill_level: Some(0.0),
            max_capacity: Some(capacity),
        }
    }

    /// Check if this is a container
    pub fn is_container(&self) -> bool {
        self.max_capacity.is_some()
    }

    /// Get fill percentage (0.0 to 1.0)
    pub fn fill_percentage(&self) -> f32 {
        match (self.fill_level, self.max_capacity) {
            (Some(fill), Some(max)) if max > 0.0 => fill / max,
            _ => 0.0,
        }
    }

    /// Add liquid to container, returns amount actually added
    pub fn fill(&mut self, amount: f32) -> f32 {
        match (self.fill_level.as_mut(), self.max_capacity) {
            (Some(fill), Some(max)) => {
                let space_available = max - *fill;
                let amount_to_add = amount.min(space_available);
                *fill += amount_to_add;
                amount_to_add
            }
            _ => 0.0,
        }
    }

    /// Remove liquid from container, returns amount actually removed
    pub fn drain(&mut self, amount: f32) -> f32 {
        match self.fill_level.as_mut() {
            Some(fill) => {
                let amount_to_remove = amount.min(*fill);
                *fill -= amount_to_remove;
                amount_to_remove
            }
            _ => 0.0,
        }
    }
}

/// Agent inventory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Items stored by item_id
    items: HashMap<String, InventoryItem>,
    /// Maximum number of item stacks
    pub max_slots: usize,
    /// Maximum weight that can be carried
    pub max_weight: f32,
    /// Current total weight
    pub current_weight: f32,
}

impl Inventory {
    pub fn new(max_slots: usize, max_weight: f32) -> Self {
        Self {
            items: HashMap::new(),
            max_slots,
            max_weight,
            current_weight: 0.0,
        }
    }

    /// Add an item to inventory
    pub fn add_item(&mut self, item: InventoryItem) -> bool {
        if self.items.len() >= self.max_slots && !self.items.contains_key(&item.item_id) {
            return false; // No room for new item type
        }

        self.items.insert(item.item_id.clone(), item);
        true
    }

    /// Remove an item from inventory
    pub fn remove_item(&mut self, item_id: &str, quantity: u32) -> Option<InventoryItem> {
        if let Some(item) = self.items.get_mut(item_id) {
            if item.quantity >= quantity {
                item.quantity -= quantity;
                let removed = InventoryItem {
                    item_id: item_id.to_string(),
                    quantity,
                    fill_level: item.fill_level,
                    max_capacity: item.max_capacity,
                };

                if item.quantity == 0 {
                    self.items.remove(item_id);
                }

                return Some(removed);
            }
        }
        None
    }

    /// Get an item from inventory
    pub fn get_item(&self, item_id: &str) -> Option<&InventoryItem> {
        self.items.get(item_id)
    }

    /// Get a mutable item from inventory
    pub fn get_item_mut(&mut self, item_id: &str) -> Option<&mut InventoryItem> {
        self.items.get_mut(item_id)
    }

    /// Get total water available from all containers
    pub fn get_total_water(&self) -> f32 {
        self.items.values()
            .filter(|item| item.is_container())
            .filter_map(|item| item.fill_level)
            .sum()
    }

    /// Drink water from any available container
    pub fn drink_water(&mut self, amount: f32) -> f32 {
        let mut remaining = amount;

        for item in self.items.values_mut() {
            if item.is_container() && remaining > 0.0 {
                let drained = item.drain(remaining);
                remaining -= drained;
            }
        }

        amount - remaining // Return amount actually drunk
    }

    /// Fill containers from a water source
    pub fn fill_containers(&mut self, available_water: f32) -> f32 {
        let mut remaining = available_water;

        for item in self.items.values_mut() {
            if item.is_container() && remaining > 0.0 {
                let filled = item.fill(remaining);
                remaining -= filled;
            }
        }

        available_water - remaining // Return amount actually used
    }

    /// Check if inventory has item with minimum quantity
    pub fn has_item(&self, item_id: &str, min_quantity: u32) -> bool {
        self.items.get(item_id)
            .map(|item| item.quantity >= min_quantity)
            .unwrap_or(false)
    }

    /// Get all items
    pub fn get_all_items(&self) -> &HashMap<String, InventoryItem> {
        &self.items
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(20, 100.0) // Default: 20 slots, 100 weight units
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub health: f32,
    pub position: (i32, i32, i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub state: AgentState,
    pub drives: DriveState,
    pub behavior_trees: Vec<BehaviorTree>,
    pub memory: Memory,
    pub inventory: Inventory,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: AgentState {
                health: 100.0,
                position: (0, 0, 0),
            },
            drives: if config.random_weights {
                DriveState::with_random_weights()
            } else {
                DriveState::new()
            },
            behavior_trees: Vec::new(),
            memory: Memory::new(),
            inventory: Inventory::default(),
        }
    }
}
