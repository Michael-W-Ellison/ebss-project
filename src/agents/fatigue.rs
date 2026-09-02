// src/agents/fatigue.rs
//! Fatigue system tracking tiredness, sleep debt, and associated penalties.

use serde::{Serialize, Deserialize};

/// Hours of wakefulness before fatigue starts impacting performance
pub const FATIGUE_ONSET_TICKS: u32 = 960; // ~16 hours at 60 ticks/hour

/// Maximum fatigue level (complete exhaustion)
pub const MAX_FATIGUE: f32 = 1.0;

/// Fatigue level at which severe penalties apply
pub const SEVERE_FATIGUE_THRESHOLD: f32 = 0.8;

/// Fatigue level at which moderate penalties apply
pub const MODERATE_FATIGUE_THRESHOLD: f32 = 0.5;

/// Fatigue level at which mild penalties apply
pub const MILD_FATIGUE_THRESHOLD: f32 = 0.3;

/// Base fatigue increase per tick while awake
pub const BASE_FATIGUE_RATE: f32 = 0.0005;

/// Fatigue decrease per tick while sleeping (base, modified by sleep quality)
pub const BASE_RECOVERY_RATE: f32 = 0.003;

/// Tracks an agent's fatigue state and sleep patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatigueState {
    /// Current fatigue level (0.0 = fully rested, 1.0 = exhausted)
    pub level: f32,

    /// Accumulated sleep debt (hours of missed sleep)
    pub sleep_debt: f32,

    /// Tick when agent last woke up
    pub last_woke_tick: u32,

    /// Tick when agent last slept
    pub last_slept_tick: u32,

    /// Total ticks slept in the last sleep session
    pub last_sleep_duration: u32,

    /// Whether agent is currently sleeping
    pub is_sleeping: bool,

    /// Quality of last sleep (0.0 to 1.0)
    pub last_sleep_quality: f32,

    /// Consecutive ticks without adequate sleep (for sleep debt calculation)
    pub ticks_without_adequate_sleep: u32,
}

impl Default for FatigueState {
    fn default() -> Self {
        Self {
            level: 0.0,
            sleep_debt: 0.0,
            last_woke_tick: 0,
            last_slept_tick: 0,
            last_sleep_duration: 0,
            is_sleeping: false,
            last_sleep_quality: 1.0,
            ticks_without_adequate_sleep: 0,
        }
    }
}

impl FatigueState {
    /// Create a new fatigue state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update fatigue while awake
    /// Returns the fatigue increase this tick
    pub fn tick_awake(&mut self, activity_level: f32, current_tick: u32) -> f32 {
        self.is_sleeping = false;

        // Base fatigue increase
        let mut fatigue_increase = BASE_FATIGUE_RATE;

        // Activity level affects fatigue (0.0 = resting, 1.0 = strenuous)
        fatigue_increase *= 0.5 + activity_level;

        // Sleep debt causes faster fatigue accumulation
        if self.sleep_debt > 0.0 {
            fatigue_increase *= 1.0 + (self.sleep_debt * 0.1);
        }

        // Apply fatigue increase
        self.level = (self.level + fatigue_increase).min(MAX_FATIGUE);

        // Track time without adequate sleep
        let ticks_awake = current_tick.saturating_sub(self.last_slept_tick);
        if ticks_awake > FATIGUE_ONSET_TICKS {
            self.ticks_without_adequate_sleep = ticks_awake - FATIGUE_ONSET_TICKS;
            // Accumulate sleep debt (1 hour debt per 2 hours over threshold)
            self.sleep_debt = (self.ticks_without_adequate_sleep as f32 / 120.0).min(24.0);
        }

        fatigue_increase
    }

    /// Update fatigue while sleeping
    /// Returns the fatigue decrease this tick
    pub fn tick_sleeping(&mut self, sleep_quality: f32, current_tick: u32) -> f32 {
        self.tick_sleeping_with_modifier(sleep_quality, current_tick, 1.0)
    }

    /// Update fatigue while sleeping with a recovery modifier from traits
    /// recovery_modifier: 1.0 = normal, <1.0 = slower recovery (Narcoleptic), >1.0 = faster
    /// Returns the fatigue decrease this tick
    pub fn tick_sleeping_with_modifier(&mut self, sleep_quality: f32, current_tick: u32, recovery_modifier: f32) -> f32 {
        if !self.is_sleeping {
            // Just started sleeping
            self.is_sleeping = true;
            self.last_slept_tick = current_tick;
            self.last_sleep_duration = 0;
        }

        self.last_sleep_duration += 1;
        self.last_sleep_quality = sleep_quality;

        // Calculate recovery rate based on sleep quality and trait modifier
        let recovery_rate = BASE_RECOVERY_RATE * sleep_quality * recovery_modifier;

        // Reduce fatigue
        let fatigue_decrease = recovery_rate;
        self.level = (self.level - fatigue_decrease).max(0.0);

        // Reduce sleep debt (slower than fatigue recovery)
        if self.sleep_debt > 0.0 {
            self.sleep_debt = (self.sleep_debt - (recovery_rate * 0.5)).max(0.0);
        }

        // Reset inadequate sleep counter if we've slept enough
        if self.last_sleep_duration > 300 { // ~5 hours minimum
            self.ticks_without_adequate_sleep = 0;
        }

        fatigue_decrease
    }

    /// Called when agent wakes up
    pub fn wake_up(&mut self, current_tick: u32) {
        self.is_sleeping = false;
        self.last_woke_tick = current_tick;
    }

    /// Get fatigue severity level
    pub fn severity(&self) -> FatigueSeverity {
        if self.level >= SEVERE_FATIGUE_THRESHOLD {
            FatigueSeverity::Severe
        } else if self.level >= MODERATE_FATIGUE_THRESHOLD {
            FatigueSeverity::Moderate
        } else if self.level >= MILD_FATIGUE_THRESHOLD {
            FatigueSeverity::Mild
        } else {
            FatigueSeverity::None
        }
    }

    /// Get movement speed modifier (1.0 = normal, lower = slower)
    pub fn movement_speed_modifier(&self) -> f32 {
        match self.severity() {
            FatigueSeverity::None => 1.0,
            FatigueSeverity::Mild => 0.95,
            FatigueSeverity::Moderate => 0.85,
            FatigueSeverity::Severe => 0.65,
        }
    }

    /// Get skill effectiveness modifier (1.0 = normal, lower = worse)
    pub fn skill_modifier(&self) -> f32 {
        match self.severity() {
            FatigueSeverity::None => 1.0,
            FatigueSeverity::Mild => 0.95,
            FatigueSeverity::Moderate => 0.80,
            FatigueSeverity::Severe => 0.55,
        }
    }

    /// Get learning rate modifier (1.0 = normal, lower = slower learning)
    pub fn learning_modifier(&self) -> f32 {
        match self.severity() {
            FatigueSeverity::None => 1.0,
            FatigueSeverity::Mild => 0.90,
            FatigueSeverity::Moderate => 0.70,
            FatigueSeverity::Severe => 0.40,
        }
    }

    /// Get happiness modifier (negative values reduce happiness)
    pub fn happiness_modifier(&self) -> f32 {
        match self.severity() {
            FatigueSeverity::None => 0.0,
            FatigueSeverity::Mild => -0.05,
            FatigueSeverity::Moderate => -0.15,
            FatigueSeverity::Severe => -0.30,
        }
    }



    /// Check if agent should be forced to sleep (extreme exhaustion)
    pub fn should_collapse(&self) -> bool {
        self.level >= 0.95 || self.sleep_debt >= 20.0
    }

    /// Check if agent desperately needs sleep
    pub fn desperately_needs_sleep(&self) -> bool {
        self.level >= SEVERE_FATIGUE_THRESHOLD || self.sleep_debt >= 12.0
    }

    /// Check if agent needs sleep soon
    pub fn needs_sleep(&self) -> bool {
        self.level >= MODERATE_FATIGUE_THRESHOLD || self.sleep_debt >= 6.0
    }

    /// Check if agent needs sleep with trait-based threshold modifier
    /// threshold_modifier: 1.0 = normal, <1.0 = needs less sleep (SoundSleeper)
    pub fn needs_sleep_with_modifier(&self, threshold_modifier: f32) -> bool {
        let adjusted_threshold = MODERATE_FATIGUE_THRESHOLD * threshold_modifier;
        let adjusted_debt_threshold = 6.0 * threshold_modifier;
        self.level >= adjusted_threshold || self.sleep_debt >= adjusted_debt_threshold
    }

    /// Check if desperately needs sleep with trait modifier
    pub fn desperately_needs_sleep_with_modifier(&self, threshold_modifier: f32) -> bool {
        let adjusted_threshold = SEVERE_FATIGUE_THRESHOLD * threshold_modifier;
        let adjusted_debt_threshold = 12.0 * threshold_modifier;
        self.level >= adjusted_threshold || self.sleep_debt >= adjusted_debt_threshold
    }

    /// Get a description of current fatigue state
    pub fn description(&self) -> &'static str {
        if self.is_sleeping {
            "Sleeping"
        } else {
            match self.severity() {
                FatigueSeverity::None => "Well-rested",
                FatigueSeverity::Mild => "Slightly tired",
                FatigueSeverity::Moderate => "Fatigued",
                FatigueSeverity::Severe => "Exhausted",
            }
        }
    }

}

/// Severity levels of fatigue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatigueSeverity {
    None,
    Mild,
    Moderate,
    Severe,
}

/// Factors affecting sleep quality
#[derive(Debug, Clone, Default)]
pub struct SleepQualityFactors {
    /// Has shelter (roof over head)
    pub has_shelter: bool,
    /// Has a bed or comfortable surface
    pub has_bed: bool,
    /// Safety level (0.0 = danger, 1.0 = safe)
    pub safety: f32,
    /// Health percentage (0.0 to 1.0)
    pub health: f32,
    /// Hunger level (0.0 = full, 1.0 = starving)
    pub hunger: f32,
    /// Environmental comfort (temperature, noise, etc.)
    pub comfort: f32,
}

impl SleepQualityFactors {
    /// Calculate overall sleep quality from factors
    pub fn calculate_quality(&self) -> f32 {
        let mut quality = 0.5; // Base quality

        // Shelter bonus
        if self.has_shelter {
            quality += 0.15;
        }

        // Bed bonus
        if self.has_bed {
            quality += 0.2;
        }

        // Safety factor (major impact)
        quality *= 0.5 + (self.safety * 0.5);

        // Health factor
        quality *= 0.7 + (self.health * 0.3);

        // Hunger penalty
        if self.hunger > 0.5 {
            quality *= 1.0 - ((self.hunger - 0.5) * 0.4);
        }

        // Comfort factor
        quality *= 0.8 + (self.comfort * 0.2);

        quality.clamp(0.1, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fatigue_accumulation() {
        let mut fatigue = FatigueState::new();
        assert_eq!(fatigue.level, 0.0);

        // Simulate being awake for a while
        for tick in 0..1000 {
            fatigue.tick_awake(0.5, tick);
        }

        // Should have accumulated some fatigue
        assert!(fatigue.level > 0.2);
        assert!(fatigue.level < 1.0);
    }

    #[test]
    fn test_sleep_recovery() {
        let mut fatigue = FatigueState::new();
        fatigue.level = 0.8; // Start tired

        // Sleep with good quality
        for tick in 0..500 {
            fatigue.tick_sleeping(0.9, tick);
        }

        // Should have recovered significantly
        assert!(fatigue.level < 0.3);
    }

    #[test]
    fn test_movement_modifier() {
        let mut fatigue = FatigueState::new();

        // Well-rested
        assert_eq!(fatigue.movement_speed_modifier(), 1.0);

        // Exhausted
        fatigue.level = 0.9;
        assert!(fatigue.movement_speed_modifier() < 0.7);
    }

    #[test]
    fn test_sleep_quality_calculation() {
        // Perfect conditions
        let perfect = SleepQualityFactors {
            has_shelter: true,
            has_bed: true,
            safety: 1.0,
            health: 1.0,
            hunger: 0.0,
            comfort: 1.0,
        };
        assert!(perfect.calculate_quality() > 0.8); // Very good quality

        // Terrible conditions
        let terrible = SleepQualityFactors {
            has_shelter: false,
            has_bed: false,
            safety: 0.2,
            health: 0.5,
            hunger: 0.9,
            comfort: 0.2,
        };
        assert!(terrible.calculate_quality() < 0.3);
    }

    #[test]
    fn test_collapse_threshold() {
        let mut fatigue = FatigueState::new();

        assert!(!fatigue.should_collapse());

        fatigue.level = 0.96;
        assert!(fatigue.should_collapse());

        fatigue.level = 0.5;
        fatigue.sleep_debt = 21.0;
        assert!(fatigue.should_collapse());
    }

    #[test]
    fn test_sleep_debt_accumulation() {
        let mut fatigue = FatigueState::new();
        fatigue.last_slept_tick = 0;

        // Stay awake way past fatigue onset
        let late_tick = FATIGUE_ONSET_TICKS + 600; // 10 extra hours
        fatigue.tick_awake(0.5, late_tick);

        // Should have accumulated sleep debt
        assert!(fatigue.sleep_debt > 0.0);
        assert!(fatigue.ticks_without_adequate_sleep > 0);
    }
}
