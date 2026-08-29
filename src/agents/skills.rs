// src/agents/skills.rs
//! Skill system for agent proficiency and progression.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use rand::Rng;

/// Types of skills agents can develop
/// Ordered, so that a map keyed by it is iterated the same way twice - see
/// `Skills::skills`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
    /// The quality a given pair of hands turns out.
    ///
    /// `hand` is the multiplier from [`Skill::hand`]: 0.5 for the clumsiest
    /// possible, 1.25 for an untrained adult, 2.0 for the best there is. The
    /// bands are set so that a founder - who arrives four or five levels
    /// below untrained, having lived a life but no more than that - makes
    /// crude things. None of them are experts at making anything.
    pub fn from_hand(hand: f32) -> Self {
        match hand {
            h if h < 0.8 => Quality::Pathetic,
            h if h < 1.1 => Quality::Crude,
            h if h < 1.4 => Quality::Basic,
            h if h < 1.65 => Quality::Moderate,
            h if h < 1.85 => Quality::Advanced,
            _ => Quality::Expert,
        }
    }

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

    /// Get value multiplier for trade/comparison purposes
    pub fn value_multiplier(&self) -> f32 {
        match self {
            Quality::Pathetic => 0.3,
            Quality::Crude => 0.6,
            Quality::Basic => 1.0,
            Quality::Moderate => 1.5,
            Quality::Advanced => 2.5,
            Quality::Expert => 4.0,
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
    /// The tick this was last actually practised
    #[serde(default)]
    pub last_used: u32,
}

impl Skill {
    pub fn new(skill_type: SkillType) -> Self {
        Self {
            skill_type,
            level: -10, // Start at lowest level
            experience: 0,
            last_used: 0,
        }
    }

    /// Create skill with specific level
    pub fn with_level(skill_type: SkillType, level: i32) -> Self {
        Self {
            skill_type,
            level: level.clamp(-10, 10),
            experience: 0,
            last_used: 0,
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

    /// What a hand of this practice is worth at the work, against an ordinary
    /// one.
    ///
    /// Half at the bottom, double at the top, and one in the middle of the
    /// range. This is the number that makes a trade worth having: it is what
    /// comes off a field per trip, what a piece of work is worth, how much of
    /// the material is not wasted. Without it a lifetime at a trade bought
    /// nothing at all, because `speed_multiplier`, `perform_check` and
    /// `determine_quality` were built and had no callers anywhere.
    pub fn hand(&self) -> f32 {
        const CLUMSIEST: f32 = 0.5;
        const BEST: f32 = 2.0;

        let along = (self.level + 10) as f32 / 20.0;
        CLUMSIEST + (BEST - CLUMSIEST) * along
    }

    /// Perform skill check with optional tool quality
    pub fn perform_check(&self, tool_quality: Option<Quality>) -> SkillCheckResult {
        let mut rng = crate::core::dice::roll();
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
        let roll = crate::core::dice::roll().gen::<f32>() * 100.0;

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

    /// What the first step up costs, from knowing nothing at all
    pub const FIRST_STEP: u32 = 40;

    /// And how much more each step costs than the one below it.
    ///
    /// Sized against how many times an agent actually does anything. Measured
    /// over fourteen worlds, the busiest trade in a settlement is gathering at
    /// something like two hundred and fifty goes in a working life, and most
    /// trades far fewer. A curve steep enough to be interesting has to fit
    /// inside that: at these numbers somebody who gives a life to one trade
    /// finishes near the top of it, somebody who dabbles at a third of the
    /// rate finishes near the bottom, and the last step still costs nearly
    /// five times the first.
    pub const STEEPER_EACH_TIME: u32 = 8;

    /// What it costs to climb one level from here.
    ///
    /// A flat hundred at every level meant the twenty steps from raw beginner
    /// to master cost the same at the top as at the bottom, and anybody who
    /// touched a trade ran to the ceiling of it. Getting the hang of something
    /// is quick and getting good at it is not: the first step costs thirty and
    /// the last costs five hundred, so the whole climb is about five and a half
    /// thousand - a life's work at one trade, and out of reach for somebody
    /// splitting their days across eight.
    pub fn experience_for_next_level(level: i32) -> u32 {
        let steps_taken = (level + 10).max(0) as u32;
        Self::FIRST_STEP + Self::STEEPER_EACH_TIME * steps_taken
    }

    /// Gain experience from successful completion
    pub fn gain_experience(&mut self, amount: u32) {
        self.experience += amount;

        while self.level < 10 {
            let wanted = Self::experience_for_next_level(self.level);
            if self.experience < wanted {
                break;
            }
            self.experience -= wanted;
            self.level += 1;
        }

        // At the ceiling there is nothing left to bank towards
        if self.level >= 10 {
            self.experience = 0;
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
    /// What this one can do, in a stable order.
    ///
    /// Iterated to find a best hand and to rust what is unused, so hash order
    /// decided ties and made the same agent behave differently between runs.
    skills: std::collections::BTreeMap<SkillType, Skill>,
}

impl Skills {
    pub fn new() -> Self {
        Self {
            skills: std::collections::BTreeMap::new(),
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

    /// What this agent's hand is worth at a trade, against an ordinary one.
    ///
    /// Half for somebody who has never done it, double for a master. Somebody
    /// who has not touched the skill at all counts as never having done it.
    pub fn hand_for(&self, skill_type: SkillType) -> f32 {
        self.get_skill_if_exists(skill_type)
            .map(|skill| skill.hand())
            .unwrap_or_else(|| Skill::new(skill_type).hand())
    }

    /// Practise a skill at a given tick, so that it is known to be in use.
    ///
    /// The same as gaining experience, and additionally the thing that keeps a
    /// trade from rusting - see [`Self::let_unused_skills_rust`].
    pub fn practise(&mut self, skill_type: SkillType, amount: u32, current_tick: u32) {
        let skill = self.get_skill_mut(skill_type);
        skill.gain_experience(amount);
        skill.last_used = current_tick;
    }

    /// How long a hand keeps its trade before it starts to go.
    ///
    /// A year of not doing the work at all. Somebody who farms in the spring
    /// and does other things all summer does not forget how to farm.
    pub const KEEPS_FOR: u32 = 1_152;

    /// And how long a level lasts after that.
    pub const LOSES_A_LEVEL_EVERY: u32 = 576;

    /// How far a trade can fall away.
    ///
    /// Not to nothing. Somebody who spent years at a trade and then left it
    /// is worse than they were and better than somebody who never did it, and
    /// stays that way - which is also what makes coming back to a trade
    /// cheaper than starting one.
    pub const NEVER_QUITE_FORGOTTEN: i32 = -5;

    /// Let go of what has not been done in a long time.
    ///
    /// Nothing took a skill back before this, so a long-lived agent could hold
    /// every trade at once and being a generalist cost nothing at all. It is
    /// the other half of making mastery expensive: the climb is long enough
    /// that only a specialist finishes it, and this is what stops somebody
    /// finishing all eight climbs one after another over a long life.
    pub fn let_unused_skills_rust(&mut self, current_tick: u32) {
        for skill in self.skills.values_mut() {
            if skill.level <= Self::NEVER_QUITE_FORGOTTEN {
                continue;
            }

            let idle = current_tick.saturating_sub(skill.last_used);
            if idle < Self::KEEPS_FOR {
                continue;
            }

            let gone = ((idle - Self::KEEPS_FOR) / Self::LOSES_A_LEVEL_EVERY) as i32;
            if gone == 0 {
                continue;
            }

            let was = skill.level;
            skill.level = (skill.level - 1).max(Self::NEVER_QUITE_FORGOTTEN);

            if skill.level != was {
                // Whatever was banked towards the next level goes with it, and
                // the clock restarts so the next level takes as long again
                skill.experience = 0;
                skill.last_used = current_tick.saturating_sub(Self::KEEPS_FOR);
            }
        }
    }

    /// Get all skills
    pub fn get_all_skills(&self) -> &std::collections::BTreeMap<SkillType, Skill> {
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
        // A level used to cost a flat hundred wherever you stood, so the last
        // step from journeyman to master was as cheap as the first away from
        // knowing nothing, and anybody who touched a trade ran to the ceiling
        // of it. It costs more the higher it goes now.
        let mut skill = Skill::new(SkillType::Mining);
        assert_eq!(skill.level, -10);

        skill.gain_experience(Skill::experience_for_next_level(-10));
        assert_eq!(skill.level, -9);

        // The same experience again buys less of a climb the second time
        let first_five: u32 = (-9..-4).map(Skill::experience_for_next_level).sum();
        skill.gain_experience(first_five);
        assert_eq!(skill.level, -4);

        let next_five: u32 = (-4..1).map(Skill::experience_for_next_level).sum();
        assert!(
            next_five > first_five,
            "the five levels after should cost more than the five before: \
             {next_five} against {first_five}"
        );
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
