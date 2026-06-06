// src/core/emotions.rs
//! Emotion system for agents.
//!
//! Tracks 5 core emotions: Fear, Anger, Sadness, Happiness, Curiosity
//! Emotions are influenced by traits and affect agent behavior.

use serde::{Deserialize, Serialize};

/// The 5 core emotions that drive agent behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionType {
    Fear,
    Anger,
    Sadness,
    Happiness,
    Curiosity,
}

impl EmotionType {
    pub fn all() -> [EmotionType; 5] {
        [
            EmotionType::Fear,
            EmotionType::Anger,
            EmotionType::Sadness,
            EmotionType::Happiness,
            EmotionType::Curiosity,
        ]
    }

    /// Get default decay rate per tick (emotions naturally return to neutral)
    pub fn default_decay_rate(&self) -> f32 {
        match self {
            EmotionType::Fear => 0.01,      // Fear decays quickly
            EmotionType::Anger => 0.005,    // Anger lingers
            EmotionType::Sadness => 0.003,  // Sadness decays slowly
            EmotionType::Happiness => 0.008, // Happiness fades moderately
            EmotionType::Curiosity => 0.002, // Curiosity persists
        }
    }
}

/// A single emotion state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion {
    pub emotion_type: EmotionType,
    /// Current value (-1.0 to 1.0)
    /// Negative values represent deficits, positive represents surplus
    pub value: f32,
}

impl Emotion {
    pub fn new(emotion_type: EmotionType) -> Self {
        Self {
            emotion_type,
            value: 0.0,
        }
    }

    /// Increase emotion value
    pub fn increase(&mut self, amount: f32) {
        self.value = (self.value + amount).min(1.0);
    }

    /// Decrease emotion value
    pub fn decrease(&mut self, amount: f32) {
        self.value = (self.value - amount).max(-1.0);
    }

    /// Natural decay towards neutral (0.0)
    pub fn decay(&mut self) {
        let decay_rate = self.emotion_type.default_decay_rate();
        if self.value > 0.0 {
            self.value = (self.value - decay_rate).max(0.0);
        } else if self.value < 0.0 {
            self.value = (self.value + decay_rate).min(0.0);
        }
    }

    /// Check if emotion is at an extreme level (> 0.7 or < -0.7)
    pub fn is_extreme(&self) -> bool {
        self.value.abs() > 0.7
    }

    /// Get intensity (0.0 to 1.0, regardless of sign)
    pub fn intensity(&self) -> f32 {
        self.value.abs()
    }
}

/// Complete emotional state for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub emotions: Vec<Emotion>,
}

impl EmotionalState {
    pub fn new() -> Self {
        Self {
            emotions: EmotionType::all()
                .iter()
                .map(|&et| Emotion::new(et))
                .collect(),
        }
    }

    /// Get an emotion by type
    pub fn get(&self, emotion_type: EmotionType) -> Option<&Emotion> {
        self.emotions.iter().find(|e| e.emotion_type == emotion_type)
    }

    /// Get a mutable emotion by type
    pub fn get_mut(&mut self, emotion_type: EmotionType) -> Option<&mut Emotion> {
        self.emotions.iter_mut().find(|e| e.emotion_type == emotion_type)
    }

    /// Update all emotions (apply decay)
    pub fn tick(&mut self) {
        for emotion in &mut self.emotions {
            emotion.decay();
        }
    }

    /// Get the most intense emotion
    pub fn dominant_emotion(&self) -> Option<&Emotion> {
        self.emotions
            .iter()
            .max_by(|a, b| a.intensity().partial_cmp(&b.intensity()).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Check if any emotions are at extreme levels
    pub fn has_extreme_emotions(&self) -> bool {
        self.emotions.iter().any(|e| e.is_extreme())
    }

    /// Get overall emotional well-being (-1.0 to 1.0)
    /// Positive emotions (happiness) contribute positively
    /// Negative emotions (fear, anger, sadness) contribute negatively
    pub fn well_being(&self) -> f32 {
        let happiness = self.get(EmotionType::Happiness).map(|e| e.value).unwrap_or(0.0);
        let fear = self.get(EmotionType::Fear).map(|e| e.value).unwrap_or(0.0);
        let anger = self.get(EmotionType::Anger).map(|e| e.value).unwrap_or(0.0);
        let sadness = self.get(EmotionType::Sadness).map(|e| e.value).unwrap_or(0.0);

        // Well-being is happiness minus negative emotions
        happiness - ((fear + anger + sadness) / 3.0)
    }
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_creation() {
        let emotion = Emotion::new(EmotionType::Happiness);
        assert_eq!(emotion.value, 0.0);
        assert_eq!(emotion.emotion_type, EmotionType::Happiness);
    }

    #[test]
    fn test_emotion_increase_decrease() {
        let mut emotion = Emotion::new(EmotionType::Fear);

        emotion.increase(0.5);
        assert_eq!(emotion.value, 0.5);

        emotion.decrease(0.2);
        assert_eq!(emotion.value, 0.3);
    }

    #[test]
    fn test_emotion_clamping() {
        let mut emotion = Emotion::new(EmotionType::Anger);

        emotion.increase(2.0);
        assert_eq!(emotion.value, 1.0);

        emotion.decrease(3.0);
        assert_eq!(emotion.value, -1.0);
    }

    #[test]
    fn test_emotion_decay() {
        let mut emotion = Emotion::new(EmotionType::Happiness);
        emotion.value = 0.5;

        emotion.decay();
        assert!(emotion.value < 0.5);
        assert!(emotion.value >= 0.0);
    }

    #[test]
    fn test_emotional_state_creation() {
        let state = EmotionalState::new();
        assert_eq!(state.emotions.len(), 5);
    }

    #[test]
    fn test_emotional_state_get() {
        let state = EmotionalState::new();
        let happiness = state.get(EmotionType::Happiness).unwrap();
        assert_eq!(happiness.emotion_type, EmotionType::Happiness);
    }

    #[test]
    fn test_well_being() {
        let mut state = EmotionalState::new();

        state.get_mut(EmotionType::Happiness).unwrap().value = 0.8;
        assert!(state.well_being() > 0.5);

        state.get_mut(EmotionType::Fear).unwrap().value = 0.9;
        assert!(state.well_being() < 0.8);
    }

    #[test]
    fn test_extreme_emotions() {
        let mut state = EmotionalState::new();
        assert!(!state.has_extreme_emotions());

        state.get_mut(EmotionType::Anger).unwrap().value = 0.9;
        assert!(state.has_extreme_emotions());
    }
}
