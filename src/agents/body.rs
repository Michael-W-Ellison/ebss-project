// src/agents/body.rs
//! Body part system for agents with anatomical structure.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Body part types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyPartType {
    Head,
    LeftArm,
    RightArm,
    Torso,
    Back,
    LeftLeg,
    RightLeg,
}

impl BodyPartType {
    /// Get all body part types
    pub fn all() -> [BodyPartType; 7] {
        [
            BodyPartType::Head,
            BodyPartType::LeftArm,
            BodyPartType::RightArm,
            BodyPartType::Torso,
            BodyPartType::Back,
            BodyPartType::LeftLeg,
            BodyPartType::RightLeg,
        ]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            BodyPartType::Head => "Head",
            BodyPartType::LeftArm => "Left Arm",
            BodyPartType::RightArm => "Right Arm",
            BodyPartType::Torso => "Torso",
            BodyPartType::Back => "Back",
            BodyPartType::LeftLeg => "Left Leg",
            BodyPartType::RightLeg => "Right Leg",
        }
    }

    /// Check if part is critical (death if destroyed)
    pub fn is_critical(&self) -> bool {
        matches!(self, BodyPartType::Head | BodyPartType::Torso)
    }

    /// Get base health for this body part
    pub fn base_health(&self) -> f32 {
        match self {
            BodyPartType::Head => 50.0,
            BodyPartType::Torso => 100.0,
            BodyPartType::Back => 80.0,
            BodyPartType::LeftArm | BodyPartType::RightArm => 60.0,
            BodyPartType::LeftLeg | BodyPartType::RightLeg => 70.0,
        }
    }

    /// Check if this is an arm
    pub fn is_arm(&self) -> bool {
        matches!(self, BodyPartType::LeftArm | BodyPartType::RightArm)
    }

    /// Check if this is a leg
    pub fn is_leg(&self) -> bool {
        matches!(self, BodyPartType::LeftLeg | BodyPartType::RightLeg)
    }
}

/// Status of a body part
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyPartStatus {
    /// Fully functional
    Healthy,
    /// Injured but functional
    Injured,
    /// Severely injured, limited function
    Crippled,
    /// Non-functional
    Disabled,
    /// Missing/destroyed
    Missing,
}

/// A single body part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPart {
    pub part_type: BodyPartType,
    pub health: f32,
    pub max_health: f32,
    pub status: BodyPartStatus,

    /// Equipment worn on this part (helmet, armor, etc.)
    pub equipped_item: Option<String>,

    /// Active conditions (bleeding, burned, frostbitten, etc.)
    pub conditions: Vec<Condition>,

    /// Protection value (from armor)
    pub protection: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub severity: f32, // 0.0 to 1.0
    pub duration: u32, // Ticks remaining
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    Bleeding,
    Burned,
    Frostbitten,
    Poisoned,
    Infected,
    Bruised,
    Fractured,
}

impl BodyPart {
    pub fn new(part_type: BodyPartType) -> Self {
        let max_health = part_type.base_health();
        Self {
            part_type,
            health: max_health,
            max_health,
            status: BodyPartStatus::Healthy,
            equipped_item: None,
            conditions: Vec::new(),
            protection: 0.0,
        }
    }

    /// Apply damage to this body part
    pub fn take_damage(&mut self, damage: f32) -> f32 {
        // Protection reduces damage
        let actual_damage = (damage * (1.0 - self.protection)).max(0.0);
        self.health = (self.health - actual_damage).max(0.0);

        self.update_status();
        actual_damage
    }

    /// Heal this body part
    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(self.max_health);
        self.update_status();
    }

    /// Update status based on health percentage
    fn update_status(&mut self) {
        if self.health == 0.0 {
            self.status = BodyPartStatus::Missing;
        } else {
            let health_pct = self.health / self.max_health;
            self.status = if health_pct >= 0.75 {
                BodyPartStatus::Healthy
            } else if health_pct >= 0.5 {
                BodyPartStatus::Injured
            } else if health_pct >= 0.25 {
                BodyPartStatus::Crippled
            } else {
                BodyPartStatus::Disabled
            };
        }
    }

    /// Get health percentage
    pub fn health_percentage(&self) -> f32 {
        if self.max_health > 0.0 {
            self.health / self.max_health
        } else {
            0.0
        }
    }

    /// Check if part is functional
    pub fn is_functional(&self) -> bool {
        !matches!(
            self.status,
            BodyPartStatus::Disabled | BodyPartStatus::Missing
        )
    }

    /// Equip an item on this body part
    pub fn equip(&mut self, item_id: String, protection: f32) {
        self.equipped_item = Some(item_id);
        self.protection = protection.clamp(0.0, 0.95); // Max 95% protection
    }

    /// Unequip item from this body part
    pub fn unequip(&mut self) -> Option<String> {
        self.protection = 0.0;
        self.equipped_item.take()
    }

    /// Add a condition to this body part
    pub fn add_condition(&mut self, condition: Condition) {
        self.conditions.push(condition);
    }

    /// Process conditions (tick effects)
    pub fn tick(&mut self) {
        // Collect damage to apply
        let mut total_damage = 0.0;

        // Age conditions and calculate damage
        for condition in &mut self.conditions {
            if condition.duration > 0 {
                condition.duration -= 1;
            }

            // Calculate condition effects
            match condition.condition_type {
                ConditionType::Bleeding => {
                    total_damage += condition.severity * 0.5;
                }
                ConditionType::Poisoned => {
                    total_damage += condition.severity * 0.3;
                }
                ConditionType::Burned => {
                    total_damage += condition.severity * 0.2;
                }
                _ => {}
            }
        }

        // Apply accumulated damage
        if total_damage > 0.0 {
            self.take_damage(total_damage);
        }

        // Remove expired conditions
        self.conditions.retain(|c| c.duration > 0);
    }
}

/// Complete body system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    pub parts: HashMap<BodyPartType, BodyPart>,
}

impl Body {
    pub fn new() -> Self {
        let mut parts = HashMap::new();

        for part_type in BodyPartType::all() {
            parts.insert(part_type, BodyPart::new(part_type));
        }

        Self { parts }
    }

    /// Get a body part
    pub fn get_part(&self, part_type: BodyPartType) -> Option<&BodyPart> {
        self.parts.get(&part_type)
    }

    /// Get a mutable body part
    pub fn get_part_mut(&mut self, part_type: BodyPartType) -> Option<&mut BodyPart> {
        self.parts.get_mut(&part_type)
    }

    /// Apply damage to a specific body part
    pub fn damage_part(&mut self, part_type: BodyPartType, damage: f32) -> f32 {
        if let Some(part) = self.parts.get_mut(&part_type) {
            part.take_damage(damage)
        } else {
            0.0
        }
    }

    /// Heal a specific body part
    pub fn heal_part(&mut self, part_type: BodyPartType, amount: f32) {
        if let Some(part) = self.parts.get_mut(&part_type) {
            part.heal(amount);
        }
    }

    /// Get overall body health (0.0 to 1.0)
    pub fn overall_health(&self) -> f32 {
        let total_health: f32 = self.parts.values().map(|p| p.health).sum();
        let total_max_health: f32 = self.parts.values().map(|p| p.max_health).sum();

        if total_max_health > 0.0 {
            total_health / total_max_health
        } else {
            0.0
        }
    }

    /// Check if any critical part is destroyed
    pub fn is_alive(&self) -> bool {
        self.parts
            .iter()
            .filter(|(part_type, _)| part_type.is_critical())
            .all(|(_, part)| part.health > 0.0)
    }

    /// Check if agent can walk (both legs functional)
    pub fn can_walk(&self) -> bool {
        self.parts
            .get(&BodyPartType::LeftLeg)
            .map(|p| p.is_functional())
            .unwrap_or(false)
            || self
                .parts
                .get(&BodyPartType::RightLeg)
                .map(|p| p.is_functional())
                .unwrap_or(false)
    }

    /// Check if agent can use tools (at least one arm functional)
    pub fn can_use_tools(&self) -> bool {
        self.parts
            .get(&BodyPartType::LeftArm)
            .map(|p| p.is_functional())
            .unwrap_or(false)
            || self
                .parts
                .get(&BodyPartType::RightArm)
                .map(|p| p.is_functional())
                .unwrap_or(false)
    }

    /// Get number of functional arms
    pub fn functional_arms(&self) -> u8 {
        let mut count = 0;
        if self
            .parts
            .get(&BodyPartType::LeftArm)
            .map(|p| p.is_functional())
            .unwrap_or(false)
        {
            count += 1;
        }
        if self
            .parts
            .get(&BodyPartType::RightArm)
            .map(|p| p.is_functional())
            .unwrap_or(false)
        {
            count += 1;
        }
        count
    }

    /// Get number of functional legs
    pub fn functional_legs(&self) -> u8 {
        let mut count = 0;
        if self
            .parts
            .get(&BodyPartType::LeftLeg)
            .map(|p| p.is_functional())
            .unwrap_or(false)
        {
            count += 1;
        }
        if self
            .parts
            .get(&BodyPartType::RightLeg)
            .map(|p| p.is_functional())
            .unwrap_or(false)
        {
            count += 1;
        }
        count
    }

    /// Get movement speed multiplier based on leg health
    pub fn movement_speed_multiplier(&self) -> f32 {
        let legs = self.functional_legs();
        match legs {
            2 => 1.0,
            1 => 0.5,
            0 => 0.0,
            _ => 1.0,
        }
    }

    /// Get tool use efficiency based on arm health
    pub fn tool_efficiency_multiplier(&self) -> f32 {
        let arms = self.functional_arms();
        match arms {
            2 => 1.0,
            1 => 0.7,
            0 => 0.0,
            _ => 1.0,
        }
    }

    /// Equip armor/clothing on a body part
    pub fn equip_on_part(&mut self, part_type: BodyPartType, item_id: String, protection: f32) {
        if let Some(part) = self.parts.get_mut(&part_type) {
            part.equip(item_id, protection);
        }
    }

    /// Unequip from a body part
    pub fn unequip_from_part(&mut self, part_type: BodyPartType) -> Option<String> {
        self.parts.get_mut(&part_type).and_then(|p| p.unequip())
    }

    /// Get all equipped items
    pub fn get_equipped_items(&self) -> Vec<(BodyPartType, String)> {
        self.parts
            .iter()
            .filter_map(|(part_type, part)| {
                part.equipped_item
                    .as_ref()
                    .map(|item| (*part_type, item.clone()))
            })
            .collect()
    }

    /// Process all body parts (tick effects like bleeding)
    pub fn tick(&mut self) {
        for part in self.parts.values_mut() {
            part.tick();
        }
    }

    /// Get body summary for display
    pub fn summary(&self) -> BodySummary {
        let mut injured_parts = Vec::new();
        let mut disabled_parts = Vec::new();
        let mut total_protection = 0.0;

        for (part_type, part) in &self.parts {
            if matches!(
                part.status,
                BodyPartStatus::Injured | BodyPartStatus::Crippled
            ) {
                injured_parts.push(*part_type);
            }
            if matches!(
                part.status,
                BodyPartStatus::Disabled | BodyPartStatus::Missing
            ) {
                disabled_parts.push(*part_type);
            }
            total_protection += part.protection;
        }

        BodySummary {
            overall_health: self.overall_health(),
            injured_parts,
            disabled_parts,
            can_walk: self.can_walk(),
            can_use_tools: self.can_use_tools(),
            average_protection: total_protection / self.parts.len() as f32,
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::new()
    }
}

/// Body summary for inspection/display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodySummary {
    pub overall_health: f32,
    pub injured_parts: Vec<BodyPartType>,
    pub disabled_parts: Vec<BodyPartType>,
    pub can_walk: bool,
    pub can_use_tools: bool,
    pub average_protection: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_creation() {
        let body = Body::new();
        assert_eq!(body.parts.len(), 7);
        assert!(body.is_alive());
        assert!(body.can_walk());
        assert!(body.can_use_tools());
    }

    #[test]
    fn test_body_part_damage() {
        let mut body = Body::new();
        let initial_health = body
            .get_part(BodyPartType::LeftArm)
            .unwrap()
            .health;

        body.damage_part(BodyPartType::LeftArm, 20.0);

        let after_damage = body
            .get_part(BodyPartType::LeftArm)
            .unwrap()
            .health;
        assert!(after_damage < initial_health);
    }

    #[test]
    fn test_body_part_healing() {
        let mut body = Body::new();
        body.damage_part(BodyPartType::Head, 30.0);

        let damaged_health = body.get_part(BodyPartType::Head).unwrap().health;

        body.heal_part(BodyPartType::Head, 15.0);

        let healed_health = body.get_part(BodyPartType::Head).unwrap().health;
        assert!(healed_health > damaged_health);
    }

    #[test]
    fn test_critical_part_death() {
        let mut body = Body::new();
        assert!(body.is_alive());

        // Destroy head (critical)
        body.damage_part(BodyPartType::Head, 1000.0);
        assert!(!body.is_alive());
    }

    #[test]
    fn test_movement_with_leg_damage() {
        let mut body = Body::new();
        assert_eq!(body.movement_speed_multiplier(), 1.0);

        // Disable one leg
        body.damage_part(BodyPartType::LeftLeg, 1000.0);
        assert_eq!(body.movement_speed_multiplier(), 0.5);

        // Disable both legs
        body.damage_part(BodyPartType::RightLeg, 1000.0);
        assert_eq!(body.movement_speed_multiplier(), 0.0);
        assert!(!body.can_walk());
    }

    #[test]
    fn test_tool_use_with_arm_damage() {
        let mut body = Body::new();
        assert_eq!(body.tool_efficiency_multiplier(), 1.0);

        // Disable one arm
        body.damage_part(BodyPartType::RightArm, 1000.0);
        assert_eq!(body.tool_efficiency_multiplier(), 0.7);

        // Disable both arms
        body.damage_part(BodyPartType::LeftArm, 1000.0);
        assert_eq!(body.tool_efficiency_multiplier(), 0.0);
        assert!(!body.can_use_tools());
    }

    #[test]
    fn test_equipment() {
        let mut body = Body::new();

        body.equip_on_part(
            BodyPartType::Head,
            "iron_helmet".to_string(),
            0.5,
        );

        let helmet = body
            .get_part(BodyPartType::Head)
            .unwrap()
            .equipped_item
            .as_ref();
        assert_eq!(helmet, Some(&"iron_helmet".to_string()));

        let protection = body.get_part(BodyPartType::Head).unwrap().protection;
        assert_eq!(protection, 0.5);
    }

    #[test]
    fn test_armor_damage_reduction() {
        let mut body = Body::new();

        // Equip armor with 50% protection
        body.equip_on_part(
            BodyPartType::Torso,
            "iron_chestplate".to_string(),
            0.5,
        );

        let initial_health = body.get_part(BodyPartType::Torso).unwrap().health;

        // Take 20 damage (should be reduced to 10)
        body.damage_part(BodyPartType::Torso, 20.0);

        let final_health = body.get_part(BodyPartType::Torso).unwrap().health;
        assert_eq!(initial_health - final_health, 10.0);
    }

    #[test]
    fn test_body_part_status_update() {
        let mut part = BodyPart::new(BodyPartType::LeftArm);
        assert_eq!(part.status, BodyPartStatus::Healthy);

        part.take_damage(20.0);
        assert_eq!(part.status, BodyPartStatus::Injured);

        part.take_damage(20.0);
        assert_eq!(part.status, BodyPartStatus::Crippled);

        part.take_damage(30.0);
        assert_eq!(part.status, BodyPartStatus::Missing);
    }

    #[test]
    fn test_bleeding_condition() {
        let mut part = BodyPart::new(BodyPartType::LeftLeg);
        let initial_health = part.health;

        part.add_condition(Condition {
            condition_type: ConditionType::Bleeding,
            severity: 0.5,
            duration: 10,
        });

        part.tick();

        assert!(part.health < initial_health);
        assert_eq!(part.conditions[0].duration, 9);
    }
}
