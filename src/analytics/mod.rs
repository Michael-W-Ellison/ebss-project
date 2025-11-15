// src/analytics/mod.rs
//! Analytics, data logging, and emergence detection.

use crate::world::World;
use crate::agents::Population;

pub mod simulation_controller;
pub mod inspector;

pub use simulation_controller::{SimulationController, SimulationState};
pub use inspector::{
    Inspector, Selection, AgentInspectorData, DriveInspectorData,
    TerrainInspectorData, MemorySummary, InventorySummary, SensorySummary, SkillsSummary,
    EmotionSummary, RelationshipSummary,
};
pub mod metrics;
pub mod emergence;
pub mod export;
pub mod performance;

pub use metrics::{SimulationMetrics, TickSnapshot, PopulationSnapshot, DriveSnapshot, EmotionSnapshot};
pub use emergence::{EmergenceDetector, EmergentPattern, PatternType};
pub use export::{DataExporter, ExportFormat};
pub use performance::{PerformanceMonitor, PerformanceSnapshot};

use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use crate::visualization::AsciiRenderer;
use log::{info, debug};

pub struct Simulation {
    pub world: World,
    pub population: Population,
    pub current_tick: u32,
    pub renderer: Option<AsciiRenderer>,
}

pub struct SimulationConfig;
pub struct Analytics;
pub struct BehaviorAnalysis;

impl Simulation {
    pub fn new(world: World, population: Population) -> Self {
        Self {
            world,
            population,
            current_tick: 0,
            renderer: None,
        }
    }

    /// Enable ASCII visualization
    pub fn with_visualization(mut self) -> Self {
        self.renderer = Some(AsciiRenderer::default());
        self
    }

    /// Run the simulation for a specified number of ticks
    pub fn run_for_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.tick();
        }
        info!("Simulation completed {} ticks", ticks);
    }

    /// Run the simulation with visualization
    pub fn run_visual(&mut self, ticks: u32, update_interval: u32) {
        for _ in 0..ticks {
            self.tick();

            // Render visualization at intervals
            if self.current_tick % update_interval == 0 {
                if let Some(renderer) = &self.renderer {
                    renderer.render(&self.population, self.current_tick);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        // Final render
        if let Some(renderer) = &self.renderer {
            renderer.render(&self.population, self.current_tick);
        }
    }

    /// Execute one simulation tick
    pub fn tick(&mut self) {
        self.current_tick += 1;
        debug!("=== Tick {} ===", self.current_tick);

        // Process each agent
        for agent in &mut self.population.agents {
            // 1. Update drives (accumulation)
            agent.tick();

            // 2. Select the most urgent drive and corresponding behavior tree
            if let Some(urgent_drive) = agent.drives.most_urgent() {
                let drive_type = urgent_drive.drive_type;
                let drive_value = urgent_drive.value;
                let agent_position = agent.state.position;

                debug!(
                    "Agent {} - Most urgent drive: {:?} (value: {:.2})",
                    agent.id, drive_type, drive_value
                );

                // 3. Select and execute behavior tree
                if let Some(tree) = agent.select_behavior_tree() {
                    let tree_name = tree.name.clone();

                    // Execute the behavior tree (learning happens here automatically)
                    let execution_result = tree.execute();

                    debug!(
                        "Agent {} - Executed tree: {} -> {:?}",
                        agent.id, tree_name, execution_result
                    );

                    // 4. Generate action based on drive type and agent position
                    let action = Self::generate_action_for_drive(drive_type, agent_position);

                    // 5. Execute action in environment and get feedback
                    let action_result = Self::execute_action_static(&action);

                    debug!(
                        "Agent {} - Action result: {} (satisfaction: {:.2})",
                        agent.id, action_result.message, action_result.drive_satisfaction
                    );

                    // 6. Apply feedback to agent (drive satisfaction)
                    agent.apply_feedback(&action_result, drive_type);

                    // 7. Update behavior tree weights based on action success
                    // This already happened in tree.execute(), but we could add additional
                    // reinforcement here based on the actual action result
                    if let Some(tree) = agent.select_behavior_tree() {
                        if action_result.success {
                            tree.total_successes += 1;
                        }
                    }
                }
            }
        }

        // Log statistics every 10 ticks
        if self.current_tick % 10 == 0 {
            self.log_statistics();
        }
    }

    /// Generate an action based on drive type and position
    fn generate_action_for_drive(drive_type: DriveType, position: (i32, i32, i32)) -> Action {
        // Map drive type to a representative action
        match drive_type {
            DriveType::Hunger => Action::Eat { food_type: "generic".to_string() },
            DriveType::Rest => Action::Sleep { duration: 10 },
            DriveType::Shelter => Action::Build {
                structure_type: "shelter".to_string(),
                position
            },
            DriveType::Construction => Action::Build {
                structure_type: "structure".to_string(),
                position
            },
            DriveType::Industry => Action::Gather { resource_type: "generic".to_string() },
            DriveType::Curiosity => Action::Explore { direction: (1, 0, 0) },
            DriveType::Social => Action::Socialize { target_agent_id: uuid::Uuid::nil() },
            DriveType::Utility => Action::Craft { item_type: "tool".to_string() },
            DriveType::Preparedness => Action::Store { item_type: "resource".to_string(), amount: 1 },
            DriveType::Sustenance => Action::Gather { resource_type: "food".to_string() },
            DriveType::Safety => Action::Move { target: position },
            DriveType::Reproduction => Action::Wait,
            DriveType::Luxury => Action::Gather { resource_type: "luxury".to_string() },
        }
    }

    /// Execute an action in the environment and return the result
    fn execute_action_static(action: &Action) -> ActionResult {
        // Simulate action execution with some randomness
        // In a full implementation, this would interact with the world state

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let success_probability = 0.7; // 70% success rate

        let success = rng.gen_bool(success_probability);

        if success {
            // Calculate satisfaction based on action type
            let satisfaction = match action {
                Action::Eat { .. } => 0.3,
                Action::Sleep { .. } => 0.5,
                Action::Build { .. } => 0.2,
                Action::Gather { .. } => 0.15,
                Action::Craft { .. } => 0.2,
                Action::Store { .. } => 0.1,
                Action::Explore { .. } => 0.15,
                Action::Socialize { .. } => 0.2,
                Action::Move { .. } => 0.05,
                Action::Wait => 0.0,
            };

            ActionResult::success(
                satisfaction,
                format!("{:?} completed successfully", action)
            )
        } else {
            ActionResult::failure(format!("{:?} failed", action))
        }
    }

    pub fn run_for_ticks(&mut self, _ticks: u32) {
        // Placeholder for simulation loop
    }

    /// Log simulation statistics
    fn log_statistics(&self) {
        info!("--- Tick {} Statistics ---", self.current_tick);
        info!("Population size: {}", self.population.agents.len());

        // Aggregate drive statistics
        let mut total_hunger = 0.0;
        let mut total_rest = 0.0;
        let mut total_curiosity = 0.0;

        for agent in &self.population.agents {
            if let Some(hunger) = agent.drives.get(DriveType::Hunger) {
                total_hunger += hunger.value;
            }
            if let Some(rest) = agent.drives.get(DriveType::Rest) {
                total_rest += rest.value;
            }
            if let Some(curiosity) = agent.drives.get(DriveType::Curiosity) {
                total_curiosity += curiosity.value;
            }
        }

        let agent_count = self.population.agents.len() as f32;
        if agent_count > 0.0 {
            info!("Average Hunger: {:.2}", total_hunger / agent_count);
            info!("Average Rest: {:.2}", total_rest / agent_count);
            info!("Average Curiosity: {:.2}", total_curiosity / agent_count);
        }
    }
}
