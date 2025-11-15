// src/agents/population.rs
use crate::agents::{Agent, AgentConfig, SharedKnowledge, can_mate, reproduce, MateSelectionCriteria};
use uuid::Uuid;
use std::collections::HashMap;

/// Statistics about population dynamics
#[derive(Debug, Clone, Default)]
pub struct PopulationStats {
    pub total_births: u64,
    pub total_deaths: u64,
    pub total_abandonments: u64,
    pub current_population: usize,
    pub average_age: f32,
    pub average_happiness: f32,
    pub infants: usize,
    pub children: usize,
    pub adolescents: usize,
    pub adults: usize,
    pub elderly: usize,
}

/// Configuration for population behavior
#[derive(Debug, Clone)]
pub struct PopulationConfig {
    /// Happiness threshold below which agents consider leaving (-1.0 to 1.0)
    pub abandonment_happiness_threshold: f32,
    /// How long an agent must be unhappy before they can leave (ticks)
    pub abandonment_unhappy_duration: u32,
    /// Probability per tick that an unhappy agent will leave
    pub abandonment_probability: f32,
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            abandonment_happiness_threshold: -0.3, // Leave if happiness below -0.3
            abandonment_unhappy_duration: 1000,    // Must be unhappy for 1000 ticks
            abandonment_probability: 0.01,         // 1% chance per tick when eligible
        }
    }
}

pub struct Population {
    pub agents: Vec<Agent>,
    pub stats: PopulationStats,
    pub mate_criteria: MateSelectionCriteria,
    pub reproduction_cooldown: HashMap<Uuid, u32>,
    pub config: PopulationConfig,
    pub unhappiness_tracker: HashMap<Uuid, u32>, // Track how long agents have been unhappy
    pub current_tick: u32, // Current simulation tick for survival mechanics
    pub shared_knowledge: SharedKnowledge, // Shared resource/world information between agents
}

impl Population {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            stats: PopulationStats::default(),
            mate_criteria: MateSelectionCriteria::default(),
            reproduction_cooldown: HashMap::new(),
            config: PopulationConfig::default(),
            unhappiness_tracker: HashMap::new(),
            current_tick: 0,
            shared_knowledge: SharedKnowledge::new(),
        }
    }

    /// Create a new population with custom configuration
    pub fn with_config(config: PopulationConfig) -> Self {
        Self {
            agents: Vec::new(),
            stats: PopulationStats::default(),
            mate_criteria: MateSelectionCriteria::default(),
            reproduction_cooldown: HashMap::new(),
            config,
            unhappiness_tracker: HashMap::new(),
            current_tick: 0,
            shared_knowledge: SharedKnowledge::new(),
        }
    }

    /// Spawn a new agent
    pub fn spawn_agent(&mut self, config: AgentConfig) {
        self.agents.push(Agent::new(config));
        self.stats.current_population = self.agents.len();
    }

    /// Get current population size (alive agents only)
    pub fn size(&self) -> usize {
        self.agents.iter().filter(|a| a.state.is_alive).count()
    }

    /// Update all agents and handle lifecycle events
    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Update shared knowledge tick counter
        self.shared_knowledge.tick();

        // Update all agents
        let current_tick = self.current_tick;
        for agent in &mut self.agents {
            agent.tick();
            agent.state.age_tick(current_tick);
        }

        // Process unhappiness tracking and abandonments
        self.process_abandonments();

        // Process deaths
        self.process_deaths();

        // Process reproduction
        self.process_reproduction();

        // Update cooldowns
        self.update_cooldowns();

        // Update statistics
        self.update_stats();
    }

    /// Remove dead agents from population
    fn process_deaths(&mut self) {
        let before = self.agents.len();
        self.agents.retain(|agent| agent.state.is_alive);
        let deaths = before - self.agents.len();
        self.stats.total_deaths += deaths as u64;
    }

    /// Process agent abandonments based on unhappiness
    /// Agents who are severely unhappy for extended periods may leave the town
    pub fn process_abandonments(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Track unhappiness and identify agents who should abandon
        let mut agents_to_remove = Vec::new();

        for agent in &self.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Get agent's current happiness from emotions
            // TODO: Implement EmotionState::get() method
            let happiness = 0.0; // Placeholder until EmotionState API is fixed

            // Check if agent is unhappy
            if happiness < self.config.abandonment_happiness_threshold {
                // Track unhappiness duration
                let unhappy_duration = self.unhappiness_tracker
                    .entry(agent.id)
                    .or_insert(0);
                *unhappy_duration += 1;

                // Check if agent has been unhappy long enough
                if *unhappy_duration >= self.config.abandonment_unhappy_duration {
                    // Probabilistic abandonment
                    if rng.gen::<f32>() < self.config.abandonment_probability {
                        agents_to_remove.push(agent.id);
                    }
                }
            } else {
                // Agent is happy, reset unhappiness tracker
                self.unhappiness_tracker.remove(&agent.id);
            }
        }

        // Remove agents who are abandoning
        if !agents_to_remove.is_empty() {
            self.agents.retain(|agent| !agents_to_remove.contains(&agent.id));
            self.stats.total_abandonments += agents_to_remove.len() as u64;

            // Clean up tracking for abandoned agents
            for agent_id in agents_to_remove {
                self.unhappiness_tracker.remove(&agent_id);
                self.reproduction_cooldown.remove(&agent_id);
            }
        }
    }

    /// Process reproduction attempts
    pub fn process_reproduction(&mut self) {
        let mut new_offspring = Vec::new();

        // Find potential mating pairs
        let alive_agents: Vec<usize> = self.agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.state.is_alive && a.can_reproduce())
            .filter(|(_, a)| !self.is_on_cooldown(a.id))
            .map(|(i, _)| i)
            .collect();

        // Attempt reproduction for each potential pair
        for i in 0..alive_agents.len() {
            for j in (i + 1)..alive_agents.len() {
                let idx1 = alive_agents[i];
                let idx2 = alive_agents[j];

                let agent1 = &self.agents[idx1];
                let agent2 = &self.agents[idx2];

                if can_mate(agent1, agent2, &self.mate_criteria) {
                    // Check reproduction drive - both must want to reproduce
                    let drive1 = agent1.drives.get(crate::core::DriveType::Reproduction)
                        .map(|d| d.is_active())
                        .unwrap_or(false);
                    let drive2 = agent2.drives.get(crate::core::DriveType::Reproduction)
                        .map(|d| d.is_active())
                        .unwrap_or(false);

                    if drive1 && drive2 {
                        // Successful reproduction
                        let offspring = reproduce(agent1, agent2, self.current_tick);
                        new_offspring.push(offspring);

                        // Add cooldown (prevent immediate re-reproduction)
                        self.reproduction_cooldown.insert(agent1.id, 500); // 500 ticks cooldown
                        self.reproduction_cooldown.insert(agent2.id, 500);

                        // Mark parents as having a new child
                        // We'll do this in a second pass to avoid borrow issues
                        // Store offspring IDs for later
                    }
                }
            }
        }

        // Store offspring IDs for establishing parent-child relationships
        let offspring_ids: Vec<(Uuid, Vec<Uuid>)> = new_offspring
            .iter()
            .map(|o| (o.id, o.parent_ids.clone()))
            .collect();

        // Add offspring to population
        let birth_count = new_offspring.len();
        self.agents.extend(new_offspring);
        self.stats.total_births += birth_count as u64;

        // Satisfy reproduction drive and establish relationships for parents who reproduced
        for agent in &mut self.agents {
            if self.reproduction_cooldown.contains_key(&agent.id) {
                // Satisfy reproduction drive
                if let Some(drive) = agent.drives.get_mut(crate::core::DriveType::Reproduction) {
                    drive.satisfy();
                }

                // Mark children in parent's memory
                for (offspring_id, parent_ids) in &offspring_ids {
                    if parent_ids.contains(&agent.id) {
                        agent.memory.mark_as_child(*offspring_id);
                    }
                }
            }
        }
    }

    /// Check if agent is on reproduction cooldown
    fn is_on_cooldown(&self, agent_id: Uuid) -> bool {
        self.reproduction_cooldown.get(&agent_id).map(|&cd| cd > 0).unwrap_or(false)
    }

    /// Update reproduction cooldowns
    fn update_cooldowns(&mut self) {
        self.reproduction_cooldown.retain(|_, cooldown| {
            *cooldown = cooldown.saturating_sub(1);
            *cooldown > 0
        });
    }

    /// Update population statistics
    fn update_stats(&mut self) {
        use crate::agents::LifeStage;

        let alive_agents: Vec<&Agent> = self.agents.iter()
            .filter(|a| a.state.is_alive)
            .collect();

        self.stats.current_population = alive_agents.len();

        if alive_agents.is_empty() {
            self.stats.average_age = 0.0;
            self.stats.average_happiness = 0.0;
            self.stats.infants = 0;
            self.stats.children = 0;
            self.stats.adolescents = 0;
            self.stats.adults = 0;
            self.stats.elderly = 0;
            return;
        }

        // Calculate average age
        let total_age: u32 = alive_agents.iter().map(|a| a.state.age).sum();
        self.stats.average_age = total_age as f32 / alive_agents.len() as f32;

        // Calculate average happiness
        // TODO: Implement EmotionState::get() method
        let total_happiness: f32 = 0.0; // Placeholder
        self.stats.average_happiness = total_happiness / alive_agents.len() as f32;

        // Count life stages
        self.stats.infants = alive_agents.iter().filter(|a| a.state.life_stage == LifeStage::Infant).count();
        self.stats.children = alive_agents.iter().filter(|a| a.state.life_stage == LifeStage::Child).count();
        self.stats.adolescents = alive_agents.iter().filter(|a| a.state.life_stage == LifeStage::Adolescent).count();
        self.stats.adults = alive_agents.iter().filter(|a| a.state.life_stage == LifeStage::Adult).count();
        self.stats.elderly = alive_agents.iter().filter(|a| a.state.life_stage == LifeStage::Elderly).count();
    }

    /// Get agent by ID
    pub fn get_agent(&self, id: Uuid) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Get mutable agent by ID
    pub fn get_agent_mut(&mut self, id: Uuid) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Get all agents within a certain distance of a position
    pub fn agents_near(&self, position: (i32, i32, i32), radius: f32) -> Vec<&Agent> {
        self.agents
            .iter()
            .filter(|a| a.state.is_alive)
            .filter(|a| {
                let dx = (a.state.position.0 - position.0) as f32;
                let dy = (a.state.position.1 - position.1) as f32;
                let dz = (a.state.position.2 - position.2) as f32;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                distance <= radius
            })
            .collect()
    }
}

impl Default for Population {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_population_creation() {
        let pop = Population::new();
        assert_eq!(pop.size(), 0);
    }

    #[test]
    fn test_spawn_agent() {
        let mut pop = Population::new();
        pop.spawn_agent(AgentConfig::default());
        assert_eq!(pop.size(), 1);
    }

    #[test]
    fn test_tick_ages_agents() {
        let mut pop = Population::new();
        pop.spawn_agent(AgentConfig::default());

        let initial_age = pop.agents[0].state.age;
        pop.tick();
        assert_eq!(pop.agents[0].state.age, initial_age + 1);
    }

    #[test]
    fn test_death_removal() {
        let mut pop = Population::new();
        pop.spawn_agent(AgentConfig::default());

        // Kill the agent
        pop.agents[0].state.is_alive = false;

        pop.tick();

        assert_eq!(pop.size(), 0);
        assert_eq!(pop.stats.total_deaths, 1);
    }
}
