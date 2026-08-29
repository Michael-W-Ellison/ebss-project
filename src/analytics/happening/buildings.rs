// src/analytics/happening/buildings.rs
//! Buildings, and what standing in one does to somebody.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::{determine_placement_approach, Simulation};
use crate::agents::religious_effects::{
    calculate_religious_effects, total_happiness_modifier, RELIGIOUS_EFFECT_RADIUS,
};
use crate::world::spatial_planning::SpatialPlanner;
use log::debug;

impl Simulation {
    /// Process building production collection
    /// Agents within range of production buildings automatically collect pending resources
    pub(in crate::analytics) fn process_building_production_collection(&mut self) {
        use crate::world::Position;

        const COLLECTION_RANGE: f32 = 5.0; // Agents must be within 5 units to collect

        // Get all pending production
        let pending_production = self.world.get_pending_production_info();

        if pending_production.is_empty() {
            return;
        }

        // For each agent, check if they're near a production building
        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);

            // Check each building with pending production
            for (building_pos, (building_type, _resource_count)) in &pending_production {
                let dx = (agent_pos.x - building_pos.x) as f32;
                let dy = (agent_pos.y - building_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= COLLECTION_RANGE {
                    // Agent is close enough to collect - collect from this building
                    let resources = self.world.collect_building_production_at(*building_pos);

                    for resource in resources {
                        // Add resource to agent's inventory
                        let item_name = format!("{:?}", resource.resource_type).to_lowercase();
                        agent.inventory.add_item(
                            crate::agents::InventoryItem::new(item_name.clone(), resource.amount)
                        );

                        debug!(
                            "Agent {} collected {} {} from {:?} at ({}, {})",
                            agent.id, resource.amount, item_name, building_type,
                            building_pos.x, building_pos.y
                        );
                    }

                    // Only collect from one building per tick per agent
                    break;
                }
            }
        }
    }

    /// Process building maintenance needs
    /// Generates maintenance goals for agents near buildings that need repair
    pub(in crate::analytics) fn process_building_maintenance(&mut self) {
        use crate::world::Position;
        use crate::core::goals::{Goal, ExternalGoal};

        const MAINTENANCE_RANGE: f32 = 20.0; // Agents within 20 units get maintenance goals

        // Get buildings needing maintenance
        let maintenance_needed = self.world.get_buildings_needing_maintenance();
        let critical_buildings = self.world.get_critical_buildings();

        if maintenance_needed.is_empty() {
            return;
        }

        // For critical buildings, assign maintenance to nearby agents
        for (building_pos, building_type, condition) in &critical_buildings {
            // Find the closest agent to this building
            let mut closest_agent_idx: Option<usize> = None;
            let mut closest_distance = f32::MAX;

            for (idx, agent) in self.population.agents.iter().enumerate() {
                if !agent.state.is_alive {
                    continue;
                }

                let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
                let dx = (agent_pos.x - building_pos.x) as f32;
                let dy = (agent_pos.y - building_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance < closest_distance && distance <= MAINTENANCE_RANGE {
                    closest_distance = distance;
                    closest_agent_idx = Some(idx);
                }
            }

            // Assign maintenance goal to closest agent
            if let Some(idx) = closest_agent_idx {
                let agent = &mut self.population.agents[idx];
                let maintenance_job = format!("maintain_{:?}", building_type);

                // Check if agent already has a maintenance goal for this building
                let has_maintenance_goal = agent.goals.goals.iter().any(|g| {
                    if let Some(ExternalGoal::CompleteJob(ref job)) = g.external {
                        job.contains("maintain")
                    } else {
                        false
                    }
                });

                if !has_maintenance_goal {
                    let priority = if *condition < 0.25 { 0.9 } else { 0.6 };
                    let goal = Goal::new_external(
                        ExternalGoal::CompleteJob(maintenance_job),
                        priority,
                        self.current_tick,
                    );
                    agent.goals.add_goal(goal);

                    debug!(
                        "Agent {} assigned maintenance goal for {:?} at ({}, {}) - condition: {:.0}%",
                        agent.id, building_type, building_pos.x, building_pos.y, condition * 100.0
                    );
                }
            }
        }

        // For non-critical but degraded buildings, inform nearby agents (lower priority)
        for (building_pos, building_type, condition) in &maintenance_needed {
            if critical_buildings.iter().any(|(p, _, _)| p == building_pos) {
                continue; // Already handled as critical
            }

            // Add to exploration knowledge of nearby agents so they're aware
            for agent in &mut self.population.agents {
                if !agent.state.is_alive {
                    continue;
                }

                let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
                let dx = (agent_pos.x - building_pos.x) as f32;
                let dy = (agent_pos.y - building_pos.y) as f32;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= MAINTENANCE_RANGE {
                    // Agent is aware of this building's condition
                    // Could be used to generate lower-priority maintenance tasks
                    // For now, just log it
                    debug!(
                        "Agent {} aware of degraded {:?} at ({}, {}) - condition: {:.0}%",
                        agent.id, building_type, building_pos.x, building_pos.y, condition * 100.0
                    );
                }
            }
        }
    }

    /// Execute a building action for an agent, using spatial planning to determine optimal location
    ///
    /// This is a test helper method that allows direct building action execution.
    /// The building will be placed at an optimal location determined by the spatial planner.
    ///
    /// # Arguments
    /// * `agent_index` - Index of the agent in the population
    /// * `building_type` - Type of building to construct
    ///
    /// # Returns
    /// * `Ok(Position)` - The position where the building was placed
    /// * `Err(String)` - Error message if building failed
    pub fn execute_building_action(
        &mut self,
        agent_index: usize,
        building_type: crate::world::BuildingType,
    ) -> Result<(i32, i32, i32), String> {
        use crate::world::{Building, Position, ResourceType};

        // Get resource requirements for this building
        let requirements = building_type.requirements();

        // Check if agent has required resources in inventory
        let agent = &self.population.agents[agent_index];
        let mut has_all_resources = true;
        let mut missing_resources = Vec::new();

        for req in &requirements {
            let item_id = match req.resource_type {
                ResourceType::Wood => "wood",
                ResourceType::Stone => "stone",
                ResourceType::Iron => "iron",
                _ => continue,
            };

            if let Some(item) = agent.inventory.get_item(item_id) {
                if item.quantity < req.amount {
                    has_all_resources = false;
                    missing_resources.push(format!("{} {} (have {})", req.amount - item.quantity, item_id, item.quantity));
                }
            } else {
                has_all_resources = false;
                missing_resources.push(format!("{} {}", req.amount, item_id));
            }
        }

        if !has_all_resources {
            return Err(format!(
                "Missing resources for {:?}: {}",
                building_type,
                missing_resources.join(", ")
            ));
        }

        // Get agent's position
        let agent_pos = {
            let agent = &self.population.agents[agent_index];
            (agent.state.position.0, agent.state.position.1, agent.state.position.2)
        };

        // Use spatial planning to find optimal build location
        let (criteria, strategy) = determine_placement_approach(building_type);
        let planner = SpatialPlanner::new(&self.world);

        debug!("Spatial planning for {:?}: criteria={:?}, strategy={:?}",
               building_type, criteria, strategy);
        debug!("World has {} resource node types", self.world.resource_nodes.len());

        let optimal_pos = planner.find_optimal_location_for_agent(
            building_type,
            agent_pos,
            strategy
        );

        debug!("Optimal position found: {:?}", optimal_pos);

        // Use optimal position if found, otherwise fall back to agent's position
        let build_tuple_pos = optimal_pos.ok_or_else(|| {
            "No suitable building location found".to_string()
        })?;

        let build_pos = Position::new(build_tuple_pos.0, build_tuple_pos.1);
        if self.world.is_position_occupied(&build_pos) {
            return Err("No suitable building location found (all positions occupied)".to_string());
        }

        // Remove resources from agent inventory
        let agent = &mut self.population.agents[agent_index];
        for req in &requirements {
            let item_id = match req.resource_type {
                ResourceType::Wood => "wood",
                ResourceType::Stone => "stone",
                ResourceType::Iron => "iron",
                _ => continue,
            };

            agent.inventory.remove_item(item_id, req.amount);
        }

        // Create new building (under construction)
        let building = Building::new_under_construction(building_type, build_pos);

        // Add building to world
        self.world.add_building(building);

        debug!(
            "Agent {} started construction of {:?} at ({}, {}, {})",
            agent_index, building_type, build_tuple_pos.0, build_tuple_pos.1, build_tuple_pos.2
        );

        Ok(build_tuple_pos)
    }


    /// Apply religious building effects to agent happiness
    /// Believers gain happiness near Shrines/Temples, Atheists feel uncomfortable
    pub(in crate::analytics) fn apply_religious_effects(&mut self) {
        use crate::world::{BuildingType, Position};
        use crate::agents::Trait;

        // Collect religious buildings (position, type, is_completed)
        let religious_buildings: Vec<(Position, BuildingType, bool)> = self.world.buildings
            .iter()
            .filter(|b| b.building_type.is_religious())
            .map(|b| (b.position, b.building_type, b.is_completed()))
            .collect();

        // If no religious buildings, skip processing
        if religious_buildings.is_empty() {
            return;
        }

        // First, count believers near each agent for zealot community bonuses
        // Pre-calculate positions and traits
        let agent_data: Vec<_> = self.population.agents.iter()
            .filter(|a| a.state.is_alive)
            .map(|a| {
                let pos = Position::new(a.state.position.0, a.state.position.1);
                let is_believer = a.traits.has(Trait::Believer) || a.traits.has(Trait::Zealot);
                (a.id, pos, is_believer)
            })
            .collect();

        // Calculate nearby believers for each agent
        let nearby_believers: std::collections::BTreeMap<_, _> = agent_data.iter()
            .map(|(id, pos, _)| {
                let count = agent_data.iter()
                    .filter(|(other_id, other_pos, is_believer)| {
                        *is_believer
                            && other_id != id
                            && pos.distance_to(other_pos) <= RELIGIOUS_EFFECT_RADIUS
                    })
                    .count() as u32;
                (*id, count)
            })
            .collect();

        // Apply religious effects to each agent
        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);
            let believers_nearby = *nearby_believers.get(&agent.id).unwrap_or(&0);

            // Calculate religious effects for this agent
            let effects = calculate_religious_effects(
                agent_pos,
                &agent.traits,
                &religious_buildings,
                believers_nearby,
            );

            // Apply effects
            let total_modifier = total_happiness_modifier(&effects);
            if total_modifier.abs() > 0.001 {
                // Generate a combined source description
                let source = if total_modifier > 0.0 {
                    format!("Religious fulfillment ({})", effects.len())
                } else {
                    format!("Religious discomfort ({})", effects.len())
                };

                agent.apply_religious_happiness(total_modifier, &source);

                debug!(
                    "Agent {} received religious effect: {:.3} happiness from {} sources",
                    agent.id, total_modifier, effects.len()
                );
            }
        }
    }
}
