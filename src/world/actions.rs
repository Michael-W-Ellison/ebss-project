// src/world/actions.rs
//! Action execution system for agent interactions with the world.

use serde::{Deserialize, Serialize};
use crate::world::{World, Position, ResourceType, BuildingType, ItemType, Building};
use crate::agents::social_interactions::SocialInteractionType;
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
        worker_skill: i32, // Construction skill of the worker
    },

    /// Craft an item
    CraftItem {
        item_type: ItemType,
        quantity: u32,
    },

    /// Rest/idle
    Rest { duration: u32 },

    /// Perform a social interaction with another agent
    SocialInteraction {
        target_agent_id: Uuid,
        interaction_type: SocialInteractionType,
    },

    /// Move towards another agent to socialize
    SeekSocialInteraction {
        target_agent_id: Uuid,
    },
}

/// Result of action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionResult {
    Success { message: String },
    SuccessWithItems { message: String, item_type: ItemType, quantity: u32 },
    Failure { reason: String },
    Partial { completed: f32, message: String },
    SocialSuccess {
        message: String,
        relationship_change: i8,
        trust_change: i8,
        social_satisfaction: f32,
    },
}

impl ActionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ActionResult::Success { .. } | ActionResult::SuccessWithItems { .. } | ActionResult::SocialSuccess { .. })
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

    /// Extract social satisfaction from the result, if any
    pub fn social_satisfaction(&self) -> f32 {
        match self {
            ActionResult::SocialSuccess { social_satisfaction, .. } => *social_satisfaction,
            _ => 0.0,
        }
    }

    /// Extract relationship change from the result, if any
    pub fn relationship_change(&self) -> (i8, i8) {
        match self {
            ActionResult::SocialSuccess { relationship_change, trust_change, .. } => {
                (*relationship_change, *trust_change)
            }
            _ => (0, 0),
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
                worker_skill,
            } => self.execute_construction_work(building_position, *work_amount, *worker_skill),

            Action::Rest { duration } => ActionResult::Success {
                message: format!("Rested for {} ticks", duration),
            },

            Action::CraftItem { item_type, quantity } => {
                self.execute_craft(agent_id, *item_type, *quantity)
            }

            Action::ConstructBuilding { building_type, position } => {
                self.execute_construct_building(*building_type, position)
            }

            Action::SocialInteraction { target_agent_id, interaction_type } => {
                self.execute_social_interaction(agent_id, *target_agent_id, interaction_type)
            }

            Action::SeekSocialInteraction { target_agent_id } => {
                self.execute_seek_social(*target_agent_id, agent_position, occupied_positions)
            }
        }
    }

    fn execute_craft(
        &mut self,
        agent_id: Uuid,
        item_type: ItemType,
        quantity: u32,
    ) -> ActionResult {
        use crate::world::crafting::ToolRequirement;

        // Map ItemType to recipe ID
        let recipe_id = match item_type {
            ItemType::WoodenAxe => "stone_axe",
            ItemType::WoodenSpear => "stone_spear",
            ItemType::StoneAxe => "stone_axe",
            ItemType::StonePickaxe => "stone_pickaxe",
            ItemType::IronSword => "iron_sword",
            ItemType::IronAxe => "iron_axe",
            ItemType::IronPickaxe => "iron_pickaxe",
            ItemType::LeatherArmor => "leather_chestplate",
            ItemType::Clothing => "simple_tunic",
            ItemType::Furniture => "wooden_chest",
            _ => {
                return ActionResult::Failure {
                    reason: format!("No recipe for {:?}", item_type),
                };
            }
        };

        // Get recipe from crafting manager
        let recipe = match self.crafting_manager.get_recipe(recipe_id) {
            Some(r) => r.clone(),
            None => {
                return ActionResult::Failure {
                    reason: format!("Recipe '{}' not found", recipe_id),
                };
            }
        };

        // Check materials from storehouse
        let mut materials_available = std::collections::HashMap::new();
        for mat_req in &recipe.materials {
            // Map material string to ItemType
            let item = match mat_req.material_id.as_str() {
                "wood" => ItemType::Wood,
                "stone" => ItemType::Stone,
                "iron_ingot" | "iron" => ItemType::Iron,
                "leather" => ItemType::Hides, // Using Hides as leather source
                "cloth" => ItemType::Cloth,
                "thread" => ItemType::Cloth, // Simplify - treat thread as cloth
                "planks" => ItemType::Wood,
                "wool" => ItemType::Wool,
                _ => {
                    return ActionResult::Failure {
                        reason: format!("Unknown material: {}", mat_req.material_id),
                    };
                }
            };
            let count = self.storehouse_inventory.count_item(&item);
            materials_available.insert(mat_req.material_id.clone(), count);
        }

        // Check tool requirement
        let has_tool = match recipe.tool_requirement {
            ToolRequirement::None => true,
            ToolRequirement::Workbench => self.buildings.iter().any(|b|
                matches!(b.building_type, crate::world::BuildingType::Workshop) && b.is_completed()
            ),
            ToolRequirement::Anvil | ToolRequirement::Furnace => self.buildings.iter().any(|b|
                matches!(b.building_type, crate::world::BuildingType::Smithy) && b.is_completed()
            ),
            ToolRequirement::Loom => self.buildings.iter().any(|b|
                matches!(b.building_type, crate::world::BuildingType::Workshop) && b.is_completed()
            ),
            ToolRequirement::Tannery => self.buildings.iter().any(|b|
                matches!(b.building_type, crate::world::BuildingType::Workshop) && b.is_completed()
            ),
            ToolRequirement::Alchemy => false, // No alchemy station yet
        };

        if !has_tool {
            return ActionResult::Failure {
                reason: format!("Missing required tool/building: {:?}", recipe.tool_requirement),
            };
        }

        // Check and consume materials
        for mat_req in &recipe.materials {
            let available = materials_available.get(&mat_req.material_id).copied().unwrap_or(0);
            if available < mat_req.quantity * quantity {
                return ActionResult::Failure {
                    reason: format!(
                        "Insufficient {}: need {}, have {}",
                        mat_req.material_id,
                        mat_req.quantity * quantity,
                        available
                    ),
                };
            }
        }

        // Consume materials from storehouse
        for mat_req in &recipe.materials {
            let item = match mat_req.material_id.as_str() {
                "wood" | "planks" => ItemType::Wood,
                "stone" => ItemType::Stone,
                "iron_ingot" | "iron" => ItemType::Iron,
                "leather" => ItemType::Hides,
                "cloth" | "thread" => ItemType::Cloth,
                "wool" => ItemType::Wool,
                _ => continue,
            };
            self.storehouse_inventory.remove_item(&item, mat_req.quantity * quantity);
        }

        // Add crafted items to storehouse
        self.storehouse_inventory.add_item(item_type, quantity * recipe.output_quantity);

        // Start crafting job (for tracking, even though it completes instantly here)
        let _ = self.crafting_manager.start_crafting(recipe_id.to_string(), agent_id);

        ActionResult::SuccessWithItems {
            message: format!("Crafted {} x{}", recipe.name, quantity * recipe.output_quantity),
            item_type,
            quantity: quantity * recipe.output_quantity,
        }
    }

    fn execute_construct_building(
        &mut self,
        building_type: BuildingType,
        position: &Position,
    ) -> ActionResult {
        // Check if terrain is valid for building
        if let Some(tile) = self.grid.get_tile(position) {
            if !tile.terrain.is_walkable() {
                return ActionResult::Failure {
                    reason: format!("Cannot build on {:?} terrain", tile.terrain.terrain_type),
                };
            }
        } else {
            return ActionResult::Failure {
                reason: "Position out of bounds".to_string(),
            };
        }

        // Check if there's already a building at this position
        if self.buildings.iter().any(|b| b.position == *position) {
            return ActionResult::Failure {
                reason: "A building already exists at this location".to_string(),
            };
        }

        // Check prerequisites
        let prerequisites = building_type.prerequisites();
        for prereq in &prerequisites {
            let has_prereq = self.buildings.iter().any(|b| {
                b.building_type == *prereq && b.is_completed()
            });
            if !has_prereq {
                return ActionResult::Failure {
                    reason: format!("Missing prerequisite building: {:?}", prereq),
                };
            }
        }

        // Create the building under construction
        let building = Building::new_under_construction(building_type, *position);
        self.buildings.push(building);

        ActionResult::Success {
            message: format!(
                "Started construction of {:?} at ({}, {})",
                building_type, position.x, position.y
            ),
        }
    }

    fn execute_craft_item(
        &mut self,
        agent_id: Uuid,
        item_type: ItemType,
        quantity: u32,
    ) -> ActionResult {
        // Map ItemType to recipe ID
        let recipe_id = match item_type {
            ItemType::WoodenAxe => "stone_axe",
            ItemType::StoneAxe => "stone_axe",
            ItemType::IronAxe => "iron_axe",
            ItemType::WoodenSpear => "stone_spear",
            ItemType::Leather => "leather",
            ItemType::Cloth => "simple_tunic",
            ItemType::Furniture => "wooden_chest",
            _ => {
                return ActionResult::Failure {
                    reason: format!("No recipe found for {:?}", item_type),
                };
            }
        };

        // Start crafting job
        if let Some(job_id) = self.crafting_manager.start_crafting(recipe_id.to_string(), agent_id) {
            ActionResult::Success {
                message: format!(
                    "Started crafting {} x{} (job {})",
                    recipe_id, quantity, job_id
                ),
            }
        } else {
            ActionResult::Failure {
                reason: format!("Failed to start crafting {}", recipe_id),
            }
        }
    }

    fn execute_social_interaction(
        &self,
        initiator_id: Uuid,
        target_id: Uuid,
        interaction_type: &SocialInteractionType,
    ) -> ActionResult {
        // Calculate base relationship change based on interaction type
        let (relationship_change, trust_change, social_satisfaction) = match interaction_type {
            SocialInteractionType::Greet => (1, 0, 0.05),
            SocialInteractionType::Converse { topic } => match topic {
                crate::agents::social_interactions::ConversationTopic::SmallTalk => (1, 0, 0.10),
                crate::agents::social_interactions::ConversationTopic::Work => (1, 1, 0.08),
                crate::agents::social_interactions::ConversationTopic::Stories => (2, 1, 0.15),
                crate::agents::social_interactions::ConversationTopic::Family => (3, 2, 0.20),
                crate::agents::social_interactions::ConversationTopic::Technology => (2, 1, 0.12),
                crate::agents::social_interactions::ConversationTopic::Beliefs => (1, 0, 0.10),
            },
            SocialInteractionType::GiveGift { quantity, .. } => {
                let gift_bonus = ((*quantity as f32 / 10.0).min(5.0) as i8).max(1);
                (gift_bonus, gift_bonus / 2, 0.15)
            }
            SocialInteractionType::OfferHelp { .. } => (2, 2, 0.10),
            SocialInteractionType::ThankYou => (1, 1, 0.05),
            SocialInteractionType::Compliment => (2, 0, 0.08),
            SocialInteractionType::ShareMeal => (3, 2, 0.25),
        };

        ActionResult::SocialSuccess {
            message: format!(
                "Agent {} performed {:?} with agent {}",
                initiator_id, interaction_type, target_id
            ),
            relationship_change,
            trust_change,
            social_satisfaction,
        }
    }

    fn execute_seek_social(
        &self,
        target_agent_id: Uuid,
        _agent_position: &Position,
        _occupied_positions: &[Position],
    ) -> ActionResult {
        // SeekSocialInteraction is a planning action - actual movement should use MoveTo
        ActionResult::Failure {
            reason: format!(
                "SeekSocialInteraction with {} requires using MoveTo to approach target",
                target_agent_id
            ),
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

        // Check visibility - poor weather makes harvesting less efficient
        let visibility_reduction = self.climate.weather.visibility_reduction();
        let effective_amount = if visibility_reduction > 0.5 {
            // Severe visibility reduction cuts harvest efficiency
            ((amount as f32) * (1.0 - visibility_reduction * 0.5)) as u32
        } else {
            amount
        };

        // Find and harvest resource
        if let Some(resource_node) = self.get_resource_at_mut(resource_position) {
            if resource_node.resource_type != resource_type {
                return ActionResult::Failure {
                    reason: "Wrong resource type".to_string(),
                };
            }

            let harvested = resource_node.harvest(effective_amount);

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

        // Apply weather movement modifier (may prevent movement in severe weather)
        let movement_modifier = self.climate.movement_modifier();

        // Severe weather may completely prevent movement
        if movement_modifier < 0.3 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen::<f32>() > movement_modifier / 0.3 {
                return ActionResult::Failure {
                    reason: format!("Severe weather ({:?}) prevents movement", self.climate.weather.weather_type),
                };
            }
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
            let message = if movement_modifier < 0.8 {
                format!("Moved to ({}, {}) (slowed by weather)", direct_pos.x, direct_pos.y)
            } else {
                format!("Moved to ({}, {})", direct_pos.x, direct_pos.y)
            };
            return ActionResult::Success { message };
        }

        // Direct path blocked, use pathfinding to route around obstacles
        if let Some(next_pos) = self.grid.find_path_with_agents(agent_position, destination, occupied_positions) {
            *agent_position = next_pos;
            let message = if movement_modifier < 0.8 {
                format!("Pathfinding: moved to ({}, {}) (slowed by weather)", next_pos.x, next_pos.y)
            } else {
                format!("Pathfinding: moved to ({}, {})", next_pos.x, next_pos.y)
            };
            return ActionResult::Success { message };
        }

        // No path found
        ActionResult::Failure {
            reason: "No path to destination (blocked)".to_string(),
        }
    }

    fn execute_construction_work(&mut self, building_position: &Position, work_amount: u32, worker_skill: i32) -> ActionResult {
        // Find building under construction
        if let Some(building) = self.buildings.iter_mut().find(|b| &b.position == building_position) {
            // Check if building has required resources
            if !building.has_all_resources() {
                let missing = building.missing_resources();
                return ActionResult::Failure {
                    reason: format!(
                        "Missing resources: {}",
                        missing.iter()
                            .map(|r| format!("{:?} x{}", r.resource_type, r.amount))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                };
            }

            if building.add_construction_progress(work_amount, worker_skill) {
                ActionResult::Success {
                    message: format!("Completed construction of {:?}", building.building_type),
                }
            } else {
                let progress = building.construction_progress();
                ActionResult::Partial {
                    completed: progress,
                    message: format!(
                        "Worked on {:?} construction ({:.0}% complete)",
                        building.building_type,
                        progress * 100.0
                    ),
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

    #[test]
    fn test_construct_building_action() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();
        let mut agent_pos = Position::new(10, 10);
        let build_pos = Position::new(15, 15);

        let occupied = vec![];

        // Construct a building (basic buildings have no prerequisites)
        let action = Action::ConstructBuilding {
            building_type: BuildingType::SmallHouse,
            position: build_pos,
        };

        let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied);
        assert!(result.is_success());

        // Verify building was created
        assert!(world.buildings.iter().any(|b|
            b.position == build_pos &&
            b.building_type == BuildingType::SmallHouse &&
            !b.is_completed()
        ));
    }

    #[test]
    fn test_construct_building_duplicate_position() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();
        let mut agent_pos = Position::new(10, 10);
        let build_pos = Position::new(15, 15);

        let occupied = vec![];

        // First construction should succeed
        let action = Action::ConstructBuilding {
            building_type: BuildingType::SmallHouse,
            position: build_pos,
        };
        let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied);
        assert!(result.is_success());

        // Second construction at same position should fail
        let action2 = Action::ConstructBuilding {
            building_type: BuildingType::Workshop,
            position: build_pos,
        };
        let result = world.execute_action(agent_id, &mut agent_pos, &action2, &occupied);
        assert!(!result.is_success());
    }

    #[test]
    fn test_craft_item_action() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();
        let mut agent_pos = Position::new(10, 10);

        // Add materials needed for stone axe (2 wood, 3 stone)
        world.storehouse_inventory.add_item(ItemType::Wood, 10);
        world.storehouse_inventory.add_item(ItemType::Stone, 10);

        let occupied = vec![];

        let action = Action::CraftItem {
            item_type: ItemType::StoneAxe,
            quantity: 1,
        };

        let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied);
        assert!(result.is_success());
    }

    #[test]
    fn test_social_interaction_greet() {
        let world = World::new(WorldConfig::default());
        let initiator_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        let interaction_type = SocialInteractionType::Greet;
        let result = world.execute_social_interaction(initiator_id, target_id, &interaction_type);

        match result {
            ActionResult::SocialSuccess { relationship_change, social_satisfaction, .. } => {
                assert_eq!(relationship_change, 1);
                assert!(social_satisfaction > 0.0);
            }
            _ => panic!("Expected SocialSuccess result"),
        }
    }

    #[test]
    fn test_social_interaction_share_meal() {
        let world = World::new(WorldConfig::default());
        let initiator_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        let interaction_type = SocialInteractionType::ShareMeal;
        let result = world.execute_social_interaction(initiator_id, target_id, &interaction_type);

        match result {
            ActionResult::SocialSuccess { relationship_change, trust_change, social_satisfaction, .. } => {
                assert_eq!(relationship_change, 3);
                assert_eq!(trust_change, 2);
                assert!(social_satisfaction >= 0.25);
            }
            _ => panic!("Expected SocialSuccess result"),
        }
    }

    #[test]
    fn test_social_interaction_give_gift() {
        let world = World::new(WorldConfig::default());
        let initiator_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        let interaction_type = SocialInteractionType::GiveGift {
            item_type: ItemType::Jewelry,
            quantity: 30,
        };
        let result = world.execute_social_interaction(initiator_id, target_id, &interaction_type);

        match result {
            ActionResult::SocialSuccess { relationship_change, .. } => {
                // quantity 30 / 10 = 3, min(3, 5) = 3, max(3, 1) = 3
                assert_eq!(relationship_change, 3);
            }
            _ => panic!("Expected SocialSuccess result"),
        }
    }

    #[test]
    fn test_seek_social_interaction_fails() {
        let mut world = World::new(WorldConfig::default());
        let agent_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let mut agent_pos = Position::new(10, 10);

        let occupied = vec![];

        // SeekSocialInteraction should fail when executed directly
        let action = Action::SeekSocialInteraction {
            target_agent_id: target_id,
        };

        let result = world.execute_action(agent_id, &mut agent_pos, &action, &occupied);
        assert!(!result.is_success());
    }
}
