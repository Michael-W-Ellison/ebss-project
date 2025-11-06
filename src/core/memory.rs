// src/core/memory.rs
//! Memory system for agents.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Spatial,
    Storage,
    Social,
    Recipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    // Placeholder for memory implementation
}

impl Memory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
