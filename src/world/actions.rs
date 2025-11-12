// src/world/actions.rs
//! Action execution system for agent interactions with the world.

use serde::{Deserialize, Serialize};
use crate::world::{World, Position, ResourceType, BuildingType, ItemType};
use uuid::Uuid;

/// Actions that agents can perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Move to a position
    MoveTo { destination: Position },

    /// Harvest a resource
    HarvestResource {
        resource_position: Position,
        resource_type: ResourceType,
        amount: u32,
    },

    /// Deposit items in storehouse
    DepositItems {
        item_type: ItemType,
        amount: u32,
    },

    /// Retrieve items from storehouse
    RetrieveItems {
        item_type: ItemType,
        amount: u32,
    },

    /// Construct a building
    ConstructBuilding {
        building_type: BuildingType,
        position: Position,
    },

    /// Work on building construction
    WorkOnConstruction {
        building_position: Position,
        work_amount: u32,
    },

    /// Craft an item
    CraftItem {
        item_type: ItemType,
        quantity: u32,
    },

    /// Rest/idle
    Rest { duration: u32 },
}

/// Result of action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    Success { message: String },
    SuccessWithItems { message: String, item_type: ItemType, quantity: u32 },
    Failure { reason: String },
    Partial { completed: f32, message: String },
}

impl ActionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ActionResult::Success { .. } | ActionResult::SuccessWithItems { .. })
    }

    /// Extract harvested items from the result, if any
    pub fn take_items(&self) -> Option<(ItemType, u32)> {
        match self {
            ActionResult::SuccessWithItems { item_type, quantity, .. } => {
                Some((*item_type, *quantity))
            }
            _ => None,
        }
    }
}

impl World {
    /// Execute an action for an agent
    /// occupied_positions: List of positions currently occupied by other agents (for collision detection)
    pub fn execute_action(&mut self, agent_id: Uuid, agent_position: &mut Position, action: &Action, occupied_positions: &[Position]) -> ActionResult {
        match action {
            Action::HarvestResource {
                resource_position,
                resource_type,
                amount,
            } => self.execute_harvest(agent_position, resource_position, *resource_type, *amount),

            Action::DepositItems { item_type, amount } => {
                self.execute_deposit(agent_id, *item_type, *amount)
            }

            Action::RetrieveItems { item_type, amount } => {
                self.execute_retrieve(agent_id, *item_type, *amount)
            }

            Action::MoveTo { destination } => {
                self.execute_move(agent_position, destination, occupied_positions)
            }

            Action::WorkOnConstruction {
                building_position,
                work_amount,
            } => self.execute_construction_work(building_position, *work_amount),

            Action::Rest { duration } => ActionResult::Success {
                message: format!("Rested for {} ticks", duration),
            },

            _ => ActionResult::Failure {
                reason: "Action not yet implemented".to_string(),
            },
        }
    }

    fn execute_harvest(
        &mut self,
        agent_position: &Position,
        resource_position: &Position,
        resource_type: ResourceType,
        amount: u32,
    ) -> ActionResult {
        // Check if agent is near resource
        if agent_position.distance_to(resource_position) > 1 {
            return ActionResult::Failure {
                reason: "Too far from resource".to_string(),
            };
        }

        // Find and harvest resource
        if let Some(resource_node) = self.get_resource_at_mut(resource_position) {
            if resource_node.resource_type != resource_type {
                return ActionResult::Failure {
                    reason: "Wrong resource type".to_string(),
                };
            }

            let harvested = resource_node.harvest(amount);

            if harvested > 0 {
                // Convert resource to item type
                let item_type = match resource_type {
                    // Basic
                    ResourceType::Wood => ItemType::Wood,
                    ResourceType::Stone => ItemType::Stone,
                    ResourceType::Iron => ItemType::Iron,
                    ResourceType::Food => ItemType::Food,

                    // Agricultural
                    ResourceType::Grain => ItemType::Grain,
                    ResourceType::Flax => ItemType::Flax,
                    ResourceType::Herbs => ItemType::Herbs,
                    ResourceType::Cotton => ItemType::Cotton,

                    // Animal
                    ResourceType::Hides => ItemType::Hides,
                    ResourceType::Wool => ItemType::Wool,
                    ResourceType::Meat => ItemType::Meat,
                    ResourceType::Milk => ItemType::Milk,
                    ResourceType::Fish => ItemType::Fish,
                    ResourceType::Honey => ItemType::Honey,

                    // Mineral
                    ResourceType::Clay => ItemType::Clay,
                    ResourceType::Sand => ItemType::Sand,
                    ResourceType::Coal => ItemType::Coal,

                    // Processed
                    ResourceType::Flour => ItemType::Flour,
                    ResourceType::Leather => ItemType::Leather,
                    ResourceType::Cloth => ItemType::Cloth,
                    ResourceType::Linen => ItemType::Linen,
                    ResourceType::Glass => ItemType::Glass,
                    ResourceType::Bricks => ItemType::Bricks,
                    ResourceType::Charcoal => ItemType::Charcoal,
                    ResourceType::Rope => ItemType::Rope,
                    ResourceType::Paper => ItemType::Paper,
                    ResourceType::Dye => ItemType::Dye,

                    // Finished Food
                    ResourceType::Bread => ItemType::Bread,
                    ResourceType::Ale => ItemType::Ale,
                    ResourceType::Cheese => ItemType::Cheese,

                    // Finished Goods
                    ResourceType::Clothing => ItemType::Clothing,
                    ResourceType::Shoes => ItemType::Shoes,
                    ResourceType::Tools => ItemType::WoodenAxe, // Default tool
                    ResourceType::Weapons => ItemType::WoodenSpear, // Default weapon
                    ResourceType::Armor => ItemType::LeatherArmor, // Default armor
                    ResourceType::Pottery => ItemType::Pottery,
                    ResourceType::Furniture => ItemType::Furniture,
                    ResourceType::Jewelry => ItemType::Jewelry,
                };

                // Food goes to agent inventory (returned in result)
                // Other resources go to storehouse
                if resource_type == ResourceType::Food {
                    ActionResult::SuccessWithItems {
                        message: format!("Harvested {} {:?}", harvested, resource_type),
                        item_type,
                        quantity: harvested,
                    }
                } else {
                    // Non-food resources go directly to storehouse
                    self.storehouse_inventory.add_item(item_type, harvested);

                    ActionResult::SuccessWithItems {
                        message: format!("Harvested {} {:?} to storehouse", harvested, resource_type),
                        item_type,
                        quantity: 0, // Already deposited
                    }
                }
            } else {
                ActionResult::Failure {
                    reason: "Resource depleted".to_string(),
                }
            }
        } else {
            ActionResult::Failure {
                reason: "Resource not found".to_string(),
            }
        }
    }

    fn execute_deposit(&mut self, _agent_id: Uuid, item_type: ItemType, amount: u32) -> ActionResult {
        // In full implementation, would take from agent inventory
        // For now, assume agent has items
        if self.storehouse_inventory.add_item(item_type, amount) {
            ActionResult::Success {
                message: format!("Deposited {} {:?}", amount, item_type),
            }
        } else {
            ActionResult::Failure {
                reason: "Storehouse full".to_string(),
            }
        }
    }

    fn execute_retrieve(&mut self, _agent_id: Uuid, item_type: ItemType, amount: u32) -> ActionResult {
        // In full implementation, would add to agent inventory
        if self.storehouse_inventory.remove_item(&item_type, amount) {
            ActionResult::Success {
                message: format!("Retrieved {} {:?}", amount, item_type),
            }
        } else {
            ActionResult::Failure {
                reason: "Not enough items in storehouse".to_string(),
            }
        }
    }

    fn execute_move(&self, agent_position: &mut Position, destination: &Position, occupied_positions: &[Position]) -> ActionResult {
        // Check if already at destination
        if agent_position == destination {
            return ActionResult::Success {
                message: "Already at destination".to_string(),
            };
        }

        // Try direct movement first (one step towards destination)
        let dx = (destination.x - agent_position.x).signum();
        let dy = (destination.y - agent_position.y).signum();
        let direct_pos = Position::new(agent_position.x + dx, agent_position.y + dy);

        // Check if direct path is clear
        let direct_blocked = occupied_positions.contains(&direct_pos) ||
            self.grid.get_tile(&direct_pos).map(|t| !t.terrain.is_walkable()).unwrap_or(true);

        if !direct_blocked {
            // Direct path is clear, move there
            *agent_position = direct_pos;
            return ActionResult::Success {
                message: format!("Moved to ({}, {})", direct_pos.x, direct_pos.y),
            };
        }

        // Direct path blocked, use pathfinding to route around obstacles
        if let Some(next_pos) = self.grid.find_path_with_agents(agent_position, destination, occupied_positions) {
            *agent_position = next_pos;
            return ActionResult::Success {
                message: format!("Pathfinding: moved to ({}, {})", next_pos.x, next_pos.y),
            };
        }

        // No path found
        ActionResult::Failure {
            reason: "No path to destination (blocked)".to_string(),
        }
    }

    fn execute_construction_work(&mut self, building_position: &Position, work_amount: u32) -> ActionResult {
        // Find building under construction
        if let Some(building) = self.buildings.iter_mut().find(|b| &b.position == building_position) {
            if building.add_construction_progress(work_amount) {
                ActionResult::Success {
                    message: format!("Completed construction of {:?}", building.building_type),
                }
            } else {
                ActionResult::Partial {
                    completed: 0.5, // Could calculate actual percentage
                    message: format!("Worked on {:?} construction", building.building_type),
                }
            }
        } else {
            ActionResult::Failure {
                reason: "Building not found".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldConfig;

    #[test]
    fn test_harvest_action() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();

        // Find a resource
        if let Some(resource) = world.resources.first() {
            let resource_pos = resource.position;
            let mut agent_pos = resource_pos; // Stand right on it

            let action = Action::HarvestResource {
                resource_position: resource_pos,
                resource_type: resource.resource_type,
                amount: 10,
            };

            let occupied = vec![]; // No other agents in test
            let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied);
            assert!(result.is_success());
        }
    }

    #[test]
    fn test_move_action() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();

        let mut agent_pos = Position::new(10, 10);
        let destination = Position::new(12, 12);

        let action = Action::MoveTo { destination };

        let occupied = vec![]; // No other agents
        let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied);
        assert!(result.is_success());

        // Agent should have moved one step closer
        assert!(agent_pos.distance_to(&destination) < Position::new(10, 10).distance_to(&destination));
    }

    #[test]
    fn test_deposit_retrieve() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();
        let mut agent_pos = Position::new(10, 10);

        let occupied = vec![]; // No other agents

        // Deposit
        let deposit_action = Action::DepositItems {
            item_type: ItemType::Wood,
            amount: 50,
        };
        let result = world.execute_action(agent_id, &mut agent_pos, &deposit_action, &occupied);
        assert!(result.is_success());
        assert_eq!(world.storehouse_inventory.count_item(&ItemType::Wood), 50);

        // Retrieve
        let retrieve_action = Action::RetrieveItems {
            item_type: ItemType::Wood,
            amount: 20,
        };
        let result = world.execute_action(agent_id, &mut agent_pos, &retrieve_action, &occupied);
        assert!(result.is_success());
        assert_eq!(world.storehouse_inventory.count_item(&ItemType::Wood), 30);
    }
}
