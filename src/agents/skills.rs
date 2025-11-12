// src/agents/skills.rs
//! Skill system for agent proficiency and progression.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;

/// Types of skills agents can develop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillType {
    /// Mining stone, ore, etc.
    Mining,
    /// Chopping trees
    Woodcutting,
    /// General crafting
    Crafting,
    /// Building structures
    Construction,
    /// Farming and agriculture
    Farming,
    /// Smelting ores
    Smelting,
    /// Cooking food
    Cooking,
    /// Hunting animals
    Hunting,
    /// Fishing
    Fishing,
    /// Herbalism and foraging
    Herbalism,
    /// Leatherworking
    Leatherworking,
    /// Metalworking
    Metalworking,
    /// Carpentry
    Carpentry,
    /// Masonry
    Masonry,
    /// Archery
    Archery,
    /// Melee combat
    MeleeCombat,
    /// Custom skill
    Custom(u32),
}

impl SkillType {
    pub fn name(&self) -> &'static str {
        match self {
            SkillType::Mining => "Mining",
            SkillType::Woodcutting => "Woodcutting",
            SkillType::Crafting => "Crafting",
            SkillType::Construction => "Construction",
            SkillType::Farming => "Farming",
            SkillType::Smelting => "Smelting",
            SkillType::Cooking => "Cooking",
            SkillType::Hunting => "Hunting",
            SkillType::Fishing => "Fishing",
            SkillType::Herbalism => "Herbalism",
            SkillType::Leatherworking => "Leatherworking",
            SkillType::Metalworking => "Metalworking",
            SkillType::Carpentry => "Carpentry",
            SkillType::Masonry => "Masonry",
            SkillType::Archery => "Archery",
            SkillType::MeleeCombat => "Melee Combat",
            SkillType::Custom(_) => "Custom Skill",
        }
    }
}

/// Skill proficiency categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCategory {
    /// Skill level -10 to -6
    None,
    /// Skill level -5 to -1 (Apprentice)
    Low,
    /// Skill level 0 to 5 (Journeyman)
    Medium,
    /// Skill level 6 to 10 (Master)
    High,
}

impl SkillCategory {
    /// Get title for this skill category
    pub fn title(&self) -> Option<&'static str> {
        match self {
            SkillCategory::None => None,
            SkillCategory::Low => Some("Apprentice"),
            SkillCategory::Medium => Some("Journeyman"),
            SkillCategory::High => Some("Master"),
        }
    }

    /// Get small injury chance
    pub fn small_injury_chance(&self) -> f32 {
        match self {
            SkillCategory::None => 0.25,
            SkillCategory::Low => 0.15,
            SkillCategory::Medium => 0.05,
            SkillCategory::High => 0.01,
        }
    }

    /// Get large injury chance
    pub fn large_injury_chance(&self) -> f32 {
        match self {
            SkillCategory::None => 0.10,
            SkillCategory::Low => 0.05,
            SkillCategory::Medium => 0.01,
            SkillCategory::High => 0.0,
        }
    }

    /// Get failure chance
    pub fn failure_chance(&self) -> f32 {
        match self {
            SkillCategory::None => 0.50,
            SkillCategory::Low => 0.30,
            SkillCategory::Medium => 0.10,
            SkillCategory::High => 0.0,
        }
    }
}

/// Quality levels for produced items
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Quality {
    Pathetic = 0,
    Crude = 1,
    Basic = 2,
    Moderate = 3,
    Advanced = 4,
    Expert = 5,
}

impl Quality {
    pub fn name(&self) -> &'static str {
        match self {
            Quality::Pathetic => "Pathetic",
            Quality::Crude => "Crude",
            Quality::Basic => "Basic",
            Quality::Moderate => "Moderate",
            Quality::Advanced => "Advanced",
            Quality::Expert => "Expert",
        }
    }

    /// Get quality modifier for item effectiveness
    pub fn modifier(&self) -> f32 {
        match self {
            Quality::Pathetic => 0.5,
            Quality::Crude => 0.7,
            Quality::Basic => 1.0,
            Quality::Moderate => 1.3,
            Quality::Advanced => 1.6,
            Quality::Expert => 2.0,
        }
    }
}

/// Result of a skill check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCheckResult {
    pub success: bool,
    pub quality: Option<Quality>,
    pub injury: Option<InjuryType>,
    pub speed_multiplier: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjuryType {
    Small,
    Large,
}

/// Individual skill with level and progression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub skill_type: SkillType,
    pub level: i32, // -10 to 10
    pub experience: u32,
}

impl Skill {
    pub fn new(skill_type: SkillType) -> Self {
        Self {
            skill_type,
            level: -10, // Start at lowest level
            experience: 0,
        }
    }

    /// Create skill with specific level
    pub fn with_level(skill_type: SkillType, level: i32) -> Self {
        Self {
            skill_type,
            level: level.clamp(-10, 10),
            experience: 0,
        }
    }

    /// Get skill category
    pub fn category(&self) -> SkillCategory {
        match self.level {
            -10..=-6 => SkillCategory::None,
            -5..=-1 => SkillCategory::Low,
            0..=5 => SkillCategory::Medium,
            6..=10 => SkillCategory::High,
            _ => SkillCategory::None,
        }
    }

    /// Get skill title
    pub fn title(&self) -> Option<&'static str> {
        self.category().title()
    }

    /// Get speed multiplier (each level = 5%)
    pub fn speed_multiplier(&self) -> f32 {
        1.0 + (self.level as f32 * 0.05)
    }

    /// Perform skill check
    pub fn perform_check(&self) -> SkillCheckResult {
        let mut rng = rand::thread_rng();
        let category = self.category();

        // Check for injury
        let injury = if rng.gen::<f32>() < category.large_injury_chance() {
            Some(InjuryType::Large)
        } else if rng.gen::<f32>() < category.small_injury_chance() {
            Some(InjuryType::Small)
        } else {
            None
        };

        // Check for failure
        let success = rng.gen::<f32>() >= category.failure_chance();

        // Determine quality if successful
        let quality = if success {
            Some(self.determine_quality())
        } else {
            None
        };

        SkillCheckResult {
            success,
            quality,
            injury,
            speed_multiplier: self.speed_multiplier(),
        }
    }

    /// Determine quality based on skill level distribution
    fn determine_quality(&self) -> Quality {
        let roll = rand::thread_rng().gen::<f32>() * 100.0;

        match self.level {
            -10 => Quality::Pathetic,
            -9 => if roll < 90.0 { Quality::Pathetic } else { Quality::Crude },
            -8 => if roll < 80.0 { Quality::Pathetic } else { Quality::Crude },
            -7 => if roll < 70.0 { Quality::Pathetic } else { Quality::Crude },
            -6 => if roll < 60.0 { Quality::Pathetic } else { Quality::Crude },
            -5 => if roll < 40.0 { Quality::Pathetic } else { Quality::Crude },
            -4 => {
                if roll < 20.0 { Quality::Pathetic }
                else if roll < 90.0 { Quality::Crude }
                else { Quality::Basic }
            }
            -3 => if roll < 80.0 { Quality::Crude } else { Quality::Basic },
            -2 => if roll < 70.0 { Quality::Crude } else { Quality::Basic },
            -1 => if roll < 60.0 { Quality::Crude } else { Quality::Basic },
            0 => {
                if roll < 40.0 { Quality::Crude }
                else if roll < 90.0 { Quality::Basic }
                else { Quality::Moderate }
            }
            1 => {
                if roll < 20.0 { Quality::Crude }
                else if roll < 80.0 { Quality::Basic }
                else { Quality::Moderate }
            }
            2 => if roll < 70.0 { Quality::Basic } else { Quality::Moderate },
            3 => if roll < 60.0 { Quality::Basic } else { Quality::Moderate },
            4 => if roll < 50.0 { Quality::Basic } else { Quality::Moderate },
            5 => if roll < 40.0 { Quality::Basic } else { Quality::Moderate },
            6 => {
                if roll < 20.0 { Quality::Basic }
                else if roll < 90.0 { Quality::Moderate }
                else { Quality::Advanced }
            }
            7 => if roll < 80.0 { Quality::Moderate } else { Quality::Advanced },
            8 => if roll < 60.0 { Quality::Moderate } else { Quality::Advanced },
            9 => if roll < 40.0 { Quality::Moderate } else { Quality::Advanced },
            10 => {
                if roll < 10.0 { Quality::Moderate }
                else if roll < 90.0 { Quality::Advanced }
                else { Quality::Expert }
            }
            _ => Quality::Pathetic,
        }
    }

    /// Gain experience from successful completion
    pub fn gain_experience(&mut self, amount: u32) {
        self.experience += amount;

        // Level up if enough experience (simple: 100 exp per level)
        while self.experience >= 100 && self.level < 10 {
            self.experience -= 100;
            self.level += 1;
        }
    }

    /// Get progress to next level (0.0 to 1.0)
    pub fn progress_to_next_level(&self) -> f32 {
        if self.level >= 10 {
            1.0
        } else {
            self.experience as f32 / 100.0
        }
    }
}

/// Collection of all agent skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    skills: HashMap<SkillType, Skill>,
}

impl Skills {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Get a skill (creates if doesn't exist)
    pub fn get_skill(&mut self, skill_type: SkillType) -> &Skill {
        self.skills.entry(skill_type).or_insert_with(|| Skill::new(skill_type))
    }

    /// Get mutable skill (creates if doesn't exist)
    pub fn get_skill_mut(&mut self, skill_type: SkillType) -> &mut Skill {
        self.skills.entry(skill_type).or_insert_with(|| Skill::new(skill_type))
    }

    /// Get skill if it exists
    pub fn get_skill_if_exists(&self, skill_type: SkillType) -> Option<&Skill> {
        self.skills.get(&skill_type)
    }

    /// Set skill level
    pub fn set_skill_level(&mut self, skill_type: SkillType, level: i32) {
        self.skills.insert(skill_type, Skill::with_level(skill_type, level));
    }

    /// Perform skill check
    pub fn perform_check(&mut self, skill_type: SkillType) -> SkillCheckResult {
        let skill = self.get_skill_mut(skill_type);
        skill.perform_check()
    }

    /// Gain experience in a skill
    pub fn gain_experience(&mut self, skill_type: SkillType, amount: u32) {
        let skill = self.get_skill_mut(skill_type);
        skill.gain_experience(amount);
    }

    /// Get all skills
    pub fn get_all_skills(&self) -> &HashMap<SkillType, Skill> {
        &self.skills
    }

    /// Get skills by category
    pub fn get_skills_by_category(&self, category: SkillCategory) -> Vec<&Skill> {
        self.skills.values()
            .filter(|s| s.category() == category)
            .collect()
    }

    /// Get highest skill
    pub fn highest_skill(&self) -> Option<&Skill> {
        self.skills.values().max_by_key(|s| s.level)
    }

    /// Get average skill level
    pub fn average_skill_level(&self) -> f32 {
        if self.skills.is_empty() {
            -10.0
        } else {
            let sum: i32 = self.skills.values().map(|s| s.level).sum();
            sum as f32 / self.skills.len() as f32
        }
    }
}

impl Default for Skills {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new(SkillType::Mining);
        assert_eq!(skill.level, -10);
        assert_eq!(skill.category(), SkillCategory::None);
        assert_eq!(skill.title(), None);
    }

    #[test]
    fn test_skill_categories() {
        assert_eq!(Skill::with_level(SkillType::Mining, -10).category(), SkillCategory::None);
        assert_eq!(Skill::with_level(SkillType::Mining, -5).category(), SkillCategory::Low);
        assert_eq!(Skill::with_level(SkillType::Mining, 0).category(), SkillCategory::Medium);
        assert_eq!(Skill::with_level(SkillType::Mining, 6).category(), SkillCategory::High);
    }

    #[test]
    fn test_skill_titles() {
        assert_eq!(Skill::with_level(SkillType::Mining, -10).title(), None);
        assert_eq!(Skill::with_level(SkillType::Mining, -5).title(), Some("Apprentice"));
        assert_eq!(Skill::with_level(SkillType::Mining, 0).title(), Some("Journeyman"));
        assert_eq!(Skill::with_level(SkillType::Mining, 6).title(), Some("Master"));
    }

    #[test]
    fn test_speed_multiplier() {
        assert_eq!(Skill::with_level(SkillType::Mining, -10).speed_multiplier(), 0.5);
        assert_eq!(Skill::with_level(SkillType::Mining, 0).speed_multiplier(), 1.0);
        assert_eq!(Skill::with_level(SkillType::Mining, 10).speed_multiplier(), 1.5);
    }

    #[test]
    fn test_skill_progression() {
        let mut skill = Skill::new(SkillType::Mining);
        assert_eq!(skill.level, -10);

        skill.gain_experience(100);
        assert_eq!(skill.level, -9);

        skill.gain_experience(500);
        assert_eq!(skill.level, -4);
    }

    #[test]
    fn test_quality_modifier() {
        assert_eq!(Quality::Pathetic.modifier(), 0.5);
        assert_eq!(Quality::Basic.modifier(), 1.0);
        assert_eq!(Quality::Expert.modifier(), 2.0);
    }

    #[test]
    fn test_skill_check() {
        let skill = Skill::with_level(SkillType::Mining, 10);
        let result = skill.perform_check();

        // Master should always succeed
        assert!(result.success);
        assert!(result.quality.is_some());
        assert_eq!(result.speed_multiplier, 1.5);
    }

    #[test]
    fn test_skills_collection() {
        let mut skills = Skills::new();

        skills.set_skill_level(SkillType::Mining, 5);
        skills.set_skill_level(SkillType::Woodcutting, -2);

        assert_eq!(skills.get_skill_if_exists(SkillType::Mining).unwrap().level, 5);
        assert_eq!(skills.get_skill_if_exists(SkillType::Woodcutting).unwrap().level, -2);
    }

    #[test]
    fn test_highest_skill() {
        let mut skills = Skills::new();

        skills.set_skill_level(SkillType::Mining, 3);
        skills.set_skill_level(SkillType::Woodcutting, 7);
        skills.set_skill_level(SkillType::Crafting, -2);

        let highest = skills.highest_skill().unwrap();
        assert_eq!(highest.skill_type, SkillType::Woodcutting);
        assert_eq!(highest.level, 7);
    }

    #[test]
    fn test_average_skill_level() {
        let mut skills = Skills::new();

        skills.set_skill_level(SkillType::Mining, 0);
        skills.set_skill_level(SkillType::Woodcutting, 10);
        skills.set_skill_level(SkillType::Crafting, -10);

        assert_eq!(skills.average_skill_level(), 0.0);
    }
}
