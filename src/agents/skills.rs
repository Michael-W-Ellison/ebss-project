// src/agents/skills.rs
//! Skill system for agent proficiency and progression.
//!
//! Configuration values can be loaded from `config/default.toml` or customized
//! via the `GameConfig` system. See `crate::config` for details.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;
use crate::config::GameConfig;

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
    /// Social interaction and persuasion
    Social,
    /// Navigation and wayfinding
    Navigation,
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
            SkillType::Social => "Social",
            SkillType::Navigation => "Navigation",
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

    /// Get tool durability modifier
    pub fn tool_durability_modifier(&self) -> f32 {
        match self {
            Quality::Pathetic => 0.5,  // -50%
            Quality::Crude => 0.75,     // -25%
            Quality::Basic => 1.0,      // default
            Quality::Moderate => 1.1,   // +10%
            Quality::Advanced => 1.25,  // +25%
            Quality::Expert => 1.5,     // +50%
        }
    }

    /// Get tool speed modifier
    pub fn tool_speed_modifier(&self) -> f32 {
        match self {
            Quality::Pathetic => 1.0,
            Quality::Crude => 1.0,
            Quality::Basic => 1.0,
            Quality::Moderate => 1.1,   // +10%
            Quality::Advanced => 1.25,  // +25%
            Quality::Expert => 1.5,     // +50%
        }
    }

    /// Get number of injury/failure rolls for tool quality
    pub fn tool_risk_roll_count(&self) -> u8 {
        match self {
            Quality::Pathetic => 3,  // Roll 3 times (more danger)
            Quality::Crude => 2,     // Roll 2 times
            _ => 1,                   // Normal single roll
        }
    }

    /// Get maximum output quality limit for materials
    pub fn material_quality_limit(&self) -> Quality {
        match self {
            Quality::Pathetic => Quality::Crude,
            Quality::Crude => Quality::Basic,
            Quality::Basic => Quality::Moderate,
            Quality::Moderate => Quality::Advanced,
            Quality::Advanced => Quality::Expert,
            Quality::Expert => Quality::Expert,
        }
    }

    /// Get drive satisfaction modifier for material/product quality
    pub fn drive_satisfaction_modifier(&self) -> f32 {
        match self {
            Quality::Pathetic => 0.5,   // -50%
            Quality::Crude => 0.75,      // -25%
            Quality::Basic => 1.0,       // normal
            Quality::Moderate => 1.0,    // normal
            Quality::Advanced => 1.1,    // +10%
            Quality::Expert => 1.25,     // +25%
        }
    }

    /// Get bonus chance for Expert quality when using Expert materials
    pub fn expert_output_bonus(&self) -> f32 {
        match self {
            Quality::Expert => 0.1,  // +10% bonus to Expert rolls
            _ => 0.0,
        }
    }

    /// Limit quality to material's maximum
    pub fn limit_to_material(&self, material_quality: Quality) -> Quality {
        let max_quality = material_quality.material_quality_limit();
        if *self > max_quality {
            max_quality
        } else {
            *self
        }
    }

    /// Get minimum skill level required for 50% success rate at this quality
    pub fn min_skill_level_for_repair(&self) -> i32 {
        match self {
            Quality::Pathetic => -9,   // 90% chance at -9
            Quality::Crude => -7,      // 70% chance at -7
            Quality::Basic => -5,      // 60% chance at -5
            Quality::Moderate => -1,   // 50% chance at -1
            Quality::Advanced => 3,    // 50% chance at 3
            Quality::Expert => 7,      // 50% chance at 7
        }
    }

    /// Downgrade quality by N levels (for recycling)
    pub fn downgrade(&self, levels: u8) -> Quality {
        let current_level = *self as i32;
        let new_level = (current_level - levels as i32).max(0);
        match new_level {
            0 => Quality::Pathetic,
            1 => Quality::Crude,
            2 => Quality::Basic,
            3 => Quality::Moderate,
            4 => Quality::Advanced,
            _ => Quality::Expert,
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

/// Result of a repair attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairResult {
    pub success: bool,
    pub experience_gained: f32,  // 0.5 for successful repair
    pub speed_multiplier: f32,
}

/// Material returned from recycling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycledMaterial {
    pub material_id: String,
    pub quantity: u32,
    pub quality: Quality,
}

/// Result of recycling an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycleResult {
    pub materials: Vec<RecycledMaterial>,
    pub return_percentage: f32,  // 0.2, 0.5, or 0.75
    pub quality_downgrade: u8,    // 0, 1, or 2
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

    /// Perform skill check with optional tool quality
    pub fn perform_check(&self, tool_quality: Option<Quality>) -> SkillCheckResult {
        let mut rng = rand::thread_rng();
        let category = self.category();

        // Determine number of rolls based on tool quality
        let roll_count = tool_quality
            .map(|q| q.tool_risk_roll_count())
            .unwrap_or(1);

        // Check for injury (multiple rolls for bad tools increase risk)
        let mut injury = None;
        for _ in 0..roll_count {
            if rng.gen::<f32>() < category.large_injury_chance() {
                injury = Some(InjuryType::Large);
                break;
            } else if rng.gen::<f32>() < category.small_injury_chance() && injury.is_none() {
                injury = Some(InjuryType::Small);
            }
        }

        // Check for failure (multiple rolls for bad tools increase risk)
        let mut success = true;
        for _ in 0..roll_count {
            if rng.gen::<f32>() < category.failure_chance() {
                success = false;
                break;
            }
        }

        // Determine quality if successful
        let quality = if success {
            Some(self.determine_quality())
        } else {
            None
        };

        // Calculate speed multiplier (skill + tool quality)
        let base_speed = self.speed_multiplier();
        let tool_speed = tool_quality
            .map(|q| q.tool_speed_modifier())
            .unwrap_or(1.0);
        let speed_multiplier = base_speed * tool_speed;

        SkillCheckResult {
            success,
            quality,
            injury,
            speed_multiplier,
        }
    }

    /// Perform skill check without tool (backwards compatibility)
    pub fn perform_check_no_tool(&self) -> SkillCheckResult {
        self.perform_check(None)
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

    /// Get skill configuration from global config or use defaults.
    fn get_skill_config() -> crate::config::learning::SkillProgressionConfig {
        GameConfig::try_global()
            .map(|c| c.learning.skills.clone())
            .unwrap_or_default()
    }

    /// Calculate XP required for the next level based on current level.
    fn xp_for_next_level(&self) -> u32 {
        let config = Self::get_skill_config();
        let base_xp = config.base_xp_per_level;
        let scaling = config.xp_scaling_factor;
        // Scale XP requirement: higher levels need more XP
        // Level -10 to 0: base_xp, Level 1+: base_xp * scaling^level
        if self.level < 0 {
            base_xp
        } else {
            (base_xp as f32 * scaling.powi(self.level)) as u32
        }
    }

    /// Gain experience from successful completion
    pub fn gain_experience(&mut self, amount: u32) {
        let config = Self::get_skill_config();
        let max_level = config.max_level as i32;

        self.experience += amount;

        // Level up if enough experience
        let mut xp_needed = self.xp_for_next_level();
        while self.experience >= xp_needed && self.level < max_level {
            self.experience -= xp_needed;
            self.level += 1;
            xp_needed = self.xp_for_next_level();
        }
    }

    /// Get progress to next level (0.0 to 1.0)
    pub fn progress_to_next_level(&self) -> f32 {
        let config = Self::get_skill_config();
        if self.level >= config.max_level as i32 {
            1.0
        } else {
            self.experience as f32 / self.xp_for_next_level() as f32
        }
    }

    /// Check if agent can repair an item of given quality
    pub fn can_repair(&self, item_quality: Quality) -> bool {
        self.level >= item_quality.min_skill_level_for_repair()
    }

    /// Perform repair (100% success, no injury, 0.5 exp gain)
    pub fn perform_repair(&mut self, item_quality: Quality) -> RepairResult {
        if !self.can_repair(item_quality) {
            return RepairResult {
                success: false,
                experience_gained: 0.0,
                speed_multiplier: self.speed_multiplier(),
            };
        }

        // Successful repair
        let exp_gain = 0.5;
        self.gain_experience((exp_gain * 100.0) as u32); // Convert to integer exp points

        RepairResult {
            success: true,
            experience_gained: exp_gain,
            speed_multiplier: self.speed_multiplier(),
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

    /// Perform skill check with optional tool quality
    pub fn perform_check(&mut self, skill_type: SkillType, tool_quality: Option<Quality>) -> SkillCheckResult {
        let skill = self.get_skill_mut(skill_type);
        skill.perform_check(tool_quality)
    }

    /// Perform skill check without tool
    pub fn perform_check_no_tool(&mut self, skill_type: SkillType) -> SkillCheckResult {
        let skill = self.get_skill_mut(skill_type);
        skill.perform_check_no_tool()
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

    /// Check if agent can repair an item of given quality with given skill
    pub fn can_repair(&self, skill_type: SkillType, item_quality: Quality) -> bool {
        self.get_skill_if_exists(skill_type)
            .map(|s| s.can_repair(item_quality))
            .unwrap_or(false)
    }

    /// Perform repair for an item
    pub fn perform_repair(&mut self, skill_type: SkillType, item_quality: Quality) -> RepairResult {
        let skill = self.get_skill_mut(skill_type);
        skill.perform_repair(item_quality)
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
        let result = skill.perform_check(None);

        // Master should always succeed
        assert!(result.success);
        assert!(result.quality.is_some());
        assert_eq!(result.speed_multiplier, 1.5);
    }

    #[test]
    fn test_tool_quality_speed_bonus() {
        let skill = Skill::with_level(SkillType::Mining, 0);

        // Basic tool: no bonus
        let result_basic = skill.perform_check(Some(Quality::Basic));
        assert_eq!(result_basic.speed_multiplier, 1.0);

        // Expert tool: +50% bonus
        let result_expert = skill.perform_check(Some(Quality::Expert));
        assert_eq!(result_expert.speed_multiplier, 1.5);
    }

    #[test]
    fn test_quality_durability_modifiers() {
        assert_eq!(Quality::Pathetic.tool_durability_modifier(), 0.5);
        assert_eq!(Quality::Basic.tool_durability_modifier(), 1.0);
        assert_eq!(Quality::Expert.tool_durability_modifier(), 1.5);
    }

    #[test]
    fn test_quality_drive_satisfaction() {
        assert_eq!(Quality::Pathetic.drive_satisfaction_modifier(), 0.5);
        assert_eq!(Quality::Basic.drive_satisfaction_modifier(), 1.0);
        assert_eq!(Quality::Expert.drive_satisfaction_modifier(), 1.25);
    }

    #[test]
    fn test_material_quality_limits() {
        assert_eq!(Quality::Pathetic.material_quality_limit(), Quality::Crude);
        assert_eq!(Quality::Crude.material_quality_limit(), Quality::Basic);
        assert_eq!(Quality::Expert.material_quality_limit(), Quality::Expert);
    }

    #[test]
    fn test_quality_limiting() {
        let output_quality = Quality::Advanced;

        // Pathetic material limits to Crude
        assert_eq!(output_quality.limit_to_material(Quality::Pathetic), Quality::Crude);

        // Expert material doesn't limit
        assert_eq!(output_quality.limit_to_material(Quality::Expert), Quality::Advanced);
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

    #[test]
    fn test_quality_downgrade() {
        assert_eq!(Quality::Expert.downgrade(0), Quality::Expert);
        assert_eq!(Quality::Expert.downgrade(1), Quality::Advanced);
        assert_eq!(Quality::Expert.downgrade(2), Quality::Moderate);
        assert_eq!(Quality::Advanced.downgrade(1), Quality::Moderate);
        assert_eq!(Quality::Crude.downgrade(1), Quality::Pathetic);
        assert_eq!(Quality::Pathetic.downgrade(1), Quality::Pathetic); // Can't go lower
    }

    #[test]
    fn test_min_skill_level_for_repair() {
        assert_eq!(Quality::Pathetic.min_skill_level_for_repair(), -9);
        assert_eq!(Quality::Crude.min_skill_level_for_repair(), -7);
        assert_eq!(Quality::Basic.min_skill_level_for_repair(), -5);
        assert_eq!(Quality::Moderate.min_skill_level_for_repair(), -1);
        assert_eq!(Quality::Advanced.min_skill_level_for_repair(), 3);
        assert_eq!(Quality::Expert.min_skill_level_for_repair(), 7);
    }

    #[test]
    fn test_can_repair() {
        let skill_low = Skill::with_level(SkillType::Crafting, -8);
        let skill_high = Skill::with_level(SkillType::Crafting, 5);

        // Low skill (-8) can repair Pathetic (-9) but not Crude (-7 required)
        assert!(skill_low.can_repair(Quality::Pathetic));
        assert!(!skill_low.can_repair(Quality::Crude)); // Requires -7, have -8
        assert!(!skill_low.can_repair(Quality::Basic));
        assert!(!skill_low.can_repair(Quality::Moderate));

        // High skill (5) can repair everything up to Advanced but not Expert
        assert!(skill_high.can_repair(Quality::Pathetic));
        assert!(skill_high.can_repair(Quality::Crude));
        assert!(skill_high.can_repair(Quality::Basic));
        assert!(skill_high.can_repair(Quality::Moderate));
        assert!(skill_high.can_repair(Quality::Advanced)); // Requires 3, have 5
        assert!(!skill_high.can_repair(Quality::Expert)); // Requires 7, have 5
    }

    #[test]
    fn test_perform_repair() {
        let mut skill = Skill::with_level(SkillType::Crafting, 5);

        // Can repair Moderate quality
        let result = skill.perform_repair(Quality::Moderate);
        assert!(result.success);
        assert_eq!(result.experience_gained, 0.5);

        // Cannot repair Expert quality (requires level 7)
        let result_fail = skill.perform_repair(Quality::Expert);
        assert!(!result_fail.success);
        assert_eq!(result_fail.experience_gained, 0.0);
    }

    #[test]
    fn test_repair_exp_gain() {
        let mut skill = Skill::with_level(SkillType::Crafting, -5);
        let initial_exp = skill.experience;

        // Perform repair (0.5 exp = 50 exp points)
        skill.perform_repair(Quality::Basic);

        assert_eq!(skill.experience, initial_exp + 50);
    }
}
