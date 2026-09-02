// src/agents/body.rs
//! Body part system for agents with anatomical structure and injury tracking.

use crate::agents::equipment::{Equipment, EquipmentSlot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Type of injury
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjuryType {
    /// Small injuries, little damage, heal quickly
    Minor,
    /// Larger injuries, decent damage, heal slowly but fully
    Major,
    /// Life-threatening, significant damage, may not heal fully
    Crippling(CripplingType),
}

/// Severity of crippling injury
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CripplingType {
    /// Partial loss of function (one eye, limp, decreased strength)
    Partial,
    /// Total loss of limb/sense (blindness, deafness, amputation)
    Full,
}

impl InjuryType {
    /// Get healing rate per tick
    pub fn healing_rate(&self) -> f32 {
        match self {
            InjuryType::Minor => 0.5,                        // Heals 0.5 HP/tick
            InjuryType::Major => 0.1,                        // Heals 0.1 HP/tick
            InjuryType::Crippling(CripplingType::Partial) => 0.05, // Very slow
            InjuryType::Crippling(CripplingType::Full) => 0.0,     // Does not heal
        }
    }

    /// Get maximum recovery percentage
    pub fn max_recovery(&self) -> f32 {
        match self {
            InjuryType::Minor => 1.0,                        // 100% recovery
            InjuryType::Major => 1.0,                        // 100% recovery
            InjuryType::Crippling(CripplingType::Partial) => 0.7, // Recovers to 70%
            InjuryType::Crippling(CripplingType::Full) => 0.0,    // No recovery
        }
    }

    /// Check if injury causes permanent impairment
    pub fn is_permanent(&self) -> bool {
        matches!(self, InjuryType::Crippling(_))
    }
}

/// An active injury on a body part
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Injury {
    pub injury_type: InjuryType,
    pub damage_taken: f32,
    pub healing_progress: f32, // 0.0 to damage_taken
    pub timestamp: u64,        // When injury occurred
}

impl Injury {
    pub fn new(injury_type: InjuryType, damage: f32, timestamp: u64) -> Self {
        Self {
            injury_type,
            damage_taken: damage,
            healing_progress: 0.0,
            timestamp,
        }
    }

    /// Heal this injury
    pub fn heal(&mut self, amount: f32) -> f32 {
        let max_healing = self.max_recoverable_damage();
        let can_heal = max_healing - self.healing_progress;
        let actual_heal = amount.min(can_heal);

        self.healing_progress += actual_heal;
        actual_heal
    }

    /// Get maximum amount that can be recovered from this injury
    fn max_recoverable_damage(&self) -> f32 {
        self.damage_taken * self.injury_type.max_recovery()
    }

    /// Check if injury is fully healed
    pub fn is_healed(&self) -> bool {
        let max_heal = self.max_recoverable_damage();
        // If there's nothing that can be healed (full crippling), never consider it "healed"
        if max_heal <= 0.0 {
            return false;
        }
        // Use small epsilon for floating point comparison
        (self.healing_progress - max_heal).abs() < 0.01 || self.healing_progress >= max_heal
    }

}

/// Body part types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
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

    /// Active injuries being healed
    pub injuries: Vec<Injury>,

    /// Permanent impairment factor (0.0 = no impairment, 1.0 = fully impaired)
    pub permanent_impairment: f32,

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
            injuries: Vec::new(),
            permanent_impairment: 0.0,
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

    /// Apply injury with specific type
    pub fn apply_injury(&mut self, injury_type: InjuryType, damage: f32, timestamp: u64) {
        // Apply the damage
        self.health = (self.health - damage).max(0.0);

        // Create injury record
        let injury = Injury::new(injury_type, damage, timestamp);
        self.injuries.push(injury);

        // Update permanent impairment for crippling injuries
        if injury_type.is_permanent() {
            let permanent_dmg = damage * (1.0 - injury_type.max_recovery());
            let impairment_increase = permanent_dmg / self.max_health;
            self.permanent_impairment = (self.permanent_impairment + impairment_increase).min(1.0);
        }

        self.update_status();
    }

    /// Heal this body part (processes injuries)
    pub fn heal(&mut self, amount: f32) {
        let mut remaining_healing = amount;

        // Heal injuries in order from oldest to newest
        for injury in &mut self.injuries {
            if remaining_healing <= 0.0 {
                break;
            }

            let healed = injury.heal(remaining_healing);
            remaining_healing -= healed;
            self.health = (self.health + healed).min(self.max_health);
        }

        // Remove fully healed injuries
        self.injuries.retain(|inj| !inj.is_healed());

        // Apply any remaining healing directly to health (for non-injury damage)
        if remaining_healing > 0.0 {
            self.health = (self.health + remaining_healing).min(self.max_health);
        }

        self.update_status();
    }

    /// Natural healing tick (uses injury healing rates)
    pub fn tick_natural_healing(&mut self) {
        for injury in &mut self.injuries {
            let heal_amount = injury.injury_type.healing_rate();
            if heal_amount > 0.0 {
                let healed = injury.heal(heal_amount);
                self.health = (self.health + healed).min(self.max_health);
            }
        }

        // Remove fully healed injuries
        self.injuries.retain(|inj| !inj.is_healed());

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

    /// Get effective functionality (0.0 to 1.0) accounting for permanent impairment
    pub fn effectiveness(&self) -> f32 {
        if !self.is_functional() {
            return 0.0;
        }

        let health_factor = self.health_percentage();
        let impairment_factor = 1.0 - self.permanent_impairment;

        health_factor * impairment_factor
    }

    /// Check if part has permanent impairment
    pub fn has_permanent_impairment(&self) -> bool {
        self.permanent_impairment > 0.0
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

        // Natural healing for injuries
        self.tick_natural_healing();
    }
}

/// Complete body system for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    pub parts: BTreeMap<BodyPartType, BodyPart>,
    /// Equipped items by slot
    pub equipment: BTreeMap<EquipmentSlot, Equipment>,
}

impl Body {
    pub fn new() -> Self {
        let mut parts = BTreeMap::new();

        for part_type in BodyPartType::all() {
            parts.insert(part_type, BodyPart::new(part_type));
        }

        Self {
            parts,
            equipment: BTreeMap::new(),
        }
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



    /// Get movement speed multiplier based on leg health
    pub fn movement_speed_multiplier(&self) -> f32 {
        let left_leg = self.parts.get(&BodyPartType::LeftLeg)
            .map(|p| p.effectiveness())
            .unwrap_or(0.0);
        let right_leg = self.parts.get(&BodyPartType::RightLeg)
            .map(|p| p.effectiveness())
            .unwrap_or(0.0);

        // Average effectiveness of both legs
        (left_leg + right_leg) / 2.0
    }

    /// How much this body can lift and carry, against a whole one.
    ///
    /// Arms and torso, which is what carrying actually is. Carrying capacity
    /// was scaled by `movement_speed_multiplier` under a comment calling it a
    /// strength - that is the leg-health figure, so how much somebody could
    /// carry was decided by how well they walked. See ISSUES #87.
    ///
    /// A body with one arm carries less than one with two and more than one
    /// with none, so the two arms average rather than taking the better; the
    /// torso is the back the load sits on and counts for as much as both arms
    /// together.
    pub fn how_much_this_body_can_lift(&self) -> f32 {
        let arm = |which| {
            self.parts
                .get(&which)
                .map(|p| p.effectiveness())
                .unwrap_or(0.0)
        };
        let arms = (arm(BodyPartType::LeftArm) + arm(BodyPartType::RightArm)) / 2.0;
        let back = self
            .parts
            .get(&BodyPartType::Torso)
            .map(|p| p.effectiveness())
            .unwrap_or(1.0);

        (arms + back) / 2.0
    }

    /// Get tool use efficiency based on arm health
    pub fn tool_efficiency_multiplier(&self) -> f32 {
        let left_arm = self.parts.get(&BodyPartType::LeftArm)
            .map(|p| p.effectiveness())
            .unwrap_or(0.0);
        let right_arm = self.parts.get(&BodyPartType::RightArm)
            .map(|p| p.effectiveness())
            .unwrap_or(0.0);

        // Use better arm effectiveness
        left_arm.max(right_arm)
    }

    /// Equip armor/clothing on a body part
    pub fn equip_on_part(&mut self, part_type: BodyPartType, item_id: String, protection: f32) {
        if let Some(part) = self.parts.get_mut(&part_type) {
            part.equip(item_id, protection);
        }
    }



    /// Equip an item of clothing/armor
    pub fn equip(&mut self, item: Equipment) {
        // Update armor protection on covered body parts
        let armor = item.armor_protection();
        for part_type in item.slot.covered_parts() {
            if let Some(part) = self.parts.get_mut(&part_type) {
                part.protection = armor;
                part.equipped_item = Some(item.name.clone());
            }
        }

        self.equipment.insert(item.slot, item);
    }

    /// Unequip an item from a slot
    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<Equipment> {
        if let Some(item) = self.equipment.remove(&slot) {
            // Remove protection from covered body parts
            for part_type in slot.covered_parts() {
                if let Some(part) = self.parts.get_mut(&part_type) {
                    part.protection = 0.0;
                    part.equipped_item = None;
                }
            }
            Some(item)
        } else {
            None
        }
    }

    /// Get total cold insulation from all equipped items
    pub fn total_cold_insulation(&self) -> f32 {
        self.equipment.values().map(|e| e.cold_insulation()).sum()
    }

    /// Get total heat resistance from all equipped items
    pub fn total_heat_resistance(&self) -> f32 {
        self.equipment.values().map(|e| e.heat_resistance()).sum()
    }

    /// Tick wear on all equipped items
    pub fn tick_equipment_wear(&mut self) {
        for item in self.equipment.values_mut() {
            item.tick_wear();
        }

        // Remove broken items
        let broken_slots: Vec<EquipmentSlot> = self
            .equipment
            .iter()
            .filter(|(_, item)| item.is_broken())
            .map(|(slot, _)| *slot)
            .collect();

        for slot in broken_slots {
            self.unequip(slot);
        }
    }

    /// Process all body parts (tick effects like bleeding)
    pub fn tick(&mut self) {
        for part in self.parts.values_mut() {
            part.tick();
        }
        self.tick_equipment_wear();
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

        // Disable one arm - can still use the other arm at full effectiveness
        body.damage_part(BodyPartType::RightArm, 1000.0);
        assert_eq!(body.tool_efficiency_multiplier(), 1.0); // Uses left arm

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

    #[test]
    fn test_minor_injury_healing() {
        let mut part = BodyPart::new(BodyPartType::LeftArm);
        let initial_health = part.health;

        // Apply minor injury (10 damage)
        part.apply_injury(InjuryType::Minor, 10.0, 0);

        assert_eq!(part.health, initial_health - 10.0);
        assert_eq!(part.injuries.len(), 1);

        // Minor injuries heal quickly (0.5 HP/tick)
        part.tick_natural_healing();
        assert_eq!(part.health, initial_health - 9.5);

        // After 20 ticks, should be fully healed
        for _ in 0..19 {
            part.tick_natural_healing();
        }

        assert_eq!(part.health, initial_health);
        assert_eq!(part.injuries.len(), 0); // Injury removed when healed
    }

    #[test]
    fn test_major_injury_healing() {
        let mut part = BodyPart::new(BodyPartType::Torso);
        let initial_health = part.health;

        // Apply major injury (30 damage)
        part.apply_injury(InjuryType::Major, 30.0, 0);

        assert_eq!(part.health, initial_health - 30.0);

        // Major injuries heal slowly (0.1 HP/tick)
        part.tick_natural_healing();
        assert_eq!(part.health, initial_health - 29.9);

        // After 300 ticks, should be fully healed
        for _ in 0..299 {
            part.tick_natural_healing();
        }

        assert!((part.health - initial_health).abs() < 0.01); // Use tolerance for floating point
        assert!(part.injuries.is_empty());
        assert_eq!(part.permanent_impairment, 0.0); // Major injuries don't cause permanent damage
    }

    #[test]
    fn test_partial_crippling_injury() {
        let mut part = BodyPart::new(BodyPartType::LeftLeg);
        let initial_health = part.health; // 70.0

        // Apply partial crippling injury (40 damage)
        part.apply_injury(InjuryType::Crippling(CripplingType::Partial), 40.0, 0);

        assert_eq!(part.health, initial_health - 40.0);
        assert_eq!(part.injuries.len(), 1);

        // Partial crippling: max recovery 70%, so 40 * 0.7 = 28 HP can be recovered
        // Permanent damage: 40 * 0.3 = 12 HP
        let expected_permanent_impairment = 12.0 / 70.0; // ~0.171
        assert!((part.permanent_impairment - expected_permanent_impairment).abs() < 0.01);

        // Heal very slowly (0.05 HP/tick)
        for _ in 0..560 {
            part.tick_natural_healing();
        }

        // Should recover to 70% of damage
        let expected_recovery = initial_health - 12.0; // 58.0
        assert!((part.health - expected_recovery).abs() < 0.1);
        assert!(part.injuries.is_empty()); // Injury fully healed (to its limit)

        // Permanent impairment remains
        assert!(part.permanent_impairment > 0.0);
        assert!(part.has_permanent_impairment());
    }

    #[test]
    fn test_full_crippling_injury() {
        let mut part = BodyPart::new(BodyPartType::RightArm);
        let initial_health = part.health; // 60.0

        // Apply full crippling injury (50 damage)
        part.apply_injury(InjuryType::Crippling(CripplingType::Full), 50.0, 0);

        assert_eq!(part.health, initial_health - 50.0);

        // Full crippling: no recovery (max_recovery = 0.0)
        // All damage is permanent
        let expected_permanent_impairment = 50.0 / 60.0; // ~0.833
        assert!((part.permanent_impairment - expected_permanent_impairment).abs() < 0.01);

        // Try to heal - should not heal at all
        for _ in 0..100 {
            part.tick_natural_healing();
        }

        assert_eq!(part.health, initial_health - 50.0); // No healing
        assert_eq!(part.injuries.len(), 1); // Injury never heals
    }

    #[test]
    fn test_effectiveness_with_permanent_impairment() {
        let mut part = BodyPart::new(BodyPartType::LeftLeg);

        // Start at 100% effectiveness
        assert_eq!(part.effectiveness(), 1.0);

        // Apply partial crippling injury (35 damage out of 70 max)
        part.apply_injury(InjuryType::Crippling(CripplingType::Partial), 35.0, 0);

        // Health is now 35/70 = 50%
        // Permanent impairment: 35 * 0.3 / 70 = ~0.15
        // Effectiveness = health_pct * (1 - impairment) = 0.5 * 0.85 = 0.425
        let expected_effectiveness = 0.5 * (1.0 - (35.0 * 0.3 / 70.0));
        assert!((part.effectiveness() - expected_effectiveness).abs() < 0.01);

        // After healing to 70% recovery
        for _ in 0..500 {
            part.tick_natural_healing();
        }

        // Health should be at ~59.5 (70% recovery of 35 damage = 24.5 healed)
        // Health: (35 + 24.5) / 70 = ~0.85
        // Permanent impairment: ~0.15
        // Effectiveness: 0.85 * 0.85 = ~0.72
        assert!(part.effectiveness() > 0.7);
        assert!(part.effectiveness() < 0.75);
    }

    #[test]
    fn test_body_movement_with_partial_crippling() {
        let mut body = Body::new();

        // Apply partial crippling to left leg
        if let Some(part) = body.get_part_mut(BodyPartType::LeftLeg) {
            part.apply_injury(InjuryType::Crippling(CripplingType::Partial), 30.0, 0);
        }

        // Movement speed should be reduced due to partial crippling
        // Left leg effectiveness: (70-30)/70 * (1 - 30*0.3/70) = 0.571 * 0.871 = ~0.497
        // Right leg effectiveness: 1.0
        // Average: (0.497 + 1.0) / 2 = ~0.75
        let speed = body.movement_speed_multiplier();
        assert!(speed > 0.7);
        assert!(speed < 0.8);
    }

    #[test]
    fn test_body_tool_use_with_crippling() {
        let mut body = Body::new();

        // Apply full crippling to left arm (amputation)
        if let Some(part) = body.get_part_mut(BodyPartType::LeftArm) {
            part.apply_injury(InjuryType::Crippling(CripplingType::Full), 60.0, 0);
        }

        // Tool efficiency should use right arm (better of the two)
        // Left arm: 0.0 effectiveness (fully crippled)
        // Right arm: 1.0 effectiveness
        // max(0.0, 1.0) = 1.0
        assert_eq!(body.tool_efficiency_multiplier(), 1.0);

        // Now cripple right arm partially
        if let Some(part) = body.get_part_mut(BodyPartType::RightArm) {
            part.apply_injury(InjuryType::Crippling(CripplingType::Partial), 30.0, 0);
        }

        // Right arm effectiveness reduced
        let efficiency = body.tool_efficiency_multiplier();
        assert!(efficiency > 0.4);
        assert!(efficiency < 0.6);
    }

    #[test]
    fn test_equip_clothing() {
        use crate::agents::equipment::ClothingTemplate;
        use crate::agents::skills::Quality;

        let mut body = Body::new();
        let tunic = ClothingTemplate::leather_tunic(Quality::Basic);

        // Equip the tunic
        body.equip(tunic);

        // Should have equipment in torso slot
        assert!(body.equipment.contains_key(&EquipmentSlot::Torso));

        // Torso should have protection
        let torso = body.get_part(BodyPartType::Torso).unwrap();
        assert!(torso.protection > 0.0);
        assert_eq!(torso.equipped_item, Some("Leather Tunic".to_string()));
    }

    #[test]
    fn test_unequip_clothing() {
        use crate::agents::equipment::ClothingTemplate;
        use crate::agents::skills::Quality;

        let mut body = Body::new();
        let tunic = ClothingTemplate::leather_tunic(Quality::Basic);

        body.equip(tunic);
        let removed = body.unequip(EquipmentSlot::Torso);

        assert!(removed.is_some());
        assert!(!body.equipment.contains_key(&EquipmentSlot::Torso));

        // Torso protection should be removed
        let torso = body.get_part(BodyPartType::Torso).unwrap();
        assert_eq!(torso.protection, 0.0);
        assert_eq!(torso.equipped_item, None);
    }

    #[test]
    fn test_cold_insulation() {
        use crate::agents::equipment::ClothingTemplate;
        use crate::agents::skills::Quality;

        let mut body = Body::new();

        // No clothing = no insulation
        assert_eq!(body.total_cold_insulation(), 0.0);

        // Add fur coat (excellent cold protection)
        body.equip(ClothingTemplate::fur_coat(Quality::Basic));
        let with_coat = body.total_cold_insulation();
        assert!(with_coat > 0.0);

        // Add fur hat
        body.equip(ClothingTemplate::fur_hat(Quality::Basic));
        let with_hat = body.total_cold_insulation();
        assert!(with_hat > with_coat);
    }

    #[test]
    fn test_heat_resistance() {
        use crate::agents::equipment::ClothingTemplate;
        use crate::agents::skills::Quality;

        let mut body = Body::new();

        // Linen provides excellent heat resistance
        body.equip(ClothingTemplate::linen_shirt(Quality::Basic));
        let linen_resistance = body.total_heat_resistance();

        let mut body2 = Body::new();
        // Fur provides poor heat resistance
        body2.equip(ClothingTemplate::fur_coat(Quality::Basic));
        let fur_resistance = body2.total_heat_resistance();

        assert!(linen_resistance > fur_resistance);
    }

    #[test]
    fn test_equipment_wear() {
        use crate::agents::equipment::ClothingTemplate;
        use crate::agents::skills::Quality;

        let mut body = Body::new();
        let mut tunic = ClothingTemplate::leather_tunic(Quality::Basic);

        // Set durability very low
        tunic.durability = 0.5;
        body.equip(tunic);

        // Tick should apply wear
        body.tick_equipment_wear();

        // Still equipped but with less durability
        assert!(body.equipment.contains_key(&EquipmentSlot::Torso));

        // Set to broken
        if let Some(item) = body.equipment.get_mut(&EquipmentSlot::Torso) {
            item.durability = 0.0;
        }

        // Tick should remove broken items
        body.tick_equipment_wear();
        assert!(!body.equipment.contains_key(&EquipmentSlot::Torso));
    }
}

