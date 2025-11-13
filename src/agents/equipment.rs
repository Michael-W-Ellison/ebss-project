// src/agents/equipment.rs
//! Equipment and clothing system with durability and temperature protection.

use crate::agents::body::BodyPartType;
use crate::agents::skills::Quality;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of equipment slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Torso,
    Back,
    Arms,   // Covers both arms
    Legs,   // Covers both legs
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
        }
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
            wear_rate: 0.01, // Default wear rate
        }
    }

    /// Get effective cold insulation (material × base × quality × durability)
    pub fn cold_insulation(&self) -> f32 {
        let material_mult = self.material.cold_insulation();
        let quality_mult = self.quality.modifier();
        let durability_mult = self.durability_multiplier();

        self.base_cold_insulation * material_mult * quality_mult * durability_mult
    }

    /// Get effective heat resistance (material × base × quality × durability)
    pub fn heat_resistance(&self) -> f32 {
        let material_mult = self.material.heat_resistance();
        let quality_mult = self.quality.modifier();
        let durability_mult = self.durability_multiplier();

        self.base_heat_resistance * material_mult * quality_mult * durability_mult
    }

    /// Get effective armor protection (material × base × quality × durability)
    pub fn armor_protection(&self) -> f32 {
        let material_mult = self.material.armor_multiplier();
        let quality_mult = self.quality.modifier();
        let durability_mult = self.durability_multiplier();

        (self.base_armor * material_mult * quality_mult * durability_mult).min(0.95)
    }

    /// Get durability as percentage (0.0 to 1.0)
    pub fn durability_percentage(&self) -> f32 {
        if self.max_durability > 0.0 {
            self.durability / self.max_durability
        } else {
            0.0
        }
    }

    /// Get durability multiplier for effectiveness
    fn durability_multiplier(&self) -> f32 {
        let pct = self.durability_percentage();
        // Linear degradation - at 50% durability, effectiveness is 50%
        pct
    }

    /// Apply wear to the equipment
    pub fn apply_wear(&mut self, amount: f32) {
        self.durability = (self.durability - amount).max(0.0);
    }

    /// Tick wear (called each game tick while equipped)
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
}

/// Predefined clothing templates
pub struct ClothingTemplate;

impl ClothingTemplate {
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
            EquipmentSlot::Legs,
            ClothingMaterial::Bark,
            quality,
            0.2,  // Minimal cold protection
            0.4,  // Decent heat resistance
            0.1,  // Minimal armor
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
