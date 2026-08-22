// src/agents/equipment.rs
//! Equipment and clothing system with durability and temperature protection.

use crate::agents::body::BodyPartType;
use crate::agents::skills::Quality;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of equipment slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    // Clothing/Armor slots
    Head,
    Torso,
    Back,
    Arms,   // Covers both arms
    Legs,   // Covers both legs
    Hands,  // Gloves/gauntlets
    Feet,   // Boots/shoes
    Neck,   // Amulets/necklaces
    Finger, // Rings

    // Weapon/Tool slots
    MainHand,  // Primary weapon or tool
    OffHand,   // Shield, secondary weapon, or torch
}

impl EquipmentSlot {
    /// Get the body parts this slot covers
    pub fn covered_parts(&self) -> Vec<BodyPartType> {
        match self {
            EquipmentSlot::Head => vec![BodyPartType::Head],
            EquipmentSlot::Torso => vec![BodyPartType::Torso],
            EquipmentSlot::Back => vec![BodyPartType::Back],
            EquipmentSlot::Arms => vec![BodyPartType::LeftArm, BodyPartType::RightArm],
            EquipmentSlot::Legs => vec![BodyPartType::LeftLeg, BodyPartType::RightLeg],
            EquipmentSlot::Hands => vec![BodyPartType::LeftArm, BodyPartType::RightArm], // Hands are part of arms
            EquipmentSlot::Feet => vec![BodyPartType::LeftLeg, BodyPartType::RightLeg], // Feet are part of legs
            EquipmentSlot::Neck => vec![BodyPartType::Head], // Neck accessories near head
            EquipmentSlot::Finger => vec![], // Rings don't cover body parts
            EquipmentSlot::MainHand => vec![BodyPartType::RightArm],
            EquipmentSlot::OffHand => vec![BodyPartType::LeftArm],
        }
    }

    /// Check if this is a weapon/tool slot
    pub fn is_weapon_slot(&self) -> bool {
        matches!(self, EquipmentSlot::MainHand | EquipmentSlot::OffHand)
    }

    /// Check if this is an armor/clothing slot
    pub fn is_armor_slot(&self) -> bool {
        !self.is_weapon_slot()
    }
}

/// Material type for clothing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClothingMaterial {
    Leather,
    Fur,
    Wool,
    Linen,
    Cotton,
    Hide,
    Bark,
}

impl ClothingMaterial {
    /// Base durability for this material
    pub fn base_durability(&self) -> f32 {
        match self {
            ClothingMaterial::Leather => 100.0,
            ClothingMaterial::Fur => 80.0,
            ClothingMaterial::Wool => 60.0,
            ClothingMaterial::Linen => 50.0,
            ClothingMaterial::Cotton => 55.0,
            ClothingMaterial::Hide => 120.0,
            ClothingMaterial::Bark => 40.0,
        }
    }

    /// Cold insulation multiplier
    pub fn cold_insulation(&self) -> f32 {
        match self {
            ClothingMaterial::Fur => 1.5,      // Excellent cold protection
            ClothingMaterial::Wool => 1.3,     // Very good cold protection
            ClothingMaterial::Hide => 1.2,     // Good cold protection
            ClothingMaterial::Leather => 1.1,  // Moderate cold protection
            ClothingMaterial::Cotton => 0.9,   // Light cold protection
            ClothingMaterial::Linen => 0.8,    // Minimal cold protection
            ClothingMaterial::Bark => 0.7,     // Poor cold protection
        }
    }

    /// Heat insulation multiplier (breathability - higher is better in heat)
    pub fn heat_resistance(&self) -> f32 {
        match self {
            ClothingMaterial::Linen => 1.4,    // Excellent in heat
            ClothingMaterial::Cotton => 1.3,   // Very good in heat
            ClothingMaterial::Bark => 1.1,     // Decent in heat
            ClothingMaterial::Leather => 0.9,  // Poor in heat
            ClothingMaterial::Wool => 0.7,     // Bad in heat
            ClothingMaterial::Hide => 0.6,     // Very bad in heat
            ClothingMaterial::Fur => 0.5,      // Terrible in heat
        }
    }

    /// Armor protection multiplier
    pub fn armor_multiplier(&self) -> f32 {
        match self {
            ClothingMaterial::Hide => 1.5,
            ClothingMaterial::Leather => 1.3,
            ClothingMaterial::Fur => 1.1,
            ClothingMaterial::Wool => 0.8,
            ClothingMaterial::Cotton => 0.7,
            ClothingMaterial::Linen => 0.6,
            ClothingMaterial::Bark => 1.0,
        }
    }

    /// Value multiplier for trade/comparison purposes
    pub fn value_multiplier(&self) -> f32 {
        match self {
            ClothingMaterial::Fur => 1.8,
            ClothingMaterial::Leather => 1.5,
            ClothingMaterial::Hide => 1.3,
            ClothingMaterial::Wool => 1.0,
            ClothingMaterial::Cotton => 0.9,
            ClothingMaterial::Linen => 0.8,
            ClothingMaterial::Bark => 0.4,
        }
    }
}

/// Metal materials for weapons and armor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetalMaterial {
    Copper,
    Bronze,
    Iron,
    Steel,
    Gold,     // Decorative, poor weapon material
    Silver,   // Decorative, moderate weapon material
}

impl MetalMaterial {
    /// Base durability for metal items
    pub fn base_durability(&self) -> f32 {
        match self {
            MetalMaterial::Copper => 80.0,
            MetalMaterial::Bronze => 150.0,
            MetalMaterial::Iron => 200.0,
            MetalMaterial::Steel => 300.0,
            MetalMaterial::Gold => 50.0,
            MetalMaterial::Silver => 70.0,
        }
    }

    /// Damage multiplier for weapons
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            MetalMaterial::Steel => 1.5,
            MetalMaterial::Iron => 1.3,
            MetalMaterial::Bronze => 1.2,
            MetalMaterial::Copper => 1.0,
            MetalMaterial::Silver => 0.9,
            MetalMaterial::Gold => 0.6,
        }
    }

    /// Armor protection multiplier
    pub fn armor_multiplier(&self) -> f32 {
        match self {
            MetalMaterial::Steel => 2.0,
            MetalMaterial::Iron => 1.7,
            MetalMaterial::Bronze => 1.5,
            MetalMaterial::Copper => 1.2,
            MetalMaterial::Silver => 1.0,
            MetalMaterial::Gold => 0.8,
        }
    }

    /// Mining/harvesting efficiency bonus
    pub fn tool_efficiency(&self) -> f32 {
        match self {
            MetalMaterial::Steel => 1.5,
            MetalMaterial::Iron => 1.3,
            MetalMaterial::Bronze => 1.2,
            MetalMaterial::Copper => 1.0,
            MetalMaterial::Silver => 0.8,
            MetalMaterial::Gold => 0.5,
        }
    }

    /// Value multiplier for trade/comparison purposes
    pub fn value_multiplier(&self) -> f32 {
        match self {
            MetalMaterial::Gold => 5.0,   // Precious metal
            MetalMaterial::Silver => 3.0, // Precious metal
            MetalMaterial::Steel => 2.0,
            MetalMaterial::Iron => 1.5,
            MetalMaterial::Bronze => 1.2,
            MetalMaterial::Copper => 1.0,
        }
    }
}

/// Wood materials for weapons and tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WoodMaterial {
    Pine,
    Oak,
    Birch,
    Ash,
    Yew,
    Ironwood,
}

impl WoodMaterial {
    /// Base durability for wooden items
    pub fn base_durability(&self) -> f32 {
        match self {
            WoodMaterial::Pine => 40.0,
            WoodMaterial::Oak => 70.0,
            WoodMaterial::Birch => 50.0,
            WoodMaterial::Ash => 65.0,
            WoodMaterial::Yew => 60.0,
            WoodMaterial::Ironwood => 90.0,
        }
    }

    /// Damage multiplier for weapons
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            WoodMaterial::Ironwood => 0.9,
            WoodMaterial::Oak => 0.8,
            WoodMaterial::Ash => 0.85,
            WoodMaterial::Yew => 0.8,
            WoodMaterial::Birch => 0.7,
            WoodMaterial::Pine => 0.6,
        }
    }

    /// Flexibility (for bows)
    pub fn flexibility(&self) -> f32 {
        match self {
            WoodMaterial::Yew => 1.5,
            WoodMaterial::Ash => 1.3,
            WoodMaterial::Birch => 1.2,
            WoodMaterial::Oak => 0.9,
            WoodMaterial::Ironwood => 0.7,
            WoodMaterial::Pine => 1.0,
        }
    }
}

/// Stone materials for primitive tools and weapons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoneMaterial {
    Flint,
    Granite,
    Obsidian,
    Limestone,
}

impl StoneMaterial {
    /// Base durability for stone items
    pub fn base_durability(&self) -> f32 {
        match self {
            StoneMaterial::Obsidian => 50.0,
            StoneMaterial::Flint => 45.0,
            StoneMaterial::Granite => 60.0,
            StoneMaterial::Limestone => 30.0,
        }
    }

    /// Damage multiplier for weapons
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            StoneMaterial::Obsidian => 1.2, // Sharp but brittle
            StoneMaterial::Flint => 1.0,
            StoneMaterial::Granite => 0.9,
            StoneMaterial::Limestone => 0.7,
        }
    }

    /// Sharpness retention (affects durability loss when used)
    pub fn sharpness_retention(&self) -> f32 {
        match self {
            StoneMaterial::Obsidian => 0.5, // Loses sharpness quickly
            StoneMaterial::Flint => 0.8,
            StoneMaterial::Granite => 0.9,
            StoneMaterial::Limestone => 0.6,
        }
    }
}

/// General equipment material type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipmentMaterial {
    Cloth(ClothingMaterial),
    Metal(MetalMaterial),
    Wood(WoodMaterial),
    Stone(StoneMaterial),
}

impl EquipmentMaterial {
    /// Get base durability for this material
    pub fn base_durability(&self) -> f32 {
        match self {
            EquipmentMaterial::Cloth(m) => m.base_durability(),
            EquipmentMaterial::Metal(m) => m.base_durability(),
            EquipmentMaterial::Wood(m) => m.base_durability(),
            EquipmentMaterial::Stone(m) => m.base_durability(),
        }
    }

    /// Get damage multiplier (for weapons)
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            EquipmentMaterial::Metal(m) => m.damage_multiplier(),
            EquipmentMaterial::Wood(m) => m.damage_multiplier(),
            EquipmentMaterial::Stone(m) => m.damage_multiplier(),
            EquipmentMaterial::Cloth(_) => 0.5, // Cloth weapons are terrible
        }
    }

    /// Get armor protection multiplier
    pub fn armor_multiplier(&self) -> f32 {
        match self {
            EquipmentMaterial::Cloth(m) => m.armor_multiplier(),
            EquipmentMaterial::Metal(m) => m.armor_multiplier(),
            EquipmentMaterial::Wood(_) => 0.6,
            EquipmentMaterial::Stone(_) => 0.5,
        }
    }

    /// Get cold insulation multiplier
    pub fn cold_insulation(&self) -> f32 {
        match self {
            EquipmentMaterial::Cloth(m) => m.cold_insulation(),
            EquipmentMaterial::Metal(_) => 0.3, // Metal is cold
            EquipmentMaterial::Wood(_) => 0.5,
            EquipmentMaterial::Stone(_) => 0.4,
        }
    }

    /// Get value multiplier for trade/comparison purposes
    pub fn value_multiplier(&self) -> f32 {
        match self {
            EquipmentMaterial::Cloth(m) => m.value_multiplier(),
            EquipmentMaterial::Metal(m) => m.value_multiplier(),
            EquipmentMaterial::Wood(_) => 0.8,
            EquipmentMaterial::Stone(_) => 0.6,
        }
    }

    /// Get heat resistance multiplier
    pub fn heat_resistance(&self) -> f32 {
        match self {
            EquipmentMaterial::Cloth(m) => m.heat_resistance(),
            EquipmentMaterial::Metal(_) => 0.2, // Metal heats up
            EquipmentMaterial::Wood(_) => 0.7,
            EquipmentMaterial::Stone(_) => 0.6,
        }
    }

    /// Check if this is a primitive material (wood or stone, not metal)
    /// Used by Traditionalist trait for bonus efficiency
    pub fn is_primitive(&self) -> bool {
        matches!(self, EquipmentMaterial::Wood(_) | EquipmentMaterial::Stone(_))
    }
}

/// Equipment type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipmentType {
    // Clothing/Armor
    Clothing,
    LightArmor,
    MediumArmor,
    HeavyArmor,
    Shield,

    // Weapons
    Sword,
    Axe,
    Spear,
    Mace,
    Dagger,
    Bow,
    Crossbow,

    // Tools
    Pickaxe,
    Shovel,
    Hatchet,
    Hammer,
    Sickle,
    FishingRod,

    // Utility
    Torch,
    Lantern,

    // Accessories (jewelry)
    Ring,       // Worn on finger, provides bonuses
    Necklace,   // Worn on neck, provides bonuses
    Amulet,     // Worn on neck, often magical/protective
    Bracelet,   // Worn on arms/wrists
}

impl EquipmentType {
    /// Can this equipment type be used as a weapon?
    pub fn is_weapon(&self) -> bool {
        matches!(
            self,
            EquipmentType::Sword
                | EquipmentType::Axe
                | EquipmentType::Spear
                | EquipmentType::Mace
                | EquipmentType::Dagger
                | EquipmentType::Bow
                | EquipmentType::Crossbow
        )
    }

    /// Can this equipment type be used as a tool?
    pub fn is_tool(&self) -> bool {
        matches!(
            self,
            EquipmentType::Pickaxe
                | EquipmentType::Shovel
                | EquipmentType::Hatchet
                | EquipmentType::Hammer
                | EquipmentType::Sickle
                | EquipmentType::FishingRod
        )
    }

    /// Is this armor or clothing?
    pub fn is_armor(&self) -> bool {
        matches!(
            self,
            EquipmentType::Clothing
                | EquipmentType::LightArmor
                | EquipmentType::MediumArmor
                | EquipmentType::HeavyArmor
                | EquipmentType::Shield
        )
    }

    /// Is this an accessory (jewelry)?
    pub fn is_accessory(&self) -> bool {
        matches!(
            self,
            EquipmentType::Ring
                | EquipmentType::Necklace
                | EquipmentType::Amulet
                | EquipmentType::Bracelet
        )
    }

    /// Get base damage for this weapon type
    pub fn base_damage(&self) -> f32 {
        match self {
            EquipmentType::Sword => 8.0,
            EquipmentType::Axe => 10.0,
            EquipmentType::Spear => 7.0,
            EquipmentType::Mace => 9.0,
            EquipmentType::Dagger => 5.0,
            EquipmentType::Bow => 6.0,
            EquipmentType::Crossbow => 8.0,
            EquipmentType::Hatchet => 6.0, // Can be used as weapon
            EquipmentType::Pickaxe => 5.0, // Can be used as weapon
            _ => 2.0, // Improvised weapon
        }
    }

    /// Get attack speed multiplier
    pub fn attack_speed(&self) -> f32 {
        match self {
            EquipmentType::Dagger => 1.5,
            EquipmentType::Sword => 1.2,
            EquipmentType::Spear => 1.1,
            EquipmentType::Mace => 1.0,
            EquipmentType::Axe => 0.9,
            EquipmentType::Bow => 0.8,
            EquipmentType::Crossbow => 0.6,
            _ => 1.0,
        }
    }

    /// Get mining/harvesting efficiency for tools
    pub fn tool_efficiency(&self) -> f32 {
        match self {
            EquipmentType::Pickaxe => 1.5,
            EquipmentType::Hatchet => 1.4,
            EquipmentType::Shovel => 1.3,
            EquipmentType::Sickle => 1.2,
            _ => 1.0,
        }
    }
}

/// A piece of equipment or clothing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub id: Uuid,
    pub name: String,
    pub slot: EquipmentSlot,
    pub material: ClothingMaterial,
    pub quality: Quality,

    /// Current durability
    pub durability: f32,
    /// Maximum durability
    pub max_durability: f32,

    /// Base cold insulation value (0.0 to 1.0)
    pub base_cold_insulation: f32,
    /// Base heat resistance value (0.0 to 1.0)
    pub base_heat_resistance: f32,
    /// Base armor protection (0.0 to 1.0)
    pub base_armor: f32,

    /// Wear rate per tick
    pub wear_rate: f32,
}

impl Equipment {
    /// Create new equipment
    pub fn new(
        name: String,
        slot: EquipmentSlot,
        material: ClothingMaterial,
        quality: Quality,
        base_cold_insulation: f32,
        base_heat_resistance: f32,
        base_armor: f32,
    ) -> Self {
        let quality_mult = quality.tool_durability_modifier();
        let max_durability = material.base_durability() * quality_mult;

        Self {
            id: Uuid::new_v4(),
            name,
            slot,
            material,
            quality,
            durability: max_durability,
            max_durability,
            base_cold_insulation,
            base_heat_resistance,
            base_armor,
            wear_rate: 0.01,
        }
    }

    pub fn cold_insulation(&self) -> f32 {
        let material_mult = self.material.cold_insulation();
        let quality_mult = self.quality.modifier();
        let durability_mult = self.durability_percentage();
        self.base_cold_insulation * material_mult * quality_mult * durability_mult
    }

    pub fn heat_resistance(&self) -> f32 {
        let material_mult = self.material.heat_resistance();
        let quality_mult = self.quality.modifier();
        let durability_mult = self.durability_percentage();
        self.base_heat_resistance * material_mult * quality_mult * durability_mult
    }

    pub fn armor_protection(&self) -> f32 {
        let material_mult = self.material.armor_multiplier();
        let quality_mult = self.quality.modifier();
        let durability_mult = self.durability_percentage();
        (self.base_armor * material_mult * quality_mult * durability_mult).min(0.95)
    }

    pub fn durability_percentage(&self) -> f32 {
        if self.max_durability > 0.0 {
            self.durability / self.max_durability
        } else {
            0.0
        }
    }

    pub fn apply_wear(&mut self, amount: f32) {
        self.durability = (self.durability - amount).max(0.0);
    }

    pub fn tick_wear(&mut self) {
        self.apply_wear(self.wear_rate);
    }

    pub fn is_broken(&self) -> bool {
        self.durability <= 0.0
    }

    pub fn repair(&mut self, amount: f32) {
        self.durability = (self.durability + amount).min(self.max_durability);
    }
}

/// New comprehensive equipment item (replaces Equipment eventually)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentItem {
    pub id: Uuid,
    pub name: String,
    pub equipment_type: EquipmentType,
    pub slot: EquipmentSlot,
    pub material: EquipmentMaterial,
    pub quality: Quality,

    /// Current durability
    pub durability: f32,
    /// Maximum durability
    pub max_durability: f32,

    // Stats for armor/clothing
    pub base_cold_insulation: f32,
    pub base_heat_resistance: f32,
    pub base_armor: f32,

    // Stats for weapons
    pub base_damage: f32,
    pub attack_speed: f32,
    pub reach: f32, // Attack range

    // Stats for tools
    pub mining_speed: f32,
    pub harvesting_speed: f32,

    // Durability
    pub wear_rate: f32,
    pub repair_material: Option<String>, // What material is needed to repair

    /// Weight in kg
    pub weight: f32,
}

impl EquipmentItem {
    /// Create a new equipment item
    pub fn new(
        name: String,
        equipment_type: EquipmentType,
        slot: EquipmentSlot,
        material: EquipmentMaterial,
        quality: Quality,
    ) -> Self {
        let quality_mult = quality.tool_durability_modifier();
        let max_durability = material.base_durability() * quality_mult;

        // Calculate base stats based on type
        let (base_damage, attack_speed, reach) = if equipment_type.is_weapon() {
            (
                equipment_type.base_damage() * material.damage_multiplier(),
                equipment_type.attack_speed(),
                match equipment_type {
                    EquipmentType::Spear => 2.0,
                    EquipmentType::Sword => 1.5,
                    EquipmentType::Dagger => 1.0,
                    _ => 1.2,
                },
            )
        } else {
            (0.0, 1.0, 1.0)
        };

        let (mining_speed, harvesting_speed) = if equipment_type.is_tool() {
            let efficiency = equipment_type.tool_efficiency();
            match equipment_type {
                EquipmentType::Pickaxe => (efficiency * 1.5, efficiency * 0.5),
                EquipmentType::Hatchet => (efficiency * 0.5, efficiency * 1.5),
                EquipmentType::Shovel => (efficiency * 1.2, efficiency * 0.8),
                EquipmentType::Sickle => (efficiency * 0.3, efficiency * 1.7),
                _ => (efficiency, efficiency),
            }
        } else {
            (1.0, 1.0)
        };

        let weight = match equipment_type {
            EquipmentType::HeavyArmor => 15.0,
            EquipmentType::MediumArmor => 10.0,
            EquipmentType::LightArmor => 5.0,
            EquipmentType::Shield => 3.0,
            EquipmentType::Clothing => 1.0,
            EquipmentType::Sword | EquipmentType::Axe | EquipmentType::Mace => 2.5,
            EquipmentType::Spear => 2.0,
            EquipmentType::Dagger => 0.5,
            EquipmentType::Bow => 1.0,
            EquipmentType::Pickaxe | EquipmentType::Hatchet => 2.0,
            EquipmentType::Shovel => 1.5,
            _ => 1.0,
        };

        Self {
            id: Uuid::new_v4(),
            name,
            equipment_type,
            slot,
            material,
            quality,
            durability: max_durability,
            max_durability,
            base_cold_insulation: if equipment_type.is_armor() {
                0.3 * material.cold_insulation()
            } else {
                0.0
            },
            base_heat_resistance: if equipment_type.is_armor() {
                0.3 * material.heat_resistance()
            } else {
                0.0
            },
            base_armor: if equipment_type.is_armor() {
                match equipment_type {
                    EquipmentType::HeavyArmor => 0.7 * material.armor_multiplier(),
                    EquipmentType::MediumArmor => 0.5 * material.armor_multiplier(),
                    EquipmentType::LightArmor => 0.3 * material.armor_multiplier(),
                    EquipmentType::Shield => 0.4 * material.armor_multiplier(),
                    EquipmentType::Clothing => 0.1 * material.armor_multiplier(),
                    _ => 0.0,
                }
            } else {
                0.0
            },
            base_damage,
            attack_speed,
            reach,
            mining_speed,
            harvesting_speed,
            wear_rate: 0.01,
            repair_material: None,
            weight,
        }
    }

    /// Get effective damage with quality and durability
    pub fn effective_damage(&self) -> f32 {
        self.base_damage * self.quality.modifier() * self.durability_multiplier()
    }

    /// Get effective armor with quality and durability
    pub fn effective_armor(&self) -> f32 {
        (self.base_armor * self.quality.modifier() * self.durability_multiplier()).min(0.95)
    }

    /// Get effective mining speed
    pub fn effective_mining_speed(&self) -> f32 {
        self.mining_speed * self.quality.modifier() * self.durability_multiplier()
    }

    /// Get effective harvesting speed
    pub fn effective_harvesting_speed(&self) -> f32 {
        self.harvesting_speed * self.quality.modifier() * self.durability_multiplier()
    }

    /// Get effective cold insulation
    pub fn effective_cold_insulation(&self) -> f32 {
        self.base_cold_insulation * self.quality.modifier() * self.durability_multiplier()
    }

    /// Get effective heat resistance
    pub fn effective_heat_resistance(&self) -> f32 {
        self.base_heat_resistance * self.quality.modifier() * self.durability_multiplier()
    }

    /// Get durability as percentage
    pub fn durability_percentage(&self) -> f32 {
        if self.max_durability > 0.0 {
            self.durability / self.max_durability
        } else {
            0.0
        }
    }

    /// Get durability multiplier for effectiveness
    fn durability_multiplier(&self) -> f32 {
        self.durability_percentage()
    }

    /// Apply wear to the equipment
    pub fn apply_wear(&mut self, amount: f32) {
        self.durability = (self.durability - amount).max(0.0);
    }

    /// Tick wear (called each game tick while equipped/used)
    pub fn tick_wear(&mut self) {
        self.apply_wear(self.wear_rate);
    }

    /// Check if equipment is broken
    pub fn is_broken(&self) -> bool {
        self.durability <= 0.0
    }

    /// Repair equipment
    pub fn repair(&mut self, amount: f32) {
        self.durability = (self.durability + amount).min(self.max_durability);
    }

    /// Get equipment condition description
    pub fn condition_description(&self) -> &'static str {
        let pct = self.durability_percentage();
        match pct {
            p if p >= 0.9 => "Pristine",
            p if p >= 0.75 => "Excellent",
            p if p >= 0.5 => "Good",
            p if p >= 0.25 => "Worn",
            p if p >= 0.1 => "Damaged",
            p if p > 0.0 => "Nearly Broken",
            _ => "Broken",
        }
    }
}

/// Equipment Manager - manages all equipped items for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentManager {
    /// Currently equipped items by slot
    equipped: std::collections::HashMap<EquipmentSlot, EquipmentItem>,

    /// Total weight of equipped items
    total_weight: f32,

    /// Maximum carry weight before penalties
    max_carry_weight: f32,
}

impl EquipmentManager {
    /// Create a new equipment manager
    pub fn new(max_carry_weight: f32) -> Self {
        Self {
            equipped: std::collections::HashMap::new(),
            total_weight: 0.0,
            max_carry_weight,
        }
    }

    /// Equip an item to a slot
    pub fn equip(&mut self, item: EquipmentItem) -> Result<Option<EquipmentItem>, String> {
        // Check if slot can accept this item
        if !Self::slot_accepts_type(&item.slot, &item.equipment_type) {
            return Err(format!("{:?} cannot be equipped to {:?} slot", item.equipment_type, item.slot));
        }

        // Check weight limit
        let new_weight = self.total_weight - self.get_slot_weight(&item.slot) + item.weight;
        if new_weight > self.max_carry_weight * 2.0 {
            // Allow up to 2x max weight (with severe penalties)
            return Err("Too heavy to equip - would exceed maximum carry capacity".to_string());
        }

        // Remove currently equipped item in this slot
        let old_item = self.equipped.remove(&item.slot);

        // Update weight
        self.total_weight = new_weight;

        // Equip new item
        self.equipped.insert(item.slot, item);

        Ok(old_item)
    }

    /// Unequip item from a slot
    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<EquipmentItem> {
        if let Some(item) = self.equipped.remove(&slot) {
            self.total_weight -= item.weight;
            Some(item)
        } else {
            None
        }
    }

    /// Get item equipped in a slot
    pub fn get_equipped(&self, slot: EquipmentSlot) -> Option<&EquipmentItem> {
        self.equipped.get(&slot)
    }

    /// Get mutable reference to equipped item
    pub fn get_equipped_mut(&mut self, slot: EquipmentSlot) -> Option<&mut EquipmentItem> {
        self.equipped.get_mut(&slot)
    }

    /// Check if a slot has equipment
    pub fn is_slot_equipped(&self, slot: EquipmentSlot) -> bool {
        self.equipped.contains_key(&slot)
    }

    /// Get all equipped items
    pub fn get_all_equipped(&self) -> Vec<&EquipmentItem> {
        self.equipped.values().collect()
    }

    /// Get total armor rating from all equipped armor
    pub fn total_armor(&self) -> f32 {
        self.equipped
            .values()
            .filter(|item| item.equipment_type.is_armor())
            .map(|item| item.effective_armor())
            .sum()
    }

    /// Get total cold insulation from equipped items
    pub fn total_cold_insulation(&self) -> f32 {
        self.equipped
            .values()
            .filter(|item| item.equipment_type.is_armor())
            .map(|item| item.effective_cold_insulation())
            .sum()
    }

    /// Get total heat resistance from equipped items
    pub fn total_heat_resistance(&self) -> f32 {
        self.equipped
            .values()
            .filter(|item| item.equipment_type.is_armor())
            .map(|item| item.effective_heat_resistance())
            .sum()
    }

    /// Get equipped weapon (main hand first, then off hand)
    pub fn get_weapon(&self) -> Option<&EquipmentItem> {
        self.equipped
            .get(&EquipmentSlot::MainHand)
            .or_else(|| self.equipped.get(&EquipmentSlot::OffHand))
    }

    /// Get weapon damage (returns fist damage if no weapon)
    pub fn weapon_damage(&self) -> f32 {
        self.get_weapon()
            .map(|w| w.effective_damage())
            .unwrap_or(2.0) // Unarmed damage
    }

    /// Get weapon attack speed
    pub fn weapon_attack_speed(&self) -> f32 {
        self.get_weapon()
            .map(|w| w.attack_speed)
            .unwrap_or(1.0)
    }

    /// Get weapon range (reach)
    /// Returns 1.0 for unarmed (melee only), higher for spears/ranged weapons
    pub fn weapon_range(&self) -> f32 {
        self.get_weapon()
            .map(|w| w.reach)
            .unwrap_or(1.0) // Unarmed reach
    }

    /// Get equipped tool for a task
    pub fn get_tool_for_task(&self, task: &str) -> Option<&EquipmentItem> {
        let required_type = match task {
            "mining" => Some(EquipmentType::Pickaxe),
            "woodcutting" => Some(EquipmentType::Hatchet),
            "digging" => Some(EquipmentType::Shovel),
            "harvesting" => Some(EquipmentType::Sickle),
            _ => None,
        };

        if let Some(req_type) = required_type {
            // Check main hand first
            if let Some(item) = self.equipped.get(&EquipmentSlot::MainHand) {
                if item.equipment_type == req_type {
                    return Some(item);
                }
            }
        }

        // Fallback to any tool in main hand
        self.equipped
            .get(&EquipmentSlot::MainHand)
            .filter(|item| item.equipment_type.is_tool())
    }

    /// Get mining speed bonus from equipped tool
    pub fn mining_speed_bonus(&self) -> f32 {
        self.get_tool_for_task("mining")
            .map(|tool| tool.effective_mining_speed())
            .unwrap_or(1.0)
    }

    /// Get harvesting speed bonus from equipped tool
    pub fn harvesting_speed_bonus(&self) -> f32 {
        self.get_tool_for_task("harvesting")
            .map(|tool| tool.effective_harvesting_speed())
            .unwrap_or(1.0)
    }

    /// Check if any equipped tool uses primitive materials (wood or stone)
    /// Used by Traditionalist trait for bonus efficiency/happiness
    pub fn has_primitive_tool(&self) -> bool {
        self.equipped.values().any(|item| {
            item.equipment_type.is_tool() && item.material.is_primitive()
        })
    }

    /// Get the current tool's primitive status for a task
    /// Returns true if tool is made of primitive materials (wood/stone)
    pub fn is_using_primitive_tool_for_task(&self, task: &str) -> bool {
        self.get_tool_for_task(task)
            .map(|tool| tool.material.is_primitive())
            .unwrap_or(false)
    }

    /// Get mining speed bonus with Traditionalist trait bonus
    /// Traditionalist trait grants +30% efficiency with primitive tools
    pub fn mining_speed_with_traits(&self, traits: &crate::core::traits::TraitSet) -> f32 {
        let base_speed = self.mining_speed_bonus();
        if traits.has(crate::core::traits::Trait::Traditionalist) {
            if self.is_using_primitive_tool_for_task("mining") {
                return base_speed * 1.3; // 30% bonus with primitive tools
            }
        }
        base_speed
    }

    /// Get harvesting speed bonus with Traditionalist trait bonus
    /// Traditionalist trait grants +30% efficiency with primitive tools
    pub fn harvesting_speed_with_traits(&self, traits: &crate::core::traits::TraitSet) -> f32 {
        let base_speed = self.harvesting_speed_bonus();
        if traits.has(crate::core::traits::Trait::Traditionalist) {
            if self.is_using_primitive_tool_for_task("harvesting") || self.is_using_primitive_tool_for_task("woodcutting") {
                return base_speed * 1.3; // 30% bonus with primitive tools
            }
        }
        base_speed
    }

    /// Tick all equipped items (apply wear)
    pub fn tick_all_equipment(&mut self) {
        for item in self.equipped.values_mut() {
            if !item.is_broken() {
                item.tick_wear();
            }
        }
    }

    /// Apply combat wear to weapon
    pub fn apply_combat_wear(&mut self, wear_amount: f32) {
        if let Some(weapon) = self.equipped.get_mut(&EquipmentSlot::MainHand) {
            weapon.apply_wear(wear_amount);
        }
    }

    /// Apply tool wear for a specific task
    pub fn apply_tool_wear(&mut self, task: &str, wear_amount: f32) {
        if let Some(slot) = self.get_tool_slot_for_task(task) {
            if let Some(tool) = self.equipped.get_mut(&slot) {
                tool.apply_wear(wear_amount);
            }
        }
    }

    /// Get armor coverage for a specific body part
    pub fn get_armor_for_part(&self, part: BodyPartType) -> f32 {
        let mut total_armor = 0.0;
        for item in self.equipped.values() {
            if item.slot.covered_parts().contains(&part) {
                total_armor += item.effective_armor();
            }
        }
        total_armor.min(0.95) // Max 95% damage reduction
    }

    /// Check if carrying too much weight
    pub fn is_encumbered(&self) -> bool {
        self.total_weight > self.max_carry_weight
    }

    /// Get encumbrance penalty (0.0 = no penalty, 1.0 = fully encumbered)
    pub fn encumbrance_penalty(&self) -> f32 {
        if self.total_weight <= self.max_carry_weight {
            0.0
        } else {
            ((self.total_weight - self.max_carry_weight) / self.max_carry_weight).min(1.0)
        }
    }

    /// Get movement speed multiplier based on weight
    pub fn movement_speed_multiplier(&self) -> f32 {
        1.0 - (self.encumbrance_penalty() * 0.5)
    }

    /// Get total weight of all equipped items
    pub fn get_total_weight(&self) -> f32 {
        self.total_weight
    }

    /// Get total value of all equipped items (for comparison purposes)
    /// Value is based on quality, material tier, and equipment type
    pub fn total_value(&self) -> f32 {
        self.equipped.values().map(|item| {
            let base_value = match item.equipment_type {
                // Weapons
                EquipmentType::Sword | EquipmentType::Axe => 20.0,
                EquipmentType::Bow | EquipmentType::Crossbow => 25.0,
                EquipmentType::Spear | EquipmentType::Hammer => 18.0,
                EquipmentType::Dagger | EquipmentType::Mace => 15.0,
                // Armor
                EquipmentType::HeavyArmor => 30.0,
                EquipmentType::MediumArmor => 20.0,
                EquipmentType::LightArmor => 12.0,
                EquipmentType::Clothing => 5.0,
                EquipmentType::Shield => 15.0,
                // Tools
                EquipmentType::Pickaxe | EquipmentType::Hatchet | EquipmentType::Shovel | EquipmentType::Sickle => 15.0,
                EquipmentType::FishingRod => 10.0,
                // Accessories
                EquipmentType::Ring | EquipmentType::Necklace | EquipmentType::Amulet | EquipmentType::Bracelet => 30.0,
                // Utility
                EquipmentType::Torch => 2.0,
                EquipmentType::Lantern => 8.0,
            };
            let quality_mult = item.quality.value_multiplier();
            let material_mult = item.material.value_multiplier();
            base_value * quality_mult * material_mult
        }).sum()
    }

    /// Get list of broken equipment
    pub fn get_broken_equipment(&self) -> Vec<EquipmentSlot> {
        self.equipped
            .iter()
            .filter(|(_, item)| item.is_broken())
            .map(|(slot, _)| *slot)
            .collect()
    }

    /// Unequip all broken items
    pub fn unequip_broken(&mut self) -> Vec<EquipmentItem> {
        let broken_slots: Vec<EquipmentSlot> = self.get_broken_equipment();
        broken_slots
            .into_iter()
            .filter_map(|slot| self.unequip(slot))
            .collect()
    }

    /// Get equipment summary string
    pub fn equipment_summary(&self) -> String {
        if self.equipped.is_empty() {
            return "No equipment".to_string();
        }

        let mut summary = Vec::new();
        for (slot, item) in &self.equipped {
            summary.push(format!(
                "{:?}: {} ({})",
                slot,
                item.name,
                item.condition_description()
            ));
        }

        summary.join(", ")
    }

    // Helper methods

    fn slot_accepts_type(slot: &EquipmentSlot, equipment_type: &EquipmentType) -> bool {
        match slot {
            EquipmentSlot::MainHand | EquipmentSlot::OffHand => {
                equipment_type.is_weapon()
                    || equipment_type.is_tool()
                    || matches!(equipment_type, EquipmentType::Shield | EquipmentType::Torch | EquipmentType::Lantern)
            }
            EquipmentSlot::Head | EquipmentSlot::Torso | EquipmentSlot::Back |
            EquipmentSlot::Legs | EquipmentSlot::Hands | EquipmentSlot::Feet => {
                equipment_type.is_armor()
            }
            EquipmentSlot::Arms => {
                // Arms can accept armor or bracelets
                equipment_type.is_armor() || matches!(equipment_type, EquipmentType::Bracelet)
            }
            EquipmentSlot::Neck => {
                // Neck accepts necklaces and amulets
                matches!(equipment_type, EquipmentType::Necklace | EquipmentType::Amulet)
            }
            EquipmentSlot::Finger => {
                // Finger accepts rings
                matches!(equipment_type, EquipmentType::Ring)
            }
        }
    }

    fn get_slot_weight(&self, slot: &EquipmentSlot) -> f32 {
        self.equipped
            .get(slot)
            .map(|item| item.weight)
            .unwrap_or(0.0)
    }

    fn get_tool_slot_for_task(&self, task: &str) -> Option<EquipmentSlot> {
        let required_type = match task {
            "mining" => Some(EquipmentType::Pickaxe),
            "woodcutting" => Some(EquipmentType::Hatchet),
            "digging" => Some(EquipmentType::Shovel),
            "harvesting" => Some(EquipmentType::Sickle),
            _ => None,
        };

        if let Some(req_type) = required_type {
            for (slot, item) in &self.equipped {
                if item.equipment_type == req_type {
                    return Some(*slot);
                }
            }
        }

        None
    }
}

impl Default for EquipmentManager {
    fn default() -> Self {
        Self::new(50.0) // Default 50kg carry capacity
    }
}

/// Predefined clothing templates
/// A garment an agent can actually make, and what it takes to make one.
///
/// This is the single table behind both making and wearing: an agent checks it
/// for something it has the material for, and reads it again to rebuild the
/// garment when it puts it on.
///
/// Material decides most of what a garment is worth. Fur and wool are what you
/// want against the cold and are only had from animals; flax and cotton grow
/// on the ground and make something that will do; bark is what you fall back
/// on. `environment::clothing_recipes` describes a richer set that wants a
/// workbench, tanned leather and spun thread - none of which exists in a
/// running simulation - so these are deliberately made from what an agent can
/// pick up.
#[derive(Debug, Clone, Copy)]
pub struct GarmentRecipe {
    /// Item id the finished garment is carried and worn under
    pub id: &'static str,
    pub name: &'static str,
    pub slot: EquipmentSlot,
    /// Inventory item it is made from, and how much of it
    pub material_item: &'static str,
    pub material_amount: u32,
    pub material: ClothingMaterial,
    pub base_cold_insulation: f32,
    pub base_heat_resistance: f32,
    pub base_armor: f32,
}

impl GarmentRecipe {
    /// Roughly how much warmth this garment is worth at ordinary quality,
    /// which is what an agent compares when deciding what to make
    pub fn warmth(&self) -> f32 {
        self.base_cold_insulation * self.material.cold_insulation()
    }
}

/// Every garment an agent knows how to make
pub const GARMENT_RECIPES: &[GarmentRecipe] = &[
    // From animals: the warm things, and the ones an agent has to hunt for
    GarmentRecipe {
        id: "fur_coat",
        name: "Fur Coat",
        slot: EquipmentSlot::Torso,
        material_item: "hides",
        material_amount: 6,
        material: ClothingMaterial::Fur,
        base_cold_insulation: 0.8,
        base_heat_resistance: 0.1,
        base_armor: 0.2,
    },
    GarmentRecipe {
        id: "fur_hat",
        name: "Fur Hat",
        slot: EquipmentSlot::Head,
        material_item: "hides",
        material_amount: 3,
        material: ClothingMaterial::Fur,
        base_cold_insulation: 0.5,
        base_heat_resistance: 0.1,
        base_armor: 0.1,
    },
    GarmentRecipe {
        id: "hide_armor",
        name: "Hide Armor",
        slot: EquipmentSlot::Torso,
        material_item: "hides",
        material_amount: 8,
        material: ClothingMaterial::Hide,
        base_cold_insulation: 0.5,
        base_heat_resistance: 0.2,
        base_armor: 0.6,
    },
    GarmentRecipe {
        id: "wool_cloak",
        name: "Wool Cloak",
        slot: EquipmentSlot::Back,
        material_item: "wool",
        material_amount: 5,
        material: ClothingMaterial::Wool,
        base_cold_insulation: 0.6,
        base_heat_resistance: 0.2,
        base_armor: 0.1,
    },
    GarmentRecipe {
        id: "leather_tunic",
        name: "Leather Tunic",
        slot: EquipmentSlot::Torso,
        material_item: "leather",
        material_amount: 8,
        material: ClothingMaterial::Leather,
        base_cold_insulation: 0.4,
        base_heat_resistance: 0.3,
        base_armor: 0.3,
    },
    GarmentRecipe {
        id: "leather_pants",
        name: "Leather Pants",
        slot: EquipmentSlot::Legs,
        material_item: "leather",
        material_amount: 6,
        material: ClothingMaterial::Leather,
        base_cold_insulation: 0.3,
        base_heat_resistance: 0.3,
        base_armor: 0.2,
    },
    // From the ground: what an agent can make without hunting anything
    GarmentRecipe {
        id: "linen_cloak",
        name: "Linen Cloak",
        slot: EquipmentSlot::Back,
        material_item: "flax",
        material_amount: 6,
        material: ClothingMaterial::Linen,
        base_cold_insulation: 0.45,
        base_heat_resistance: 0.3,
        base_armor: 0.05,
    },
    GarmentRecipe {
        id: "cotton_cloak",
        name: "Cotton Cloak",
        slot: EquipmentSlot::Back,
        material_item: "cotton",
        material_amount: 6,
        material: ClothingMaterial::Cotton,
        base_cold_insulation: 0.45,
        base_heat_resistance: 0.35,
        base_armor: 0.05,
    },
    GarmentRecipe {
        id: "linen_hood",
        name: "Linen Hood",
        slot: EquipmentSlot::Head,
        material_item: "flax",
        material_amount: 3,
        material: ClothingMaterial::Linen,
        base_cold_insulation: 0.3,
        base_heat_resistance: 0.3,
        base_armor: 0.05,
    },
    GarmentRecipe {
        id: "cotton_hood",
        name: "Cotton Hood",
        slot: EquipmentSlot::Head,
        material_item: "cotton",
        material_amount: 3,
        material: ClothingMaterial::Cotton,
        base_cold_insulation: 0.3,
        base_heat_resistance: 0.35,
        base_armor: 0.05,
    },
    GarmentRecipe {
        id: "linen_shirt",
        name: "Linen Shirt",
        slot: EquipmentSlot::Torso,
        material_item: "flax",
        material_amount: 5,
        material: ClothingMaterial::Linen,
        base_cold_insulation: 0.2,
        base_heat_resistance: 0.7,
        base_armor: 0.05,
    },
    // The last resort, and the only ones made of something there is always
    // plenty of. Bark and bast are poor insulators next to fur, but there are
    // trees everywhere and nobody has to hunt for them.
    GarmentRecipe {
        id: "bark_cloak",
        name: "Bark Cloak",
        slot: EquipmentSlot::Back,
        material_item: "wood",
        material_amount: 6,
        material: ClothingMaterial::Bark,
        base_cold_insulation: 0.35,
        base_heat_resistance: 0.3,
        base_armor: 0.05,
    },
    GarmentRecipe {
        id: "bark_boots",
        name: "Bark Boots",
        slot: EquipmentSlot::Feet,
        material_item: "wood",
        material_amount: 4,
        material: ClothingMaterial::Bark,
        base_cold_insulation: 0.2,
        base_heat_resistance: 0.4,
        base_armor: 0.1,
    },
];

/// The recipe for a garment, by the id it is carried under
pub fn garment_recipe(id: &str) -> Option<&'static GarmentRecipe> {
    GARMENT_RECIPES.iter().find(|recipe| recipe.id == id)
}

pub struct ClothingTemplate;

impl ClothingTemplate {
    /// Build a garment from its id, as made by an agent of the given skill.
    ///
    /// Quality carries through to warmth and to how long it lasts, so a first
    /// attempt is a poor thing that falls apart and a practised hand makes
    /// something worth wearing.
    pub fn from_id(id: &str, quality: Quality) -> Option<Equipment> {
        garment_recipe(id).map(|recipe| {
            Equipment::new(
                recipe.name.to_string(),
                recipe.slot,
                recipe.material,
                quality,
                recipe.base_cold_insulation,
                recipe.base_heat_resistance,
                recipe.base_armor,
            )
        })
    }

    /// Leather tunic - basic torso protection
    pub fn leather_tunic(quality: Quality) -> Equipment {
        Equipment::new(
            "Leather Tunic".to_string(),
            EquipmentSlot::Torso,
            ClothingMaterial::Leather,
            quality,
            0.4,  // Moderate cold protection
            0.3,  // Light heat resistance
            0.3,  // Light armor
        )
    }

    /// Fur coat - excellent cold weather gear
    pub fn fur_coat(quality: Quality) -> Equipment {
        Equipment::new(
            "Fur Coat".to_string(),
            EquipmentSlot::Torso,
            ClothingMaterial::Fur,
            quality,
            0.8,  // Excellent cold protection
            0.1,  // Poor in heat
            0.2,  // Light armor
        )
    }

    /// Wool cloak - good all-around protection
    pub fn wool_cloak(quality: Quality) -> Equipment {
        Equipment::new(
            "Wool Cloak".to_string(),
            EquipmentSlot::Back,
            ClothingMaterial::Wool,
            quality,
            0.6,  // Good cold protection
            0.2,  // Poor in heat
            0.1,  // Minimal armor
        )
    }

    /// Linen shirt - hot weather clothing
    pub fn linen_shirt(quality: Quality) -> Equipment {
        Equipment::new(
            "Linen Shirt".to_string(),
            EquipmentSlot::Torso,
            ClothingMaterial::Linen,
            quality,
            0.2,  // Minimal cold protection
            0.7,  // Excellent heat resistance
            0.05, // Very light armor
        )
    }

    /// Leather pants
    pub fn leather_pants(quality: Quality) -> Equipment {
        Equipment::new(
            "Leather Pants".to_string(),
            EquipmentSlot::Legs,
            ClothingMaterial::Leather,
            quality,
            0.3,  // Light cold protection
            0.3,  // Light heat resistance
            0.2,  // Light armor
        )
    }

    /// Fur hat
    pub fn fur_hat(quality: Quality) -> Equipment {
        Equipment::new(
            "Fur Hat".to_string(),
            EquipmentSlot::Head,
            ClothingMaterial::Fur,
            quality,
            0.5,  // Good cold protection
            0.1,  // Poor in heat
            0.1,  // Minimal armor
        )
    }

    /// Hide armor - heavy protection
    pub fn hide_armor(quality: Quality) -> Equipment {
        Equipment::new(
            "Hide Armor".to_string(),
            EquipmentSlot::Torso,
            ClothingMaterial::Hide,
            quality,
            0.5,  // Decent cold protection
            0.2,  // Poor in heat
            0.6,  // Good armor
        )
    }

    /// Leather gloves
    pub fn leather_gloves(quality: Quality) -> Equipment {
        Equipment::new(
            "Leather Gloves".to_string(),
            EquipmentSlot::Arms,
            ClothingMaterial::Leather,
            quality,
            0.3,  // Light cold protection
            0.3,  // Light heat resistance
            0.15, // Light armor
        )
    }

    /// Bark boots (primitive)
    pub fn bark_boots(quality: Quality) -> Equipment {
        Equipment::new(
            "Bark Boots".to_string(),
            EquipmentSlot::Feet,
            ClothingMaterial::Bark,
            quality,
            0.2,  // Minimal cold protection
            0.4,  // Decent heat resistance
            0.1,  // Minimal armor
        )
    }
}

/// Weapon templates using the new EquipmentItem system
pub struct WeaponTemplate;

impl WeaponTemplate {
    // Swords
    pub fn iron_sword(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Sword".to_string(),
            EquipmentType::Sword,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_sword(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Sword".to_string(),
            EquipmentType::Sword,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    pub fn bronze_sword(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Bronze Sword".to_string(),
            EquipmentType::Sword,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Bronze),
            quality,
        )
    }

    // Axes
    pub fn iron_axe(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Axe".to_string(),
            EquipmentType::Axe,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_axe(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Axe".to_string(),
            EquipmentType::Axe,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    // Spears
    pub fn iron_spear(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Spear".to_string(),
            EquipmentType::Spear,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn wooden_spear(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Wooden Spear".to_string(),
            EquipmentType::Spear,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Wood(WoodMaterial::Ash),
            quality,
        )
    }

    // Daggers
    pub fn iron_dagger(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Dagger".to_string(),
            EquipmentType::Dagger,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn obsidian_dagger(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Obsidian Dagger".to_string(),
            EquipmentType::Dagger,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Stone(StoneMaterial::Obsidian),
            quality,
        )
    }

    // Maces
    pub fn iron_mace(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Mace".to_string(),
            EquipmentType::Mace,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn stone_mace(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Stone Mace".to_string(),
            EquipmentType::Mace,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Stone(StoneMaterial::Granite),
            quality,
        )
    }

    // Bows
    pub fn yew_bow(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Yew Bow".to_string(),
            EquipmentType::Bow,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Wood(WoodMaterial::Yew),
            quality,
        )
    }

    pub fn ash_bow(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Ash Bow".to_string(),
            EquipmentType::Bow,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Wood(WoodMaterial::Ash),
            quality,
        )
    }
}

/// Tool templates
pub struct ToolTemplate;

impl ToolTemplate {
    // Pickaxes
    pub fn stone_pickaxe(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Stone Pickaxe".to_string(),
            EquipmentType::Pickaxe,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Stone(StoneMaterial::Flint),
            quality,
        )
    }

    pub fn iron_pickaxe(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Pickaxe".to_string(),
            EquipmentType::Pickaxe,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_pickaxe(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Pickaxe".to_string(),
            EquipmentType::Pickaxe,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    // Hatchets
    pub fn stone_hatchet(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Stone Hatchet".to_string(),
            EquipmentType::Hatchet,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Stone(StoneMaterial::Flint),
            quality,
        )
    }

    pub fn iron_hatchet(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Hatchet".to_string(),
            EquipmentType::Hatchet,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_hatchet(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Hatchet".to_string(),
            EquipmentType::Hatchet,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    // Shovels
    pub fn wooden_shovel(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Wooden Shovel".to_string(),
            EquipmentType::Shovel,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Wood(WoodMaterial::Oak),
            quality,
        )
    }

    pub fn iron_shovel(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Shovel".to_string(),
            EquipmentType::Shovel,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    // Hammers
    pub fn stone_hammer(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Stone Hammer".to_string(),
            EquipmentType::Hammer,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Stone(StoneMaterial::Granite),
            quality,
        )
    }

    pub fn iron_hammer(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Hammer".to_string(),
            EquipmentType::Hammer,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    // Sickles
    pub fn iron_sickle(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Sickle".to_string(),
            EquipmentType::Sickle,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn bronze_sickle(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Bronze Sickle".to_string(),
            EquipmentType::Sickle,
            EquipmentSlot::MainHand,
            EquipmentMaterial::Metal(MetalMaterial::Bronze),
            quality,
        )
    }
}

/// Armor templates (metal armor)
pub struct ArmorTemplate;

impl ArmorTemplate {
    // Light Armor
    pub fn leather_light_armor(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Leather Armor".to_string(),
            EquipmentType::LightArmor,
            EquipmentSlot::Torso,
            EquipmentMaterial::Cloth(ClothingMaterial::Leather),
            quality,
        )
    }

    // Medium Armor
    pub fn bronze_medium_armor(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Bronze Armor".to_string(),
            EquipmentType::MediumArmor,
            EquipmentSlot::Torso,
            EquipmentMaterial::Metal(MetalMaterial::Bronze),
            quality,
        )
    }

    pub fn iron_medium_armor(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Armor".to_string(),
            EquipmentType::MediumArmor,
            EquipmentSlot::Torso,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    // Heavy Armor
    pub fn iron_heavy_armor(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Plate Armor".to_string(),
            EquipmentType::HeavyArmor,
            EquipmentSlot::Torso,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_heavy_armor(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Plate Armor".to_string(),
            EquipmentType::HeavyArmor,
            EquipmentSlot::Torso,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    // Shields
    pub fn wooden_shield(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Wooden Shield".to_string(),
            EquipmentType::Shield,
            EquipmentSlot::OffHand,
            EquipmentMaterial::Wood(WoodMaterial::Oak),
            quality,
        )
    }

    pub fn iron_shield(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Shield".to_string(),
            EquipmentType::Shield,
            EquipmentSlot::OffHand,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_shield(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Shield".to_string(),
            EquipmentType::Shield,
            EquipmentSlot::OffHand,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    // Helmets
    pub fn leather_helmet(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Leather Helmet".to_string(),
            EquipmentType::LightArmor,
            EquipmentSlot::Head,
            EquipmentMaterial::Cloth(ClothingMaterial::Leather),
            quality,
        )
    }

    pub fn iron_helmet(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Helmet".to_string(),
            EquipmentType::MediumArmor,
            EquipmentSlot::Head,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    pub fn steel_helmet(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Steel Helmet".to_string(),
            EquipmentType::HeavyArmor,
            EquipmentSlot::Head,
            EquipmentMaterial::Metal(MetalMaterial::Steel),
            quality,
        )
    }

    // Gauntlets
    pub fn iron_gauntlets(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Gauntlets".to_string(),
            EquipmentType::MediumArmor,
            EquipmentSlot::Hands,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }

    // Boots
    pub fn leather_boots(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Leather Boots".to_string(),
            EquipmentType::LightArmor,
            EquipmentSlot::Feet,
            EquipmentMaterial::Cloth(ClothingMaterial::Leather),
            quality,
        )
    }

    pub fn iron_boots(quality: Quality) -> EquipmentItem {
        EquipmentItem::new(
            "Iron Boots".to_string(),
            EquipmentType::MediumArmor,
            EquipmentSlot::Feet,
            EquipmentMaterial::Metal(MetalMaterial::Iron),
            quality,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equipment_creation() {
        let tunic = ClothingTemplate::leather_tunic(Quality::Basic);
        assert_eq!(tunic.name, "Leather Tunic");
        assert_eq!(tunic.slot, EquipmentSlot::Torso);
        assert_eq!(tunic.material, ClothingMaterial::Leather);
        assert!(tunic.durability > 0.0);
        assert_eq!(tunic.durability, tunic.max_durability);
    }

    #[test]
    fn test_material_properties() {
        // Fur should be excellent for cold
        assert!(ClothingMaterial::Fur.cold_insulation() > ClothingMaterial::Linen.cold_insulation());

        // Linen should be excellent for heat
        assert!(ClothingMaterial::Linen.heat_resistance() > ClothingMaterial::Fur.heat_resistance());

        // Hide should be best for armor
        assert!(ClothingMaterial::Hide.armor_multiplier() > ClothingMaterial::Linen.armor_multiplier());
    }

    #[test]
    fn test_quality_affects_durability() {
        let basic = ClothingTemplate::leather_tunic(Quality::Basic);
        let expert = ClothingTemplate::leather_tunic(Quality::Expert);

        assert!(expert.max_durability > basic.max_durability);
    }

    #[test]
    fn test_wear_degradation() {
        let mut tunic = ClothingTemplate::leather_tunic(Quality::Basic);
        let initial_effectiveness = tunic.cold_insulation();

        // Apply 50% wear
        tunic.apply_wear(tunic.max_durability * 0.5);

        let degraded_effectiveness = tunic.cold_insulation();
        assert!(degraded_effectiveness < initial_effectiveness);
        assert!((tunic.durability_percentage() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_tick_wear() {
        let mut tunic = ClothingTemplate::leather_tunic(Quality::Basic);
        let initial_durability = tunic.durability;

        tunic.tick_wear();

        assert!(tunic.durability < initial_durability);
        assert_eq!(tunic.durability, initial_durability - tunic.wear_rate);
    }

    #[test]
    fn test_broken_equipment() {
        let mut tunic = ClothingTemplate::leather_tunic(Quality::Basic);
        assert!(!tunic.is_broken());

        tunic.durability = 0.0;
        assert!(tunic.is_broken());
    }

    #[test]
    fn test_repair() {
        let mut tunic = ClothingTemplate::leather_tunic(Quality::Basic);
        let max = tunic.max_durability;

        tunic.apply_wear(50.0);
        assert!(tunic.durability < max);

        tunic.repair(30.0);
        assert_eq!(tunic.durability, max - 20.0);

        // Can't repair beyond max
        tunic.repair(100.0);
        assert_eq!(tunic.durability, max);
    }

    #[test]
    fn test_equipment_slots() {
        assert_eq!(EquipmentSlot::Head.covered_parts(), vec![BodyPartType::Head]);
        assert_eq!(
            EquipmentSlot::Arms.covered_parts(),
            vec![BodyPartType::LeftArm, BodyPartType::RightArm]
        );
        assert_eq!(
            EquipmentSlot::Legs.covered_parts(),
            vec![BodyPartType::LeftLeg, BodyPartType::RightLeg]
        );
    }

    #[test]
    fn test_fur_vs_linen_temperature() {
        let fur_coat = ClothingTemplate::fur_coat(Quality::Basic);
        let linen_shirt = ClothingTemplate::linen_shirt(Quality::Basic);

        // Fur should be much better for cold
        assert!(fur_coat.cold_insulation() > linen_shirt.cold_insulation());

        // Linen should be much better for heat
        assert!(linen_shirt.heat_resistance() > fur_coat.heat_resistance());
    }

    #[test]
    fn test_hide_armor_protection() {
        let hide_armor = ClothingTemplate::hide_armor(Quality::Basic);
        let linen_shirt = ClothingTemplate::linen_shirt(Quality::Basic);

        assert!(hide_armor.armor_protection() > linen_shirt.armor_protection());
    }
}
