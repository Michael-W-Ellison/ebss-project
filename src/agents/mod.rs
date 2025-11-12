// src/agents/mod.rs
//! Agent implementation and population management.

pub mod agent;
pub mod population;

pub use agent::{Agent, AgentConfig, AgentState, Inventory, InventoryItem};
pub use population::Population;
