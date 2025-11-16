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

/// Olfactory (smell) perception system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Smell {
    /// Maximum smell detection range
    pub smell_range: f32,
    /// Olfactory sensitivity (0.0 = no smell, 1.0 = perfect)
    pub sensitivity: f32,
    /// Whether smell is currently impaired
    pub impaired: bool,
    /// Recently detected scents
    pub detected_scents: Vec<Scent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scent {
    pub source_position: (i32, i32, i32),
    pub scent_type: ScentType,
    pub strength: f32, // 0.0 to 1.0
    pub age: u32, // Ticks since detected
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScentType {
    /// Food/edible items
    Food,
    /// Fresh water
    Water,
    /// Blood (from injury or hunt)
    Blood,
    /// Other agents (pheromones)
    Agent,
    /// Smoke from fire
    Smoke,
    /// Decay/rot (danger warning)
    Decay,
    /// Pleasant scents (flowers, herbs)
    Pleasant,
    /// Dangerous/poisonous
    Danger,
    /// Custom scent
    Custom(String),
}

impl Smell {
    pub fn new(smell_range: f32, sensitivity: f32) -> Self {
        Self {
            smell_range,
            sensitivity: sensitivity.clamp(0.0, 1.0),
            impaired: false,
            detected_scents: Vec::new(),
        }
    }

    /// Check if agent can smell a scent at a position
    pub fn can_smell(
        &self,
        smeller_pos: (i32, i32, i32),
        scent_pos: (i32, i32, i32),
        strength: f32,
    ) -> bool {
        if self.impaired || self.sensitivity == 0.0 {
            return false;
        }

        let dx = (scent_pos.0 - smeller_pos.0) as f32;
        let dy = (scent_pos.1 - smeller_pos.1) as f32;
        let dz = (scent_pos.2 - smeller_pos.2) as f32;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Scent diminishes with distance
        let effective_range = self.smell_range * self.sensitivity * strength;
        distance <= effective_range
    }

    /// Register a detected scent
    pub fn detect_scent(&mut self, scent: Scent) {
        self.detected_scents.push(scent);
    }

    /// Age and remove old scents
    pub fn tick(&mut self) {
        for scent in &mut self.detected_scents {
            scent.age += 1;
        }
        // Remove scents older than 200 ticks (scents linger longer than sounds)
        self.detected_scents.retain(|s| s.age < 200);
    }

    /// Get scents of a specific type
    pub fn get_scents_by_type(&self, scent_type: ScentType) -> Vec<&Scent> {
        self.detected_scents
            .iter()
            .filter(|s| s.scent_type == scent_type)
            .collect()
    }

    /// Find nearest scent of a type
    pub fn find_nearest_scent(
        &self,
        scent_type: ScentType,
        agent_pos: (i32, i32, i32),
    ) -> Option<&Scent> {
        self.get_scents_by_type(scent_type)
            .into_iter()
            .min_by_key(|scent| {
                let dx = (scent.source_position.0 - agent_pos.0).abs();
                let dy = (scent.source_position.1 - agent_pos.1).abs();
                let dz = (scent.source_position.2 - agent_pos.2).abs();
                dx + dy + dz
            })
    }

    /// Set impairment (blocked nose, etc.)
    pub fn set_impaired(&mut self, impaired: bool) {
        self.impaired = impaired;
        if impaired {
            self.detected_scents.clear();
        }
    }

    /// Modify sensitivity
    pub fn modify_sensitivity(&mut self, delta: f32) {
        self.sensitivity = (self.sensitivity + delta).clamp(0.0, 1.0);
    }

    /// Get effective smell range
    pub fn effective_range(&self) -> f32 {
        if self.impaired {
            0.0
        } else {
            self.smell_range * self.sensitivity
        }
    }
}

impl Default for Smell {
    fn default() -> Self {
        Self::new(25.0, 1.0) // 25 units range, perfect sensitivity
    }
}

/// Attention/focus system - what the agent is currently paying attention to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attention {
    /// Current focus target (agent, position, or nothing)
    pub focus: Option<Focus>,
    /// Attention span (how long can maintain focus without distraction)
    pub attention_span: u32,
    /// Current attention duration
    pub current_duration: u32,
    /// Distractibility (0.0 = laser focus, 1.0 = easily distracted)
    pub distractibility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Focus {
    /// Focusing on a specific agent
    Agent(Uuid),
    /// Focusing on a position/object
    Position((i32, i32, i32)),
    /// Focusing on a task/activity
    Activity(String),
}

impl Attention {
    pub fn new(attention_span: u32, distractibility: f32) -> Self {
        Self {
            focus: None,
            attention_span,
            current_duration: 0,
            distractibility: distractibility.clamp(0.0, 1.0),
        }
    }

    /// Set focus on something
    pub fn focus_on(&mut self, target: Focus) {
        self.focus = Some(target);
        self.current_duration = 0;
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        self.focus = None;
        self.current_duration = 0;
    }

    /// Update attention state
    pub fn tick(&mut self) {
        if self.focus.is_some() {
            self.current_duration += 1;

            // Check if attention span exceeded
            if self.current_duration > self.attention_span {
                // Chance to lose focus based on distractibility
                use rand::Rng;
                let mut rng = rand::thread_rng();
                if rng.gen_bool(self.distractibility as f64 * 0.1) {
                    self.clear_focus();
                }
            }
        }
    }

    /// Check if agent is currently focused
    pub fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    /// Check if focused on specific agent
    pub fn is_focused_on_agent(&self, agent_id: Uuid) -> bool {
        matches!(&self.focus, Some(Focus::Agent(id)) if *id == agent_id)
    }

    /// Get remaining attention time
    pub fn remaining_attention(&self) -> u32 {
        if let Some(_) = self.focus {
            self.attention_span.saturating_sub(self.current_duration)
        } else {
            0
        }
    }
}

impl Default for Attention {
    fn default() -> Self {
        Self::new(100, 0.3) // 100 ticks attention span, moderate distractibility
    }
}

/// Sensory memory - remembers what was sensed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryMemory {
    /// Recently seen agents (agent_id, last_position, ticks_since_seen)
    pub seen_agents: Vec<(Uuid, (i32, i32, i32), u32)>,
    /// Recently seen positions of interest
    pub seen_positions: Vec<((i32, i32, i32), String, u32)>, // position, description, age
    /// Maximum memory capacity
    pub capacity: usize,
}

impl SensoryMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            seen_agents: Vec::new(),
            seen_positions: Vec::new(),
            capacity,
        }
    }

    /// Remember seeing an agent
    pub fn remember_agent(&mut self, agent_id: Uuid, position: (i32, i32, i32)) {
        // Update if already in memory
        if let Some(entry) = self.seen_agents.iter_mut().find(|(id, _, _)| *id == agent_id) {
            entry.1 = position;
            entry.2 = 0;
            return;
        }

        // Add new memory
        self.seen_agents.push((agent_id, position, 0));

        // Enforce capacity
        if self.seen_agents.len() > self.capacity {
            // Remove oldest (highest age)
            if let Some((idx, _)) = self.seen_agents.iter()
                .enumerate()
                .max_by_key(|(_, (_, _, age))| *age) {
                self.seen_agents.remove(idx);
            }
        }
    }

    /// Remember seeing a position of interest
    pub fn remember_position(&mut self, position: (i32, i32, i32), description: String) {
        // Update if already in memory
        if let Some(entry) = self.seen_positions.iter_mut().find(|(pos, _, _)| *pos == position) {
            entry.1 = description;
            entry.2 = 0;
            return;
        }

        // Add new memory
        self.seen_positions.push((position, description, 0));

        // Enforce capacity
        if self.seen_positions.len() > self.capacity {
            if let Some((idx, _)) = self.seen_positions.iter()
                .enumerate()
                .max_by_key(|(_, (_, _, age))| *age) {
                self.seen_positions.remove(idx);
            }
        }
    }

    /// Get last known position of an agent
    pub fn get_agent_position(&self, agent_id: Uuid) -> Option<(i32, i32, i32)> {
        self.seen_agents
            .iter()
            .find(|(id, _, _)| *id == agent_id)
            .map(|(_, pos, _)| *pos)
    }

    /// Age memories
    pub fn tick(&mut self) {
        for entry in &mut self.seen_agents {
            entry.2 += 1;
        }
        for entry in &mut self.seen_positions {
            entry.2 += 1;
        }

        // Remove very old memories (>1000 ticks)
        self.seen_agents.retain(|(_, _, age)| *age < 1000);
        self.seen_positions.retain(|(_, _, age)| *age < 1000);
    }

    /// Get all recently seen agents (within N ticks)
    pub fn get_recent_agents(&self, max_age: u32) -> Vec<(Uuid, (i32, i32, i32))> {
        self.seen_agents
            .iter()
            .filter(|(_, _, age)| *age <= max_age)
            .map(|(id, pos, _)| (*id, *pos))
            .collect()
    }
}

impl Default for SensoryMemory {
    fn default() -> Self {
        Self::new(50) // Remember up to 50 things
    }
}

/// Complete sensory system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Senses {
    pub vision: Vision,
    pub hearing: Hearing,
    pub speech: Speech,
    pub smell: Smell,
    pub attention: Attention,
    pub memory: SensoryMemory,
}

impl Senses {
    pub fn new() -> Self {
        Self {
            vision: Vision::default(),
            hearing: Hearing::default(),
            speech: Speech::default(),
            smell: Smell::default(),
            attention: Attention::default(),
            memory: SensoryMemory::default(),
        }
    }

    /// Update all sensory systems
    pub fn tick(&mut self) {
        self.hearing.tick();
        self.speech.tick();
        self.smell.tick();
        self.attention.tick();
        self.memory.tick();
    }

    /// Get overall sensory health (0.0 to 1.0)
    pub fn overall_health(&self) -> f32 {
        let vision_health = if self.vision.impaired { 0.0 } else { self.vision.acuity };
        let hearing_health = if self.hearing.impaired { 0.0 } else { self.hearing.sensitivity };
        let speech_health = if !self.speech.can_speak || self.speech.impaired { 0.0 } else { 1.0 };
        let smell_health = if self.smell.impaired { 0.0 } else { self.smell.sensitivity };

        (vision_health + hearing_health + speech_health + smell_health) / 4.0
    }

    /// Check if any important sense is impaired
    pub fn has_impairment(&self) -> bool {
        self.vision.impaired || self.hearing.impaired || self.smell.impaired || self.speech.impaired
    }

    /// Get count of detected threats (loud sounds, danger scents, etc.)
    pub fn threat_level(&self) -> u32 {
        let mut threats = 0;

        // Loud sounds nearby are threatening
        threats += self.hearing.heard_sounds.iter()
            .filter(|s| matches!(s.sound_type, SoundType::Combat) && s.loudness > 0.7)
            .count() as u32;

        // Danger scents
        threats += self.smell.get_scents_by_type(ScentType::Danger).len() as u32;
        threats += self.smell.get_scents_by_type(ScentType::Blood).len() as u32;
        threats += self.smell.get_scents_by_type(ScentType::Decay).len() as u32;

        threats
    }

    /// Find food using senses (smell + memory)
    pub fn find_food_source(&self, agent_pos: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
        // First check smell
        if let Some(scent) = self.smell.find_nearest_scent(ScentType::Food, agent_pos) {
            return Some(scent.source_position);
        }

        // Check memory for food positions
        for (pos, desc, age) in &self.memory.seen_positions {
            if desc.contains("food") && *age < 500 {
                return Some(*pos);
            }
        }

        None
    }

    /// Find water using senses
    pub fn find_water_source(&self, agent_pos: (i32, i32, i32)) -> Option<(i32, i32, i32)> {
        if let Some(scent) = self.smell.find_nearest_scent(ScentType::Water, agent_pos) {
            return Some(scent.source_position);
        }

        for (pos, desc, age) in &self.memory.seen_positions {
            if desc.contains("water") && *age < 500 {
                return Some(*pos);
            }
        }

        None
    }

    /// Find other agents using all senses
    pub fn find_nearby_agents(&self) -> Vec<Uuid> {
        let mut agents = Vec::new();

        // From vision
        agents.extend(self.vision.visible_agents.iter());

        // From recent memory
        let recent = self.memory.get_recent_agents(50);
        for (id, _pos) in recent {
            if !agents.contains(&id) {
                agents.push(id);
            }
        }

        agents
    }

    /// Check if can sense danger
    pub fn senses_danger(&self) -> bool {
        self.threat_level() > 0
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

    #[test]
    fn test_smell_detection() {
        let smell = Smell::new(25.0, 1.0);

        // Close food should be smelled
        assert!(smell.can_smell((0, 0, 0), (10, 0, 0), 1.0));

        // Distant food should not be smelled
        assert!(!smell.can_smell((0, 0, 0), (100, 0, 0), 1.0));
    }

    #[test]
    fn test_smell_find_nearest() {
        let mut smell = Smell::default();

        smell.detect_scent(Scent {
            source_position: (10, 10, 0),
            scent_type: ScentType::Food,
            strength: 1.0,
            age: 0,
        });

        smell.detect_scent(Scent {
            source_position: (5, 5, 0),
            scent_type: ScentType::Food,
            strength: 1.0,
            age: 0,
        });

        let nearest = smell.find_nearest_scent(ScentType::Food, (0, 0, 0));
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().source_position, (5, 5, 0));
    }

    #[test]
    fn test_attention_focus() {
        let mut attention = Attention::default();

        assert!(!attention.is_focused());

        attention.focus_on(Focus::Activity("Mining".to_string()));
        assert!(attention.is_focused());

        attention.clear_focus();
        assert!(!attention.is_focused());
    }

    #[test]
    fn test_attention_span() {
        let mut attention = Attention::new(10, 0.0); // 10 tick span, no distractibility

        attention.focus_on(Focus::Activity("Building".to_string()));

        for _ in 0..5 {
            attention.tick();
        }

        assert!(attention.is_focused());
        assert_eq!(attention.remaining_attention(), 5);
    }

    #[test]
    fn test_sensory_memory_agent() {
        let mut memory = SensoryMemory::new(10);
        let agent_id = Uuid::new_v4();

        memory.remember_agent(agent_id, (10, 10, 0));
        assert_eq!(memory.get_agent_position(agent_id), Some((10, 10, 0)));

        // Update position
        memory.remember_agent(agent_id, (20, 20, 0));
        assert_eq!(memory.get_agent_position(agent_id), Some((20, 20, 0)));
    }

    #[test]
    fn test_sensory_memory_positions() {
        let mut memory = SensoryMemory::new(10);

        memory.remember_position((5, 5, 0), "Resource node".to_string());
        assert_eq!(memory.seen_positions.len(), 1);

        memory.tick();
        assert_eq!(memory.seen_positions[0].2, 1); // Age should increase
    }

    #[test]
    fn test_sensory_memory_capacity() {
        let mut memory = SensoryMemory::new(3);

        for i in 0..5 {
            let id = Uuid::new_v4();
            memory.remember_agent(id, (i, i, 0));
        }

        assert!(memory.seen_agents.len() <= 3); // Should not exceed capacity
    }

    #[test]
    fn test_senses_find_food() {
        let mut senses = Senses::new();

        // Add food scent
        senses.smell.detect_scent(Scent {
            source_position: (15, 15, 0),
            scent_type: ScentType::Food,
            strength: 1.0,
            age: 0,
        });

        let food_pos = senses.find_food_source((0, 0, 0));
        assert!(food_pos.is_some());
        assert_eq!(food_pos.unwrap(), (15, 15, 0));
    }

    #[test]
    fn test_senses_threat_detection() {
        let mut senses = Senses::new();

        assert_eq!(senses.threat_level(), 0);
        assert!(!senses.senses_danger());

        // Add danger scent
        senses.smell.detect_scent(Scent {
            source_position: (5, 5, 0),
            scent_type: ScentType::Danger,
            strength: 1.0,
            age: 0,
        });

        assert!(senses.threat_level() > 0);
        assert!(senses.senses_danger());
    }

    #[test]
    fn test_senses_tick() {
        let mut senses = Senses::new();

        // Add various stimuli
        senses.smell.detect_scent(Scent {
            source_position: (0, 0, 0),
            scent_type: ScentType::Food,
            strength: 1.0,
            age: 0,
        });

        senses.hearing.hear_sound(Sound {
            source_position: (0, 0, 0),
            loudness: 1.0,
            sound_type: SoundType::Speech,
            age: 0,
        });

        senses.tick();

        // Ages should have increased
        assert_eq!(senses.smell.detected_scents[0].age, 1);
        assert_eq!(senses.hearing.heard_sounds[0].age, 1);
    }
}
