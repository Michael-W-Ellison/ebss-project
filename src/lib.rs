// src/lib.rs
//! # Emergent Behavior Society Simulator (EBSS)
//!
//! A general-purpose AI platform for simulating societies of autonomous agents
//! that learn and adapt through behavioral evolution.
//!
//! ## Core Concepts
//!
//! - **Behavior Trees**: Learned decision-making patterns
//! - **Drive System**: Internal motivations that guide behavior
//! - **Memory**: Spatial, storage, social, and recipe knowledge
//! - **Learning**: Trial & error, observation, and genetic inheritance
//! - **Environment Abstraction**: Plugin system for different world types
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ebss::prelude::*;
//!
//! fn main() {
//!     let world = World::new(GridConfig::default());
//!     let mut population = Population::new();
//!     
//!     for _ in 0..10 {
//!         population.spawn_agent(AgentConfig::default());
//!     }
//!     
//!     let mut sim = Simulation::new(world, population);
//!     sim.run_for_ticks(1000);
//! }
//! ```

pub mod core;
pub mod agents;
pub mod environment;
pub mod world;
pub mod analytics;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::core::{
        behavior_tree::{BehaviorTree, BehaviorNode, NodeType},
        drives::{Drive, DriveType, DriveState},
        learning::LearningSystem,
    };
    
    pub use crate::agents::{
        Agent, AgentConfig, AgentState,
        Population, Inventory, InventoryItem,
        Senses, Vision, Hearing, Speech,
        Body, BodyPart, BodyPartType, BodyPartStatus,
        Skills, Skill, SkillType, SkillCategory, Quality,
    };

    pub use crate::environment::{
        EnvironmentPlugin, PluginRegistry,
        Material, Action, CraftingTemplate,
        ActionResult, ActionContext, Position as EnvPosition,
        ToolType, ToolTier, MaterialCategory,
        Structure, StructureType, StructureLevel, StructureRegistry,
    };
    
    pub use crate::world::{
        World, WorldConfig, GridConfig,
        Position, Chunk,
    };
    
    pub use crate::analytics::{
        Simulation, SimulationConfig,
        Analytics, BehaviorAnalysis,
        SimulationController, SimulationState,
        Inspector, Selection, AgentInspectorData,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_imports() {
        // Verify all modules are accessible
        use crate::prelude::*;
        assert!(true);
    }
}
