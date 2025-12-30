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
            // Creative/Dramatic distortions
            Trait::Imaginative => self.apply_imaginative_distortion(),
            Trait::Gossip => self.apply_gossip_distortion(),

            // Deceptive distortions
            Trait::Manipulative | Trait::Manipulator => self.apply_manipulative_distortion(),
            Trait::Dishonest => self.apply_dishonest_distortion(),

            // Fear/Anxiety-based distortions
            Trait::Paranoid => self.apply_paranoid_distortion(),
            Trait::Anxious => self.apply_anxious_distortion(),
            Trait::Suspicious => self.apply_suspicious_distortion(),

            // Conflict-related distortions
            Trait::HotHeaded | Trait::Hottempered => self.apply_hothead_distortion(),
            Trait::Vengeful => self.apply_vengeful_distortion(),
            Trait::Aggressive => self.apply_aggressive_distortion(),

            // Calming/Minimizing distortions
            Trait::Calm => self.apply_calm_distortion(),
            Trait::Forgiving => self.apply_forgiving_distortion(),
            Trait::Peaceful => self.apply_peaceful_distortion(),

            // Empathy-based distortions
            Trait::KindHearted => self.apply_kindhearted_distortion(),
            Trait::Cruel | Trait::Callous => self.apply_cruel_distortion(),
            Trait::Empathic | Trait::Empathetic => self.apply_empathic_distortion(),

            // Trust-based distortions
            Trait::Trusting => self.apply_trusting_distortion(),
            Trait::Skeptic => self.apply_skeptic_distortion(),

            // Other traits - no distortion
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

    /// Apply gossip distortion (dramatization for entertainment)
    fn apply_gossip_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Conflict { agent1, agent2 } => {
                // Minor disagreement becomes "bitter feud"
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Exaggeration)
            }
            InformationType::EmotionalOutburst { agent, emotion } => {
                // Emotion is dramatically amplified
                let dramatic = format!("extreme {}", emotion);
                (InformationType::EmotionalOutburst { agent: *agent, emotion: dramatic }, DistortionType::Exaggeration)
            }
            InformationType::UnattachedAgent { agent } => {
                // Single person becomes "desperately lonely"
                (InformationType::UnattachedAgent { agent: *agent }, DistortionType::Exaggeration)
            }
            InformationType::RecreationalActivity { building, rating } => {
                // Either best or worst - gossips love extremes
                let extreme_rating = if *rating >= 5 { 10 } else { 1 };
                (InformationType::RecreationalActivity { building: building.clone(), rating: extreme_rating }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply dishonest distortion (outright lies)
    fn apply_dishonest_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::ResourceLocation { resource, location } => {
                // Lie about resource location
                let fake_location = (location.0 + 50, location.1 + 50, location.2);
                (InformationType::ResourceLocation { resource: resource.clone(), location: fake_location }, DistortionType::Fabrication)
            }
            InformationType::Accusation { accuser, accused, crime } => {
                // Swap accuser and accused, or change the crime
                (InformationType::Accusation { accuser: *accused, accused: *accuser, crime: crime.clone() }, DistortionType::Fabrication)
            }
            InformationType::AgentTrait { agent, trait_name: _ } => {
                // Lie about someone's traits
                (InformationType::AgentTrait { agent: *agent, trait_name: "dishonest".to_string() }, DistortionType::Fabrication)
            }
            _ => (self.info_type.clone(), DistortionType::Fabrication),
        }
    }

    /// Apply paranoid distortion (assumes malice)
    fn apply_paranoid_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Observation { observer, observed, location } => {
                // Neutral observation becomes threatening
                let threatening = format!("suspicious {} (probably plotting something)", observed);
                (InformationType::Observation { observer: *observer, observed: threatening, location: *location }, DistortionType::Exaggeration)
            }
            InformationType::UnattachedAgent { agent } => {
                // Single person is "watching everyone"
                (InformationType::AgentTrait { agent: *agent, trait_name: "suspicious loner".to_string() }, DistortionType::Fabrication)
            }
            InformationType::TechnologyDiscovered { tech } => {
                // New tech is "dangerous"
                let dangerous = format!("dangerous {} (could be used against us)", tech);
                (InformationType::TechnologyDiscovered { tech: dangerous }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply anxious distortion (exaggerates dangers)
    fn apply_anxious_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Observation { observer, observed, location } => {
                // Any observation becomes worrying
                let worrying = format!("alarming {} (we should be careful)", observed);
                (InformationType::Observation { observer: *observer, observed: worrying, location: *location }, DistortionType::Exaggeration)
            }
            InformationType::Death { agent, cause } => {
                // Death becomes epidemic warning
                let scary_cause = format!("{} (could happen to any of us!)", cause);
                (InformationType::Death { agent: *agent, cause: scary_cause }, DistortionType::Exaggeration)
            }
            InformationType::Conflict { agent1, agent2 } => {
                // Conflict might escalate to violence
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply suspicious distortion (adds negative interpretation)
    fn apply_suspicious_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Alibi { agent, witnesses, time_period } => {
                // Alibi seems "too convenient"
                (InformationType::Alibi { agent: *agent, witnesses: witnesses.clone(), time_period: format!("suspiciously during {}", time_period) }, DistortionType::Exaggeration)
            }
            InformationType::ResourceLocation { resource, location } => {
                // Why are they sharing this? What's the angle?
                let suspicious = format!("{} (why tell us this?)", resource);
                (InformationType::ResourceLocation { resource: suspicious, location: *location }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply hot-headed distortion (escalates conflicts)
    fn apply_hothead_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Conflict { agent1, agent2 } => {
                // Minor disagreement becomes major fight
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Exaggeration)
            }
            InformationType::Observation { observer, observed, location } => {
                // Observations become provocations
                let aggressive = format!("{} (an insult to all of us!)", observed);
                (InformationType::Observation { observer: *observer, observed: aggressive, location: *location }, DistortionType::Exaggeration)
            }
            InformationType::Accusation { accuser, accused, crime } => {
                // Crime is definitely intentional and malicious
                let worse_crime = format!("deliberate {}", crime);
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: worse_crime }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply vengeful distortion (emphasizes wrongs)
    fn apply_vengeful_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Conflict { agent1, agent2 } => {
                // Remembers and emphasizes who was wronged
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Exaggeration)
            }
            InformationType::Accusation { accuser, accused, crime } => {
                // Crime is unforgivable
                let unforgivable = format!("unforgivable {}", crime);
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: unforgivable }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply aggressive distortion (frames things as challenges)
    fn apply_aggressive_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Observation { observer, observed, location } => {
                let challenging = format!("{} (a challenge to our strength)", observed);
                (InformationType::Observation { observer: *observer, observed: challenging, location: *location }, DistortionType::Exaggeration)
            }
            InformationType::TechnologyDiscovered { tech } => {
                let weapon_potential = format!("{} (could be weaponized)", tech);
                (InformationType::TechnologyDiscovered { tech: weapon_potential }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply calm distortion (minimizes events)
    fn apply_calm_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Conflict { agent1, agent2 } => {
                // "Fight" becomes "minor disagreement"
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Omission)
            }
            InformationType::EmotionalOutburst { agent, emotion } => {
                // Downplay the emotion
                let mild = format!("slight {}", emotion);
                (InformationType::EmotionalOutburst { agent: *agent, emotion: mild }, DistortionType::Omission)
            }
            InformationType::RecreationalActivity { building, rating } => {
                // Moderate the rating toward average
                let moderate_rating = ((*rating as f32 + 5.0) / 2.0) as i32;
                (InformationType::RecreationalActivity { building: building.clone(), rating: moderate_rating }, DistortionType::Omission)
            }
            _ => (self.info_type.clone(), DistortionType::Omission),
        }
    }

    /// Apply forgiving distortion (downplays wrongdoing)
    fn apply_forgiving_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Accusation { accuser, accused, crime } => {
                // Crime was probably an accident
                let accident = format!("accidental {}", crime);
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: accident }, DistortionType::Omission)
            }
            InformationType::Conflict { agent1, agent2 } => {
                // They've probably made up by now
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Omission)
            }
            _ => (self.info_type.clone(), DistortionType::Omission),
        }
    }

    /// Apply peaceful distortion (removes conflict elements)
    fn apply_peaceful_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Conflict { agent1, agent2 } => {
                // Omit the conflict entirely, focus on resolution
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Omission)
            }
            InformationType::Accusation { accuser, accused, crime } => {
                // Maybe omit the accusation or soften it
                let misunderstanding = format!("misunderstanding about {}", crime);
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: misunderstanding }, DistortionType::Omission)
            }
            _ => (self.info_type.clone(), DistortionType::Omission),
        }
    }

    /// Apply kind-hearted distortion (protects reputations)
    fn apply_kindhearted_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Accusation { accuser, accused, crime: _ } => {
                // Omit negative details about the accused
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: "minor incident".to_string() }, DistortionType::Omission)
            }
            InformationType::AgentTrait { agent, trait_name } => {
                // Reframe negative traits positively
                let positive = match trait_name.as_str() {
                    "lazy" => "relaxed",
                    "aggressive" => "assertive",
                    "dishonest" => "creative",
                    "greedy" => "ambitious",
                    _ => trait_name.as_str(),
                };
                (InformationType::AgentTrait { agent: *agent, trait_name: positive.to_string() }, DistortionType::Omission)
            }
            InformationType::EmotionalOutburst { agent, emotion } => {
                // Protect their dignity
                let gentle = format!("understandable {}", emotion);
                (InformationType::EmotionalOutburst { agent: *agent, emotion: gentle }, DistortionType::Omission)
            }
            _ => (self.info_type.clone(), DistortionType::Omission),
        }
    }

    /// Apply cruel distortion (emphasizes suffering)
    fn apply_cruel_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Death { agent, cause } => {
                // Emphasize the suffering
                let painful = format!("painful {}", cause);
                (InformationType::Death { agent: *agent, cause: painful }, DistortionType::Exaggeration)
            }
            InformationType::EmotionalOutburst { agent, emotion } => {
                // Emphasize their distress
                let pathetic = format!("pathetic {}", emotion);
                (InformationType::EmotionalOutburst { agent: *agent, emotion: pathetic }, DistortionType::Exaggeration)
            }
            InformationType::AgentTrait { agent, trait_name } => {
                // Emphasize negative traits
                let harsh = match trait_name.as_str() {
                    "relaxed" => "lazy",
                    "assertive" => "aggressive",
                    "creative" => "dishonest",
                    "ambitious" => "greedy",
                    _ => trait_name.as_str(),
                };
                (InformationType::AgentTrait { agent: *agent, trait_name: harsh.to_string() }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::Exaggeration),
        }
    }

    /// Apply empathic distortion (adds emotional context)
    fn apply_empathic_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Death { agent, cause } => {
                // Add emotional weight
                let tragic = format!("tragic {}", cause);
                (InformationType::Death { agent: *agent, cause: tragic }, DistortionType::Exaggeration)
            }
            InformationType::Conflict { agent1, agent2 } => {
                // Understand both sides - might blur details
                (InformationType::Conflict { agent1: *agent1, agent2: *agent2 }, DistortionType::Omission)
            }
            InformationType::EmotionalOutburst { agent, emotion } => {
                // Validate and expand on the emotion
                let validated = format!("deeply felt {}", emotion);
                (InformationType::EmotionalOutburst { agent: *agent, emotion: validated }, DistortionType::Exaggeration)
            }
            _ => (self.info_type.clone(), DistortionType::None),
        }
    }

    /// Apply trusting distortion (assumes best intentions)
    fn apply_trusting_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::Accusation { accuser, accused, crime } => {
                // Surely there's a good explanation
                let innocent = format!("alleged {} (probably a misunderstanding)", crime);
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: innocent }, DistortionType::Omission)
            }
            InformationType::Observation { observer, observed, location } => {
                // Interpret positively
                let positive = format!("{} (with good intentions)", observed);
                (InformationType::Observation { observer: *observer, observed: positive, location: *location }, DistortionType::Omission)
            }
            _ => (self.info_type.clone(), DistortionType::None),
        }
    }

    /// Apply skeptic distortion (questions and minimizes)
    fn apply_skeptic_distortion(&self) -> (InformationType, DistortionType) {
        match &self.info_type {
            InformationType::TechnologyDiscovered { tech } => {
                // Downplay discoveries until verified
                let unverified = format!("unconfirmed {}", tech);
                (InformationType::TechnologyDiscovered { tech: unverified }, DistortionType::Omission)
            }
            InformationType::Observation { observer, observed, location } => {
                // Add doubt
                let doubtful = format!("allegedly {}", observed);
                (InformationType::Observation { observer: *observer, observed: doubtful, location: *location }, DistortionType::Omission)
            }
            InformationType::Accusation { accuser, accused, crime } => {
                // Require more evidence
                let unproven = format!("unproven {}", crime);
                (InformationType::Accusation { accuser: *accuser, accused: *accused, crime: unproven }, DistortionType::Omission)
            }
            _ => (self.info_type.clone(), DistortionType::Omission),
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
