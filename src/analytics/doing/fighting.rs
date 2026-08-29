// src/analytics/doing/fighting.rs
//! Threat, and the four answers to it.
//!
//! Turning on a man, turning on a beast, gentling a beast, freezing, and
//! running.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`]. The bodies are as they were when all fifty-two
//! lived in one five-thousand-line `match`; what changed is that a verb can
//! now be found, read and altered without scrolling past the other fifty-one.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::ActionResult;
use log::debug;
use rand::Rng;

impl Simulation {
    /// `Action::Attack`.
    pub(in crate::analytics) fn attacking(&mut self, target_agent_id: &uuid::Uuid, weapon: &Option<String>, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        use crate::agents::body::{BodyPartType, InjuryType};

        // Find target agent
        let target_index = self.population.agents.iter()
            .position(|a| &a.id == target_agent_id);

        if target_index.is_none() {
            return ActionResult::failure("Target agent not found".to_string());
        }
        let target_index = target_index.unwrap();

        // Can't attack yourself
        if target_index == agent_index {
            return ActionResult::failure("Cannot attack yourself".to_string());
        }

        // Get attacker and target positions
        let attacker_pos = self.population.agents[agent_index].state.position;
        let target_pos = self.population.agents[target_index].state.position;

        // Check if target is in range
        let distance = ((target_pos.0 - attacker_pos.0).abs() + (target_pos.1 - attacker_pos.1).abs()) as u32;

        // Get weapon range from equipment (melee = 1, ranged = further)
        let attacker = &self.population.agents[agent_index];
        let weapon_range = attacker.equipment.weapon_range();

        if (distance as f32) > weapon_range {
            return ActionResult::failure(format!("Target too far away (distance: {}, weapon range: {})", distance, weapon_range));
        }

        // Calculate weapon-based damage
        let attacker = &self.population.agents[agent_index];
        let weapon_damage = attacker.equipment.weapon_damage();
        let weapon_speed = attacker.equipment.weapon_attack_speed();

        // Get combat skill bonus (MeleeCombat for melee, Archery for ranged)
        let skill_level = attacker.skills.get_skill_if_exists(crate::agents::SkillType::MeleeCombat)
            .map(|s| s.level)
            .unwrap_or(0);
        let skill_modifier = 1.0 + (skill_level as f32 / 20.0); // -10 to 10 -> 0.5 to 1.5

        // Calculate base damage with variance
        // Weapon speed affects damage: faster weapons deal slightly less damage per hit
        let damage_variance = rng.gen_range(0.8..1.2); // +/- 20% randomness
        let speed_factor = (2.0 - weapon_speed).max(0.5); // Fast weapons (1.5) -> 0.5, slow (0.6) -> 1.4
        let base_damage = weapon_damage * damage_variance * skill_modifier * speed_factor;

        // Get attacker's tool efficiency (arm health affects combat)
        let attacker = &self.population.agents[agent_index];
        let attacker_efficiency = attacker.body.tool_efficiency_multiplier();

        // Get mounted combat bonus (warhorses provide significant advantage!)
        let mount_bonus = attacker.transport.mounted_combat_bonus();
        let combat_multiplier = 1.0 + mount_bonus;

        // Apply all modifiers: base * arm_health * mount_bonus
        let actual_damage = base_damage * attacker_efficiency * combat_multiplier;

        // Select random body part to hit (weighted toward torso/limbs)
        let body_parts = [
            (BodyPartType::Head, 10),       // 10% chance (critical)
            (BodyPartType::Torso, 30),      // 30% chance (common target)
            (BodyPartType::LeftArm, 15),    // 15% chance
            (BodyPartType::RightArm, 15),   // 15% chance
            (BodyPartType::LeftLeg, 12),    // 12% chance
            (BodyPartType::RightLeg, 12),   // 12% chance
            (BodyPartType::Back, 6),        // 6% chance (hard to hit)
        ];

        let total_weight: u32 = body_parts.iter().map(|(_, w)| w).sum();
        let roll = rng.gen_range(0..total_weight);

        let mut cumulative = 0;
        let mut target_part = BodyPartType::Torso; // Default
        for (part, weight) in &body_parts {
            cumulative += weight;
            if roll < cumulative {
                target_part = *part;
                break;
            }
        }

        // Determine injury type based on damage and weapon
        let injury_type = if actual_damage >= 30.0 {
            // High damage can cause crippling injuries
            if rng.gen_bool(0.3) {
                InjuryType::Crippling(crate::agents::body::CripplingType::Partial)
            } else {
                InjuryType::Major
            }
        } else if actual_damage >= 15.0 {
            InjuryType::Major
        } else {
            InjuryType::Minor
        };

        // Apply damage to target
        let target = &mut self.population.agents[target_index];
        if let Some(part) = target.body.get_part_mut(target_part) {
            part.apply_injury(injury_type, actual_damage, self.current_tick as u64);
        }

        // Also reduce target's overall health
        let target = &mut self.population.agents[target_index];
        target.state.lose_health(actual_damage * 0.2, "a blow");

        // Get IDs before borrowing
        let attacker_id = self.population.agents[agent_index].id;
        let target_id = self.population.agents[target_index].id;

        // EMOTIONAL RESPONSE: Target responds emotionally to being attacked
        {
            // Calculate attacker's apparent strength
            let attacker = &self.population.agents[agent_index];
            let attacker_health = attacker.state.health / 100.0;
            let attacker_armor = attacker.equipment.total_armor() / 100.0;
            let attacker_has_weapon = attacker.equipment.get_weapon().is_some();
            let attacker_strength = attacker_health * (1.0 + attacker_armor * 0.5)
                + if attacker_has_weapon { 0.3 } else { 0.0 };

            // Target responds to threat
            let target = &mut self.population.agents[target_index];
            let emotion_source = crate::agents::EmotionSource::Agent(attacker_id);

            // Record who attacked for potential retaliation
            target.emotions.record_attack(attacker_id, self.current_tick);

            // Scale emotional response by damage severity
            let damage_severity = (actual_damage / 50.0).min(1.0);

            // Use threat assessment to determine fear vs anger
            target.respond_to_threat(attacker_strength + damage_severity * 0.5, emotion_source);

            debug!(
                "Agent {} emotional response to attack: fear={:.2}, anger={:.2}, should_flee={}, should_attack={}",
                target_id, target.emotions.fear, target.emotions.anger,
                target.emotions.should_flee(), target.emotions.should_attack()
            );
        }

        // And what it does to what the two of them are to each other.
        //
        // The executor dealt damage, wrote anger and broke a bone, and
        // never touched the relationship, so a man who had just been
        // hit went on counting the man who hit him a close friend and
        // the settlement graph had no hostile edge anywhere in it.
        {
            use crate::agents::Relationship;

            let current_tick = self.current_tick;

            let struck = self.population.agents[target_index]
                .relationships
                .get_or_create_relationship(attacker_id, current_tick);
            struck.weaken(Relationship::WHAT_A_BLOW_COSTS);
            struck.settle_what_we_are();

            // You do not warm to somebody you have just hit either
            let striking = self.population.agents[agent_index]
                .relationships
                .get_or_create_relationship(target_id, current_tick);
            striking.weaken(Relationship::WHAT_THROWING_ONE_COSTS);
            striking.settle_what_we_are();
        }

        // Check if target died from the attack
        let target_alive = self.population.agents[target_index].body.is_alive()
            && self.population.agents[target_index].state.health > 0.0;

        // What each of them takes away from it.
        //
        // "If an agent has fought back and won, then fighting becomes
        // a more attractive option. If an agent has fought back and
        // lost, then running away becomes a more attractive option."
        // The record is kept in the same place every other lesson is,
        // and it moves what the agent reckons itself worth the next
        // time something comes at it - see `Agent::own_strength`.
        {
            use crate::agents::practices::Undertaking;

            let attacker_standing = self.population.agents[agent_index]
                .state
                .health;
            let target_standing = self.population.agents[target_index]
                .state
                .health;

            // The attacker has won if the other one is down, and is
            // losing if it is the worse off of the two
            let attacker_won = !target_alive || attacker_standing > target_standing;

            self.population.agents[agent_index]
                .lessons
                .record(Undertaking::Fighting, attacker_won);

            // And the one being set upon learns from standing there
            // just as much. Being alive at the end of it is the whole
            // of winning, from that side.
            if target_alive {
                self.population.agents[target_index]
                    .lessons
                    .record(Undertaking::Fighting, !attacker_won);
            } else {
                self.population.agents[target_index]
                    .lessons
                    .record(Undertaking::Fighting, false);
            }
        }

        let attacker_mounted = self.population.agents[agent_index].transport.is_mounted();

        // Emit conflict event for timeline
        #[cfg(feature = "gui")]
        {
            use crate::gui::events::{SimulationEvent, SimulationEventType};
            let event = SimulationEvent::new(
                self.current_tick,
                SimulationEventType::Conflict {
                    attacker_id,
                    target_id,
                    damage: actual_damage,
                    fatal: !target_alive,
                },
                Some((attacker_pos.0, attacker_pos.1)),
            );
            self.population.pending_events.push(event);
        }

        debug!(
            "Agent {} attacked Agent {} ({:?}): {:.1} damage to {:?} ({}, mounted: {}, bonus: +{:.0}%)",
            attacker_id,
            self.population.agents[target_index].id,
            weapon.as_ref().unwrap_or(&"unarmed".to_string()),
            actual_damage,
            target_part,
            if target_alive { "survived" } else { "FATAL" },
            if attacker_mounted { "yes" } else { "no" },
            mount_bonus * 100.0
        );

        // Grant combat XP (more for kills, check weapon type for skill)
        let attacker = &mut self.population.agents[agent_index];
        let combat_xp = if !target_alive { 5 } else { 2 };
        // TODO: Check weapon type for Archery vs MeleeCombat
        attacker.skills.practise(crate::agents::skills::SkillType::MeleeCombat, combat_xp, tick_now);

        if !target_alive {
            ActionResult::success()
                .with_drive_change(DriveType::Safety, -0.3)
                .with_energy_cost(25.0)
                .with_message(format!(
                    "Attacked and killed target ({:.1} damage to {:?})",
                    actual_damage, target_part
                ))
        } else {
            ActionResult::success()
                .with_drive_change(DriveType::Safety, -0.2)
                .with_energy_cost(15.0)
                .with_message(format!(
                    "Attacked target ({:.1} damage to {:?}, {:?} injury)",
                    actual_damage, target_part, injury_type
                ))
        }
    }

    /// `Action::Fight`.
    ///
    /// `_weapon` is not a mistake in the signature: **nothing in here reads
    /// it**. A man standing his ground against a wolf fights it the same with
    /// a flint spear in his hand as with nothing, which is not what the tool
    /// ladder says and not what `hunting` does two modules away. Left as it
    /// was, because this split is behaviour-neutral by contract; see
    /// ISSUES_FOUND.md #95.
    pub(in crate::analytics) fn fighting_a_beast(&mut self, animal_id: &uuid::Uuid, _weapon: &Option<String>, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        // Standing your ground. The agent is not after this thing's
        // skin - it is here because the thing is close enough to be a
        // problem and the agent reckons it can be driven off.
        let (species, animal_position) = {
            let Some(animal) = self.world.animals.get(animal_id) else {
                return ActionResult::failure("Nothing there to fight".to_string());
            };
            if !animal.is_alive() {
                return ActionResult::failure("It is already dead".to_string());
            }

            let species_id = animal.species_id.clone();
            let position = animal.position;
            match self.world.animals.get_species(&species_id) {
                Some(found) => (found.clone(), position),
                None => return ActionResult::failure("Unknown creature".to_string()),
            }
        };

        // You cannot fight what you cannot reach, and an agent that
        // stands its ground does not go looking - if the thing has
        // moved off, that is the fight over.
        let standing = self.population.agents[agent_index].state.position;
        let reach = (animal_position.0 - standing.0)
            .abs()
            .max((animal_position.1 - standing.1).abs());
        if reach > Self::HUNT_REACH {
            return ActionResult::failure(format!(
                "{} is {} tiles off",
                species.name, reach
            ));
        }

        // Whether the blow lands is what the agent is worth against
        // what the creature is worth, on the same scale the appraisal
        // used to decide to be here at all.
        let mine = self.population.agents[agent_index].own_strength();
        let condition = {
            let animal = self.world.animals.get(animal_id);
            animal
                .map(|a| (a.current_health / species.health.max(1.0)).clamp(0.0, 1.0))
                .unwrap_or(1.0)
        };
        let theirs = condition * (species.attack_damage / 20.0).clamp(0.1, 2.0);
        let odds = (mine / (mine + theirs).max(0.01)).clamp(0.1, 0.9);

        let landed = rng.gen_bool(odds as f64);

        // A blow struck in a fight teaches the arm that struck it,
        // whichever way the fight goes
        self.population.agents[agent_index].skills.practise(
            crate::agents::skills::SkillType::MeleeCombat,
            if landed { 25 } else { 10 },
            tick_now,
        );

        if landed {
            let hurt = 25.0 + mine * 25.0;
            let killed = {
                let Some(animal) = self.world.animals.get_mut(animal_id) else {
                    return ActionResult::failure("Nothing there to fight".to_string());
                };
                animal.take_damage(hurt);
                !animal.is_alive()
            };

            // Winning without a mark on you is what teaches an agent
            // that fighting is worth doing
            self.population.agents[agent_index]
                .lessons
                .record(crate::agents::practices::Undertaking::Fighting, true);

            if !killed {
                return ActionResult::success()
                    .with_energy_cost(12.0)
                    .with_experience(3.0)
                    .with_message(format!("Beat {} back", species.name));
            }

            // What is killed in a fight is still worth butchering -
            // a wolf driven off is a wolf, a wolf killed is a hide
            let mut items_gained = Vec::new();
            for drop in &species.drops {
                if rng.gen_bool(drop.drop_chance as f64) {
                    let quantity =
                        rng.gen_range(drop.min_quantity..=drop.max_quantity);
                    items_gained.push(crate::environment::ItemStack {
                        material_id: drop.material_id.clone(),
                        quantity,
                    });
                }
            }

            let knife = self.population.agents[agent_index]
                .how_much_my_tools_help(
                    crate::agents::skills::SkillType::Leatherworking,
                );
            let butchered = self.butcher(&items_gained, knife);
            {
                let where_it_fell = {
                    let at = self.population.agents[agent_index].state.position;
                    crate::world::Position::new(at.0, at.1)
                };
                self.into_the_pack_or_on_the_ground(
                    agent_index,
                    butchered,
                    where_it_fell,
                );
            }

            let mut result = ActionResult::success()
                .with_drive_change(DriveType::Safety, -0.3)
                .with_energy_cost(20.0)
                .with_experience(6.0)
                .with_message(format!("Killed {}", species.name));
            for item in items_gained {
                result = result.with_item_gained(item);
            }
            result
        } else {
            // It got the better of the exchange
            let damage = species.attack_damage.max(1.0);
            let agent = &mut self.population.agents[agent_index];
            let came_off_well = damage < agent.state.health * Self::A_SCRATCH;
            agent.take_damage(damage);
            agent
                .lessons
                .record(crate::agents::practices::Undertaking::Fighting, came_off_well);

            ActionResult::failure(format!(
                "{} got the better of it ({:.0} damage)",
                species.name, damage
            ))
            .with_energy_cost(12.0)
        }
    }

    /// `Action::Tame`.
    pub(in crate::analytics) fn taming(&mut self, animal_id: &uuid::Uuid, food_type: &Option<String>, agent_index: usize, tick_now: u32) -> ActionResult {
        // Get species data first (clone to avoid borrow issues)
        let species = {
            if let Some(animal) = self.world.animals.get(animal_id) {
                if !animal.is_alive() {
                    return ActionResult::failure("Animal is dead".to_string());
                }
                if animal.is_domesticated {
                    return ActionResult::failure("Animal is already domesticated".to_string());
                }

                let species_id = animal.species_id.clone();
                match self.world.animals.get_species(&species_id) {
                    Some(s) => s.clone(),
                    None => return ActionResult::failure("Unknown animal species".to_string()),
                }
            } else {
                return ActionResult::failure("Animal not found".to_string());
            }
        };

        if !species.can_domesticate {
            return ActionResult::failure(format!("{} cannot be domesticated", species.name));
        }

        // Calculate taming progress based on food and agent relationship skills (using Farming)
        let agent = &self.population.agents[agent_index];
        let social_skill = agent.skills.get_skill_if_exists(crate::agents::skills::SkillType::Farming)
            .map(|s| s.level)
            .unwrap_or(-5);
        let taming_bonus = if food_type.is_some() { 0.15 } else { 0.05 };
        let taming_progress = 0.1 + (social_skill as f32 * 0.02) + taming_bonus;

        // Now get mutable reference to animal
        if let Some(animal) = self.world.animals.get_mut(animal_id) {
            animal.tame(taming_progress);

            if animal.is_domesticated {
                // Successfully domesticated
                animal.owner_id = Some(agent.id);

                // Create Transport for the tamed animal (if suitable)
                let transport_type = match species.name.as_str() {
                    "Cow" => Some(crate::agents::transport::TransportType::OxCart),
                    "Sheep" => Some(crate::agents::transport::TransportType::PackDonkey),
                    "Goat" => Some(crate::agents::transport::TransportType::PackDonkey),
                    // Rabbit, Chicken, and Wild Boar are too small or unsuitable for transport
                    _ => None,
                };

                // Increase social skill
                let agent = &mut self.population.agents[agent_index];
                agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, tick_now);

                // Add transport to agent's inventory if applicable
                if let Some(t_type) = transport_type {
                    let transport = crate::agents::transport::Transport::with_animal(t_type, *animal_id);
                    agent.transport.add_transport(transport);
                }

                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.3)
                    .with_energy_cost(10.0)
                    .with_experience(10.0)
                    .with_message(format!("Successfully tamed {}!", species.name))
            } else {
                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.1)
                    .with_energy_cost(8.0)
                    .with_message(format!("Made progress taming {} ({:.0}%)", species.name, animal.tame_level * 100.0))
            }
        } else {
            ActionResult::failure("Animal not found".to_string())
        }
    }

    /// `Action::Freeze`.
    pub(in crate::analytics) fn freezing(&mut self, agent_index: usize) -> ActionResult {
        // The third answer, and the only one nobody arrives at on
        // purpose: it is what is left when a body can neither run nor
        // raise a hand. Nothing happens. The agent stays exactly where
        // it is, which is the whole of what freezing costs - whatever
        // was coming is still coming, and is now a tick closer.
        let agent = &self.population.agents[agent_index];
        debug!("Agent {} froze", agent.id);

        ActionResult::success()
            .with_energy_cost(Self::WHAT_FREEZING_COSTS)
            .with_message("Froze".to_string())
    }

    /// `Action::FleeFrom`.
    pub(in crate::analytics) fn fleeing_from(&mut self, away_from: &(i32, i32, i32), agent_index: usize) -> ActionResult {
        // Running is not walking. A frightened person covers more
        // ground in a turn and is a good deal more tired at the end of
        // it, and this is where that difference lives rather than
        // being a `Move` the matrix cannot tell from a stroll.
        let stood = self.population.agents[agent_index].state.position;

        // Where it goes is the decision's question as well as this
        // one's, and both of them ask it here.
        let landed = self.where_this_one_would_run(
            &self.population.agents[agent_index].exploration_knowledge,
            stood,
            (away_from.0, away_from.1),
        );

        let Some(landed) = landed else {
            // Standing your ground, which is not the same as refusing
            // to answer. This was a failure, and a failure nothing
            // about the next turn could change: the same agent, in
            // the same corner, with the same thing in front of it,
            // refused again. Whatever a person hemmed in does, it is
            // not nothing, forever - so it costs a turn and stands,
            // which is what freezing is.
            debug!(
                "Agent {} had nowhere to run and stood",
                self.population.agents[agent_index].id
            );

            return ActionResult::success()
                .with_energy_cost(Self::WHAT_FREEZING_COSTS)
                .with_message("Nowhere to run, and stood".to_string());
        };

        let agent = &mut self.population.agents[agent_index];
        agent.state.position = landed;

        debug!("Agent {} bolted to {landed:?}", agent.id);

        ActionResult::success()
            .with_drive_change(DriveType::Safety, -0.4)
            .with_energy_cost(Self::WHAT_RUNNING_COSTS)
            .with_message("Ran".to_string())
    }
}
