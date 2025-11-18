// src/agents/gossip.rs
//! Information sharing and gossip system with truth tracking and trait-based distortion.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::core::traits::Trait;

/// Type of information being shared
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InformationType {
    // Resource information
    ResourceLocation {
        resource: String,
        location: (i32, i32, i32),
    },

    // Social information
    Conflict {
        agent1: Uuid,
        agent2: Uuid,
    },
    EmotionalOutburst {
        agent: Uuid,
        emotion: String,
    },
    UnattachedAgent {
        agent: Uuid,
    },
    Pregnancy {
        agent: Uuid,
    },
    Childbirth {
        agent: Uuid,
        child: Uuid,
    },
    Death {
        agent: Uuid,
        cause: String,
    },

    // Technology/Discovery
    TechnologyDiscovered {
        tech: String,
    },
    RecreationalActivity {
        building: String,
        rating: i32, // 1-10
    },

    // Accusations and suspicions
    Accusation {
        accuser: Uuid,
        accused: Uuid,
        crime: String,
    },
    Alibi {
        agent: Uuid,
        witnesses: Vec<Uuid>,
        time_period: String,
    },
    Observation {
        observer: Uuid,
        observed: String,
        location: (i32, i32, i32),
    },

    // Trait information
    AgentTrait {
        agent: Uuid,
        trait_name: String,
    },
}

/// Piece of information with truth tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Information {
    /// Unique ID for this information
    pub id: Uuid,
    /// Type of information
    pub info_type: InformationType,
    /// Original source (who first reported it)
    pub original_source: Uuid,
    /// Current reliability (0.0 = false, 1.0 = true)
    pub reliability: f32,
    /// Is this objectively true?
    pub ground_truth: bool,
    /// How this info was distorted from truth
    pub distortion: Option<InformationDistortion>,
    /// When this info was created
    pub timestamp: u64,
}

impl Information {
    pub fn new(info_type: InformationType, source: Uuid, ground_truth: bool, timestamp: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            info_type,
            original_source: source,
            reliability: if ground_truth { 1.0 } else { 0.5 },
            ground_truth,
            distortion: None,
            timestamp,
        }
    }

    /// Create distorted version of this information
    pub fn distort(&self, distortion_trait: Trait, distorter: Uuid) -> Self {
        let (new_info_type, distortion_type) = match distortion_trait {
            Trait::Imaginative => self.apply_imaginative_distortion(),
            Trait::Manipulative => self.apply_manipulative_distortion(),
            _ => (self.info_type.clone(), DistortionType::None),
        };

        Self {
            id: Uuid::new_v4(),
            info_type: new_info_type,
            original_source: self.original_source,
            reliability: self.reliability * 0.7, // Distorted info is less reliable
            ground_truth: false, // Distorted info is not true
            distortion: Some(InformationDistortion {
                distortion_type,
                distorter,
                original_info: self.id,
            }),
            timestamp: self.timestamp,
        }
    }

    /// Apply imaginative distortion (exaggeration)
    fn apply_imaginative_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Observation { observer, observed, location } => {
                // "1 rabbit" becomes "a dozen rabbits"
                let exaggerated = format!("a dozen {}", observed);
                (
                    InformationType::Observation {
                        observer: *observer,
                        observed: exaggerated,
                        location: *location,
                    },
                    DistortionType::Exaggeration,
                )
            }
            InformationType::Conflict { agent1: _, agent2: _ } => {
                // "argument" becomes "violent fight"
                (self.info_type.clone(), DistortionType::Exaggeration)
            }
            InformationType::RecreationalActivity { building, rating } => {
                // Boost rating by 3
                (
                    InformationType::RecreationalActivity {
                        building: building.clone(),
                        rating: (*rating + 3).min(10),
                    },
                    DistortionType::Exaggeration,
                )
            }
            _ => (self.info_type.clone(), DistortionType::None),
        }
    }

    /// Apply manipulative distortion (lying)
    fn apply_manipulative_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Observation { observer: _, observed, location: _ } => {
                // "rabbit eating crops" becomes accusation against neighbor
                if observed.contains("rabbit") || observed.contains("animal") {
                    // Would need neighbor info, simplified here
                    (self.info_type.clone(), DistortionType::Fabrication)
                } else {
                    (self.info_type.clone(), DistortionType::None)
                }
            }
            InformationType::Alibi { agent: _, witnesses: _, time_period: _ } => {
                // Manipulative person might claim false alibi
                (self.info_type.clone(), DistortionType::Fabrication)
            }
            _ => (self.info_type.clone(), DistortionType::None),
        }
    }
}

/// How information was distorted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformationDistortion {
    pub distortion_type: DistortionType,
    pub distorter: Uuid,
    pub original_info: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistortionType {
    None,
    Exaggeration,
    Fabrication,
    Omission,
}

/// Agent's belief about a piece of information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub info_id: Uuid,
    pub believed_by: Uuid,
    pub confidence: f32, // 0.0 to 1.0
    pub source: Uuid, // Who told them
    pub timestamp: u64,
}

impl Belief {
    pub fn new(info_id: Uuid, believer: Uuid, source: Uuid, confidence: f32, timestamp: u64) -> Self {
        Self {
            info_id,
            believed_by: believer,
            confidence: confidence.clamp(0.0, 1.0),
            source,
            timestamp,
        }
    }
}

/// Trust rating one agent has for another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRating {
    /// Who is doing the trusting
    pub truster: Uuid,
    /// Who is being trusted
    pub trustee: Uuid,
    /// Trust level (0.0 to 1.0)
    pub trust: f32,
    /// Number of times trustee has been correct
    pub correct_count: u32,
    /// Number of times trustee has been wrong
    pub wrong_count: u32,
}

impl TrustRating {
    pub fn new(truster: Uuid, trustee: Uuid) -> Self {
        Self {
            truster,
            trustee,
            trust: 0.5, // Start neutral
            correct_count: 0,
            wrong_count: 0,
        }
    }

    /// Update trust based on verification
    pub fn update_on_verification(&mut self, was_correct: bool) {
        if was_correct {
            self.correct_count += 1;
            self.trust = (self.trust + 0.1).min(1.0);
        } else {
            self.wrong_count += 1;
            self.trust = (self.trust - 0.15).max(0.0);
        }
    }

    /// Get reliability factor
    pub fn reliability(&self) -> f32 {
        self.trust
    }
}

/// Knowledge base tracking what an agent knows and believes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// All information this agent has heard
    pub known_information: HashMap<Uuid, Information>,
    /// What this agent believes
    pub beliefs: Vec<Belief>,
    /// Trust ratings for other agents
    pub trust_ratings: HashMap<Uuid, TrustRating>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self {
            known_information: HashMap::new(),
            beliefs: Vec::new(),
            trust_ratings: HashMap::new(),
        }
    }

    /// Receive information from another agent
    pub fn receive_information(
        &mut self,
        info: Information,
        source: Uuid,
        receiver: Uuid,
        receiver_traits: &super::TraitSet,
        timestamp: u64,
    ) {
        // Get trust in source
        let trust_rating = self.trust_ratings
            .entry(source)
            .or_insert_with(|| TrustRating::new(receiver, source));

        // Calculate confidence based on trust and receiver traits
        let base_confidence = info.reliability * trust_rating.reliability();
        let trait_modifier = receiver_traits.combined_trust_modifier();
        let confidence = (base_confidence + trait_modifier).clamp(0.0, 1.0);

        // Store information
        let info_id = info.id;
        self.known_information.insert(info_id, info);

        // Create belief
        let belief = Belief::new(info_id, receiver, source, confidence, timestamp);
        self.beliefs.push(belief);
    }

    /// Check if agent believes specific information
    pub fn believes(&self, info_id: &Uuid) -> bool {
        self.beliefs.iter()
            .any(|b| b.info_id == *info_id && b.confidence > 0.5)
    }

    /// Get trust in another agent
    pub fn get_trust(&self, agent_id: &Uuid) -> f32 {
        self.trust_ratings
            .get(agent_id)
            .map(|t| t.trust)
            .unwrap_or(0.5)
    }

    /// Update trust based on verification
    pub fn verify_information(&mut self, info_id: &Uuid, ground_truth: bool) {
        if let Some(info) = self.known_information.get(info_id) {
            let was_correct = info.ground_truth == ground_truth;

            // Find who told us this info
            if let Some(belief) = self.beliefs.iter().find(|b| b.info_id == *info_id) {
                let source = belief.source;

                // Update trust in source
                if let Some(trust) = self.trust_ratings.get_mut(&source) {
                    trust.update_on_verification(was_correct);
                }
            }
        }
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_information_creation() {
        let source = Uuid::new_v4();
        let info = Information::new(
            InformationType::ResourceLocation {
                resource: "wood".to_string(),
                location: (10, 20, 30),
            },
            source,
            true,
            0,
        );

        assert!(info.ground_truth);
        assert_eq!(info.reliability, 1.0);
        assert!(info.distortion.is_none());
    }

    #[test]
    fn test_imaginative_distortion() {
        let source = Uuid::new_v4();
        let observer = Uuid::new_v4();
        let info = Information::new(
            InformationType::Observation {
                observer,
                observed: "rabbit".to_string(),
                location: (5, 5, 0),
            },
            source,
            true,
            0,
        );

        let distorter = Uuid::new_v4();
        let distorted = info.distort(Trait::Imaginative, distorter);

        assert!(!distorted.ground_truth);
        assert_eq!(distorted.reliability, 0.7); // 1.0 * 0.7
        assert!(distorted.distortion.is_some());

        if let InformationType::Observation { observed, .. } = &distorted.info_type {
            assert!(observed.contains("dozen"));
        }
    }

    #[test]
    fn test_trust_rating_update() {
        let truster = Uuid::new_v4();
        let trustee = Uuid::new_v4();
        let mut trust = TrustRating::new(truster, trustee);

        assert_eq!(trust.trust, 0.5);

        trust.update_on_verification(true);
        assert_eq!(trust.trust, 0.6);
        assert_eq!(trust.correct_count, 1);

        trust.update_on_verification(false);
        assert!((trust.trust - 0.45).abs() < 0.001); // Floating point approximation
        assert_eq!(trust.wrong_count, 1);
    }

    #[test]
    fn test_knowledge_base_receive() {
        let mut kb = KnowledgeBase::new();
        let source = Uuid::new_v4();
        let receiver = Uuid::new_v4();
        let traits = super::super::TraitSet::new();

        let info = Information::new(
            InformationType::ResourceLocation {
                resource: "water".to_string(),
                location: (0, 0, 0),
            },
            source,
            true,
            0,
        );

        let info_id = info.id;
        kb.receive_information(info, source, receiver, &traits, 0);

        assert!(kb.known_information.contains_key(&info_id));
        assert_eq!(kb.beliefs.len(), 1);
    }

    #[test]
    fn test_belief_confidence() {
        let mut kb = KnowledgeBase::new();
        let source = Uuid::new_v4();
        let receiver = Uuid::new_v4();

        let mut traits = super::super::TraitSet::new();
        traits.add_trait(Trait::Trusting); // +0.3 trust modifier

        let info = Information::new(
            InformationType::Death {
                agent: Uuid::new_v4(),
                cause: "bear".to_string(),
            },
            source,
            true,
            0,
        );

        kb.receive_information(info, source, receiver, &traits, 0);

        // Should have higher confidence due to Trusting trait
        let belief = &kb.beliefs[0];
        assert!(belief.confidence > 0.5);
    }

    #[test]
    fn test_trust_affects_belief() {
        let mut kb = KnowledgeBase::new();
        let source = Uuid::new_v4();
        let receiver = Uuid::new_v4();
        let traits = super::super::TraitSet::new();

        // Establish low trust
        let mut trust = TrustRating::new(receiver, source);
        trust.trust = 0.2;
        kb.trust_ratings.insert(source, trust);

        let info = Information::new(
            InformationType::Accusation {
                accuser: source,
                accused: Uuid::new_v4(),
                crime: "theft".to_string(),
            },
            source,
            false, // This is a lie
            0,
        );

        kb.receive_information(info, source, receiver, &traits, 0);

        // Low trust should result in low confidence
        let belief = &kb.beliefs[0];
        assert!(belief.confidence < 0.3);
    }

    #[test]
    fn test_verify_information() {
        let mut kb = KnowledgeBase::new();
        let source = Uuid::new_v4();
        let receiver = Uuid::new_v4();
        let traits = super::super::TraitSet::new();

        let info = Information::new(
            InformationType::ResourceLocation {
                resource: "stone".to_string(),
                location: (100, 100, 0),
            },
            source,
            true,
            0,
        );

        let info_id = info.id;
        kb.receive_information(info, source, receiver, &traits, 0);

        // Initial trust
        let initial_trust = kb.get_trust(&source);

        // Verify information as correct
        kb.verify_information(&info_id, true);

        // Trust should increase
        let new_trust = kb.get_trust(&source);
        assert!(new_trust > initial_trust);
    }
}
