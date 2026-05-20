// src/agents/job_happiness.rs
//! Job happiness calculation system.
//!
//! This module calculates how much happiness an agent derives from different
//! types of work based on their personality traits. Agents will gravitate
//! toward jobs that make them happy, while still prioritizing survival needs.

use serde::{Deserialize, Serialize};
use crate::core::traits::{Trait, TraitSet};

/// Categories of jobs/work that agents can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobCategory {
    /// Mining stone, ore, clay
    Mining,
    /// Constructing buildings and structures
    Building,
    /// Creating tools, weapons, clothing
    Crafting,
    /// Growing and harvesting crops
    Farming,
    /// Hunting animals for food/materials
    Hunting,
    /// Catching fish
    Fishing,
    /// Preparing food
    Cooking,
    /// Interacting with other agents
    Social,
    /// Discovering new areas
    Exploring,
    /// Caring for sick/injured/young
    Caretaking,
    /// Gathering wood, berries, etc.
    Gathering,
    /// Generic labor/hauling
    Labor,
}

impl JobCategory {
    /// Get all job categories
    pub fn all() -> &'static [JobCategory] {
        &[
            JobCategory::Mining,
            JobCategory::Building,
            JobCategory::Crafting,
            JobCategory::Farming,
            JobCategory::Hunting,
            JobCategory::Fishing,
            JobCategory::Cooking,
            JobCategory::Social,
            JobCategory::Exploring,
            JobCategory::Caretaking,
            JobCategory::Gathering,
            JobCategory::Labor,
        ]
    }

    /// Get the string name of this job category
    pub fn name(&self) -> &'static str {
        match self {
            JobCategory::Mining => "mining",
            JobCategory::Building => "building",
            JobCategory::Crafting => "crafting",
            JobCategory::Farming => "farming",
            JobCategory::Hunting => "hunting",
            JobCategory::Fishing => "fishing",
            JobCategory::Cooking => "cooking",
            JobCategory::Social => "social",
            JobCategory::Exploring => "exploring",
            JobCategory::Caretaking => "caretaking",
            JobCategory::Gathering => "gathering",
            JobCategory::Labor => "labor",
        }
    }

    /// Parse a job category from a string name
    pub fn from_name(name: &str) -> Option<JobCategory> {
        match name.to_lowercase().as_str() {
            "mining" => Some(JobCategory::Mining),
            "building" | "construction" => Some(JobCategory::Building),
            "crafting" => Some(JobCategory::Crafting),
            "farming" => Some(JobCategory::Farming),
            "hunting" => Some(JobCategory::Hunting),
            "fishing" => Some(JobCategory::Fishing),
            "cooking" => Some(JobCategory::Cooking),
            "social" | "socializing" => Some(JobCategory::Social),
            "exploring" | "exploration" => Some(JobCategory::Exploring),
            "caretaking" | "caring" => Some(JobCategory::Caretaking),
            "gathering" => Some(JobCategory::Gathering),
            "labor" | "hauling" => Some(JobCategory::Labor),
            _ => None,
        }
    }
}

/// Get the happiness bonus a specific trait provides for a job category
pub fn trait_job_happiness(trait_type: Trait, job: JobCategory) -> f32 {
    match (trait_type, job) {
        // Builder trait - loves construction
        (Trait::Builder, JobCategory::Building) => 6.0,
        (Trait::Builder, JobCategory::Crafting) => 2.0,

        // Handy trait - enjoys completing any hands-on work
        (Trait::Handy, JobCategory::Mining) => 4.0,
        (Trait::Handy, JobCategory::Building) => 4.0,
        (Trait::Handy, JobCategory::Crafting) => 5.0,
        (Trait::Handy, JobCategory::Farming) => 3.0,
        (Trait::Handy, JobCategory::Gathering) => 3.0,

        // Diligent trait - satisfaction from hard work
        (Trait::Diligent, JobCategory::Mining) => 3.0,
        (Trait::Diligent, JobCategory::Farming) => 3.0,
        (Trait::Diligent, JobCategory::Labor) => 3.0,
        (Trait::Diligent, JobCategory::Gathering) => 2.0,

        // Lazy trait - negative happiness from work
        (Trait::Lazy, JobCategory::Mining) => -3.0,
        (Trait::Lazy, JobCategory::Building) => -2.0,
        (Trait::Lazy, JobCategory::Farming) => -2.0,
        (Trait::Lazy, JobCategory::Labor) => -4.0,
        (Trait::Lazy, JobCategory::Gathering) => -1.0,

        // CraftObsessed - loves crafting above all
        (Trait::CraftObsessed, JobCategory::Crafting) => 8.0,
        (Trait::CraftObsessed, JobCategory::Building) => 2.0,

        // Proud trait - happiness from accomplishment-oriented work
        (Trait::Proud, JobCategory::Building) => 3.0,
        (Trait::Proud, JobCategory::Crafting) => 3.0,
        (Trait::Proud, JobCategory::Hunting) => 2.0,

        // Explorer trait - loves discovering new areas
        (Trait::Explorer, JobCategory::Exploring) => 6.0,
        (Trait::Explorer, JobCategory::Gathering) => 2.0,
        (Trait::Explorer, JobCategory::Hunting) => 2.0,

        // Caretaker trait - happiness from helping others
        (Trait::Caretaker, JobCategory::Caretaking) => 5.0,
        (Trait::Caretaker, JobCategory::Cooking) => 3.0,
        (Trait::Caretaker, JobCategory::Social) => 2.0,

        // Altruist trait - happiness from community-benefiting work
        (Trait::Altruist, JobCategory::Caretaking) => 4.0,
        (Trait::Altruist, JobCategory::Cooking) => 3.0,
        (Trait::Altruist, JobCategory::Farming) => 2.0,
        (Trait::Altruist, JobCategory::Building) => 2.0,

        // Extrovert trait - loves social interaction
        (Trait::Extrovert, JobCategory::Social) => 5.0,
        (Trait::Sociable, JobCategory::Social) => 5.0,

        // Introvert trait - dislikes social work
        (Trait::Introvert, JobCategory::Social) => -3.0,
        (Trait::Introverted, JobCategory::Social) => -3.0,

        // Brave/Protector traits - enjoy hunting dangerous prey
        (Trait::Brave, JobCategory::Hunting) => 3.0,
        (Trait::Protector, JobCategory::Hunting) => 4.0,

        // Coward trait - dislikes dangerous work
        (Trait::Coward, JobCategory::Hunting) => -3.0,

        // Peaceful/Calm traits - prefer non-violent work
        (Trait::Peaceful, JobCategory::Farming) => 2.0,
        (Trait::Peaceful, JobCategory::Fishing) => 3.0,
        (Trait::Peaceful, JobCategory::Cooking) => 2.0,
        (Trait::Peaceful, JobCategory::Hunting) => -2.0,
        (Trait::Calm, JobCategory::Fishing) => 3.0,
        (Trait::Calm, JobCategory::Farming) => 2.0,

        // Pragmatist trait - satisfaction from survival-oriented work
        (Trait::Pragmatist, JobCategory::Farming) => 3.0,
        (Trait::Pragmatist, JobCategory::Hunting) => 2.0,
        (Trait::Pragmatist, JobCategory::Gathering) => 2.0,

        // Survivalist trait - enjoys self-sufficiency work
        (Trait::Survivalist, JobCategory::Hunting) => 3.0,
        (Trait::Survivalist, JobCategory::Gathering) => 3.0,
        (Trait::Survivalist, JobCategory::Farming) => 2.0,

        // Glutton trait - loves cooking/food work
        (Trait::Glutton, JobCategory::Cooking) => 4.0,
        (Trait::Glutton, JobCategory::Farming) => 2.0,
        (Trait::Glutton, JobCategory::Fishing) => 2.0,

        // AnimalLover trait
        (Trait::AnimalLover, JobCategory::Caretaking) => 3.0,
        (Trait::AnimalLover, JobCategory::Farming) => 2.0,
        (Trait::AnimalLover, JobCategory::Hunting) => -4.0,

        // Curious/Bookworm traits
        (Trait::Curious, JobCategory::Exploring) => 4.0,
        (Trait::Bookworm, JobCategory::Crafting) => 2.0,

        // Traditionalist - prefers simple, traditional work
        (Trait::Traditionalist, JobCategory::Farming) => 3.0,
        (Trait::Traditionalist, JobCategory::Gathering) => 2.0,
        (Trait::Traditionalist, JobCategory::Crafting) => 2.0,

        // Ambitious trait - likes achievement-oriented work
        (Trait::Ambitious, JobCategory::Building) => 3.0,
        (Trait::Ambitious, JobCategory::Crafting) => 2.0,
        (Trait::Ambitious, JobCategory::Mining) => 2.0,

        // Default: no special happiness modifier
        _ => 0.0,
    }
}

/// Calculate total job happiness for an agent based on their traits
pub fn calculate_job_happiness(traits: &TraitSet, job: JobCategory) -> f32 {
    let mut total = 0.0;
    for trait_type in &traits.traits {
        total += trait_job_happiness(*trait_type, job);
    }
    total
}

/// Find the job category that would make an agent happiest
pub fn find_preferred_job(traits: &TraitSet) -> (JobCategory, f32) {
    let mut best_job = JobCategory::Labor;
    let mut best_happiness = f32::MIN;

    for job in JobCategory::all() {
        let happiness = calculate_job_happiness(traits, *job);
        if happiness > best_happiness {
            best_happiness = happiness;
            best_job = *job;
        }
    }

    (best_job, best_happiness)
}

/// Get happiness scores for all jobs, sorted by preference (highest first)
pub fn rank_jobs_by_happiness(traits: &TraitSet) -> Vec<(JobCategory, f32)> {
    let mut rankings: Vec<(JobCategory, f32)> = JobCategory::all()
        .iter()
        .map(|job| (*job, calculate_job_happiness(traits, *job)))
        .collect();

    rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rankings
}

/// Calculate effective job priority considering both drive urgency and happiness
///
/// Formula: effective_priority = drive_urgency * (1.0 + happiness_factor * weight)
///
/// The weight parameter controls how much happiness influences the decision:
/// - 0.0 = happiness has no effect (pure drive-based)
/// - 0.3 = happiness has moderate effect (recommended)
/// - 1.0 = happiness has strong effect
pub fn calculate_effective_priority(
    drive_urgency: f32,
    job_happiness: f32,
    happiness_weight: f32,
) -> f32 {
    // Normalize happiness to a 0-1 range for the multiplier
    // Assuming happiness ranges from -5 to +10
    let normalized_happiness = ((job_happiness + 5.0) / 15.0).clamp(0.0, 1.0);

    drive_urgency * (1.0 + normalized_happiness * happiness_weight)
}

/// Check if survival drives should override happiness consideration
pub fn should_override_happiness(hunger: f32, thirst: f32, health_percent: f32) -> bool {
    // Survival thresholds - happiness is overridden when:
    // - Hunger > 0.7 (very hungry)
    // - Thirst > 0.7 (very thirsty)
    // - Health < 30% (critically injured)
    hunger > 0.7 || thirst > 0.7 || health_percent < 0.3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_prefers_building() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Builder);

        let happiness = calculate_job_happiness(&traits, JobCategory::Building);
        assert_eq!(happiness, 6.0);

        let (preferred, _) = find_preferred_job(&traits);
        assert_eq!(preferred, JobCategory::Building);
    }

    #[test]
    fn test_lazy_dislikes_labor() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Lazy);

        let happiness = calculate_job_happiness(&traits, JobCategory::Labor);
        assert_eq!(happiness, -4.0);
    }

    #[test]
    fn test_multiple_traits_stack() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Builder);
        traits.add_trait(Trait::Handy);

        // Builder: +6.0, Handy: +4.0 = 10.0
        let happiness = calculate_job_happiness(&traits, JobCategory::Building);
        assert_eq!(happiness, 10.0);
    }

    #[test]
    fn test_craft_obsessed_loves_crafting() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::CraftObsessed);

        let happiness = calculate_job_happiness(&traits, JobCategory::Crafting);
        assert_eq!(happiness, 8.0);

        let (preferred, _) = find_preferred_job(&traits);
        assert_eq!(preferred, JobCategory::Crafting);
    }

    #[test]
    fn test_effective_priority_calculation() {
        // High happiness should increase effective priority
        let high_priority = calculate_effective_priority(0.5, 6.0, 0.3);
        let low_priority = calculate_effective_priority(0.5, -3.0, 0.3);

        assert!(high_priority > low_priority);
    }

    #[test]
    fn test_survival_override() {
        assert!(should_override_happiness(0.8, 0.3, 0.8)); // Very hungry
        assert!(should_override_happiness(0.3, 0.8, 0.8)); // Very thirsty
        assert!(should_override_happiness(0.3, 0.3, 0.2)); // Critical health
        assert!(!should_override_happiness(0.5, 0.5, 0.8)); // Normal state
    }

    #[test]
    fn test_rank_jobs() {
        let mut traits = TraitSet::new();
        traits.add_trait(Trait::Explorer);

        let rankings = rank_jobs_by_happiness(&traits);
        assert_eq!(rankings[0].0, JobCategory::Exploring);
        assert_eq!(rankings[0].1, 6.0);
    }

    #[test]
    fn test_job_category_names() {
        assert_eq!(JobCategory::Building.name(), "building");
        assert_eq!(JobCategory::from_name("building"), Some(JobCategory::Building));
        assert_eq!(JobCategory::from_name("construction"), Some(JobCategory::Building));
    }
}
