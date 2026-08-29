// src/agents/sensory_processing.rs
//! Sensory processing layer that integrates all senses into meaningful percepts.
//!
//! This module takes raw sensory data and processes it into actionable information.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::senses::{Senses, ScentType, SoundType};

/// A percept - a meaningful piece of information derived from senses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Percept {
    /// Detected another agent
    AgentDetected {
        agent_id: Uuid,
        position: (i32, i32, i32),
        detection_method: DetectionMethod,
    },
    /// Detected a resource
    ResourceDetected {
        resource_type: String,
        position: (i32, i32, i32),
        detection_method: DetectionMethod,
    },
    /// Detected danger
    DangerDetected {
        threat_type: ThreatType,
        position: Option<(i32, i32, i32)>,
        severity: f32, // 0.0 to 1.0
    },
    /// Heard speech/communication
    CommunicationHeard {
        source_agent: Option<Uuid>,
        position: (i32, i32, i32),
        volume: f32,
    },
    /// Environmental awareness
    EnvironmentalCondition {
        condition_type: EnvironmentType,
        severity: f32,
    },
    /// Detected a storage container or stockpile
    StorageDetected {
        storage_type: String,
        position: (i32, i32, i32),
        capacity: f32, // 0.0 to 1.0 (how full it is)
        detection_method: DetectionMethod,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionMethod {
    Visual,
    Auditory,
    Olfactory,
    Memory,
    Multiple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatType {
    Combat,
    Predator,
    Environmental,
    Disease,
    Starvation,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvironmentType {
    DarknessImpairing,
    LoudNoise,
    BadSmell,
    Pleasant,
}

/// Process all sensory input and generate percepts
pub fn process_sensory_input(senses: &Senses, agent_pos: (i32, i32, i32)) -> Vec<Percept> {
    let mut percepts = Vec::new();

    // Process vision
    for &agent_id in &senses.vision.visible_agents {
        // Check if this agent is in memory to get position
        if let Some(pos) = senses.memory.get_agent_position(agent_id) {
            percepts.push(Percept::AgentDetected {
                agent_id,
                position: pos,
                detection_method: DetectionMethod::Visual,
            });
        } else {
            // Visible but no position in memory - use agent's position as approximation
            percepts.push(Percept::AgentDetected {
                agent_id,
                position: agent_pos,
                detection_method: DetectionMethod::Visual,
            });
        }
    }

    // Process smell for resources
    for scent in &senses.smell.detected_scents {
        match &scent.scent_type {
            ScentType::Food => {
                percepts.push(Percept::ResourceDetected {
                    resource_type: "Food".to_string(),
                    position: scent.source_position,
                    detection_method: DetectionMethod::Olfactory,
                });
            }
            ScentType::Water => {
                percepts.push(Percept::ResourceDetected {
                    resource_type: "Water".to_string(),
                    position: scent.source_position,
                    detection_method: DetectionMethod::Olfactory,
                });
            }
            ScentType::Blood | ScentType::Danger | ScentType::Decay => {
                percepts.push(Percept::DangerDetected {
                    threat_type: match &scent.scent_type {
                        ScentType::Blood => ThreatType::Combat,
                        ScentType::Decay => ThreatType::Disease,
                        _ => ThreatType::Unknown,
                    },
                    position: Some(scent.source_position),
                    severity: scent.strength,
                });
            }
            ScentType::Pleasant => {
                percepts.push(Percept::EnvironmentalCondition {
                    condition_type: EnvironmentType::Pleasant,
                    severity: scent.strength,
                });
            }
            _ => {}
        }
    }

    // Process hearing
    for sound in &senses.hearing.heard_sounds {
        match sound.sound_type {
            SoundType::Combat => {
                percepts.push(Percept::DangerDetected {
                    threat_type: ThreatType::Combat,
                    position: Some(sound.source_position),
                    severity: sound.loudness,
                });
            }
            SoundType::Speech => {
                percepts.push(Percept::CommunicationHeard {
                    source_agent: None, // Would need additional tracking
                    position: sound.source_position,
                    volume: sound.loudness,
                });
            }
            _ => {}
        }

        // Very loud sounds are environmental conditions
        if sound.loudness > 0.9 {
            percepts.push(Percept::EnvironmentalCondition {
                condition_type: EnvironmentType::LoudNoise,
                severity: sound.loudness,
            });
        }
    }

    // Check for impairments as environmental conditions
    if senses.vision.impaired {
        percepts.push(Percept::EnvironmentalCondition {
            condition_type: EnvironmentType::DarknessImpairing,
            severity: 1.0,
        });
    }

    // Check for multiple detection methods (higher confidence)
    consolidate_percepts(&mut percepts);

    percepts
}

/// Consolidate percepts detected by multiple senses
fn consolidate_percepts(percepts: &mut Vec<Percept>) {
    // Find agents detected by multiple methods
    let mut agent_detections: std::collections::BTreeMap<Uuid, Vec<usize>> = std::collections::BTreeMap::new();

    for (idx, percept) in percepts.iter().enumerate() {
        if let Percept::AgentDetected { agent_id, .. } = percept {
            agent_detections.entry(*agent_id).or_insert_with(Vec::new).push(idx);
        }
    }

    // Mark multi-detection agents
    for (agent_id, indices) in agent_detections {
        if indices.len() > 1 {
            // Keep first, mark as multiple, remove others
            if let Some(&first_idx) = indices.first() {
                if let Percept::AgentDetected { position, .. } = percepts[first_idx] {
                    percepts[first_idx] = Percept::AgentDetected {
                        agent_id,
                        position,
                        detection_method: DetectionMethod::Multiple,
                    };

                    // Remove duplicates
                    for &idx in indices.iter().skip(1).rev() {
                        percepts.remove(idx);
                    }
                }
            }
        }
    }
}

/// Calculate percept importance/salience (0.0 to 1.0)
pub fn calculate_salience(percept: &Percept, agent_drives: &crate::core::DriveState) -> f32 {
    use crate::core::DriveType;

    match percept {
        Percept::DangerDetected { severity, .. } => {
            // Danger is always highly salient
            0.7 + (severity * 0.3)
        }
        Percept::ResourceDetected { resource_type, .. } => {
            // Salience depends on current drives
            let hunger = agent_drives.get(DriveType::Hunger)
                .map(|d| d.value)
                .unwrap_or(0.0);
            let thirst = agent_drives.get(DriveType::Thirst)
                .map(|d| d.value)
                .unwrap_or(0.0);

            match resource_type.as_str() {
                "Food" => hunger * 0.9,
                "Water" => thirst * 0.9,
                _ => 0.3, // Other resources moderately important
            }
        }
        Percept::AgentDetected { detection_method, .. } => {
            // Agent detection salience depends on social drive
            let social = agent_drives.get(DriveType::Social)
                .map(|d| d.value)
                .unwrap_or(0.0);

            let base_salience = social * 0.6;

            // Multiple detection methods increase salience (more certain)
            if *detection_method == DetectionMethod::Multiple {
                (base_salience + 0.2).min(1.0)
            } else {
                base_salience
            }
        }
        Percept::CommunicationHeard { volume, .. } => {
            // Communication is moderately salient, more so if loud
            0.4 + (volume * 0.3)
        }
        Percept::EnvironmentalCondition { severity, .. } => {
            // Environmental conditions have moderate salience
            0.3 + (severity * 0.3)
        }
        Percept::StorageDetected { capacity, detection_method, .. } => {
            // Storage detection salience depends on curiosity and preparedness drives
            let curiosity = agent_drives.get(DriveType::Curiosity)
                .map(|d| d.value)
                .unwrap_or(0.0);
            let preparedness = agent_drives.get(DriveType::Preparedness)
                .map(|d| d.value)
                .unwrap_or(0.0);

            // Base salience from drives
            let base_salience = (curiosity * 0.4 + preparedness * 0.6).min(0.8);

            // Empty storage is less interesting
            let capacity_bonus = capacity * 0.2;

            // Multiple detection methods increase salience
            let detection_bonus = if *detection_method == DetectionMethod::Multiple {
                0.1
            } else {
                0.0
            };

            (base_salience + capacity_bonus + detection_bonus).min(1.0)
        }
    }
}

/// Filter percepts by minimum salience threshold
pub fn filter_by_salience(
    percepts: Vec<Percept>,
    agent_drives: &crate::core::DriveState,
    threshold: f32,
) -> Vec<Percept> {
    percepts.into_iter()
        .filter(|p| calculate_salience(p, agent_drives) >= threshold)
        .collect()
}

/// Get the most salient percept
pub fn most_salient_percept<'a>(
    percepts: &'a [Percept],
    agent_drives: &crate::core::DriveState,
) -> Option<&'a Percept> {
    percepts.iter()
        .max_by(|a, b| {
            let sal_a = calculate_salience(a, agent_drives);
            let sal_b = calculate_salience(b, agent_drives);
            sal_a.partial_cmp(&sal_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::senses::{Senses, Scent, ScentType};

    #[test]
    fn test_process_sensory_input() {
        let mut senses = Senses::new();

        // Add a food scent
        senses.smell.detect_scent(Scent {
            source_position: (10, 10, 0),
            scent_type: ScentType::Food,
            strength: 1.0,
            age: 0,
        });

        let percepts = process_sensory_input(&senses, (0, 0, 0));

        // Should detect food resource
        assert!(!percepts.is_empty());
        let has_food = percepts.iter().any(|p| {
            matches!(p, Percept::ResourceDetected { resource_type, .. } if resource_type == "Food")
        });
        assert!(has_food);
    }

    #[test]
    fn test_danger_detection() {
        let mut senses = Senses::new();

        // Add danger scent
        senses.smell.detect_scent(Scent {
            source_position: (5, 5, 0),
            scent_type: ScentType::Danger,
            strength: 0.8,
            age: 0,
        });

        let percepts = process_sensory_input(&senses, (0, 0, 0));

        let has_danger = percepts.iter().any(|p| {
            matches!(p, Percept::DangerDetected { .. })
        });
        assert!(has_danger);
    }

    #[test]
    fn test_salience_calculation() {
        use crate::core::{DriveState, DriveType};

        let mut drives = DriveState::new();

        // Set high hunger
        if let Some(hunger) = drives.get_mut(DriveType::Hunger) {
            hunger.value = 0.9;
        }

        let food_percept = Percept::ResourceDetected {
            resource_type: "Food".to_string(),
            position: (10, 10, 0),
            detection_method: DetectionMethod::Olfactory,
        };

        let salience = calculate_salience(&food_percept, &drives);
        assert!(salience > 0.7); // Should be highly salient due to high hunger
    }

    #[test]
    fn test_danger_salience() {
        use crate::core::DriveState;

        let drives = DriveState::new();

        let danger_percept = Percept::DangerDetected {
            threat_type: ThreatType::Combat,
            position: Some((5, 5, 0)),
            severity: 1.0,
        };

        let salience = calculate_salience(&danger_percept, &drives);
        assert!(salience >= 0.9); // Danger should always be highly salient
    }

    #[test]
    fn test_filter_by_salience() {
        use crate::core::DriveState;

        let drives = DriveState::new();

        let percepts = vec![
            Percept::DangerDetected {
                threat_type: ThreatType::Combat,
                position: Some((5, 5, 0)),
                severity: 1.0,
            },
            Percept::EnvironmentalCondition {
                condition_type: EnvironmentType::Pleasant,
                severity: 0.5,
            },
        ];

        let filtered = filter_by_salience(percepts, &drives, 0.7);

        // Only danger should remain (high salience)
        assert_eq!(filtered.len(), 1);
        assert!(matches!(filtered[0], Percept::DangerDetected { .. }));
    }

    #[test]
    fn test_most_salient_percept() {
        use crate::core::DriveState;

        let drives = DriveState::new();

        let percepts = vec![
            Percept::EnvironmentalCondition {
                condition_type: EnvironmentType::Pleasant,
                severity: 0.5,
            },
            Percept::DangerDetected {
                threat_type: ThreatType::Combat,
                position: Some((5, 5, 0)),
                severity: 1.0,
            },
        ];

        let most_salient = most_salient_percept(&percepts, &drives);
        assert!(most_salient.is_some());
        assert!(matches!(most_salient.unwrap(), Percept::DangerDetected { .. }));
    }

    #[test]
    fn test_storage_detected_percept() {
        use crate::core::DriveState;

        let percept = Percept::StorageDetected {
            storage_type: "Chest".to_string(),
            position: (10, 10, 0),
            capacity: 0.8,
            detection_method: DetectionMethod::Visual,
        };

        let drives = DriveState::new();
        let salience = calculate_salience(&percept, &drives);

        // Storage should have moderate salience with default drives
        assert!(salience > 0.0);
        assert!(salience < 1.0);
    }

    #[test]
    fn test_storage_salience_with_high_curiosity() {
        use crate::core::{DriveState, DriveType};

        let mut drives = DriveState::new();

        // Set high curiosity
        if let Some(curiosity) = drives.get_mut(DriveType::Curiosity) {
            curiosity.value = 0.9;
        }

        let percept = Percept::StorageDetected {
            storage_type: "Barrel".to_string(),
            position: (5, 5, 0),
            capacity: 1.0,
            detection_method: DetectionMethod::Visual,
        };

        let salience = calculate_salience(&percept, &drives);

        // Should have higher salience with high curiosity
        assert!(salience > 0.4);
    }

    #[test]
    fn test_storage_salience_with_preparedness() {
        use crate::core::{DriveState, DriveType};

        let mut drives = DriveState::new();

        // Set high preparedness drive
        if let Some(preparedness) = drives.get_mut(DriveType::Preparedness) {
            preparedness.value = 0.8;
        }

        let percept = Percept::StorageDetected {
            storage_type: "Storage Pit".to_string(),
            position: (3, 3, 0),
            capacity: 0.5,
            detection_method: DetectionMethod::Visual,
        };

        let salience = calculate_salience(&percept, &drives);

        // Should have high salience with preparedness drive
        assert!(salience > 0.4);
    }

    #[test]
    fn test_storage_multiple_detection() {
        use crate::core::DriveState;

        let drives = DriveState::new();

        let percept = Percept::StorageDetected {
            storage_type: "Cache".to_string(),
            position: (7, 7, 0),
            capacity: 0.9,
            detection_method: DetectionMethod::Multiple,
        };

        let salience = calculate_salience(&percept, &drives);

        // Multiple detection should give bonus salience
        let percept_single = Percept::StorageDetected {
            storage_type: "Cache".to_string(),
            position: (7, 7, 0),
            capacity: 0.9,
            detection_method: DetectionMethod::Visual,
        };

        let salience_single = calculate_salience(&percept_single, &drives);

        assert!(salience > salience_single);
    }
}
