// src/agents/population.rs
use crate::agents::{Agent, AgentConfig, SharedKnowledge, Trait};
use crate::agents::{can_mate, reproduce, attempt_impregnation, give_birth, MateSelectionCriteria, PregnancyState};
use crate::environment::technology::TechnologyRegistry;
#[cfg(feature = "gui")]
use crate::gui::events::{SimulationEvent, SimulationEventType, DeathCause};
#[cfg(not(feature = "gui"))]
use crate::agents::population::gui_stubs::{SimulationEvent, SimulationEventType, DeathCause};
use uuid::Uuid;
use std::collections::HashMap;

#[cfg(not(feature = "gui"))]
mod gui_stubs {
    use uuid::Uuid;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum DeathCause {
        OldAge,
        Starvation,
        Dehydration,
        Combat { killer_id: Option<Uuid> },
        Exhaustion,
        Exposure,
        Unknown,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum SimulationEventType {
        Birth {
            mother_id: Uuid,
            child_id: Uuid,
            father_id: Option<Uuid>,
        },
        Death {
            agent_id: Uuid,
            cause: DeathCause,
        },
        Pregnancy {
            mother_id: Uuid,
            father_id: Uuid,
        },
        Abandonment {
            agent_id: Uuid,
        },
        TechnologyDiscovered {
            tech_id: String,
            discoverer_id: Uuid,
            is_world_first: bool,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SimulationEvent {
        pub id: Uuid,
        pub tick: u32,
        pub event_type: SimulationEventType,
        pub position: Option<(i32, i32)>,
    }

    impl SimulationEvent {
        pub fn new(tick: u32, event_type: SimulationEventType, position: Option<(i32, i32)>) -> Self {
            Self {
                id: Uuid::new_v4(),
                tick,
                event_type,
                position,
            }
        }
    }
}

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
    // Per-tick tracking (reset at start of each tick)
    pub births_this_tick: u32,
    pub deaths_this_tick: u32,
    pub abandonments_this_tick: u32,
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
    /// Where bodies fell since the simulation last collected them, and what
    /// each is worth to the ground as soft matter and as bone. A population
    /// has no world to put them in, so it holds them here until one does.
    pub bodies_where_they_fell: Vec<((i32, i32, i32), f32, f32)>,
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
            bodies_where_they_fell: Vec::new(),
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
            bodies_where_they_fell: Vec::new(),
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

        // Give the person a personality.
        //
        // Agents used to carry `TraitSet::default()`, which is empty, and
        // nothing on any live path added a trait except the congenital
        // infertility roll. So no agent in a running world held one of the
        // sixty-odd defined traits, and everything downstream of them - the
        // job affinities, the gossip distortion, the affinity model that
        // decides who gets on with whom, the emotional modifiers, how long a
        // plan a person will countenance, the religious effects - had an input
        // that never varied. A settlement of eighty people was eighty copies of
        // the same person.
        //
        // Inheritance was already written and already being called on every
        // birth. It simply had nothing to inherit, because the founding
        // generation had nothing. This is the one place that was missing: a
        // founder has no parents to take after, so they are drawn from the
        // pool, and everybody born afterwards takes after the two people who
        // made them.
        agent.traits = crate::core::traits::TraitSet::a_person();
        agent.apply_trait_sensory_modifications();

        // And let it reach what they want, not only what they can see. Without
        // this a personality decides how somebody feels about their life and
        // nothing about how they spend it.
        agent.drives.lean_towards(&agent.traits);

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

        // Reset per-tick counters at the start of each tick
        self.stats.births_this_tick = 0;
        self.stats.deaths_this_tick = 0;
        self.stats.abandonments_this_tick = 0;

        // Update shared knowledge tick counter
        self.shared_knowledge.tick();

        // Update all agents
        let current_tick = self.current_tick;
        for agent in &mut self.agents {
            agent.tick_with_percepts(current_tick); // Process percepts with timestamp
            // Aging, metabolism, food spoilage and fatigue (pregnancy modifier applied inside)
            agent.process_survival_tick(current_tick);
        }

        // Update relationships between nearby agents
        self.update_relationships();

        // Decay distant relationships (every 100 ticks to reduce overhead)
        if current_tick % 100 == 0 {
            self.decay_relationships();
        }

        // Process social interactions (every 10 ticks to reduce overhead)
        if current_tick % 10 == 0 {
            self.process_social_interactions();
        }

        // Process trait-based proximity effects (every 10 ticks)
        // Handles: Romantic partner happiness, Mediator calming, Intolerant stranger penalty
        if current_tick % 10 == 0 {
            self.process_trait_proximity_effects();
        }

        // Who can see whom.
        //
        // Nothing populated `vision.visible_agents`, and observation is gated
        // on it, so no agent had ever recorded seeing another do anything:
        // the whole observational learning system ran every twenty ticks over
        // an empty list. It is also what `Percept::AgentDetected` is built on.
        self.update_who_can_see_whom();

        // Process observational learning (every 20 ticks to reduce overhead)
        if current_tick % 20 == 0 {
            self.process_observational_learning();
        }

        // Process exploration for all agents (vision-based discovery)
        self.process_exploration();

        // Share technologies between nearby agents
        self.share_technologies();

        // Attempt technology discovery (every 50 ticks to reduce overhead)
        if current_tick % 50 == 0 {
            self.discover_technologies();
        }

        // Process unhappiness tracking and abandonments
        self.process_abandonments();

        // Process deaths
        self.process_deaths();

        // Process active pregnancies (nutrition updates and births)
        self.process_pregnancies();

        // Process new reproduction attempts
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
                .iter()
                .map(|(id, _)| *id)
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

        // Where the bodies fell, and what each was worth to the ground. The
        // simulation puts them there: the population has no world to put them
        // in.
        for agent in self.agents.iter().filter(|agent| !agent.state.is_alive) {
            let (soft, bone) = agent.state.life_stage.body_left_behind();
            self.bodies_where_they_fell
                .push((agent.state.position, soft, bone));
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
        self.stats.deaths_this_tick += deaths as u32;

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
                        // Determine male/female for impregnation
                        use crate::agents::gender::Gender;
                        let (male_idx, female_idx) = match (agent1.gender, agent2.gender) {
                            (Gender::Male, Gender::Female) => (idx1, idx2),
                            (Gender::Female, Gender::Male) => (idx2, idx1),
                            _ => continue, // Same-sex pairs can't reproduce
                        };

                        let male = &self.agents[male_idx];
                        let female = &self.agents[female_idx];

                        // Try to impregnate - this uses proper pregnancy system
                        if let Some(pregnancy) = attempt_impregnation(male, female, self.current_tick) {
                            let mother_id = female.id;
                            let father_id = male.id;
                            let pos = (female.state.position.0, female.state.position.1);

                            // Store pregnancy info to apply after iteration
                            new_offspring.push((female_idx, pregnancy, mother_id, father_id, pos));

                            // Add cooldown (prevent immediate re-reproduction)
                            self.reproduction_cooldown.insert(mother_id, 800); // Full pregnancy duration
                            self.reproduction_cooldown.insert(father_id, 100); // Short cooldown for males
                        }
                    }
                }
            }
        }

        // Apply pregnancies to female agents and emit pregnancy events
        for (female_idx, pregnancy, mother_id, father_id, pos) in new_offspring {
            self.agents[female_idx].pregnancy = Some(pregnancy);

            // Emit pregnancy event
            self.pending_events.push(SimulationEvent::new(
                self.current_tick,
                SimulationEventType::Pregnancy {
                    mother_id,
                    father_id,
                },
                Some(pos),
            ));

            // Partially satisfy reproduction drive (full satisfaction comes at birth)
            if let Some(drive) = self.agents[female_idx].drives.get_mut(crate::core::DriveType::Reproduction) {
                drive.value = (drive.value - 0.3).max(0.0);
            }
        }
    }

    /// Process active pregnancies and handle births
    /// Should be called every tick to update nutrition and check for due deliveries
    pub fn process_pregnancies(&mut self) {
        use crate::core::DriveType;

        // First pass: update pregnancy nutrition and collect due births
        let mut births_to_process: Vec<(usize, PregnancyState)> = Vec::new();

        for (idx, agent) in self.agents.iter_mut().enumerate() {
            if let Some(ref mut pregnancy) = agent.pregnancy {
                // Update prenatal nutrition based on mother's current state
                let hunger_drive = agent.drives.get(DriveType::Hunger)
                    .map(|d| d.value)
                    .unwrap_or(0.0);
                pregnancy.update_nutrition(hunger_drive, agent.state.health);

                // Check if due
                if pregnancy.is_due(self.current_tick) {
                    births_to_process.push((idx, pregnancy.clone()));
                }
            }
        }

        // Second pass: process births
        let mut new_offspring: Vec<Agent> = Vec::new();

        for (mother_idx, pregnancy) in births_to_process {
            // Clear pregnancy from mother
            self.agents[mother_idx].pregnancy = None;

            // Find father
            let father_idx = self.agents.iter()
                .position(|a| a.id == pregnancy.father_id);

            // Create offspring
            let offspring = if let Some(f_idx) = father_idx {
                let mother = &self.agents[mother_idx];
                let father = &self.agents[f_idx];
                give_birth(mother, father, &pregnancy, self.current_tick)
            } else {
                // Father not found (dead?), use legacy reproduce with just mother
                let mother = &self.agents[mother_idx];
                reproduce(mother, mother, self.current_tick)
            };

            let child_id = offspring.id;
            let mother_id = self.agents[mother_idx].id;
            let father_id = pregnancy.father_id;
            let child_pos = (offspring.state.position.0, offspring.state.position.1);

            // Emit birth event
            self.pending_events.push(SimulationEvent::new(
                self.current_tick,
                SimulationEventType::Birth {
                    mother_id,
                    child_id,
                    father_id: Some(father_id),
                },
                Some(child_pos),
            ));

            new_offspring.push(offspring);

            // Satisfy reproduction drive for mother
            if let Some(drive) = self.agents[mother_idx].drives.get_mut(DriveType::Reproduction) {
                drive.satisfy();
            }

            // Establish parent-child relationship for mother
            {
                use crate::agents::emotions::{Relationship, RelationshipType};
                self.agents[mother_idx].relationships.add_relationship(
                    Relationship::new(child_id, RelationshipType::Child)
                );
            }

            // Establish parent-child relationship for father if alive
            if let Some(f_idx) = father_idx {
                use crate::agents::emotions::{Relationship, RelationshipType};
                self.agents[f_idx].relationships.add_relationship(
                    Relationship::new(child_id, RelationshipType::Child)
                );
                // Satisfy reproduction drive for father
                if let Some(drive) = self.agents[f_idx].drives.get_mut(DriveType::Reproduction) {
                    drive.satisfy();
                }
            }
        }

        // Add offspring to population
        let birth_count = new_offspring.len();
        self.agents.extend(new_offspring);
        self.stats.total_births += birth_count as u64;
        self.stats.births_this_tick += birth_count as u32;
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

            // Calculate social range based on personality traits
            let agent1_social_range_sq = calculate_social_range_squared(&self.agents[i].traits.traits);

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

                // Use the larger of the two agents' social ranges
                // (more social agent can reach out to less social one)
                let agent2_social_range_sq = calculate_social_range_squared(&self.agents[j].traits.traits);
                let max_social_range_sq = agent1_social_range_sq.max(agent2_social_range_sq);

                // Must be within social interaction range
                if distance_squared > max_social_range_sq {
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
            let agent1_traits: Vec<Trait> = self.agents[i].traits.get_traits().iter().cloned().collect();
            let agent2_traits: Vec<Trait> = self.agents[j].traits.get_traits().iter().cloned().collect();

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
                if gossip_probability_j > 0.0 && rng.gen::<f32>() < gossip_probability_j {
                    if !self.agents[j].knowledge.known_information.is_empty() {
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

        probability.max(0.0).min(0.8) // Clamp between 0% and 80%
    }

    /// Process exploration for all living agents
    /// Agents discover tiles within their vision range
    pub fn process_exploration_with_world(&mut self, world: &mut crate::world::World) {
        use crate::core::memory::SpatialMemoryType;
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

            // How far this agent can make out detail, which is zero if it
            // cannot see: a blind agent discovers nothing by sight, and finds
            // the world by smell and memory instead.
            let vision_range = agent.sight_range();
            if vision_range == 0 {
                continue;
            }

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

                // Award Navigation skill XP for exploration
                agent.skills.gain_experience(super::SkillType::Navigation, new_discoveries as u32 * 2);
            }

            // Learn skills from newly discovered resources.
            let newly_seen: Vec<(crate::world::Position, crate::world::ResourceType)> = agent
                .exploration_knowledge
                .known_resources
                .iter()
                .filter(|(pos, _)| {
                    agent
                        .exploration_knowledge
                        .resource_discovery_ticks
                        .get(pos)
                        .map(|&tick| current_tick.saturating_sub(tick) < 10)
                        .unwrap_or(false)
                })
                .map(|(pos, resource_type)| (*pos, *resource_type))
                .collect();

            for (_, resource_type) in &newly_seen {
                for (skill_type, xp) in Self::get_skill_for_resource_discovery(resource_type) {
                    agent.skills.gain_experience(skill_type, xp);
                }
            }

            // Remember the food and water currently in view.
            //
            // Exploration reports a tile only the first time it is looked at,
            // so an agent driven by that alone would stop noticing a berry
            // patch the moment it had walked past it once. Sight is not a
            // one-off: whatever is in range is seen again every tick, which is
            // what keeps foraging memory current as patches are emptied and
            // regrow. Foraging reads spatial memory rather than the
            // exploration record, so without this an agent would have a patch
            // catalogued and still starve walking past it.
            let sight = vision_range as i32;
            let in_view: Vec<(crate::world::Position, SpatialMemoryType)> = world
                .resources
                .iter()
                .filter(|resource| resource.amount > 0)
                .filter(|resource| {
                    let dx = resource.position.x - agent_pos.x;
                    let dy = resource.position.y - agent_pos.y;
                    dx * dx + dy * dy <= sight * sight
                })
                .filter_map(|resource| {
                    let memory_type = if resource.resource_type.is_edible() {
                        SpatialMemoryType::Food
                    } else if resource.resource_type == crate::world::ResourceType::Water {
                        SpatialMemoryType::Water
                    } else {
                        return None;
                    };
                    Some((resource.position, memory_type))
                })
                .collect();

            for (pos, memory_type) in in_view {
                agent
                    .memory
                    .remember_location(memory_type, (pos.x, pos.y, 0));
            }

            // Learn skills from discovered buildings
            for (pos, building_type) in &agent.exploration_knowledge.known_buildings {
                // Only give XP for recently discovered buildings (within last 10 ticks)
                if let Some(&discover_tick) = agent.exploration_knowledge.building_discovery_ticks.get(pos) {
                    if current_tick.saturating_sub(discover_tick) < 10 {
                        let skill_xp = Self::get_skill_for_building_discovery(building_type);
                        for (skill_type, xp) in skill_xp {
                            agent.skills.gain_experience(skill_type, xp);
                        }
                    }
                }
            }
        }
    }

    /// Get skill XP gains for discovering a resource type
    fn get_skill_for_resource_discovery(resource_type: &crate::world::ResourceType) -> Vec<(super::SkillType, u32)> {
        use crate::world::ResourceType;
        use super::SkillType;

        match resource_type {
            // Mining resources teach Mining skill
            ResourceType::Stone | ResourceType::Iron | ResourceType::Coal
            | ResourceType::Clay | ResourceType::Sand => {
                vec![(SkillType::Mining, 5)]
            }
            // Wood resources teach Woodcutting
            ResourceType::Wood => vec![(SkillType::Woodcutting, 5)],
            // Agricultural resources teach Farming/Herbalism
            ResourceType::Grain | ResourceType::Flax | ResourceType::Cotton => {
                vec![(SkillType::Farming, 5)]
            }
            ResourceType::Herbs => vec![(SkillType::Herbalism, 5)],
            // Animal resources teach Hunting
            ResourceType::Meat | ResourceType::Hides => vec![(SkillType::Hunting, 5)],
            // Fish teaches Fishing
            ResourceType::Fish => vec![(SkillType::Fishing, 5)],
            // Food and foraging
            ResourceType::Food => vec![(SkillType::Herbalism, 3)],
            // Other resources
            _ => vec![(SkillType::Navigation, 2)],
        }
    }

    /// Get skill XP gains for discovering a building type
    fn get_skill_for_building_discovery(building_type: &crate::world::BuildingType) -> Vec<(super::SkillType, u32)> {
        use crate::world::BuildingType;
        use super::SkillType;

        match building_type {
            // Production buildings teach relevant crafting skills
            BuildingType::Forge | BuildingType::Smithy => {
                vec![(SkillType::Smelting, 10), (SkillType::Metalworking, 5)]
            }
            BuildingType::Workshop => vec![(SkillType::Crafting, 10)],
            BuildingType::Farm => vec![(SkillType::Farming, 10)],
            BuildingType::Bakery => vec![(SkillType::Cooking, 10)],
            BuildingType::Butchery => vec![(SkillType::Cooking, 5), (SkillType::Hunting, 5)],
            BuildingType::WeaverHut | BuildingType::TailorShop => {
                vec![(SkillType::Crafting, 10)]
            }
            BuildingType::Tannery => vec![(SkillType::Leatherworking, 10)],
            BuildingType::PotteryKiln | BuildingType::Brickyard => {
                vec![(SkillType::Masonry, 10)]
            }
            BuildingType::Mill | BuildingType::Brewery | BuildingType::Dairy => {
                vec![(SkillType::Cooking, 8)]
            }
            // Specialized craft buildings
            BuildingType::Glassworks | BuildingType::Dyeworks | BuildingType::Ropewalk | BuildingType::PaperMill => {
                vec![(SkillType::Crafting, 8)]
            }
            BuildingType::CobblerShop => {
                vec![(SkillType::Crafting, 5), (SkillType::Leatherworking, 5)]
            }
            BuildingType::Scriptorium => {
                vec![(SkillType::Crafting, 5), (SkillType::Social, 5)]
            }
            BuildingType::BarberShop => vec![(SkillType::Social, 8)],
            // Animal husbandry
            BuildingType::AnimalPen => {
                vec![(SkillType::Farming, 5), (SkillType::Hunting, 5)]
            }
            // Housing teaches Construction
            BuildingType::SmallHouse | BuildingType::MediumHouse | BuildingType::LargeHouse
            | BuildingType::Longhouse | BuildingType::UpgradedLonghouse | BuildingType::Manor => {
                vec![(SkillType::Construction, 10)]
            }
            // Civic buildings
            BuildingType::TownCenter => vec![(SkillType::Social, 10), (SkillType::Construction, 5)],
            BuildingType::TownStorage => vec![(SkillType::Navigation, 5), (SkillType::Construction, 5)],
            BuildingType::GuardPost => vec![(SkillType::MeleeCombat, 10)],
            // Storage buildings
            BuildingType::Storehouse => vec![(SkillType::Navigation, 8)],
            // Religious buildings teach social skills
            BuildingType::Shrine => vec![(SkillType::Social, 8)],
            BuildingType::Temple => vec![(SkillType::Social, 10)],
            // Medical building teaches herbalism and cooking (medicine preparation)
            BuildingType::MedicalBuilding => {
                vec![(SkillType::Herbalism, 10), (SkillType::Cooking, 5)]
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

        // Knowledge sharing between nearby agents (gossip about discoveries)
        // Agents share knowledge about buildings, resources, and terrain when in proximity
        use rand::seq::SliceRandom;
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for i in 0..self.agents.len() {
            let (agent_i_id, pos_i, alive_i) = agent_positions[i];
            if !alive_i {
                continue;
            }

            for j in (i + 1)..self.agents.len() {
                let (agent_j_id, pos_j, alive_j) = agent_positions[j];
                if !alive_j {
                    continue;
                }

                // Check if within sharing range
                let dx = (pos_i.0 - pos_j.0) as f32;
                let dy = (pos_i.1 - pos_j.1) as f32;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq <= EXPLORATION_SHARE_RANGE_SQ {
                    let current_tick = self.current_tick;

                    // Get actual agent UUIDs for relationship lookups
                    let uuid_j = self.agents[j].id;

                    // Calculate sharing probability based on relationship
                    let relationship_i_to_j = self.agents[i].relationships
                        .get_relationship(&uuid_j)
                        .map(|r| r.bond_strength)
                        .unwrap_or(0.0);
                    let share_probability = 0.3 + (relationship_i_to_j * 0.5).max(0.0); // 30-80% based on relationship

                    // Share buildings (prioritize recent discoveries)
                    if rng.gen_bool(share_probability as f64) {
                        // Agent i shares with agent j - prioritize recent discoveries
                        let buildings_i: Vec<_> = self.agents[i].exploration_knowledge
                            .known_buildings.iter()
                            .map(|(p, t)| (*p, *t))
                            .collect();

                        if !buildings_i.is_empty() {
                            // Share up to 3 buildings, prioritizing ones j doesn't know
                            let mut shared = 0;
                            for (pos, building_type) in buildings_i.choose_multiple(&mut rng, 5) {
                                if !self.agents[j].exploration_knowledge.known_buildings.contains_key(pos) {
                                    self.agents[j].exploration_knowledge
                                        .discover_building(*pos, *building_type, current_tick);
                                    shared += 1;
                                    if shared >= 3 { break; }
                                }
                            }

                            // Sharing satisfies social drive for agent i
                            if shared > 0 {
                                if let Some(drive) = self.agents[i].drives.get_mut(DriveType::Social) {
                                    drive.partial_satisfy(0.02 * shared as f32);
                                }
                            }
                        }
                    }

                    // Agent j shares with agent i
                    if rng.gen_bool(share_probability as f64) {
                        let buildings_j: Vec<_> = self.agents[j].exploration_knowledge
                            .known_buildings.iter()
                            .map(|(p, t)| (*p, *t))
                            .collect();

                        if !buildings_j.is_empty() {
                            let mut shared = 0;
                            for (pos, building_type) in buildings_j.choose_multiple(&mut rng, 5) {
                                if !self.agents[i].exploration_knowledge.known_buildings.contains_key(pos) {
                                    self.agents[i].exploration_knowledge
                                        .discover_building(*pos, *building_type, current_tick);
                                    shared += 1;
                                    if shared >= 3 { break; }
                                }
                            }

                            if shared > 0 {
                                if let Some(drive) = self.agents[j].drives.get_mut(DriveType::Social) {
                                    drive.partial_satisfy(0.02 * shared as f32);
                                }
                            }
                        }
                    }

                    // Share resources (important for survival)
                    if rng.gen_bool((share_probability * 0.8) as f64) {
                        // Agent i shares resource knowledge with agent j
                        let resources_i: Vec<_> = self.agents[i].exploration_knowledge
                            .known_resources.iter()
                            .map(|(p, t)| (*p, *t))
                            .collect();

                        if !resources_i.is_empty() {
                            let mut shared = 0;
                            for (pos, resource_type) in resources_i.choose_multiple(&mut rng, 5) {
                                if !self.agents[j].exploration_knowledge.known_resources.contains_key(pos) {
                                    self.agents[j].exploration_knowledge
                                        .discover_resource(*pos, *resource_type, current_tick);
                                    shared += 1;
                                    if shared >= 3 { break; }
                                }
                            }
                        }

                        // Agent j shares with agent i
                        let resources_j: Vec<_> = self.agents[j].exploration_knowledge
                            .known_resources.iter()
                            .map(|(p, t)| (*p, *t))
                            .collect();

                        if !resources_j.is_empty() {
                            for (pos, resource_type) in resources_j.choose_multiple(&mut rng, 5) {
                                if !self.agents[i].exploration_knowledge.known_resources.contains_key(pos) {
                                    self.agents[i].exploration_knowledge
                                        .discover_resource(*pos, *resource_type, current_tick);
                                }
                            }
                        }
                    }

                    // Strengthen relationship through gossip interaction
                    let uuid_i = self.agents[i].id;
                    if let Some(rel) = self.agents[i].relationships.get_relationship_mut(&uuid_j) {
                        rel.bond_strength = (rel.bond_strength + 0.001).min(1.0);
                        rel.total_interactions += 1;
                    }
                    if let Some(rel) = self.agents[j].relationships.get_relationship_mut(&uuid_i) {
                        rel.bond_strength = (rel.bond_strength + 0.001).min(1.0);
                        rel.total_interactions += 1;
                    }
                }
            }
        }
    }

    /// Work out which agents each agent can currently see.
    ///
    /// Sight range is the same one that finds berries and firewood, so a blind
    /// agent sees nobody and learns nothing by watching - it has to be told.
    pub fn update_who_can_see_whom(&mut self) {
        let seen: Vec<(uuid::Uuid, (i32, i32, i32))> = self
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.id, agent.state.position))
            .collect();

        for agent in &mut self.agents {
            if !agent.state.is_alive {
                agent.senses.vision.visible_agents.clear();
                continue;
            }

            let range = agent.sight_range() as i32;
            if range == 0 {
                agent.senses.vision.visible_agents.clear();
                continue;
            }

            let position = agent.state.position;

            let visible: Vec<uuid::Uuid> = seen
                .iter()
                .filter(|(id, _)| *id != agent.id)
                .filter(|(_, other)| {
                    (other.0 - position.0).abs() <= range
                        && (other.1 - position.1).abs() <= range
                })
                .map(|(id, _)| *id)
                .collect();

            agent.senses.vision.update_visible_agents(visible);
        }
    }

    /// Process observational learning between agents
    ///
    /// This handles:
    /// - Broadcasting actions to nearby observers
    /// - Automatic adoption of ready behaviors
    /// - Skill learning from adopted behaviors
    /// - Drive satisfaction and emotional responses for learners and teachers
    pub fn process_observational_learning(&mut self) {
        use super::observation_processing::auto_adopt_ready_behaviors;
        use crate::core::DriveType;
        use crate::agents::emotions::EmotionSource;

        // Collect adopted behaviors with agent indices
        let mut adoptions: Vec<(usize, uuid::Uuid, super::ActionType)> = Vec::new();

        // Process auto-adoption for each agent
        for i in 0..self.agents.len() {
            let adopted = auto_adopt_ready_behaviors(&mut self.agents[i]);

            if !adopted.is_empty() {
                for (teacher_id, action_type) in adopted {
                    adoptions.push((i, teacher_id, action_type));

                    // Satisfy learner's curiosity drive - learning is discovery!
                    if let Some(drive) = self.agents[i].drives.get_mut(DriveType::Curiosity) {
                        drive.partial_satisfy(0.15); // Learning satisfies curiosity
                    }

                    // Learner experiences positive emotions from successful learning
                    self.agents[i].emotions.add_happiness(
                        EmotionSource::Event("learning_success".to_string()),
                        0.1,
                    );
                }
            }
        }

        // Process teacher satisfaction (teachers feel good when others learn from them)
        for (learner_idx, teacher_id, action_type) in adoptions {
            // Find the teacher in the population
            if let Some(teacher_idx) = self.agents.iter().position(|a| a.id == teacher_id) {
                // Teacher gets social drive satisfaction from teaching
                if let Some(drive) = self.agents[teacher_idx].drives.get_mut(DriveType::Social) {
                    drive.partial_satisfy(0.1); // Teaching is social interaction
                }

                // Teacher experiences positive emotions from being a role model
                let learner_id = self.agents[learner_idx].id;
                self.agents[teacher_idx].emotions.add_happiness(
                    EmotionSource::Agent(learner_id),
                    0.08,
                );

                // Strengthen relationship between teacher and learner
                use crate::agents::emotions::{Relationship, RelationshipType};

                // Learner develops respect for teacher
                if self.agents[learner_idx].relationships.get_relationship(&teacher_id).is_none() {
                    self.agents[learner_idx].relationships.add_relationship(
                        Relationship::new(teacher_id, RelationshipType::Acquaintance)
                    );
                }
                // Strengthen bond
                if let Some(rel) = self.agents[learner_idx].relationships.get_relationship_mut(&teacher_id) {
                    rel.bond_strength = (rel.bond_strength + 0.05).min(1.0);
                }

                // Teacher recognizes learner
                if self.agents[teacher_idx].relationships.get_relationship(&learner_id).is_none() {
                    self.agents[teacher_idx].relationships.add_relationship(
                        Relationship::new(learner_id, RelationshipType::Acquaintance)
                    );
                }
                // Strengthen bond
                if let Some(rel) = self.agents[teacher_idx].relationships.get_relationship_mut(&learner_id) {
                    rel.bond_strength = (rel.bond_strength + 0.03).min(1.0);
                }

                // Log the learning event for debugging/analytics
                let _ = action_type; // Action type available for logging if needed
            }
        }
    }

    /// Process passive trait effects that depend on proximity or relationships
    /// This handles:
    /// - Romantic: happiness from partner proximity
    /// - Mediator: reduces nearby negative emotions
    /// - Intolerant: affection penalty with strangers
    /// - Insecure: anxiety when partner socializes with others
    /// - Copycat: happiness from mimicking nearby agents
    pub fn process_trait_proximity_effects(&mut self) {
        use crate::core::traits::Trait;
        use crate::agents::emotions::EmotionSource;
        use crate::agents::emotions::RelationshipType;

        const PROXIMITY_RANGE_SQ: f32 = 100.0; // 10 tiles squared

        // Collect agent positions and traits first to avoid borrow issues
        let agent_data: Vec<_> = self.agents.iter()
            .enumerate()
            .filter(|(_, a)| a.state.is_alive)
            .map(|(i, a)| (i, a.id, a.state.position, a.traits.clone()))
            .collect();

        // Process Romantic trait - happiness from partner proximity
        for (i, agent_id, pos_i, traits_i) in &agent_data {
            if traits_i.has(Trait::Romantic) {
                // Check if any romantic partner is nearby
                for (j, other_id, pos_j, _) in &agent_data {
                    if i == j { continue; }

                    // Check if this is a romantic partner
                    let is_partner = self.agents[*i].relationships
                        .get_relationship(other_id)
                        .map(|r| r.relationship_type == RelationshipType::Partner)
                        .unwrap_or(false);

                    if is_partner {
                        let dx = (pos_i.0 - pos_j.0) as f32;
                        let dy = (pos_i.1 - pos_j.1) as f32;
                        let dist_sq = dx * dx + dy * dy;

                        if dist_sq <= PROXIMITY_RANGE_SQ {
                            // Partner is nearby - gain happiness
                            self.agents[*i].emotions.add_happiness(
                                EmotionSource::Agent(*other_id),
                                0.02  // Small but constant happiness from partner proximity
                            );
                        }
                    }
                }
            }
        }

        // Process Mediator trait - reduces nearby negative emotions
        for (i, agent_id, pos_i, traits_i) in &agent_data {
            if traits_i.has(Trait::Mediator) {
                // Find nearby agents and reduce their negative emotions
                for (j, _, pos_j, _) in &agent_data {
                    if i == j { continue; }

                    let dx = (pos_i.0 - pos_j.0) as f32;
                    let dy = (pos_i.1 - pos_j.1) as f32;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq <= PROXIMITY_RANGE_SQ {
                        // Mediator calms nearby agents - reduce anger slightly
                        let current_anger = self.agents[*j].emotions.anger;
                        if current_anger > 0.1 {
                            // Reduce anger by small amount
                            for (_, amount) in self.agents[*j].emotions.anger_sources.iter_mut() {
                                *amount = (*amount * 0.98).max(0.0); // 2% reduction per tick
                            }
                        }
                    }
                }
            }
        }

        // Process Intolerant trait - affection penalty with strangers
        for (i, agent_id, pos_i, traits_i) in &agent_data {
            if traits_i.has(Trait::Intolerant) {
                // Check nearby agents for strangers
                for (j, other_id, pos_j, _) in &agent_data {
                    if i == j { continue; }

                    let dx = (pos_i.0 - pos_j.0) as f32;
                    let dy = (pos_i.1 - pos_j.1) as f32;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq <= PROXIMITY_RANGE_SQ {
                        // Check if this is a stranger (no relationship or weak bond)
                        let is_stranger = self.agents[*i].relationships
                            .get_relationship(other_id)
                            .map(|r| r.bond_strength < 0.2)
                            .unwrap_or(true);

                        if is_stranger {
                            // Intolerant agents lose happiness around strangers
                            self.agents[*i].emotions.add_sadness(
                                EmotionSource::Agent(*other_id),
                                0.01
                            );
                        }
                    }
                }
            }
        }

        // Process Insecure trait - anxiety when partner socializes with others
        for (i, agent_id, pos_i, traits_i) in &agent_data {
            if traits_i.has(Trait::Insecure) {
                // Find partner
                let partner_id: Option<uuid::Uuid> = self.agents[*i].relationships
                    .get_all()
                    .values()
                    .find(|r| r.relationship_type == RelationshipType::Partner)
                    .map(|r| r.other_agent);

                if let Some(partner) = partner_id {
                    // Check if partner is socializing with someone else nearby
                    let partner_idx = agent_data.iter()
                        .find(|(_, id, _, _)| *id == partner)
                        .map(|(idx, _, _, _)| *idx);

                    if let Some(p_idx) = partner_idx {
                        let partner_pos = agent_data.iter()
                            .find(|(idx, _, _, _)| *idx == p_idx)
                            .map(|(_, _, pos, _)| pos);

                        if let Some(ppos) = partner_pos {
                            // Check if partner is near other agents (not us)
                            for (k, other_id, pos_k, _) in &agent_data {
                                if *k == p_idx || *k == *i { continue; }

                                let dx = (ppos.0 - pos_k.0) as f32;
                                let dy = (ppos.1 - pos_k.1) as f32;
                                let dist_sq = dx * dx + dy * dy;

                                if dist_sq <= 25.0 { // Very close proximity (5 tiles)
                                    // Partner is close to someone else - trigger insecurity
                                    self.agents[*i].emotions.add_sadness(
                                        EmotionSource::Event("partner_jealousy".to_string()),
                                        0.02
                                    );
                                    break; // Only trigger once per tick
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process Envious trait - sadness when others have better equipment
        for (i, _agent_id, pos_i, traits_i) in &agent_data {
            if traits_i.has(Trait::Envious) {
                let my_equipment_value = self.agents[*i].equipment.total_value();

                for (j, other_id, pos_j, _) in &agent_data {
                    if i == j { continue; }

                    let dx = (pos_i.0 - pos_j.0) as f32;
                    let dy = (pos_i.1 - pos_j.1) as f32;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq <= PROXIMITY_RANGE_SQ {
                        let other_equipment_value = self.agents[*j].equipment.total_value();

                        if other_equipment_value > my_equipment_value * 1.5 {
                            // Others have significantly better equipment - feel envious
                            self.agents[*i].emotions.add_sadness(
                                EmotionSource::Agent(*other_id),
                                0.02
                            );
                        }
                    }
                }
            }
        }

        // Process Greedy trait - happiness from having more inventory than others
        for (i, _agent_id, pos_i, traits_i) in &agent_data {
            if traits_i.has(Trait::Greedy) {
                let my_inventory_count = self.agents[*i].inventory.total_items();

                for (j, _other_id, pos_j, _) in &agent_data {
                    if i == j { continue; }

                    let dx = (pos_i.0 - pos_j.0) as f32;
                    let dy = (pos_i.1 - pos_j.1) as f32;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq <= PROXIMITY_RANGE_SQ {
                        let other_inventory_count = self.agents[*j].inventory.total_items();

                        if my_inventory_count > other_inventory_count * 2 {
                            // I have significantly more stuff - feel satisfied
                            self.agents[*i].emotions.add_happiness(
                                EmotionSource::Event("wealth_satisfaction".to_string()),
                                0.01
                            );
                            break; // Only need one comparison
                        }
                    }
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

/// Calculate social interaction range based on agent personality traits
///
/// Returns squared range (to avoid sqrt in distance calculations)
/// Base range is 5 tiles. Traits can modify this:
/// - Extrovert/Sociable: +3 tiles (8 total)
/// - Charismatic: +2 tiles
/// - Introvert/Introverted: -2 tiles (3 total)
/// - Mute: -1 tile
/// - Explorer: +1 tile (willing to travel to meet people)
///
/// Range is clamped between 2 and 10 tiles.
fn calculate_social_range_squared(traits: &[Trait]) -> f32 {
    let mut range: f32 = 5.0; // Base social range in tiles

    for trait_item in traits {
        match trait_item {
            // Traits that increase social range
            Trait::Extrovert | Trait::Sociable => range += 3.0,
            Trait::Charismatic => range += 2.0,
            Trait::Explorer => range += 1.0,
            Trait::Curious => range += 1.0,

            // Traits that decrease social range
            Trait::Introvert | Trait::Introverted => range -= 2.0,
            Trait::Mute => range -= 1.0,
            Trait::Anxious => range -= 1.0,
            Trait::Paranoid => range -= 2.0,

            _ => {}
        }
    }

    // Clamp between 2 and 10 tiles
    range = range.clamp(2.0, 10.0);

    // Return squared range for efficient distance comparison
    range * range
}

/// Get the base social range for a set of traits (in tiles, not squared)
pub fn get_social_range(traits: &[Trait]) -> f32 {
    calculate_social_range_squared(traits).sqrt()
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
