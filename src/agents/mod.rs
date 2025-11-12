// src/agents/mod.rs
//! Agent implementation and population management.

pub mod agent;
pub mod population;
pub mod reproduction;
pub mod shared_knowledge;

pub use agent::{Agent, AgentConfig, AgentState, LifeStage};
pub use population::{Population, PopulationConfig};
pub use reproduction::{can_mate, reproduce, MateSelectionCriteria};
pub use shared_knowledge::{SharedKnowledge, DiscoveredResource};
