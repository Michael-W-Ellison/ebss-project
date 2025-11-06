// src/agents/population.rs
use crate::agents::{Agent, AgentConfig};

pub struct Population {
    pub agents: Vec<Agent>,
}

impl Population {
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }
    
    pub fn spawn_agent(&mut self, config: AgentConfig) {
        self.agents.push(Agent::new(config));
    }
    
    pub fn size(&self) -> usize {
        self.agents.len()
    }
}

impl Default for Population {
    fn default() -> Self {
        Self::new()
    }
}
