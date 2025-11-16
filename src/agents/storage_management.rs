// src/agents/storage_management.rs
//! Storage management and decision-making for agents.
//!
//! Handles when and what agents should deposit/retrieve from communal storage.

use serde::{Deserialize, Serialize};
use crate::world::ItemType;
use crate::core::DriveType;

/// Decision about what to do with storage
#[derive(Debug, Clone)]
pub enum StorageDecision {
    /// Should deposit these items
    Deposit {
        item_type: ItemType,
        quantity: u32,
        reason: String,
    },
    /// Should retrieve these items
    Retrieve {
        item_type: ItemType,
        quantity: u32,
        reason: String,
    },
    /// No storage action needed
    NoAction {
        reason: String,
    },
}

/// Agent's storage preferences and behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePreferences {
    /// Minimum food to keep in personal inventory
    pub min_personal_food: u32,
    /// Maximum food before depositing excess
    pub max_personal_food: u32,
    /// Minimum tools to keep
    pub min_personal_tools: u32,
    /// Whether to hoard resources (greedy trait)
    pub hoarding_tendency: f32, // 0.0 = shares everything, 1.0 = hoards everything
}

impl Default for StoragePreferences {
    fn default() -> Self {
        Self {
            min_personal_food: 5,
            max_personal_food: 20,
            min_personal_tools: 2,
            hoarding_tendency: 0.0, // Default is to share
        }
    }
}

impl StoragePreferences {
    /// Create preferences based on agent traits
    pub fn from_traits(traits: &[crate::agents::Trait]) -> Self {
        let mut prefs = Self::default();

        // Diligent agents keep less personal stock (trust the community)
        if traits.contains(&crate::agents::Trait::Diligent) {
            prefs.min_personal_food = 3;
            prefs.max_personal_food = 15;
            prefs.hoarding_tendency = -0.2;
        }

        // Lazy agents might hoard more
        if traits.contains(&crate::agents::Trait::Lazy) {
            prefs.min_personal_food = 8;
            prefs.max_personal_food = 30;
            prefs.hoarding_tendency = 0.3;
        }

        // Suspicious agents don't trust communal storage as much
        if traits.contains(&crate::agents::Trait::Suspicious) {
            prefs.min_personal_food = 10;
            prefs.max_personal_food = 40;
            prefs.hoarding_tendency = 0.5;
        }

        // Trusting agents are happy to share
        if traits.contains(&crate::agents::Trait::Trusting) {
            prefs.min_personal_food = 3;
            prefs.max_personal_food = 12;
            prefs.hoarding_tendency = -0.3;
        }

        prefs
    }
}

/// Determine what storage action an agent should take
pub fn decide_storage_action(
    agent_food_count: u32,
    agent_resource_count: u32,
    agent_tool_count: u32,
    storehouse_food: u32,
    storehouse_resources: u32,
    preparedness_drive: f32,
    preferences: &StoragePreferences,
) -> StorageDecision {
    // Priority 1: Ensure personal survival (food)
    if agent_food_count < preferences.min_personal_food && storehouse_food > 10 {
        let amount = (preferences.min_personal_food - agent_food_count).min(storehouse_food / 2);
        return StorageDecision::Retrieve {
            item_type: ItemType::Food,
            quantity: amount,
            reason: format!("Personal food low ({} < {})", agent_food_count, preferences.min_personal_food),
        };
    }

    // Priority 2: Deposit excess food
    if agent_food_count > preferences.max_personal_food {
        let excess = agent_food_count - preferences.max_personal_food;
        // Keep some extra if hoarding tendency is high
        let hoard_reduction = (excess as f32 * preferences.hoarding_tendency) as u32;
        let deposit_amount = excess.saturating_sub(hoard_reduction);

        if deposit_amount > 0 {
            return StorageDecision::Deposit {
                item_type: ItemType::Food,
                quantity: deposit_amount,
                reason: format!("Excess food ({} > {})", agent_food_count, preferences.max_personal_food),
            };
        }
    }

    // Priority 3: Deposit raw resources (high Preparedness drive)
    if preparedness_drive > 0.6 && agent_resource_count > 10 {
        // Deposit most resources, keeping a small buffer
        let keep_amount = if preferences.hoarding_tendency > 0.5 { 10 } else { 5 };
        let deposit_amount = agent_resource_count.saturating_sub(keep_amount);

        if deposit_amount > 0 {
            return StorageDecision::Deposit {
                item_type: ItemType::Wood, // Generic resource deposit
                quantity: deposit_amount,
                reason: format!("Stockpiling for community (Preparedness: {:.2})", preparedness_drive),
            };
        }
    }

    // Priority 4: Retrieve tools if needed
    if agent_tool_count < preferences.min_personal_tools && storehouse_resources > 20 {
        return StorageDecision::Retrieve {
            item_type: ItemType::WoodenAxe,
            quantity: 1,
            reason: "Need tools for work".to_string(),
        };
    }

    StorageDecision::NoAction {
        reason: "Inventory balanced".to_string(),
    }
}

/// Calculate storage priority (0.0 to 1.0)
/// Higher means storage management is more urgent
pub fn calculate_storage_priority(
    agent_food_count: u32,
    inventory_fullness: f32, // 0.0 to 1.0
    preparedness_drive: f32,
    preferences: &StoragePreferences,
) -> f32 {
    let mut priority = 0.0;

    // Very urgent if food is critically low
    if agent_food_count < preferences.min_personal_food / 2 {
        priority += 0.8;
    } else if agent_food_count < preferences.min_personal_food {
        priority += 0.5;
    }

    // Urgent if inventory is very full
    if inventory_fullness > 0.9 {
        priority += 0.6;
    } else if inventory_fullness > 0.75 {
        priority += 0.3;
    }

    // Moderate priority from high Preparedness drive
    if preparedness_drive > 0.7 {
        priority += 0.3;
    } else if preparedness_drive > 0.5 {
        priority += 0.15;
    }

    priority.min(1.0)
}

/// Calculate how much food the community should stockpile
pub fn calculate_target_stockpile(
    population_size: usize,
    current_stockpile: u32,
) -> u32 {
    // Target: 30 food per person minimum, 60 ideal
    let minimum_target = (population_size as u32) * 30;
    let ideal_target = (population_size as u32) * 60;

    if current_stockpile < minimum_target {
        minimum_target
    } else {
        ideal_target
    }
}

/// Check if communal storage is critically low
pub fn is_storage_critical(
    storehouse_food: u32,
    population_size: usize,
) -> bool {
    let minimum_per_person = 10;
    let minimum_total = (population_size as u32) * minimum_per_person;

    storehouse_food < minimum_total
}

/// Check if agent should prioritize gathering over other activities
pub fn should_prioritize_gathering(
    storehouse_food: u32,
    storehouse_resources: u32,
    population_size: usize,
) -> bool {
    // Food is critically low
    if is_storage_critical(storehouse_food, population_size) {
        return true;
    }

    // Resources are very low and population is growing
    if storehouse_resources < (population_size as u32 * 20) && population_size > 3 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_preferences_default() {
        let prefs = StoragePreferences::default();
        assert_eq!(prefs.min_personal_food, 5);
        assert_eq!(prefs.max_personal_food, 20);
        assert_eq!(prefs.hoarding_tendency, 0.0);
    }

    #[test]
    fn test_storage_preferences_from_traits() {
        let suspicious = vec![crate::agents::Trait::Suspicious];
        let prefs = StoragePreferences::from_traits(&suspicious);
        assert!(prefs.hoarding_tendency > 0.0);
        assert!(prefs.min_personal_food > 5);
    }

    #[test]
    fn test_decide_storage_low_food() {
        let decision = decide_storage_action(
            2, 10, 2, // agent has 2 food
            100, 50,  // storehouse has plenty
            0.5,
            &StoragePreferences::default(),
        );

        match decision {
            StorageDecision::Retrieve { item_type, .. } => {
                assert_eq!(item_type, ItemType::Food);
            }
            _ => panic!("Expected retrieve decision for low food"),
        }
    }

    #[test]
    fn test_decide_storage_excess_food() {
        let decision = decide_storage_action(
            25, 10, 2, // agent has 25 food (excess)
            50, 50,
            0.5,
            &StoragePreferences::default(),
        );

        match decision {
            StorageDecision::Deposit { item_type, quantity, .. } => {
                assert_eq!(item_type, ItemType::Food);
                assert!(quantity > 0);
            }
            _ => panic!("Expected deposit decision for excess food"),
        }
    }

    #[test]
    fn test_storage_priority_critical_food() {
        let priority = calculate_storage_priority(
            1, // very low food
            0.5,
            0.3,
            &StoragePreferences::default(),
        );

        assert!(priority > 0.7); // Should be high priority
    }

    #[test]
    fn test_target_stockpile() {
        let target = calculate_target_stockpile(10, 100);
        assert!(target >= 300); // At least 30 per person
    }

    #[test]
    fn test_is_storage_critical() {
        assert!(is_storage_critical(50, 10)); // 50 food for 10 people is critical
        assert!(!is_storage_critical(500, 10)); // 500 food for 10 people is fine
    }

    #[test]
    fn test_should_prioritize_gathering() {
        assert!(should_prioritize_gathering(50, 100, 10)); // Low food
        assert!(!should_prioritize_gathering(500, 500, 10)); // Plenty of everything
    }
}
