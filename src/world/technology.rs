// src/world/technology.rs
//! Technology discovery and progression system.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use crate::world::{ItemType, ResourceType};
use crate::agents::profession::JobType;

/// Technological eras that define progression stages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum TechEra {
    StoneAge,     // Flint tools, fire, basic shelter
    CopperAge,    // Native copper working (cold)
    BronzeAge,    // Copper + tin alloy, improved furnaces
    IronAge,      // Iron smelting, steel production
    Medieval,     // Advanced crafting, specialized professions
}

impl TechEra {
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            TechEra::StoneAge => "Stone Age",
            TechEra::CopperAge => "Copper Age",
            TechEra::BronzeAge => "Bronze Age",
            TechEra::IronAge => "Iron Age",
            TechEra::Medieval => "Medieval Era",
        }
    }
}

/// A technology that can be discovered
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Technology {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub era: TechEra,
    pub prerequisites: Vec<&'static str>, // IDs of required techs
    pub discovery_difficulty: u8, // 0-100, how hard to discover
    pub required_items: Vec<ItemType>, // Items needed for experimentation
    pub unlocks_recipes: Vec<ItemType>, // What can be crafted after discovery
}

impl Technology {
    /// Check if all prerequisites are met
    pub fn can_discover(&self, known_techs: &HashSet<&'static str>) -> bool {
        self.prerequisites.iter().all(|prereq| known_techs.contains(prereq))
    }

    /// Get discovery chance based on agent curiosity trait
    pub fn discovery_chance(&self, curiosity: i8) -> f32 {
        // Base chance inversely proportional to difficulty
        let base_chance = (100 - self.discovery_difficulty) as f32 / 100.0;

        // Curiosity modifier: -10 to +10 becomes 0.5x to 1.5x
        let curiosity_mult = 1.0 + (curiosity as f32 * 0.05);

        (base_chance * curiosity_mult * 0.1).min(0.5) // Max 50% per attempt
    }
}

/// Global technology tree
#[derive(Debug, Clone)]
pub struct TechnologyTree {
    technologies: HashMap<&'static str, Technology>,
}

impl TechnologyTree {
    pub fn new() -> Self {
        let mut tree = Self {
            technologies: HashMap::new(),
        };
        tree.initialize_technologies();
        tree
    }

    fn add_tech(&mut self, tech: Technology) {
        self.technologies.insert(tech.id, tech);
    }

    fn initialize_technologies(&mut self) {
        // === STONE AGE ===

        self.add_tech(Technology {
            id: "fire",
            name: "Fire Making",
            description: "Creating and controlling fire for warmth and cooking",
            era: TechEra::StoneAge,
            prerequisites: vec![],
            discovery_difficulty: 0, // Everyone starts with this
            required_items: vec![ItemType::Wood],
            unlocks_recipes: vec![],
        });

        self.add_tech(Technology {
            id: "flint_knapping",
            name: "Flint Knapping",
            description: "Shaping flint into sharp tools and weapons",
            era: TechEra::StoneAge,
            prerequisites: vec![],
            discovery_difficulty: 10,
            required_items: vec![ItemType::Stone],
            unlocks_recipes: vec![ItemType::StoneAxe, ItemType::StonePickaxe, ItemType::StoneSpear],
        });

        self.add_tech(Technology {
            id: "stone_tools",
            name: "Stone Tool Crafting",
            description: "Creating basic tools from stone and wood",
            era: TechEra::StoneAge,
            prerequisites: vec!["flint_knapping"],
            discovery_difficulty: 15,
            required_items: vec![ItemType::Stone, ItemType::Wood],
            unlocks_recipes: vec![ItemType::StoneHammer],
        });

        self.add_tech(Technology {
            id: "basic_shelter",
            name: "Basic Shelter",
            description: "Constructing simple wooden shelters",
            era: TechEra::StoneAge,
            prerequisites: vec![],
            discovery_difficulty: 5,
            required_items: vec![ItemType::Wood],
            unlocks_recipes: vec![],
        });

        self.add_tech(Technology {
            id: "wooden_tools",
            name: "Wooden Tool Making",
            description: "Crafting basic tools from wood",
            era: TechEra::StoneAge,
            prerequisites: vec![],
            discovery_difficulty: 5,
            required_items: vec![ItemType::Wood],
            unlocks_recipes: vec![ItemType::WoodenAxe, ItemType::WoodenPickaxe, ItemType::WoodenSpear],
        });

        // === COPPER AGE ===

        self.add_tech(Technology {
            id: "native_copper",
            name: "Native Copper Working",
            description: "Cold-hammering native copper into tools (no smelting required)",
            era: TechEra::CopperAge,
            prerequisites: vec!["stone_tools"],
            discovery_difficulty: 40,
            required_items: vec![ItemType::Iron], // Using Iron as copper placeholder
            unlocks_recipes: vec![],
        });

        self.add_tech(Technology {
            id: "pottery",
            name: "Pottery",
            description: "Shaping and firing clay into vessels",
            era: TechEra::CopperAge,
            prerequisites: vec!["fire"],
            discovery_difficulty: 25,
            required_items: vec![ItemType::Clay],
            unlocks_recipes: vec![ItemType::Pottery],
        });

        self.add_tech(Technology {
            id: "basic_smelting",
            name: "Basic Smelting",
            description: "Using hot fires to extract metals from ore",
            era: TechEra::CopperAge,
            prerequisites: vec!["fire", "pottery"],
            discovery_difficulty: 50,
            required_items: vec![ItemType::Coal, ItemType::Iron],
            unlocks_recipes: vec![],
        });

        // === BRONZE AGE ===

        self.add_tech(Technology {
            id: "bronze_alloy",
            name: "Bronze Alloying",
            description: "Combining copper and tin to create stronger bronze",
            era: TechEra::BronzeAge,
            prerequisites: vec!["basic_smelting"],
            discovery_difficulty: 60,
            required_items: vec![ItemType::Iron], // Copper + tin
            unlocks_recipes: vec![],
        });

        self.add_tech(Technology {
            id: "advanced_pottery",
            name: "Advanced Pottery",
            description: "Creating high-quality ceramic vessels and storage",
            era: TechEra::BronzeAge,
            prerequisites: vec!["pottery"],
            discovery_difficulty: 35,
            required_items: vec![ItemType::Clay],
            unlocks_recipes: vec![],
        });

        // === IRON AGE ===

        self.add_tech(Technology {
            id: "iron_smelting",
            name: "Iron Smelting",
            description: "High-temperature furnaces to smelt iron from ore",
            era: TechEra::IronAge,
            prerequisites: vec!["basic_smelting", "bronze_alloy"],
            discovery_difficulty: 70,
            required_items: vec![ItemType::Iron, ItemType::Charcoal],
            unlocks_recipes: vec![ItemType::IronAxe, ItemType::IronPickaxe, ItemType::IronSword],
        });

        self.add_tech(Technology {
            id: "charcoal_making",
            name: "Charcoal Production",
            description: "Converting wood into high-heat charcoal fuel",
            era: TechEra::IronAge,
            prerequisites: vec!["fire"],
            discovery_difficulty: 30,
            required_items: vec![ItemType::Wood],
            unlocks_recipes: vec![ItemType::Charcoal],
        });

        self.add_tech(Technology {
            id: "steel_making",
            name: "Steel Production",
            description: "Carburizing iron with carbon to create steel",
            era: TechEra::IronAge,
            prerequisites: vec!["iron_smelting", "charcoal_making"],
            discovery_difficulty: 80,
            required_items: vec![ItemType::Iron, ItemType::Charcoal],
            unlocks_recipes: vec![ItemType::SteelSword, ItemType::SteelArmor],
        });

        // === MEDIEVAL ===

        self.add_tech(Technology {
            id: "textile_production",
            name: "Textile Production",
            description: "Weaving cloth from plant and animal fibers",
            era: TechEra::Medieval,
            prerequisites: vec![],
            discovery_difficulty: 20,
            required_items: vec![ItemType::Flax],
            unlocks_recipes: vec![ItemType::Cloth, ItemType::Linen],
        });

        self.add_tech(Technology {
            id: "leather_working",
            name: "Leather Working",
            description: "Tanning hides into usable leather",
            era: TechEra::Medieval,
            prerequisites: vec![],
            discovery_difficulty: 25,
            required_items: vec![ItemType::Hides],
            unlocks_recipes: vec![ItemType::Leather],
        });

        self.add_tech(Technology {
            id: "advanced_smithing",
            name: "Advanced Smithing",
            description: "Complex metalworking for armor and weapons",
            era: TechEra::Medieval,
            prerequisites: vec!["iron_smelting"],
            discovery_difficulty: 55,
            required_items: vec![ItemType::Iron],
            unlocks_recipes: vec![ItemType::IronArmor, ItemType::IronHammer],
        });

        self.add_tech(Technology {
            id: "food_preservation",
            name: "Food Preservation",
            description: "Techniques for preserving food through processing",
            era: TechEra::Medieval,
            prerequisites: vec!["fire"],
            discovery_difficulty: 20,
            required_items: vec![ItemType::Food],
            unlocks_recipes: vec![ItemType::Bread, ItemType::Cheese],
        });

        self.add_tech(Technology {
            id: "brewing",
            name: "Brewing",
            description: "Fermenting grains into ale and beer",
            era: TechEra::Medieval,
            prerequisites: vec!["food_preservation"],
            discovery_difficulty: 30,
            required_items: vec![ItemType::Grain],
            unlocks_recipes: vec![ItemType::Ale],
        });

        self.add_tech(Technology {
            id: "glass_making",
            name: "Glass Making",
            description: "Melting sand into glass",
            era: TechEra::Medieval,
            prerequisites: vec!["basic_smelting"],
            discovery_difficulty: 45,
            required_items: vec![ItemType::Sand],
            unlocks_recipes: vec![ItemType::Glass],
        });
    }

    /// Get a technology by ID
    pub fn get(&self, id: &str) -> Option<&Technology> {
        self.technologies.get(id)
    }

    /// Get all technologies in an era
    pub fn get_by_era(&self, era: TechEra) -> Vec<&Technology> {
        self.technologies
            .values()
            .filter(|t| t.era == era)
            .collect()
    }

    /// Get all discoverable technologies (prerequisites met but not yet known)
    pub fn get_discoverable(&self, known_techs: &HashSet<&'static str>) -> Vec<&Technology> {
        self.technologies
            .values()
            .filter(|t| !known_techs.contains(t.id) && t.can_discover(known_techs))
            .collect()
    }

    /// Get all technologies
    pub fn all(&self) -> Vec<&Technology> {
        self.technologies.values().collect()
    }
}

impl Default for TechnologyTree {
    fn default() -> Self {
        Self::new()
    }
}

/// An agent's discovered technologies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownTechnologies {
    /// Technology IDs this agent knows
    known: HashSet<String>,

    /// Experimentation progress toward discovering new techs
    /// Maps tech_id -> progress (0-100)
    experimentation_progress: HashMap<String, u8>,

    /// Technologies discovered by this agent (for prestige)
    discovered_by_self: HashSet<String>,
}

impl KnownTechnologies {
    pub fn new() -> Self {
        let mut known = HashSet::new();

        // Everyone starts with basic knowledge
        known.insert("fire".to_string());
        known.insert("basic_shelter".to_string());

        Self {
            known,
            experimentation_progress: HashMap::new(),
            discovered_by_self: HashSet::new(),
        }
    }

    /// Check if agent knows a technology
    pub fn knows(&self, tech_id: &str) -> bool {
        self.known.contains(tech_id)
    }

    /// Learn a technology (from teaching or discovery)
    pub fn learn(&mut self, tech_id: &str, discovered: bool) {
        self.known.insert(tech_id.to_string());
        if discovered {
            self.discovered_by_self.insert(tech_id.to_string());
        }
        // Clear experimentation progress
        self.experimentation_progress.remove(tech_id);
    }

    /// Add experimentation progress toward a technology
    pub fn add_experimentation(&mut self, tech_id: &str, progress: u8) -> bool {
        let current = self.experimentation_progress.entry(tech_id.to_string()).or_insert(0);
        *current = (*current + progress).min(100);

        // Discovery happens at 100
        if *current >= 100 {
            self.learn(tech_id, true);
            true
        } else {
            false
        }
    }

    /// Get known tech IDs (as a HashSet for compatibility)
    pub fn get_known(&self) -> HashSet<&str> {
        self.known.iter().map(|s| s.as_str()).collect()
    }

    /// Get experimentation progress for a tech
    pub fn get_progress(&self, tech_id: &str) -> u8 {
        *self.experimentation_progress.get(tech_id).unwrap_or(&0)
    }

    /// Check if agent can craft an item (based on tech unlocks)
    pub fn can_craft(&self, item: ItemType, tech_tree: &TechnologyTree) -> bool {
        // Check if any known technology unlocks this item
        for tech_id in &self.known {
            if let Some(tech) = tech_tree.get(tech_id) {
                if tech.unlocks_recipes.contains(&item) {
                    return true;
                }
            }
        }
        false
    }

    /// Get all craftable items
    pub fn get_craftable_items(&self, tech_tree: &TechnologyTree) -> HashSet<ItemType> {
        let mut craftable = HashSet::new();
        for tech_id in &self.known {
            if let Some(tech) = tech_tree.get(tech_id) {
                for item in &tech.unlocks_recipes {
                    craftable.insert(*item);
                }
            }
        }
        craftable
    }

    /// Get current technological era (highest era with known tech)
    pub fn current_era(&self, tech_tree: &TechnologyTree) -> TechEra {
        let mut highest_era = TechEra::StoneAge;

        for tech_id in &self.known {
            if let Some(tech) = tech_tree.get(tech_id) {
                if tech.era > highest_era {
                    highest_era = tech.era;
                }
            }
        }

        highest_era
    }
}

impl Default for KnownTechnologies {
    fn default() -> Self {
        Self::new()
    }
}

/// Discovery event when an agent discovers new technology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEvent {
    pub tech_id: &'static str,
    pub discoverer_id: Uuid,
    pub tick: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_technology_prerequisites() {
        let tree = TechnologyTree::new();
        let mut known = HashSet::new();

        let flint = tree.get("flint_knapping").unwrap();
        assert!(flint.can_discover(&known)); // No prereqs

        let stone_tools = tree.get("stone_tools").unwrap();
        assert!(!stone_tools.can_discover(&known)); // Needs flint_knapping

        known.insert("flint_knapping");
        assert!(stone_tools.can_discover(&known)); // Now can discover
    }

    #[test]
    fn test_discovery_chance() {
        let tree = TechnologyTree::new();
        let tech = tree.get("flint_knapping").unwrap();

        let low_curiosity = tech.discovery_chance(-5);
        let high_curiosity = tech.discovery_chance(10);

        assert!(high_curiosity > low_curiosity);
        assert!(high_curiosity <= 0.5); // Max 50%
    }

    #[test]
    fn test_known_technologies() {
        let mut known = KnownTechnologies::new();

        assert!(known.knows("fire")); // Starts with fire
        assert!(!known.knows("flint_knapping"));

        known.learn("flint_knapping", true);
        assert!(known.knows("flint_knapping"));
        assert!(known.discovered_by_self.contains("flint_knapping"));
    }

    #[test]
    fn test_experimentation_progress() {
        let mut known = KnownTechnologies::new();

        assert!(!known.add_experimentation("flint_knapping", 30));
        assert_eq!(known.get_progress("flint_knapping"), 30);

        assert!(!known.add_experimentation("flint_knapping", 50));
        assert_eq!(known.get_progress("flint_knapping"), 80);

        // Discovery at 100
        assert!(known.add_experimentation("flint_knapping", 30));
        assert!(known.knows("flint_knapping"));
    }

    #[test]
    fn test_tech_eras() {
        let tree = TechnologyTree::new();

        let stone_age = tree.get_by_era(TechEra::StoneAge);
        assert!(!stone_age.is_empty());

        let iron_age = tree.get_by_era(TechEra::IronAge);
        assert!(!iron_age.is_empty());
    }

    #[test]
    fn test_craftable_items() {
        let tree = TechnologyTree::new();
        let mut known = KnownTechnologies::new();

        known.learn("flint_knapping", false);

        let craftable = known.get_craftable_items(&tree);
        assert!(craftable.contains(&ItemType::StoneAxe));
        assert!(craftable.contains(&ItemType::StonePickaxe));
    }

    #[test]
    fn test_current_era() {
        let tree = TechnologyTree::new();
        let mut known = KnownTechnologies::new();

        assert_eq!(known.current_era(&tree), TechEra::StoneAge);

        known.learn("iron_smelting", false);
        assert_eq!(known.current_era(&tree), TechEra::IronAge);
    }
}
