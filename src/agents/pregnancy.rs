// src/agents/pregnancy.rs
//! Pregnancy system for agents.

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Duration of pregnancy in ticks
pub const PREGNANCY_DURATION: u32 = 800;

/// Extra energy cost per tick while pregnant (percentage multiplier)
pub const PREGNANCY_ENERGY_MULTIPLIER: f32 = 1.3;

/// Movement speed reduction while pregnant (late stages)
pub const PREGNANCY_SPEED_PENALTY: f32 = 0.7;

/// Pregnancy state for female agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PregnancyState {
    /// Tick when conception occurred
    pub conception_tick: u32,
    /// ID of the father
    pub father_id: Uuid,
    /// Tick when birth is due
    pub due_tick: u32,
    /// Nutrition quality during pregnancy (0.0 to 1.0)
    /// Affects offspring's developmental potential
    pub nutrition_quality: f32,
    /// Number of nutrition samples taken
    nutrition_samples: u32,
}

impl PregnancyState {
    /// Create a new pregnancy
    pub fn new(conception_tick: u32, father_id: Uuid) -> Self {
        Self {
            conception_tick,
            father_id,
            due_tick: conception_tick + PREGNANCY_DURATION,
            nutrition_quality: 1.0,
            nutrition_samples: 0,
        }
    }

    /// Check if pregnancy has reached term
    pub fn is_due(&self, current_tick: u32) -> bool {
        current_tick >= self.due_tick
    }

    /// Get pregnancy progress (0.0 to 1.0)
    pub fn progress(&self, current_tick: u32) -> f32 {
        let elapsed = current_tick.saturating_sub(self.conception_tick) as f32;
        (elapsed / PREGNANCY_DURATION as f32).min(1.0)
    }

    /// Get trimester (1, 2, or 3)
    pub fn trimester(&self, current_tick: u32) -> u8 {
        let progress = self.progress(current_tick);
        if progress < 0.33 {
            1
        } else if progress < 0.66 {
            2
        } else {
            3
        }
    }

    /// Update nutrition quality based on mother's current satiation
    /// Should be called each tick during pregnancy
    pub fn update_nutrition(&mut self, mother_hunger_drive: f32, mother_health: f32) {
        // Lower hunger drive value = better fed (drives are urgency, not satisfaction)
        let nutrition_this_tick = (1.0 - mother_hunger_drive) * (mother_health / 100.0);

        self.nutrition_samples += 1;
        // Rolling average of nutrition quality
        let weight = 1.0 / self.nutrition_samples as f32;
        self.nutrition_quality = self.nutrition_quality * (1.0 - weight) + nutrition_this_tick * weight;
    }

    /// Get movement speed modifier based on pregnancy stage
    pub fn speed_modifier(&self, current_tick: u32) -> f32 {
        let progress = self.progress(current_tick);
        if progress < 0.5 {
            1.0 // No penalty in first half
        } else {
            // Gradually reduce speed in second half
            let late_progress = (progress - 0.5) * 2.0; // 0.0 to 1.0 in second half
            1.0 - (late_progress * (1.0 - PREGNANCY_SPEED_PENALTY))
        }
    }

    /// Get energy cost multiplier based on pregnancy stage
    pub fn energy_multiplier(&self, current_tick: u32) -> f32 {
        let progress = self.progress(current_tick);
        // Energy cost increases throughout pregnancy
        1.0 + (progress * (PREGNANCY_ENERGY_MULTIPLIER - 1.0))
    }

    /// Get ticks remaining until birth
    pub fn ticks_remaining(&self, current_tick: u32) -> u32 {
        self.due_tick.saturating_sub(current_tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pregnancy_progress() {
        let father_id = Uuid::new_v4();
        let pregnancy = PregnancyState::new(100, father_id);

        assert_eq!(pregnancy.progress(100), 0.0);
        assert_eq!(pregnancy.progress(500), 0.5);
        assert_eq!(pregnancy.progress(900), 1.0);
        assert!(pregnancy.progress(1000) <= 1.0); // Capped at 1.0
    }

    #[test]
    fn test_pregnancy_due() {
        let father_id = Uuid::new_v4();
        let pregnancy = PregnancyState::new(100, father_id);

        assert!(!pregnancy.is_due(100));
        assert!(!pregnancy.is_due(899));
        assert!(pregnancy.is_due(900));
        assert!(pregnancy.is_due(1000));
    }

    #[test]
    fn test_trimester() {
        let father_id = Uuid::new_v4();
        let pregnancy = PregnancyState::new(0, father_id);

        assert_eq!(pregnancy.trimester(0), 1);
        assert_eq!(pregnancy.trimester(200), 1);
        assert_eq!(pregnancy.trimester(300), 2);
        assert_eq!(pregnancy.trimester(500), 2);
        assert_eq!(pregnancy.trimester(600), 3);
        assert_eq!(pregnancy.trimester(800), 3);
    }

    #[test]
    fn test_speed_modifier() {
        let father_id = Uuid::new_v4();
        let pregnancy = PregnancyState::new(0, father_id);

        // First half: no penalty
        assert_eq!(pregnancy.speed_modifier(0), 1.0);
        assert_eq!(pregnancy.speed_modifier(400), 1.0);

        // Second half: gradually decreasing
        assert!(pregnancy.speed_modifier(600) < 1.0);
        assert!(pregnancy.speed_modifier(800) <= PREGNANCY_SPEED_PENALTY + 0.01);
    }

    #[test]
    fn test_nutrition_tracking() {
        let father_id = Uuid::new_v4();
        let mut pregnancy = PregnancyState::new(0, father_id);

        // Well-fed mother
        pregnancy.update_nutrition(0.2, 100.0); // Low hunger = well fed
        assert!(pregnancy.nutrition_quality > 0.7);

        // Starving mother reduces quality
        for _ in 0..10 {
            pregnancy.update_nutrition(0.9, 50.0); // High hunger = starving
        }
        assert!(pregnancy.nutrition_quality < 0.5);
    }
}
