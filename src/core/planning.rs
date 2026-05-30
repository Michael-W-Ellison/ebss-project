// src/core/planning.rs
//! Decision-making and planning engine for agent goal decomposition.
//!
//! This module provides the planning infrastructure for agents to break down
//! high-level goals into actionable sub-tasks, calculate efficiency of different
//! methods, and learn from outcomes.
//!
//! # Architecture
//!
//! - [`Planner`]: Tracks action history and learns from outcomes
//! - [`ActionPlan`]: A sequence of steps to achieve a goal
//! - [`PlanStep`]: Individual action with timing and requirements
//! - [`PlanningContext`]: World knowledge used for plan generation
//!
//! # Learning System
//!
//! The Planner learns from completed actions via [`Planner::record_outcome()`]:
//! - Tracks average completion times for action types
//! - Calculates success rates for different actions
//! - Measures tool efficiency for specific tasks
//!
//! # Example: "Get wood" plan
//!
//! 1. Walk to forest (30 ticks)
//! 2. Equip axe (5 ticks)
//! 3. Chop tree (20 ticks with iron axe, 40 with stone)
//! 4. Return to storehouse (30 ticks)
//! 5. Deposit wood (5 ticks)
//!
//! Total: 90 ticks with iron axe vs 110 with stone axe
//!
//! # Extending the Planner
//!
//! To add new plan types, add methods similar to `plan_wood_gathering()` that:
//! 1. Use `PlanningContext` to find relevant locations
//! 2. Create `PlanStep` sequences with timing estimates
//! 3. Use learned data via `get_average_time()` and `get_success_rate()`

use serde::{Deserialize, Serialize};
use crate::core::{Trait, ExternalGoal};

/// A single step in an action plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: PlanActionType,
    pub estimated_ticks: u32,
    pub required_tool: Option<String>,
    pub required_resources: Vec<(String, u32)>,
    pub target_location: Option<(i32, i32, i32)>,
    pub confidence: f32, // 0.0 to 1.0, based on past experience
}

/// Types of actions an agent can plan and execute.
/// This enum represents concrete, plannable actions with their parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanActionType {
    /// Move to a location
    MoveTo { location: (i32, i32, i32) },
    /// Equip a tool or item
    EquipItem { item: String },
    /// Gather a resource
    GatherResource { resource: String, amount: u32 },
    /// Craft an item
    CraftItem { item: String, count: u32 },
    /// Build a structure
    BuildStructure { structure: String },
    /// Deposit items in storage
    Deposit { resource: String, amount: u32 },
    /// Retrieve items from storage
    Retrieve { resource: String, amount: u32 },
    /// Interact socially with another agent
    Socialize { target_id: uuid::Uuid },
    /// Rest or wait
    Rest { duration: u32 },
    /// Learn a skill or recipe
    LearnSkill { skill: String },
}

/// A complete action plan with sub-tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub id: uuid::Uuid,
    pub goal_description: String,
    pub steps: Vec<PlanStep>,
    pub total_estimated_ticks: u32,
    pub current_step: usize,
    pub created_at: u32, // tick
    pub method: String, // Description of method (e.g., "using iron axe")
}

impl ActionPlan {
    /// Create a new action plan
    pub fn new(goal_description: String, steps: Vec<PlanStep>, tick: u32, method: String) -> Self {
        let total_estimated_ticks = steps.iter().map(|s| s.estimated_ticks).sum();

        Self {
            id: uuid::Uuid::new_v4(),
            goal_description,
            steps,
            total_estimated_ticks,
            current_step: 0,
            created_at: tick,
            method,
        }
    }

    /// Get the current step to execute
    pub fn current_step(&self) -> Option<&PlanStep> {
        self.steps.get(self.current_step)
    }

    /// Advance to the next step
    pub fn advance_step(&mut self) -> bool {
        if self.current_step < self.steps.len() {
            self.current_step += 1;
            self.current_step < self.steps.len() // Returns true if more steps remain
        } else {
            false // Already completed
        }
    }

    /// Check if plan is complete
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.steps.len()
    }

    /// Calculate progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.steps.is_empty() {
            return 1.0;
        }
        self.current_step as f32 / self.steps.len() as f32
    }

    /// Get complexity (number of steps)
    pub fn complexity(&self) -> usize {
        self.steps.len()
    }

    /// Check if this plan exceeds an agent's complexity limit
    pub fn exceeds_complexity_limit(&self, traits: &[Trait]) -> bool {
        let max_steps = Self::calculate_max_steps(traits);
        self.steps.len() > max_steps
    }

    /// Calculate maximum steps an agent will tolerate based on personality traits
    ///
    /// Base complexity is 10 steps. Traits apply additive modifiers:
    ///
    /// **Positive modifiers (increase complexity tolerance):**
    /// - Ambitious: +6 (tackles complex challenges)
    /// - Stubborn: +4 (persists through long plans)
    /// - Diligent: +4 (works hard on complex tasks)
    /// - Curious: +3 (willing to explore complex solutions)
    /// - Bookworm: +3 (enjoys intellectual challenges)
    /// - Brave: +2 (not intimidated by complexity)
    /// - Proud: +2 (wants to accomplish difficult goals)
    /// - Explorer: +2 (willing to try complex paths)
    ///
    /// **Negative modifiers (decrease complexity tolerance):**
    /// - Lazy: -5 (avoids complex work)
    /// - Anxious: -3 (overwhelmed by complex plans)
    /// - Coward: -2 (avoids challenging situations)
    /// - Calm/Peaceful: -1 (prefers simple, stress-free plans)
    ///
    /// **Neutral/balancing modifiers:**
    /// - Pragmatist: sets baseline to 8 (practical, efficient plans)
    ///
    /// Minimum complexity is 2 steps, maximum is 25 steps.
    fn calculate_max_steps(traits: &[Trait]) -> usize {
        let mut max_steps: i32 = 10; // Default base
        let mut has_pragmatist = false;

        for trait_item in traits {
            match trait_item {
                // Large positive modifiers
                Trait::Ambitious => max_steps += 6,
                Trait::Stubborn => max_steps += 4,
                Trait::Diligent => max_steps += 4,

                // Medium positive modifiers
                Trait::Curious => max_steps += 3,
                Trait::Bookworm => max_steps += 3,
                Trait::Brave => max_steps += 2,
                Trait::Proud => max_steps += 2,
                Trait::Explorer => max_steps += 2,

                // Small positive modifiers
                Trait::Resilient => max_steps += 1,
                Trait::Handy => max_steps += 1,

                // Large negative modifiers
                Trait::Lazy => max_steps -= 5,

                // Medium negative modifiers
                Trait::Anxious => max_steps -= 3,
                Trait::Coward => max_steps -= 2,

                // Small negative modifiers
                Trait::Calm => max_steps -= 1,
                Trait::Peaceful => max_steps -= 1,
                Trait::Ascetic => max_steps -= 1, // Prefers simplicity

                // Pragmatist sets a reasonable baseline
                Trait::Pragmatist => has_pragmatist = true,

                _ => {}
            }
        }

        // Pragmatist prefers efficient plans - cap complexity at reasonable level
        if has_pragmatist && max_steps > 12 {
            max_steps = 12;
        }

        // Clamp between 2 and 25 steps
        max_steps.clamp(2, 25) as usize
    }

    /// Get a descriptive complexity level for the agent's max steps
    pub fn complexity_description(traits: &[Trait]) -> &'static str {
        let max = Self::calculate_max_steps(traits);
        match max {
            0..=4 => "very simple",
            5..=8 => "simple",
            9..=12 => "moderate",
            13..=17 => "complex",
            18..=22 => "very complex",
            _ => "extremely complex",
        }
    }
}

/// Planning engine for generating and comparing action plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planner {
    /// Historical data for learning from outcomes
    pub action_history: Vec<ActionOutcome>,
    /// Maximum history entries to keep
    pub max_history: usize,
}

/// Record of an action's outcome for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub action_type: PlanActionType,
    pub estimated_ticks: u32,
    pub actual_ticks: u32,
    pub success: bool,
    pub tool_used: Option<String>,
    pub tick: u32,
}

/// Context for planning that provides location information.
///
/// This struct bridges the gap between the Planner (which handles action sequencing
/// and time estimation) and the agent's knowledge of the world (exploration knowledge).
#[derive(Debug, Clone, Default)]
pub struct PlanningContext {
    /// Known resource locations: (position, resource_type_name)
    pub known_resources: Vec<((i32, i32, i32), String)>,
    /// Known storage/storehouse locations
    pub known_storage: Vec<(i32, i32, i32)>,
}

impl PlanningContext {
    /// Create an empty planning context
    pub fn new() -> Self {
        Self {
            known_resources: Vec::new(),
            known_storage: Vec::new(),
        }
    }

    /// Create a planning context from exploration knowledge
    ///
    /// This converts the agent's exploration knowledge into a format
    /// usable by the planning system.
    pub fn from_exploration_knowledge(
        known_resources: &std::collections::HashMap<crate::world::Position, crate::world::ResourceType>,
        known_buildings: &std::collections::HashMap<crate::world::Position, crate::world::BuildingType>,
    ) -> Self {
        use crate::world::BuildingType;

        // Convert resources to planning format
        let resources: Vec<((i32, i32, i32), String)> = known_resources
            .iter()
            .map(|(pos, res_type)| ((pos.x, pos.y, 0), format!("{:?}", res_type).to_lowercase()))
            .collect();

        // Find storage buildings (Storehouse, TownStorage)
        let storage: Vec<(i32, i32, i32)> = known_buildings
            .iter()
            .filter(|(_, building_type)| {
                matches!(building_type, BuildingType::Storehouse | BuildingType::TownStorage)
            })
            .map(|(pos, _)| (pos.x, pos.y, 0))
            .collect();

        Self {
            known_resources: resources,
            known_storage: storage,
        }
    }

    /// Find the nearest resource location of a given type from a position.
    ///
    /// Returns None if no resource of that type is known.
    pub fn find_nearest_resource(
        &self,
        from: (i32, i32, i32),
        resource_type: &str,
    ) -> Option<(i32, i32, i32)> {
        let resource_lower = resource_type.to_lowercase();

        self.known_resources
            .iter()
            .filter(|(_, res_type)| res_type.contains(&resource_lower))
            .min_by_key(|(pos, _)| {
                let dx = (pos.0 - from.0).abs();
                let dy = (pos.1 - from.1).abs();
                dx + dy // Manhattan distance for simplicity
            })
            .map(|(pos, _)| *pos)
    }

    /// Find the nearest storage location from a position.
    ///
    /// Returns None if no storage is known.
    pub fn find_nearest_storage(&self, from: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
        self.known_storage
            .iter()
            .min_by_key(|pos| {
                let dx = (pos.0 - from.0).abs();
                let dy = (pos.1 - from.1).abs();
                dx + dy
            })
            .copied()
    }

    /// Check if context has any useful location information
    pub fn has_locations(&self) -> bool {
        !self.known_resources.is_empty() || !self.known_storage.is_empty()
    }
}

impl Planner {
    pub fn new() -> Self {
        Self {
            action_history: Vec::new(),
            max_history: 100, // Keep last 100 actions
        }
    }

    /// Record an action outcome for learning
    pub fn record_outcome(&mut self, outcome: ActionOutcome) {
        self.action_history.push(outcome);

        // Keep only recent history
        if self.action_history.len() > self.max_history {
            self.action_history.remove(0);
        }
    }

    /// Get average actual time for an action type
    pub fn get_average_time(&self, action_type: &PlanActionType) -> Option<u32> {
        let matching: Vec<&ActionOutcome> = self.action_history
            .iter()
            .filter(|o| o.success && std::mem::discriminant(&o.action_type) == std::mem::discriminant(action_type))
            .collect();

        if matching.is_empty() {
            return None;
        }

        let total: u32 = matching.iter().map(|o| o.actual_ticks).sum();
        Some(total / matching.len() as u32)
    }

    /// Get success rate for an action type
    pub fn get_success_rate(&self, action_type: &PlanActionType) -> f32 {
        let matching: Vec<&ActionOutcome> = self.action_history
            .iter()
            .filter(|o| std::mem::discriminant(&o.action_type) == std::mem::discriminant(action_type))
            .collect();

        if matching.is_empty() {
            return 0.5; // Default 50% confidence
        }

        let successes = matching.iter().filter(|o| o.success).count();
        successes as f32 / matching.len() as f32
    }

    /// Get tool efficiency (average time) for a specific tool and action
    pub fn get_tool_efficiency(&self, action_type: &PlanActionType, tool: &str) -> Option<u32> {
        let matching: Vec<&ActionOutcome> = self.action_history
            .iter()
            .filter(|o| {
                o.success
                    && std::mem::discriminant(&o.action_type) == std::mem::discriminant(action_type)
                    && o.tool_used.as_ref().map(|t| t.as_str()) == Some(tool)
            })
            .collect();

        if matching.is_empty() {
            return None;
        }

        let total: u32 = matching.iter().map(|o| o.actual_ticks).sum();
        Some(total / matching.len() as u32)
    }

    /// Generate a plan to gather wood (example)
    pub fn plan_gather_wood(
        &self,
        current_position: (i32, i32, i32),
        forest_position: (i32, i32, i32),
        storehouse_position: (i32, i32, i32),
        available_tools: &[String],
        amount: u32,
    ) -> ActionPlan {
        let mut steps = Vec::new();

        // Step 1: Move to forest
        let distance_to_forest = Self::calculate_distance(current_position, forest_position);
        let move_time = distance_to_forest as u32;
        steps.push(PlanStep {
            action: PlanActionType::MoveTo { location: forest_position },
            estimated_ticks: move_time,
            required_tool: None,
            required_resources: vec![],
            target_location: Some(forest_position),
            confidence: self.get_success_rate(&PlanActionType::MoveTo { location: forest_position }),
        });

        // Step 2: Equip best available axe
        let best_axe = Self::choose_best_tool(available_tools, &["iron_axe", "stone_axe", "wooden_axe"]);
        let equip_time = 5;
        if let Some(axe) = &best_axe {
            steps.push(PlanStep {
                action: PlanActionType::EquipItem { item: axe.clone() },
                estimated_ticks: equip_time,
                required_tool: None,
                required_resources: vec![],
                target_location: None,
                confidence: 0.95,
            });
        }

        // Step 3: Chop trees
        let chop_time = self.estimate_gathering_time(&best_axe, "wood", amount);
        steps.push(PlanStep {
            action: PlanActionType::GatherResource { resource: "wood".to_string(), amount },
            estimated_ticks: chop_time,
            required_tool: best_axe.clone(),
            required_resources: vec![],
            target_location: Some(forest_position),
            confidence: self.get_success_rate(&PlanActionType::GatherResource {
                resource: "wood".to_string(),
                amount
            }),
        });

        // Step 4: Return to storehouse
        let distance_to_storehouse = Self::calculate_distance(forest_position, storehouse_position);
        let return_time = distance_to_storehouse as u32;
        steps.push(PlanStep {
            action: PlanActionType::MoveTo { location: storehouse_position },
            estimated_ticks: return_time,
            required_tool: None,
            required_resources: vec![],
            target_location: Some(storehouse_position),
            confidence: 0.95,
        });

        // Step 5: Deposit wood
        let deposit_time = 5;
        steps.push(PlanStep {
            action: PlanActionType::Deposit { resource: "wood".to_string(), amount },
            estimated_ticks: deposit_time,
            required_tool: None,
            required_resources: vec![("wood".to_string(), amount)],
            target_location: Some(storehouse_position),
            confidence: 0.95,
        });

        let method = format!("using {}", best_axe.as_ref().unwrap_or(&"bare hands".to_string()));
        ActionPlan::new("Gather wood".to_string(), steps, 0, method)
    }

    /// Estimate gathering time based on tool and amount
    fn estimate_gathering_time(&self, tool: &Option<String>, resource: &str, amount: u32) -> u32 {
        let base_time_per_unit = match resource {
            "wood" => 20,
            "stone" => 30,
            "iron" => 40,
            "food" => 15,
            _ => 25,
        };

        // Tool efficiency multipliers
        let tool_multiplier = match tool.as_ref().map(|s| s.as_str()) {
            Some("iron_axe") => 0.5,  // 2x faster
            Some("stone_axe") => 0.75, // 1.33x faster
            Some("iron_pickaxe") => 0.5,
            Some("stone_pickaxe") => 0.75,
            _ => 1.0, // No tool
        };

        // Check historical data for more accurate estimates
        if let Some(tool_name) = tool {
            let action = PlanActionType::GatherResource {
                resource: resource.to_string(),
                amount: 1
            };
            if let Some(historical_time) = self.get_tool_efficiency(&action, tool_name) {
                return historical_time * amount;
            }
        }

        (base_time_per_unit as f32 * tool_multiplier * amount as f32) as u32
    }

    /// Choose the best tool from available options
    fn choose_best_tool(available: &[String], preferences: &[&str]) -> Option<String> {
        for pref in preferences {
            if available.iter().any(|t| t == pref) {
                return Some(pref.to_string());
            }
        }
        available.first().cloned()
    }

    /// Calculate Euclidean distance between positions
    fn calculate_distance(pos1: (i32, i32, i32), pos2: (i32, i32, i32)) -> f32 {
        let dx = (pos1.0 - pos2.0) as f32;
        let dy = (pos1.1 - pos2.1) as f32;
        let dz = (pos1.2 - pos2.2) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Generate multiple plan alternatives and choose the best one.
    ///
    /// Uses the provided `PlanningContext` to find actual resource and storage
    /// locations based on the agent's exploration knowledge. If no suitable
    /// locations are known, returns None.
    ///
    /// # Arguments
    /// * `goal` - The external goal to plan for
    /// * `current_position` - Agent's current position
    /// * `context` - Planning context with known locations from exploration
    /// * `_available_tools` - Tools available to the agent
    /// * `traits` - Agent traits that affect plan complexity limits
    pub fn generate_best_plan(
        &self,
        goal: &ExternalGoal,
        current_position: (i32, i32, i32),
        context: &PlanningContext,
        _available_tools: &[String],
        traits: &[Trait],
    ) -> Option<ActionPlan> {
        let mut plans = Vec::new();

        match goal {
            ExternalGoal::GatherResource(resource, amount) => {
                // Find resource location from context using the resource type
                let resource_location = context.find_nearest_resource(current_position, resource)?;

                // Find storage location from context
                // If no storage is known, use a position near the agent as fallback
                let storehouse = context
                    .find_nearest_storage(current_position)
                    .unwrap_or(current_position);

                // Generate plans with different tools
                for tool_option in &["iron_axe", "stone_axe", "wooden_axe", "none"] {
                    let tools = if *tool_option == "none" {
                        vec![]
                    } else {
                        vec![tool_option.to_string()]
                    };

                    let plan = self.plan_gather_wood(
                        current_position,
                        resource_location,
                        storehouse,
                        &tools,
                        *amount,
                    );

                    // Check if plan exceeds complexity limit
                    if !plan.exceeds_complexity_limit(traits) {
                        plans.push(plan);
                    }
                }
            }
            _ => {
                // Other goal types would have their own plan generators
                return None;
            }
        }

        // Choose the plan with lowest estimated time
        plans.into_iter().min_by_key(|p| p.total_estimated_ticks)
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_plan_creation() {
        let steps = vec![
            PlanStep {
                action: PlanActionType::MoveTo { location: (10, 10, 0) },
                estimated_ticks: 20,
                required_tool: None,
                required_resources: vec![],
                target_location: Some((10, 10, 0)),
                confidence: 0.9,
            },
        ];

        let plan = ActionPlan::new("Test".to_string(), steps, 0, "test method".to_string());
        assert_eq!(plan.total_estimated_ticks, 20);
        assert!(!plan.is_complete());
    }

    #[test]
    fn test_plan_progression() {
        let steps = vec![
            PlanStep {
                action: PlanActionType::MoveTo { location: (10, 10, 0) },
                estimated_ticks: 20,
                required_tool: None,
                required_resources: vec![],
                target_location: Some((10, 10, 0)),
                confidence: 0.9,
            },
            PlanStep {
                action: PlanActionType::Rest { duration: 10 },
                estimated_ticks: 10,
                required_tool: None,
                required_resources: vec![],
                target_location: None,
                confidence: 1.0,
            },
        ];

        let mut plan = ActionPlan::new("Test".to_string(), steps, 0, "test".to_string());
        assert_eq!(plan.current_step, 0);
        assert_eq!(plan.progress(), 0.0);

        assert!(plan.advance_step());
        assert_eq!(plan.current_step, 1);
        assert_eq!(plan.progress(), 0.5);

        assert!(!plan.advance_step());
        assert!(plan.is_complete());
    }

    #[test]
    fn test_complexity_limits() {
        let mut steps = Vec::new();
        for i in 0..15 {
            steps.push(PlanStep {
                action: PlanActionType::Rest { duration: 1 },
                estimated_ticks: 1,
                required_tool: None,
                required_resources: vec![],
                target_location: None,
                confidence: 1.0,
            });
        }

        let plan = ActionPlan::new("Complex".to_string(), steps, 0, "test".to_string());

        // Lazy trait should reject this (max 3 steps)
        assert!(plan.exceeds_complexity_limit(&[Trait::Lazy]));

        // Ambitious trait should accept this (max 20 steps)
        assert!(!plan.exceeds_complexity_limit(&[Trait::Ambitious]));

        // Default should accept (max 10 steps, but we have 15)
        assert!(plan.exceeds_complexity_limit(&[]));
    }

    #[test]
    fn test_planner_history() {
        let mut planner = Planner::new();

        let outcome = ActionOutcome {
            action_type: PlanActionType::GatherResource {
                resource: "wood".to_string(),
                amount: 10
            },
            estimated_ticks: 100,
            actual_ticks: 90,
            success: true,
            tool_used: Some("iron_axe".to_string()),
            tick: 0,
        };

        planner.record_outcome(outcome);
        assert_eq!(planner.action_history.len(), 1);
    }

    #[test]
    fn test_success_rate() {
        let mut planner = Planner::new();

        let action_type = PlanActionType::GatherResource {
            resource: "wood".to_string(),
            amount: 10
        };

        // Add 3 successes and 1 failure
        for i in 0..4 {
            planner.record_outcome(ActionOutcome {
                action_type: action_type.clone(),
                estimated_ticks: 100,
                actual_ticks: 90,
                success: i < 3, // First 3 succeed
                tool_used: Some("iron_axe".to_string()),
                tick: i as u32,
            });
        }

        let success_rate = planner.get_success_rate(&action_type);
        assert_eq!(success_rate, 0.75); // 3/4 = 0.75
    }

    #[test]
    fn test_tool_efficiency() {
        let mut planner = Planner::new();

        let action_type = PlanActionType::GatherResource {
            resource: "wood".to_string(),
            amount: 10
        };

        // Record iron axe as faster (50 ticks)
        planner.record_outcome(ActionOutcome {
            action_type: action_type.clone(),
            estimated_ticks: 100,
            actual_ticks: 50,
            success: true,
            tool_used: Some("iron_axe".to_string()),
            tick: 0,
        });

        // Record stone axe as slower (80 ticks)
        planner.record_outcome(ActionOutcome {
            action_type: action_type.clone(),
            estimated_ticks: 100,
            actual_ticks: 80,
            success: true,
            tool_used: Some("stone_axe".to_string()),
            tick: 1,
        });

        let iron_time = planner.get_tool_efficiency(&action_type, "iron_axe");
        let stone_time = planner.get_tool_efficiency(&action_type, "stone_axe");

        assert_eq!(iron_time, Some(50));
        assert_eq!(stone_time, Some(80));
    }

    #[test]
    fn test_distance_calculation() {
        let pos1 = (0, 0, 0);
        let pos2 = (3, 4, 0);
        let distance = Planner::calculate_distance(pos1, pos2);
        assert!((distance - 5.0).abs() < 0.001); // 3-4-5 triangle
    }

    #[test]
    fn test_plan_generation() {
        let planner = Planner::new();
        let plan = planner.plan_gather_wood(
            (0, 0, 0),
            (50, 50, 0),
            (0, 0, 0),
            &["iron_axe".to_string()],
            10,
        );

        assert_eq!(plan.steps.len(), 5); // Move, equip, gather, return, deposit
        assert!(plan.total_estimated_ticks > 0);
    }
}
