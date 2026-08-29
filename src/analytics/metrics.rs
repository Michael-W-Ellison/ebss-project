// src/analytics/metrics.rs
//! Time-series metrics tracking for simulation analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agents::Population;
use crate::core::{DriveType, EmotionType, Trait};

/// Complete snapshot of simulation state at a specific tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickSnapshot {
    pub tick: u32,
    pub population: PopulationSnapshot,
    pub drives: HashMap<DriveType, DriveSnapshot>,
    pub emotions: HashMap<EmotionType, EmotionSnapshot>,
    pub traits: HashMap<Trait, u32>, // Count of agents with each trait
    pub relationships: RelationshipSnapshot,
    pub goals: GoalSnapshot,
    pub curiosity: CuriositySnapshot,
}

/// Population metrics at a specific tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationSnapshot {
    pub total: usize,
    pub by_life_stage: HashMap<String, usize>, // Infant, Child, etc.
    pub births_this_tick: u32,
    pub deaths_this_tick: u32,
    pub abandonments_this_tick: u32,
    pub average_happiness: f32,
    pub average_age: f32,
}

/// Drive satisfaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSnapshot {
    pub average_value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub satisfied_count: usize, // Agents with value > 0.7
    pub critical_count: usize,  // Agents with value < 0.3
}

/// Emotion statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    pub average_value: f32,
    pub extreme_positive_count: usize, // Value > 0.7
    pub extreme_negative_count: usize, // Value < -0.7
    pub average_well_being: f32,
}

/// Relationship network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshot {
    pub total_relationships: usize,
    pub by_strength: HashMap<String, usize>, // CloseFriend, Friend, etc.
    pub average_trust: f32,
    pub average_affection: f32,
    pub family_bonds: usize,
    pub conflicts: usize, // Relationships with negative trust/affection
}

/// Goal completion statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSnapshot {
    pub total_active_goals: usize,
    pub average_goals_per_agent: f32,
    pub internal_goals: usize,
    pub external_goals: usize,
    pub average_progress: f32,
}

/// Curiosity and exploration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriositySnapshot {
    pub total_discoveries: usize,
    pub discoveries_by_type: HashMap<String, usize>,
    pub average_exploration_efficiency: f32,
    pub total_curiosity_driven_explorations: u32,
    pub average_curiosity_satisfaction: f32,
    pub total_tiles_explored: usize,
    pub exploration_percentage: f32,
    pub agents_with_high_curiosity: usize, // Curiosity drive > 0.6
}

/// Complete time-series metrics for simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationMetrics {
    pub snapshots: Vec<TickSnapshot>,
    pub sampling_interval: u32, // Take snapshot every N ticks
    pub max_snapshots: usize,   // Keep only last N snapshots
    pub world_size: (usize, usize), // World dimensions (width, height) for exploration calculation
}

impl SimulationMetrics {
    pub fn new(sampling_interval: u32, max_snapshots: usize) -> Self {
        Self {
            snapshots: Vec::new(),
            sampling_interval,
            max_snapshots,
            world_size: (100, 100), // Default, can be overridden
        }
    }



    /// Record a snapshot if it's time to sample
    pub fn record_if_time(&mut self, tick: u32, population: &Population) {
        if tick % self.sampling_interval == 0 {
            self.record_snapshot(tick, population);
        }
    }

    /// Record a snapshot of current simulation state
    pub fn record_snapshot(&mut self, tick: u32, population: &Population) {
        let snapshot = self.create_snapshot(tick, population);
        self.snapshots.push(snapshot);

        // Keep only recent snapshots
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.remove(0);
        }
    }

    /// Create a snapshot from current population state
    fn create_snapshot(&self, tick: u32, population: &Population) -> TickSnapshot {
        TickSnapshot {
            tick,
            population: self.snapshot_population(population),
            drives: self.snapshot_drives(population),
            emotions: self.snapshot_emotions(population),
            traits: self.snapshot_traits(population),
            relationships: self.snapshot_relationships(population),
            goals: self.snapshot_goals(population),
            curiosity: self.snapshot_curiosity(population),
        }
    }

    fn snapshot_population(&self, population: &Population) -> PopulationSnapshot {
        let total = population.agents.len();

        let mut by_life_stage = HashMap::new();
        let mut total_age = 0u32;

        for agent in &population.agents {
            let stage = format!("{:?}", agent.state.life_stage);
            *by_life_stage.entry(stage).or_insert(0) += 1;
            total_age += agent.state.age;
        }

        let average_age = if total > 0 {
            total_age as f32 / total as f32
        } else {
            0.0
        };

        PopulationSnapshot {
            total,
            by_life_stage,
            births_this_tick: population.stats.births_this_tick,
            deaths_this_tick: population.stats.deaths_this_tick,
            abandonments_this_tick: population.stats.abandonments_this_tick,
            average_happiness: population.stats.average_happiness,
            average_age,
        }
    }

    fn snapshot_drives(&self, population: &Population) -> HashMap<DriveType, DriveSnapshot> {
        let mut drive_map: HashMap<DriveType, Vec<f32>> = HashMap::new();

        // Collect all drive values
        for agent in &population.agents {
            for drive in &agent.drives.drives {
                drive_map
                    .entry(drive.drive_type.clone())
                    .or_insert_with(Vec::new)
                    .push(drive.value);
            }
        }

        // Calculate statistics for each drive
        drive_map
            .into_iter()
            .map(|(drive_type, values)| {
                let snapshot = DriveSnapshot {
                    average_value: values.iter().sum::<f32>() / values.len() as f32,
                    min_value: values.iter().cloned().fold(f32::INFINITY, f32::min),
                    max_value: values.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
                    satisfied_count: values.iter().filter(|&&v| v > 0.7).count(),
                    critical_count: values.iter().filter(|&&v| v < 0.3).count(),
                };
                (drive_type, snapshot)
            })
            .collect()
    }

    fn snapshot_emotions(&self, population: &Population) -> HashMap<EmotionType, EmotionSnapshot> {
        let mut emotion_map: HashMap<EmotionType, Vec<f32>> = HashMap::new();
        let mut well_being_sum = 0.0;

        for agent in &population.agents {
            well_being_sum += agent.emotions.well_being();

            // Collect negative emotion values (0.0 to 1.0 range)
            emotion_map
                .entry(crate::core::EmotionType::Anger)
                .or_insert_with(Vec::new)
                .push(agent.emotions.anger);
            emotion_map
                .entry(crate::core::EmotionType::Fear)
                .or_insert_with(Vec::new)
                .push(agent.emotions.fear);
            emotion_map
                .entry(crate::core::EmotionType::Sadness)
                .or_insert_with(Vec::new)
                .push(agent.emotions.sadness);

            // Collect happiness (calculated from negative emotions, -1.0 to 1.0 range)
            emotion_map
                .entry(crate::core::EmotionType::Happiness)
                .or_insert_with(Vec::new)
                .push(agent.emotions.happiness);

            // Collect curiosity from agent's emotion state
            emotion_map
                .entry(crate::core::EmotionType::Curiosity)
                .or_insert_with(Vec::new)
                .push(agent.emotions.curiosity);
        }

        let average_well_being = if population.agents.is_empty() {
            0.0
        } else {
            well_being_sum / population.agents.len() as f32
        };

        emotion_map
            .into_iter()
            .map(|(emotion_type, values)| {
                let snapshot = EmotionSnapshot {
                    average_value: values.iter().sum::<f32>() / values.len() as f32,
                    extreme_positive_count: values.iter().filter(|&&v| v > 0.7).count(),
                    extreme_negative_count: values.iter().filter(|&&v| v < -0.7).count(),
                    average_well_being,
                };
                (emotion_type, snapshot)
            })
            .collect()
    }

    fn snapshot_traits(&self, population: &Population) -> HashMap<Trait, u32> {
        let mut trait_counts: HashMap<Trait, u32> = HashMap::new();

        for agent in &population.agents {
            for trait_type in agent.traits.get_traits() {
                *trait_counts.entry(*trait_type).or_insert(0) += 1;
            }
        }

        trait_counts
    }

    fn snapshot_relationships(&self, population: &Population) -> RelationshipSnapshot {
        let mut total_relationships = 0;
        let mut by_strength: HashMap<String, usize> = HashMap::new();
        let mut trust_sum = 0.0;
        let mut affection_sum = 0.0;
        let mut family_bonds = 0;
        let mut conflicts = 0;

        for agent in &population.agents {
            for (_, relationship) in agent.relationships.get_all() {
                total_relationships += 1;

                // Categorize by bond strength
                let strength = if relationship.bond_strength >= 0.8 {
                    "VeryStrong"
                } else if relationship.bond_strength >= 0.6 {
                    "Strong"
                } else if relationship.bond_strength >= 0.3 {
                    "Moderate"
                } else if relationship.bond_strength >= 0.0 {
                    "Weak"
                } else if relationship.bond_strength >= -0.3 {
                    "Negative"
                } else {
                    "Hostile"
                };
                *by_strength.entry(strength.to_string()).or_insert(0) += 1;

                // Use bond_strength for both trust and affection (they're unified now)
                trust_sum += relationship.bond_strength;
                affection_sum += relationship.bond_strength;

                if relationship.is_family() {
                    family_bonds += 1;
                }

                if relationship.bond_strength < 0.0 {
                    conflicts += 1;
                }
            }
        }

        let avg_trust = if total_relationships > 0 {
            trust_sum / total_relationships as f32
        } else {
            0.0
        };

        let avg_affection = if total_relationships > 0 {
            affection_sum / total_relationships as f32
        } else {
            0.0
        };

        RelationshipSnapshot {
            total_relationships,
            by_strength,
            average_trust: avg_trust,
            average_affection: avg_affection,
            family_bonds,
            conflicts,
        }
    }

    fn snapshot_goals(&self, population: &Population) -> GoalSnapshot {
        let mut total_active_goals = 0;
        let mut internal_goals = 0;
        let mut external_goals = 0;
        let mut progress_sum = 0.0;

        for agent in &population.agents {
            for goal in &agent.goals.goals {
                if !goal.completed {
                    total_active_goals += 1;
                    progress_sum += goal.progress;

                    match goal.goal_type {
                        crate::core::GoalType::Internal => internal_goals += 1,
                        crate::core::GoalType::External => external_goals += 1,
                    }
                }
            }
        }

        let average_goals_per_agent = if population.agents.is_empty() {
            0.0
        } else {
            total_active_goals as f32 / population.agents.len() as f32
        };

        let average_progress = if total_active_goals > 0 {
            progress_sum / total_active_goals as f32
        } else {
            0.0
        };

        GoalSnapshot {
            total_active_goals,
            average_goals_per_agent,
            internal_goals,
            external_goals,
            average_progress,
        }
    }

    fn snapshot_curiosity(&self, population: &Population) -> CuriositySnapshot {
        let mut total_discoveries = 0;
        let mut combined_discoveries: HashMap<String, usize> = HashMap::new();
        let mut total_efficiency = 0.0;
        let mut total_explorations = 0u32;
        let mut total_satisfaction = 0.0;
        let mut total_tiles = 0;
        let mut agents_with_high_curiosity = 0;

        for agent in &population.agents {
            // Access exploration knowledge directly (it's not an Option)
            let exploration = &agent.exploration_knowledge;
            total_discoveries += exploration.discoveries.len();
            total_explorations += exploration.curiosity_driven_explorations;
            total_satisfaction += exploration.total_curiosity_satisfaction;
            total_tiles += exploration.total_tiles_explored;

            // Combine discoveries by type
            for (type_name, count) in exploration.discoveries_by_type() {
                *combined_discoveries.entry(type_name).or_insert(0) += count;
            }

            // Add exploration efficiency
            total_efficiency += exploration.exploration_efficiency();

            // Check curiosity drive level
            if let Some(curiosity_drive) = agent.drives.get(crate::core::DriveType::Curiosity) {
                if curiosity_drive.value > 0.6 {
                    agents_with_high_curiosity += 1;
                }
            }
        }

        let agent_count = population.agents.len();
        let average_efficiency = if agent_count > 0 {
            total_efficiency / agent_count as f32
        } else {
            0.0
        };

        let average_satisfaction = if total_discoveries > 0 {
            total_satisfaction / total_discoveries as f32
        } else {
            0.0
        };

        // Calculate exploration percentage based on actual world size
        let total_world_tiles = (self.world_size.0 * self.world_size.1) as f32;
        let exploration_percentage = if total_tiles > 0 && total_world_tiles > 0.0 {
            (total_tiles as f32 / total_world_tiles) * 100.0
        } else {
            0.0
        };

        CuriositySnapshot {
            total_discoveries,
            discoveries_by_type: combined_discoveries,
            average_exploration_efficiency: average_efficiency,
            total_curiosity_driven_explorations: total_explorations,
            average_curiosity_satisfaction: average_satisfaction,
            total_tiles_explored: total_tiles,
            exploration_percentage,
            agents_with_high_curiosity,
        }
    }

    /// Get trend for population over time
    pub fn population_trend(&self) -> Vec<(u32, usize)> {
        self.snapshots
            .iter()
            .map(|s| (s.tick, s.population.total))
            .collect()
    }

    /// Get trend for average happiness over time
    pub fn happiness_trend(&self) -> Vec<(u32, f32)> {
        self.snapshots
            .iter()
            .map(|s| (s.tick, s.population.average_happiness))
            .collect()
    }








    /// Get summary statistics for the entire simulation
    pub fn summary(&self) -> SimulationSummary {
        if self.snapshots.is_empty() {
            return SimulationSummary::default();
        }

        let first = self.snapshots.first().unwrap();
        let last = self.snapshots.last().unwrap();

        SimulationSummary {
            total_ticks: last.tick - first.tick,
            initial_population: first.population.total,
            final_population: last.population.total,
            population_change: last.population.total as i32 - first.population.total as i32,
            peak_population: self.snapshots.iter().map(|s| s.population.total).max().unwrap_or(0),
            average_happiness: self
                .snapshots
                .iter()
                .map(|s| s.population.average_happiness)
                .sum::<f32>()
                / self.snapshots.len() as f32,
            total_relationships: last.relationships.total_relationships,
            most_common_trait: self.find_most_common_trait(),
        }
    }

    fn find_most_common_trait(&self) -> Option<Trait> {
        if let Some(last_snapshot) = self.snapshots.last() {
            last_snapshot
                .traits
                .iter()
                .max_by_key(|(_, &count)| count)
                .map(|(trait_item, _)| trait_item.clone())
        } else {
            None
        }
    }
}

/// Summary statistics for entire simulation run
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimulationSummary {
    pub total_ticks: u32,
    pub initial_population: usize,
    pub final_population: usize,
    pub population_change: i32,
    pub peak_population: usize,
    pub average_happiness: f32,
    pub total_relationships: usize,
    pub most_common_trait: Option<Trait>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{Agent, AgentConfig};

    #[test]
    fn test_metrics_creation() {
        let metrics = SimulationMetrics::new(10, 100);
        assert_eq!(metrics.sampling_interval, 10);
        assert_eq!(metrics.max_snapshots, 100);
        assert_eq!(metrics.snapshots.len(), 0);
    }

    #[test]
    fn test_record_snapshot() {
        let mut metrics = SimulationMetrics::new(1, 10);
        let mut population = Population::new();

        // Add a test agent
        let config = AgentConfig::default();
        population.spawn_agent(config);

        metrics.record_snapshot(0, &population);
        assert_eq!(metrics.snapshots.len(), 1);
        assert_eq!(metrics.snapshots[0].population.total, 1);
    }

    #[test]
    fn test_max_snapshots_limit() {
        let mut metrics = SimulationMetrics::new(1, 5);
        let population = Population::new();

        for tick in 0..10 {
            metrics.record_snapshot(tick, &population);
        }

        assert_eq!(metrics.snapshots.len(), 5); // Should keep only last 5
        assert_eq!(metrics.snapshots.first().unwrap().tick, 5);
        assert_eq!(metrics.snapshots.last().unwrap().tick, 9);
    }

    #[test]
    fn test_population_trend() {
        let mut metrics = SimulationMetrics::new(1, 100);
        let mut population = Population::new();

        let config = AgentConfig::default();
        population.spawn_agent(config);
        metrics.record_snapshot(0, &population);

        population.spawn_agent(AgentConfig::default());
        metrics.record_snapshot(10, &population);

        let trend = metrics.population_trend();
        assert_eq!(trend.len(), 2);
        assert_eq!(trend[0], (0, 1));
        assert_eq!(trend[1], (10, 2));
    }
}
