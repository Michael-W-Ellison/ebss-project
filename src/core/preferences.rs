// src/core/preferences.rs
//! Preference system for agents.
//!
//! Agents develop preferences for specific foods, jobs, animals, people, and tools.
//! These preferences provide happiness bonuses and influence behavior.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    /// Favorite food that provides happiness bonus
    pub favorite_food: Option<String>,

    /// Favorite job that provides happiness when working
    pub favorite_job: Option<String>,

    /// Favorite animal type that provides extra happiness
    pub favorite_animal: Option<String>,

    /// Favorite person (provides extra happiness when near)
    pub favorite_person: Option<Uuid>,

    /// Favorite tool (provides work speed and happiness bonus)
    pub favorite_tool: Option<String>,

    /// Favorite target for bullying (for Cruel trait agents)
    pub favorite_target: Option<Uuid>,

    /// Object of obsession (for Obsessive trait agents)
    pub obsession: Option<Obsession>,
}

/// Obsession details for Obsessive trait
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obsession {
    pub obsession_type: ObsessionType,
    pub target_id: Option<Uuid>, // For agent/animal obsessions
    pub target_name: Option<String>, // For material/food obsessions
    pub intensity: f32, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObsessionType {
    Agent,
    Animal,
    Material,
    Food,
    Location,
    Item,
}

impl Preferences {
    pub fn new() -> Self {
        Self {
            favorite_food: None,
            favorite_job: None,
            favorite_animal: None,
            favorite_person: None,
            favorite_tool: None,
            favorite_target: None,
            obsession: None,
        }
    }

    /// Set favorite food
    pub fn set_favorite_food(&mut self, food: String) {
        self.favorite_food = Some(food);
    }

    /// Set favorite job
    pub fn set_favorite_job(&mut self, job: String) {
        self.favorite_job = Some(job);
    }

    /// Set favorite animal
    pub fn set_favorite_animal(&mut self, animal: String) {
        self.favorite_animal = Some(animal);
    }

    /// Set favorite person
    pub fn set_favorite_person(&mut self, person_id: Uuid) {
        self.favorite_person = Some(person_id);
    }

    /// Set favorite tool
    pub fn set_favorite_tool(&mut self, tool: String) {
        self.favorite_tool = Some(tool);
    }

    /// Set favorite target (for bullying)
    pub fn set_favorite_target(&mut self, target_id: Uuid) {
        self.favorite_target = Some(target_id);
    }

    /// Set obsession
    pub fn set_obsession(&mut self, obsession: Obsession) {
        self.obsession = Some(obsession);
    }

    /// Check if eating specific food
    pub fn is_favorite_food(&self, food: &str) -> bool {
        self.favorite_food.as_ref().map_or(false, |f| f == food)
    }

    /// Check if doing favorite job
    pub fn is_favorite_job(&self, job: &str) -> bool {
        self.favorite_job.as_ref().map_or(false, |j| j == job)
    }

    /// Check if near favorite animal
    pub fn is_favorite_animal(&self, animal: &str) -> bool {
        self.favorite_animal.as_ref().map_or(false, |a| a == animal)
    }

    /// Check if using favorite tool
    pub fn is_favorite_tool(&self, tool: &str) -> bool {
        self.favorite_tool.as_ref().map_or(false, |t| t == tool)
    }

    /// Check if near favorite person
    pub fn is_near_favorite_person(&self, person_id: Uuid) -> bool {
        self.favorite_person == Some(person_id)
    }

    /// Check if targeting favorite bullying target
    pub fn is_favorite_target(&self, target_id: Uuid) -> bool {
        self.favorite_target == Some(target_id)
    }

    /// Check if near obsession
    pub fn is_near_obsession(&self, check_type: &ObsessionType, check_id: Option<Uuid>, check_name: Option<&str>) -> bool {
        if let Some(obsession) = &self.obsession {
            if std::mem::discriminant(&obsession.obsession_type) != std::mem::discriminant(check_type) {
                return false;
            }

            match check_type {
                ObsessionType::Agent | ObsessionType::Animal => {
                    obsession.target_id == check_id
                }
                ObsessionType::Material | ObsessionType::Food | ObsessionType::Item | ObsessionType::Location => {
                    if let (Some(obs_name), Some(chk_name)) = (&obsession.target_name, check_name) {
                        obs_name == chk_name
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    /// Get obsession intensity (for happiness calculations)
    pub fn obsession_intensity(&self) -> f32 {
        self.obsession.as_ref().map(|o| o.intensity).unwrap_or(0.0)
    }

    /// Generate random preferences
    pub fn generate_random() -> Self {
        use rand::Rng;
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        let mut prefs = Preferences::new();

        // Common foods
        let foods = ["bread", "meat", "fish", "vegetables", "fruit", "cheese", "soup"];
        if rng.gen_bool(0.7) { // 70% chance to have favorite food
            prefs.favorite_food = foods.choose(&mut rng).map(|s| s.to_string());
        }

        // Common jobs
        let jobs = ["mining", "farming", "fishing", "building", "crafting", "hunting", "cooking"];
        if rng.gen_bool(0.6) { // 60% chance to have favorite job
            prefs.favorite_job = jobs.choose(&mut rng).map(|s| s.to_string());
        }

        // Common animals
        let animals = ["cow", "sheep", "chicken", "pig", "horse", "dog", "cat"];
        if rng.gen_bool(0.5) { // 50% chance to have favorite animal
            prefs.favorite_animal = animals.choose(&mut rng).map(|s| s.to_string());
        }

        // Common tools
        let tools = ["pickaxe", "axe", "hoe", "sword", "hammer", "shovel"];
        if rng.gen_bool(0.4) { // 40% chance to have favorite tool
            prefs.favorite_tool = tools.choose(&mut rng).map(|s| s.to_string());
        }

        prefs
    }

    /// Generate preferences based on agent traits
    ///
    /// This derives the favorite job from the agent's personality traits,
    /// ensuring agents naturally gravitate toward work that makes them happy.
    pub fn from_traits(traits: &crate::core::traits::TraitSet) -> Self {
        use crate::agents::job_happiness::find_preferred_job;
        use rand::Rng;
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        let mut prefs = Preferences::new();

        // Derive favorite job from traits
        let (preferred_job, happiness) = find_preferred_job(traits);

        // Only set favorite job if there's positive happiness from it
        if happiness > 0.0 {
            prefs.favorite_job = Some(preferred_job.name().to_string());
        } else {
            // Fall back to random if no trait provides happiness
            let jobs = ["mining", "farming", "fishing", "building", "crafting", "hunting", "cooking"];
            if rng.gen_bool(0.4) {
                prefs.favorite_job = jobs.choose(&mut rng).map(|s| s.to_string());
            }
        }

        // Foods - random for now, could be trait-influenced later
        let foods = ["bread", "meat", "fish", "vegetables", "fruit", "cheese", "soup"];
        if rng.gen_bool(0.7) {
            prefs.favorite_food = foods.choose(&mut rng).map(|s| s.to_string());
        }

        // Animals - influenced by AnimalLover trait
        let animals = ["cow", "sheep", "chicken", "pig", "horse", "dog", "cat"];
        if traits.has(crate::core::traits::Trait::AnimalLover) {
            prefs.favorite_animal = animals.choose(&mut rng).map(|s| s.to_string());
        } else if rng.gen_bool(0.3) {
            prefs.favorite_animal = animals.choose(&mut rng).map(|s| s.to_string());
        }

        // Tools - Handy trait more likely to have favorite tool
        let tools = ["pickaxe", "axe", "hoe", "sword", "hammer", "shovel"];
        let tool_chance = if traits.has(crate::core::traits::Trait::Handy) { 0.8 } else { 0.4 };
        if rng.gen_bool(tool_chance) {
            prefs.favorite_tool = tools.choose(&mut rng).map(|s| s.to_string());
        }

        prefs
    }

    /// Calculate happiness modifier for doing a specific job
    ///
    /// Returns a value from -1.0 to 1.0:
    /// - Positive if this is the favorite job
    /// - Zero for neutral jobs
    /// - Negative for disliked jobs (based on traits)
    pub fn job_happiness_modifier(&self, job: &str, traits: &crate::core::traits::TraitSet) -> f32 {
        use crate::agents::job_happiness::{calculate_job_happiness, JobCategory};

        let mut modifier = 0.0;

        // Bonus for favorite job
        if self.is_favorite_job(job) {
            modifier += 0.3;
        }

        // Add trait-based happiness
        if let Some(job_category) = JobCategory::from_name(job) {
            let trait_happiness = calculate_job_happiness(traits, job_category);
            // Normalize to -0.5 to 0.5 range
            modifier += (trait_happiness / 20.0).clamp(-0.5, 0.5);
        }

        modifier.clamp(-1.0, 1.0)
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_creation() {
        let prefs = Preferences::new();
        assert!(prefs.favorite_food.is_none());
        assert!(prefs.favorite_job.is_none());
    }

    #[test]
    fn test_set_favorite_food() {
        let mut prefs = Preferences::new();
        prefs.set_favorite_food("bread".to_string());
        assert_eq!(prefs.favorite_food, Some("bread".to_string()));
        assert!(prefs.is_favorite_food("bread"));
        assert!(!prefs.is_favorite_food("meat"));
    }

    #[test]
    fn test_set_favorite_job() {
        let mut prefs = Preferences::new();
        prefs.set_favorite_job("mining".to_string());
        assert!(prefs.is_favorite_job("mining"));
        assert!(!prefs.is_favorite_job("farming"));
    }

    #[test]
    fn test_set_favorite_person() {
        let mut prefs = Preferences::new();
        let person_id = Uuid::new_v4();
        prefs.set_favorite_person(person_id);
        assert!(prefs.is_near_favorite_person(person_id));
        assert!(!prefs.is_near_favorite_person(Uuid::new_v4()));
    }

    #[test]
    fn test_obsession() {
        let mut prefs = Preferences::new();
        let obsession = Obsession {
            obsession_type: ObsessionType::Material,
            target_id: None,
            target_name: Some("gold".to_string()),
            intensity: 0.8,
        };
        prefs.set_obsession(obsession);

        assert_eq!(prefs.obsession_intensity(), 0.8);
        assert!(prefs.is_near_obsession(&ObsessionType::Material, None, Some("gold")));
        assert!(!prefs.is_near_obsession(&ObsessionType::Material, None, Some("iron")));
    }

    #[test]
    fn test_generate_random() {
        let prefs = Preferences::generate_random();
        // Can't assert specific values due to randomness, but should not panic
        assert!(true);
    }
}
