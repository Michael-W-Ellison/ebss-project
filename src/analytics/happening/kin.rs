// src/analytics/happening/kin.rs
//! Carrying, bearing, and feeding what cannot feed itself.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;
use crate::core::DriveType;
use log::debug;

impl Simulation {
    /// Somebody worth having a child with.
    ///
    /// `resolve_action_target` filled a nil Mate target with whoever happened
    /// to be nearest, which is neither a courtship nor a plan. Measured, Mate
    /// was 19.7% of everything a settlement did and failed 99.9% of the time:
    /// the target could not reproduce, or was too far off, or one of the two
    /// was barely fertile. One birth per thousand-odd attempts.
    ///
    /// Three things decide it, and trust is the first of them. Somebody an
    /// agent has not built up any confidence in is not somebody it will have a
    /// child with, however close they are standing - and trust here is the
    /// whole of what one agent thinks of another: the bond, whether they have
    /// been straight with it before, and what sort of people they both are.
    ///
    /// Then the plain facts of the matter, so the attempt can actually come to
    /// something: near enough, and a pair who could have a child at all.
    pub(in crate::analytics) fn somebody_to_have_a_child_with(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        use crate::agents::reproduction::{can_mate, MateSelectionCriteria};

        let criteria = MateSelectionCriteria::default();

        self.population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                let paces = (them.state.position.0 - agent_position.0)
                    .abs()
                    .max((them.state.position.1 - agent_position.1).abs());
                paces <= Self::CLOSE_ENOUGH_TO_COURT
            })
            .filter(|them| agent.would_take_their_word(them.id, &them.traits))
            .filter(|them| can_mate(agent, them, &criteria))
            .max_by(|a, b| {
                let trust = |them: &&crate::agents::Agent| {
                    agent.how_far_i_trust(them.id, &them.traits)
                };
                trust(a)
                    .partial_cmp(&trust(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|them| them.id)
    }

    /// Process pregnancies and handle births
    pub(in crate::analytics) fn process_pregnancies_and_births(&mut self) {
        use crate::agents::reproduction::give_birth;
        use crate::agents::gossip::{Information, InformationType};

        let current_tick = self.current_tick;

        // Collect births to process (to avoid borrowing issues)
        let mut births_to_process: Vec<(usize, uuid::Uuid)> = Vec::new();

        // First pass: update pregnancies and collect due births
        for (idx, agent) in self.population.agents.iter_mut().enumerate() {
            if let Some(ref mut pregnancy) = agent.pregnancy {
                // Update prenatal nutrition based on mother's current state
                let hunger_drive = agent.drives.get(DriveType::Hunger)
                    .map(|d| d.value)
                    .unwrap_or(0.0);
                pregnancy.update_nutrition(hunger_drive, agent.state.health);

                // Check if due
                if pregnancy.is_due(current_tick) {
                    births_to_process.push((idx, pregnancy.father_id));
                }
            }
        }

        // Second pass: process births
        for (mother_idx, father_id) in births_to_process {
            // Find the father
            let father_idx = self.population.agents.iter()
                .position(|a| a.id == father_id);

            // Get pregnancy data before clearing it
            let pregnancy = self.population.agents[mother_idx].pregnancy.take();

            if let Some(preg) = pregnancy {
                // Create offspring
                let offspring = if let Some(f_idx) = father_idx {
                    let mother = &self.population.agents[mother_idx];
                    let father = &self.population.agents[f_idx];
                    give_birth(mother, father, &preg, current_tick)
                } else {
                    // Father not found (dead?), use mother twice (not ideal but handles edge case)
                    let mother = &self.population.agents[mother_idx];
                    give_birth(mother, mother, &preg, current_tick)
                };

                let offspring_id = offspring.id;
                let mother_id = self.population.agents[mother_idx].id;
                let mother_pos = self.population.agents[mother_idx].state.position;

                // Add offspring to population
                self.population.agents.push(offspring);
                self.population.stats.total_births += 1;

                debug!(
                    "Agent {} gave birth to {}! Prenatal nutrition: {:.2}",
                    mother_id, offspring_id, preg.nutrition_quality
                );

                // Generate gossip about the birth
                let birth_info = Information::new(
                    InformationType::Childbirth {
                        agent: mother_id,
                        child: offspring_id,
                    },
                    mother_id,
                    true,
                    current_tick as u64,
                );

                // Share birth information with nearby agents
                for other_agent in &mut self.population.agents {
                    if other_agent.id != mother_id && other_agent.id != offspring_id {
                        let distance = {
                            let dx = (other_agent.state.position.0 - mother_pos.0) as f32;
                            let dy = (other_agent.state.position.1 - mother_pos.1) as f32;
                            (dx * dx + dy * dy).sqrt()
                        };

                        if distance <= 20.0 {
                            other_agent.knowledge.receive_information(
                                birth_info.clone(),
                                mother_id,
                                other_agent.id,
                                &other_agent.traits,
                                current_tick as u64,
                            );
                        }
                    }
                }

                // Add parent-child relationships
                let offspring_idx = self.population.agents.len() - 1;

                // Mother bonds with child
                use crate::agents::emotions::{Relationship, RelationshipType};
                self.population.agents[mother_idx].relationships.add_relationship(
                    Relationship::new(offspring_id, RelationshipType::Child)
                );

                // Father bonds with child (if alive)
                if let Some(f_idx) = father_idx {
                    self.population.agents[f_idx].relationships.add_relationship(
                        Relationship::new(offspring_id, RelationshipType::Child)
                    );
                }
            }
        }
    }

    /// How much of a child's belly one feed is.
    ///
    /// A third, so a fed child takes three or four in a day and stops when it
    /// is full. Filling the whole stomach every time somebody was standing
    /// nearby put several times what the child burned into it, and its mother
    /// paid for all of it.
    pub(in crate::analytics) const A_FEED_IS_THIS_MUCH_OF_A_BELLY: f32 = 3.0;

    /// Process nursing for infants
    pub(in crate::analytics) fn process_nursing(&mut self) {
        use crate::agents::childcare::{MAX_CAREGIVER_DISTANCE, NURSING_ENERGY_GAIN};
        use crate::agents::LifeStage;

        let current_tick = self.current_tick;

        // Collect caregiver positions for distance checks
        let caregiver_positions: std::collections::BTreeMap<uuid::Uuid, (i32, i32, i32)> =
            self.population.agents.iter()
                .filter(|a| a.state.is_alive)
                .map(|a| (a.id, a.state.position))
                .collect();

        // What each nursing costs the woman doing it, applied after the loop
        let mut what_the_milk_cost: Vec<(uuid::Uuid, f32)> = Vec::new();

        for agent in &mut self.population.agents {
            // Only process living infants with nursing state
            if !agent.state.is_alive || agent.state.life_stage != LifeStage::Infant {
                continue;
            }

            if let Some(ref mut nursing) = agent.nursing {
                // Check if still in nursing period
                if !nursing.needs_nursing(current_tick) {
                    // Nursing period ended
                    agent.nursing = None;
                    continue;
                }

                // Check if caregiver is nearby
                let agent_pos = agent.state.position;
                let caregiver_nearby = nursing.is_caregiver(nursing.primary_caregiver)
                    && caregiver_positions.get(&nursing.primary_caregiver)
                        .map(|&pos| {
                            let dx = (pos.0 - agent_pos.0) as f32;
                            let dy = (pos.1 - agent_pos.1) as f32;
                            (dx * dx + dy * dy).sqrt() <= MAX_CAREGIVER_DISTANCE
                        })
                        .unwrap_or(false);

                // Also check secondary caregivers
                let secondary_nearby = nursing.secondary_caregivers.iter()
                    .any(|&cg_id| {
                        caregiver_positions.get(&cg_id)
                            .map(|&pos| {
                                let dx = (pos.0 - agent_pos.0) as f32;
                                let dy = (pos.1 - agent_pos.1) as f32;
                                (dx * dx + dy * dy).sqrt() <= MAX_CAREGIVER_DISTANCE
                            })
                            .unwrap_or(false)
                    });

                if caregiver_nearby || secondary_nearby {
                    // Being nursed
                    nursing.nurse();

                    // Gain energy from nursing
                    agent.state.energy = (agent.state.energy + NURSING_ENERGY_GAIN).min(100.0);

                    // And milk, which is the point of the exercise.
                    //
                    // This did the line above and nothing else: five points of
                    // `energy`, a field that #70 measured as never scarce and
                    // that fires in `is_starving` exactly nought times in
                    // twenty thousand adult-turns. The stomach, the gut and the
                    // reserve - which are what starvation is reckoned on - never
                    // saw a drop. So a nursed infant was fed nothing, and had to
                    // forage for itself from the hour it was born, needing three
                    // and a half meals a day against a grown woman's two and a
                    // half because its stomach is a quarter the size and it
                    // burns more for its size. Every child born in every world
                    // ever measured died as an infant. See ISSUES #78.
                    //
                    // Fed on demand, a mouthful at a time: a child that has
                    // room takes one and a full one does not, so it regulates
                    // itself the way a fed child does rather than being filled
                    // every two hours whether it wants it or not.
                    let a_mouthful =
                        agent.state.physiology.stomach_capacity / Self::A_FEED_IS_THIS_MUCH_OF_A_BELLY;
                    let taken = if agent.state.physiology.room_for_another_mouthful() {
                        agent
                            .state
                            .physiology
                            .eat(a_mouthful, crate::agents::physiology::WHAT_MILK_IS_WORTH)
                    } else {
                        0.0
                    };
                    if taken > 0.0 {
                        agent.state.took_a_meal(current_tick, 0.0);
                        what_the_milk_cost.push((
                            nursing.primary_caregiver,
                            taken * crate::agents::physiology::WHAT_MILK_IS_WORTH,
                        ));
                    }

                    // Update developmental nutrition (well nursed)
                    let hunger_satisfaction = 1.0 - agent.drives.get(DriveType::Hunger)
                        .map(|d| d.value)
                        .unwrap_or(0.0);
                    agent.developmental_nutrition.update_infant_nutrition(hunger_satisfaction, true);
                } else {
                    // Not being nursed
                    nursing.tick_without_nursing();

                    // Apply health penalty if suffering
                    let penalty = nursing.health_penalty();
                    if penalty > 0.0 {
                        agent.state.lose_health(penalty, "illness");
                        debug!(
                            "Infant {} suffering from lack of nursing: -{:.1} health",
                            agent.id, penalty
                        );
                    }

                    // Update developmental nutrition (not nursed)
                    let hunger_satisfaction = 1.0 - agent.drives.get(DriveType::Hunger)
                        .map(|d| d.value)
                        .unwrap_or(0.0);
                    agent.developmental_nutrition.update_infant_nutrition(hunger_satisfaction, false);
                }
            }

            // Update child nutrition for children
            if agent.state.life_stage == LifeStage::Child {
                let hunger_satisfaction = 1.0 - agent.drives.get(DriveType::Hunger)
                    .map(|d| d.value)
                    .unwrap_or(0.0);
                agent.developmental_nutrition.update_child_nutrition(hunger_satisfaction, agent.state.health);
            }

            // Finalize developmental stats when transitioning to adult
            if agent.state.life_stage == LifeStage::Adult && !agent.developmental_nutrition.finalized {
                let became_infertile = agent.developmental_nutrition.finalize();

                if became_infertile {
                    // Severe malnutrition caused permanent infertility
                    agent.traits.add_trait(crate::core::traits::Trait::Infertile);
                    debug!(
                        "Agent {} reached adulthood but severe malnutrition caused INFERTILITY",
                        agent.id
                    );
                }

                debug!(
                    "Agent {} reached adulthood with development: {:?}",
                    agent.id, agent.developmental_nutrition.stat_modifiers
                );
            }
        }

        // And what feeding them cost the women who did it.
        //
        // Milk is not free. A nursing mother eats for two, and if there is not
        // enough for two she is the one who goes short - which is why a hungry
        // season shows up in the next generation rather than only in this one.
        for (who, units) in what_the_milk_cost {
            if let Some(mother) = self
                .population
                .agents
                .iter_mut()
                .find(|a| a.id == who && a.state.is_alive)
            {
                mother.state.physiology.reserve =
                    (mother.state.physiology.reserve - units).max(0.0);
            }
        }
    }
}
