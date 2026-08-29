// src/agents/gender.rs
//! Gender system for agents.

use serde::{Serialize, Deserialize};
use rand::Rng;

/// Biological gender affecting reproduction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    /// Generate a random gender (50/50 distribution)
    pub fn random() -> Self {
        let mut rng = crate::core::dice::roll();
        if rng.gen_bool(0.5) {
            Gender::Male
        } else {
            Gender::Female
        }
    }

    /// Check if this gender can become pregnant
    pub fn can_become_pregnant(&self) -> bool {
        matches!(self, Gender::Female)
    }

    /// Check if this gender can impregnate
    pub fn can_impregnate(&self) -> bool {
        matches!(self, Gender::Male)
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Gender::Male => "Male",
            Gender::Female => "Female",
        }
    }
}

impl Default for Gender {
    fn default() -> Self {
        Gender::random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gender_pregnancy_capability() {
        assert!(Gender::Female.can_become_pregnant());
        assert!(!Gender::Male.can_become_pregnant());
        assert!(Gender::Male.can_impregnate());
        assert!(!Gender::Female.can_impregnate());
    }

    #[test]
    fn test_random_gender_distribution() {
        let mut male_count = 0;
        let mut female_count = 0;

        for _ in 0..1000 {
            match Gender::random() {
                Gender::Male => male_count += 1,
                Gender::Female => female_count += 1,
            }
        }

        // Should be roughly 50/50 (allow 40-60% range)
        assert!(male_count > 400 && male_count < 600);
        assert!(female_count > 400 && female_count < 600);
    }
}
