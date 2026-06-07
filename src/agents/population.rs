// src/agents/population.rs
use crate::agents::{Agent, AgentConfig, SharedKnowledge, Trait};
use crate::agents::{can_mate, reproduce, MateSelectionCriteria};
use crate::environment::technology::TechnologyRegistry;
use crate::core::events::{SimulationEvent, SimulationEventType, DeathCause};
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
    pub technology_registry: TechnologyRegistry, // Global technology discovery tracking
    /// Events that occurred this tick (for GUI timeline)
    pub pending_events: Vec<SimulationEvent>,
}

impl Population {
    pub fn new() -> Self {
        let mut registry = TechnologyRegistry::new();
        Self::initialize_basic_technologies(&mut registry);

        Self {
            agents: Vec::new(),
            stats: PopulationStats::default(),
            mate_criteria: MateSelectionCriteria::default(),
            reproduction_cooldown: HashMap::new(),
            config: PopulationConfig::default(),
            unhappiness_tracker: HashMap::new(),
            current_tick: 0,
            shared_knowledge: SharedKnowledge::new(),
            technology_registry: registry,
            pending_events: Vec::new(),
        }
    }

    /// Create a new population with custom configuration
    pub fn with_config(config: PopulationConfig) -> Self {
        let mut registry = TechnologyRegistry::new();
        Self::initialize_basic_technologies(&mut registry);

        Self {
            agents: Vec::new(),
            stats: PopulationStats::default(),
            mate_criteria: MateSelectionCriteria::default(),
            reproduction_cooldown: HashMap::new(),
            config,
            unhappiness_tracker: HashMap::new(),
            current_tick: 0,
            shared_knowledge: SharedKnowledge::new(),
            technology_registry: registry,
            pending_events: Vec::new(),
        }
    }

    /// Initialize basic Stone Age technologies
    fn initialize_basic_technologies(registry: &mut TechnologyRegistry) {
        use crate::environment::technology::Technology;

        // Fire making - everyone starts with this
        let fire = Technology::new("fire".to_string(), "Fire Making".to_string())
            .with_description("Creating and controlling fire for warmth and cooking".to_string())
            .with_discovery_chance(1.0); // Already known

        // Basic wooden tools - easy to discover
        let wooden_tools = Technology::new("wooden_tools".to_string(), "Wooden Tools".to_string())
            .with_description("Crafting basic tools from wood".to_string())
            .with_required_materials(vec!["wood".to_string()])
            .with_discovery_chance(0.3)
            .with_accidental_discovery(0.01);

        // Flint knapping - stone age advancement
        let flint_knapping = Technology::new("flint_knapping".to_string(), "Flint Knapping".to_string())
            .with_description("Shaping flint into sharp tools and weapons".to_string())
            .with_required_materials(vec!["stone".to_string()])
            .with_prerequisites(vec!["wooden_tools".to_string()])
            .with_discovery_chance(0.2)
            .with_curiosity_threshold(0.4);

        // Stone tools - requires flint knapping
        let stone_tools = Technology::new("stone_tools".to_string(), "Stone Tool Crafting".to_string())
            .with_description("Creating durable tools from stone and wood".to_string())
            .with_required_materials(vec!["stone".to_string(), "wood".to_string()])
            .with_prerequisites(vec!["flint_knapping".to_string()])
            .with_discovery_chance(0.25);

        // Iron working - advanced technology
        let iron_working = Technology::new("iron_working".to_string(), "Iron Working".to_string())
            .with_description("Smelting and working iron into tools".to_string())
            .with_required_materials(vec!["iron".to_string(), "coal".to_string()])
            .with_prerequisites(vec!["fire".to_string(), "stone_tools".to_string()])
            .with_discovery_chance(0.05)
            .with_curiosity_threshold(0.6)
            .with_accidental_discovery(0.001);

        registry.register(fire);
        registry.register(wooden_tools);
        registry.register(flint_knapping);
        registry.register(stone_tools);
        registry.register(iron_working);
    }

    /// Spawn a new agent
    pub fn spawn_agent(&mut self, config: AgentConfig) {
        let mut agent = Agent::new(config);

        // Give new agents basic starting knowledge
        
        agent.technology_knowledge.add_initial_technology(
            "fire".to_string(),
            agent.id,
            self.current_tick as u64
        );

        self.agents.push(agent);
        self.stats.current_population = self.agents.len();
    }

    /// Get current population size (alive agents only)
    pub fn size(&self) -> usize {
        self.agents.iter().filter(|a| a.state.is_alive).count()
    }

    /// Update all agents and handle lifecycle events
    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Clear pending events from previous tick
        self.pending_events.clear();

        // Update shared knowledge tick counter
        self.shared_knowledge.tick();

        // Update all agents
        let current_tick = self.current_tick;
        for agent in &mut self.agents {
            agent.tick_with_percepts(current_tick); // Process percepts with timestamp
            agent.state.age_tick(current_tick);
        }

        // Update relationships between nearby agents
        self.update_relationships();

        // Decay distant relationships (every 100 ticks to reduce overhead)
        if current_tick.is_multiple_of(100) {
            self.decay_relationships();
        }

        // Process social interactions (every 10 ticks to reduce overhead)
        if current_tick.is_multiple_of(10) {
            self.process_social_interactions();
        }

        // Process gossip spreading (every 15 ticks)
        if current_tick.is_multiple_of(15) {
            self.process_gossip();
        }

        // Process observational learning (every 20 ticks to reduce overhead)
        if current_tick.is_multiple_of(20) {
            self.process_observational_learning();
        }

        // Process exploration for all agents (vision-based discovery)
        self.process_exploration();

        // Share technologies between nearby agents
        self.share_technologies();

        // Attempt technology discovery (every 50 ticks to reduce overhead)
        if current_tick.is_multiple_of(50) {
            self.discover_technologies();
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

    /// Update relationships between nearby agents
    ///
    /// Forms new relationships when agents meet and updates existing ones
    /// based on proximity and trait compatibility.
    fn update_relationships(&mut self) {
        use super::{Relationship, RelationshipType};

        // Pre-compute squared interaction distance threshold (avoids sqrt)
        const INTERACTION_RANGE_SQUARED: f32 = 100.0; // 10.0 * 10.0

        // Process all pairs of agents
        for i in 0..self.agents.len() {
            for j in (i + 1)..self.agents.len() {
                let agent1_id = self.agents[i].id;
                let agent2_id = self.agents[j].id;
                let agent1_pos = self.agents[i].state.position;
                let agent2_pos = self.agents[j].state.position;

                // Calculate squared distance (avoid expensive sqrt)
                let dx = (agent1_pos.0 - agent2_pos.0) as f32;
                let dy = (agent1_pos.1 - agent2_pos.1) as f32;
                let distance_squared = dx * dx + dy * dy;

                // Agents must be within interaction range (10 tiles)
                if distance_squared <= INTERACTION_RANGE_SQUARED {
                    // Only compute actual distance when needed for proximity bonus
                    let distance = distance_squared.sqrt();

                    // Get traits for compatibility check (clone needed due to borrow rules)
                    let agent1_traits = self.agents[i].traits.clone();
                    let agent2_traits = self.agents[j].traits.clone();

                    // Check if relationship exists for agent 1 -> agent 2
                    let agent1_has_rel = self.agents[i].relationships.get_relationship(&agent2_id).is_some();

                    if !agent1_has_rel {
                        // Form new relationship as Acquaintance
                        let new_rel = Relationship::new(agent2_id, RelationshipType::Acquaintance);
                        self.agents[i].relationships.add_relationship(new_rel);
                    } else {
                        // Update existing relationship based on traits
                        self.agents[i].relationships.update_relationship_from_traits(
                            &agent2_id,
                            &agent1_traits,
                            &agent2_traits,
                        );
                    }

                    // Check if relationship exists for agent 2 -> agent 1
                    let agent2_has_rel = self.agents[j].relationships.get_relationship(&agent1_id).is_some();

                    if !agent2_has_rel {
                        // Form new relationship as Acquaintance
                        let new_rel = Relationship::new(agent1_id, RelationshipType::Acquaintance);
                        self.agents[j].relationships.add_relationship(new_rel);
                    } else {
                        // Update existing relationship based on traits
                        self.agents[j].relationships.update_relationship_from_traits(
                            &agent1_id,
                            &agent2_traits,
                            &agent1_traits,
                        );
                    }

                    // Strengthen bonds slightly for nearby agents
                    if let Some(rel) = self.agents[i].relationships.get_relationship_mut(&agent2_id) {
                        // Closer = stronger bond increase (inverse of distance)
                        let proximity_bonus = (11.0 - distance) / 100.0; // Max 0.10 at distance 0
                        rel.strengthen(proximity_bonus);
                        rel.time_together += 1;
                    }

                    if let Some(rel) = self.agents[j].relationships.get_relationship_mut(&agent1_id) {
                        let proximity_bonus = (11.0 - distance) / 100.0;
                        rel.strengthen(proximity_bonus);
                        rel.time_together += 1;
                    }
                }
            }
        }
    }

    /// Decay relationships when agents don't interact
    ///
    /// Relationships fade over time if agents don't spend time together.
    fn decay_relationships(&mut self) {
        // First, collect agent positions to avoid borrowing issues
        let agent_positions: std::collections::HashMap<Uuid, (i32, i32, i32)> =
            self.agents.iter()
                .map(|a| (a.id, a.state.position))
                .collect();

        // Now update relationships based on distance
        for agent in &mut self.agents {
            let agent_pos = agent.state.position;
            let current_relationships: Vec<_> = agent.relationships.get_all()
                .keys()
                .copied()
                .collect();

            for other_id in current_relationships {
                if let Some(&other_pos) = agent_positions.get(&other_id) {
                    let dx = (agent_pos.0 - other_pos.0) as f32;
                    let dy = (agent_pos.1 - other_pos.1) as f32;
                    let distance = (dx * dx + dy * dy).sqrt();

                    // If agents are far apart (>50 tiles), decay relationship
                    if distance > 50.0 {
                        if let Some(rel) = agent.relationships.get_relationship_mut(&other_id) {
                            // Not family - decay faster
                            let decay_amount = if rel.is_family() {
                                0.0001 // Family bonds decay very slowly
                            } else {
                                0.001 // Non-family decays faster
                            };

                            // Decay towards neutral
                            if rel.bond_strength > 0.0 {
                                rel.bond_strength = (rel.bond_strength - decay_amount).max(0.0);
                            } else if rel.bond_strength < 0.0 {
                                rel.bond_strength = (rel.bond_strength + decay_amount).min(0.0);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Share technologies between nearby agents
    ///
    /// Agents teach technologies they know to nearby agents through conversation.
    fn share_technologies(&mut self) {
        use crate::environment::technology::DiscoveryMethod;

        // Pre-compute squared distance threshold (avoids sqrt)
        const SHARE_RANGE_SQUARED: f32 = 25.0; // 5.0 * 5.0

        // Process all pairs of nearby agents
        for i in 0..self.agents.len() {
            for j in (i + 1)..self.agents.len() {
                let agent1_id = self.agents[i].id;
                let agent2_id = self.agents[j].id;
                let agent1_pos = self.agents[i].state.position;
                let agent2_pos = self.agents[j].state.position;

                // Calculate squared distance (avoid expensive sqrt)
                let dx = (agent1_pos.0 - agent2_pos.0) as f32;
                let dy = (agent1_pos.1 - agent2_pos.1) as f32;
                let distance_squared = dx * dx + dy * dy;

                // Only share when very close (within 5 tiles)
                if distance_squared <= SHARE_RANGE_SQUARED {
                    // Get technologies each agent knows
                    let agent1_techs: Vec<_> = self.agents[i]
                        .technology_knowledge
                        .known_technologies
                        .keys()
                        .cloned()
                        .collect();

                    let agent2_techs: Vec<_> = self.agents[j]
                        .technology_knowledge
                        .known_technologies
                        .keys()
                        .cloned()
                        .collect();

                    // Get relationship trust (for teaching confidence)
                    let trust_1_to_2 = self.agents[i]
                        .relationships
                        .get_relationship(&agent2_id)
                        .map(|r| (r.bond_strength + 1.0) / 2.0) // Map -1..1 to 0..1
                        .unwrap_or(0.5);

                    let trust_2_to_1 = self.agents[j]
                        .relationships
                        .get_relationship(&agent1_id)
                        .map(|r| (r.bond_strength + 1.0) / 2.0)
                        .unwrap_or(0.5);

                    // Agent 1 teaches Agent 2
                    for tech_id in &agent1_techs {
                        if !agent2_techs.contains(tech_id) {
                            let teacher_confidence = self.agents[i]
                                .technology_knowledge
                                .teaching_confidence(tech_id);

                            // Only teach if confident enough
                            if teacher_confidence > 0.5 {
                                self.agents[j].technology_knowledge.learn_from_agent(
                                    tech_id.clone(),
                                    agent2_id,
                                    DiscoveryMethod::Instruction,
                                    teacher_confidence,
                                    trust_2_to_1,
                                    self.current_tick as u64,
                                );
                            }
                        }
                    }

                    // Agent 2 teaches Agent 1
                    for tech_id in &agent2_techs {
                        if !agent1_techs.contains(tech_id) {
                            let teacher_confidence = self.agents[j]
                                .technology_knowledge
                                .teaching_confidence(tech_id);

                            if teacher_confidence > 0.5 {
                                self.agents[i].technology_knowledge.learn_from_agent(
                                    tech_id.clone(),
                                    agent1_id,
                                    DiscoveryMethod::Instruction,
                                    teacher_confidence,
                                    trust_1_to_2,
                                    self.current_tick as u64,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Attempt technology discovery through experimentation
    ///
    /// Curious agents with required materials may discover new technologies.
    fn discover_technologies(&mut self) {
        use crate::environment::technology::DiscoveryMethod;
        use crate::core::DriveType;
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let current_tick = self.current_tick;

        // Collect discoveries to be made (to avoid borrowing issues)
        // (agent_idx, tech_id, agent_uuid, position)
        let mut discoveries: Vec<(usize, String, Uuid, (i32, i32))> = Vec::new();

        // Get list of discoverable technologies for each agent
        for agent_idx in 0..self.agents.len() {
            let agent = &self.agents[agent_idx];

            // Check curiosity drive
            let curiosity = agent.drives
                .get(DriveType::Curiosity)
                .map(|d| d.value)
                .unwrap_or(0.0);

            // Only curious agents experiment
            if curiosity < 0.3 {
                continue;
            }

            // Get known technologies
            let known_techs = agent.technology_knowledge.known_technologies.clone();

            // Get technologies available for discovery
            let available_techs = self.technology_registry.available_for_discovery(&known_techs);

            for tech in available_techs {
                // Check if agent meets curiosity threshold
                if curiosity < tech.curiosity_threshold {
                    continue;
                }

                // Check if agent has required materials in inventory
                let has_materials = tech.required_materials.iter().all(|material| {
                    agent.inventory.get_item(material).is_some()
                });

                if !has_materials {
                    continue;
                }

                // Attempt discovery
                let discovery_roll: f32 = rng.gen();
                if discovery_roll < tech.discovery_chance * curiosity {
                    // Record discovery for later
                    let pos = (agent.state.position.0, agent.state.position.1);
                    discoveries.push((agent_idx, tech.id.clone(), agent.id, pos));
                    break; // Only one discovery per tick per agent
                }
            }
        }

        // Now apply all discoveries
        for (agent_idx, tech_id, agent_id, pos) in discoveries {
            let is_world_first = self.technology_registry.record_first_discovery(
                tech_id.clone(),
                agent_id,
                current_tick as u64,
            );

            // Emit technology discovery event
            self.pending_events.push(SimulationEvent::new(
                self.current_tick,
                SimulationEventType::TechnologyDiscovered {
                    tech_id: tech_id.clone(),
                    discoverer_id: agent_id,
                    is_world_first,
                },
                Some(pos),
            ));

            self.agents[agent_idx].technology_knowledge.discover_technology(
                tech_id,
                agent_id,
                DiscoveryMethod::Experimentation,
                current_tick as u64,
                is_world_first,
            );
        }
    }

    /// Remove dead agents from population and process grief for survivors
    fn process_deaths(&mut self) {
        use crate::agents::EmotionSource;

        // Identify dead agents before removing them, collecting position and detailed cause
        let dead_agents: Vec<(uuid::Uuid, String, (i32, i32), DeathCause)> = self.agents
            .iter()
            .filter(|agent| !agent.state.is_alive)
            .map(|agent| {
                // Determine cause of death from agent state
                let (cause_str, cause_enum) = if agent.state.is_starving() {
                    ("starvation".to_string(), DeathCause::Starvation)
                } else if agent.state.is_dehydrated() {
                    ("dehydration".to_string(), DeathCause::Dehydration)
                } else if agent.state.age >= agent.state.max_age {
                    ("old age".to_string(), DeathCause::OldAge)
                } else if agent.state.health <= 0.0 {
                    // Could be combat or other damage
                    if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
                        ("combat".to_string(), DeathCause::Combat { killer_id: Some(attacker_id) })
                    } else {
                        ("health depletion".to_string(), DeathCause::Unknown)
                    }
                } else if agent.state.energy <= 0.0 {
                    ("exhaustion".to_string(), DeathCause::Exhaustion)
                } else {
                    ("unknown cause".to_string(), DeathCause::Unknown)
                };
                let pos = (agent.state.position.0, agent.state.position.1);
                (agent.id, cause_str, pos, cause_enum)
            })
            .collect();

        if dead_agents.is_empty() {
            return; // No deaths to process
        }

        // Emit death events for timeline
        for (deceased_id, _cause_str, pos, cause_enum) in &dead_agents {
            self.pending_events.push(SimulationEvent::new(
                self.current_tick,
                SimulationEventType::Death {
                    agent_id: *deceased_id,
                    cause: cause_enum.clone(),
                },
                Some(*pos),
            ));
        }

        // Process grief for each death
        for (deceased_id, cause_description, _, _) in &dead_agents {
            let cause_source = EmotionSource::Event(cause_description.clone());

            // Notify all surviving agents about the death
            for agent in &mut self.agents {
                if !agent.state.is_alive || agent.id == *deceased_id {
                    continue; // Skip dead agents and self
                }

                // Check if they had a relationship with the deceased
                let had_relationship = agent.relationships.get_relationship(deceased_id).is_some();

                // Check if deceased was a drive satisfaction source (without removing yet)
                let drive_dependencies: Vec<crate::core::DriveType> = [
                    crate::core::DriveType::Social,
                    crate::core::DriveType::Reproduction,
                    crate::core::DriveType::Safety,
                    crate::core::DriveType::Hunger,
                    crate::core::DriveType::Shelter,
                ]
                .iter()
                .copied()
                .filter(|drive_type| {
                    agent.get_source_importance(*drive_type, *deceased_id) > 0.05
                })
                .collect();

                let had_drive_dependency = !drive_dependencies.is_empty();

                if had_relationship || had_drive_dependency {
                    // 1. Existing relationship grief (if they were loved ones)
                    if let Some(relationship) = agent.relationships.get_relationship(deceased_id) {
                        if relationship.is_loved_one() {
                            agent.respond_to_loved_one_death(deceased_id, cause_source.clone());
                        }
                    }

                    // 2. NEW: Functional grief from losing drive satisfaction sources
                    // This will remove the source and trigger appropriate emotions
                    for drive_type in drive_dependencies {
                        agent.process_drive_source_loss_with_cause(
                            drive_type,
                            *deceased_id,
                            Some(cause_source.clone())
                        );
                    }

                    // 3. Share death information via gossip system
                    if agent.knowledge.known_information.len() < 1000 {  // Don't overflow knowledge
                        use crate::agents::gossip::{Information, InformationType};
                        let death_info = Information::new(
                            InformationType::Death {
                                agent: *deceased_id,
                                cause: cause_description.clone(),
                            },
                            *deceased_id, // Source is the deceased
                            true, // Ground truth
                            self.current_tick as u64,
                        );
                        agent.knowledge.known_information.insert(death_info.id, death_info);
                    }
                }
            }
        }

        // Now remove the dead agents
        let before = self.agents.len();
        self.agents.retain(|agent| agent.state.is_alive);
        let deaths = before - self.agents.len();
        self.stats.total_deaths += deaths as u64;

        // Clean up tracking for dead agents
        for (deceased_id, _, _, _) in &dead_agents {
            self.unhappiness_tracker.remove(deceased_id);
            self.reproduction_cooldown.remove(deceased_id);
        }
    }

    /// Process agent abandonments based on unhappiness
    /// Agents who are severely unhappy for extended periods may leave the town
    pub fn process_abandonments(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Track unhappiness and identify agents who should abandon (with position)
        let mut agents_to_remove: Vec<(Uuid, (i32, i32))> = Vec::new();

        for agent in &self.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Get agent's current happiness from emotions
            let happiness = agent.emotions.happiness;

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
                        agents_to_remove.push((agent.id, (agent.state.position.0, agent.state.position.1)));
                    }
                }
            } else {
                // Agent is happy, reset unhappiness tracker
                self.unhappiness_tracker.remove(&agent.id);
            }
        }

        // Remove agents who are abandoning
        if !agents_to_remove.is_empty() {
            // Emit abandonment events
            for (agent_id, pos) in &agents_to_remove {
                self.pending_events.push(SimulationEvent::new(
                    self.current_tick,
                    SimulationEventType::Abandonment {
                        agent_id: *agent_id,
                    },
                    Some(*pos),
                ));
            }

            let agent_ids: Vec<Uuid> = agents_to_remove.iter().map(|(id, _)| *id).collect();
            self.agents.retain(|agent| !agent_ids.contains(&agent.id));
            self.stats.total_abandonments += agents_to_remove.len() as u64;

            // Clean up tracking for abandoned agents
            for (agent_id, _) in agents_to_remove {
                self.unhappiness_tracker.remove(&agent_id);
                self.reproduction_cooldown.remove(&agent_id);
            }
        }
    }

    /// Process reproduction attempts
    ///
    /// Agents will only reproduce when survival needs are met (not hungry/thirsty).
    /// This naturally limits population growth based on resource availability.
    pub fn process_reproduction(&mut self) {
        let mut new_offspring = Vec::new();

        // Find potential mating pairs
        // Use should_attempt_reproduction() which checks both capability AND survival state
        // Agents with active hunger/thirst drives are excluded - they must secure food first
        let alive_agents: Vec<usize> = self.agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.should_attempt_reproduction())
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
                        let child_id = offspring.id;
                        let child_pos = (offspring.state.position.0, offspring.state.position.1);
                        let parent1_id = agent1.id;
                        let parent2_id = agent2.id;

                        new_offspring.push(offspring);

                        // Emit birth event
                        self.pending_events.push(SimulationEvent::new(
                            self.current_tick,
                            SimulationEventType::Birth {
                                mother_id: parent1_id,
                                child_id,
                                father_id: Some(parent2_id),
                            },
                            Some(child_pos),
                        ));

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

                // Mark children in parent's relationships
                for (offspring_id, parent_ids) in &offspring_ids {
                    if parent_ids.contains(&agent.id) {
                        use crate::agents::emotions::{Relationship, RelationshipType};
                        agent.relationships.add_relationship(
                            Relationship::new(*offspring_id, RelationshipType::Child)
                        );
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
        let total_happiness: f32 = alive_agents.iter()
            .map(|a| a.emotions.happiness)
            .sum();
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

    /// Process social interactions between nearby agents
    ///
    /// Agents with active survival drives (hunger/thirst) will not engage in social
    /// interactions - they must prioritize finding food and water over socializing.
    pub fn process_social_interactions(&mut self) {
        use crate::agents::social_interactions::{
            SocialInteractionType,
            calculate_relationship_change, calculate_social_satisfaction,
            should_greet, select_conversation_topic,
        };
        use crate::core::DriveType;
        use rand::Rng;

        // Pre-compute squared distance threshold (avoids sqrt)
        const SOCIAL_RANGE_SQUARED: f32 = 25.0; // 5.0 * 5.0

        let mut rng = rand::thread_rng();
        let current_tick = self.current_tick;

        // Collect interaction pairs (to avoid borrowing issues)
        let mut interactions = Vec::new();

        // Find nearby agent pairs who want to socialize
        for i in 0..self.agents.len() {
            if !self.agents[i].state.is_alive {
                continue;
            }

            // Skip agents with active survival drives - they must focus on survival
            let hunger_active = self.agents[i].drives.get(DriveType::Hunger)
                .map(|d| d.is_active())
                .unwrap_or(false);
            let thirst_active = self.agents[i].drives.get(DriveType::Thirst)
                .map(|d| d.is_active())
                .unwrap_or(false);
            if hunger_active || thirst_active {
                continue;
            }

            let _agent1_id = self.agents[i].id;
            let agent1_pos = self.agents[i].state.position;
            let agent1_social_drive = self.agents[i].drives.get(DriveType::Social)
                .map(|d| d.value)
                .unwrap_or(0.0);

            // Only socialize if social drive is somewhat active (>0.3)
            if agent1_social_drive < 0.3 {
                continue;
            }

            for j in (i + 1)..self.agents.len() {
                if !self.agents[j].state.is_alive {
                    continue;
                }

                // Skip agents with active survival drives
                let hunger_active_2 = self.agents[j].drives.get(DriveType::Hunger)
                    .map(|d| d.is_active())
                    .unwrap_or(false);
                let thirst_active_2 = self.agents[j].drives.get(DriveType::Thirst)
                    .map(|d| d.is_active())
                    .unwrap_or(false);
                if hunger_active_2 || thirst_active_2 {
                    continue;
                }

                let _agent2_id = self.agents[j].id;
                let agent2_pos = self.agents[j].state.position;

                // Calculate squared distance (avoid expensive sqrt)
                let dx = (agent1_pos.0 - agent2_pos.0) as f32;
                let dy = (agent1_pos.1 - agent2_pos.1) as f32;
                let distance_squared = dx * dx + dy * dy;

                // Must be within social interaction range (5 tiles)
                if distance_squared > SOCIAL_RANGE_SQUARED {
                    continue;
                }

                // Check if they should interact
                // Higher social drive = higher interaction probability
                let interaction_probability = (agent1_social_drive * 0.3).min(0.5);
                if !rng.gen_bool(interaction_probability as f64) {
                    continue;
                }

                interactions.push((i, j));
            }
        }

        // Process each interaction
        for (i, j) in interactions {
            let agent1_id = self.agents[i].id;
            let agent2_id = self.agents[j].id;

            // Get relationship info (or create new relationship)
            let relationship_1_to_2 = self.agents[i]
                .relationships
                .get_or_create_relationship(agent2_id, current_tick)
                .clone();

            let relationship_2_to_1 = self.agents[j]
                .relationships
                .get_or_create_relationship(agent1_id, current_tick)
                .clone();

            // Get traits
            let agent1_traits: Vec<Trait> = self.agents[i].traits.get_traits().to_vec();
            let agent2_traits: Vec<Trait> = self.agents[j].traits.get_traits().to_vec();

            // Determine interaction type
            let interaction_type = if should_greet(
                relationship_1_to_2.last_interaction_tick,
                current_tick,
                &relationship_1_to_2.relationship_level(),
            ) {
                // Greet if haven't interacted recently
                SocialInteractionType::Greet
            } else {
                // Otherwise, have a conversation
                let topic = select_conversation_topic(
                    &relationship_1_to_2.relationship_level(),
                    &agent1_traits,
                    &agent2_traits,
                );
                SocialInteractionType::Converse { topic }
            };

            // Calculate relationship changes for both agents
            let rel_change_1 = calculate_relationship_change(
                &interaction_type,
                &agent1_traits,
                &agent2_traits,
                &relationship_1_to_2.relationship_level(),
            );

            let rel_change_2 = calculate_relationship_change(
                &interaction_type,
                &agent2_traits,
                &agent1_traits,
                &relationship_2_to_1.relationship_level(),
            );

            // Calculate social satisfaction for both agents
            let satisfaction_1 = calculate_social_satisfaction(
                &interaction_type,
                &agent1_traits,
                &relationship_1_to_2.relationship_level(),
            );

            let satisfaction_2 = calculate_social_satisfaction(
                &interaction_type,
                &agent2_traits,
                &relationship_2_to_1.relationship_level(),
            );

            // Apply changes to agent 1
            if let Some(rel) = self.agents[i].relationships.get_relationship_mut(&agent2_id) {
                rel.positive_interaction(rel_change_1, current_tick);
            }
            if let Some(drive) = self.agents[i].drives.get_mut(DriveType::Social) {
                drive.partial_satisfy(satisfaction_1);
            }

            // Apply changes to agent 2
            if let Some(rel) = self.agents[j].relationships.get_relationship_mut(&agent1_id) {
                rel.positive_interaction(rel_change_2, current_tick);
            }
            if let Some(drive) = self.agents[j].drives.get_mut(DriveType::Social) {
                drive.partial_satisfy(satisfaction_2);
            }
        }
    }

    /// Process gossip spreading between nearby agents
    ///
    /// Agents share information from their knowledge base with nearby agents.
    /// Information is distorted based on the sharer's personality traits.
    /// Trust ratings affect how much weight the receiver gives to information.
    pub fn process_gossip(&mut self) {
        use crate::agents::gossip::{Information, InformationType};
        use crate::core::DriveType;
        use rand::seq::SliceRandom;
        use rand::Rng;

        const GOSSIP_RANGE_SQUARED: f32 = 36.0; // 6 tiles - slightly further than social range

        let mut rng = rand::thread_rng();
        let current_tick = self.current_tick;

        // Collect gossip pairs and what info to share
        let mut gossip_events: Vec<(usize, usize, Information)> = Vec::new();

        // Find nearby agent pairs who might gossip
        for i in 0..self.agents.len() {
            if !self.agents[i].state.is_alive {
                continue;
            }

            // Skip agents with active survival drives
            let hunger_active = self.agents[i].drives.get(DriveType::Hunger)
                .map(|d| d.value > 0.7)
                .unwrap_or(false);
            let thirst_active = self.agents[i].drives.get(DriveType::Thirst)
                .map(|d| d.value > 0.7)
                .unwrap_or(false);
            if hunger_active || thirst_active {
                continue;
            }

            // Calculate gossip probability based on traits
            let gossip_probability = self.calculate_gossip_probability(&self.agents[i]);
            if gossip_probability <= 0.0 {
                continue;
            }

            // Check if agent has any information to share
            if self.agents[i].knowledge.known_information.is_empty() {
                continue;
            }

            let agent1_pos = self.agents[i].state.position;

            for j in (i + 1)..self.agents.len() {
                if !self.agents[j].state.is_alive {
                    continue;
                }

                // Skip agents with active survival drives
                let hunger_active_2 = self.agents[j].drives.get(DriveType::Hunger)
                    .map(|d| d.value > 0.7)
                    .unwrap_or(false);
                let thirst_active_2 = self.agents[j].drives.get(DriveType::Thirst)
                    .map(|d| d.value > 0.7)
                    .unwrap_or(false);
                if hunger_active_2 || thirst_active_2 {
                    continue;
                }

                let agent2_pos = self.agents[j].state.position;

                // Calculate squared distance
                let dx = (agent1_pos.0 - agent2_pos.0) as f32;
                let dy = (agent1_pos.1 - agent2_pos.1) as f32;
                let distance_squared = dx * dx + dy * dy;

                if distance_squared > GOSSIP_RANGE_SQUARED {
                    continue;
                }

                // Roll for gossip attempt
                if rng.gen::<f32>() > gossip_probability {
                    continue;
                }

                // Select random information to share from agent i
                let info_ids: Vec<_> = self.agents[i].knowledge.known_information.keys().cloned().collect();
                if let Some(info_id) = info_ids.choose(&mut rng) {
                    if let Some(info) = self.agents[i].knowledge.known_information.get(info_id) {
                        // Don't share very old information (older than 10000 ticks)
                        if current_tick as u64 - info.timestamp < 10000 {
                            // Filter: don't share information about the recipient
                            let is_about_recipient = match &info.info_type {
                                InformationType::Death { agent, .. } => *agent == self.agents[j].id,
                                InformationType::Conflict { agent1, agent2 } => {
                                    *agent1 == self.agents[j].id || *agent2 == self.agents[j].id
                                }
                                InformationType::EmotionalOutburst { agent, .. } => *agent == self.agents[j].id,
                                InformationType::Accusation { accused, .. } => *accused == self.agents[j].id,
                                InformationType::AgentTrait { agent, .. } => *agent == self.agents[j].id,
                                _ => false,
                            };

                            if !is_about_recipient {
                                gossip_events.push((i, j, info.clone()));
                            }
                        }
                    }
                }

                // Agent j might also share with agent i (bidirectional gossip)
                let gossip_probability_j = self.calculate_gossip_probability(&self.agents[j]);
                if gossip_probability_j > 0.0
                    && rng.gen::<f32>() < gossip_probability_j
                    && !self.agents[j].knowledge.known_information.is_empty()
                {
                    let info_ids_j: Vec<_> = self.agents[j].knowledge.known_information.keys().cloned().collect();
                    if let Some(info_id) = info_ids_j.choose(&mut rng) {
                        if let Some(info) = self.agents[j].knowledge.known_information.get(info_id) {
                            if current_tick as u64 - info.timestamp < 10000 {
                                let is_about_recipient = match &info.info_type {
                                    InformationType::Death { agent, .. } => *agent == self.agents[i].id,
                                    InformationType::Conflict { agent1, agent2 } => {
                                        *agent1 == self.agents[i].id || *agent2 == self.agents[i].id
                                    }
                                    InformationType::EmotionalOutburst { agent, .. } => *agent == self.agents[i].id,
                                    InformationType::Accusation { accused, .. } => *accused == self.agents[i].id,
                                    InformationType::AgentTrait { agent, .. } => *agent == self.agents[i].id,
                                    _ => false,
                                };

                                if !is_about_recipient {
                                    gossip_events.push((j, i, info.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process all gossip events (share information with distortion)
        for (sharer_idx, receiver_idx, info) in gossip_events {
            let sharer_id = self.agents[sharer_idx].id;
            let sharer_traits = self.agents[sharer_idx].traits.clone();

            // Apply distortion based on sharer's traits
            let distorted_info = if let Some(distortion_trait) = sharer_traits.would_distort_info() {
                info.distort(distortion_trait, sharer_id)
            } else if self.agents[sharer_idx].traits.has(Trait::Gossip) {
                // Gossip trait also causes exaggeration
                info.distort(Trait::Gossip, sharer_id)
            } else {
                info.clone()
            };

            // Check if receiver already knows this exact information
            let already_knows = self.agents[receiver_idx].knowledge
                .known_information
                .values()
                .any(|existing| existing.info_type == distorted_info.info_type);

            if !already_knows {
                // Receiver processes the information
                let receiver_id = self.agents[receiver_idx].id;
                let receiver_traits = self.agents[receiver_idx].traits.clone();

                self.agents[receiver_idx].knowledge.receive_information(
                    distorted_info,
                    sharer_id,
                    receiver_id,
                    &receiver_traits,
                    current_tick as u64,
                );

                // Gossip trait agents get happiness from sharing
                if self.agents[sharer_idx].traits.has(Trait::Gossip) {
                    self.agents[sharer_idx].emotions.happiness =
                        (self.agents[sharer_idx].emotions.happiness + 0.02).min(1.0);
                }
            }
        }
    }

    /// Calculate gossip probability for an agent based on traits
    pub fn calculate_gossip_probability(&self, agent: &Agent) -> f32 {
        let mut probability: f32 = 0.15; // Base 15% chance to gossip when nearby

        // Gossip trait: much more likely to share
        if agent.traits.has(Trait::Gossip) {
            probability += 0.35;
        }

        // Extrovert: more likely to share
        if agent.traits.has(Trait::Extrovert) {
            probability += 0.15;
        }

        // Charismatic: more likely to engage
        if agent.traits.has(Trait::Charismatic) {
            probability += 0.10;
        }

        // Introvert: less likely to share
        if agent.traits.has(Trait::Introvert) || agent.traits.has(Trait::Introverted) {
            probability -= 0.20;
        }

        // Stoic: less chatty
        if agent.traits.has(Trait::Stoic) {
            probability -= 0.10;
        }

        // Honest: won't spread unverified info as freely
        if agent.traits.has(Trait::Honest) {
            probability -= 0.05;
        }

        probability.clamp(0.0, 0.8) // Clamp between 0% and 80%
    }

    /// Process exploration for all living agents
    /// Agents discover tiles within their vision range
    pub fn process_exploration_with_world(&mut self, world: &mut crate::world::World) {
        use crate::core::DriveType;

        let current_tick = self.current_tick;

        for agent in &mut self.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Get agent position
            let agent_pos = crate::world::Position::new(
                agent.state.position.0,
                agent.state.position.1,
            );

            // Vision range based on terrain and conditions (default 10 tiles)
            let vision_range = 10;

            // Process exploration - discovers tiles, resources, buildings
            let new_discoveries = world.process_exploration(
                &mut agent.exploration_knowledge,
                &agent_pos,
                vision_range,
                current_tick,
            );

            // Satisfy curiosity drive based on discoveries
            if new_discoveries > 0 {
                if let Some(drive) = agent.drives.get_mut(DriveType::Curiosity) {
                    // Each new tile discovery provides small curiosity satisfaction
                    let satisfaction = (new_discoveries as f32 * 0.02).min(0.5);
                    drive.partial_satisfy(satisfaction);
                }

                // Also track in observational learning if discovering new actions
                // (future enhancement: learn from discovered resources/buildings)
            }
        }
    }

    /// Process exploration without world (for standalone population updates)
    /// This is called from tick() and handles exploration-related drive updates
    /// and knowledge sharing between nearby agents
    fn process_exploration(&mut self) {
        use crate::core::DriveType;

        const EXPLORATION_SHARE_RANGE_SQ: f32 = 25.0; // 5 tiles

        // First pass: identify agent positions for knowledge sharing
        let agent_positions: Vec<(usize, (i32, i32, i32), bool)> = self.agents
            .iter()
            .enumerate()
            .map(|(i, a)| (i, a.state.position, a.state.is_alive))
            .collect();

        // Second pass: process exploration drives and knowledge sharing
        for i in 0..self.agents.len() {
            let agent = &mut self.agents[i];
            if !agent.state.is_alive {
                continue;
            }

            // Slowly increase curiosity drive when not actively discovering
            // This makes agents want to explore over time
            if let Some(curiosity_drive) = agent.drives.get_mut(DriveType::Curiosity) {
                // Curiosity increases by 0.002 per tick if below 0.7
                if curiosity_drive.value < 0.7 {
                    curiosity_drive.value = (curiosity_drive.value + 0.002).min(0.7);
                }
            }
        }

        // Knowledge sharing between nearby agents (simplified gossip about discoveries)
        // Share random discoveries with nearby agents
        for i in 0..self.agents.len() {
            let (_, pos_i, alive_i) = agent_positions[i];
            if !alive_i {
                continue;
            }

            #[allow(clippy::needless_range_loop)]
            for j in (i + 1)..self.agents.len() {
                let (_, pos_j, alive_j) = agent_positions[j];
                if !alive_j {
                    continue;
                }

                // Check if within sharing range
                let dx = (pos_i.0 - pos_j.0) as f32;
                let dy = (pos_i.1 - pos_j.1) as f32;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq <= EXPLORATION_SHARE_RANGE_SQ {
                    // Share a random discovery from i to j and vice versa
                    // This simulates agents telling each other about places they've been
                    let current_tick = self.current_tick;

                    // Agent i shares with agent j
                    if let Some((pos, building_type)) = self.agents[i].exploration_knowledge
                        .known_buildings.iter().next().map(|(p, t)| (*p, *t))
                    {
                        self.agents[j].exploration_knowledge
                            .discover_building(pos, building_type, current_tick);
                    }

                    // Agent j shares with agent i
                    if let Some((pos, building_type)) = self.agents[j].exploration_knowledge
                        .known_buildings.iter().next().map(|(p, t)| (*p, *t))
                    {
                        self.agents[i].exploration_knowledge
                            .discover_building(pos, building_type, current_tick);
                    }
                }
            }
        }
    }

    /// Process observational learning between agents
    ///
    /// This handles:
    /// - Broadcasting actions to nearby observers
    /// - Automatic adoption of ready behaviors
    /// - Skill learning from adopted behaviors
    pub fn process_observational_learning(&mut self) {
        use super::observation_processing::{auto_adopt_ready_behaviors};

        // Process auto-adoption for each agent
        for i in 0..self.agents.len() {
            let adopted = auto_adopt_ready_behaviors(&mut self.agents[i]);

            // Log adoptions (could be extended to notify parents, etc.)
            if !adopted.is_empty() {
                for (teacher_id, action_type) in adopted {
                    // Future: Could add events, notifications, or drive satisfaction here
                    // For now, just record that learning happened
                    let _ = (teacher_id, action_type);
                }
            }
        }
    }

    /// Broadcast an action from one agent to all nearby observers
    ///
    /// This should be called whenever an agent performs a visible action
    pub fn broadcast_action(
        &mut self,
        performer_id: uuid::Uuid,
        position: (i32, i32, i32),
        action_type: super::ActionType,
        success: bool,
        details: String,
        timestamp: u64,
    ) {
        use super::observation_processing::{BroadcastAction, process_observations};

        let broadcast = BroadcastAction::new(
            performer_id,
            position,
            action_type,
            success,
            details,
            timestamp,
        );

        process_observations(&mut self.agents, &broadcast);
    }

    /// Get observational learning statistics for the entire population
    pub fn get_population_learning_stats(&self) -> PopulationLearningStats {
        use super::observation_processing::get_learning_stats;

        let mut total_adopted = 0;
        let mut total_ready = 0;
        let mut agents_learning_from_parents = 0;
        let mut total_unique_teachers = 0;

        for agent in &self.agents {
            let stats = get_learning_stats(agent);
            total_adopted += stats.total_adopted;
            total_ready += stats.ready_to_adopt;
            total_unique_teachers += stats.unique_teachers;
            if stats.learning_from_parents > 0 {
                agents_learning_from_parents += 1;
            }
        }

        PopulationLearningStats {
            total_behaviors_adopted: total_adopted,
            total_ready_to_adopt: total_ready,
            agents_learning_from_parents,
            average_unique_teachers: if self.agents.is_empty() {
                0.0
            } else {
                total_unique_teachers as f32 / self.agents.len() as f32
            },
        }
    }

    /// Check which agents are actively learning (have recent observations)
    pub fn get_active_learners(&self) -> Vec<(uuid::Uuid, usize)> {
        self.agents
            .iter()
            .filter_map(|agent| {
                let opportunities = agent.check_learning_opportunities();
                if opportunities.is_empty() {
                    None
                } else {
                    Some((agent.id, opportunities.len()))
                }
            })
            .collect()
    }

    /// Get parent-child learning pairs
    pub fn get_parent_child_learning(&self) -> Vec<(uuid::Uuid, uuid::Uuid, Vec<super::ActionType>)> {
        let mut learning_pairs = Vec::new();

        for child in &self.agents {
            let parent_learning = child.learning_from_parents();
            for (parent_id, actions) in parent_learning {
                learning_pairs.push((child.id, parent_id, actions));
            }
        }

        learning_pairs
    }

    /// Drain and return all pending events
    ///
    /// This is called by the GUI snapshot system to collect events for the timeline.
    pub fn drain_events(&mut self) -> Vec<SimulationEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Get pending events without draining (for read-only access)
    pub fn get_pending_events(&self) -> &[SimulationEvent] {
        &self.pending_events
    }
}

/// Statistics about observational learning in the population
#[derive(Debug, Clone)]
pub struct PopulationLearningStats {
    pub total_behaviors_adopted: usize,
    pub total_ready_to_adopt: usize,
    pub agents_learning_from_parents: usize,
    pub average_unique_teachers: f32,
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
