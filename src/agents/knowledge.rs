// src/agents/knowledge.rs
//! Personal knowledge system for individual agents.
//!
//! Each agent maintains their own knowledge about the world, learned through
//! personal observation or communication with other agents. Knowledge ages
//! over time and becomes less reliable.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::world::{Position, ResourceType};

/// Information about a known resource location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceKnowledge {
    pub position: Position,
    pub resource_type: ResourceType,
    pub estimated_amount: u32,
    pub learned_tick: u32,        // When this information was acquired
    pub last_verified_tick: u32,  // Last time agent personally verified this
    pub source: KnowledgeSource,   // How agent learned this
}

/// How the agent acquired this knowledge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KnowledgeSource {
    PersonalObservation,           // Saw it themselves (most reliable)
    DirectCommunication(uuid::Uuid), // Told by specific agent
    Overheard(uuid::Uuid),         // Overheard conversation (less reliable)
}

impl ResourceKnowledge {
    /// Calculate reliability of this knowledge based on age
    /// Returns value from 0.0 (unreliable) to 1.0 (very reliable)
    pub fn reliability(&self, current_tick: u32) -> f32 {
        let age = current_tick.saturating_sub(self.last_verified_tick);

        // Base reliability depends on source
        let base_reliability = match self.source {
            KnowledgeSource::PersonalObservation => 1.0,
            KnowledgeSource::DirectCommunication(_) => 0.8,
            KnowledgeSource::Overheard(_) => 0.6,
        };

        // Reliability decays over time
        // After 500 ticks, reliability drops by 50%
        // After 1000 ticks, reliability drops by 75%
        let age_factor = if age < 500 {
            1.0 - (age as f32 * 0.001) // Slow decay
        } else if age < 1000 {
            0.5 - ((age - 500) as f32 * 0.001) // Medium decay
        } else {
            0.25 * (1.0 - ((age - 1000) as f32 * 0.0005).min(1.0)) // Fast decay
        };

        base_reliability * age_factor.max(0.0)
    }

    /// Check if this knowledge is still trustworthy
    pub fn is_reliable(&self, current_tick: u32) -> bool {
        self.reliability(current_tick) > 0.3
    }
}

/// Personal knowledge base for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalKnowledge {
    /// Known resource locations indexed by position
    resources: BTreeMap<Position, ResourceKnowledge>,
    /// Current tick (for age calculations)
    current_tick: u32,
}

impl PersonalKnowledge {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            current_tick: 0,
        }
    }

    /// Update current tick
    pub fn tick(&mut self, current_tick: u32) {
        self.current_tick = current_tick;
    }

    /// Learn about a resource through personal observation
    pub fn observe_resource(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        amount: u32,
    ) {
        self.resources.insert(
            position,
            ResourceKnowledge {
                position,
                resource_type,
                estimated_amount: amount,
                learned_tick: self.current_tick,
                last_verified_tick: self.current_tick,
                source: KnowledgeSource::PersonalObservation,
            },
        );
    }

    /// Learn about a resource from another agent (direct communication)
    pub fn learn_from_agent(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        amount: u32,
        source_agent: uuid::Uuid,
    ) {
        // Only learn if we don't already know, or if new info is more recent
        let should_learn = if let Some(existing) = self.resources.get(&position) {
            // Update if this is newer information
            self.current_tick > existing.learned_tick
        } else {
            true
        };

        if should_learn {
            self.resources.insert(
                position,
                ResourceKnowledge {
                    position,
                    resource_type,
                    estimated_amount: amount,
                    learned_tick: self.current_tick,
                    last_verified_tick: self.current_tick,
                    source: KnowledgeSource::DirectCommunication(source_agent),
                },
            );
        }
    }

    /// Overhear information (less reliable than direct communication)
    pub fn overhear_information(
        &mut self,
        position: Position,
        resource_type: ResourceType,
        amount: u32,
        source_agent: uuid::Uuid,
    ) {
        // Only learn if we don't already have better information
        let should_learn = if let Some(existing) = self.resources.get(&position) {
            // Don't override personal observation or direct communication with overheard info
            matches!(existing.source, KnowledgeSource::Overheard(_))
        } else {
            true
        };

        if should_learn {
            self.resources.insert(
                position,
                ResourceKnowledge {
                    position,
                    resource_type,
                    estimated_amount: amount,
                    learned_tick: self.current_tick,
                    last_verified_tick: self.current_tick,
                    source: KnowledgeSource::Overheard(source_agent),
                },
            );
        }
    }


    /// Get all known resources of a specific type
    pub fn get_known_resources(&self, resource_type: ResourceType) -> Vec<&ResourceKnowledge> {
        self.resources
            .values()
            .filter(|k| {
                k.resource_type == resource_type
                    && k.estimated_amount > 0
                    && k.is_reliable(self.current_tick)
            })
            .collect()
    }

    /// Find closest known resource of a type
    pub fn find_closest_resource(
        &self,
        from: &Position,
        resource_type: ResourceType,
    ) -> Option<&ResourceKnowledge> {
        self.get_known_resources(resource_type)
            .into_iter()
            .min_by_key(|k| from.distance_to(&k.position))
    }

    /// Check if agent knows about a specific resource location
    pub fn knows_about(&self, position: &Position, resource_type: ResourceType) -> bool {
        self.resources
            .get(position)
            .map(|k| k.resource_type == resource_type && k.is_reliable(self.current_tick))
            .unwrap_or(false)
    }


    /// Clean up old unreliable knowledge
    pub fn cleanup_stale(&mut self) {
        self.resources.retain(|_, k| k.is_reliable(self.current_tick));
    }

}

impl Default for PersonalKnowledge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_observation() {
        let mut knowledge = PersonalKnowledge::new();
        let pos = Position::new(10, 10);

        knowledge.observe_resource(pos, ResourceType::Food, 50);

        assert!(knowledge.knows_about(&pos, ResourceType::Food));
        assert_eq!(knowledge.get_known_resources(ResourceType::Food).len(), 1);
    }

    #[test]
    fn test_knowledge_reliability() {
        let knowledge = ResourceKnowledge {
            position: Position::new(0, 0),
            resource_type: ResourceType::Food,
            estimated_amount: 50,
            learned_tick: 0,
            last_verified_tick: 0,
            source: KnowledgeSource::PersonalObservation,
        };

        // Fresh observation is highly reliable
        assert!(knowledge.reliability(0) > 0.9);

        // After 500 ticks, still fairly reliable
        assert!(knowledge.reliability(500) > 0.4);

        // After 1500 ticks, much less reliable
        assert!(knowledge.reliability(1500) < 0.2);
    }

    #[test]
    fn test_learn_from_agent() {
        let mut knowledge = PersonalKnowledge::new();
        let pos = Position::new(10, 10);
        let other_agent = crate::core::dice::name();

        knowledge.learn_from_agent(pos, ResourceType::Food, 50, other_agent);

        assert!(knowledge.knows_about(&pos, ResourceType::Food));

        if let Some(info) = knowledge.resources.get(&pos) {
            assert!(matches!(info.source, KnowledgeSource::DirectCommunication(_)));
        }
    }

    #[test]
    fn test_overhear_doesnt_override_personal() {
        let mut knowledge = PersonalKnowledge::new();
        let pos = Position::new(10, 10);
        let other_agent = crate::core::dice::name();

        // Personal observation first
        knowledge.observe_resource(pos, ResourceType::Food, 50);

        // Overhear different information
        knowledge.overhear_information(pos, ResourceType::Food, 30, other_agent);

        // Should keep personal observation
        if let Some(info) = knowledge.resources.get(&pos) {
            assert_eq!(info.source, KnowledgeSource::PersonalObservation);
            assert_eq!(info.estimated_amount, 50); // Original amount
        }
    }
}
