# Phase 3 Implementation Tasks - Social Systems

## Overview
Phase 3 focuses on adding reproduction, genetic inheritance, observational learning, social memory, and lifecycle management to create a living, evolving population.

**Target Duration**: 9-12 weeks
**Priority**: Social Systems for emergent behavior
**Success Criteria**: Agents birth, learn from parents, form social bonds, age, and die

---

## EPIC 1: Lifecycle Management (Week 1-2)
Enable agents to age, mature, and die

### Task 1.1: Extend Agent Structure
**File**: `src/agents/agent.rs`
**Changes**:
```rust
pub struct Agent {
    pub id: Uuid,
    pub state: AgentState,
    pub drives: DriveState,
    pub behavior_trees: Vec<BehaviorTree>,
    pub memory: Memory,
    
    // NEW FIELDS:
    pub age: u32,                          // Ticks since birth
    pub birth_tick: u32,                   // When born
    pub maturity_age: u32,                 // Age when can reproduce (~500-1000)
    pub max_lifespan: u32,                 // Max age before natural death (~5000-10000)
    pub generation: u32,                   // Which generation (0 = initial)
    pub parents: Option<(Uuid, Uuid)>,     // Parent IDs
    pub is_alive: bool,                    // Death flag
}
```

**Tests Needed**:
- Agent creation with age tracking
- Age increment
- Maturity checking
- Lifespan limits

### Task 1.2: Implement Aging Mechanics
**File**: `src/agents/agent.rs`
**Add Methods**:
```rust
impl Agent {
    pub fn increment_age(&mut self) {
        self.age += 1;
        // Age increases some drives (fatigue, loneliness)
    }
    
    pub fn is_mature(&self) -> bool {
        self.age >= self.maturity_age && self.is_alive
    }
    
    pub fn is_elderly(&self) -> bool {
        self.age >= self.max_lifespan * 80 / 100  // Last 20% of life
    }
    
    pub fn attempt_death(&mut self) -> bool {
        // Check natural death conditions:
        // 1. Age-based death chance (increases with age)
        // 2. Starvation (hunger > 0.9 for too long)
        // 3. Health depletion (health <= 0)
        
        if self.age >= self.max_lifespan {
            self.is_alive = false;
            return true;
        }
        
        let death_chance = (self.age as f32 / self.max_lifespan as f32).powi(3);
        if rand::random::<f32>() < death_chance {
            self.is_alive = false;
            return true;
        }
        
        false
    }
}
```

**Tests Needed**:
- Age increments correctly
- Maturity threshold triggers
- Death at natural lifespan
- Starvation death
- Health-based death

### Task 1.3: Extend Population for Lifecycle
**File**: `src/agents/population.rs`
**Changes**:
```rust
pub struct Population {
    pub agents: Vec<Agent>,
    pub dead_agents: Vec<Agent>,          // NEW: Track deceased
    pub birth_count: u32,                  // NEW: Total births
    pub death_count: u32,                  // NEW: Total deaths
    pub generation: u32,                   // NEW: Max generation
    pub max_population: Option<usize>,     // NEW: Population cap
}

impl Population {
    pub fn remove_dead_agents(&mut self) {
        self.agents.retain(|agent| {
            if agent.is_alive {
                true
            } else {
                self.dead_agents.push(agent.clone());
                self.death_count += 1;
                false
            }
        });
    }
    
    pub fn get_alive_agents(&self) -> Vec<&Agent> {
        self.agents.iter().filter(|a| a.is_alive).collect()
    }
    
    pub fn get_mature_agents(&mut self) -> Vec<&mut Agent> {
        self.agents.iter_mut().filter(|a| a.is_mature()).collect()
    }
}
```

**Tests Needed**:
- Dead agent removal
- Population tracking
- Mature agent filtering
- Generation tracking

---

## EPIC 2: Basic Reproduction (Week 2-3)
Enable agents to create offspring

### Task 2.1: Implement Reproduction Mechanics
**File**: `src/agents/agent.rs`
**Add Methods**:
```rust
impl Agent {
    pub fn can_reproduce(&self) -> bool {
        self.is_alive && self.is_mature()
    }
    
    /// Create offspring from two parents
    pub fn create_offspring(
        parent1: &Agent,
        parent2: &Agent,
        birth_tick: u32,
        mutation_rate: f32,  // 0.05 = 5% mutation
    ) -> Self {
        let mut offspring = Agent::new(AgentConfig::default());
        
        // Inherit from both parents
        offspring.age = 0;
        offspring.birth_tick = birth_tick;
        offspring.generation = (parent1.generation.max(parent2.generation)) + 1;
        offspring.parents = Some((parent1.id, parent2.id));
        offspring.is_alive = true;
        
        // Inherit maturity age (with small variation)
        let avg_maturity = (parent1.maturity_age as f32 + parent2.maturity_age as f32) / 2.0;
        let mutation = (rand::random::<f32>() - 0.5) * 100.0 * mutation_rate;
        offspring.maturity_age = (avg_maturity + mutation) as u32;
        
        // Inherit lifespan (with small variation)
        let avg_lifespan = (parent1.max_lifespan as f32 + parent2.max_lifespan as f32) / 2.0;
        let mutation = (rand::random::<f32>() - 0.5) * 200.0 * mutation_rate;
        offspring.max_lifespan = (avg_lifespan + mutation) as u32;
        
        offspring
    }
}
```

### Task 2.2: Population Reproduction Logic
**File**: `src/agents/population.rs`
**Add Methods**:
```rust
impl Population {
    pub fn reproduce(&mut self, reproduction_rate: f32, current_tick: u32) {
        let mature_agents: Vec<Uuid> = self.agents
            .iter()
            .filter(|a| a.can_reproduce())
            .map(|a| a.id)
            .collect();
        
        let pair_count = (mature_agents.len() / 2) as f32 * reproduction_rate;
        
        for _ in 0..(pair_count as usize) {
            if self.agents.len() >= self.max_population.unwrap_or(1000) {
                break;  // Population cap reached
            }
            
            // Random pairing
            let idx1 = rand::random::<usize>() % mature_agents.len();
            let idx2 = rand::random::<usize>() % mature_agents.len();
            
            if idx1 == idx2 {
                continue;  // Can't self-reproduce
            }
            
            let parent1 = self.agents.iter().find(|a| a.id == mature_agents[idx1]).unwrap();
            let parent2 = self.agents.iter().find(|a| a.id == mature_agents[idx2]).unwrap();
            
            let offspring = Agent::create_offspring(
                parent1,
                parent2,
                current_tick,
                0.05,  // 5% mutation
            );
            
            self.agents.push(offspring);
            self.birth_count += 1;
        }
    }
}
```

**Tests Needed**:
- Reproduction creates offspring
- Offspring inherit parent traits
- Generation tracking works
- Population cap enforced
- Reproduction rate controls

### Task 2.3: Integrate into Simulation Loop
**File**: `src/analytics/mod.rs`
**Modify run_for_ticks**:
```rust
impl Simulation {
    pub fn run_for_ticks(&mut self, ticks: u32) {
        for tick in 0..ticks {
            // 1. Age all agents
            for agent in &mut self.population.agents {
                agent.increment_age();
            }
            
            // 2. Check for deaths
            for agent in &mut self.population.agents {
                agent.attempt_death();
            }
            self.population.remove_dead_agents();
            
            // 3. Reproduction (if reproduction drive is high)
            self.population.reproduce(0.1, tick);  // 10% reproduction rate
            
            // 4. Update drives
            for agent in &mut self.population.agents {
                agent.drives.tick();
            }
            
            // 5. Placeholder for decision-making and actions
            // (Will be expanded in later tasks)
        }
    }
}
```

---

## EPIC 3: Genetic Inheritance (Week 3-5)
Offspring inherit parent behavior trees and drive weights

### Task 3.1: Enhance Drive Inheritance
**File**: `src/core/drives.rs`
**Add Methods**:
```rust
impl Drive {
    pub fn inherit(parent1: &Drive, parent2: &Drive, mutation_rate: f32) -> Self {
        let avg_weight = (parent1.weight + parent2.weight) / 2.0;
        let mutation = (rand::random::<f32>() - 0.5) * 2.0 * mutation_rate;
        let new_weight = (avg_weight + mutation).clamp(0.5, 2.0);
        
        Self {
            drive_type: parent1.drive_type,
            value: 0.0,
            threshold: parent1.threshold,  // Could also inherit this
            weight: new_weight,
        }
    }
}

impl DriveState {
    pub fn inherit(parent1: &DriveState, parent2: &DriveState, mutation_rate: f32) -> Self {
        Self {
            drives: DriveType::all()
                .iter()
                .map(|&drive_type| {
                    let p1_drive = parent1.get(drive_type).unwrap();
                    let p2_drive = parent2.get(drive_type).unwrap();
                    Drive::inherit(p1_drive, p2_drive, mutation_rate)
                })
                .collect(),
        }
    }
}
```

**Tests Needed**:
- Inherited drive weights are averages
- Mutation creates variation
- Weights stay in valid range
- Personality diversity maintained

### Task 3.2: Behavior Tree Inheritance
**File**: `src/core/behavior_tree.rs`
**Extend existing method**:
```rust
impl BehaviorTree {
    // Already exists: clone_with_pruning()
    // Now use it in agent reproduction:
    
    pub fn mutate(&mut self, mutation_rate: f32) {
        // Random mutation of weights
        self.root.mutate_recursive(mutation_rate);
    }
}

impl BehaviorNode {
    fn mutate_recursive(&mut self, mutation_rate: f32) {
        if rand::random::<f32>() < mutation_rate {
            let factor = (rand::random::<f32>() - 0.5) * 0.4 + 1.0; // 0.8 to 1.2
            self.weight *= factor;
            self.weight = self.weight.clamp(0.1, 10.0);
        }
        
        for child in &mut self.children {
            child.mutate_recursive(mutation_rate);
        }
    }
}
```

### Task 3.3: Update Agent Reproduction
**File**: `src/agents/agent.rs`
**Modify create_offspring**:
```rust
impl Agent {
    pub fn create_offspring(
        parent1: &Agent,
        parent2: &Agent,
        birth_tick: u32,
        mutation_rate: f32,
    ) -> Self {
        let mut offspring = Agent::new(AgentConfig::default());
        
        // ... (existing age/generation code) ...
        
        // Inherit drives with variation
        offspring.drives = DriveState::inherit(&parent1.drives, &parent2.drives, mutation_rate);
        
        // Inherit behavior trees with pruning and mutation
        offspring.behavior_trees = Vec::new();
        for parent_tree in &parent1.behavior_trees {
            let mut tree = parent_tree.clone_with_pruning(0.5);  // Prune weak branches
            tree.mutate(mutation_rate);
            offspring.behavior_trees.push(tree);
        }
        
        // Start with parent's proven behaviors instead of empty
        for parent_tree in &parent2.behavior_trees {
            if offspring.behavior_trees.len() < 3 {  // Limit tree count
                let mut tree = parent_tree.clone_with_pruning(0.5);
                tree.mutate(mutation_rate);
                offspring.behavior_trees.push(tree);
            }
        }
        
        offspring
    }
}
```

**Tests Needed**:
- Trees inherited from parents
- Pruning removes weak branches
- Mutations create variation
- Behavioral diversity increases over generations

---

## EPIC 4: Memory System Implementation (Week 5-7)
Flesh out memory to store knowledge

### Task 4.1: Implement Memory Structure
**File**: `src/core/memory.rs`
**Complete implementation**:
```rust
use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialMemory {
    pub position: (i32, i32, i32),
    pub resource_type: String,
    pub quantity: f32,
    pub last_seen: u32,  // Tick when last seen
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRelationship {
    pub agent_id: Uuid,
    pub relation_type: RelationType,  // Kin, ally, threat
    pub trust: f32,  // 0.0 to 1.0
    pub last_interaction: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RelationType {
    Parent,
    Sibling,
    Offspring,
    Mate,
    Ally,
    Neutral,
    Threat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingRecipe {
    pub name: String,
    pub inputs: Vec<(String, u32)>,    // (material, quantity)
    pub output: (String, u32),
    pub discovered_tick: u32,
    pub success_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub spatial_memory: HashMap<String, Vec<SpatialMemory>>,  // By resource type
    pub agent_memory: HashMap<Uuid, AgentRelationship>,
    pub recipe_memory: Vec<CraftingRecipe>,
    pub last_update: u32,
    pub decay_rate: f32,  // How fast memories fade
}

impl Memory {
    pub fn new() -> Self {
        Self {
            spatial_memory: HashMap::new(),
            agent_memory: HashMap::new(),
            recipe_memory: Vec::new(),
            last_update: 0,
            decay_rate: 0.99,  // 1% decay per tick
        }
    }
    
    pub fn remember_location(
        &mut self,
        position: (i32, i32, i32),
        resource_type: String,
        quantity: f32,
        current_tick: u32,
    ) {
        self.spatial_memory
            .entry(resource_type)
            .or_insert_with(Vec::new)
            .push(SpatialMemory {
                position,
                resource_type: resource_type.clone(),
                quantity,
                last_seen: current_tick,
            });
    }
    
    pub fn remember_agent(
        &mut self,
        agent_id: Uuid,
        relation_type: RelationType,
        current_tick: u32,
    ) {
        self.agent_memory.insert(
            agent_id,
            AgentRelationship {
                agent_id,
                relation_type,
                trust: 0.5,
                last_interaction: current_tick,
            },
        );
    }
    
    pub fn update_trust(
        &mut self,
        agent_id: Uuid,
        change: f32,  // Positive or negative
    ) {
        if let Some(rel) = self.agent_memory.get_mut(&agent_id) {
            rel.trust = (rel.trust + change).clamp(0.0, 1.0);
        }
    }
    
    pub fn get_nearby_resources(
        &self,
        position: (i32, i32, i32),
        range: i32,
        resource_type: &str,
    ) -> Vec<&SpatialMemory> {
        self.spatial_memory
            .get(resource_type)
            .unwrap_or(&Vec::new())
            .iter()
            .filter(|mem| {
                let dx = (mem.position.0 - position.0).abs();
                let dy = (mem.position.1 - position.1).abs();
                let dz = (mem.position.2 - position.2).abs();
                dx <= range && dy <= range && dz <= range
            })
            .collect()
    }
    
    pub fn decay(&mut self) {
        // Memories fade over time
        self.spatial_memory.values_mut().for_each(|memories| {
            memories.iter_mut().for_each(|mem| {
                mem.quantity *= self.decay_rate;
            });
            memories.retain(|mem| mem.quantity > 0.01);  // Remove forgotten memories
        });
    }
}
```

**Tests Needed**:
- Location memory storage
- Agent relationship tracking
- Trust updates
- Nearby resource queries
- Memory decay

### Task 4.2: Integrate Memory into Agent Actions
**File**: `src/agents/agent.rs`
**Add methods**:
```rust
impl Agent {
    pub fn record_experience(
        &mut self,
        action: &str,
        success: bool,
        current_tick: u32,
    ) {
        // Will be expanded when behavior trees are linked
        // For now, update memory with interaction results
    }
    
    pub fn recognize_agent(&mut self, other_id: Uuid, relation: RelationType) {
        self.memory.remember_agent(other_id, relation, 0);  // TODO: pass tick
    }
}
```

---

## EPIC 5: Observational Learning (Week 6-8)
Young agents learn from parents through observation

### Task 5.1: Young Agent Learning System
**File**: `src/agents/agent.rs`
**Add fields and methods**:
```rust
pub struct Agent {
    // ... existing fields ...
    pub observed_parent: Option<Uuid>,    // NEW: Parent being observed
    pub learning_progress: f32,            // NEW: 0.0 = no learning, 1.0 = independent
}

impl Agent {
    pub fn is_young(&self) -> bool {
        self.age < self.maturity_age / 2
    }
    
    pub fn set_observation_target(&mut self, parent_id: Uuid) {
        self.observed_parent = Some(parent_id);
    }
    
    pub fn stop_observation(&mut self) {
        self.observed_parent = None;
    }
}
```

### Task 5.2: Imitation Mechanics
**File**: `src/core/learning.rs`
**Implement learning system**:
```rust
pub struct LearningSystem;

impl LearningSystem {
    pub fn imitate_parent(
        young_agent: &mut Agent,
        parent: &Agent,
        learning_factor: f32,  // 0.0 to 1.0
    ) {
        // Young agents copy successful parent trees
        if learning_factor < 0.5 {
            return;  // Too young to learn effectively
        }
        
        // Copy high-weight parent trees to young agent
        for parent_tree in &parent.behavior_trees {
            if parent_tree.success_rate() > 0.6 {
                let copied_tree = parent_tree.clone();
                
                // Don't fully copy - mix with own learning
                if young_agent.behavior_trees.len() < parent.behavior_trees.len() {
                    young_agent.behavior_trees.push(copied_tree);
                }
            }
        }
    }
    
    pub fn update_learning_progress(
        agent: &mut Agent,
        ticks_alive: u32,
        maturity_age: u32,
    ) {
        if agent.is_young() {
            agent.learning_progress = ticks_alive as f32 / (maturity_age as f32 / 2.0);
            agent.learning_progress = agent.learning_progress.min(1.0);
        } else {
            agent.learning_progress = 1.0;  // Fully independent
        }
    }
}
```

### Task 5.3: Integrate into Simulation Loop
**File**: `src/analytics/mod.rs`
**Add per-tick step**:
```rust
impl Simulation {
    pub fn run_for_ticks(&mut self, ticks: u32) {
        for tick in 0..ticks {
            // ... existing code ...
            
            // NEW: Handle observational learning
            for agent in &mut self.population.agents {
                if agent.is_young() {
                    if let Some(parent_id) = agent.observed_parent {
                        // Find parent and learn from them
                        if let Some(parent) = self.population.agents
                            .iter()
                            .find(|a| a.id == parent_id && a.is_alive)
                        {
                            let parent_clone = parent.clone();
                            LearningSystem::imitate_parent(
                                agent,
                                &parent_clone,
                                agent.learning_progress,
                            );
                        }
                    }
                    
                    LearningSystem::update_learning_progress(
                        agent,
                        agent.age,
                        agent.maturity_age,
                    );
                }
            }
        }
    }
}
```

**Tests Needed**:
- Young agents imitate parents
- Learning progress increases with age
- Parent's successful trees are copied
- Independence is achieved at maturity

---

## EPIC 6: Social Memory & Relationships (Week 8-10)
Agents maintain kinship and social bonds

### Task 6.1: Social Network Management
**File**: `src/agents/population.rs`
**Add social tracking**:
```rust
pub struct Population {
    // ... existing fields ...
    pub kinship_matrix: HashMap<Uuid, Vec<(Uuid, RelationType)>>,  // NEW
    pub social_events: Vec<SocialEvent>,                           // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialEvent {
    pub agent1_id: Uuid,
    pub agent2_id: Uuid,
    pub event_type: String,  // "cooperated", "fought", "mated", etc.
    pub tick: u32,
    pub success: bool,
}

impl Population {
    pub fn record_relationship(
        &mut self,
        parent_id: Uuid,
        child_id: Uuid,
        relation: RelationType,
    ) {
        self.kinship_matrix
            .entry(parent_id)
            .or_insert_with(Vec::new)
            .push((child_id, relation));
            
        self.kinship_matrix
            .entry(child_id)
            .or_insert_with(Vec::new)
            .push((parent_id, relation));
    }
    
    pub fn get_relatives(&self, agent_id: Uuid) -> Vec<(Uuid, RelationType)> {
        self.kinship_matrix
            .get(&agent_id)
            .cloned()
            .unwrap_or_default()
    }
    
    pub fn record_social_event(
        &mut self,
        agent1_id: Uuid,
        agent2_id: Uuid,
        event_type: String,
        success: bool,
        tick: u32,
    ) {
        self.social_events.push(SocialEvent {
            agent1_id,
            agent2_id,
            event_type,
            tick,
            success,
        });
    }
}
```

### Task 6.2: Kinship Recognition
**File**: `src/agents/agent.rs`
**Add kinship methods**:
```rust
impl Agent {
    pub fn recognize_kin(&mut self, kin_id: Uuid, relation: RelationType) {
        self.memory.remember_agent(kin_id, relation, 0);  // Mark as kin
        
        // Kin are automatically trusted higher
        if let Some(rel) = self.memory.agent_memory.get_mut(&kin_id) {
            rel.trust = 0.8;  // High trust for kin
        }
    }
    
    pub fn is_kin_of(&self, other: &Agent) -> bool {
        self.memory.agent_memory.get(&other.id)
            .map(|rel| matches!(
                rel.relation_type,
                RelationType::Parent | RelationType::Sibling | RelationType::Offspring
            ))
            .unwrap_or(false)
    }
}
```

### Task 6.3: Group Formation
**File**: `src/agents/population.rs`
**Add group management**:
```rust
pub struct SocialGroup {
    pub id: Uuid,
    pub members: Vec<Uuid>,
    pub parent_id: Option<Uuid>,  // Founder
    pub trust_level: f32,
    pub cooperation_count: u32,
}

impl Population {
    pub fn form_group(
        &mut self,
        founder_id: Uuid,
        members: Vec<Uuid>,
    ) -> Uuid {
        let group_id = Uuid::new_v4();
        // TODO: Implement group storage
        group_id
    }
    
    pub fn encourage_cooperation(
        &mut self,
        agent1_id: Uuid,
        agent2_id: Uuid,
        success: bool,
        tick: u32,
    ) {
        self.record_social_event(
            agent1_id,
            agent2_id,
            "cooperated".to_string(),
            success,
            tick,
        );
        
        if success {
            // Increase trust between agents
            if let Some(agent1) = self.agents.iter_mut().find(|a| a.id == agent1_id) {
                agent1.memory.update_trust(agent2_id, 0.1);
            }
            if let Some(agent2) = self.agents.iter_mut().find(|a| a.id == agent2_id) {
                agent2.memory.update_trust(agent1_id, 0.1);
            }
        }
    }
}
```

**Tests Needed**:
- Kinship recognized and stored
- Social events recorded
- Groups formed correctly
- Cooperation affects trust
- Family members seek each other

---

## EPIC 7: Population Dynamics & Emergence Analysis (Week 10-12)
Track population statistics and detect emergent behaviors

### Task 7.1: Population Statistics
**File**: `src/analytics/mod.rs`
**Add tracking**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationStats {
    pub tick: u32,
    pub population_size: usize,
    pub dead_agents: usize,
    pub births: u32,
    pub deaths: u32,
    pub avg_age: f32,
    pub avg_generation: f32,
    pub genetic_diversity: f32,
    pub behavioral_diversity: f32,
    pub social_connections: usize,
}

impl Simulation {
    pub fn collect_stats(&self) -> PopulationStats {
        let agents = &self.population.agents;
        let alive_count = agents.iter().filter(|a| a.is_alive).count();
        let avg_age = agents.iter()
            .filter(|a| a.is_alive)
            .map(|a| a.age as f32)
            .sum::<f32>() / alive_count.max(1) as f32;
        
        let avg_generation = agents.iter()
            .filter(|a| a.is_alive)
            .map(|a| a.generation as f32)
            .sum::<f32>() / alive_count.max(1) as f32;
        
        PopulationStats {
            tick: 0,  // TODO: pass current tick
            population_size: alive_count,
            dead_agents: self.population.dead_agents.len(),
            births: self.population.birth_count,
            deaths: self.population.death_count,
            avg_age,
            avg_generation,
            genetic_diversity: self.calculate_genetic_diversity(),
            behavioral_diversity: self.calculate_behavioral_diversity(),
            social_connections: self.population.kinship_matrix.len(),
        }
    }
    
    fn calculate_genetic_diversity(&self) -> f32 {
        // Compare drive weights across population
        let agents = self.population.agents.iter().filter(|a| a.is_alive).collect::<Vec<_>>();
        if agents.is_empty() { return 0.0; }
        
        // Simple metric: variance in drive weights
        let mut total_variance = 0.0;
        for drive_idx in 0..13 {
            let weights: Vec<f32> = agents.iter()
                .map(|a| a.drives.drives[drive_idx].weight)
                .collect();
            
            let mean = weights.iter().sum::<f32>() / weights.len() as f32;
            let variance = weights.iter()
                .map(|w| (w - mean).powi(2))
                .sum::<f32>() / weights.len() as f32;
            
            total_variance += variance;
        }
        
        (total_variance / 13.0).min(1.0)  // Normalize to 0-1
    }
    
    fn calculate_behavioral_diversity(&self) -> f32 {
        // Compare behavior tree complexity across population
        let agents = self.population.agents.iter().filter(|a| a.is_alive).collect::<Vec<_>>();
        if agents.is_empty() { return 0.0; }
        
        let complexities: Vec<usize> = agents.iter()
            .map(|a| a.behavior_trees.len())
            .collect();
        
        let mean = complexities.iter().sum::<usize>() as f32 / complexities.len() as f32;
        let variance = complexities.iter()
            .map(|c| (*c as f32 - mean).powi(2))
            .sum::<f32>() / complexities.len() as f32;
        
        (variance.sqrt() / 10.0).min(1.0)  // Normalize
    }
}
```

### Task 7.2: Emergence Detection
**File**: `src/analytics/mod.rs`
**Add emergence analysis**:
```rust
pub struct EmergenceIndicators {
    pub unexpected_behavior_patterns: Vec<String>,
    pub novel_social_structures: Vec<String>,
    pub adaptive_strategies: Vec<String>,
}

impl Simulation {
    pub fn detect_emergence(&self) -> EmergenceIndicators {
        let mut indicators = EmergenceIndicators {
            unexpected_behavior_patterns: Vec::new(),
            novel_social_structures: Vec::new(),
            adaptive_strategies: Vec::new(),
        };
        
        // Check for unexpected cooperation despite no explicit programming
        let cooperation_events: usize = self.population.social_events.iter()
            .filter(|e| e.event_type == "cooperated" && e.success)
            .count();
        
        if cooperation_events > 0 {
            indicators.adaptive_strategies.push(
                format!("Spontaneous cooperation detected ({} events)", cooperation_events)
            );
        }
        
        // Check for family group formation
        let family_groups = self.detect_family_clustering();
        if !family_groups.is_empty() {
            indicators.novel_social_structures.push(
                format!("Family clustering detected: {} groups", family_groups.len())
            );
        }
        
        indicators
    }
    
    fn detect_family_clustering(&self) -> Vec<Vec<Uuid>> {
        // Group agents by kinship
        let mut groups: Vec<Vec<Uuid>> = Vec::new();
        
        for (agent_id, relatives) in &self.population.kinship_matrix {
            let mut found = false;
            for group in &mut groups {
                if group.contains(agent_id) {
                    found = true;
                    break;
                }
            }
            
            if !found {
                let mut group = vec![*agent_id];
                for (relative_id, _) in relatives {
                    group.push(*relative_id);
                }
                groups.push(group);
            }
        }
        
        groups.into_iter().filter(|g| g.len() > 1).collect()
    }
}
```

### Task 7.3: Logging & Visualization Prep
**File**: `src/analytics/mod.rs`
**Add output**:
```rust
impl Simulation {
    pub fn print_stats(&self) {
        let stats = self.collect_stats();
        println!(
            "Tick {}: Pop={}, Alive={}, Age={:.1}, Gen={:.1}, Div={:.2}",
            stats.tick,
            stats.population_size,
            stats.population_size - stats.dead_agents,
            stats.avg_age,
            stats.avg_generation,
            stats.genetic_diversity
        );
    }
    
    pub fn save_stats_to_file(&self, filename: &str) -> std::io::Result<()> {
        // TODO: Implement CSV/JSON output for analysis
        Ok(())
    }
}
```

**Tests Needed**:
- Statistics calculated correctly
- Emergence indicators detected
- Family groups identified
- Genetic diversity measured
- Behavioral diversity measured

---

## Task Dependencies Graph

```
1.1 Extend Agent
    ↓
1.2 Aging Mechanics
    ↓
1.3 Population Lifecycle
    ↓
2.1-2.3 Basic Reproduction ← depends on lifecycle
    ↓
3.1-3.2 Genetic Inheritance ← depends on reproduction
    ↓
4.1-4.2 Memory System (parallel)
    ↓
5.1-5.3 Observational Learning ← depends on memory + inheritance
    ↓
6.1-6.3 Social Memory & Relationships (parallel with 5)
    ↓
7.1-7.3 Analytics & Emergence (final, depends on all above)
```

---

## Testing Strategy

### Unit Tests (All tasks)
Each new function needs unit tests:
- Happy path (normal operation)
- Edge cases (empty collections, boundary values)
- Error conditions (if applicable)

### Integration Tests (Per EPIC)
- Lifecycle: Age incrementing, death, reproduction
- Inheritance: Offspring have correct traits
- Learning: Young agents learn from parents
- Social: Relationships tracked correctly
- Analytics: Statistics accurate

### Emergence Tests
- Run 10,000+ tick simulations
- Check for unexpected behaviors
- Verify population stability
- Measure diversity increases

---

## Success Criteria

✅ Phase 3 Complete When:
- [ ] Agents are born, mature, and die naturally
- [ ] Offspring inherit parent traits with variations
- [ ] Young agents learn from observing parents
- [ ] Agents maintain social memories and relationships
- [ ] Population remains stable over 10,000+ ticks
- [ ] Genetic diversity increases over generations
- [ ] Emergent cooperation without explicit programming
- [ ] Family groups form naturally
- [ ] Behavioral diversity increases over time
- [ ] All systems have 80%+ test coverage

---

## Performance Targets

- **Simulation Speed**: 10,000+ ticks/second for 100 agents
- **Memory**: < 500MB for 1000-agent simulation
- **Scalability**: Linear scaling to 10,000 agents

---

## Documentation Requirements

- [ ] Update README.md with Phase 3 features
- [ ] Document reproduction mechanics
- [ ] Document observational learning system
- [ ] Add lifecycle guide
- [ ] Add memory usage guide
- [ ] Example simulations showing emergent behaviors
