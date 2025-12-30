// src/agents/profession.rs
//! Agent profession and job system.
//!
//! This module defines the profession system that allows agents to specialize in
//! specific roles, develop skills, and be assigned to workplaces.
//!
//! # Integration Status
//!
//! **IMPORTANT**: This profession system is currently **NOT INTEGRATED** with the Agent struct.
//! The 52 job types defined here are available for future use, but agents do not currently
//! have a `profession` field. The crafting/production system uses skill-based mechanics
//! in `src/world/crafting.rs` instead.
//!
//! ## Usage Status
//!
//! - `JobType` enum: Defined but not actively used outside this module
//! - `Profession` struct: Complete but not attached to agents
//! - `workplace()` method: Ready for building assignment integration
//! - `tick_production()`: Deprecated, replaced by skill-based crafting
//!
//! ## Implementation Priority Guide
//!
//! Jobs are classified by implementation priority for future integration:
//!
//! | Priority | Category | Jobs |
//! |----------|----------|------|
//! | 1 (Core) | Essential survival | Farmer, Woodcutter, Miner, Hunter, Fisher, Blacksmith, Carpenter, Baker, Healer, Laborer |
//! | 2 (Important) | Secondary production | Miller, Butcher, Tanner, Tailor, Stonemason, Merchant, Cook, Herder, Brewer |
//! | 3 (Advanced) | Specialized crafts | Armorer, Potter, Weaver, Cobbler, Bowyer, Fletcher, Apothecary, Watchman |
//! | 4 (Luxury) | Non-essential | Goldsmith, Painter, Candlemaker, Birdcatcher, Glassblower, Dyer, etc. |
//!
//! ## Path to Integration
//!
//! To integrate professions with agents:
//! 1. Add `profession: Option<Profession>` field to Agent struct
//! 2. Update agent spawn logic to assign professions based on settlement needs
//! 3. Connect `workplace()` with building assignment system
//! 4. Integrate profession skills with existing crafting system

use serde::{Deserialize, Serialize};
use crate::world::{Position, BuildingType};
use uuid::Uuid;

/// Types of jobs/professions agents can have
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobType {
    // === Primary Production (Resource Gathering) ===
    /// Gathers wood from forests
    Woodcutter,
    /// Mines stone and ore
    Miner,
    /// Grows crops and manages fields
    Farmer,
    /// Catches fish
    Fisher,
    /// Hunts wild animals
    Hunter,
    /// Manages livestock
    Herder,
    /// Collects herbs and plants
    Herbalist,

    // === Food Processing ===
    /// Grinds grain into flour
    Miller,
    /// Processes meat from animals
    Butcher,
    /// Bakes bread and other goods
    Baker,
    /// Brews ale and beer
    Brewer,
    /// Makes cheese and butter
    Cheesemaker,

    // === Material Processing ===
    /// Processes hides into leather
    Tanner,
    /// Creates pottery and ceramics
    Potter,
    /// Weaves cloth from fibers
    Weaver,
    /// Spins thread from raw fibers
    Spinner,
    /// Creates glass items
    Glassblower,
    /// Dyes cloth and materials
    Dyer,
    /// Makes rope and cordage
    Ropemaker,
    /// Makes bricks
    Brickmaker,
    /// Produces charcoal
    CharcoalMaker,

    // === Crafting (Wood & Stone) ===
    /// Works with wood, makes furniture and tools
    Carpenter,
    /// Cuts stone blocks
    Stonemason,
    /// Saws lumber
    Sawyer,
    /// Creates turned wood items
    Turner,

    // === Metalworking ===
    /// Basic metalworking
    Blacksmith,
    /// Makes armor
    Armorer,
    /// Works with precious metals
    Goldsmith,
    /// Works with tin
    Tinsmith,

    // === Textile & Leather Goods ===
    /// Makes clothing
    Tailor,
    /// Makes shoes
    Cobbler,
    /// Works leather goods
    Leatherworker,

    // === Weapons & Tools ===
    /// Makes bows
    Bowyer,
    /// Makes arrows
    Fletcher,
    /// Makes wheels
    Wheelwright,

    // === Services ===
    /// Heals the sick and wounded
    Healer,
    /// Prepares medicines
    Apothecary,
    /// Cuts hair and performs minor surgery
    Barber,
    /// Writes documents
    Scribe,
    /// Prints books
    Printer,
    /// Makes paper/parchment
    Papermaker,

    // === Commerce & Administration ===
    /// Trades goods
    Merchant,
    /// Guards the settlement
    Watchman,
    /// Makes public announcements
    TownCrier,

    // === Specialized ===
    /// Cooks food
    Cook,
    /// Keeps bees for honey
    Beekeeper,
    /// Catches birds
    Birdcatcher,
    /// Paints and decorates
    Painter,
    /// Makes candles
    Candlemaker,
    /// Transports goods
    Carter,

    // === Unemployed/General ===
    /// No specific job, performs general labor
    Laborer,
    /// Not yet assigned a job
    Unemployed,
}

/// Array of all job types for iteration and filtering.
pub const ALL_JOB_TYPES: [JobType; 52] = [
    // Primary Production
    JobType::Woodcutter, JobType::Miner, JobType::Farmer, JobType::Fisher,
    JobType::Hunter, JobType::Herder, JobType::Herbalist,
    // Food Processing
    JobType::Miller, JobType::Butcher, JobType::Baker, JobType::Brewer, JobType::Cheesemaker,
    // Material Processing
    JobType::Tanner, JobType::Potter, JobType::Weaver, JobType::Spinner,
    JobType::Glassblower, JobType::Dyer, JobType::Ropemaker, JobType::Brickmaker, JobType::CharcoalMaker,
    // Crafting (Wood & Stone)
    JobType::Carpenter, JobType::Stonemason, JobType::Sawyer, JobType::Turner,
    // Metalworking
    JobType::Blacksmith, JobType::Armorer, JobType::Goldsmith, JobType::Tinsmith,
    // Textile & Leather Goods
    JobType::Tailor, JobType::Cobbler, JobType::Leatherworker,
    // Weapons & Tools
    JobType::Bowyer, JobType::Fletcher, JobType::Wheelwright,
    // Services
    JobType::Healer, JobType::Apothecary, JobType::Barber, JobType::Scribe, JobType::Printer, JobType::Papermaker,
    // Commerce & Administration
    JobType::Merchant, JobType::Watchman, JobType::TownCrier,
    // Specialized
    JobType::Cook, JobType::Beekeeper, JobType::Birdcatcher, JobType::Painter, JobType::Candlemaker, JobType::Carter,
    // General
    JobType::Laborer, JobType::Unemployed,
];

impl JobType {
    /// Get the building type where this job is performed (if any)
    pub fn workplace(&self) -> Option<BuildingType> {
        match self {
            // Food processing
            JobType::Miller => Some(BuildingType::Mill),
            JobType::Butcher => Some(BuildingType::Butchery),
            JobType::Baker => Some(BuildingType::Bakery),
            JobType::Brewer => Some(BuildingType::Brewery),
            JobType::Cheesemaker => Some(BuildingType::Dairy),

            // Material processing
            JobType::Tanner => Some(BuildingType::Tannery),
            JobType::Potter => Some(BuildingType::PotteryKiln),
            JobType::Weaver | JobType::Spinner => Some(BuildingType::WeaverHut),
            JobType::Glassblower => Some(BuildingType::Glassworks),
            JobType::Dyer => Some(BuildingType::Dyeworks),
            JobType::Ropemaker => Some(BuildingType::Ropewalk),
            JobType::Brickmaker => Some(BuildingType::Brickyard),

            // Crafting
            JobType::Carpenter | JobType::Sawyer | JobType::Turner |
            JobType::Bowyer | JobType::Fletcher | JobType::Wheelwright => Some(BuildingType::Workshop),
            JobType::Stonemason => Some(BuildingType::Workshop),

            // Metalworking
            JobType::Blacksmith => Some(BuildingType::Forge),
            JobType::Armorer | JobType::Goldsmith | JobType::Tinsmith => Some(BuildingType::Smithy),

            // Textile & leather goods
            JobType::Tailor => Some(BuildingType::TailorShop),
            JobType::Cobbler => Some(BuildingType::CobblerShop),
            JobType::Leatherworker => Some(BuildingType::Tannery),

            // Services
            JobType::Healer | JobType::Apothecary => Some(BuildingType::MedicalBuilding),
            JobType::Barber => Some(BuildingType::BarberShop),
            JobType::Scribe | JobType::Printer => Some(BuildingType::Scriptorium),
            JobType::Papermaker => Some(BuildingType::PaperMill),

            // Commerce
            JobType::Merchant => Some(BuildingType::TownStorage),
            JobType::Watchman => Some(BuildingType::GuardPost),

            // Primary production & general - outdoor work
            JobType::Woodcutter | JobType::Miner | JobType::Farmer |
            JobType::Fisher | JobType::Hunter | JobType::Herder |
            JobType::Herbalist | JobType::Beekeeper | JobType::Birdcatcher |
            JobType::CharcoalMaker | JobType::Cook | JobType::Painter |
            JobType::Candlemaker | JobType::Carter | JobType::TownCrier |
            JobType::Laborer | JobType::Unemployed => None,
        }
    }

    /// Get the category of this job
    pub fn category(&self) -> JobCategory {
        match self {
            JobType::Woodcutter | JobType::Miner | JobType::Farmer |
            JobType::Fisher | JobType::Hunter | JobType::Herder |
            JobType::Herbalist => JobCategory::ResourceGathering,

            JobType::Miller | JobType::Butcher | JobType::Baker |
            JobType::Brewer | JobType::Cheesemaker => JobCategory::FoodProcessing,

            JobType::Tanner | JobType::Potter | JobType::Weaver |
            JobType::Spinner | JobType::Glassblower | JobType::Dyer |
            JobType::Ropemaker | JobType::Brickmaker | JobType::CharcoalMaker |
            JobType::Papermaker => JobCategory::MaterialProcessing,

            JobType::Carpenter | JobType::Stonemason | JobType::Sawyer |
            JobType::Turner | JobType::Blacksmith | JobType::Armorer |
            JobType::Goldsmith | JobType::Tinsmith | JobType::Bowyer |
            JobType::Fletcher | JobType::Wheelwright => JobCategory::Crafting,

            JobType::Tailor | JobType::Cobbler | JobType::Leatherworker => JobCategory::TextileAndLeather,

            JobType::Healer | JobType::Apothecary | JobType::Barber => JobCategory::Medical,

            JobType::Scribe | JobType::Printer => JobCategory::Scholarly,

            JobType::Merchant | JobType::Carter => JobCategory::Commerce,

            JobType::Watchman | JobType::TownCrier => JobCategory::Civic,

            JobType::Cook | JobType::Beekeeper | JobType::Birdcatcher |
            JobType::Painter | JobType::Candlemaker => JobCategory::Specialized,

            JobType::Laborer | JobType::Unemployed => JobCategory::General,
        }
    }

    /// Get a description of what this job does
    pub fn description(&self) -> &'static str {
        match self {
            JobType::Woodcutter => "Chops wood and gathers timber from forests.",
            JobType::Miner => "Extracts stone, ore, and minerals from the earth.",
            JobType::Farmer => "Cultivates crops and manages agricultural fields.",
            JobType::Fisher => "Catches fish from rivers and lakes.",
            JobType::Hunter => "Hunts wild animals for meat and hides.",
            JobType::Herder => "Raises and manages livestock.",
            JobType::Herbalist => "Gathers medicinal herbs and plants.",

            JobType::Miller => "Grinds grain into flour for bread making.",
            JobType::Butcher => "Processes animal carcasses into meat and byproducts.",
            JobType::Baker => "Bakes bread and other baked goods.",
            JobType::Brewer => "Brews ale, beer, and other fermented beverages.",
            JobType::Cheesemaker => "Produces cheese, butter, and dairy products.",

            JobType::Tanner => "Processes animal hides into leather.",
            JobType::Potter => "Creates pottery, ceramics, and clay vessels.",
            JobType::Weaver => "Weaves cloth from thread and fibers.",
            JobType::Spinner => "Spins raw fibers into thread.",
            JobType::Glassblower => "Creates glass objects and containers.",
            JobType::Dyer => "Dyes cloth and materials in various colors.",
            JobType::Ropemaker => "Makes rope and cordage from fibers.",
            JobType::Brickmaker => "Forms and fires clay into bricks.",
            JobType::CharcoalMaker => "Burns wood to produce charcoal.",

            JobType::Carpenter => "Works wood into furniture, tools, and structures.",
            JobType::Stonemason => "Cuts and shapes stone for construction.",
            JobType::Sawyer => "Saws logs into lumber and planks.",
            JobType::Turner => "Creates turned wood items on a lathe.",

            JobType::Blacksmith => "Forges metal tools and basic metalwork.",
            JobType::Armorer => "Crafts armor and protective equipment.",
            JobType::Goldsmith => "Works with precious metals to create jewelry and fine goods.",
            JobType::Tinsmith => "Works with tin to create containers and implements.",

            JobType::Tailor => "Sews and repairs clothing and textiles.",
            JobType::Cobbler => "Makes and repairs shoes and boots.",
            JobType::Leatherworker => "Crafts leather goods and accessories.",

            JobType::Bowyer => "Crafts bows for hunting and warfare.",
            JobType::Fletcher => "Makes arrows and ammunition.",
            JobType::Wheelwright => "Builds and repairs wheels and carts.",

            JobType::Healer => "Treats injuries and illnesses.",
            JobType::Apothecary => "Prepares medicines and remedies.",
            JobType::Barber => "Cuts hair and performs minor medical procedures.",

            JobType::Scribe => "Writes and copies documents.",
            JobType::Printer => "Prints books and documents.",
            JobType::Papermaker => "Makes paper and parchment.",

            JobType::Merchant => "Trades goods and manages commerce.",
            JobType::Watchman => "Guards the settlement and keeps the peace.",
            JobType::TownCrier => "Makes public announcements and spreads news.",

            JobType::Cook => "Prepares meals and cooks food.",
            JobType::Beekeeper => "Maintains beehives and harvests honey.",
            JobType::Birdcatcher => "Catches birds for food or trade.",
            JobType::Painter => "Creates artwork and decorations.",
            JobType::Candlemaker => "Makes candles and lighting supplies.",
            JobType::Carter => "Transports goods between locations.",

            JobType::Laborer => "Performs general manual labor as needed.",
            JobType::Unemployed => "Currently without a profession.",
        }
    }

    /// Get the implementation priority for this job type.
    ///
    /// Returns a priority level from 1-4:
    /// - 1 (Core): Essential for basic settlement survival
    /// - 2 (Important): Secondary production and services
    /// - 3 (Advanced): Specialized crafts and trades
    /// - 4 (Luxury): Non-essential, quality of life jobs
    ///
    /// This helps guide which jobs should be implemented first when
    /// integrating the profession system.
    pub fn implementation_priority(&self) -> u8 {
        match self {
            // Priority 1: Core survival jobs
            JobType::Farmer | JobType::Woodcutter | JobType::Miner |
            JobType::Hunter | JobType::Fisher | JobType::Blacksmith |
            JobType::Carpenter | JobType::Baker | JobType::Healer |
            JobType::Laborer => 1,

            // Priority 2: Important secondary production
            JobType::Miller | JobType::Butcher | JobType::Tanner |
            JobType::Tailor | JobType::Stonemason | JobType::Merchant |
            JobType::Cook | JobType::Herder | JobType::Brewer |
            JobType::Sawyer | JobType::Herbalist => 2,

            // Priority 3: Specialized crafts
            JobType::Armorer | JobType::Potter | JobType::Weaver |
            JobType::Cobbler | JobType::Bowyer | JobType::Fletcher |
            JobType::Apothecary | JobType::Watchman | JobType::Spinner |
            JobType::Leatherworker | JobType::Wheelwright |
            JobType::Cheesemaker | JobType::Carter => 3,

            // Priority 4: Luxury/non-essential
            JobType::Goldsmith | JobType::Painter | JobType::Candlemaker |
            JobType::Birdcatcher | JobType::Glassblower | JobType::Dyer |
            JobType::Ropemaker | JobType::Brickmaker | JobType::CharcoalMaker |
            JobType::Turner | JobType::Tinsmith | JobType::Barber |
            JobType::Scribe | JobType::Printer | JobType::Papermaker |
            JobType::TownCrier | JobType::Beekeeper => 4,

            // Unemployed has no priority
            JobType::Unemployed => 0,
        }
    }

    /// Check if this is a core job essential for settlement survival.
    ///
    /// Core jobs (priority 1) should be implemented first and are required
    /// for a settlement to function at a basic level.
    pub fn is_core_job(&self) -> bool {
        self.implementation_priority() == 1
    }

    /// Check if this job is actively used in the simulation.
    ///
    /// Currently returns `false` for all jobs as the profession system
    /// is not yet integrated with the Agent struct.
    pub fn is_actively_used(&self) -> bool {
        // TODO: Update this when profession system is integrated
        false
    }

    /// Get all job types at a specific implementation priority level.
    pub fn jobs_at_priority(priority: u8) -> Vec<JobType> {
        ALL_JOB_TYPES.iter()
            .filter(|job| job.implementation_priority() == priority)
            .copied()
            .collect()
    }
}

/// Categories of jobs for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobCategory {
    ResourceGathering,
    FoodProcessing,
    MaterialProcessing,
    Crafting,
    TextileAndLeather,
    Medical,
    Scholarly,
    Commerce,
    Civic,
    Specialized,
    General,
}

/// Agent's profession including skill level and workplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profession {
    /// The job type
    pub job: JobType,

    /// Skill level (0-100)
    pub skill_level: u8,

    /// Experience points toward next skill level (0-1000)
    pub experience: u16,

    /// Building where this agent works (if assigned)
    pub workplace: Option<Position>,

    /// ID of the building where they work
    pub workplace_building_id: Option<Uuid>,

    /// How many items/resources produced (lifetime)
    pub items_produced: u32,

    /// Efficiency multiplier based on skill (0.5 to 2.0)
    pub efficiency: f32,

    /// Ticks spent working at this job
    pub time_in_profession: u32,

    /// Current production task (recipe index being worked on)
    pub current_recipe_index: Option<usize>,

    /// Progress on current production (ticks)
    pub production_progress: u32,
}

impl Profession {
    /// Create a new profession with starting skill
    pub fn new(job: JobType) -> Self {
        Self {
            job,
            skill_level: 1,
            experience: 0,
            workplace: None,
            workplace_building_id: None,
            items_produced: 0,
            efficiency: 0.6, // Novice efficiency
            time_in_profession: 0,
            current_recipe_index: None,
            production_progress: 0,
        }
    }

    /// Create with specific skill level
    pub fn with_skill(job: JobType, skill_level: u8) -> Self {
        let mut prof = Self::new(job);
        prof.skill_level = skill_level.min(100);
        prof.update_efficiency();
        prof
    }

    /// Assign to a workplace
    pub fn assign_workplace(&mut self, position: Position, building_id: Uuid) {
        self.workplace = Some(position);
        self.workplace_building_id = Some(building_id);
    }

    /// Remove workplace assignment
    pub fn remove_workplace(&mut self) {
        self.workplace = None;
        self.workplace_building_id = None;
    }

    /// Gain experience from working
    pub fn gain_experience(&mut self, amount: u16) {
        if self.skill_level >= 100 {
            return; // Already master
        }

        self.experience += amount;

        // Check for level up (1000 XP per level)
        while self.experience >= 1000 && self.skill_level < 100 {
            self.experience -= 1000;
            self.skill_level += 1;
            self.update_efficiency();
        }

        // Cap experience if at max level
        if self.skill_level >= 100 {
            self.experience = 0;
        }
    }

    /// Update efficiency based on skill level
    fn update_efficiency(&mut self) {
        // Efficiency ranges from 0.6 (novice) to 2.0 (master)
        // Skill 1 = 0.6, Skill 50 = 1.3, Skill 100 = 2.0
        self.efficiency = 0.6 + (self.skill_level as f32 / 100.0) * 1.4;
    }

    /// Record production
    pub fn record_production(&mut self, quantity: u32) {
        self.items_produced += quantity;

        // Gain experience based on production
        // More items = more XP, but diminishing returns
        let xp_gain = (quantity as f32).sqrt().ceil() as u16;
        self.gain_experience(xp_gain);
    }

    /// Tick for time tracking
    pub fn tick(&mut self) {
        self.time_in_profession += 1;

        // Passive XP gain from time spent (very slow)
        if self.time_in_profession % 500 == 0 {
            self.gain_experience(1);
        }
    }

    /// Get skill description
    pub fn skill_description(&self) -> &'static str {
        match self.skill_level {
            0..=10 => "Novice",
            11..=25 => "Apprentice",
            26..=40 => "Journeyman",
            41..=60 => "Skilled",
            61..=80 => "Expert",
            81..=95 => "Master",
            _ => "Grandmaster",
        }
    }

    /// Check if this profession requires a specific workplace
    pub fn requires_workplace(&self) -> bool {
        self.job.workplace().is_some()
    }

    /// Get the required building type for this profession
    pub fn required_building_type(&self) -> Option<BuildingType> {
        self.job.workplace()
    }

    /// Check if currently producing something
    pub fn is_producing(&self) -> bool {
        self.current_recipe_index.is_some()
    }

    /// Start production on a recipe
    pub fn start_production(&mut self, recipe_index: usize) {
        self.current_recipe_index = Some(recipe_index);
        self.production_progress = 0;
    }

    /// Tick production progress, returns Some((ItemType, quantity)) if production completes
    /// NOTE: This method is deprecated as the profession system is no longer in active use.
    /// Crafting is now handled through the skill-based system in src/world/crafting.rs
    #[allow(dead_code)]
    pub fn tick_production(&mut self) -> Option<Vec<(crate::world::ItemType, u32)>> {
        // This method is no longer functional after profession system removal
        None
    }

    /// Cancel current production
    #[allow(dead_code)]
    pub fn cancel_production(&mut self) {
        self.current_recipe_index = None;
        self.production_progress = 0;
    }

    /// Get progress percentage of current production (0-100)
    /// NOTE: This method is deprecated as the profession system is no longer in active use.
    #[allow(dead_code)]
    pub fn production_progress_percent(&self) -> u8 {
        0
    }

    /// Get the current recipe being worked on
    /// NOTE: This method is deprecated as the profession system is no longer in active use.
    #[allow(dead_code)]
    pub fn get_current_recipe(&self) -> Option<crate::world::Recipe> {
        None
    }
}

impl Default for Profession {
    fn default() -> Self {
        Self::new(JobType::Unemployed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profession_creation() {
        let prof = Profession::new(JobType::Blacksmith);
        assert_eq!(prof.job, JobType::Blacksmith);
        assert_eq!(prof.skill_level, 1);
        assert_eq!(prof.experience, 0);
    }

    #[test]
    fn test_skill_progression() {
        let mut prof = Profession::new(JobType::Carpenter);

        // Gain experience
        prof.gain_experience(500);
        assert_eq!(prof.skill_level, 1);
        assert_eq!(prof.experience, 500);

        // Level up
        prof.gain_experience(500);
        assert_eq!(prof.skill_level, 2);
        assert_eq!(prof.experience, 0);
    }

    #[test]
    fn test_efficiency_increase() {
        let prof_novice = Profession::with_skill(JobType::Blacksmith, 1);
        let prof_master = Profession::with_skill(JobType::Blacksmith, 100);

        assert!(prof_master.efficiency > prof_novice.efficiency);
        assert!(prof_novice.efficiency >= 0.6);
        assert!(prof_master.efficiency <= 2.0);
    }

    #[test]
    fn test_workplace_assignment() {
        let mut prof = Profession::new(JobType::Baker);
        let pos = Position::new(10, 15);
        let building_id = Uuid::new_v4();

        prof.assign_workplace(pos, building_id);
        assert_eq!(prof.workplace, Some(pos));
        assert_eq!(prof.workplace_building_id, Some(building_id));

        prof.remove_workplace();
        assert_eq!(prof.workplace, None);
    }

    #[test]
    fn test_production_tracking() {
        let mut prof = Profession::new(JobType::Potter);

        prof.record_production(10);
        assert_eq!(prof.items_produced, 10);
        assert!(prof.experience > 0);
    }

    #[test]
    fn test_job_categories() {
        assert_eq!(JobType::Blacksmith.category(), JobCategory::Crafting);
        assert_eq!(JobType::Farmer.category(), JobCategory::ResourceGathering);
        assert_eq!(JobType::Baker.category(), JobCategory::FoodProcessing);
    }

    #[test]
    fn test_workplace_requirements() {
        let baker = Profession::new(JobType::Baker);
        assert!(baker.requires_workplace());
        assert_eq!(baker.required_building_type(), Some(BuildingType::Bakery));

        let farmer = Profession::new(JobType::Farmer);
        assert!(!farmer.requires_workplace());
    }

    #[test]
    fn test_implementation_priority() {
        // Core jobs should be priority 1
        assert_eq!(JobType::Farmer.implementation_priority(), 1);
        assert_eq!(JobType::Blacksmith.implementation_priority(), 1);
        assert_eq!(JobType::Healer.implementation_priority(), 1);

        // Important jobs should be priority 2
        assert_eq!(JobType::Miller.implementation_priority(), 2);
        assert_eq!(JobType::Tanner.implementation_priority(), 2);

        // Specialized jobs should be priority 3
        assert_eq!(JobType::Armorer.implementation_priority(), 3);
        assert_eq!(JobType::Potter.implementation_priority(), 3);

        // Luxury jobs should be priority 4
        assert_eq!(JobType::Goldsmith.implementation_priority(), 4);
        assert_eq!(JobType::Painter.implementation_priority(), 4);

        // Unemployed has no priority
        assert_eq!(JobType::Unemployed.implementation_priority(), 0);
    }

    #[test]
    fn test_is_core_job() {
        assert!(JobType::Farmer.is_core_job());
        assert!(JobType::Woodcutter.is_core_job());
        assert!(JobType::Blacksmith.is_core_job());
        assert!(JobType::Laborer.is_core_job());

        assert!(!JobType::Goldsmith.is_core_job());
        assert!(!JobType::Painter.is_core_job());
        assert!(!JobType::Unemployed.is_core_job());
    }

    #[test]
    fn test_is_actively_used() {
        // Currently no jobs are actively used (system not integrated)
        assert!(!JobType::Farmer.is_actively_used());
        assert!(!JobType::Blacksmith.is_actively_used());
    }

    #[test]
    fn test_jobs_at_priority() {
        let core_jobs = JobType::jobs_at_priority(1);
        assert!(core_jobs.contains(&JobType::Farmer));
        assert!(core_jobs.contains(&JobType::Blacksmith));
        assert!(!core_jobs.contains(&JobType::Goldsmith));

        let luxury_jobs = JobType::jobs_at_priority(4);
        assert!(luxury_jobs.contains(&JobType::Goldsmith));
        assert!(luxury_jobs.contains(&JobType::Painter));
        assert!(!luxury_jobs.contains(&JobType::Farmer));
    }

    #[test]
    fn test_all_job_types_coverage() {
        // Verify ALL_JOB_TYPES contains exactly 52 jobs
        assert_eq!(ALL_JOB_TYPES.len(), 52);

        // Verify all priorities from 0-4 are covered
        let mut priority_counts = [0u32; 5];
        for job in ALL_JOB_TYPES.iter() {
            let priority = job.implementation_priority() as usize;
            priority_counts[priority] += 1;
        }

        // Check we have jobs at each priority level
        assert!(priority_counts[0] > 0, "Should have unemployed (priority 0)");
        assert!(priority_counts[1] > 0, "Should have core jobs (priority 1)");
        assert!(priority_counts[2] > 0, "Should have important jobs (priority 2)");
        assert!(priority_counts[3] > 0, "Should have specialized jobs (priority 3)");
        assert!(priority_counts[4] > 0, "Should have luxury jobs (priority 4)");
    }
}
