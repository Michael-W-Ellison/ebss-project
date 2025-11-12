// src/agents/senses.rs
//! Sensory system for agents including sight, hearing, and speech.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Visual perception system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vision {
    /// Field of view in degrees (0-360)
    pub field_of_view: f32,
    /// Maximum detection range
    pub detection_range: f32,
    /// Visual acuity (0.0 = blind, 1.0 = perfect vision)
    pub acuity: f32,
    /// Whether vision is currently impaired (darkness, blindness, etc.)
    pub impaired: bool,
    /// Currently visible agents
    pub visible_agents: HashSet<Uuid>,
    /// Currently visible positions (materials/terrain)
    pub visible_positions: HashSet<(i32, i32, i32)>,
}

impl Vision {
    pub fn new(field_of_view: f32, detection_range: f32, acuity: f32) -> Self {
        Self {
            field_of_view: field_of_view.clamp(0.0, 360.0),
            detection_range,
            acuity: acuity.clamp(0.0, 1.0),
            impaired: false,
            visible_agents: HashSet::new(),
            visible_positions: HashSet::new(),
        }
    }

    /// Check if an agent can see a position given their position and facing direction
    pub fn can_see_position(
        &self,
        observer_pos: (i32, i32, i32),
        target_pos: (i32, i32, i32),
        facing_direction: f32, // Angle in degrees
    ) -> bool {
        if self.impaired || self.acuity == 0.0 {
            return false;
        }

        // Calculate distance
        let dx = (target_pos.0 - observer_pos.0) as f32;
        let dy = (target_pos.1 - observer_pos.1) as f32;
        let dz = (target_pos.2 - observer_pos.2) as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Check range
        if distance > self.detection_range {
            return false;
        }

        // Calculate angle to target
        let angle_to_target = dz.atan2(dx).to_degrees();
        let angle_diff = ((angle_to_target - facing_direction + 180.0) % 360.0 - 180.0).abs();

        // Check if within field of view
        angle_diff <= self.field_of_view / 2.0
    }

    /// Update list of visible agents
    pub fn update_visible_agents(&mut self, agents: Vec<Uuid>) {
        self.visible_agents = agents.into_iter().collect();
    }

    /// Update list of visible positions
    pub fn update_visible_positions(&mut self, positions: Vec<(i32, i32, i32)>) {
        self.visible_positions = positions.into_iter().collect();
    }

    /// Impair vision (temporary blindness, darkness, etc.)
    pub fn set_impaired(&mut self, impaired: bool) {
        self.impaired = impaired;
        if impaired {
            self.visible_agents.clear();
            self.visible_positions.clear();
        }
    }

    /// Modify acuity (injury, enhancement, etc.)
    pub fn modify_acuity(&mut self, delta: f32) {
        self.acuity = (self.acuity + delta).clamp(0.0, 1.0);
    }

    /// Get effective detection range considering acuity
    pub fn effective_range(&self) -> f32 {
        if self.impaired {
            0.0
        } else {
            self.detection_range * self.acuity
        }
    }
}

impl Default for Vision {
    fn default() -> Self {
        Self::new(180.0, 50.0, 1.0) // 180° FOV, 50 units range, perfect acuity
    }
}

/// Auditory perception system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hearing {
    /// Maximum hearing range
    pub hearing_range: f32,
    /// Hearing sensitivity (0.0 = deaf, 1.0 = perfect hearing)
    pub sensitivity: f32,
    /// Whether hearing is currently impaired
    pub impaired: bool,
    /// Recently heard sounds (source position, loudness)
    pub heard_sounds: Vec<Sound>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sound {
    pub source_position: (i32, i32, i32),
    pub loudness: f32,
    pub sound_type: SoundType,
    pub age: u32, // Ticks since heard
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundType {
    /// Footsteps from movement
    Footsteps,
    /// Speech/communication
    Speech,
    /// Tool use (mining, chopping, etc.)
    ToolUse,
    /// Combat sounds
    Combat,
    /// Building/construction
    Construction,
    /// Environmental (water, wind, etc.)
    Environmental,
    /// Custom sound type
    Custom,
}

impl Hearing {
    pub fn new(hearing_range: f32, sensitivity: f32) -> Self {
        Self {
            hearing_range,
            sensitivity: sensitivity.clamp(0.0, 1.0),
            impaired: false,
            heard_sounds: Vec::new(),
        }
    }

    /// Check if agent can hear a sound at a position
    pub fn can_hear(
        &self,
        listener_pos: (i32, i32, i32),
        sound_pos: (i32, i32, i32),
        loudness: f32,
    ) -> bool {
        if self.impaired || self.sensitivity == 0.0 {
            return false;
        }

        let dx = (sound_pos.0 - listener_pos.0) as f32;
        let dy = (sound_pos.1 - listener_pos.1) as f32;
        let dz = (sound_pos.2 - listener_pos.2) as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Sound diminishes with distance
        let effective_range = self.hearing_range * self.sensitivity * loudness;
        distance <= effective_range
    }

    /// Register a heard sound
    pub fn hear_sound(&mut self, sound: Sound) {
        self.heard_sounds.push(sound);
    }

    /// Age and remove old sounds
    pub fn tick(&mut self) {
        for sound in &mut self.heard_sounds {
            sound.age += 1;
        }
        // Remove sounds older than 100 ticks
        self.heard_sounds.retain(|s| s.age < 100);
    }

    /// Get sounds of a specific type
    pub fn get_sounds_by_type(&self, sound_type: SoundType) -> Vec<&Sound> {
        self.heard_sounds
            .iter()
            .filter(|s| s.sound_type == sound_type)
            .collect()
    }

    /// Set impairment (deafness, loud noise, etc.)
    pub fn set_impaired(&mut self, impaired: bool) {
        self.impaired = impaired;
        if impaired {
            self.heard_sounds.clear();
        }
    }

    /// Modify sensitivity
    pub fn modify_sensitivity(&mut self, delta: f32) {
        self.sensitivity = (self.sensitivity + delta).clamp(0.0, 1.0);
    }

    /// Get effective hearing range
    pub fn effective_range(&self) -> f32 {
        if self.impaired {
            0.0
        } else {
            self.hearing_range * self.sensitivity
        }
    }
}

impl Default for Hearing {
    fn default() -> Self {
        Self::new(30.0, 1.0) // 30 units range, perfect sensitivity
    }
}

/// Speech and communication system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speech {
    /// Whether agent can currently speak
    pub can_speak: bool,
    /// Volume of speech (0.0 = whisper, 1.0 = shout)
    pub volume: f32,
    /// Speech impairment (injury, enchantment, etc.)
    pub impaired: bool,
    /// Languages known by agent
    pub known_languages: HashSet<String>,
    /// Recent utterances
    pub recent_speech: Vec<Utterance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utterance {
    pub content: String,
    pub language: String,
    pub volume: f32,
    pub age: u32, // Ticks since spoken
}

impl Speech {
    pub fn new() -> Self {
        Self {
            can_speak: true,
            volume: 0.7,
            impaired: false,
            known_languages: ["common".to_string()].into_iter().collect(),
            recent_speech: Vec::new(),
        }
    }

    /// Speak an utterance
    pub fn speak(&mut self, content: String, language: String) -> Option<Utterance> {
        if !self.can_speak || self.impaired {
            return None;
        }

        if !self.known_languages.contains(&language) {
            return None;
        }

        let utterance = Utterance {
            content,
            language,
            volume: self.volume,
            age: 0,
        };

        self.recent_speech.push(utterance.clone());
        Some(utterance)
    }

    /// Learn a new language
    pub fn learn_language(&mut self, language: String) {
        self.known_languages.insert(language);
    }

    /// Check if agent knows a language
    pub fn knows_language(&self, language: &str) -> bool {
        self.known_languages.contains(language)
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Set speech impairment
    pub fn set_impaired(&mut self, impaired: bool) {
        self.impaired = impaired;
    }

    /// Age and remove old speech
    pub fn tick(&mut self) {
        for utterance in &mut self.recent_speech {
            utterance.age += 1;
        }
        // Remove utterances older than 50 ticks
        self.recent_speech.retain(|u| u.age < 50);
    }

    /// Get effective speech range (for others to hear)
    pub fn effective_range(&self) -> f32 {
        if !self.can_speak || self.impaired {
            0.0
        } else {
            // Base range of 20, scaled by volume
            20.0 * self.volume
        }
    }
}

impl Default for Speech {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete sensory system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Senses {
    pub vision: Vision,
    pub hearing: Hearing,
    pub speech: Speech,
}

impl Senses {
    pub fn new() -> Self {
        Self {
            vision: Vision::default(),
            hearing: Hearing::default(),
            speech: Speech::default(),
        }
    }

    /// Update all sensory systems
    pub fn tick(&mut self) {
        self.hearing.tick();
        self.speech.tick();
    }

    /// Get overall sensory health (0.0 to 1.0)
    pub fn overall_health(&self) -> f32 {
        let vision_health = if self.vision.impaired { 0.0 } else { self.vision.acuity };
        let hearing_health = if self.hearing.impaired { 0.0 } else { self.hearing.sensitivity };
        let speech_health = if !self.speech.can_speak || self.speech.impaired { 0.0 } else { 1.0 };

        (vision_health + hearing_health + speech_health) / 3.0
    }
}

impl Default for Senses {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_creation() {
        let vision = Vision::new(180.0, 50.0, 1.0);
        assert_eq!(vision.field_of_view, 180.0);
        assert_eq!(vision.detection_range, 50.0);
        assert_eq!(vision.acuity, 1.0);
        assert!(!vision.impaired);
    }

    #[test]
    fn test_vision_impairment() {
        let mut vision = Vision::default();
        vision.visible_agents.insert(Uuid::new_v4());

        vision.set_impaired(true);
        assert!(vision.impaired);
        assert!(vision.visible_agents.is_empty());
        assert_eq!(vision.effective_range(), 0.0);
    }

    #[test]
    fn test_hearing_sound_detection() {
        let hearing = Hearing::new(30.0, 1.0);

        // Close sound should be heard
        assert!(hearing.can_hear((0, 0, 0), (10, 0, 0), 1.0));

        // Distant sound should not be heard
        assert!(!hearing.can_hear((0, 0, 0), (100, 0, 0), 1.0));
    }

    #[test]
    fn test_hearing_tick() {
        let mut hearing = Hearing::default();
        hearing.hear_sound(Sound {
            source_position: (0, 0, 0),
            loudness: 1.0,
            sound_type: SoundType::Footsteps,
            age: 0,
        });

        assert_eq!(hearing.heard_sounds.len(), 1);

        hearing.tick();
        assert_eq!(hearing.heard_sounds[0].age, 1);
    }

    #[test]
    fn test_speech_utterance() {
        let mut speech = Speech::new();

        let utterance = speech.speak("Hello".to_string(), "common".to_string());
        assert!(utterance.is_some());
        assert_eq!(speech.recent_speech.len(), 1);
    }

    #[test]
    fn test_speech_unknown_language() {
        let mut speech = Speech::new();

        let utterance = speech.speak("Bonjour".to_string(), "french".to_string());
        assert!(utterance.is_none());

        speech.learn_language("french".to_string());
        let utterance = speech.speak("Bonjour".to_string(), "french".to_string());
        assert!(utterance.is_some());
    }

    #[test]
    fn test_speech_impairment() {
        let mut speech = Speech::new();
        speech.set_impaired(true);

        let utterance = speech.speak("Hello".to_string(), "common".to_string());
        assert!(utterance.is_none());
        assert_eq!(speech.effective_range(), 0.0);
    }

    #[test]
    fn test_senses_overall_health() {
        let senses = Senses::new();
        assert_eq!(senses.overall_health(), 1.0);

        let mut impaired_senses = Senses::new();
        impaired_senses.vision.set_impaired(true);
        assert!(impaired_senses.overall_health() < 1.0);
    }
}
