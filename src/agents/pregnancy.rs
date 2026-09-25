// src/agents/pregnancy.rs
//! Pregnancy system for agents.

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// How many days a pregnancy lasts: about nine months.
pub const DAYS_A_PREGNANCY_LASTS: u32 = 270;

/// Duration of pregnancy, in ticks - the unit every clock in the model counts
/// in, and the one `PregnancyState::is_due` is asked in.
///
/// It was `800`, written when a clock step was a unit of its own. Every clock
/// now advances by `TICKS_BETWEEN_PLANS`, so eight hundred came to **thirteen
/// hours**: a pair conceived after breakfast and the child was born before
/// dawn. Nine months makes a child something a settlement has to see a body
/// through a season or three to get, which is what a generation costs. See
/// ISSUES_FOUND #253.
pub const PREGNANCY_DURATION: u32 =
    DAYS_A_PREGNANCY_LASTS * crate::environment::seasons::TICKS_PER_DAY;

/// Extra energy cost per turn while pregnant (percentage multiplier)
pub const PREGNANCY_ENERGY_MULTIPLIER: f32 = 1.3;

/// Movement speed reduction while pregnant (late stages)
pub const PREGNANCY_SPEED_PENALTY: f32 = 0.7;

/// Pregnancy state, held by whichever agent is carrying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PregnancyState {
    /// Turn when conception occurred
    pub conception_turn: u32,
    /// ID of the father
    pub father_id: Uuid,
    /// Turn when birth is due
    pub due_turn: u32,
    /// Nutrition quality during pregnancy (0.0 to 1.0)
    /// Affects offspring's developmental potential
    pub nutrition_quality: f32,
    /// Number of nutrition samples taken
    nutrition_samples: u32,
}

impl PregnancyState {
    /// Create a new pregnancy
    pub fn new(conception_turn: u32, father_id: Uuid) -> Self {
        Self {
            conception_turn,
            father_id,
            due_turn: conception_turn + PREGNANCY_DURATION,
            nutrition_quality: 1.0,
            nutrition_samples: 0,
        }
    }

    /// Check if pregnancy has reached term
    pub fn is_due(&self, current_turn: u32) -> bool {
        current_turn >= self.due_turn
    }

    /// Get pregnancy progress (0.0 to 1.0)
    pub fn progress(&self, current_turn: u32) -> f32 {
        let elapsed = current_turn.saturating_sub(self.conception_turn) as f32;
        (elapsed / PREGNANCY_DURATION as f32).min(1.0)
    }

    /// Get trimester (1, 2, or 3)
    pub fn trimester(&self, current_turn: u32) -> u8 {
        let progress = self.progress(current_turn);
        if progress < 0.33 {
            1
        } else if progress < 0.66 {
            2
        } else {
            3
        }
    }

    /// Update nutrition quality based on mother's current satiation
    /// Should be called each turn during pregnancy
    pub fn update_nutrition(&mut self, mother_hunger_drive: f32, mother_health: f32) {
        // Lower hunger drive value = better fed (drives are urgency, not satisfaction)
        let nutrition_this_turn = (1.0 - mother_hunger_drive) * (mother_health / 100.0);

        self.nutrition_samples += 1;
        // Rolling average of nutrition quality
        let weight = 1.0 / self.nutrition_samples as f32;
        self.nutrition_quality = self.nutrition_quality * (1.0 - weight) + nutrition_this_turn * weight;
    }

    /// Get movement speed modifier based on pregnancy stage
    pub fn speed_modifier(&self, current_turn: u32) -> f32 {
        let progress = self.progress(current_turn);
        if progress < 0.5 {
            1.0 // No penalty in first half
        } else {
            // Gradually reduce speed in second half
            let late_progress = (progress - 0.5) * 2.0; // 0.0 to 1.0 in second half
            1.0 - (late_progress * (1.0 - PREGNANCY_SPEED_PENALTY))
        }
    }

    /// Get energy cost multiplier based on pregnancy stage
    pub fn energy_multiplier(&self, current_turn: u32) -> f32 {
        let progress = self.progress(current_turn);
        // Energy cost increases throughout pregnancy
        1.0 + (progress * (PREGNANCY_ENERGY_MULTIPLIER - 1.0))
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    /// A share of the way through, in ticks.
    fn of_the_way(share: f32) -> u32 {
        (PREGNANCY_DURATION as f32 * share) as u32
    }

    #[test]
    fn test_pregnancy_progress() {
        let father_id = crate::core::dice::name();
        let pregnancy = PregnancyState::new(100, father_id);

        assert_eq!(pregnancy.progress(100), 0.0);
        assert_eq!(pregnancy.progress(100 + PREGNANCY_DURATION / 2), 0.5);
        assert_eq!(pregnancy.progress(100 + PREGNANCY_DURATION), 1.0);
        assert!(pregnancy.progress(100 + 2 * PREGNANCY_DURATION) <= 1.0); // Capped at 1.0
    }

    #[test]
    fn test_pregnancy_due() {
        let father_id = crate::core::dice::name();
        let pregnancy = PregnancyState::new(100, father_id);

        assert!(!pregnancy.is_due(100));
        assert!(!pregnancy.is_due(100 + PREGNANCY_DURATION - 1));
        assert!(pregnancy.is_due(100 + PREGNANCY_DURATION));
        assert!(pregnancy.is_due(100 + PREGNANCY_DURATION + 1));
    }

    /// About nine months, and not the thirteen hours `800` came to once every
    /// clock counted in ticks.
    #[test]
    fn a_pregnancy_lasts_about_nine_months() {
        use crate::environment::seasons::{DAYS_PER_MONTH, TICKS_PER_DAY};

        let months = PREGNANCY_DURATION as f32 / (TICKS_PER_DAY * DAYS_PER_MONTH) as f32;
        assert!((8.5..=9.5).contains(&months), "a pregnancy lasts {months:.1} months");
    }

    #[test]
    fn test_trimester() {
        let father_id = crate::core::dice::name();
        let pregnancy = PregnancyState::new(0, father_id);

        assert_eq!(pregnancy.trimester(0), 1);
        assert_eq!(pregnancy.trimester(of_the_way(0.25)), 1);
        assert_eq!(pregnancy.trimester(of_the_way(0.4)), 2);
        assert_eq!(pregnancy.trimester(of_the_way(0.6)), 2);
        assert_eq!(pregnancy.trimester(of_the_way(0.75)), 3);
        assert_eq!(pregnancy.trimester(PREGNANCY_DURATION), 3);
    }

    #[test]
    fn test_speed_modifier() {
        let father_id = crate::core::dice::name();
        let pregnancy = PregnancyState::new(0, father_id);

        // First half: no penalty
        assert_eq!(pregnancy.speed_modifier(0), 1.0);
        assert_eq!(pregnancy.speed_modifier(of_the_way(0.5)), 1.0);

        // Second half: gradually decreasing
        assert!(pregnancy.speed_modifier(of_the_way(0.75)) < 1.0);
        assert!(pregnancy.speed_modifier(PREGNANCY_DURATION) <= PREGNANCY_SPEED_PENALTY + 0.01);
    }

    #[test]
    fn test_nutrition_tracking() {
        let father_id = crate::core::dice::name();
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
