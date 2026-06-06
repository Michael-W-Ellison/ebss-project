// src/core/events.rs
//! Core simulation event types for tracking significant occurrences.
//!
//! These types are used throughout the simulation for event logging,
//! analytics, and optional GUI timeline display.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::world::{BuildingType, Position};

/// Types of simulation events that can occur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationEventType {
    /// A new agent was born
    Birth {
        mother_id: Uuid,
        child_id: Uuid,
        father_id: Option<Uuid>,
    },
    /// An agent died
    Death {
        agent_id: Uuid,
        cause: DeathCause,
    },
    /// Combat occurred between agents
    Conflict {
        attacker_id: Uuid,
        target_id: Uuid,
        damage: f32,
        fatal: bool,
    },
    /// A new technology was discovered
    TechnologyDiscovered {
        tech_id: String,
        discoverer_id: Uuid,
        is_world_first: bool,
    },
    /// An agent became pregnant
    Pregnancy {
        mother_id: Uuid,
        father_id: Uuid,
    },
    /// A building was started
    BuildingStarted {
        building_type: BuildingType,
        position: Position,
        builder_id: Uuid,
    },
    /// A building was completed
    BuildingCompleted {
        building_type: BuildingType,
        position: Position,
    },
    /// A major emotional event occurred
    MajorEmotionalEvent {
        agent_id: Uuid,
        emotion: String,
        intensity: f32,
        trigger: String,
    },
    /// An agent collapsed from exhaustion
    Collapse {
        agent_id: Uuid,
    },
    /// An agent was abandoned (left the simulation)
    Abandonment {
        agent_id: Uuid,
    },
    /// Resources were deposited to storehouse
    StorehouseDeposit {
        agent_id: Uuid,
        resource: String,
        amount: u32,
    },
}

/// Cause of death for an agent
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

impl DeathCause {
    /// Get a human-readable description of the death cause
    pub fn description(&self) -> String {
        match self {
            DeathCause::OldAge => "old age".to_string(),
            DeathCause::Starvation => "starvation".to_string(),
            DeathCause::Dehydration => "dehydration".to_string(),
            DeathCause::Combat { killer_id: Some(_) } => "combat".to_string(),
            DeathCause::Combat { killer_id: None } => "injuries".to_string(),
            DeathCause::Exhaustion => "exhaustion".to_string(),
            DeathCause::Exposure => "exposure".to_string(),
            DeathCause::Unknown => "unknown causes".to_string(),
        }
    }
}

/// A single simulation event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationEvent {
    /// Unique identifier for this event
    pub id: Uuid,
    /// Tick when the event occurred
    pub tick: u32,
    /// Type of event with associated data
    pub event_type: SimulationEventType,
    /// Position where the event occurred (if applicable)
    pub position: Option<(i32, i32)>,
}

impl SimulationEvent {
    /// Create a new simulation event
    pub fn new(tick: u32, event_type: SimulationEventType, position: Option<(i32, i32)>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tick,
            event_type,
            position,
        }
    }

    /// Get the primary agent ID associated with this event
    pub fn primary_agent_id(&self) -> Option<Uuid> {
        match &self.event_type {
            SimulationEventType::Birth { child_id, .. } => Some(*child_id),
            SimulationEventType::Death { agent_id, .. } => Some(*agent_id),
            SimulationEventType::Conflict { attacker_id, .. } => Some(*attacker_id),
            SimulationEventType::TechnologyDiscovered { discoverer_id, .. } => Some(*discoverer_id),
            SimulationEventType::Pregnancy { mother_id, .. } => Some(*mother_id),
            SimulationEventType::BuildingStarted { builder_id, .. } => Some(*builder_id),
            SimulationEventType::BuildingCompleted { .. } => None,
            SimulationEventType::MajorEmotionalEvent { agent_id, .. } => Some(*agent_id),
            SimulationEventType::Collapse { agent_id } => Some(*agent_id),
            SimulationEventType::Abandonment { agent_id } => Some(*agent_id),
            SimulationEventType::StorehouseDeposit { agent_id, .. } => Some(*agent_id),
        }
    }

    /// Get a short description of the event
    pub fn short_description(&self) -> String {
        match &self.event_type {
            SimulationEventType::Birth { .. } => "Birth".to_string(),
            SimulationEventType::Death { cause, .. } => format!("Death ({})", cause.description()),
            SimulationEventType::Conflict { fatal, .. } => {
                if *fatal { "Fatal Attack".to_string() } else { "Attack".to_string() }
            }
            SimulationEventType::TechnologyDiscovered { tech_id, is_world_first, .. } => {
                if *is_world_first {
                    format!("Discovery: {}", tech_id)
                } else {
                    format!("Learned: {}", tech_id)
                }
            }
            SimulationEventType::Pregnancy { .. } => "Pregnancy".to_string(),
            SimulationEventType::BuildingStarted { building_type, .. } => {
                format!("Building: {:?}", building_type)
            }
            SimulationEventType::BuildingCompleted { building_type, .. } => {
                format!("Completed: {:?}", building_type)
            }
            SimulationEventType::MajorEmotionalEvent { emotion, .. } => {
                format!("Emotional: {}", emotion)
            }
            SimulationEventType::Collapse { .. } => "Collapsed".to_string(),
            SimulationEventType::Abandonment { .. } => "Left".to_string(),
            SimulationEventType::StorehouseDeposit { resource, amount, .. } => {
                format!("Stored: {} {}", amount, resource)
            }
        }
    }

    /// Get a detailed description of the event
    pub fn detailed_description(&self) -> String {
        match &self.event_type {
            SimulationEventType::Birth { .. } => "A new agent was born".to_string(),
            SimulationEventType::Death { cause, .. } => {
                format!("An agent died from {}", cause.description())
            }
            SimulationEventType::Conflict { damage, fatal, .. } => {
                if *fatal {
                    format!("A fatal attack dealt {:.1} damage", damage)
                } else {
                    format!("An attack dealt {:.1} damage", damage)
                }
            }
            SimulationEventType::TechnologyDiscovered { tech_id, is_world_first, .. } => {
                if *is_world_first {
                    format!("First discovery of {}", tech_id)
                } else {
                    format!("Agent learned {}", tech_id)
                }
            }
            SimulationEventType::Pregnancy { .. } => "An agent became pregnant".to_string(),
            SimulationEventType::BuildingStarted { building_type, .. } => {
                format!("Construction started: {:?}", building_type)
            }
            SimulationEventType::BuildingCompleted { building_type, .. } => {
                format!("Construction completed: {:?}", building_type)
            }
            SimulationEventType::MajorEmotionalEvent { emotion, intensity, trigger, .. } => {
                format!("Strong {} ({:.0}%) from {}", emotion, intensity * 100.0, trigger)
            }
            SimulationEventType::Collapse { .. } => "An agent collapsed from exhaustion".to_string(),
            SimulationEventType::Abandonment { .. } => "An agent left the settlement".to_string(),
            SimulationEventType::StorehouseDeposit { resource, amount, .. } => {
                format!("Deposited {} {} to storehouse", amount, resource)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_death_cause_description() {
        assert_eq!(DeathCause::OldAge.description(), "old age");
        assert_eq!(DeathCause::Starvation.description(), "starvation");
        assert_eq!(DeathCause::Combat { killer_id: Some(Uuid::new_v4()) }.description(), "combat");
        assert_eq!(DeathCause::Combat { killer_id: None }.description(), "injuries");
    }

    #[test]
    fn test_simulation_event_creation() {
        let event = SimulationEvent::new(
            100,
            SimulationEventType::Birth {
                mother_id: Uuid::new_v4(),
                child_id: Uuid::new_v4(),
                father_id: None,
            },
            Some((10, 20)),
        );

        assert_eq!(event.tick, 100);
        assert!(event.primary_agent_id().is_some());
        assert_eq!(event.short_description(), "Birth");
    }

    #[test]
    fn test_event_descriptions() {
        let tech_event = SimulationEvent::new(
            50,
            SimulationEventType::TechnologyDiscovered {
                tech_id: "fire".to_string(),
                discoverer_id: Uuid::new_v4(),
                is_world_first: true,
            },
            None,
        );

        assert_eq!(tech_event.short_description(), "Discovery: fire");
        assert!(tech_event.detailed_description().contains("First discovery"));
    }
}
