// src/world/crafting.rs
//! Crafting system for EBSS
//!
//! Handles crafting recipes, material requirements, skill checks, and item creation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Material requirement for a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialRequirement {
    pub material_id: String,
    pub quantity: u32,
}

impl MaterialRequirement {
    pub fn new(material_id: String, quantity: u32) -> Self {
        Self { material_id, quantity }
    }
}

/// Tool requirement for crafting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolRequirement {
    None,
    Workbench,
    Anvil,
    Furnace,
    Loom,
    Tannery,
    Alchemy,
}

/// Skill requirement for crafting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirement {
    pub skill_name: String,
    pub min_level: u32,
}

impl SkillRequirement {
    pub fn new(skill_name: String, min_level: u32) -> Self {
        Self { skill_name, min_level }
    }
}

/// Crafting recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingRecipe {
    pub id: String,
    pub name: String,
    pub description: String,

    /// Materials needed
    pub materials: Vec<MaterialRequirement>,

    /// Tool/station required
    pub tool_requirement: ToolRequirement,

    /// Skills needed
    pub skill_requirements: Vec<SkillRequirement>,

    /// Output item
    pub output_item_id: String,
    pub output_quantity: u32,

    /// Crafting time in ticks
    pub crafting_time: u32,

    /// Category for organization
    pub category: CraftingCategory,
}

/// Crafting categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftingCategory {
    Weapons,
    Armor,
    Tools,
    Clothing,
    Furniture,
    Building,
    Food,
    Alchemy,
    Materials,
}

impl CraftingRecipe {
    pub fn new(id: String, name: String, output_item_id: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            materials: Vec::new(),
            tool_requirement: ToolRequirement::None,
            skill_requirements: Vec::new(),
            output_item_id,
            output_quantity: 1,
            crafting_time: 100, // Default 100 ticks
            category: CraftingCategory::Materials,
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn with_material(mut self, material_id: String, quantity: u32) -> Self {
        self.materials.push(MaterialRequirement::new(material_id, quantity));
        self
    }

    pub fn with_tool(mut self, tool: ToolRequirement) -> Self {
        self.tool_requirement = tool;
        self
    }

    pub fn with_skill(mut self, skill_name: String, min_level: u32) -> Self {
        self.skill_requirements.push(SkillRequirement::new(skill_name, min_level));
        self
    }

    pub fn with_quantity(mut self, quantity: u32) -> Self {
        self.output_quantity = quantity;
        self
    }

    pub fn with_time(mut self, ticks: u32) -> Self {
        self.crafting_time = ticks;
        self
    }

    pub fn with_category(mut self, category: CraftingCategory) -> Self {
        self.category = category;
        self
    }
}

/// Recipe registry
#[derive(Debug, Clone)]
pub struct RecipeRegistry {
    recipes: HashMap<String, CraftingRecipe>,
}

impl RecipeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            recipes: HashMap::new(),
        };
        registry.register_all_recipes();
        registry
    }

    fn register(&mut self, recipe: CraftingRecipe) {
        self.recipes.insert(recipe.id.clone(), recipe);
    }

    pub fn get(&self, id: &str) -> Option<&CraftingRecipe> {
        self.recipes.get(id)
    }

    pub fn get_by_category(&self, category: CraftingCategory) -> Vec<&CraftingRecipe> {
        self.recipes
            .values()
            .filter(|r| r.category == category)
            .collect()
    }

    pub fn all_recipes(&self) -> Vec<&CraftingRecipe> {
        self.recipes.values().collect()
    }

    fn register_all_recipes(&mut self) {
        // ===== Weapons =====

        // Stone tools/weapons
        self.register(
            CraftingRecipe::new(
                "stone_axe".to_string(),
                "Stone Axe".to_string(),
                "stone_axe".to_string(),
            )
            .with_description("Basic stone axe for chopping wood".to_string())
            .with_material("wood".to_string(), 2)
            .with_material("stone".to_string(), 3)
            .with_time(50)
            .with_category(CraftingCategory::Tools)
        );

        self.register(
            CraftingRecipe::new(
                "stone_pickaxe".to_string(),
                "Stone Pickaxe".to_string(),
                "stone_pickaxe".to_string(),
            )
            .with_description("Stone pickaxe for mining".to_string())
            .with_material("wood".to_string(), 2)
            .with_material("stone".to_string(), 3)
            .with_time(50)
            .with_category(CraftingCategory::Tools)
        );

        self.register(
            CraftingRecipe::new(
                "stone_spear".to_string(),
                "Stone Spear".to_string(),
                "stone_spear".to_string(),
            )
            .with_description("Simple hunting spear".to_string())
            .with_material("wood".to_string(), 3)
            .with_material("stone".to_string(), 1)
            .with_material("leather".to_string(), 1)
            .with_time(40)
            .with_category(CraftingCategory::Weapons)
        );

        // Iron tools/weapons
        self.register(
            CraftingRecipe::new(
                "iron_sword".to_string(),
                "Iron Sword".to_string(),
                "iron_sword".to_string(),
            )
            .with_description("Well-forged iron sword".to_string())
            .with_material("iron_ingot".to_string(), 3)
            .with_material("wood".to_string(), 1)
            .with_material("leather".to_string(), 1)
            .with_tool(ToolRequirement::Anvil)
            .with_skill("smithing".to_string(), 2)
            .with_time(200)
            .with_category(CraftingCategory::Weapons)
        );

        self.register(
            CraftingRecipe::new(
                "iron_axe".to_string(),
                "Iron Axe".to_string(),
                "iron_axe".to_string(),
            )
            .with_description("Efficient iron axe".to_string())
            .with_material("iron_ingot".to_string(), 2)
            .with_material("wood".to_string(), 2)
            .with_tool(ToolRequirement::Anvil)
            .with_skill("smithing".to_string(), 1)
            .with_time(150)
            .with_category(CraftingCategory::Tools)
        );

        self.register(
            CraftingRecipe::new(
                "iron_pickaxe".to_string(),
                "Iron Pickaxe".to_string(),
                "iron_pickaxe".to_string(),
            )
            .with_description("Durable iron pickaxe".to_string())
            .with_material("iron_ingot".to_string(), 3)
            .with_material("wood".to_string(), 2)
            .with_tool(ToolRequirement::Anvil)
            .with_skill("smithing".to_string(), 1)
            .with_time(150)
            .with_category(CraftingCategory::Tools)
        );

        // ===== Armor =====

        // Leather armor
        self.register(
            CraftingRecipe::new(
                "leather_helmet".to_string(),
                "Leather Helmet".to_string(),
                "leather_helmet".to_string(),
            )
            .with_description("Basic leather head protection".to_string())
            .with_material("leather".to_string(), 3)
            .with_material("thread".to_string(), 2)
            .with_tool(ToolRequirement::Workbench)
            .with_time(100)
            .with_category(CraftingCategory::Armor)
        );

        self.register(
            CraftingRecipe::new(
                "leather_chestplate".to_string(),
                "Leather Chestplate".to_string(),
                "leather_chestplate".to_string(),
            )
            .with_description("Leather chest armor".to_string())
            .with_material("leather".to_string(), 8)
            .with_material("thread".to_string(), 4)
            .with_tool(ToolRequirement::Workbench)
            .with_time(150)
            .with_category(CraftingCategory::Armor)
        );

        self.register(
            CraftingRecipe::new(
                "leather_boots".to_string(),
                "Leather Boots".to_string(),
                "leather_boots".to_string(),
            )
            .with_description("Leather foot protection".to_string())
            .with_material("leather".to_string(), 4)
            .with_material("thread".to_string(), 2)
            .with_tool(ToolRequirement::Workbench)
            .with_time(80)
            .with_category(CraftingCategory::Armor)
        );

        // Iron armor
        self.register(
            CraftingRecipe::new(
                "iron_helmet".to_string(),
                "Iron Helmet".to_string(),
                "iron_helmet".to_string(),
            )
            .with_description("Heavy iron helmet".to_string())
            .with_material("iron_ingot".to_string(), 5)
            .with_material("leather".to_string(), 2)
            .with_tool(ToolRequirement::Anvil)
            .with_skill("smithing".to_string(), 3)
            .with_time(250)
            .with_category(CraftingCategory::Armor)
        );

        self.register(
            CraftingRecipe::new(
                "iron_chestplate".to_string(),
                "Iron Chestplate".to_string(),
                "iron_chestplate".to_string(),
            )
            .with_description("Strong iron chest armor".to_string())
            .with_material("iron_ingot".to_string(), 12)
            .with_material("leather".to_string(), 4)
            .with_tool(ToolRequirement::Anvil)
            .with_skill("smithing".to_string(), 4)
            .with_time(400)
            .with_category(CraftingCategory::Armor)
        );

        // ===== Materials =====

        self.register(
            CraftingRecipe::new(
                "planks".to_string(),
                "Wooden Planks".to_string(),
                "planks".to_string(),
            )
            .with_description("Process logs into planks".to_string())
            .with_material("wood".to_string(), 1)
            .with_quantity(4)
            .with_time(20)
            .with_category(CraftingCategory::Materials)
        );

        self.register(
            CraftingRecipe::new(
                "thread".to_string(),
                "Thread".to_string(),
                "thread".to_string(),
            )
            .with_description("Spin fibers into thread".to_string())
            .with_material("plant_fiber".to_string(), 2)
            .with_quantity(3)
            .with_time(30)
            .with_category(CraftingCategory::Materials)
        );

        self.register(
            CraftingRecipe::new(
                "leather".to_string(),
                "Leather".to_string(),
                "leather".to_string(),
            )
            .with_description("Tan hide into leather".to_string())
            .with_material("raw_hide".to_string(), 1)
            .with_tool(ToolRequirement::Tannery)
            .with_time(150)
            .with_category(CraftingCategory::Materials)
        );

        // ===== Clothing =====

        self.register(
            CraftingRecipe::new(
                "simple_tunic".to_string(),
                "Simple Tunic".to_string(),
                "simple_tunic".to_string(),
            )
            .with_description("Basic cloth tunic".to_string())
            .with_material("cloth".to_string(), 4)
            .with_material("thread".to_string(), 2)
            .with_tool(ToolRequirement::Workbench)
            .with_time(120)
            .with_category(CraftingCategory::Clothing)
        );

        self.register(
            CraftingRecipe::new(
                "wool_cloak".to_string(),
                "Wool Cloak".to_string(),
                "wool_cloak".to_string(),
            )
            .with_description("Warm wool cloak".to_string())
            .with_material("wool".to_string(), 6)
            .with_material("thread".to_string(), 3)
            .with_tool(ToolRequirement::Loom)
            .with_time(180)
            .with_category(CraftingCategory::Clothing)
        );

        // ===== Furniture/Building =====

        self.register(
            CraftingRecipe::new(
                "wooden_chest".to_string(),
                "Wooden Chest".to_string(),
                "wooden_chest".to_string(),
            )
            .with_description("Storage chest".to_string())
            .with_material("planks".to_string(), 8)
            .with_material("iron_ingot".to_string(), 1)
            .with_tool(ToolRequirement::Workbench)
            .with_time(200)
            .with_category(CraftingCategory::Furniture)
        );

        self.register(
            CraftingRecipe::new(
                "workbench".to_string(),
                "Workbench".to_string(),
                "workbench".to_string(),
            )
            .with_description("Crafting station".to_string())
            .with_material("planks".to_string(), 10)
            .with_material("stone".to_string(), 4)
            .with_time(150)
            .with_category(CraftingCategory::Building)
        );

        self.register(
            CraftingRecipe::new(
                "anvil".to_string(),
                "Anvil".to_string(),
                "anvil".to_string(),
            )
            .with_description("Smithing station".to_string())
            .with_material("iron_ingot".to_string(), 15)
            .with_tool(ToolRequirement::Furnace)
            .with_skill("smithing".to_string(), 2)
            .with_time(500)
            .with_category(CraftingCategory::Building)
        );
    }
}

/// Crafting result
#[derive(Debug, Clone)]
pub enum CraftingResult {
    Success {
        item_id: String,
        quantity: u32,
    },
    InsufficientMaterials {
        missing: Vec<(String, u32)>,
    },
    InsufficientSkill {
        skill: String,
        required: u32,
        current: u32,
    },
    MissingTool {
        tool: ToolRequirement,
    },
    RecipeNotFound,
}

/// Active crafting job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingJob {
    pub id: Uuid,
    pub recipe_id: String,
    pub crafter_id: Uuid,
    pub progress: u32,
    pub total_time: u32,
}

/// Completed craft ready for collection
#[derive(Debug, Clone)]
pub struct CompletedCraft {
    pub crafter_id: Uuid,
    pub item_id: String,
    pub quantity: u32,
}

/// Crafting manager
#[derive(Debug, Clone)]
pub struct CraftingManager {
    registry: RecipeRegistry,
    active_jobs: Vec<CraftingJob>,
    /// Completed crafts waiting to be collected by agents
    pending_completions: Vec<CompletedCraft>,
}

impl Default for CraftingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftingManager {
    pub fn new() -> Self {
        Self {
            registry: RecipeRegistry::new(),
            active_jobs: Vec::new(),
            pending_completions: Vec::new(),
        }
    }

    /// Get a recipe by ID
    pub fn get_recipe(&self, recipe_id: &str) -> Option<&CraftingRecipe> {
        self.registry.get(recipe_id)
    }

    /// Get all recipes in a category
    pub fn get_recipes_by_category(&self, category: CraftingCategory) -> Vec<&CraftingRecipe> {
        self.registry.get_by_category(category)
    }

    /// Get all recipes
    pub fn all_recipes(&self) -> Vec<&CraftingRecipe> {
        self.registry.all_recipes()
    }

    /// Check if an agent can craft a recipe
    pub fn can_craft(
        &self,
        recipe_id: &str,
        inventory_materials: &HashMap<String, u32>,
        skills: &HashMap<String, u32>,
        available_tools: &[ToolRequirement],
    ) -> CraftingResult {
        let recipe = match self.registry.get(recipe_id) {
            Some(r) => r,
            None => return CraftingResult::RecipeNotFound,
        };

        // Check materials
        let mut missing = Vec::new();
        for req in &recipe.materials {
            let available = inventory_materials.get(&req.material_id).copied().unwrap_or(0);
            if available < req.quantity {
                missing.push((req.material_id.clone(), req.quantity - available));
            }
        }

        if !missing.is_empty() {
            return CraftingResult::InsufficientMaterials { missing };
        }

        // Check tool requirement
        if recipe.tool_requirement != ToolRequirement::None {
            if !available_tools.contains(&recipe.tool_requirement) {
                return CraftingResult::MissingTool {
                    tool: recipe.tool_requirement,
                };
            }
        }

        // Check skill requirements
        for skill_req in &recipe.skill_requirements {
            let skill_level = skills.get(&skill_req.skill_name).copied().unwrap_or(0);
            if skill_level < skill_req.min_level {
                return CraftingResult::InsufficientSkill {
                    skill: skill_req.skill_name.clone(),
                    required: skill_req.min_level,
                    current: skill_level,
                };
            }
        }

        CraftingResult::Success {
            item_id: recipe.output_item_id.clone(),
            quantity: recipe.output_quantity,
        }
    }

    /// Start a crafting job
    pub fn start_crafting(
        &mut self,
        recipe_id: String,
        crafter_id: Uuid,
    ) -> Option<Uuid> {
        let recipe = self.registry.get(&recipe_id)?;

        let job = CraftingJob {
            id: Uuid::new_v4(),
            recipe_id,
            crafter_id,
            progress: 0,
            total_time: recipe.crafting_time,
        };

        let job_id = job.id;
        self.active_jobs.push(job);
        Some(job_id)
    }

    /// Update crafting jobs and store completed crafts for collection
    pub fn tick(&mut self) {
        for job in &mut self.active_jobs {
            job.progress += 1;

            if job.progress >= job.total_time {
                if let Some(recipe) = self.registry.get(&job.recipe_id) {
                    self.pending_completions.push(CompletedCraft {
                        crafter_id: job.crafter_id,
                        item_id: recipe.output_item_id.clone(),
                        quantity: recipe.output_quantity,
                    });
                }
            }
        }

        // Remove completed jobs
        self.active_jobs.retain(|job| job.progress < job.total_time);
    }



    /// Get active jobs for a crafter
    pub fn get_crafter_jobs(&self, crafter_id: &Uuid) -> Vec<&CraftingJob> {
        self.active_jobs
            .iter()
            .filter(|job| job.crafter_id == *crafter_id)
            .collect()
    }

}
